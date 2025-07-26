use bevy::{log::info, platform::collections::HashMap, winit::accessibility};
use rand::{Rng, seq::IteratorRandom};

use crate::grid::{GridDirections, GridIndex};

use super::{
    DistanceAwareSinglePathAlgorithm, HexPath,
    context::{
        Cache, CacheBehaviour, CacheUpdateResult, DistanceCache, IgnoreMissingEntries,
        InsertMissingEntries, PathContext,
    },
    resolver::{Resolver, ShortesPathResolver},
};

pub struct Dijkstra;
impl Dijkstra {
    pub fn create_data<
        'a,
        C: Cache<InsertMissingEntries, Access = GridIndex, Output = GridIndex>,
        D: DistanceCache<Access = GridIndex, Output = u32>,
        R: Resolver<InsertMissingEntries, C>,
        TS: TileStateCache,
    >(
        &'a self,
        prevs: &'a mut C,
        distances: &'a mut D,
        resolver: &'a R,
        tile_state: &'a TS,
    ) -> DijkstraData<'a, D, C, R, TS> {
        DijkstraData {
            distances,
            prevs,
            resolver,
            tile_state,
        }
    }
}
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum TileState {
    Blocked,
    Useable,
}

pub trait TileStateCache {
    fn get_random_unoccupied<R: Rng>(&self, rng: &mut R) -> Option<GridIndex>;
    fn is_blocked(&self, access: &GridIndex) -> bool;
    fn get_initial_distances<
        C: DistanceCache<Access = GridIndex, Output = u32> + FromIterator<(GridIndex, u32)>,
    >(
        &self,
        start: &GridIndex,
    ) -> C;
}

impl<C: Cache<InsertMissingEntries, Access = GridIndex, Output = TileState>> TileStateCache for C {
    fn is_blocked(&self, access: &GridIndex) -> bool {
        self.get(access)
            .map(|ts| *ts == TileState::Blocked)
            .unwrap_or(false)
    }

    fn get_initial_distances<
        D: DistanceCache<Access = GridIndex, Output = u32> + FromIterator<(GridIndex, u32)>,
    >(
        &self,
        start: &GridIndex,
    ) -> D {
        self.iter()
            .filter(|(i, ts)| **ts != TileState::Blocked)
            .map(|(i, _)| (*i, if i == start { 0 } else { u32::MAX }))
            .collect::<D>()
    }

    fn get_random_unoccupied<R: Rng>(&self, rng: &mut R) -> Option<GridIndex> {
        self.iter()
            .filter(|e| *e.1 == TileState::Useable)
            .map(|e| *e.0)
            .choose(rng)
    }
}

pub trait MutTileStateCache: TileStateCache {
    fn set_is_blocked(&mut self, access: &GridIndex);
}

impl<C: Cache<InsertMissingEntries, Access = GridIndex, Output = TileState> + TileStateCache>
    MutTileStateCache for C
{
    fn set_is_blocked(&mut self, access: &GridIndex) {
        self.update(access, TileState::Blocked, |_| true);
    }
}

pub struct DijkstraData<
    'a,
    D: DistanceCache<Access = GridIndex, Output = u32>,
    C: Cache<InsertMissingEntries, Access = GridIndex, Output = GridIndex>,
    R: Resolver<InsertMissingEntries, C>,
    TS: TileStateCache,
> {
    distances: &'a mut D,
    prevs: &'a mut C,
    resolver: &'a R,
    tile_state: &'a TS,
}

impl<
    'a,
    D: DistanceCache<Access = GridIndex, Output = u32>,
    C: Cache<InsertMissingEntries, Access = GridIndex, Output = GridIndex>,
    R: Resolver<InsertMissingEntries, C>,
    TS: TileStateCache,
> DijkstraData<'a, D, C, R, TS>
{
    pub fn run(&mut self, start: GridIndex, end: GridIndex) -> Option<HexPath> {
        info!("finding path from {start:?} to {end:?}");
        self.calcualte_distances();
        self.resolver.resolve_path(self.prevs, start, end)
    }

    pub fn calcualte_distances(&mut self) {
        while let Some((min, cur_d)) = get_min(self.distances) {
            if cur_d == u32::MAX {
                info!(
                    "{:?}",
                    self.distances
                        .iter()
                        .filter(|(i, d)| **d != u32::MAX)
                        .collect::<Vec<(&GridIndex, &u32)>>()
                );
                info!("reached max");
                return;
            }
            if !self.can_be_path(&min) {
                continue;
            }

            for d in GridDirections::VARIANTS {
                let neighbor = min + d.get();
                if self.can_be_path(&neighbor)
                    && self.distances.update_distance(&neighbor, cur_d + 1)
                        == CacheUpdateResult::Updated
                {
                    info!("updating {neighbor:?}");
                    self.prevs.update(&neighbor, min, |old| *old != min);
                }
            }
        }
    }
    fn can_be_path(&self, access: &GridIndex) -> bool {
        !self.tile_state.is_blocked(access)
    }
}

impl DistanceAwareSinglePathAlgorithm for Dijkstra {
    fn calculate_path_distance_aware<D: DistanceCache<Access = GridIndex, Output = u32>>(
        &self,
        context: PathContext<'_>,
        mut distances: D,
        start: GridIndex,
        end: GridIndex,
    ) -> Option<super::HexPath> {
        let mut prevs: HashMap<GridIndex, GridIndex> = HashMap::new();
        let ts = context.tile_state(start, end);
        let mut data = self.create_data(&mut prevs, &mut distances, &ShortesPathResolver, &ts);
        data.run(start, end)
    }
}

fn get_min<D: DistanceCache<Access = GridIndex, Output = u32>>(
    distances: &mut D,
) -> Option<(GridIndex, u32)> {
    let min = distances.get_min();
    if let Some(m) = min {
        distances.remove(&m.0);
        Some(m)
    } else {
        None
    }
}
