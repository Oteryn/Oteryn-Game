//! Pure chain and area world snapshots captured by the spell dispatch owners.

use crate::dispatch_plans::geometry::{self, TileFlags};
use oteryn_gameplay_combat_rules::chain::{
    CHAIN_PATH_SEARCH_TILES, ChainCreature, ChainWorld, TilePosition,
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug)]
pub struct World<D> {
    pub origin: TilePosition,
    /// The position every populated sight path starts from: the caster for a directional
    /// cast, otherwise the aimed origin. Area visibility must look up the same key.
    pub sight_origin: TilePosition,
    pub direction: D,
    pub caster: ChainCreature,
    pub creatures: Vec<ChainCreature>,
    pub legal: BTreeSet<u64>,
    pub sight: BTreeSet<(TilePosition, TilePosition)>,
    pub tiles: BTreeMap<TilePosition, TileFlags>,
}
impl<D> World<D> {
    /// Area targets, created items and tile cues are visible from the sight origin.
    pub fn area_sight_clear(&self, p: TilePosition) -> bool {
        self.sight_clear(self.sight_origin, p)
    }
}
impl<D> ChainWorld for World<D> {
    fn caster(&self) -> &ChainCreature {
        &self.caster
    }
    fn creatures(&self) -> &[ChainCreature] {
        &self.creatures
    }
    fn may_hit(&self, c: &ChainCreature) -> bool {
        self.legal.contains(&c.id)
    }
    fn sight_clear(&self, a: TilePosition, b: TilePosition) -> bool {
        a == b || self.sight.contains(&(a, b))
    }
    fn path(&self, from: TilePosition, to: TilePosition) -> Option<Vec<TilePosition>> {
        geometry::path(
            plan_position(from),
            plan_position(to),
            CHAIN_PATH_SEARCH_TILES,
            |p| self.tiles.get(&cast_position(p)).copied(),
        )
        .map(|path| path.into_iter().map(cast_position).collect())
    }
}
fn plan_position(p: TilePosition) -> geometry::TilePosition {
    (p.x, p.y, p.floor)
}

fn cast_position(p: geometry::TilePosition) -> TilePosition {
    TilePosition {
        x: p.0,
        y: p.1,
        floor: p.2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tile_aimed_area_visibility_uses_the_populated_sight_origin() {
        // A non-directional rune aimed at a tile populates (aimed tile, candidate) paths.
        let at = |x| TilePosition { x, y: 0, floor: 7 };
        let world = World {
            origin: at(4),
            sight_origin: at(4),
            direction: (1, 0),
            caster: ChainCreature {
                id: 1,
                actor: "actor:caster".into(),
                position: at(0),
            },
            creatures: Vec::new(),
            legal: BTreeSet::new(),
            sight: BTreeSet::from([(at(0), at(4)), (at(4), at(5))]),
            tiles: BTreeMap::new(),
        };
        // The neighbour of the aimed tile is visible from the aimed tile, which is the
        // key the area paths were populated with; the caster has no path to it.
        assert!(world.area_sight_clear(at(5)));
        assert!(world.area_sight_clear(at(4)));
        assert!(!world.sight_clear(world.caster.position, at(5)));
        assert!(!world.area_sight_clear(at(6)));
    }
}
