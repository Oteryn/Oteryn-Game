//! Search-local tile facts and contiguous, step-aware A* labels. Only the existing heap's
//! estimate/neighbour/discovery keys determine search order.
use super::path_tiles::TileIndex;
use super::*;

#[derive(Clone, Copy)]
pub(super) struct Label {
    pub(super) at: MovementLocalPosition,
    pub(super) cost: u32,
    pub(super) steps: usize,
    via: Option<(CardinalStep, usize)>,
    previous: Option<usize>,
    tile: usize,
}

struct Tile {
    extra: Option<u32>,
    head: Option<usize>,
}

pub(super) struct SearchLabels {
    index: TileIndex,
    tiles: Vec<Tile>,
    labels: Vec<Label>,
}

impl SearchLabels {
    pub(super) fn new(start: MovementLocalPosition, distance: i32) -> Self {
        let mut index = TileIndex::new(start, distance);
        index.index(start, 0);
        Self {
            index,
            tiles: vec![Tile {
                extra: Some(0),
                head: Some(0),
            }],
            labels: vec![Label {
                at: start,
                cost: 0,
                steps: 0,
                via: None,
                previous: None,
                tile: 0,
            }],
        }
    }

    pub(super) fn get(&self, index: usize) -> Label {
        self.labels[index]
    }

    pub(super) fn dominated(&self, index: usize) -> bool {
        let label = self.get(index);
        if index == 0 {
            return false;
        }
        self.find(label.tile, |known| {
            known.cost <= label.cost
                && known.steps <= label.steps
                && (known.cost, known.steps) != (label.cost, label.steps)
        })
    }

    fn find(&self, tile: usize, predicate: impl Fn(Label) -> bool) -> bool {
        let mut cursor = self.tiles[tile].head;
        while let Some(index) = cursor {
            let label = self.get(index);
            if predicate(label) {
                return true;
            }
            cursor = label.previous;
        }
        false
    }

    pub(super) fn visit(
        &mut self,
        at: MovementLocalPosition,
        from: usize,
        step: CardinalStep,
        tile_cost: &mut impl FnMut(MovementLocalPosition) -> Option<u32>,
    ) -> Option<usize> {
        // Admission/occupancy is immutable for this synchronous search. Cache refusals too;
        // the next search and the actual step commit read fresh owner facts.
        let tile = self.index.index(at, self.tiles.len());
        if tile == self.tiles.len() {
            self.tiles.push(Tile {
                extra: tile_cost(at),
                head: None,
            });
        }
        let extra = self.tiles[tile].extra?;
        let previous = self.tiles[tile].head;
        let parent = self.get(from);
        let cost = parent.cost.saturating_add(STEP_COST).saturating_add(extra);
        let steps = parent.steps + 1;
        if self.find(tile, |known| known.cost <= cost && known.steps <= steps) {
            return None;
        }
        let index = self.labels.len();
        self.labels.push(Label {
            at,
            cost,
            steps,
            via: Some((step, from)),
            previous,
            tile,
        });
        self.tiles[tile].head = Some(index);
        Some(index)
    }

    pub(super) fn route_to(&self, mut index: usize) -> Vec<CardinalStep> {
        let mut route = Vec::with_capacity(self.get(index).steps);
        while let Some((step, from)) = self.get(index).via {
            route.push(step);
            index = from;
        }
        route.reverse();
        route
    }
}
