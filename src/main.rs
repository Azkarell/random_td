pub mod assets;
pub mod enemy;
pub mod grid;
pub mod input;
pub mod macros;
pub mod path;
pub mod player;
pub mod state_conditions;
pub mod stats;
pub mod tower;
pub mod ui;

use assets::MAIN_LOOP;
use bevy::{
    DefaultPlugins,
    app::{App, Startup, Update},
    asset::AssetServer,
    audio::{AudioPlayer, PlaybackSettings},
    core_pipeline::core_2d::Camera2d,
    diagnostic::{EntityCountDiagnosticsPlugin, FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    ecs::{
        component::ComponentId,
        entity::Entity,
        query::{Or, With},
        schedule::{Condition, IntoScheduleConfigs, common_conditions::resource_exists},
        system::{Commands, Query, Res, ResMut},
    },
    log::{error, info},
    picking::mesh_picking::MeshPickingPlugin,
    prelude::*,
    render::{
        camera::{OrthographicProjection, Projection, ScalingMode},
        diagnostic::RenderDiagnosticsPlugin,
    },
    sprite::{ColorMaterial, MeshMaterial2d},
    state::{app::AppExtStates, condition::in_state, state::OnEnter},
    transform::components::Transform,
};
use bevy_dev_tools::picking_debug::{DebugPickingMode, DebugPickingPlugin};
use enemy::{
    DamageTaken, EnemyMoved, SpawnCounter, enemies_are_loaded, init_spawn_timer,
    setup_enemy_resources, spawn_enemy, update_enemy,
};
use grid::{
    DefaultHexMaterial, GridEntity, GridEntry, GridPlugin, GridSet, HexGridColumns, HexGridHeight,
    HexGridRenderRadius, HexGridRows, HexGridWidth, HexHashGrid, Path, PathEnd, PathEndMaterial,
    PathMaterial, PathStart, PathStartMaterial,
};
use input::{InputPlugin, InputSet};
use path::{
    DefaultSinglePathFinder, PathPlugin, PathSet, SinglePathFinder, context::PathContext,
    random_selected::RandomDijkstra,
};
use player::{GoldGained, game_running, on_gold_gained, setup_player};
use state_conditions::{change_state, wave_done};
use stats::Wave;
use tower::{Tower, init_tower_resources, update_projectiles, update_tower};
use ui::UiOverlay;

fn main() -> bevy::app::AppExit {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins);
    //app.add_plugins(DefaultPlugins.set(bevy::log::LogPlugin {
    //    filter: "bevy_dev_tools=trace".into(),
    //    ..Default::default()
    //}));
    app.add_plugins(FrameTimeDiagnosticsPlugin::default());
    app.add_plugins(LogDiagnosticsPlugin::default());
    // app.add_plugins(EntityCountDiagnosticsPlugin);
    // app.add_plugins(RenderDiagnosticsPlugin);
    app.add_plugins(MeshPickingPlugin);
    //app.add_plugins(DebugPickingPlugin);
    app.add_plugins(GridPlugin::default());
    app.add_plugins(InputPlugin);
    app.add_plugins(PathPlugin);
    app.add_plugins(UiOverlay);
    //app.add_plugins(DebugUiOverlay);
    app.insert_resource(Wave(0));
    app.insert_resource(DebugPickingMode::Normal);
    app.insert_state(GameState::Startup);
    app.add_event::<DamageTaken>();
    app.add_event::<GoldGained>();
    app.add_event::<EnemyMoved>();
    app.world_mut().register_component::<Tower>();
    let id = app.world().component_id::<Tower>().unwrap();
    app.insert_resource(TowerTargets(id));
    app.add_observer(on_gold_gained);
    app.add_systems(Startup, setup_camera.after(GridSet));
    app.add_systems(
        OnEnter(GameState::Startup),
        (init_tower_resources, setup_enemy_resources),
    );
    app.add_systems(
        OnEnter(GameState::Loading),
        ((setup_player), change_state(GameState::BeforeWave)).chain(),
    );
    app.add_systems(
        Update,
        change_state(GameState::Loading)
            .run_if(enemies_are_loaded)
            .in_set(StartupSet),
    );
    app.add_systems(OnEnter(GameState::BeforeWave), generate_path);
    app.add_systems(
        Update,
        (change_state(GameState::Wave).run_if(path_ready)).in_set(BeforeWave),
    );
    app.add_systems(
        OnEnter(GameState::Wave),
        (init_spawn_timer).in_set(DuringWave),
    );
    app.add_systems(
        Update,
        (
            spawn_enemy,
            update_enemy,
            update_tower,
            update_projectiles,
            change_state(GameState::AfterWave)
                .run_if(resource_exists::<SpawnCounter>.and(wave_done)),
        )
            .in_set(DuringWave),
    );

    app.add_systems(
        OnEnter(GameState::AfterWave),
        (
            cleanup_path,
            update_wave,
            change_state(GameState::BeforeWave).run_if(game_running),
        )
            .chain(),
    );
    app.configure_sets(
        Update,
        (
            InputSet.before(GridSet),
            PathSet.after(GridSet),
            DuringWave.run_if(in_state(GameState::Wave)),
            BeforeWave.run_if(in_state(GameState::BeforeWave)),
            AfterWave.run_if(in_state(GameState::AfterWave)),
            GridSet.run_if(in_state(GameState::Wave).or(in_state(GameState::BeforeWave))),
            StartupSet.run_if(in_state(GameState::Startup)),
        ),
    );
    app.run()
}
#[derive(Resource)]
pub struct TowerTargets(pub ComponentId);
#[derive(Clone, Copy, Hash, Debug, Default, States, PartialEq, Eq)]
pub enum GameState {
    #[default]
    Startup,
    Loading,
    Wave,
    BeforeWave,
    AfterWave,
}
#[derive(SystemSet, Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct DuringWave;
#[derive(SystemSet, Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct BeforeWave;
#[derive(SystemSet, Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct AfterWave;
#[derive(SystemSet, Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct StartupSet;

pub fn path_ready(query: Query<(), With<PathStart>>) -> bool {
    !query.is_empty()
}
pub fn setup_camera(
    mut commands: Commands,
    width: Res<HexGridWidth>,
    _height: Res<HexGridHeight>,
    asset_server: Res<AssetServer>,
) {
    let background = asset_server.load(MAIN_LOOP);
    commands.spawn((
        AudioPlayer::new(background),
        PlaybackSettings::LOOP.with_volume(bevy::audio::Volume::Linear(0.2)),
    ));
    let mut projection = OrthographicProjection::default_2d();
    projection.scaling_mode = ScalingMode::FixedHorizontal {
        viewport_width: **width,
    };
    commands
        .spawn(Camera2d)
        .insert(Projection::Orthographic(projection))
        .insert(Transform::from_xyz(0.0, 50.0, 0.0));
}
#[allow(clippy::type_complexity)]
pub fn cleanup_path(
    mut commands: Commands,
    path_endings: Query<
        (Entity, &mut MeshMaterial2d<ColorMaterial>, &GridEntity),
        Or<(With<PathStart>, With<PathEnd>, With<Path>)>,
    >,
    mut grid: ResMut<HexHashGrid>,
    default_color: Res<DefaultHexMaterial>,
) {
    info!("removing path");
    for (e, mut m, ge) in path_endings {
        commands.entity(e).remove::<PathStart>();
        commands.entity(e).remove::<PathEnd>();
        commands.entity(e).remove::<Path>();
        m.0 = default_color.0.clone();
        grid[ge.0] = GridEntry::None;
    }
}

pub fn update_wave(mut wave: ResMut<Wave>) {
    wave.0 += 1;
}

#[allow(clippy::too_many_arguments)]
pub fn generate_path(
    mut commands: Commands,
    mut grid: ResMut<HexHashGrid>,
    rows: Res<HexGridRows>,
    columns: Res<HexGridColumns>,
    render_radius: Res<HexGridRenderRadius>,
    mut grid_entities: Query<(Entity, &GridEntity, &mut MeshMaterial2d<ColorMaterial>)>,
    path_material: Res<PathMaterial>,
    path_start_material: Res<PathStartMaterial>,
    path_end_material: Res<PathEndMaterial>,
) {
    let path_finder = DefaultSinglePathFinder::new(RandomDijkstra {
        tile_size: **render_radius,
    });
    let context = PathContext::from_args(&rows, &columns, &grid);
    let path = path_finder.get_path(context);
    if let Some(pa) = path {
        grid_entities.iter_mut().for_each(|(e, entry, mut color)| {
            if entry.0 == pa.start {
                commands.entity(e).insert(PathStart);
                color.0 = path_start_material.0.clone();
            } else if entry.0 == pa.end {
                commands.entity(e).insert(PathEnd);
                color.0 = path_end_material.0.clone();
            } else if pa.contains(&entry.0) {
                commands.entity(e).insert(Path);
                color.0 = path_material.0.clone();
            }
        });
        for p in &pa.nodes {
            if *p == pa.start {
                grid[*p] = GridEntry::PathStart;
            } else if *p == pa.end {
                grid[*p] = GridEntry::PathEnd;
            } else {
                grid[*p] = GridEntry::Path
            }
        }
        commands.insert_resource(pa);
    } else {
        error!("failed to find path");
    }
}
