use std::num;

use bevy::{
    log::info,
    platform::collections::{HashMap, HashSet},
};
use rand::{random_range, rng, seq::IteratorRandom};

use crate::grid::{GridDirections, GridIndex};

use super::{
    HexPath, SinglePathAlgorithm,
    context::Cache,
    dijkstra::{Dijkstra, MutTileStateCache, TileStateCache},
    resolver::ShortesPathResolver,
};

pub struct TotalRandom;

impl SinglePathAlgorithm for TotalRandom {
    fn calculate_path(
        &self,
        context: super::context::PathContext<'_>,
        start: crate::grid::GridIndex,
        end: crate::grid::GridIndex,
    ) -> Option<super::HexPath> {
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

pub struct RandomDijkstra;
impl SinglePathAlgorithm for RandomDijkstra {
    fn calculate_path(
        &self,
        context: super::context::PathContext<'_>,
        start: crate::grid::GridIndex,
        end: crate::grid::GridIndex,
    ) -> Option<HexPath> {
        let mut rng = rng();
        let mut path = vec![];
        let mut tile_state = context.tile_state(start, end);

        // let points = context
        //     .all()
        //     .filter(|i| context.can_be_path(i) && *i != start && *i != end)
        //     .choose_multiple(&mut rng, rand::random_range(2usize..=5usize));
        let dijkstra = Dijkstra;
        // let mut tmp = vec![start];
        // tmp.extend_from_slice(&points);
        // tmp.push(end);
        //tmp.sort_by(|l, r| l.q.cmp(&r.q));
        let num_points = random_range(1..=3usize);
        let mut c_start = start;
        for i in 0..=num_points {
            let Some(mut c_end) = tile_state.get_random_unoccupied(&mut rng) else {
                continue;
            };
            if i == num_points {
                c_end = end;
            }
            let mut distances: HashMap<GridIndex, u32> = tile_state.get_initial_distances(&c_start);
            let mut prevs: HashMap<GridIndex, GridIndex> = HashMap::new();
            let mut data = dijkstra.create_data(
                &mut prevs,
                &mut distances,
                &ShortesPathResolver,
                &tile_state,
            );
            let hex_path = data.run(c_start, c_end);
            if let Some(p) = hex_path {
                for n in &p.nodes {
                    if *n != c_end {
                        tile_state.set_is_blocked(n);
                        path.push(*n);
                    }
                }
                c_start = c_end;
            } else {
                info!("its none!");
                return Some(HexPath {
                    nodes: path,
                    start,
                    end,
                });
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
