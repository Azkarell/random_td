use bevy::{
    asset::{AssetServer, Assets, Handle, LoadedFolder},
    ecs::{
        component::Component,
        entity::Entity,
        event::Event,
        observer::Trigger,
        query::{With, Without},
        resource::Resource,
        system::{Commands, Query, Res, ResMut},
    },
    image::Image,
    log::{error, info},
    math::{Vec2, Vec3, Vec3Swizzles, primitives::Circle},
    prelude::{Deref, DerefMut},
    render::{mesh::Mesh, view::Visibility},
    sprite::{ColorMaterial, Sprite},
    time::{Time, Timer},
    transform::components::Transform,
};
use rand::{
    Rng, random_range, rng,
    seq::{IndexedRandom, IteratorRandom},
};

use crate::{
    assets::{ENEMY_COLOR, ENEMY_FOLDER, ENEMY_PLAYER_DAMAGE, ENEMY_RADIUS},
    grid::{GridEntity, GridIndex, HexGridRenderRadius, PathStart},
    path::HexPath,
    player::{Gold, GoldGained, Player},
    stats::{Damage, Health, Speed, Wave},
    tower::TowerTraversal,
};

#[derive(Event, Clone)]
#[event(traversal = TowerTraversal, auto_propagate)]
pub struct EnemyMoved {
    pub entity: Entity,
    pub position: Vec2,
}

#[derive(Event)]
pub struct DamageTaken {
    pub amount: f32,
}

#[derive(Event)]
pub struct EnemyRemoved {
    pub entity: Entity,
}

#[derive(Component)]
pub struct Enemy;
#[derive(Component)]
pub struct EnemyCurrentTarget(pub GridIndex);
#[derive(Component, Deref)]
pub struct EnemySize(pub f32);
#[derive(Resource)]
pub struct EnemyMesh(pub Handle<Mesh>);
#[derive(Resource)]
pub struct EnemyMaterial(pub Handle<ColorMaterial>);
#[derive(Resource, Deref, DerefMut)]
pub struct SpawnTimer(pub Timer);
#[derive(Resource, Deref)]
pub struct EnemyImageFolder(Handle<LoadedFolder>);

impl EnemyImageFolder {
    pub fn get_random_enemy_image<R: Rng>(
        &self,
        rng: &mut R,
        assets: &Assets<LoadedFolder>,
    ) -> Handle<Image> {
        let real = assets.get(&self.0).unwrap();
        real.handles.choose(rng).unwrap().clone().typed::<Image>()
    }
}
#[derive(Resource)]
pub struct SpawnCounter {
    pub current: u32,
    pub max: u32,
}

impl SpawnCounter {
    pub fn can_spawn(&self) -> bool {
        self.current < self.max
    }
    pub fn new(max: u32) -> Self {
        Self { current: 0, max }
    }
}

pub fn enemies_are_loaded(enemies: Res<EnemyImageFolder>, asset_server: Res<AssetServer>) -> bool {
    asset_server.is_loaded_with_dependencies(enemies.id())
}
pub fn setup_enemy_resources(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    asset_server: Res<AssetServer>,
) {
    let mesh = meshes.add(Circle::new(ENEMY_RADIUS));
    commands.insert_resource(EnemyMesh(mesh));
    let mat = materials.add(ENEMY_COLOR);
    commands.insert_resource(EnemyMaterial(mat));

    let enemies = asset_server.load_folder(ENEMY_FOLDER);
    commands.insert_resource(EnemyImageFolder(enemies));
}

pub fn init_spawn_timer(mut commands: Commands, wave: Res<Wave>) {
    commands.insert_resource(SpawnTimer(Timer::from_seconds(
        1.0,
        bevy::time::TimerMode::Repeating,
    )));

    commands.insert_resource(SpawnCounter::new(20 + wave.0 * 5));
}

#[allow(clippy::too_many_arguments)]
pub fn spawn_enemy(
    mut commands: Commands,
    wave: Res<Wave>,
    //mesh: Res<EnemyMesh>,
    //material: Res<EnemyMaterial>,
    starts: Query<&GridEntity, With<PathStart>>,
    size: Res<HexGridRenderRadius>,
    time: Res<Time>,
    mut spawn_timer: ResMut<SpawnTimer>,
    mut spawn_counter: ResMut<SpawnCounter>,
    hex_path: Res<HexPath<GridIndex>>,
    enemy_image_folder: Res<EnemyImageFolder>,
    loaded_folder_assets: Res<Assets<LoadedFolder>>,
) {
    let mut rng = rng();
    let start = starts.iter().choose(&mut rng);
    if start.is_none() {
        error!("Failed to get start");
        return;
    }
    spawn_timer.tick(time.delta());
    if spawn_timer.just_finished() && spawn_counter.can_spawn() {
        let world_pos = start.unwrap().0.to_world_pos(**size);

        if let Some(n) = hex_path.get_next(start.unwrap().0) {
            let health = random_range(10.0..=30.0) + wave.0 as f32 * 10.0;
            let speed = random_range(30.0..100.0) + wave.0 as f32 * 30.0;
            let gold = random_range(5..=10) * (wave.0 + 1);
            let image = enemy_image_folder.get_random_enemy_image(&mut rng, &loaded_folder_assets);
            info!("image: {image:?}");
            commands
                .spawn((
                    Enemy,
                    EnemyCurrentTarget(n),
                    Damage(ENEMY_PLAYER_DAMAGE),
                    Health(health),
                    Speed(speed),
                    Gold(gold),
                    EnemySize(ENEMY_RADIUS),
                    //Mesh2d(mesh.0.clone()),
                    //MeshMaterial2d(material.0.clone()),
                    Visibility::Visible,
                    Transform::from_xyz(world_pos.x, world_pos.y, 10.0),
                    Sprite {
                        image,
                        custom_size: Some(Vec2::new(ENEMY_RADIUS * 2.0, ENEMY_RADIUS * 2.0)),
                        color: bevy::color::Color::hsl(
                            random_range(0.0..=360.0),
                            random_range(0.0..=1.0),
                            random_range(0.0..=1.0),
                        ),
                        ..Default::default()
                    },
                ))
                .observe(on_hit);
            spawn_counter.current += 1;
        } else {
            info!("Enemy: at {}, index {:?}", world_pos, start.unwrap().0);
            error!("could not get next destination");
        }
    }
}

pub fn on_hit(
    trigger: Trigger<DamageTaken>,
    mut commands: Commands,
    mut query: Query<(&mut Health, &Gold), With<Enemy>>,
) {
    let Ok((mut h, g)) = query.get_mut(trigger.target()) else {
        return;
    };

    h.0 -= trigger.event().amount;
    if h.0 <= 0.0 {
        commands.entity(trigger.target()).despawn();
        commands.trigger(GoldGained { amount: g.0 });
    }
}

#[allow(clippy::type_complexity)]
pub fn update_enemy(
    mut commands: Commands,
    enemies: Query<
        (
            Entity,
            &mut Transform,
            &mut EnemyCurrentTarget,
            &Damage,
            &Speed,
            &Health,
        ),
        (With<Enemy>, Without<Player>),
    >,
    path: Res<HexPath<GridIndex>>,
    size: Res<HexGridRenderRadius>,
    time: Res<Time>,
    mut player: Query<&mut Health, With<Player>>,
) {
    if let Ok(mut p_h) = player.single_mut() {
        for (e, mut t, mut target, d, s, h) in enemies {
            if h.0 < 0.0 {
                commands.entity(e).despawn();
                continue;
            }
            let wp = t.translation.xy();
            if (wp.distance(target.0.to_world_pos(**size))) < ENEMY_RADIUS {
                if path.end == target.0 {
                    commands.entity(e).despawn();
                    p_h.0 -= d.0;
                    continue;
                }
                if let Some(n) = path.get_next(target.0) {
                    target.0 = n
                } else {
                    error!("failed to get next pos");
                    continue;
                }
            }

            let target_pos = target.0.to_world_pos(**size);
            let dir = (target_pos - t.translation.xy()).normalize();

            t.translation += Vec3::new(dir.x, dir.y, 0.0) * s.0 * time.delta_secs();
            commands.trigger(EnemyMoved {
                entity: e,
                position: t.translation.xy(),
            });
            commands.trigger_targets(
                EnemyMoved {
                    entity: e,
                    position: t.translation.xy(),
                },
                Entity::PLACEHOLDER,
            );
        }
    }
}
