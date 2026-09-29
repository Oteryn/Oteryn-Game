//! Chain targeting of player spells (monster D12 `Ability.chain`, spell rule S23;
//! `docs/architecture/OTERYN_SPELL_CHAIN_BEHAVIOUR_CANDIDATE_V1.md` §3).
//!
//! Picks the creatures one cast hits, in order, with the step, hit delay and effect tiles of each,
//! and scales a rolled value by the step. The world facts (creatures, whom the caster may hit, line
//! of sight, walking paths) come from a [`ChainWorld`]; this module keeps no state.

use std::collections::BTreeSet;

/// Delay between two chain steps (Canary `combatChainDelay`, at least 50 ms).
pub(crate) const CHAIN_STEP_DELAY_MICROS: u64 = 50_000;

/// Search distance of the chain effect path (Canary `doChainEffect`, `maxSearchDist = 9`).
pub(crate) const CHAIN_PATH_SEARCH_TILES: u32 = 9;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ChainShape {
    /// Each further creature is searched from the last creature hit.
    Sequential,
    /// Every further creature is searched from the first creature and hit at step 1.
    Fork,
}

/// An admitted `Ability.chain` of a player spell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ChainSpec {
    /// Further creatures after the first: one cast hits at most `1 + max_targets`.
    pub(crate) max_targets: u32,
    pub(crate) range_tiles: u32,
    /// Search radius for the first creature of a cast without a target (absent: `range_tiles`).
    pub(crate) initial_range_tiles: u32,
    pub(crate) shape: ChainShape,
    pub(crate) damage_step_percent: i32,
    pub(crate) asset_binding: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct TilePosition {
    pub(crate) x: i32,
    pub(crate) y: i32,
    pub(crate) floor: i16,
}

impl TilePosition {
    /// Inside the square of `radius` around `self`, on the same floor.
    fn within(self, other: Self, radius: u32) -> bool {
        self.floor == other.floor
            && self.x.abs_diff(other.x) <= radius
            && self.y.abs_diff(other.y) <= radius
    }

    /// Squared Euclidean distance; orders like Canary `getEuclideanDistance`, exactly.
    fn distance_squared(self, other: Self) -> u64 {
        let dx = u64::from(self.x.abs_diff(other.x));
        let dy = u64::from(self.y.abs_diff(other.y));
        dx * dx + dy * dy
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ChainCreature {
    /// Runtime creature id; equal distances go to the lowest id (§7 Q1 proposal).
    pub(crate) id: u64,
    /// Exact actor atom of the Ability pipeline.
    pub(crate) actor: String,
    pub(crate) position: TilePosition,
}

/// World facts one chain cast reads.
pub(crate) trait ChainWorld {
    fn caster(&self) -> &ChainCreature;
    /// Creatures that may be picked; the caster may be listed and is never picked.
    fn creatures(&self) -> &[ChainCreature];
    /// Whether the caster may hit the creature with this spell (Canary `canDoCombat`). No party
    /// service exists, so party rules treat the caster as solo.
    fn may_hit(&self, creature: &ChainCreature) -> bool;
    /// Clear line of sight between two tiles of one floor (Canary `isSightClear`).
    fn sight_clear(&self, from: TilePosition, to: TilePosition) -> bool;
    /// Walking steps from `from` to a tile next to `to`, searched within
    /// [`CHAIN_PATH_SEARCH_TILES`]; `None` without a path.
    fn path(&self, from: TilePosition, to: TilePosition) -> Option<Vec<TilePosition>>;
}

/// Who starts the chain of one cast.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct ChainStart {
    /// The cast target of a spell that takes one.
    pub(crate) target: Option<u64>,
    /// The creature the caster is attacking.
    pub(crate) attacked: Option<u64>,
}

/// One creature a chain reaches.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ChainHit {
    pub(crate) creature: u64,
    pub(crate) actor: String,
    /// 0 for the first creature; the fork shape hits every further creature at step 1.
    pub(crate) step: u32,
    /// `step * 50 ms` after the cast.
    pub(crate) delay_micros: u64,
    /// Where the jump starts: the caster for step 0, else the searching creature.
    pub(crate) from: TilePosition,
    /// Tiles showing the chain effect: the walking path, then the creature's tile. Empty without
    /// a `chain_asset_binding`.
    pub(crate) effect_tiles: Vec<TilePosition>,
}

/// The creatures one cast hits, in hit order. Empty when there is no first creature (the cast
/// fails, §3 step 2). `start.target` is set only for a spell that takes a target.
pub(crate) fn pick_chain(
    spec: &ChainSpec,
    world: &dyn ChainWorld,
    start: ChainStart,
) -> Vec<ChainHit> {
    let caster = world.caster();
    let mut visited = BTreeSet::from([caster.id]);
    let first = match start.target {
        Some(id) => {
            find(world, id).filter(|creature| creature.id != caster.id && world.may_hit(creature))
        }
        None => start
            .attacked
            .and_then(|id| find(world, id))
            .filter(|creature| {
                valid(
                    world,
                    caster.position,
                    spec.initial_range_tiles,
                    &visited,
                    creature,
                )
            })
            .or_else(|| nearest(world, caster.position, spec.initial_range_tiles, &visited)),
    };
    let Some(first) = first else {
        return Vec::new();
    };
    visited.insert(first.id);
    let mut hits = vec![hit(spec, world, first, 0, caster.position)];
    let limit = usize::try_from(spec.max_targets)
        .unwrap_or(usize::MAX)
        .saturating_add(1);
    match spec.shape {
        ChainShape::Sequential => {
            let mut current = first.position;
            while hits.len() < limit {
                let Some(next) = nearest(world, current, spec.range_tiles, &visited) else {
                    break;
                };
                visited.insert(next.id);
                let step = u32::try_from(hits.len()).unwrap_or(u32::MAX);
                hits.push(hit(spec, world, next, step, current));
                current = next.position;
            }
        }
        ChainShape::Fork => {
            let mut further: Vec<_> = world
                .creatures()
                .iter()
                .filter(|creature| {
                    valid(world, first.position, spec.range_tiles, &visited, creature)
                })
                .collect();
            further.sort_by_key(|creature| {
                (
                    creature.position.distance_squared(first.position),
                    creature.id,
                )
            });
            for creature in further.into_iter().take(limit - 1) {
                hits.push(hit(spec, world, creature, 1, first.position));
            }
        }
    }
    hits
}

/// The value a creature reached at `step` gets: `value * max(0, 100 + step * step_percent) / 100`,
/// rounded half away from zero, in exact integer arithmetic (monster D12, B2). The rolled value is
/// an integer already (truncated formula bounds, then the world draw); mitigation comes later.
pub(crate) fn step_value(value: i64, step: u32, step_percent: i32) -> i64 {
    let percent = (100 + i128::from(step) * i128::from(step_percent)).max(0);
    let scaled = i128::from(value) * percent;
    let magnitude = (scaled.unsigned_abs() + 50) / 100;
    let rounded = i128::try_from(magnitude).unwrap_or(i128::MAX);
    let signed = if scaled < 0 { -rounded } else { rounded };
    i64::try_from(signed).unwrap_or(if signed < 0 { i64::MIN } else { i64::MAX })
}

fn find(world: &dyn ChainWorld, id: u64) -> Option<&ChainCreature> {
    world.creatures().iter().find(|creature| creature.id == id)
}

/// §3 step 4 from `searcher` within `radius`.
fn valid(
    world: &dyn ChainWorld,
    searcher: TilePosition,
    radius: u32,
    visited: &BTreeSet<u64>,
    creature: &ChainCreature,
) -> bool {
    !visited.contains(&creature.id)
        && searcher.within(creature.position, radius)
        && world.may_hit(creature)
        && world.sight_clear(searcher, creature.position)
}

/// The closest valid creature; equal distances go to the lowest id.
fn nearest<'a>(
    world: &'a dyn ChainWorld,
    searcher: TilePosition,
    radius: u32,
    visited: &BTreeSet<u64>,
) -> Option<&'a ChainCreature> {
    world
        .creatures()
        .iter()
        .filter(|creature| valid(world, searcher, radius, visited, creature))
        .min_by_key(|creature| (creature.position.distance_squared(searcher), creature.id))
}

fn hit(
    spec: &ChainSpec,
    world: &dyn ChainWorld,
    creature: &ChainCreature,
    step: u32,
    from: TilePosition,
) -> ChainHit {
    let effect_tiles = if spec.asset_binding.is_some() {
        let mut tiles = world.path(from, creature.position).unwrap_or_default();
        tiles.push(creature.position);
        tiles
    } else {
        Vec::new()
    };
    ChainHit {
        creature: creature.id,
        actor: creature.actor.clone(),
        step,
        delay_micros: u64::from(step).saturating_mul(CHAIN_STEP_DELAY_MICROS),
        from,
        effect_tiles,
    }
}
