pub mod assets;
pub mod enemy;
pub mod grid;
pub mod input;
pub mod macros;
pub mod path;
pub mod tower;

use bevy::{
    DefaultPlugins,
    app::{App, Startup, Update},
    core_pipeline::core_2d::Camera2d,
    ecs::{
        change_detection::DetectChanges,
        component::Component,
        entity::Entity,
        query::With,
        resource::Resource,
        schedule::{Condition, IntoScheduleConfigs, SystemSet},
        system::{Commands, Query, Res, ResMut},
    },
    log::error,
    render::camera::{OrthographicProjection, Projection, ScalingMode},
    state::{
        app::AppExtStates,
        condition::in_state,
        state::{NextState, OnEnter, States},
    },
    transform::components::Transform,
    ui::{Node, Val, widget::Text},
};
use enemy::{Wave, init_spawn_timer, move_enemy, setup_enemy_resources, spawn_enemy};
use grid::{
    GridEntry, GridPlugin, GridSet, HexGridColumns, HexGridHeight, HexGridRenderRadius,
    HexGridRows, HexGridWidth, HexHashGrid,
};
use input::{InputPlugin, InputSet, MouseWorldPos};
use path::{
    DefaultSinglePathFinder, PathPlugin, PathSegment, PathSet, SinglePathFinder,
    context::PathContext, random_selected::RandomDijkstra,
};
use tower::{init_turret_resources, place_tower, update_projectiles, update_tower};

fn main() -> bevy::app::AppExit {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins);
    app.add_plugins(GridPlugin::default());
    app.add_plugins(InputPlugin);
    app.add_plugins(PathPlugin);

    app.insert_resource(Wave(0));
    app.insert_state(GameState::Loading);
    app.add_systems(Startup, setup_camera.after(GridSet));
    app.add_systems(Startup, ui_overlay.after(InputSet));
    app.add_systems(Startup, (init_turret_resources, setup_enemy_resources));
    app.add_systems(OnEnter(GameState::BeforeWave), generate_path);
    app.add_systems(Update, update_ui_overlay);
    app.add_systems(
        OnEnter(GameState::Wave),
        (init_spawn_timer).in_set(DuringWave),
    );
    app.add_systems(
        Update,
        (spawn_enemy, move_enemy, update_tower, update_projectiles).in_set(DuringWave),
    );
    app.add_systems(
        Update,
        place_tower.run_if(in_state(GameState::Wave).or(in_state(GameState::BeforeWave))),
    );
    app.configure_sets(
        Update,
        (
            InputSet.before(GridSet),
            PathSet.after(GridSet),
            DuringWave.run_if(in_state(GameState::Wave)),
            BeforeWave.run_if(in_state(GameState::BeforeWave)),
            AfterWave.run_if(in_state(GameState::AfterWave)),
        ),
    );
    app.run()
}

#[derive(Clone, Copy, Hash, Debug, Default, States, PartialEq, Eq)]
pub enum GameState {
    #[default]
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

pub fn setup_camera(
    mut commands: Commands,
    width: Res<HexGridWidth>,
    _height: Res<HexGridHeight>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    let mut projection = OrthographicProjection::default_2d();
    projection.scaling_mode = ScalingMode::FixedHorizontal {
        viewport_width: **width,
    };
    commands
        .spawn(Camera2d)
        .insert(Projection::Orthographic(projection))
        .insert(Transform::from_xyz(0.0, -150.0, 0.0));
    next_state.set(GameState::BeforeWave);
}

#[allow(clippy::too_many_arguments)]
pub fn generate_path(
    mut commands: Commands,
    mut grid: ResMut<HexHashGrid>,
    old: Query<Entity, With<PathSegment>>,
    rows: Res<HexGridRows>,
    columns: Res<HexGridColumns>,
    render_radius: Res<HexGridRenderRadius>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    for g in grid.values_mut() {
        if g.0 == GridEntry::PathEnd || g.0 == GridEntry::PathStart || g.0 == GridEntry::Path {
            g.0 = GridEntry::None;
        }
    }
    for o in old {
        commands.entity(o).despawn();
    }
    let path_finder = DefaultSinglePathFinder::new(RandomDijkstra {
        tile_size: **render_radius,
    });
    let context = PathContext::from_args(&rows, &columns, &grid);
    let path = path_finder.get_path(context);
    if let Some(pa) = path {
        for p in &pa.nodes {
            if *p == pa.start {
                grid[*p].0 = GridEntry::PathStart;
            } else if *p == pa.end {
                grid[*p].0 = GridEntry::PathEnd;
            } else {
                grid[*p].0 = GridEntry::Path
            }
        }
        commands.insert_resource(pa);
        next_state.set(GameState::Wave)
    } else {
        error!("Failed to find path");
    }
}

#[derive(Component)]
pub struct MousePositionText;

fn ui_overlay(mut commands: Commands, mouse_position: Res<MouseWorldPos>) {
    let text = if let Some(p) = **mouse_position {
        format!("mouse: {},{}", p.x, p.y)
    } else {
        "mouse: None".to_string()
    };
    commands.spawn((
        Text::new(text),
        MousePositionText,
        Node {
            position_type: bevy::ui::PositionType::Absolute,
            top: Val::Px(10.0),
            left: Val::Px(10.0),
            ..Default::default()
        },
    ));
}

fn update_ui_overlay(
    query: Query<&mut Text, With<MousePositionText>>,
    mouse_position: Res<MouseWorldPos>,
) {
    if mouse_position.is_changed() {
        let text = if let Some(p) = **mouse_position {
            format!("mouse: {},{}", p.x, p.y)
        } else {
            "mouse: None".to_string()
        };
        for mut t in query {
            t.0 = text.clone();
        }
    }
}
