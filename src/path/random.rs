use rand::{Rng, rng, seq::IteratorRandom};

use crate::grid::GridIndex;

use super::{EndSelector, StartSelector, context::PathContext};

#[derive(Debug, Default, Clone, Copy)]
pub struct RandomSelector;

impl StartSelector for RandomSelector {
    fn get_start(&self, context: PathContext<'_>) -> Option<GridIndex> {
        let mut rng = rng();
        context
            .iter_start_column()
            .filter(|i| context.can_be_path_ending(*i))
            .choose(&mut rng)
    }
}

impl EndSelector for RandomSelector {
    fn get_end(&self, context: PathContext<'_>) -> Option<GridIndex> {
        let mut rng = rng();
        context
            .iter_end_column()
            .filter(|i| context.can_be_path_ending(*i))
            .choose(&mut rng)
    }
}

pub fn choose_from_vec<T: Clone, R: Rng + ?Sized>(vec: &mut Vec<T>, rng: &mut R) -> Option<T> {
    let ele = vec.iter().cloned().enumerate().choose(rng);
    if let Some(t) = ele {
        vec.remove(t.0);
        Some(t.1)
    } else {
        None
    }
}
