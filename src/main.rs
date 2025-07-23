use std::{
    collections::HashMap,
    ops::{Index, IndexMut},
    sync::LazyLock,
};

use bevy::{
    DefaultPlugins,
    app::{App, Startup, Update},
    asset::{AssetServer, Assets, Handle},
    color::{Color, palettes::css::BLACK},
    core_pipeline::core_2d::Camera2d,
    ecs::{
        component::Component,
        query::{With, Without},
        resource::Resource,
        system::{Commands, Query, Res, ResMut},
    },
    log::info,
    math::{
        Mat2, Vec2, Vec3, Vec3Swizzles,
        ops::{floor, sqrt},
        primitives::RegularPolygon,
    },
    render::{
        camera::{Camera, CameraProjection, OrthographicProjection, Projection, ScalingMode},
        mesh::{Mesh, Mesh2d},
    },
    sprite::{ColorMaterial, MeshMaterial2d},
    text::{Text2d, TextColor, TextFont},
    transform::components::{GlobalTransform, Transform},
    window::Window,
};
pub const COLUMNS: i32 = 16;
pub const COLUMN_WIDTH: f32 = 100.0;
pub const ROWS: i32 = 10;
fn main() -> bevy::app::AppExit {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins);
    app.add_systems(Startup, init_grid);
    app.add_systems(Startup, setup_camera);
    app.add_systems(Update, highlight_grid_cell);
    app.run()
}
pub fn setup_camera(mut commands: Commands) {
    let mut projection = OrthographicProjection::default_2d();
    projection.scaling_mode = ScalingMode::Fixed {
        width: COLUMN_WIDTH * (COLUMNS + 1) as f32,
        height: COLUMN_WIDTH * (ROWS + 1) as f32,
    };
    commands
        .spawn(Camera2d)
        .insert(Projection::Orthographic(projection));
}

pub fn init_grid(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    asset_server: Res<AssetServer>,
) {
    let font = asset_server.load("fonts/FiraCodeNerdFont-Regular.ttf");
    let text_font = TextFont {
        font: font.clone(),
        font_size: 10.0,
        ..Default::default()
    };
    let padding = 10.0;
    let grid_margin = 2.0;
    let cirumradius = (COLUMN_WIDTH - 2.0 * padding) / 2.0 - grid_margin;
    let hexagon = meshes.add(RegularPolygon::new(cirumradius, 6));
    let color = Color::hsla(0.0, 1.0, 0.95, 1.0);
    let material = materials.add(color);
    let highlight_material = materials.add(Color::hsla(0.5, 0.5, 0.3, 1.0));
    commands.insert_resource(HighlightColor(highlight_material));
    commands.insert_resource(NormalColor(material.clone()));
    commands.insert_resource(HexHashGrid::new());
    for r in -COLUMNS / 2..=COLUMNS / 2 {
        let s = -ROWS + (-(r as f32) / 2.0).ceil() as i32;
        let e = ROWS + (-(r as f32) / 2.0).ceil() as i32;
        for q in s..=e {
            let coords = GridAccess::Axial { q, r };
            let pos = coords.to_pixel(cirumradius + grid_margin);
            commands.spawn((
                Mesh2d(hexagon.clone()),
                MeshMaterial2d(material.clone()),
                Transform::from_xyz(pos.x, pos.y, 0.0),
                GridEntity(coords),
                Text2d(format!("{q}\n{r}")),
                text_font.clone(),
                TextColor(Color::from(BLACK)),
            ));
        }
    }
}

#[derive(Resource)]
pub struct HighlightColor(Handle<ColorMaterial>);

#[derive(Resource)]
pub struct NormalColor(Handle<ColorMaterial>);

fn axial_to_cube(vec: Vec2) -> Vec3 {
    Vec3::new(vec.x, vec.y, -vec.x - vec.y)
}

fn cube_to_axial(vec: Vec3) -> Vec2 {
    vec.xy()
}

fn axial_round(vec: Vec2) -> (i32, i32) {
    let v = axial_to_cube(vec);
    let r = cube_round(v);
    (r.x as i32, r.y as i32)
}

fn cube_round(vec: Vec3) -> Vec3 {
    let mut rounded = vec.round();
    let diff = (rounded - vec).abs();

    if diff.x > diff.y && diff.x > diff.z {
        rounded.x = -rounded.y - rounded.z;
    } else if diff.y > diff.z {
        rounded.y = -rounded.x - rounded.z;
    } else {
        rounded.z = -rounded.x - rounded.y;
    }
    rounded
}

pub fn highlight_grid_cell(
    window: Query<&Window>,
    camera: Query<(&Camera, &GlobalTransform)>,
    q_entries: Query<(&GridEntity, &mut MeshMaterial2d<ColorMaterial>)>,
    highlight_material: Res<HighlightColor>,
    normal_color: Res<NormalColor>,
) {
    let (c, t) = camera.single().expect("Camera should be singelton");
    let w = window.single().expect("window should be singelton");
    let pos = if let Some(world_position) = w
        .cursor_position()
        .and_then(|cursor| c.viewport_to_world_2d(t, cursor).ok())
    {
        let padding = 10.0;
        let size = (COLUMN_WIDTH - 2.0 * padding) / 2.0;
        info!("{world_position:?}");
        let x = world_position.x / size;
        let y = world_position.y / size;
        let q = AXIAL_INVERTED.mul_vec2(Vec2::new(x, y));
        let rounded = axial_round(q);
        let pos = GridAccess::Axial {
            q: rounded.0,
            r: rounded.1,
        };
        Some(pos)
    } else {
        None
    };
    info!("{pos:?}");
    if let Some(p) = pos {
        for mut e in q_entries {
            if e.0.0 == p {
                e.1.0 = highlight_material.0.clone();
            } else {
                e.1.0 = normal_color.0.clone();
            }
        }
    } else {
        for mut e in q_entries {
            e.1.0 = normal_color.0.clone();
        }
    }
}

#[derive(Resource)]
pub struct HexHashGrid {
    data: HashMap<(i32, i32), GridEntry>,
}

#[derive(Component)]
pub struct GridEntity(pub GridAccess);

impl HexHashGrid {
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }
}

impl Default for HexHashGrid {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum GridAccess {
    Axial { q: i32, r: i32 },
}

pub static AXIAL_CONVERT: LazyLock<Mat2> =
    LazyLock::new(|| Mat2::from_cols(Vec2::new(sqrt(3.0), 0.0), Vec2::new(sqrt(3.0) / 2.0, 1.5)));
pub static AXIAL_INVERTED: LazyLock<Mat2> = LazyLock::new(|| {
    Mat2::from_cols(
        Vec2::new(sqrt(3.0) / 3.0, 0.0),
        Vec2::new(-1.0 / 3.0, 2.0 / 3.0),
    )
});
impl GridAccess {
    fn to_axial(&self) -> Self {
        match self {
            GridAccess::Axial { q, r } => Self::Axial { q: *q, r: *r },
        }
    }
    pub fn to_pixel(&self, size: f32) -> Vec2 {
        match self.to_axial() {
            GridAccess::Axial { r, q } => {
                let mat = *AXIAL_CONVERT;
                let vec = Vec2::new(q as f32, r as f32);
                mat.mul_vec2(vec) * size
            }
            _ => {
                unreachable!("should never happen")
            }
        }
    }
}

impl IndexMut<GridAccess> for HexHashGrid {
    fn index_mut(&mut self, index: GridAccess) -> &mut Self::Output {
        match index {
            GridAccess::Axial { r, q } => {
                if let None = self.data.get_mut(&(q, r)) {
                    self.data.insert((q, r), GridEntry::None);
                }
                self.data.get_mut(&(q, r)).unwrap()
            }
        }
    }
}

impl Index<GridAccess> for HexHashGrid {
    type Output = GridEntry;

    fn index(&self, index: GridAccess) -> &Self::Output {
        match index {
            GridAccess::Axial { r, q } => &self.data[&(q, r)],
        }
    }
}

#[derive(Clone, Copy)]
pub enum GridEntry {
    None,
    Tower,
    Path,
}
