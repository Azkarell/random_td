use std::{
    hash::Hash,
    marker::PhantomData,
    ops::{Add, Sub},
};

use bevy::platform::collections::HashMap;
use rand::{Rng, seq::IteratorRandom};

use crate::grid::{GridDirections, GridIndex};

use super::{
    DistanceAwareSinglePathAlgorithm, HexPath,
    context::{Cache, CacheUpdateResult, DistanceCache, InsertMissingEntries, PathContext},
    resolver::{Resolver, ShortesPathResolver},
};

pub trait DistanceValue: PartialOrd + PartialEq + Clone + Copy + Add<Output = Self> + Sub {
    fn max() -> Self;
    fn zero() -> Self;
    fn one() -> Self;
}

impl DistanceValue for u32 {
    fn max() -> Self {
        Self::MAX
    }

    fn zero() -> Self {
        0
    }

    fn one() -> Self {
        1
    }
}

impl DistanceValue for f32 {
    fn max() -> Self {
        f32::MAX
    }

    fn zero() -> Self {
        0.0
    }

    fn one() -> Self {
        1.0
    }
}

pub trait DistanceFunction<I, O> {
    fn get_distance(&self, rhs: &I, lhs: &I) -> O;
}

pub struct Dijkstra;

impl Dijkstra {
    pub fn create_data<
        'a,
        I: Indexable,
        DF: DistanceFunction<D::Access, D::Output>,
        DT: DistanceValue,
        C: Cache<InsertMissingEntries, Access = I, Output = I>,
        D: DistanceCache<Access = I, Output = DT>,
        R: Resolver<I, InsertMissingEntries, C>,
        TS: TileStateCache<I>,
    >(
        &'a self,
        prevs: &'a mut C,
        distances: &'a mut D,
        resolver: &'a R,
        tile_state: &'a TS,
        distance_function: &'a DF,
    ) -> DijkstraData<'a, I, DF, DT, D, C, R, TS> {
        DijkstraData {
            distances,
            prevs,
            resolver,
            tile_state,
            distance_function,
            _pd: PhantomData,
        }
    }
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum TileState {
    Blocked,
    Useable,
}

pub trait Indexable: Hash + Copy + Clone + Eq + PartialEq + Add<Output = Self> {}
impl<C: Hash + Copy + Clone + Eq + PartialEq + Add<Output = Self>> Indexable for C {}

pub trait TileStateCache<Index: Indexable> {
    fn get_random_unoccupied<R: Rng>(&self, rng: &mut R) -> Option<Index>;
    fn is_blocked(&self, access: &Index) -> bool;
    fn get_initial_distances<
        Distance: DistanceValue,
        C: DistanceCache<Access = Index, Output = Distance> + FromIterator<(Index, Distance)>,
    >(
        &self,
        start: &Index,
    ) -> C;
}

impl<Index: Indexable, C: Cache<InsertMissingEntries, Access = Index, Output = TileState>>
    TileStateCache<Index> for C
{
    fn is_blocked(&self, access: &Index) -> bool {
        self.get(access)
            .map(|ts| *ts == TileState::Blocked)
            .unwrap_or(false)
    }

    fn get_initial_distances<
        Distance: DistanceValue,
        D: DistanceCache<Access = Index, Output = Distance> + FromIterator<(Index, Distance)>,
    >(
        &self,
        start: &Index,
    ) -> D {
        self.iter()
            .filter(|(_i, ts)| **ts != TileState::Blocked)
            .map(|(i, _)| {
                (
                    *i,
                    if i == start {
                        Distance::zero()
                    } else {
                        Distance::max()
                    },
                )
            })
            .collect::<D>()
    }

    fn get_random_unoccupied<R: Rng>(&self, rng: &mut R) -> Option<Index> {
        self.iter()
            .filter(|e| *e.1 == TileState::Useable)
            .map(|e| *e.0)
            .choose(rng)
    }
}

pub trait MutTileStateCache<Index: Indexable>: TileStateCache<Index> {
    fn set_is_blocked(&mut self, access: &GridIndex);
}

impl<
    Index: Indexable,
    C: Cache<InsertMissingEntries, Access = GridIndex, Output = TileState> + TileStateCache<Index>,
> MutTileStateCache<Index> for C
{
    fn set_is_blocked(&mut self, access: &GridIndex) {
        self.update(access, TileState::Blocked, |_| true);
    }
}

pub struct DijkstraData<
    'a,
    I: Indexable,
    DF: DistanceFunction<D::Access, D::Output>,
    DT: DistanceValue,
    D: DistanceCache<Access = I, Output = DT>,
    C: Cache<InsertMissingEntries, Access = I, Output = I>,
    R: Resolver<I, InsertMissingEntries, C>,
    TS: TileStateCache<I>,
> {
    distances: &'a mut D,
    prevs: &'a mut C,
    resolver: &'a R,
    tile_state: &'a TS,
    _pd: PhantomData<DT>,
    distance_function: &'a DF,
}

impl<
    'a,
    I: Indexable,
    DF: DistanceFunction<D::Access, D::Output>,
    DT: DistanceValue,
    D: DistanceCache<Access = I, Output = DT>,
    C: Cache<InsertMissingEntries, Access = I, Output = I>,
    R: Resolver<I, InsertMissingEntries, C>,
    TS: TileStateCache<I>,
> DijkstraData<'a, I, DF, DT, D, C, R, TS>
{
    pub fn run<DIRS: IntoIterator<Item = I> + Clone>(
        &mut self,
        start: I,
        end: I,
        dirs: DIRS,
    ) -> Option<HexPath<I>> {
        self.calcualte_distances(dirs, start, end);
        self.resolver.resolve_path(self.prevs, start, end)
    }

    pub fn calcualte_distances<DIRS: IntoIterator<Item = I> + Clone>(
        &mut self,
        dirs: DIRS,
        _start: I,
        _end: I,
    ) {
        while let Some((min, cur_d)) = get_min(self.distances) {
            if cur_d == DT::max() {
                return;
            }
            if !self.can_be_path(&min) {
                continue;
            }
            for d in dirs.clone().into_iter() {
                let neighbor = min + d;

                let distance = self.distance_function.get_distance(&neighbor, &min);
                if self.can_be_path(&neighbor)
                    && self.distances.update_distance(&neighbor, distance + cur_d)
                        == CacheUpdateResult::Updated
                {
                    self.prevs.update(&neighbor, min, |old| *old != min);
                }
            }
        }
    }
    fn can_be_path(&self, access: &I) -> bool {
        !self.tile_state.is_blocked(access)
    }
}

pub struct ConstOneDF;

impl<V: DistanceValue> DistanceFunction<GridIndex, V> for ConstOneDF {
    fn get_distance(&self, _rhs: &GridIndex, _lhs: &GridIndex) -> V {
        V::one()
    }
}

impl DistanceAwareSinglePathAlgorithm for Dijkstra {
    fn calculate_path_distance_aware<D: DistanceCache<Access = GridIndex>>(
        &self,
        context: PathContext<'_>,
        mut distances: D,
        start: GridIndex,
        end: GridIndex,
    ) -> Option<super::HexPath<GridIndex>>
    where
        D::Output: DistanceValue,
    {
        let mut prevs: HashMap<GridIndex, GridIndex> = HashMap::new();
        let ts = context.tile_state(start, end);
        let mut data = self.create_data(
            &mut prevs,
            &mut distances,
            &ShortesPathResolver,
            &ts,
            &ConstOneDF,
        );
        let dirs = GridDirections::VARIANTS.iter().map(|i| i.get());
        data.run(start, end, dirs)
    }
}

fn get_min<I: Indexable, DT: DistanceValue, D: DistanceCache<Access = I, Output = DT>>(
    distances: &mut D,
) -> Option<(I, DT)> {
    let min = distances.get_min();
    if let Some(m) = min {
        distances.remove(&m.0);
        Some(m)
    } else {
        None
    }
}
