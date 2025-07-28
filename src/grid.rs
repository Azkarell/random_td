use std::{
    ops::{Add, Div, Index, IndexMut, Mul, RangeInclusive, Sub},
    sync::LazyLock,
};

use bevy::{
    app::{Plugin, Startup, Update},
    asset::{AssetServer, Assets, Handle},
    color::Color,
    ecs::{
        component::Component,
        entity::Entity,
        resource::Resource,
        schedule::{IntoScheduleConfigs, SystemSet},
        system::{Commands, Query, Res, ResMut},
    },
    log::info,
    math::{Mat2, Vec2, Vec3, Vec3Swizzles, ops::sqrt, primitives::RegularPolygon},
    platform::collections::HashMap,
    prelude::{Deref, DerefMut},
    render::mesh::{Mesh, Mesh2d},
    sprite::{ColorMaterial, MeshMaterial2d},
    text::{Text2d, TextColor, TextFont},
    transform::components::Transform,
};

use crate::{
    assets::{FONT, FONT_COLOR, FONT_SIZE, PATH_DEBUG_COLOR},
    def_enum,
    input::MouseWorldPos,
    path::HexPath,
};
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GridSet;
pub struct GridPlugin {
    column_width: f32,
    row_width: f32,
    columns: i32,
    rows: i32,
    padding: f32,
    margin: f32,
}
impl Plugin for GridPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.insert_resource(HexGridRows(self.rows));
        app.insert_resource(HexGridColumns(self.columns));
        app.insert_resource(HexHashGrid::new());
        app.insert_resource(HexGridColumnWidth(self.column_width));
        app.insert_resource(HexGridWidth(self.column_width * (self.columns) as f32));
        app.insert_resource(HexGridHeight(self.row_width * (self.rows) as f32));
        app.insert_resource(HexGridCirumRadius(
            ((self.column_width) / 2.0) - self.margin - 2.0 * self.padding,
        ));
        app.insert_resource(HexGridRenderRadius(
            ((self.column_width) / 2.0) - 2.0 * self.padding,
        ));
        app.insert_resource(HexColorMap::default());
        app.add_systems(
            Startup,
            (prepare_colors_materials, init_grid)
                .chain()
                .in_set(GridSet),
        );
        app.add_systems(Update, update_color.in_set(GridSet));
    }
}
impl Default for GridPlugin {
    fn default() -> Self {
        Self {
            column_width: 120.0,
            row_width: 120.0,
            columns: 20,
            rows: 20,
            margin: 2.0,
            padding: 16.0,
        }
    }
}

#[derive(Hash, PartialEq, Eq, Clone, Copy)]
pub struct HexGridEntryState(pub GridEntry, pub GridEntryState);
impl From<(GridEntry, GridEntryState)> for HexGridEntryState {
    fn from(value: (GridEntry, GridEntryState)) -> Self {
        Self(value.0, value.1)
    }
}
#[derive(Resource)]
pub struct HexColorMap {
    colors: HashMap<HexGridEntryState, Color>,
    default_color: Color,
    materials: HashMap<HexGridEntryState, Handle<ColorMaterial>>,
    path_debug_color: Handle<ColorMaterial>,
}

impl Default for HexColorMap {
    fn default() -> Self {
        let default_color = Color::hsla(0.0, 1.0, 0.95, 1.0);

        let mut map = HashMap::new();
        let highlight_color = Color::hsla(0.5, 0.5, 0.3, 1.0);
        map.insert(
            HexGridEntryState(GridEntry::None, GridEntryState::Highlight),
            highlight_color,
        );
        map.insert(
            HexGridEntryState(GridEntry::Tower, GridEntryState::Normal),
            Color::hsla(180.0, 0.5, 0.8, 1.0),
        );
        map.insert(
            HexGridEntryState(GridEntry::Tower, GridEntryState::Highlight),
            Color::hsla(180.0, 0.5, 0.8, 1.0),
        );
        map.insert(
            HexGridEntryState(GridEntry::Path, GridEntryState::Normal),
            Color::hsla(0.3, 0.5, 0.8, 1.0),
        );
        map.insert(
            HexGridEntryState(GridEntry::Path, GridEntryState::Highlight),
            Color::hsla(0.3, 0.5, 0.8, 1.0),
        );
        map.insert(
            HexGridEntryState(GridEntry::PathStart, GridEntryState::Normal),
            Color::hsla(0.8, 0.78, 0.3, 1.0),
        );
        map.insert(
            HexGridEntryState(GridEntry::PathStart, GridEntryState::Highlight),
            Color::hsla(0.8, 0.78, 0.3, 1.0),
        );
        map.insert(
            HexGridEntryState(GridEntry::PathEnd, GridEntryState::Normal),
            Color::hsla(0.8, 0.78, 0.4, 1.0),
        );
        map.insert(
            HexGridEntryState(GridEntry::PathEnd, GridEntryState::Highlight),
            Color::hsla(0.8, 0.78, 0.4, 1.0),
        );
        Self {
            colors: map,
            default_color,
            materials: HashMap::new(),
            path_debug_color: Handle::default(),
        }
    }
}

impl HexColorMap {
    pub fn get_color(&self, key: &HexGridEntryState) -> Color {
        self.colors.get(key).cloned().unwrap_or(self.default_color)
    }

    pub fn prepare_materials(
        &mut self,
        assets: &mut Assets<ColorMaterial>,
    ) -> Handle<ColorMaterial> {
        for (k, v) in self.colors.iter() {
            self.materials.insert(*k, assets.add(*v));
        }
        self.path_debug_color = assets.add(PATH_DEBUG_COLOR);

        assets.add(self.default_color)
    }

    pub fn get_handle(&self, k: &HexGridEntryState) -> Option<Handle<ColorMaterial>> {
        self.materials.get(k).cloned()
    }

    pub fn debug_color(&self) -> Handle<ColorMaterial> {
        self.path_debug_color.clone()
    }
}

#[derive(Resource, Deref, DerefMut)]
pub struct DefaultHexMaterial(pub Handle<ColorMaterial>);
pub fn prepare_colors_materials(
    mut commands: Commands,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut colors: ResMut<HexColorMap>,
) {
    info!("Preparing colors");
    let default_material = colors.prepare_materials(&mut materials);
    commands.insert_resource(DefaultHexMaterial(default_material));
    info!("done...");
}

#[allow(clippy::too_many_arguments)]
pub fn init_grid(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    asset_server: Res<AssetServer>,
    cirumradius: Res<HexGridCirumRadius>,
    render_radius: Res<HexGridRenderRadius>,
    columns: Res<HexGridColumns>,
    rows: Res<HexGridRows>,
    default_color: Res<DefaultHexMaterial>,
    mut grid: ResMut<HexHashGrid>,
) {
    let font = asset_server.load(FONT);
    let text_font = TextFont {
        font,
        font_size: FONT_SIZE,
        ..Default::default()
    };
    let hexagon = meshes.add(RegularPolygon::new(**cirumradius, 6));

    for r in columns.get_actual_column_count() {
        let range = rows.get_actual_row_count(r);
        for q in range {
            let coords = GridIndex { q, r };
            let pos = coords.to_world_pos(**render_radius);
            grid[coords] = HexGridEntryState(GridEntry::None, GridEntryState::Normal);
            commands.spawn((
                Mesh2d(hexagon.clone()),
                MeshMaterial2d(default_color.clone()),
                Transform::from_xyz(pos.x, pos.y, 0.0),
                GridEntity(coords),
                Text2d(format!("q:{q}\nr:{r}")),
                text_font.clone(),
                TextColor(FONT_COLOR),
            ));
        }
    }
}

pub fn update_color(
    mut commands: Commands,
    world_pos: Res<MouseWorldPos>,
    size: Res<HexGridRenderRadius>,
    q_entries: Query<(Entity, &GridEntity, &mut MeshMaterial2d<ColorMaterial>)>,
    colors: Res<HexColorMap>,
    mut hex_grid: ResMut<HexHashGrid>,
    path: Res<HexPath<GridIndex>>,
    default_color: Res<DefaultHexMaterial>,
) {
    let access = world_pos.0.map(|p| GridIndex::from_world_pos(p, **size));

    if let Some(p) = access {
        for (entity, e, mut mat) in q_entries {
            if e.0 == path.start {
                commands.entity(entity).insert_if_new(PathStart(e.0));
            }
            if e.0 == path.end {
                commands.entity(entity).insert_if_new(PathEnd(e.0));
            }
            let state = &mut hex_grid[e.0];
            if e.0 != p {
                state.1 = GridEntryState::Normal;
            } else {
                state.1 = GridEntryState::Highlight;
            }
            let color_handle = colors.get_handle(state).unwrap_or(default_color.clone());
            **mat = color_handle;
        }
    } else {
        for (entity, e, mut mat) in q_entries {
            if e.0 == path.start {
                commands.entity(entity).insert_if_new(PathStart(e.0));
            }
            if e.0 == path.end {
                commands.entity(entity).insert_if_new(PathEnd(e.0));
            }
            let state = &mut hex_grid[e.0];
            state.1 = GridEntryState::Normal;
            let color_handle = colors.get_handle(state).unwrap_or(default_color.clone());
            **mat = color_handle;
        }
    }
}
pub fn axial_to_cube(vec: Vec2) -> Vec3 {
    Vec3::new(vec.x, vec.y, -vec.x - vec.y)
}

pub fn cube_to_axial(vec: Vec3) -> Vec2 {
    vec.xy()
}

pub fn axial_round(vec: Vec2) -> Vec2 {
    let v = axial_to_cube(vec);
    let r = cube_round(v);
    cube_to_axial(r)
}

pub fn cube_round(vec: Vec3) -> Vec3 {
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
#[derive(Component, Deref)]
pub struct PathStart(pub GridIndex);
#[derive(Component, Deref)]
pub struct PathEnd(pub GridIndex);
#[derive(Resource)]
pub struct HexHashGrid {
    data: HashMap<GridIndex, HexGridEntryState>,
}

#[derive(Component)]
#[require(Transform, Mesh2d, MeshMaterial2d<ColorMaterial>)]
pub struct GridEntity(pub GridIndex);

impl HexHashGrid {
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }
    pub fn from_path(path: &HexPath<GridIndex>) -> Self {
        Self {
            data: path
                .nodes
                .iter()
                .map(|a| {
                    (
                        *a,
                        HexGridEntryState(
                            if *a == path.start {
                                GridEntry::PathStart
                            } else if *a == path.end {
                                GridEntry::PathEnd
                            } else {
                                GridEntry::Path
                            },
                            GridEntryState::Normal,
                        ),
                    )
                })
                .collect(),
        }
    }

    pub fn contains(&self, index: &GridIndex) -> bool {
        self.data.contains_key(index)
    }

    pub fn keys(&self) -> impl Iterator<Item = GridIndex> {
        self.data.keys().cloned()
    }

    pub fn can_be_path(&self, a: &GridIndex) -> bool {
        if self.data.get(a).is_none() {
            return false;
        }
        self.data[a].0 == GridEntry::None || self.data[a].0 == GridEntry::Path
    }

    pub fn set_entry(&mut self, key: GridIndex, entry: GridEntry) {
        let old = &mut self[key];
        old.0 = entry;
    }

    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut HexGridEntryState> {
        self.data.values_mut()
    }
}

impl Default for HexHashGrid {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Resource, Deref, DerefMut, Clone, Copy)]
pub struct HexGridColumns(pub i32);
impl HexGridColumns {
    pub fn get_actual_column_count(&self) -> RangeInclusive<i32> {
        -self.0 / 2..=self.0 / 2
    }
}

#[derive(Resource, Deref, DerefMut, Clone, Copy)]
pub struct HexGridRows(pub i32);
impl HexGridRows {
    pub fn get_actual_row_count(&self, column: i32) -> RangeInclusive<i32> {
        let s = -self.0 + (-(column as f32) / 2.0).ceil() as i32;
        let e = self.0 + (-(column as f32) / 2.0).ceil() as i32;
        s..=e
    }
}
#[derive(Resource, DerefMut, Deref, Clone, Copy)]
pub struct HexGridColumnWidth(f32);
#[derive(Resource, DerefMut, Deref, Clone, Copy)]
pub struct HexGridWidth(f32);
#[derive(Resource, DerefMut, Deref, Clone, Copy)]
pub struct HexGridHeight(f32);
#[derive(Resource, DerefMut, Deref, Clone, Copy)]
pub struct HexGridCirumRadius(f32);
#[derive(Resource, DerefMut, Deref, Clone, Copy)]
pub struct HexGridRenderRadius(f32);

pub static AXIAL_CONVERT: LazyLock<Mat2> =
    LazyLock::new(|| Mat2::from_cols(Vec2::new(sqrt(3.0), 0.0), Vec2::new(sqrt(3.0) / 2.0, 1.5)));
pub static AXIAL_INVERTED: LazyLock<Mat2> = LazyLock::new(|| {
    Mat2::from_cols(
        Vec2::new(sqrt(3.0) / 3.0, 0.0),
        Vec2::new(-1.0 / 3.0, 2.0 / 3.0),
    )
});

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub struct GridIndex {
    pub q: i32,
    pub r: i32,
}

impl Add for GridIndex {
    type Output = GridIndex;

    fn add(self, rhs: Self) -> Self::Output {
        GridIndex::new(self.q + rhs.q, self.r + rhs.r)
    }
}

impl Sub for GridIndex {
    type Output = GridIndex;

    fn sub(self, rhs: Self) -> Self::Output {
        GridIndex::new(self.q - rhs.q, self.r - rhs.r)
    }
}

impl Mul<i32> for GridIndex {
    type Output = GridIndex;

    fn mul(self, rhs: i32) -> Self::Output {
        GridIndex::new(self.q * rhs, self.r * rhs)
    }
}

impl Div<i32> for GridIndex {
    type Output = GridIndex;

    fn div(self, rhs: i32) -> Self::Output {
        GridIndex::new(self.q / rhs, self.r / rhs)
    }
}

impl GridIndex {
    pub const fn new(q: i32, r: i32) -> Self {
        Self { q, r }
    }
    pub fn to_world_pos(&self, size: f32) -> Vec2 {
        let mat = *AXIAL_CONVERT;
        let vec = Vec2::new(self.q as f32, self.r as f32);
        mat.mul_vec2(vec) * size
    }

    pub fn from_cube_vec(vec: Vec3) -> Self {
        let rounded = cube_round(vec);

        GridIndex {
            q: rounded.x as i32,
            r: rounded.y as i32,
        }
    }

    pub fn from_axial_vec(vec: Vec2) -> Self {
        let rounded = axial_round(vec);

        GridIndex {
            q: rounded.x as i32,
            r: rounded.y as i32,
        }
    }

    pub fn from_world_pos(world_position: Vec2, size: f32) -> Self {
        let x = world_position.x / size;
        let y = world_position.y / size;
        let q = AXIAL_INVERTED.mul_vec2(Vec2::new(x, y));

        GridIndex::from_axial_vec(q)
    }
}

impl IndexMut<GridIndex> for HexHashGrid {
    fn index_mut(&mut self, index: GridIndex) -> &mut Self::Output {
        if self.data.get_mut(&index).is_none() {
            self.data
                .insert(index, (GridEntry::None, GridEntryState::Normal).into());
        }
        self.data.get_mut(&index).unwrap()
    }
}

impl Index<GridIndex> for HexHashGrid {
    type Output = HexGridEntryState;

    fn index(&self, index: GridIndex) -> &Self::Output {
        &self.data[&index]
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum GridEntry {
    None,
    Tower,
    Path,
    PathStart,
    PathEnd,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum GridEntryState {
    Highlight,
    Normal,
}

def_enum! {
    pub GridDirections => GridIndex {
        RIGHT => GridIndex::new(1,0),
        LEFT => GridIndex::new(-1,0),
        TOPLEFT => GridIndex::new(-1, 1),
        TOPRIGHT => GridIndex::new(0, 1),
        BOTTOMLEFT => GridIndex::new(0, -1),
        BOTTOMRIGHT => GridIndex::new(1, -1)
    }
}
