use bevy::log::info;
use rand::rng;

use crate::grid::{GridIndex, HexHashGrid};

use super::{
    HexPath, SinglePathAlgorithm, context::PathContext, dijkstra::Dijkstra, random::choose_from_vec,
};

pub struct Chiseled;

impl SinglePathAlgorithm for Chiseled {
    fn calculate_path(
        &self,
        context: PathContext<'_>,
        start: GridIndex,
        end: GridIndex,
    ) -> Option<super::HexPath<GridIndex>> {
        let mut rng = rng();
        let mut path: Vec<GridIndex> = context.all().filter(|a| context.can_be_path(a)).collect();

        let mut current = choose_from_vec(&mut path, &mut rng);

        while let Some(ga) = current {
            let hex_path = HexPath::<GridIndex> {
                end,
                start,
                nodes: path.clone(),
            };
            info!("left_over_tiles: {path:?}");
            if !is_valid_path(&hex_path, context) {
                path.push(ga);
            }
            current = choose_from_vec(&mut path, &mut rng)
        }
        let hex_path = HexPath {
            nodes: path,
            start,
            end,
        };
        if is_valid_path(&hex_path, context) {
            Some(hex_path)
        } else {
            None
        }
    }
}

fn is_valid_path(path: &HexPath<GridIndex>, context: PathContext) -> bool {
    let dijkstra = Dijkstra;
    let grid = HexHashGrid::from_path(path);
    let context = context.with_grid(&grid);
    dijkstra
        .calculate_path(context, path.start, path.end)
        .is_some()
}
