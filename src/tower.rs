use std::{collections::HashSet, time::Duration};

use bevy::{
    asset::{AssetServer, Assets, Handle},
    audio::{AudioPlayer, AudioSource, PlaybackSettings},
    ecs::{
        component::Component,
        entity::Entity,
        hierarchy::ChildOf,
        observer::Trigger,
        query::{QueryData, With, Without},
        resource::Resource,
        system::{Commands, Query, Res, ResMut},
        traversal::Traversal,
    },
    image::Image,
    log::{debug, error, info},
    math::{
        Vec2, Vec3, Vec3Swizzles,
        primitives::{Annulus, Circle},
    },
    picking::{
        Pickable,
        events::{Out, Over, Pointer},
    },
    prelude::{Deref, DerefMut},
    render::mesh::{Mesh, Mesh2d},
    sprite::{ColorMaterial, MeshMaterial2d, Sprite},
    time::{Time, Timer, TimerMode},
    transform::components::{GlobalTransform, Transform},
};
use rand::random_range;

use crate::{
    assets::{
        BASE_TOWER, PROJECTILE_COLOR, PROJECTILE_SIZE, PROJECTILE_SPEED, RANGE_INDICATOR_COLOR,
        SHOT_SOUND,
    },
    enemy::{DamageTaken, Enemy, EnemyMoved, EnemySize},
    grid::{GridIndex, HexGridRenderRadius, HexSpatialGrid},
    player::Gold,
    stats::{Damage, FireRate, Range, Speed},
};
#[derive(Resource, Deref)]
pub struct TowerRangeIndicatorMesh(pub Handle<Mesh>);
#[derive(Resource, Deref)]
pub struct TowerRangeIndicatorMaterial(pub Handle<ColorMaterial>);

#[derive(Component)]
pub struct Projectile {
    start: Vec2,
}
#[derive(Component)]
pub struct ProjectileDirection(pub Vec2);

#[derive(Component)]
pub struct Tower;
#[derive(Component, Default, Deref, DerefMut)]
pub struct TargetsInRange(pub HashSet<Entity>);
#[derive(Resource)]
pub struct ProjectilMesh(pub Handle<Mesh>);
#[derive(Resource)]
pub struct ProjectilColor(pub Handle<ColorMaterial>);
#[derive(Resource)]
pub struct ShotSound(pub Handle<AudioSource>);
#[derive(Resource)]
pub struct BaseTowerImage(pub Handle<Image>);

#[derive(QueryData)]
pub struct TowerTraversal {
    entity: Entity,
    tower: &'static Tower,
}

impl<E> Traversal<E> for TowerTraversal
where
    E: Clone,
{
    fn traverse(item: Self::Item<'_>, _data: &E) -> Option<Entity> {
        Some(item.entity)
    }
}
pub fn init_tower_resources(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let mesh = meshes.add(Circle::new(PROJECTILE_SIZE));
    let color = materials.add(PROJECTILE_COLOR);
    let shot_sound = asset_server.load(SHOT_SOUND);
    let base_tower = asset_server.load(BASE_TOWER);
    let range_indicator = meshes.add(Annulus::new(0.99, 1.0));
    let range_material = materials.add(RANGE_INDICATOR_COLOR);

    commands.insert_resource(ProjectilMesh(mesh));
    commands.insert_resource(ProjectilColor(color));
    commands.insert_resource(ShotSound(shot_sound));
    commands.insert_resource(BaseTowerImage(base_tower));
    commands.insert_resource(TowerRangeIndicatorMaterial(range_material));
    commands.insert_resource(TowerRangeIndicatorMesh(range_indicator));
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn update_tower(
    mut commands: Commands,
    query: Query<(&mut FireRate, &Damage, &Range, &GlobalTransform), (With<Tower>, Without<Enemy>)>,
    enemies: Query<(Entity, &Transform), With<Enemy>>,
    projectile_mesh: Res<ProjectilMesh>,
    projectile_material: Res<ProjectilColor>,
    time: Res<Time>,
    //shot_sound: Res<ShotSound>,
) {
    let delta = time.delta();
    for (mut fr, d, r, e) in query {
        let position = e.translation().xy();

        if let Some((_e, t)) = enemies.iter().min_by(|(_, rhs_t), (_, lhs_t)| {
            let d1 = rhs_t.translation.xy().distance_squared(position);
            let d2 = lhs_t.translation.xy().distance_squared(position);
            d1.partial_cmp(&d2).unwrap_or(std::cmp::Ordering::Equal)
        }) && t.translation.xy().distance(position) <= r.0
        {
            let mut timer_spawned = false;
            let timer = if let Some(timer) = &mut fr.1 {
                timer
            } else {
                fr.1 = Some(Timer::new(
                    Duration::from_secs_f32(60.0 / fr.0),
                    TimerMode::Repeating,
                ));
                timer_spawned = true;
                fr.1.as_mut().unwrap()
            };
            timer.tick(delta);
            if timer.just_finished() || timer_spawned {
                let transform = Transform::from_xyz(position.x, position.y, 5.0);
                let dir = t.translation.xy() - position;
                commands
                    .spawn((
                        Mesh2d(projectile_mesh.0.clone()),
                        MeshMaterial2d(projectile_material.0.clone()),
                        transform,
                        Projectile { start: position },
                        ProjectileDirection(dir.normalize()),
                        Speed(PROJECTILE_SPEED),
                        TargetsInRange::default(),
                        // AudioPlayer::new(shot_sound.0.clone()),
                        // PlaybackSettings::REMOVE.with_volume(bevy::audio::Volume::Linear(0.3)),
                        *d,
                        *r,
                    ))
                    .observe(on_enemy_moved);
            }
        } else {
            fr.1 = None;
        }
    }
}

fn on_enemy_moved(
    trigger: Trigger<EnemyMoved>,
    mut query: Query<(&mut TargetsInRange, &GlobalTransform, &Range), With<Tower>>,
) {
    info!("tower enemy moved");
    let observer = trigger.observer();
    let Ok((mut tir, t, r)) = query.get_mut(observer) else {
        error!("Observer is not a tower");
        return;
    };

    if trigger.event().position.distance(t.translation().xy()) <= r.0 {
        tir.insert(trigger.event().entity);
    } else {
        tir.remove(&trigger.event().entity);
    }
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn update_projectiles(
    mut commands: Commands,
    query: Query<
        (
            Entity,
            &Projectile,
            &ProjectileDirection,
            &Range,
            &Speed,
            &Damage,
            &mut Transform,
        ),
        Without<Enemy>,
    >,
    mut enemies: Query<(&Transform, &EnemySize), With<Enemy>>,
    time: Res<Time>,
    mut spatial_grid: ResMut<HexSpatialGrid>,
    size: Res<HexGridRenderRadius>,
) {
    for (e, p, dir, r, s, d, mut t) in query {
        t.translation += dir.0.extend(0.0) * s.0 * time.delta_secs();
        if t.translation.xy().distance(p.start) > r.0 {
            commands.entity(e).despawn();
            continue;
        }
        let grid_index = GridIndex::from_world_pos(t.translation.xy(), **size);
        let nearby = spatial_grid.get_nearby(&grid_index);

        for enemy_entity in nearby {
            let Ok((e_t, size)) = enemies.get_mut(enemy_entity) else {
                continue;
            };
            if check_collision(e_t, t.as_ref(), size.0) {
                commands.trigger_targets(DamageTaken { amount: d.0 }, enemy_entity);
                commands.entity(e).despawn();
            }
        }
    }
}

pub fn spawn_tower_at(
    entity: Entity,
    mut commands: Commands,
    base_tower_image: Res<BaseTowerImage>,
    tower_cost: &Gold,
    player_gold: &mut Gold,
) -> bool {
    if player_gold.0 >= tower_cost.0 {
        player_gold.0 -= tower_cost.0;
        let damage = random_range(5.0..=15.0);
        let range = random_range(150.0..=350.0);
        let fire_rate = random_range(30.0..=90.0);
        commands
            .entity(entity)
            .with_child((
                Tower,
                Damage(damage),
                Range(range),
                FireRate(fire_rate, None),
                Sprite {
                    image: base_tower_image.0.clone(),
                    custom_size: Some(Vec2::new(40.0, 40.0)),
                    ..Default::default()
                },
                Transform::from_xyz(0.0, 0.0, 5.0),
                Pickable {
                    should_block_lower: false,
                    is_hoverable: true,
                },
            ))
            .observe(on_tower_hover)
            .observe(on_tower_out);
        true
    } else {
        false
    }
}
#[derive(Component)]
pub struct Indicator;
fn on_tower_hover(
    trigger: Trigger<Pointer<Over>>,
    mut commands: Commands,
    query: Query<&Range, With<Tower>>,
    mesh_handle_indicator: Res<TowerRangeIndicatorMesh>,
    material_handle_indicator: Res<TowerRangeIndicatorMaterial>,
) {
    let Ok(range) = query.get(trigger.target) else {
        return;
    };
    commands.entity(trigger.target).with_child((
        Mesh2d(mesh_handle_indicator.0.clone()),
        MeshMaterial2d(material_handle_indicator.0.clone()),
        Pickable::IGNORE,
        Transform::from_scale(Vec3::new(range.0, range.0, range.0)),
        Indicator,
    ));
}

fn on_tower_out(
    trigger: Trigger<Pointer<Out>>,
    mut commands: Commands,
    query: Query<(Entity, &ChildOf), With<Indicator>>,
) {
    debug!("on_tower_out");
    for (e, p) in query {
        if p.parent() == trigger.target {
            commands.entity(e).despawn();
        }
    }
}

//fn on_shot(trigger: Trigger<OnShot>, position)
fn check_collision(
    enemy_transform: &Transform,
    projectile_transform: &Transform,
    size: f32,
) -> bool {
    enemy_transform
        .translation
        .xy()
        .distance(projectile_transform.translation.xy())
        < size
}
