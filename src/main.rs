pub mod assets;
pub mod grid;
pub mod input;
pub mod macros;
pub mod path;

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
        schedule::IntoScheduleConfigs,
        system::{Commands, Query, Res, ResMut},
    },
    input::{ButtonInput, keyboard::KeyCode, mouse::MouseButton},
    log::error,
    render::{
        camera::{OrthographicProjection, Projection, ScalingMode},
        mesh::{Mesh, Mesh2d},
    },
    sprite::{ColorMaterial, MeshMaterial2d},
    ui::widget::Text,
};
use grid::{
    GridEntry, GridIndex, GridPlugin, GridSet, HexColorMap, HexGridColumns, HexGridHeight,
    HexGridRenderRadius, HexGridRows, HexGridWidth, HexHashGrid,
};
use input::{InputPlugin, InputSet, MouseWorldPos};
use path::{
    DefaultSinglePathFinder, PathPlugin, PathSegment, PathSet, SinglePathFinder,
    chiseled::Chiseled, context::PathContext, dijkstra::Dijkstra, random_selected::RandomDijkstra,
};

fn main() -> bevy::app::AppExit {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins);
    app.add_plugins(GridPlugin::default());
    app.add_plugins(InputPlugin);
    app.add_plugins(PathPlugin);
    app.add_systems(Startup, setup_camera.after(GridSet));
    app.add_systems(Startup, ui_overlay.after(InputSet));
    app.add_systems(Update, generate_path.run_if(path_condition));
    app.add_systems(Update, update_ui_overlay);
    app.add_systems(Update, place_turret);
    app.configure_sets(Update, (InputSet.before(GridSet), PathSet.after(GridSet)));
    app.run()
}

pub fn setup_camera(mut commands: Commands, width: Res<HexGridWidth>, height: Res<HexGridHeight>) {
    let mut projection = OrthographicProjection::default_2d();
    projection.scaling_mode = ScalingMode::FixedHorizontal {
        viewport_width: **width,
    };
    commands
        .spawn(Camera2d)
        .insert(Projection::Orthographic(projection));
}

pub fn path_condition(input: Res<ButtonInput<KeyCode>>) -> bool {
    input.just_pressed(KeyCode::KeyP)
}
pub fn generate_path(
    mut commands: Commands,
    mut grid: ResMut<HexHashGrid>,
    old: Query<Entity, With<PathSegment>>,
    colors: Res<HexColorMap>,
    rows: Res<HexGridRows>,
    columns: Res<HexGridColumns>,
    render_radius: Res<HexGridRenderRadius>,
) {
    for g in grid.values_mut() {
        if g.0 == GridEntry::PathEnd || g.0 == GridEntry::PathStart || g.0 == GridEntry::Path {
            g.0 = GridEntry::None;
        }
    }
    for o in old {
        commands.entity(o).despawn();
    }
    let path_finder = DefaultSinglePathFinder::new(RandomDijkstra);
    let context = PathContext::from_args(&rows, &columns, &grid);
    let path = path_finder.get_path(context);
    if let Some(pa) = path {
        let prev = pa.start;

        for p in pa.nodes {
            if prev != p {
                commands.spawn(PathSegment {
                    start: prev.to_world_pos(**render_radius),
                    end: p.to_world_pos(**render_radius),
                    color: colors.debug_color(),
                });
            }
            if p == pa.start {
                grid[p].0 = GridEntry::PathStart;
            } else if p == pa.end {
                grid[p].0 = GridEntry::PathEnd;
            } else {
                grid[p].0 = GridEntry::Path
            }
        }
    } else {
        error!("Failed to find path");
    }
}

pub fn place_turret(
    mouse_position: Res<MouseWorldPos>,
    input: Res<ButtonInput<MouseButton>>,
    mut grid: ResMut<HexHashGrid>,
    size: Res<HexGridRenderRadius>,
) {
    if input.just_pressed(MouseButton::Left)
        && let Some(p) = **mouse_position
        && grid.contains(&GridIndex::from_world_pos(p, **size))
    {
        grid[GridIndex::from_world_pos(p, **size)].0 = GridEntry::Tower;
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
    commands.spawn((Text::new(text), MousePositionText));
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
