pub mod chiseled;
pub mod context;
pub mod dijkstra;
pub mod random;
pub mod random_selected;
pub mod resolver;
pub mod steps;

use crate::grid::GridIndex;
use bevy::{
    app::Plugin,
    asset::{Assets, Handle},
    ecs::{
        component::Component,
        entity::Entity,
        query::Added,
        schedule::SystemSet,
        system::{Commands, Query, ResMut},
    },
    log::info,
    math::{Vec2, primitives::Rectangle},
    platform::collections::HashMap,
    render::mesh::{Mesh, Mesh2d, Meshable},
    sprite::{ColorMaterial, MeshMaterial2d},
    transform::components::Transform,
};
use context::{DistanceCache, PathContext};
use random::RandomSelector;

#[derive(Component, Debug)]
pub struct PathSegment {
    pub start: Vec2,
    pub end: Vec2,
    pub color: Handle<ColorMaterial>,
}

pub struct PathPlugin;

impl Plugin for PathPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        // app.add_systems(Update, update_segments.in_set(PathSet));
    }
}
#[derive(SystemSet, PartialEq, Eq, Clone, Copy, Hash, Debug)]
pub struct PathSet;

fn update_segments(
    mut commands: Commands,
    query: Query<(Entity, &PathSegment), Added<PathSegment>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    for (e, p) in query {
        let square = Rectangle::new((p.start.x - p.end.x).abs(), (p.start.y - p.end.y).abs());
        let m = meshes.add(square.mesh());
        info!("inserting {p:?}");
        commands.entity(e).insert((
            Mesh2d(m),
            MeshMaterial2d(p.color.clone()),
            Transform::from_xyz(p.start.x, p.start.y, 20.0),
        ));
    }
}

#[derive(Debug)]
pub struct HexPath {
    pub nodes: Vec<GridIndex>,
    pub start: GridIndex,
    pub end: GridIndex,
}

pub trait StartSelector {
    fn get_start(&self, context: PathContext<'_>) -> Option<GridIndex>;
}
pub trait EndSelector {
    fn get_end(&self, context: PathContext<'_>) -> Option<GridIndex>;
}

pub trait SinglePathAlgorithm {
    fn calculate_path(
        &self,
        context: PathContext<'_>,
        start: GridIndex,
        end: GridIndex,
    ) -> Option<HexPath>;
}

pub trait SinglePathFinder<S: StartSelector, E: EndSelector, A: SinglePathAlgorithm> {
    fn get_path(&self, context: PathContext<'_>) -> Option<HexPath>;
}
pub trait DistanceAwareSinglePathAlgorithm {
    fn calculate_path_distance_aware<D: DistanceCache<Access = GridIndex, Output = u32>>(
        &self,
        context: PathContext<'_>,
        distance_cache: D,
        start: GridIndex,
        end: GridIndex,
    ) -> Option<HexPath>;
}

impl<D: DistanceAwareSinglePathAlgorithm> SinglePathAlgorithm for D {
    fn calculate_path(
        &self,
        context: PathContext<'_>,
        start: GridIndex,
        end: GridIndex,
    ) -> Option<HexPath> {
        let d: HashMap<GridIndex, u32> = context
            .all_pathable()
            .map(|a| if a == start { (a, 0) } else { (a, u32::MAX) })
            .collect();
        self.calculate_path_distance_aware(context, d, start, end)
    }
}

pub struct DefaultSinglePathFinder<S: StartSelector, E: EndSelector, A: SinglePathAlgorithm> {
    s: S,
    e: E,
    a: A,
}
impl<A: SinglePathAlgorithm> DefaultSinglePathFinder<RandomSelector, RandomSelector, A> {
    pub fn new(algo: A) -> Self {
        Self {
            s: RandomSelector,
            e: RandomSelector,
            a: algo,
        }
    }
}

impl<S: StartSelector, E: EndSelector, A: SinglePathAlgorithm> SinglePathFinder<S, E, A>
    for DefaultSinglePathFinder<S, E, A>
{
    fn get_path(&self, context: PathContext<'_>) -> Option<HexPath> {
        let start = self.s.get_start(context);
        let end = self.e.get_end(context);
        if start.is_none() || end.is_none() {
            None
        } else {
            self.a.calculate_path(context, start.unwrap(), end.unwrap())
        }
    }
}
