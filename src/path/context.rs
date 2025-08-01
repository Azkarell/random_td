use bevy::platform::collections::HashMap;
use std::cmp::Ordering;
use std::hash::Hash;

use crate::grid::{GridEntry, GridIndex, HexGridColumns, HexGridRows, HexHashGrid};

use super::dijkstra::TileState;

#[derive(Clone, Copy)]
pub struct PathContext<'a> {
    columns: &'a HexGridColumns,
    rows: &'a HexGridRows,
    grid: &'a HexHashGrid,
}

impl<'a> PathContext<'a> {
    pub fn iter_start_column(&self) -> impl Iterator<Item = GridIndex> {
        StartIterator::from_context(self)
    }

    pub fn iter_end_column(&self) -> impl Iterator<Item = GridIndex> {
        EndIterator::from_context(self)
    }

    pub fn all(&self) -> impl Iterator<Item = GridIndex> {
        self.columns.get_actual_column_count().flat_map(|r| {
            self.rows
                .get_actual_row_count(r)
                .map(move |q| GridIndex { q, r })
        })
    }

    pub fn can_be_path(&self, a: &GridIndex) -> bool {
        let range = self.columns.get_actual_column_count();
        let row_range = self.rows.get_actual_row_count(a.r);
        !self.iter_start_column().any(|i| i == *a)
            && !self.iter_end_column().any(|i| i == *a)
            && range.contains(&a.r)
            && row_range.contains(&a.q)
            && self.grid.can_be_path(a)
    }

    pub fn from_args(
        rows: &'a HexGridRows,
        columns: &'a HexGridColumns,
        grid: &'a HexHashGrid,
    ) -> Self {
        Self {
            rows,
            columns,
            grid,
        }
    }

    pub fn with_grid<'b>(&'a self, grid: &'b HexHashGrid) -> PathContext<'b>
    where
        'a: 'b,
    {
        PathContext {
            columns: self.columns,
            rows: self.rows,
            grid,
        }
    }

    pub fn all_pathable(&self) -> impl Iterator<Item = GridIndex> {
        self.all().filter(|g| self.can_be_path(g))
    }

    pub fn tile_state(&self, start: GridIndex, end: GridIndex) -> HashMap<GridIndex, TileState> {
        self.all()
            .map(|i| {
                if self.can_be_path(&i) || i == start || i == end {
                    (i, TileState::Useable)
                } else {
                    (i, TileState::Blocked)
                }
            })
            .collect()
    }

    pub fn can_be_path_ending(&self, index: GridIndex) -> bool {
        self.grid[index] == GridEntry::None
    }
}

struct EndIterator<'a> {
    context: &'a PathContext<'a>,
    cur: Option<GridIndex>,
    actual_end: i32,
}
impl<'a> EndIterator<'a> {
    fn from_context(context: &'a PathContext<'a>) -> Self {
        let range = context.columns.get_actual_column_count();
        let r = *range.start();
        let q = *context.rows.get_actual_row_count(r).end();
        let actual_end = *range.end();
        Self {
            context,
            cur: Some(GridIndex { r, q }),
            actual_end,
        }
    }
}

impl<'a> Iterator for EndIterator<'a> {
    type Item = GridIndex;
    fn size_hint(&self) -> (usize, Option<usize>) {
        if let Some(c) = self.cur {
            let end = (self.actual_end - c.r) as usize + 1;
            (end, Some(end))
        } else {
            (0, None)
        }
    }
    fn next(&mut self) -> Option<Self::Item> {
        let ret = self.cur;
        self.cur = if let Some(mut n) = self.cur {
            n.r += 1;
            let column_start = *self.context.rows.get_actual_row_count(n.r).end();
            n.q = column_start;
            if n.r > self.actual_end { None } else { Some(n) }
        } else {
            None
        };
        ret
    }
}

struct StartIterator<'a> {
    context: &'a PathContext<'a>,
    cur: Option<GridIndex>,
    actual_end: i32,
}
impl<'a> StartIterator<'a> {
    fn from_context(context: &'a PathContext<'a>) -> Self {
        let range = context.columns.get_actual_column_count();
        let r = *range.start();
        let q = *context.rows.get_actual_row_count(r).start();
        let actual_end = *range.end();
        Self {
            context,
            cur: Some(GridIndex { r, q }),
            actual_end,
        }
    }
}

impl<'a> Iterator for StartIterator<'a> {
    type Item = GridIndex;
    fn size_hint(&self) -> (usize, Option<usize>) {
        if let Some(c) = self.cur {
            let end = (self.actual_end - c.r) as usize + 1;
            (end, Some(end))
        } else {
            (0, None)
        }
    }
    fn next(&mut self) -> Option<Self::Item> {
        let ret = self.cur;
        self.cur = if let Some(mut n) = self.cur {
            n.r += 1;
            let column_start = *self.context.rows.get_actual_row_count(n.r).start();
            n.q = column_start;
            if n.r > self.actual_end { None } else { Some(n) }
        } else {
            None
        };
        ret
    }
}

#[derive(Clone)]
pub struct Distances {
    storage: HashMap<GridIndex, u32>,
}

impl Distances {
    pub fn new() -> Self {
        Self {
            storage: HashMap::new(),
        }
    }

    pub fn update(&mut self, pos: &GridIndex, distance: u32) -> bool {
        if let Some(d) = self.storage.get_mut(pos) {
            if distance < *d {
                *d = distance;
                true
            } else {
                false
            }
        } else {
            self.storage.insert(*pos, distance);
            true
        }
    }

    pub fn clear(&mut self) {
        self.storage.clear();
    }
}

impl Default for Distances {
    fn default() -> Self {
        Self::new()
    }
}
#[derive(Debug, PartialEq, Eq, Hash)]
pub enum CacheUpdateResult {
    Inserted,
    Updated,
    Ignored,
}
pub struct IgnoreMissingEntries;
impl CacheBehaviour for IgnoreMissingEntries {}
pub struct InsertMissingEntries;
impl CacheBehaviour for InsertMissingEntries {}
pub trait CacheBehaviour {}

pub trait Cache<B: CacheBehaviour> {
    type Access;
    type Output;
    fn behaviour(&self) -> B;
    fn get(&self, access: &Self::Access) -> Option<&Self::Output>;
    fn update<C: FnMut(&mut Self::Output) -> bool>(
        &mut self,
        access: &Self::Access,
        value: Self::Output,
        should_update: C,
    ) -> CacheUpdateResult;
    fn clear(&mut self);
    fn min_by<F: FnMut(&Self::Output, &Self::Output) -> Option<Ordering>>(
        &self,
        cmp: F,
    ) -> Option<(Self::Access, Self::Output)>;
    fn remove(&mut self, acces: &Self::Access) -> Option<Self::Output>;
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool;
    fn iter<'a>(&'a self) -> impl Iterator<Item = (&'a Self::Access, &'a Self::Output)>
    where
        Self::Access: 'a,
        Self::Output: 'a;
}
impl<K, V> Cache<IgnoreMissingEntries> for HashMap<K, V>
where
    K: Hash + Clone + Copy + PartialEq + Eq,
    V: Clone + Copy,
{
    type Access = K;

    type Output = V;

    fn get(&self, access: &Self::Access) -> Option<&Self::Output> {
        self.get(access)
    }

    fn update<C: FnMut(&mut Self::Output) -> bool>(
        &mut self,
        access: &Self::Access,
        value: Self::Output,
        mut should_update: C,
    ) -> CacheUpdateResult {
        if let Some(a) = self.get_mut(access) {
            if should_update(a) {
                *a = value;
                CacheUpdateResult::Updated
            } else {
                CacheUpdateResult::Ignored
            }
        } else {
            CacheUpdateResult::Ignored
        }
    }

    fn clear(&mut self) {
        self.clear();
    }

    fn min_by<F: FnMut(&Self::Output, &Self::Output) -> Option<Ordering>>(
        &self,
        mut cmp: F,
    ) -> Option<(Self::Access, Self::Output)> {
        self.iter()
            .min_by(|x, y| cmp(x.1, y.1).unwrap_or(Ordering::Equal))
            .map(|v| (*v.0, *v.1))
    }

    fn remove(&mut self, access: &Self::Access) -> Option<Self::Output> {
        self.remove(access)
    }

    fn len(&self) -> usize {
        self.len()
    }

    fn is_empty(&self) -> bool {
        self.is_empty()
    }

    fn behaviour(&self) -> IgnoreMissingEntries {
        IgnoreMissingEntries
    }

    fn iter<'a>(&'a self) -> impl Iterator<Item = (&'a Self::Access, &'a Self::Output)>
    where
        Self::Access: 'a,
        Self::Output: 'a,
    {
        self.iter()
    }
}

impl<K, V> Cache<InsertMissingEntries> for HashMap<K, V>
where
    K: Hash + Clone + Copy + PartialEq + Eq,
    V: Clone + Copy,
{
    type Access = K;

    type Output = V;

    fn get(&self, access: &Self::Access) -> Option<&Self::Output> {
        self.get(access)
    }

    fn update<C: FnMut(&mut Self::Output) -> bool>(
        &mut self,
        access: &Self::Access,
        value: Self::Output,
        mut should_update: C,
    ) -> CacheUpdateResult {
        if let Some(a) = self.get_mut(access) {
            if should_update(a) {
                *a = value;
                CacheUpdateResult::Updated
            } else {
                CacheUpdateResult::Ignored
            }
        } else {
            self.insert(*access, value);
            CacheUpdateResult::Inserted
        }
    }

    fn clear(&mut self) {
        self.clear();
    }

    fn min_by<F: FnMut(&Self::Output, &Self::Output) -> Option<Ordering>>(
        &self,
        mut cmp: F,
    ) -> Option<(Self::Access, Self::Output)> {
        self.iter()
            .min_by(|x, y| cmp(x.1, y.1).unwrap_or(Ordering::Equal))
            .map(|v| (*v.0, *v.1))
    }

    fn remove(&mut self, access: &Self::Access) -> Option<Self::Output> {
        self.remove(access)
    }

    fn len(&self) -> usize {
        self.len()
    }

    fn is_empty(&self) -> bool {
        self.is_empty()
    }

    fn behaviour(&self) -> InsertMissingEntries {
        InsertMissingEntries
    }

    fn iter<'a>(&'a self) -> impl Iterator<Item = (&'a Self::Access, &'a Self::Output)>
    where
        Self::Access: 'a,
        Self::Output: 'a,
    {
        self.iter()
    }
}

pub trait DistanceCache: Cache<IgnoreMissingEntries> {
    fn get_min(&self) -> Option<(Self::Access, Self::Output)>;
    fn update_distance(&mut self, access: &Self::Access, value: Self::Output) -> CacheUpdateResult;
}

impl<C: Cache<IgnoreMissingEntries>> DistanceCache for C
where
    Self::Output: PartialOrd + Clone + Copy,
{
    fn get_min(&self) -> Option<(Self::Access, Self::Output)> {
        self.min_by(|l, r| l.partial_cmp(r))
    }

    fn update_distance(&mut self, access: &Self::Access, value: Self::Output) -> CacheUpdateResult {
        self.update(access, value, |old| value < *old)
    }
}

#[cfg(test)]
mod tests {
    use super::PathContext;
    use crate::grid::{GridIndex, HexGridColumns, HexGridRows, HexHashGrid};
    #[test]
    fn size_hint_end() {
        let context = PathContext {
            columns: &HexGridColumns(10),
            rows: &HexGridRows(10),
            grid: &HexHashGrid::new(),
        };
        let mut iter = context.iter_end_column();
        assert_eq!((11, Some(11)), iter.size_hint());
        iter.next();
        assert_eq!((10, Some(10)), iter.size_hint());
        iter.next();
        assert_eq!((9, Some(9)), iter.size_hint());
        iter.next();
        assert_eq!((8, Some(8)), iter.size_hint());
        iter.next();
        assert_eq!((7, Some(7)), iter.size_hint());
        iter.next();
        assert_eq!((6, Some(6)), iter.size_hint());
        iter.next();
        assert_eq!((5, Some(5)), iter.size_hint());
        iter.next();
        assert_eq!((4, Some(4)), iter.size_hint());
        iter.next();
        assert_eq!((3, Some(3)), iter.size_hint());
        iter.next();
        assert_eq!((2, Some(2)), iter.size_hint());
        iter.next();
        assert_eq!((1, Some(1)), iter.size_hint());
        iter.next();
        assert_eq!((0, None), iter.size_hint());
    }
    #[test]
    fn size_hint_start() {
        let context = PathContext {
            columns: &HexGridColumns(10),
            rows: &HexGridRows(10),
            grid: &HexHashGrid::new(),
        };
        let mut iter = context.iter_start_column();
        assert_eq!((11, Some(11)), iter.size_hint());
        iter.next();
        assert_eq!((10, Some(10)), iter.size_hint());
        iter.next();
        assert_eq!((9, Some(9)), iter.size_hint());
        iter.next();
        assert_eq!((8, Some(8)), iter.size_hint());
        iter.next();
        assert_eq!((7, Some(7)), iter.size_hint());
        iter.next();
        assert_eq!((6, Some(6)), iter.size_hint());
        iter.next();
        assert_eq!((5, Some(5)), iter.size_hint());
        iter.next();
        assert_eq!((4, Some(4)), iter.size_hint());
        iter.next();
        assert_eq!((3, Some(3)), iter.size_hint());
        iter.next();
        assert_eq!((2, Some(2)), iter.size_hint());
        iter.next();
        assert_eq!((1, Some(1)), iter.size_hint());
        iter.next();
        assert_eq!((0, None), iter.size_hint());
    }
    #[test]
    fn test_start_iterator() {
        let context = PathContext {
            columns: &HexGridColumns(10),
            rows: &HexGridRows(10),
            grid: &HexHashGrid::new(),
        };
        let accesses: Vec<GridIndex> = context.iter_start_column().collect();
        let expected = vec![
            GridIndex::new(-7, -5),
            GridIndex::new(-8, -4),
            GridIndex::new(-8, -3),
            GridIndex::new(-9, -2),
            GridIndex::new(-9, -1),
            GridIndex::new(-10, 0),
            GridIndex::new(-10, 1),
            GridIndex::new(-11, 2),
            GridIndex::new(-11, 3),
            GridIndex::new(-12, 4),
            GridIndex::new(-12, 5),
        ];

        assert_eq!(accesses, expected);
    }

    #[test]
    fn test_end_iterator() {
        let context = PathContext {
            columns: &HexGridColumns(10),
            rows: &HexGridRows(10),
            grid: &HexHashGrid::new(),
        };
        let accesses: Vec<GridIndex> = context.iter_end_column().collect();
        let expected = vec![
            GridIndex::new(13, -5),
            GridIndex::new(12, -4),
            GridIndex::new(12, -3),
            GridIndex::new(11, -2),
            GridIndex::new(11, -1),
            GridIndex::new(10, 0),
            GridIndex::new(10, 1),
            GridIndex::new(9, 2),
            GridIndex::new(9, 3),
            GridIndex::new(8, 4),
            GridIndex::new(8, 5),
        ];

        assert_eq!(accesses, expected);
    }
}
