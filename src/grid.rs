use std::{
    ops::{Add, Div, Index, IndexMut, Mul, RangeInclusive, Sub},
    sync::LazyLock,
};

use bevy::{
    app::{Plugin, Startup},
    asset::{Assets, Handle},
    color::Color,
    ecs::{
        entity::Entity,
        hierarchy::ChildOf,
        observer::Trigger,
        schedule::IntoScheduleConfigs,
        system::{Commands, Query, Res, ResMut},
    },
    log::info,
    math::{Mat2, Vec2, Vec3, Vec3Swizzles, ops::sqrt, primitives::RegularPolygon},
    picking::{
        Pickable,
        events::{Out, Over, Pointer},
    },
    platform::collections::{HashMap, HashSet},
    prelude::*,
    render::mesh::{Mesh, Mesh2d},
    sprite::{ColorMaterial, ColorMaterialUniform, MeshMaterial2d},
    transform::components::Transform,
};

use crate::{
    assets::{
        DEFAULT_HEX_COLOR, HOVER_TINT_COLOR, PATH_COLOR, PATH_DEBUG_COLOR, PATH_END_COLOR,
        PATH_START_COLOR, TOWER_COST,
    },
    def_enum,
    enemy::{Enemy, EnemyMoved},
    path::HexPath,
    player::{Gold, Player},
    tower::{BaseTowerImage, spawn_tower_at},
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
        app.insert_resource(HexSpatialGrid::default());
        app.add_systems(
            Startup,
            (prepare_colors_materials, init_grid)
                .chain()
                .in_set(GridSet),
        );
        app.add_observer(on_enemy_moved);
        app.add_observer(on_enemy_removed);
        //app.add_systems(Update, update_color.in_set(GridSet));
    }
}

impl Default for GridPlugin {
    fn default() -> Self {
        Self {
            column_width: 120.0,
            row_width: 120.0,
            columns: 15,
            rows: 10,
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
//#[derive(Resource)]
//pub struct HexColorMap {
//    colors: HashMap<HexGridEntryState, Color>,
//    default_color: Color,
//    materials: HashMap<HexGridEntryState, Handle<ColorMaterial>>,
//    path_debug_color: Handle<ColorMaterial>,
//    hover_tint_color: Handle<ColorMaterial>,
//}
//
//impl Default for HexColorMap {
//    fn default() -> Self {
//        let default_color = Color::hsla(0.0, 1.0, 0.95, 1.0);
//
//        let mut map = HashMap::new();
//        let highlight_color = Color::hsla(0.5, 0.5, 0.3, 1.0);
//        map.insert(
//            HexGridEntryState(GridEntry::None, GridEntryState::Highlight),
//            highlight_color,
//        );
//        map.insert(
//            HexGridEntryState(GridEntry::Tower, GridEntryState::Normal),
//            Color::hsla(180.0, 0.5, 0.8, 1.0),
//        );
//        map.insert(
//            HexGridEntryState(GridEntry::Tower, GridEntryState::Highlight),
//            Color::hsla(180.0, 0.5, 0.8, 1.0),
//        );
//        map.insert(
//            HexGridEntryState(GridEntry::Path, GridEntryState::Normal),
//            Color::hsla(0.3, 0.5, 0.8, 1.0),
//        );
//        map.insert(
//            HexGridEntryState(GridEntry::Path, GridEntryState::Highlight),
//            Color::hsla(0.3, 0.5, 0.8, 1.0),
//        );
//        map.insert(
//            HexGridEntryState(GridEntry::PathStart, GridEntryState::Normal),
//            Color::hsla(0.8, 0.78, 0.3, 1.0),
//        );
//        map.insert(
//            HexGridEntryState(GridEntry::PathStart, GridEntryState::Highlight),
//            Color::hsla(0.8, 0.78, 0.3, 1.0),
//        );
//        map.insert(
//            HexGridEntryState(GridEntry::PathEnd, GridEntryState::Normal),
//            Color::hsla(0.8, 0.78, 0.4, 1.0),
//        );
//        map.insert(
//            HexGridEntryState(GridEntry::PathEnd, GridEntryState::Highlight),
//            Color::hsla(0.8, 0.78, 0.4, 1.0),
//        );
//        Self {
//            colors: map,
//            default_color,
//            materials: HashMap::new(),
//            path_debug_color: Handle::default(),
//            hover_tint_color: Handle::default(),
//        }
//    }
//}
//
//impl HexColorMap {
//    pub fn get_color(&self, key: &HexGridEntryState) -> Color {
//        self.colors.get(key).cloned().unwrap_or(self.default_color)
//    }
//    pub fn prepare_materials(
//        &mut self,
//        assets: &mut Assets<ColorMaterial>,
//    ) -> Handle<ColorMaterial> {
//        for (k, v) in self.colors.iter() {
//            self.materials.insert(*k, assets.add(*v));
//        }
//        self.path_debug_color = assets.add(PATH_DEBUG_COLOR);
//        self.hover_tint_color = assets.add(HOVER_TINT_COLOR);
//
//        assets.add(self.default_color)
//    }
//
//    pub fn get_handle(&self, k: &HexGridEntryState) -> Option<Handle<ColorMaterial>> {
//        self.materials.get(k).cloned()
//    }
//
//    pub fn debug_color(&self) -> Handle<ColorMaterial> {
//        self.path_debug_color.clone()
//    }
//}

#[derive(Resource, Deref, DerefMut)]
pub struct DefaultHexMaterial(pub Handle<ColorMaterial>);
#[derive(Resource, Deref, DerefMut)]
pub struct PathStartMaterial(pub Handle<ColorMaterial>);
#[derive(Resource, Deref, DerefMut)]
pub struct PathEndMaterial(pub Handle<ColorMaterial>);
#[derive(Resource, Deref, DerefMut)]
pub struct PathMaterial(pub Handle<ColorMaterial>);

#[derive(Resource, Deref, DerefMut)]
pub struct HoverTintMaterial(pub Handle<ColorMaterial>);
#[derive(Resource, Deref, DerefMut)]
pub struct Hexagon(pub Handle<Mesh>);

pub fn prepare_colors_materials(
    mut commands: Commands,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    info!("Preparing colors");
    let default_material = materials.add(DEFAULT_HEX_COLOR);
    let hover_tint_color = materials.add(HOVER_TINT_COLOR);
    let path_start_material = materials.add(PATH_START_COLOR);
    let path_end_material = materials.add(PATH_END_COLOR);
    let path_material = materials.add(PATH_COLOR);
    commands.insert_resource(DefaultHexMaterial(default_material));
    commands.insert_resource(HoverTintMaterial(hover_tint_color));
    commands.insert_resource(PathStartMaterial(path_start_material));
    commands.insert_resource(PathEndMaterial(path_end_material));
    commands.insert_resource(PathMaterial(path_material));
    info!("done...");
}

#[allow(clippy::too_many_arguments)]
pub fn init_grid(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    //asset_server: Res<AssetServer>,
    cirumradius: Res<HexGridCirumRadius>,
    render_radius: Res<HexGridRenderRadius>,
    columns: Res<HexGridColumns>,
    rows: Res<HexGridRows>,
    default_color: Res<DefaultHexMaterial>,
) {
    //let font = asset_server.load(FONT);
    //let text_font = TextFont {
    //    font,
    //    font_size: FONT_SIZE,
    //    ..Default::default()
    //};
    let hexagon = meshes.add(RegularPolygon::new(**cirumradius, 6));
    commands.insert_resource(Hexagon(hexagon.clone()));
    let grid = HexHashGrid::from_rows_and_columns_with_init(&columns, &rows, |coords| {
        let pos = coords.to_world_pos(**render_radius);
        commands
            .spawn((
                Mesh2d(hexagon.clone()),
                MeshMaterial2d(default_color.clone()),
                Transform::from_xyz(pos.x, pos.y, 0.0),
                GridEntity(coords),
                Pickable::default(),
            ))
            .observe(on_hex_hover)
            .observe(on_hex_out)
            .observe(on_hex_click);
    });
    commands.insert_resource(grid);
}

#[derive(Component)]
pub struct Hover;

pub fn on_hex_hover(
    trigger: Trigger<Pointer<Over>>,
    mut commands: Commands,
    hexagon: Res<Hexagon>,
    hover_tint: Res<HoverTintMaterial>,
) {
    let e = commands
        .spawn((
            Hover,
            Mesh2d(hexagon.0.clone()),
            MeshMaterial2d(hover_tint.0.clone()),
            Transform::IDENTITY,
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(trigger.target).add_child(e);
}

pub fn on_hex_out(
    trigger: Trigger<Pointer<Out>>,
    mut commands: Commands,
    query: Query<(Entity, &ChildOf), With<Hover>>,
) {
    for q in query {
        if q.1.parent() == trigger.target {
            commands.entity(q.0).despawn();
        }
    }
}

pub fn on_hex_click(
    mut trigger: Trigger<Pointer<Click>>,
    commands: Commands,
    base_tower_image: Res<BaseTowerImage>,
    mut hex_grid: ResMut<HexHashGrid>,
    mut player_gold: Single<&mut Gold, With<Player>>,
    grid_query: Query<&GridEntity>,
) {
    if let Ok(index) = grid_query.get(trigger.target)
        && hex_grid[index.0] == GridEntry::None
        && spawn_tower_at(
            trigger.target,
            commands,
            base_tower_image,
            &Gold(TOWER_COST),
            &mut player_gold,
        )
    {
        info!("set tower: {:?}", index.0);
        hex_grid[index.0] = GridEntry::Tower;
    }
    trigger.propagate(true);
}

//#[allow(clippy::too_many_arguments)]
//pub fn update_color(
//    mut commands: Commands,
//    world_pos: Res<MouseWorldPos>,
//    size: Res<HexGridRenderRadius>,
//    q_entries: Query<(Entity, &GridEntity, &mut MeshMaterial2d<ColorMaterial>)>,
//    colors: Res<HexColorMap>,
//    mut hex_grid: ResMut<HexHashGrid>,
//    path: Res<HexPath<GridIndex>>,
//    default_color: Res<DefaultHexMaterial>,
//) {
//    let access = world_pos.0.map(|p| GridIndex::from_world_pos(p, **size));
//
//    if let Some(p) = access {
//        for (entity, e, mut mat) in q_entries {
//            if e.0 == path.start {
//                commands.entity(entity).insert_if_new(PathStart(e.0));
//            }
//            if e.0 == path.end {
//                commands.entity(entity).insert_if_new(PathEnd(e.0));
//            }
//            let state = &mut hex_grid[e.0];
//            if e.0 != p {
//                state.1 = GridEntryState::Normal;
//            } else {
//                state.1 = GridEntryState::Highlight;
//            }
//            let color_handle = colors.get_handle(state).unwrap_or(default_color.clone());
//            **mat = color_handle;
//        }
//    } else {
//        for (entity, e, mut mat) in q_entries {
//            if e.0 == path.start {
//                commands.entity(entity).insert_if_new(PathStart(e.0));
//            }
//            if e.0 == path.end {
//                commands.entity(entity).insert_if_new(PathEnd(e.0));
//            }
//            let state = &mut hex_grid[e.0];
//            state.1 = GridEntryState::Normal;
//            let color_handle = colors.get_handle(state).unwrap_or(default_color.clone());
//            **mat = color_handle;
//        }
//    }
//}
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
#[derive(Component)]
pub struct PathStart;
#[derive(Component)]
pub struct PathEnd;
#[derive(Component)]
pub struct Path;
#[derive(Resource)]
pub struct HexHashGrid {
    data: HashMap<GridIndex, GridEntry>,
}
#[derive(Resource, Default)]
pub struct HexSpatialGrid {
    data: HashMap<GridIndex, HashSet<Entity>>,
    entries: HashMap<Entity, GridIndex>,
}

impl HexSpatialGrid {
    pub fn update(&mut self, index: GridIndex, entity: Entity) {
        self.entries
            .entry(entity)
            .and_modify(|i| {
                self.data.entry(*i).and_modify(|s| {
                    s.remove(&entity);
                });
                *i = index;
            })
            .or_insert(index);

        self.data
            .entry(index)
            .and_modify(|s| {
                s.insert(entity);
            })
            .or_insert_with(|| {
                let mut hs = HashSet::new();
                hs.insert(entity);
                hs
            });
    }

    pub fn get_nearby(&mut self, index: &GridIndex) -> impl Iterator<Item = Entity> {
        let directions = GridDirections::VARIANTS.iter().map(|d| d.get());

        let mut initial_set = self.data.entry(*index).or_default().clone();
        for d in directions {
            initial_set.extend(self.data.entry(*index + d).or_default().iter().cloned());
        }
        initial_set.into_iter()
    }

    pub fn remove(&mut self, entity: Entity) {
        let Some(position) = self.entries.remove(&entity) else {
            return;
        };

        self.data.entry(position).and_modify(|s| {
            s.remove(&entity);
        });
    }
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
                        if *a == path.start {
                            GridEntry::PathStart
                        } else if *a == path.end {
                            GridEntry::PathEnd
                        } else {
                            GridEntry::Path
                        },
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
        self.data[a] == GridEntry::None || self.data[a] == GridEntry::Path
    }

    pub fn set_entry(&mut self, key: GridIndex, entry: GridEntry) {
        let old = &mut self[key];
        *old = entry;
    }
    pub fn clear_path(&mut self) {
        self.data.values_mut().for_each(|e| {
            if *e == GridEntry::Path || *e == GridEntry::PathStart || *e == GridEntry::PathEnd {
                *e = GridEntry::None
            }
        });
    }

    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut GridEntry> {
        self.data.values_mut()
    }

    pub fn from_rows_and_columns_with_init(
        columns: &HexGridColumns,
        rows: &HexGridRows,
        mut on_entry: impl FnMut(GridIndex),
    ) -> Self {
        let mut s = Self::new();
        for r in columns.get_actual_column_count() {
            let range = rows.get_actual_row_count(r);
            for q in range {
                let coords = GridIndex { q, r };
                on_entry(coords);
                s[coords] = GridEntry::None;
            }
        }
        s
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
            self.data.insert(index, GridEntry::None);
        }
        self.data.get_mut(&index).unwrap()
    }
}

impl Index<GridIndex> for HexHashGrid {
    type Output = GridEntry;

    fn index(&self, index: GridIndex) -> &Self::Output {
        &self.data[&index]
    }
}

fn on_enemy_moved(
    mut trigger: Trigger<EnemyMoved>,
    mut spatial_grid: ResMut<HexSpatialGrid>,
    size: Res<HexGridRenderRadius>,
) {
    let index = GridIndex::from_world_pos(trigger.event().position, **size);
    spatial_grid.update(index, trigger.event().entity);
    trigger.propagate(true);
}

fn on_enemy_removed(trigger: Trigger<OnRemove, Enemy>, mut spatial_grid: ResMut<HexSpatialGrid>) {
    spatial_grid.remove(trigger.target());
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
