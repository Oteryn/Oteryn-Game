//! Resumable form of the existing bounded A*: sync and transaction readers share every label.
use super::*;

pub(super) enum SearchStep {
    Tile(MovementLocalPosition),
    Finished(Option<Vec<CardinalStep>>),
}
pub(super) struct SearchKernel {
    start: MovementLocalPosition,
    target: MovementLocalPosition,
    bounds: SearchBounds,
    exact: bool,
    open: BinaryHeap<Reverse<(u32, u32, u8, usize)>>,
    labels: SearchLabels,
    current: Option<(usize, u8)>,
    pending: Option<(usize, u8, CardinalStep, MovementLocalPosition)>,
    pub(super) expansions: usize,
}
impl SearchKernel {
    pub(super) fn new(
        start: MovementLocalPosition,
        target: MovementLocalPosition,
        bounds: SearchBounds,
        exact: bool,
    ) -> Self {
        let h = estimate(start, target, exact);
        Self {
            start,
            target,
            bounds,
            exact,
            open: BinaryHeap::from([Reverse((h, h, 0, 0))]),
            labels: SearchLabels::new(start, bounds.distance),
            current: None,
            pending: None,
            expansions: 0,
        }
    }
    pub(super) fn next(&mut self) -> SearchStep {
        if self.start.floor != self.target.floor {
            return SearchStep::Finished(None);
        }
        loop {
            if let Some((index, order)) = self.current {
                if usize::from(order) >= NEIGHBOURS.len() {
                    self.current = None;
                    continue;
                }
                self.current = Some((index, order + 1));
                let at = self.labels.get(index).at;
                let step = NEIGHBOURS[usize::from(order)];
                let (dx, dy) = delta(step);
                let (Some(x), Some(y)) = (at.x.checked_add(dx), at.y.checked_add(dy)) else {
                    continue;
                };
                let next = MovementLocalPosition {
                    x,
                    y,
                    floor: at.floor,
                };
                if chebyshev(self.start, next) > i64::from(self.bounds.distance) {
                    continue;
                }
                self.pending = Some((index, order, step, next));
                if let Some(cost) = self.labels.known_cost(next) {
                    self.supply(cost);
                    continue;
                }
                return SearchStep::Tile(next);
            }
            let Some(Reverse((_, _, _, index))) = self.open.pop() else {
                return SearchStep::Finished(None);
            };
            if self.labels.dominated(index) {
                continue;
            }
            let label = self.labels.get(index);
            if is_goal(label.at, self.target, self.exact) {
                return SearchStep::Finished(Some(self.labels.route_to(index)));
            }
            if self.expansions >= self.bounds.work {
                return SearchStep::Finished(None);
            }
            self.expansions += 1;
            if label.steps < self.bounds.route_steps {
                self.current = Some((index, 0));
            }
        }
    }
    pub(super) fn supply(&mut self, cost: Option<u32>) {
        let Some((index, order, step, at)) = self.pending.take() else {
            return;
        };
        if let Some(next) = self.labels.visit(at, index, step, &mut |_| cost) {
            let h = estimate(at, self.target, self.exact);
            self.open.push(Reverse((
                self.labels.get(next).cost.saturating_add(h),
                h,
                order,
                next,
            )));
        }
    }
}
