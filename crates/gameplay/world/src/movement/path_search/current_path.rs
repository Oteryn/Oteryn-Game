//! Awaitable reader for the same Movement-owned search kernel; no per-tile boxed future.
use super::path_kernel::{SearchKernel, SearchStep};
use super::*;

pub trait PathTileReader {
    fn cost(&mut self, at: MovementLocalPosition) -> impl Future<Output = Option<u32>> + Send;
}
pub async fn measured_current_route(
    start: MovementLocalPosition,
    target: MovementLocalPosition,
    exact: bool,
    reader: &mut impl PathTileReader,
    budget: &mut crate::movement::path_budget::ChannelPathBudget,
) -> (Option<Vec<CardinalStep>>, usize) {
    let mut search = SearchKernel::new(
        start,
        target,
        SearchBounds {
            work: PATH_SEARCH_WORK,
            route_steps: ROUTE_STEPS,
            distance: if exact { 50 } else { CHASE_SEARCH_DISTANCE },
        },
        exact,
    );
    let mut charge = SearchCharge { budget, work: 0 };
    loop {
        let next = search.next();
        charge.work = search.expansions;
        match next {
            SearchStep::Tile(at) => search.supply(reader.cost(at).await),
            SearchStep::Finished(route) => return (route, search.expansions),
        }
    }
}

struct SearchCharge<'a> {
    budget: &'a mut crate::movement::path_budget::ChannelPathBudget,
    work: usize,
}
impl Drop for SearchCharge<'_> {
    fn drop(&mut self) {
        self.budget.charge(false, self.work);
    }
}

#[cfg(test)]
mod current_path_tests;
