use crate::grid::GridIndex;

use super::{
    HexPath,
    context::{Cache, CacheBehaviour},
};

pub trait Resolver<B: CacheBehaviour, C: Cache<B, Access = GridIndex, Output = GridIndex>> {
    fn resolve_path(&self, cache: &C, start: GridIndex, end: GridIndex) -> Option<HexPath>;
}

pub struct ShortesPathResolver;

impl<B: CacheBehaviour, C: Cache<B, Access = GridIndex, Output = GridIndex>> Resolver<B, C>
    for ShortesPathResolver
{
    fn resolve_path(&self, cache: &C, start: GridIndex, end: GridIndex) -> Option<HexPath> {
        get_shortest_path(cache, end, start).map(|n| HexPath {
            nodes: n,
            start,
            end,
        })
    }
}

fn get_shortest_path<B: CacheBehaviour, C: Cache<B, Access = GridIndex, Output = GridIndex>>(
    prevs: &C,
    end: GridIndex,
    start: GridIndex,
) -> Option<Vec<GridIndex>> {
    let mut ret = vec![end];
    let mut cur = end;
    while let Some(p) = prevs.get(&cur) {
        ret.push(*p);
        cur = *p;
        if cur == start {
            ret.reverse();
            return Some(ret);
        }
    }
    None
}
