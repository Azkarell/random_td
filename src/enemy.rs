use bevy::{
    asset::{Assets, Handle},
    ecs::{
        component::Component,
        entity::Entity,
        query::With,
        resource::Resource,
        system::{Commands, Query, Res, ResMut},
    },
    log::error,
    math::{Vec3, Vec3Swizzles, primitives::Circle},
    prelude::{Deref, DerefMut},
    render::mesh::{Mesh, Mesh2d},
    sprite::{ColorMaterial, MeshMaterial2d},
    time::{Time, Timer},
    transform::components::Transform,
};
use rand::{random_range, rng, seq::IteratorRandom};

use crate::{
    assets::{ENEMY_COLOR, ENEMY_RADIUS},
    grid::{GridEntity, GridIndex, HexGridRenderRadius, PathStart},
    path::HexPath,
};

#[derive(Component)]
#[require(Health = Health(20.0), Speed = Speed(10.0))]
pub struct Enemy;
#[derive(Component)]
pub struct EnemyCurrentTarget(pub GridIndex);
#[derive(Component)]
pub struct Health(pub f32);
#[derive(Component)]
pub struct Speed(pub f32);
#[derive(Component)]
pub struct Armor(pub f32);
#[derive(Component, Deref)]
pub struct EnemySize(pub f32);
#[derive(Resource, Deref, Debug)]
pub struct Wave(pub u32);
#[derive(Resource)]
pub struct EnemyMesh(pub Handle<Mesh>);
#[derive(Resource)]
pub struct EnemyMaterial(pub Handle<ColorMaterial>);
#[derive(Resource, Deref, DerefMut)]
pub struct SpawnTimer(pub Timer);
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
pub fn setup_enemy_resources(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let mesh = meshes.add(Circle::new(ENEMY_RADIUS));
    commands.insert_resource(EnemyMesh(mesh));
    let mat = materials.add(ENEMY_COLOR);
    commands.insert_resource(EnemyMaterial(mat));
}
pub fn init_spawn_timer(mut commands: Commands) {
    commands.insert_resource(SpawnTimer(Timer::from_seconds(
        1.0,
        bevy::time::TimerMode::Repeating,
    )));

    commands.insert_resource(SpawnCounter::new(20));
}
#[allow(clippy::too_many_arguments)]
pub fn spawn_enemy(
    mut commands: Commands,
    wave: Res<Wave>,
    mesh: Res<EnemyMesh>,
    material: Res<EnemyMaterial>,
    starts: Query<&GridEntity, With<PathStart>>,
    size: Res<HexGridRenderRadius>,
    time: Res<Time>,
    mut spawn_timer: ResMut<SpawnTimer>,
    mut spawn_counter: ResMut<SpawnCounter>,
    hex_path: Res<HexPath<GridIndex>>,
) {
    let mut rng = rng();
    let health = random_range(10.0..=30.0) + wave.0 as f32 * 10.0;
    let speed = random_range(30.0..100.0) + wave.0 as f32 * 10.0;

    let start = starts.iter().choose(&mut rng);
    if start.is_none() {
        error!("Failed to get start");
        return;
    }
    spawn_timer.tick(time.delta());
    if spawn_timer.just_finished() && spawn_counter.can_spawn() {
        let world_pos = start.unwrap().0.to_world_pos(**size);
        if let Some(n) = hex_path.get_next(start.unwrap().0) {
            commands.spawn((
                Enemy,
                EnemyCurrentTarget(n),
                Health(health),
                Speed(speed),
                EnemySize(ENEMY_RADIUS),
                Mesh2d(mesh.0.clone()),
                MeshMaterial2d(material.0.clone()),
                Transform::from_xyz(world_pos.x, world_pos.y, 5.0),
            ));
            spawn_counter.current += 1;
        } else {
            error!("could not get next destination");
        }
    }
}

pub fn move_enemy(
    mut commands: Commands,
    enemies: Query<(Entity, &mut Transform, &mut EnemyCurrentTarget, &Speed), With<Enemy>>,
    path: Res<HexPath<GridIndex>>,
    size: Res<HexGridRenderRadius>,
    time: Res<Time>,
) {
    for (e, mut t, mut target, s) in enemies {
        let wp = t.translation.xy();
        if (wp.distance(target.0.to_world_pos(**size))) < 1.0 {
            if path.end == target.0 {
                commands.entity(e).despawn();
                // TODO: trigger player hit
                return;
            }
            if let Some(n) = path.get_next(target.0) {
                target.0 = n
            } else {
                error!("failed to get next pos");
                return;
            }
        }

        let target_pos = target.0.to_world_pos(**size);
        let dir = (target_pos - t.translation.xy()).normalize();

        t.translation += Vec3::new(dir.x, dir.y, 0.0) * s.0 * time.delta_secs();
    }
}
