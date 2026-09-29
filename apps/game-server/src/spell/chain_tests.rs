//! Engine tests of the chain runtime (`OTERYN_SPELL_CHAIN_BEHAVIOUR_CANDIDATE_V1.md` §6): each
//! builds a deterministic world with fixed creature ids.

#![allow(clippy::expect_used)]

use std::collections::{BTreeSet, VecDeque};

use oteryn_simulation_determinism::SemanticTimeMicros;
use serde_json::Value;

use super::authoring::spell_from_bundle;
use super::chain::{
    CHAIN_PATH_SEARCH_TILES, ChainCreature, ChainShape, ChainStart, ChainWorld, TilePosition,
    step_value,
};
use super::plan::{CastPlanError, chain_plans, effect_plan};
use super::*;
use crate::ability::{AbilityEngine, AbilityOccurrence, RevisionSet};

const SPELL: &str = include_str!(
    "../../../../tools/content-schema/spell-authoring/samples/starter-bundles/instant-energy_strike/spell.json"
);
const DEPENDENCIES: &str = include_str!(
    "../../../../tools/content-schema/spell-authoring/samples/starter-bundles/instant-energy_strike/dependencies.json"
);
const CHAINS: &str =
    include_str!("../../../../tools/content-schema/spell-authoring/chain-behaviours.json");

const CASTER: u64 = 1;

/// The Energy Strike bundle with the accepted chain of `name` (`chain-behaviours.json`) on its
/// Ability; `target_or_direction` as Lightning (true) or a spell without a target (false).
fn bundle(name: &str, target_or_direction: bool) -> (Value, Value) {
    let mut spell: Value = serde_json::from_str(SPELL).expect("spell");
    let mut dependencies: Value = serde_json::from_str(DEPENDENCIES).expect("dependencies");
    let chains: Value = serde_json::from_str(CHAINS).expect("chain-behaviours.json");
    let mut chain = chains["spells"][name].clone();
    chain
        .as_object_mut()
        .expect("chain object")
        .remove("sources");
    for ability in dependencies["abilities"].as_array_mut().expect("abilities") {
        ability["chain"] = chain.clone();
    }
    spell["spell"]["targeting"]["target_or_direction"] = Value::Bool(target_or_direction);
    (spell, dependencies)
}

fn chain_spell(name: &str, target_or_direction: bool) -> SpellDefinition {
    let (spell, dependencies) = bundle(name, target_or_direction);
    spell_from_bundle(&spell, &dependencies).expect("chain spell admitted")
}

fn with_binding(mut spell: SpellDefinition) -> SpellDefinition {
    if let Some(chain) = spell.chain.as_mut() {
        chain.asset_binding = Some("canary.appearance:effect/pink_energy_spark".into());
    }
    spell
}

fn sorcerer() -> CasterState {
    CasterState {
        vocation: Vocation::Sorcerer,
        level: 100,
        magic_level: 50,
        premium: true,
        mana: 1000,
        max_mana: 1000,
        soul: 100,
        learned: BTreeSet::new(),
        attack_skill: 10,
        attack_value: 7,
        attack_factor: 1.0,
        shielding_skill: 10,
    }
}

fn lowest(minimum: i64, _: i64) -> i64 {
    minimum
}

fn at(x: i32, y: i32) -> TilePosition {
    TilePosition { x, y, floor: 7 }
}

/// A floor with walls and protection-zone tiles; the caster stands at (0, 0).
#[derive(Default)]
struct Grid {
    caster: Option<ChainCreature>,
    creatures: Vec<ChainCreature>,
    walls: BTreeSet<TilePosition>,
    protection: BTreeSet<TilePosition>,
}

impl Grid {
    fn new() -> Self {
        let caster = creature(CASTER, at(0, 0));
        Self {
            caster: Some(caster.clone()),
            creatures: vec![caster],
            ..Self::default()
        }
    }

    fn with(mut self, id: u64, position: TilePosition) -> Self {
        self.creatures.push(creature(id, position));
        self
    }
}

fn creature(id: u64, position: TilePosition) -> ChainCreature {
    ChainCreature {
        id,
        actor: format!("actor:{id}"),
        position,
    }
}

impl ChainWorld for Grid {
    fn caster(&self) -> &ChainCreature {
        self.caster.as_ref().expect("caster")
    }

    fn creatures(&self) -> &[ChainCreature] {
        &self.creatures
    }

    fn may_hit(&self, creature: &ChainCreature) -> bool {
        !self.protection.contains(&creature.position)
    }

    fn sight_clear(&self, from: TilePosition, to: TilePosition) -> bool {
        if from.floor != to.floor {
            return false;
        }
        let (dx, dy) = (to.x - from.x, to.y - from.y);
        let steps = dx.abs().max(dy.abs());
        (1..steps).all(|t| {
            let x = from.x + (f64::from(dx * t) / f64::from(steps)).round() as i32;
            let y = from.y + (f64::from(dy * t) / f64::from(steps)).round() as i32;
            !self.walls.contains(&TilePosition {
                x,
                y,
                floor: from.floor,
            })
        })
    }

    fn path(&self, from: TilePosition, to: TilePosition) -> Option<Vec<TilePosition>> {
        // Breadth first, straight moves before diagonal ones, until a tile next to `to`.
        const MOVES: [(i32, i32); 8] = [
            (1, 0),
            (-1, 0),
            (0, 1),
            (0, -1),
            (1, 1),
            (1, -1),
            (-1, 1),
            (-1, -1),
        ];
        let near = |p: TilePosition| p.x.abs_diff(to.x) <= 1 && p.y.abs_diff(to.y) <= 1;
        let mut queue = VecDeque::from([(from, Vec::new())]);
        let mut seen = BTreeSet::from([from]);
        while let Some((position, steps)) = queue.pop_front() {
            if near(position) {
                return Some(steps);
            }
            for (dx, dy) in MOVES {
                let next = TilePosition {
                    x: position.x + dx,
                    y: position.y + dy,
                    floor: position.floor,
                };
                if next.x.abs_diff(from.x) > CHAIN_PATH_SEARCH_TILES
                    || next.y.abs_diff(from.y) > CHAIN_PATH_SEARCH_TILES
                    || self.walls.contains(&next)
                    || !seen.insert(next)
                {
                    continue;
                }
                let mut steps = steps.clone();
                steps.push(next);
                queue.push_back((next, steps));
            }
        }
        None
    }
}

fn cast(
    spell: &SpellDefinition,
    world: &Grid,
    start: ChainStart,
) -> Result<CastResolution, CastRejection> {
    resolve_chain_cast(
        spell,
        &sorcerer(),
        &Cooldowns::default(),
        SemanticTimeMicros::from_micros(0),
        world,
        start,
        &mut lowest,
    )
}

fn hit_ids(resolution: &CastResolution) -> Vec<u64> {
    resolution
        .chain
        .iter()
        .map(|hit| hit.hit.creature)
        .collect()
}

fn attacked(id: u64) -> ChainStart {
    ChainStart {
        target: None,
        attacked: Some(id),
    }
}

/// The magnitude of the one damage effect.
fn damage(effects: &[ResolvedEffect]) -> i64 {
    assert_eq!(effects.len(), 1, "{effects:?}");
    effects
        .iter()
        .map(|effect| match effect {
            ResolvedEffect::Damage { magnitude, .. } => *magnitude,
            _ => 0,
        })
        .sum()
}

#[test]
fn every_accepted_chain_is_admitted_with_its_values() {
    for (name, max_targets, range, initial, shape, step) in [
        ("chained penance", 4, 4, 4, ChainShape::Sequential, -5),
        ("forked glacier", 6, 4, 7, ChainShape::Fork, 0),
        ("forked thorns", 5, 4, 7, ChainShape::Fork, 0),
        ("lightning", 2, 4, 4, ChainShape::Sequential, 0),
    ] {
        let chain = chain_spell(name, false).chain.expect("chain");
        assert_eq!(
            (
                chain.max_targets,
                chain.range_tiles,
                chain.initial_range_tiles,
                chain.shape,
                chain.damage_step_percent
            ),
            (max_targets, range, initial, shape, step),
            "{name}"
        );
    }
}

#[test]
fn unresolved_chain_forms_stay_rejected() {
    // Backtracking, the support-chain filters (§4.2) and unknown fields fail closed.
    for (field, value) in [
        ("backtracking", Value::Bool(true)),
        ("target_filter", Value::from("ranged_monsters")),
        ("target_filter", Value::from("players")),
        ("shape", Value::from("spiral")),
        ("damage_step_percent", Value::from(-101)),
        ("initial_range_tiles", Value::from(0)),
        ("bounces", Value::from(1)),
    ] {
        let (spell, mut dependencies) = bundle("chained penance", false);
        for ability in dependencies["abilities"].as_array_mut().expect("abilities") {
            ability["chain"][field] = value.clone();
        }
        assert!(
            spell_from_bundle(&spell, &dependencies).is_err(),
            "{field} = {value}"
        );
    }
}

/// §6 test 1.
#[test]
fn a_cast_without_a_target_starts_on_the_attacked_or_the_nearest_creature() {
    let spell = chain_spell("chained penance", false);
    let mut world = Grid::new()
        .with(10, at(3, 0))
        .with(11, at(1, 1))
        .with(12, at(6, 0))
        .with(13, at(-2, 0));
    // The attacked creature is valid and within the initial range 4.
    let first = |world: &Grid, start| hit_ids(&cast(&spell, world, start).expect("cast"))[0];
    assert_eq!(first(&world, attacked(10)), 10);
    // Out of the initial range: the nearest valid creature.
    assert_eq!(first(&world, attacked(12)), 11);
    // Without an attacked creature: the nearest.
    assert_eq!(first(&world, ChainStart::default()), 11);
    // A cast target of a spell that takes none is ignored.
    let targeted = ChainStart {
        target: Some(13),
        attacked: None,
    };
    assert_eq!(first(&world, targeted), 11);
    // An attacked creature in a protection zone is not valid.
    world.protection.insert(at(3, 0));
    assert_eq!(first(&world, attacked(10)), 11);
}

/// §6 test 2.
#[test]
fn with_no_valid_creature_in_range_the_cast_fails_and_spends_nothing() {
    let spell = chain_spell("chained penance", false);
    let mut world = Grid::new().with(10, at(5, 0)).with(11, at(1, 0));
    world.protection.insert(at(1, 0));
    world.creatures.push(creature(
        12,
        TilePosition {
            x: 1,
            y: 1,
            floor: 6,
        },
    ));
    let cooldowns = Cooldowns::default();
    let caster = sorcerer();
    let failed = resolve_chain_cast(
        &spell,
        &caster,
        &cooldowns,
        SemanticTimeMicros::from_micros(0),
        &world,
        ChainStart::default(),
        &mut lowest,
    );
    // The rejection carries no debit or cooldown: the caller keeps its state.
    assert_eq!(failed, Err(CastRejection::NoChainTarget));
    world.creatures.push(creature(13, at(4, 4)));
    let cast = resolve_chain_cast(
        &spell,
        &caster,
        &cooldowns,
        SemanticTimeMicros::from_micros(0),
        &world,
        ChainStart::default(),
        &mut lowest,
    )
    .expect("a creature in range");
    assert_eq!(hit_ids(&cast)[0], 13);
    assert!(cast.mana_spent > 0);
    assert!(cast.cooldowns.spell_ready_at(&spell.key).is_some());
}

/// §6 test 3 and the Q1 tie proposal.
#[test]
fn a_sequential_chain_picks_the_closest_valid_creature() {
    let spell = chain_spell("chained penance", false);
    let mut world = Grid::new()
        .with(10, at(1, 0)) // first
        .with(20, at(3, 0)) // behind the wall at (2, 0) from 10
        .with(21, at(2, 3)) // in a protection zone
        .with(22, at(0, 3)) // valid, distance^2 10 from 10
        .with(23, at(4, 1)); // valid, distance^2 10 from 10, higher id
    world.creatures.push(creature(
        24,
        TilePosition {
            x: 2,
            y: 0,
            floor: 6,
        },
    ));
    world.walls.insert(at(2, 0));
    world.protection.insert(at(2, 3));
    let hits = hit_ids(&cast(&spell, &world, attacked(10)).expect("cast"));
    // From 10: 22 and 23 tie at the same distance, the lower id wins. From 22 (0, 3): 21 is in a
    // protection zone, 10 is already hit; 20 at (3, 0) is 3 tiles away with clear sight.
    assert_eq!(hits[..2], [10, 22]);
    assert_eq!(hits, vec![10, 22, 20, 23]);
    assert!(!hits.contains(&24) && !hits.contains(&21));
}

/// §6 test 4.
#[test]
fn the_count_is_capped_at_one_plus_max_targets() {
    let mut world = Grid::new();
    for id in 0..8 {
        world = world.with(10 + id, at(1 + i32::try_from(id).expect("id"), 0));
    }
    let mut spell = chain_spell("chained penance", false);
    assert_eq!(
        hit_ids(&cast(&spell, &world, ChainStart::default()).expect("cast")).len(),
        5
    );
    // A Wheel augment adds one further creature.
    if let Some(chain) = spell.chain.as_mut() {
        chain.max_targets += 1;
    }
    assert_eq!(
        hit_ids(&cast(&spell, &world, ChainStart::default()).expect("cast")).len(),
        6
    );
}

/// §6 test 5.
#[test]
fn chained_penance_steps_the_damage_and_the_delay() {
    let spell = chain_spell("chained penance", false);
    let mut plain = spell.clone();
    plain.chain = None;
    let base = resolve_cast(
        &plain,
        &sorcerer(),
        &Cooldowns::default(),
        SemanticTimeMicros::from_micros(0),
        true,
        &mut lowest,
    )
    .expect("plain cast");
    let base = damage(&base.effects);
    assert!(base > 0);
    let mut world = Grid::new();
    for id in 0..5 {
        world = world.with(10 + id, at(1 + i32::try_from(id).expect("id"), 0));
    }
    let resolution = cast(&spell, &world, ChainStart::default()).expect("cast");
    assert_eq!(hit_ids(&resolution), vec![10, 11, 12, 13, 14]);
    let magnitudes: Vec<_> = resolution
        .chain
        .iter()
        .map(|hit| damage(&hit.effects))
        .collect();
    let expected: Vec<_> = [100_i64, 95, 90, 85, 80]
        .iter()
        .map(|percent| (base * percent + 50) / 100)
        .collect();
    assert_eq!(magnitudes, expected);
    let delays: Vec<_> = resolution
        .chain
        .iter()
        .map(|hit| hit.hit.delay_micros)
        .collect();
    assert_eq!(delays, vec![0, 50_000, 100_000, 150_000, 200_000]);
    assert!(resolution.effects.is_empty());
}

/// §6 test 6.
#[test]
fn a_fork_picks_every_further_creature_from_the_first() {
    let spell = chain_spell("forked glacier", false);
    let world = Grid::new()
        .with(10, at(1, 0)) // first: nearest to the caster
        .with(11, at(5, 0)) // 4 from the first
        .with(12, at(6, 0)) // 5 from the first, 1 from 11: not picked
        .with(13, at(1, 2))
        .with(14, at(1, -2)); // ties with 13; the lower id first
    let resolution = cast(&spell, &world, ChainStart::default()).expect("cast");
    assert_eq!(hit_ids(&resolution), vec![10, 13, 14, 11]);
    for hit in &resolution.chain[1..] {
        assert_eq!((hit.hit.step, hit.hit.from), (1, at(1, 0)));
        assert_eq!(hit.hit.delay_micros, 50_000);
    }
    // The fork cap: 1 + 6 creatures.
    let mut crowded = Grid::new().with(10, at(1, 0));
    for id in 0..9 {
        crowded = crowded.with(20 + id, at(3, i32::try_from(id).expect("id") - 4));
    }
    assert_eq!(
        hit_ids(&cast(&spell, &crowded, ChainStart::default()).expect("cast")).len(),
        7
    );
}

/// §6 test 7.
#[test]
fn lightning_chains_at_a_target_and_not_by_direction() {
    let spell = chain_spell("lightning", true);
    let world = Grid::new()
        .with(10, at(3, 0))
        .with(11, at(0, 1))
        .with(12, at(5, 0))
        .with(13, at(7, 0));
    // By direction: one plain hit on the tile in front, no chain.
    let direction = cast(&spell, &world, ChainStart::default()).expect("direction cast");
    assert!(direction.chain.is_empty());
    assert_eq!(direction.effects.len(), 1);
    let plain = resolve_cast(
        &spell,
        &sorcerer(),
        &Cooldowns::default(),
        SemanticTimeMicros::from_micros(0),
        false,
        &mut lowest,
    )
    .expect("a direction cast needs no world");
    assert!(plain.chain.is_empty());
    // At a target: the target, then 2 more, closest first.
    let targeted = ChainStart {
        target: Some(10),
        attacked: None,
    };
    assert_eq!(
        hit_ids(&cast(&spell, &world, targeted).expect("cast")),
        vec![10, 12, 13]
    );
    assert_eq!(
        resolve_cast(
            &spell,
            &sorcerer(),
            &Cooldowns::default(),
            SemanticTimeMicros::from_micros(0),
            true,
            &mut lowest,
        ),
        Err(CastRejection::ChainWorldRequired)
    );
}

/// §3 step 4 holds for the cast target too: a target the caster cannot reach fails the cast before
/// anything is spent.
#[test]
fn an_unreachable_cast_target_fails_the_cast() {
    let spell = chain_spell("lightning", true);
    let target = ChainStart {
        target: Some(10),
        attacked: None,
    };
    let reachable = Grid::new().with(10, at(3, 0)).with(11, at(4, 0));
    assert_eq!(
        hit_ids(&cast(&spell, &reachable, target).expect("cast")),
        vec![10, 11]
    );
    // Behind a wall.
    let mut walled = Grid::new().with(10, at(3, 0)).with(11, at(4, 0));
    walled.walls.insert(at(2, 0));
    assert_eq!(
        cast(&spell, &walled, target),
        Err(CastRejection::NoChainTarget)
    );
    // On another floor.
    let other_floor = Grid::new()
        .with(
            10,
            TilePosition {
                x: 1,
                y: 0,
                floor: 6,
            },
        )
        .with(11, at(1, 1));
    assert_eq!(
        cast(&spell, &other_floor, target),
        Err(CastRejection::NoChainTarget)
    );
    // Beyond the cast range of 3.
    let far = Grid::new().with(10, at(4, 0)).with(11, at(1, 0));
    assert_eq!(
        cast(&spell, &far, target),
        Err(CastRejection::NoChainTarget)
    );
    // The caster itself.
    let own = ChainStart {
        target: Some(CASTER),
        attacked: None,
    };
    assert_eq!(
        cast(&spell, &reachable, own),
        Err(CastRejection::NoChainTarget)
    );
}

/// §6 test 9.
#[test]
fn the_chain_effect_shows_on_each_path_tile() {
    let spell = with_binding(chain_spell("chained penance", false));
    let world = Grid::new().with(10, at(3, 0)).with(11, at(3, 3));
    let resolution = cast(&spell, &world, ChainStart::default()).expect("cast");
    let tiles: Vec<_> = resolution
        .chain
        .iter()
        .map(|hit| hit.hit.effect_tiles.clone())
        .collect();
    // The caster to 10 and 10 to 11: the path tiles up to the creature, then its tile.
    assert_eq!(tiles[0], vec![at(1, 0), at(2, 0), at(3, 0)]);
    assert_eq!(tiles[1], vec![at(3, 1), at(3, 2), at(3, 3)]);
    // Without a binding nothing is shown.
    let quiet = cast(
        &chain_spell("chained penance", false),
        &world,
        ChainStart::default(),
    )
    .expect("cast");
    assert!(
        quiet
            .chain
            .iter()
            .all(|hit| hit.hit.effect_tiles.is_empty())
    );
}

/// Monster D12 rounding (B2): exact integer, half away from zero, at least 0.
#[test]
fn a_step_rounds_half_away_from_zero_in_integers() {
    assert_eq!(step_value(101, 1, -5), 96); // 95.95
    assert_eq!(step_value(101, 2, -5), 91); // 90.9
    assert_eq!(step_value(10, 1, -5), 10); // 9.5
    assert_eq!(step_value(-10, 1, -5), -10);
    assert_eq!(step_value(101, 0, -5), 101);
    assert_eq!(step_value(100, 21, -5), 0);
    assert_eq!(step_value(100, 2, 10), 120);
    assert_eq!(step_value(i64::MAX, 1, 100), i64::MAX);
}

/// Canary `doCombatChain`: `round(value * max(0, 1 - reduction / 100 * i))` in doubles. The
/// integer rule gives the same value for every step Chained Penance reaches (with a Wheel +1).
#[test]
fn the_integer_step_matches_canary_for_chained_penance() {
    fn canary(value: i64, reduction: u8, step: u32) -> i64 {
        let multiplier = (1.0 - (f64::from(reduction) / 100.0 * f64::from(step))).max(0.0);
        (value as f64 * multiplier).round() as i64
    }
    for step in 0..=5 {
        for value in 0..=200_000 {
            assert_eq!(
                step_value(value, step, -5),
                canary(value, 5, step),
                "{value} at step {step}"
            );
            assert_eq!(step_value(-value, step, -5), canary(-value, 5, step));
        }
    }
}

#[test]
fn a_chain_cast_becomes_one_plan_per_hit() {
    let spell = chain_spell("chained penance", false);
    let world = Grid::new().with(10, at(1, 0)).with(11, at(2, 0));
    let resolution = cast(&spell, &world, ChainStart::default()).expect("cast");
    let revisions = RevisionSet::new(
        "ruleset:r1",
        "content:chain",
        "world:r1",
        "formula:s5",
        "simulation:r1",
    )
    .expect("revisions");
    let occurrence = AbilityOccurrence::new("cast:chain", revisions).expect("occurrence");
    assert_eq!(
        effect_plan(
            &spell,
            &resolution,
            "actor:1",
            None,
            occurrence.clone(),
            "channel:test"
        ),
        Err(CastPlanError::WrongPlanKind)
    );
    let plans =
        chain_plans(&spell, &resolution, "actor:1", &occurrence, "channel:test").expect("plans");
    assert_eq!(plans.len(), 2);
    let mut engine = AbilityEngine::new();
    for (plan, hit) in plans.iter().zip(&resolution.chain) {
        let effects = plan.plan.effects.clone().expect("damage");
        assert_eq!(
            effects.occurrence().id().as_str(),
            format!("cast:chain/chain-{}", hit.hit.step)
        );
        assert_eq!(plan.delay_micros, hit.hit.delay_micros);
        assert!(engine.commit(effects).expect("commit").applied());
    }
    assert_eq!(
        engine.fixture_health("actor:10"),
        Some(-damage(&resolution.chain[0].effects))
    );
    assert_eq!(
        engine.fixture_health("actor:11"),
        Some(-damage(&resolution.chain[1].effects))
    );
}
