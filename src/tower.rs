use std::time::Duration;

use bevy::{
    asset::{Assets, Handle},
    ecs::{
        component::Component,
        entity::Entity,
        query::{With, Without},
        resource::Resource,
        system::{Commands, Query, Res, ResMut},
    },
    input::{ButtonInput, mouse::MouseButton},
    log::info,
    math::{Vec2, Vec3Swizzles, primitives::Circle},
    prelude::Deref,
    render::mesh::{Mesh, Mesh2d},
    sprite::{ColorMaterial, MeshMaterial2d},
    time::{Time, Timer, TimerMode},
    transform::components::Transform,
};
use rand::random_range;

use crate::{
    assets::{PROJECTILE_COLOR, PROJECTILE_SIZE, PROJECTILE_SPEED},
    enemy::{Enemy, EnemySize, Health, Speed},
    grid::{GridEntity, GridEntry, GridIndex, HexGridRenderRadius, HexHashGrid},
    input::MouseWorldPos,
};

#[derive(Component, Deref, Clone, Copy)]
pub struct Damage(pub f32);
#[derive(Component, Deref, Clone, Copy)]
pub struct Range(pub f32);
#[derive(Component)]
pub struct FireRate(pub f32, pub Option<Timer>);
#[derive(Component)]
pub struct Projectile {
    start: Vec2,
}
#[derive(Component)]
pub struct ProjectileDirection(pub Vec2);

#[derive(Component)]
pub struct Tower;

#[derive(Resource)]
pub struct ProjectilMesh(pub Handle<Mesh>);
#[derive(Resource)]
pub struct ProjectilColor(pub Handle<ColorMaterial>);

pub fn init_turret_resources(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let mesh = meshes.add(Circle::new(PROJECTILE_SIZE));
    let color = materials.add(PROJECTILE_COLOR);
    commands.insert_resource(ProjectilMesh(mesh));
    commands.insert_resource(ProjectilColor(color));
}

pub fn place_tower(
    mut commands: Commands,
    mouse_position: Res<MouseWorldPos>,
    input: Res<ButtonInput<MouseButton>>,
    mut grid: ResMut<HexHashGrid>,
    size: Res<HexGridRenderRadius>,
    query: Query<(Entity, &GridEntity)>,
) {
    if input.just_pressed(MouseButton::Left)
        && let Some(p) = **mouse_position
        && grid.contains(&GridIndex::from_world_pos(p, **size))
        && grid[GridIndex::from_world_pos(p, **size)].0 == GridEntry::None
    {
        let index = GridIndex::from_world_pos(p, **size);
        grid[index].0 = GridEntry::Tower;
        if let Some((e, _i)) = query.iter().find(|(_e, i)| i.0 == index) {
            let damage = random_range(5.0..=15.0);
            let range = random_range(150.0..=350.0);
            let fire_rate = random_range(30.0..=90.0);
            commands.entity(e).with_child((
                Tower,
                Damage(damage),
                Range(range),
                GridEntity(index),
                FireRate(fire_rate, None),
            ));
        }
    }
}

pub fn update_tower(
    mut commands: Commands,
    query: Query<(&mut FireRate, &Damage, &Range, &GridEntity), With<Tower>>,
    enemies: Query<(Entity, &Transform), With<Enemy>>,
    projectile_mesh: Res<ProjectilMesh>,
    projectile_material: Res<ProjectilColor>,
    time: Res<Time>,
    size: Res<HexGridRenderRadius>,
) {
    let delta = time.delta();
    for (mut fr, d, r, e) in query {
        let position = e.0.to_world_pos(**size);
        if let Some((_e, t)) = enemies.iter().min_by(|(_, rhs_t), (_, lhs_t)| {
            let d1 = rhs_t.translation.xy().distance_squared(position);
            let d2 = lhs_t.translation.xy().distance_squared(position);
            d1.partial_cmp(&d2).unwrap_or(std::cmp::Ordering::Equal)
        }) && t.translation.xy().distance(position) <= r.0
        {
            info!("found enemy in range");
            let timer = if let Some(timer) = &mut fr.1 {
                timer
            } else {
                fr.1 = Some(Timer::new(
                    Duration::from_secs_f32(60.0 / fr.0),
                    TimerMode::Repeating,
                ));
                fr.1.as_mut().unwrap()
            };
            timer.tick(delta);
            if timer.just_finished() {
                let transform = Transform::from_xyz(position.x, position.y, 5.0);
                let dir = t.translation.xy() - position;
                commands.spawn((
                    Mesh2d(projectile_mesh.0.clone()),
                    MeshMaterial2d(projectile_material.0.clone()),
                    transform,
                    Projectile { start: position },
                    ProjectileDirection(dir.normalize()),
                    Speed(PROJECTILE_SPEED),
                    *d,
                    *r,
                ));
            }
        } else {
            fr.1 = None;
        }
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
    mut enemies: Query<(Entity, &Transform, &mut Health, &EnemySize), With<Enemy>>,
    time: Res<Time>,
) {
    for (e, p, dir, r, s, d, mut t) in query {
        t.translation += dir.0.extend(0.0) * s.0 * time.delta_secs();
        if t.translation.xy().distance(p.start) > r.0 {
            commands.entity(e).despawn();
            continue;
        }
        enemies.iter_mut().for_each(|(entity, e_t, mut e_h, size)| {
            if check_collision(e_t, t.as_ref(), size.0) {
                commands.entity(e).despawn();
                if apply_damage(&mut e_h, d) {
                    commands.entity(entity).despawn();
                }
            }
        });
    }
}

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

fn apply_damage(health: &mut Health, damage: &Damage) -> bool {
    health.0 -= damage.0;
    health.0 < 0.0
}
