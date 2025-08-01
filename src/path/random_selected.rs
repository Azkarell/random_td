use bevy::{
    log::info,
    platform::collections::{HashMap, HashSet},
};
use rand::{random_range, rng, seq::IteratorRandom};

use crate::grid::{GridDirections, GridIndex};

use super::{
    HexPath, SinglePathAlgorithm,
    dijkstra::{Dijkstra, DistanceFunction, MutTileStateCache, TileStateCache},
    resolver::ShortesPathResolver,
};

pub struct TotalRandom;

impl SinglePathAlgorithm for TotalRandom {
    fn calculate_path(
        &self,
        context: super::context::PathContext<'_>,
        start: crate::grid::GridIndex,
        end: crate::grid::GridIndex,
    ) -> Option<super::HexPath<GridIndex>> {
        let mut rng = rng();
        let mut cur = Some(start);
        let mut path = vec![];
        let mut visited = HashSet::new();

        while let Some(c) = cur {
            path.push(c);
            visited.insert(start);
            if c == end {
                return Some(HexPath {
                    nodes: path,
                    start,
                    end,
                });
            }

            cur = GridDirections::VARIANTS
                .iter()
                .map(|d| c + d.get())
                .filter(|d| !visited.contains(d) && context.can_be_path(d))
                .choose(&mut rng);
        }

        None
    }
}

pub struct WorldDistance {
    size: f32,
}

impl DistanceFunction<GridIndex, f32> for WorldDistance {
    fn get_distance(&self, rhs: &GridIndex, lhs: &GridIndex) -> f32 {
        (rhs.to_world_pos(self.size) - lhs.to_world_pos(self.size)).length()
    }
}
pub struct RandomDijkstra {
    pub tile_size: f32,
}

pub fn try_get_path<TS: TileStateCache<GridIndex>>(
    tile_state: &TS,
    dijkstra: &Dijkstra,
    start: GridIndex,
    end: GridIndex,
    tile_size: f32,
) -> Option<HexPath<GridIndex>> {
    let mut distances: HashMap<GridIndex, f32> = tile_state.get_initial_distances(&start);
    let mut prevs: HashMap<GridIndex, GridIndex> = HashMap::new();
    let fun = &WorldDistance { size: tile_size };
    let mut data = dijkstra.create_data(
        &mut prevs,
        &mut distances,
        &ShortesPathResolver,
        tile_state,
        fun,
    );

    data.run(start, end, GridDirections::VARIANTS.iter().map(|i| i.get()))
}

impl SinglePathAlgorithm for RandomDijkstra {
    fn calculate_path(
        &self,
        context: super::context::PathContext<'_>,
        start: crate::grid::GridIndex,
        end: crate::grid::GridIndex,
    ) -> Option<HexPath<GridIndex>> {
        let mut rng = rng();
        let mut path = vec![];
        let mut tile_state = context.tile_state(start, end);

        let dijkstra = Dijkstra;
        let num_points = random_range(1..=3usize);
        let mut c_start = start;
        let mut i = 0;
        let upper_boundary = 100;
        let mut tries = 0;
        while i <= num_points {
            let c_end = if i == num_points {
                end
            } else {
                tile_state.get_random_unoccupied(&mut rng)?
            };

            let hex_path = try_get_path(&tile_state, &dijkstra, c_start, c_end, self.tile_size);
            if let Some(p) = hex_path {
                for n in &p.nodes {
                    if *n != c_end {
                        tile_state.set_state(n, super::dijkstra::TileState::Blocked);
                        path.push(*n);
                    }
                }
                c_start = c_end;
                i += 1;
            } else {
                for p in path {
                    tile_state.set_state(&p, crate::path::dijkstra::TileState::Useable);
                }
                info!("failed to connect start end:{i}");
                // if we run into an issue reset and try again;
                if tries > upper_boundary {
                    return None;
                }
                i = 0;
                path = vec![];
                c_start = start;
                tries += 1;
                continue;
            }
        }
        path.push(end);
        Some(HexPath {
            nodes: path,
            start,
            end,
        })
    }
}

#[cfg(test)]
mod tests {

    use crate::{
        grid::{HexGridColumns, HexGridRows, HexHashGrid},
        path::{SinglePathAlgorithm, context::PathContext},
    };

    use super::*;

    fn hex_column() -> HexGridColumns {
        HexGridColumns(15)
    }
    fn hex_rows() -> HexGridRows {
        HexGridRows(10)
    }
    fn create_test_data() -> HexHashGrid {
        HexHashGrid::from_rows_and_columns_with_init(&hex_column(), &hex_rows(), |_| {})
    }

    #[test]
    fn random_dijkstra_should_work() {
        let dijkstra = RandomDijkstra { tile_size: 50.0 };
        let grid = create_test_data();
        let column = hex_column();
        let rows = hex_rows();
        let context = PathContext::from_args(&rows, &column, &grid);
        let start = GridIndex { q: -12, r: 4 };
        let end = GridIndex { q: 13, r: -5 };

        let path = dijkstra.calculate_path(context, start, end);
        assert!(path.is_some());
    }

    #[test]
    fn dijkstra_data_should_work() {
        let dijkstra = Dijkstra;
        let grid = create_test_data();
        let column = hex_column();
        let rows = hex_rows();
        let context = PathContext::from_args(&rows, &column, &grid);

        let mut prevs = HashMap::new();
        let start = context.iter_start_column().next().unwrap();
        let end = context.iter_end_column().last().unwrap();

        let tile_state = context.tile_state(start, end);

        let mut distances =
            tile_state.get_initial_distances::<f32, HashMap<GridIndex, f32>>(&start);
        let resolver = ShortesPathResolver;
        let mut data = dijkstra.create_data(
            &mut prevs,
            &mut distances,
            &resolver,
            &tile_state,
            &WorldDistance { size: 50.0 },
        );
        let dirs = GridDirections::VARIANTS.iter().map(|e| e.get());
        let path = data.run(start, end, dirs);
        assert!(path.is_some());
    }
}
