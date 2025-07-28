use super::{
    HexPath,
    context::{Cache, CacheBehaviour},
    dijkstra::Indexable,
};

pub trait Resolver<I: Indexable, B: CacheBehaviour, C: Cache<B, Access = I, Output = I>> {
    fn resolve_path(&self, cache: &C, start: I, end: I) -> Option<HexPath<I>>;
}

pub struct ShortesPathResolver;

impl<I: Indexable, B: CacheBehaviour, C: Cache<B, Access = I, Output = I>> Resolver<I, B, C>
    for ShortesPathResolver
{
    fn resolve_path(&self, cache: &C, start: I, end: I) -> Option<HexPath<I>> {
        get_shortest_path(cache, end, start).map(|n| HexPath {
            nodes: n,
            start,
            end,
        })
    }
}

fn get_shortest_path<I: Indexable, B: CacheBehaviour, C: Cache<B, Access = I, Output = I>>(
    prevs: &C,
    end: I,
    start: I,
) -> Option<Vec<I>> {
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
