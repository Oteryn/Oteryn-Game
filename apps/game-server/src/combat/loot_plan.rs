//! VSL-COMBAT-01 Combat D2a: a pure, deterministic loot plan for one creature
//! death.
//!
//! This module computes *what* a death drops. It has no durability, no
//! foundation wiring and no XP: it never constructs a real
//! `foundation::CreatureDeathOccurrenceKey`, never calls
//! `durability::item_mint`, and never loads content. Converting a real
//! committed death into [`LootPlanDeathKey`], loading the loot table content
//! (`content::reference_playable::ReferenceLootDefinition`, validated at
//! content-load time by `validate_loot_definition`,
//! `apps/game-server/src/content/reference_playable.rs:1946-1980`) into
//! [`LootTableDefinition`], and turning a [`LootPlan`] into
//! `ItemMintCause`/`ItemMintRequest` calls, is production wiring left to a
//! later stage (D2b).
//!
//! This module intentionally does not import `crate::content` or
//! `crate::durability`: `combat.rs` (and so this submodule) is also compiled,
//! unchanged, as a standalone Foundation test crate
//! (`foundation/mod.rs`'s `#[path = "../combat.rs"] mod
//! exact_actor_test_combat;`, and several `apps/game-server/tests/*.rs`
//! Postgres/composition harnesses that re-declare `foundation` at their own
//! crate root), none of which also declare a `content` module. `[LootTableDefinition`],
//! [`LootTableEntry`], [`LootSelectionAlgorithm`] and [`LootDefinitionRef`]
//! are this module's own minimal shapes, deliberately field-compatible with
//! `content::reference_playable::{ReferenceLootDefinition, ReferenceLootEntry,
//! ReferenceLootSelectionAlgorithm, TypedDefinitionRef}` (same fields,
//! same algorithm names) so D2b's conversion from the real content types is a
//! straight field copy, and with `durability::item_mint::TypedDefinitionRef`
//! (`family`/`production_key`/`revision_ref`, all `String`) so its own
//! conversion into `ItemMintCause`/`ItemMintRequest` is too. This mirrors the
//! precedent `durability::item_mint::TypedDefinitionRef` itself already set
//! for the same reason (it does not import `content::TypedDefinitionRef`
//! either).
//!
//! Every [`LootPlanEntry`] this module returns maps 1:1 onto the inputs
//! `durability::item_mint::ItemMintCause::from_creature_death` and
//! `ItemMintRequest` take besides the death key and the loot table's own ref
//! (carried once on [`LootPlan::loot_table`], since it is shared by every
//! entry): an item ref and `quantity` (`ItemMintRequest`), and a
//! `purpose_key` and `draw_ordinal` (`ItemMintCause`).
//!
//! Algorithm (VSL-COMBAT-01 §19 / evidence packet Q2, content schema
//! `algorithm: "IndependentBernoulliPpm"`): one independent chance draw and
//! one quantity draw per loot-table entry, both deterministically derived
//! from the death key, the loot table's own identity and the entry's ordinal
//! (`COMBAT01-LOOT-RNG-DRAWS`), so a replayed death always produces the same
//! plan.
//!
//! Resource rows enforced here
//! (`docs/contracts/RESOURCE_LIMITS_REGISTRY.json`, VSL-COMBAT-01 resource
//! rows decision D77, §4.1/§4.1.1): `COMBAT01-LOOT-PLAN-ENTRIES`,
//! `COMBAT01-LOOT-PLAN-ITEMS`, `COMBAT01-LOOT-PLAN-BYTES` and
//! `COMBAT01-LOOT-RNG-DRAWS`.

use sha2::{Digest, Sha256};

/// `COMBAT01-LOOT-PLAN-ENTRIES`: at most 16 loot-table entries can be planned
/// for one death (D77).
pub(crate) const COMBAT01_LOOT_PLAN_ENTRIES_MAX: usize = 16;
/// `COMBAT01-LOOT-PLAN-ITEMS`: at most 16 `ItemInstance`s per death (one MINT
/// per plan entry; a stack is one instance with a quantity) (D77).
pub(crate) const COMBAT01_LOOT_PLAN_ITEMS_MAX: usize = 16;
/// `COMBAT01-LOOT-PLAN-BYTES`: the plan's typed-identity footprint, derived
/// the way `durability/item_mint.rs` encodes a `TypedDefinitionRef` and a
/// content key (VSL-COMBAT-01 §4.1.1): `16 * 1,792 B + 2,048 B = 30,720 B`.
pub(crate) const COMBAT01_LOOT_PLAN_BYTES_MAX: usize = 30_720;
/// `COMBAT01-LOOT-RNG-DRAWS`: one chance draw and one quantity draw per
/// loot-table entry, at most 16 entries (32 draws), checked before planning.
pub(crate) const COMBAT01_LOOT_RNG_DRAWS_MAX: usize = 32;

/// Content schema `probability_ppm` scale (parts per million), matching
/// `content::reference_playable::REFERENCE_LOOT_PROBABILITY_PPM_SCALE`.
const LOOT_PROBABILITY_PPM_SCALE: u32 = 1_000_000;

/// §4.1.1: a definition ref's `family` technical field is charged at its
/// registered `DUR03-RL-07-TECHNICAL-FIELD-BYTES` maximum (128 B) regardless
/// of the actual (always short) family text.
const DEFINITION_FAMILY_FIELD_BYTES: usize = 128;
/// §4.1.1 entry, 1,792 B at maximum content-key width (512 B): the family
/// field above, plus 128 B for length prefixes and fixed fields (draw
/// ordinal, quantity, chance and quantity draws), plus the item's
/// `production_key`, `revision_ref` and the `purpose_key` (each up to 512 B).
const ENTRY_FIXED_OVERHEAD_BYTES: usize = DEFINITION_FAMILY_FIELD_BYTES + 128;
/// §4.1.1 header, 2,048 B at maximum content-key width: the family field
/// above, plus 896 B for the death key, length prefixes and fixed fields,
/// plus the loot table's `production_key` and `revision_ref`.
const HEADER_FIXED_OVERHEAD_BYTES: usize = DEFINITION_FAMILY_FIELD_BYTES + 896;

/// This module's own minimal, `Clone`/`PartialEq` typed-identity shape:
/// field-compatible with both `content::reference_playable::TypedDefinitionRef`
/// (`family`/`key`/`revision`) and `durability::item_mint::TypedDefinitionRef`
/// (`family`/`production_key`/`revision_ref`, all `String`) without importing
/// either (see module docs).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LootDefinitionRef {
    pub(crate) family: String,
    pub(crate) production_key: String,
    pub(crate) revision_ref: String,
}

impl LootDefinitionRef {
    pub(crate) fn new(
        family: impl Into<String>,
        production_key: impl Into<String>,
        revision_ref: impl Into<String>,
    ) -> Self {
        Self {
            family: family.into(),
            production_key: production_key.into(),
            revision_ref: revision_ref.into(),
        }
    }
}

/// Mirrors `content::reference_playable::ReferenceLootSelectionAlgorithm`
/// (same variant names); only `IndependentBernoulliPpm` is implemented here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LootSelectionAlgorithm {
    IndependentBernoulliPpm,
    WeightedSingleSelection,
    GuaranteedEntries,
    NestedGroups,
}

/// Mirrors `content::reference_playable::ReferenceLootEntry`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LootTableEntry {
    pub(crate) item: LootDefinitionRef,
    pub(crate) min_count: u32,
    pub(crate) max_count: u32,
    pub(crate) probability_ppm: Option<u32>,
}

/// Mirrors `content::reference_playable::ReferenceLootDefinition`: the
/// already-loaded, already-validated loot table this module plans from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LootTableDefinition {
    pub(crate) algorithm: LootSelectionAlgorithm,
    pub(crate) entries: Vec<LootTableEntry>,
}

/// One accepted output of the loot plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LootPlanEntry {
    pub(crate) item: LootDefinitionRef,
    pub(crate) quantity: u32,
    pub(crate) draw_ordinal: u32,
    pub(crate) purpose_key: String,
}

/// The complete, bounded loot plan of one death: at most
/// `COMBAT01_LOOT_PLAN_ENTRIES_MAX` entries, already within
/// `COMBAT01_LOOT_PLAN_BYTES_MAX` of typed-identity footprint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LootPlan {
    pub(crate) loot_table: LootDefinitionRef,
    pub(crate) entries: Vec<LootPlanEntry>,
}

/// Fails closed. Every variant is checked before allocating plan storage; a
/// rejected plan is never truncated to fit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LootPlanError {
    /// `COMBAT01-LOOT-PLAN-ENTRIES`: the source loot table has more entries
    /// than one death may ever plan.
    TooManyTableEntries,
    /// `COMBAT01-LOOT-RNG-DRAWS`: planning the table would need more than 32
    /// deterministic draws.
    TooManyRngDraws,
    /// `COMBAT01-LOOT-PLAN-ITEMS`: the plan accepted more entries than one
    /// death may mint.
    TooManyPlanItems,
    /// `COMBAT01-LOOT-PLAN-BYTES`: the plan's typed-identity footprint
    /// exceeds the registered ceiling.
    PlanBytesExceeded,
    /// The loot table's algorithm is not the one this D2a slice implements
    /// (`IndependentBernoulliPpm`).
    UnsupportedAlgorithm,
    /// An entry's `min_count`/`max_count` is zero or out of order.
    InvalidQuantityRange,
    /// An `IndependentBernoulliPpm` entry has no `probability_ppm`, or one
    /// above the one-million ppm scale.
    InvalidProbability,
}

impl std::fmt::Display for LootPlanError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooManyTableEntries => {
                formatter.write_str("loot table exceeds COMBAT01-LOOT-PLAN-ENTRIES")
            }
            Self::TooManyRngDraws => {
                formatter.write_str("loot table exceeds COMBAT01-LOOT-RNG-DRAWS")
            }
            Self::TooManyPlanItems => {
                formatter.write_str("loot plan exceeds COMBAT01-LOOT-PLAN-ITEMS")
            }
            Self::PlanBytesExceeded => {
                formatter.write_str("loot plan exceeds COMBAT01-LOOT-PLAN-BYTES")
            }
            Self::UnsupportedAlgorithm => {
                formatter.write_str("loot table algorithm is not IndependentBernoulliPpm")
            }
            Self::InvalidQuantityRange => {
                formatter.write_str("loot entry has a zero or out-of-order quantity range")
            }
            Self::InvalidProbability => {
                formatter.write_str("loot entry has a missing or out-of-scale probability_ppm")
            }
        }
    }
}

impl std::error::Error for LootPlanError {}

/// Pure, test-constructible identity of one creature death, sufficient to key
/// the deterministic RNG: the same fields as
/// `CREATURE-DEATH-OCCURRENCE-IDENTITY-V1`
/// (`(WorldId, ChannelId, ScopeOwnershipGeneration, ActorLocalId,
/// ActorLocalGeneration)`, DUR-03 death-identity decision §4.1). D2a takes no
/// dependency on `foundation::CreatureDeathOccurrenceKey`; building this seed
/// from the real committed death is production wiring (D2b).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct LootPlanDeathKey {
    world_id: [u8; 16],
    channel_id: [u8; 16],
    scope_ownership_generation: u64,
    actor_local_id: u32,
    actor_local_generation: u64,
}

impl LootPlanDeathKey {
    pub(crate) const fn new(
        world_id: [u8; 16],
        channel_id: [u8; 16],
        scope_ownership_generation: u64,
        actor_local_id: u32,
        actor_local_generation: u64,
    ) -> Self {
        Self {
            world_id,
            channel_id,
            scope_ownership_generation,
            actor_local_id,
            actor_local_generation,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DrawKind {
    Chance,
    Quantity,
}

/// Plan a death's loot from its already-loaded, already-validated loot table.
/// Rejects before allocating plan storage; never truncates a plan that would
/// exceed a registered ceiling. Deterministic: the same `death_key`,
/// `loot_table_ref` and `loot_table` always produce the same [`LootPlan`], so
/// replay is idempotent.
pub(crate) fn plan_creature_loot(
    death_key: LootPlanDeathKey,
    loot_table_ref: &LootDefinitionRef,
    loot_table: &LootTableDefinition,
) -> Result<LootPlan, LootPlanError> {
    if loot_table.algorithm != LootSelectionAlgorithm::IndependentBernoulliPpm {
        return Err(LootPlanError::UnsupportedAlgorithm);
    }
    if loot_table.entries.len() > COMBAT01_LOOT_PLAN_ENTRIES_MAX {
        return Err(LootPlanError::TooManyTableEntries);
    }
    let draws_needed = loot_table
        .entries
        .len()
        .checked_mul(2)
        .ok_or(LootPlanError::TooManyRngDraws)?;
    if draws_needed > COMBAT01_LOOT_RNG_DRAWS_MAX {
        return Err(LootPlanError::TooManyRngDraws);
    }
    for entry in &loot_table.entries {
        validate_entry_shape(entry)?;
    }

    let mut total_bytes = charge_plan_bytes(0, header_footprint_bytes(loot_table_ref))?;
    let mut entries = Vec::with_capacity(loot_table.entries.len());

    for (index, entry) in loot_table.entries.iter().enumerate() {
        let draw_ordinal = index as u32;
        // Validated by `validate_entry_shape` above for every entry.
        let probability_ppm = match entry.probability_ppm {
            Some(value) => value,
            None => return Err(LootPlanError::InvalidProbability),
        };

        let chance_draw =
            deterministic_draw(death_key, loot_table_ref, draw_ordinal, DrawKind::Chance);
        if !bernoulli_success(chance_draw, probability_ppm) {
            continue;
        }

        let quantity_draw =
            deterministic_draw(death_key, loot_table_ref, draw_ordinal, DrawKind::Quantity);
        let quantity = quantity_in_range(quantity_draw, entry.min_count, entry.max_count);
        let purpose_key = entry.item.production_key.clone();

        total_bytes = charge_plan_bytes(
            total_bytes,
            entry_footprint_bytes(&entry.item, &purpose_key),
        )?;

        entries.push(LootPlanEntry {
            item: entry.item.clone(),
            quantity,
            draw_ordinal,
            purpose_key,
        });
    }

    if entries.len() > COMBAT01_LOOT_PLAN_ITEMS_MAX {
        return Err(LootPlanError::TooManyPlanItems);
    }

    Ok(LootPlan {
        loot_table: loot_table_ref.clone(),
        entries,
    })
}

fn validate_entry_shape(entry: &LootTableEntry) -> Result<(), LootPlanError> {
    if entry.min_count == 0 || entry.max_count < entry.min_count {
        return Err(LootPlanError::InvalidQuantityRange);
    }
    match entry.probability_ppm {
        Some(value) if value <= LOOT_PROBABILITY_PPM_SCALE => Ok(()),
        _ => Err(LootPlanError::InvalidProbability),
    }
}

/// `COMBAT01-LOOT-PLAN-BYTES`: add `additional` bytes to the plan's running
/// footprint, rejecting before the running total (or the addition itself)
/// would exceed the registered ceiling.
fn charge_plan_bytes(total_bytes: usize, additional: usize) -> Result<usize, LootPlanError> {
    let updated = total_bytes
        .checked_add(additional)
        .ok_or(LootPlanError::PlanBytesExceeded)?;
    if updated > COMBAT01_LOOT_PLAN_BYTES_MAX {
        return Err(LootPlanError::PlanBytesExceeded);
    }
    Ok(updated)
}

fn entry_footprint_bytes(item: &LootDefinitionRef, purpose_key: &str) -> usize {
    ENTRY_FIXED_OVERHEAD_BYTES
        + item.production_key.len()
        + item.revision_ref.len()
        + purpose_key.len()
}

fn header_footprint_bytes(loot_table_ref: &LootDefinitionRef) -> usize {
    HEADER_FIXED_OVERHEAD_BYTES
        + loot_table_ref.production_key.len()
        + loot_table_ref.revision_ref.len()
}

/// `success = draw_ppm < probability_ppm`, `draw_ppm` uniform over
/// `0..1,000,000`: `probability_ppm == 0` never succeeds, and
/// `probability_ppm == 1,000,000` always succeeds.
fn bernoulli_success(draw: u64, probability_ppm: u32) -> bool {
    let draw_ppm = (draw % u64::from(LOOT_PROBABILITY_PPM_SCALE)) as u32;
    draw_ppm < probability_ppm
}

fn quantity_in_range(draw: u64, min_count: u32, max_count: u32) -> u32 {
    let span = u64::from(max_count - min_count) + 1;
    min_count + (draw % span) as u32
}

/// One deterministic SHA-256 counter-mode draw, keyed by the death, the loot
/// table's identity, the entry ordinal and the draw kind. Reuses the `sha2`
/// dependency `durability/item_mint.rs` already carries; no new crate.
fn deterministic_draw(
    death_key: LootPlanDeathKey,
    loot_table_ref: &LootDefinitionRef,
    draw_ordinal: u32,
    kind: DrawKind,
) -> u64 {
    let mut hasher = Sha256::new();
    hasher.update(death_key.world_id);
    hasher.update(death_key.channel_id);
    hasher.update(death_key.scope_ownership_generation.to_be_bytes());
    hasher.update(death_key.actor_local_id.to_be_bytes());
    hasher.update(death_key.actor_local_generation.to_be_bytes());
    // Length-prefixed so distinct typed references never frame to the same bytes.
    for field in [
        loot_table_ref.family.as_bytes(),
        loot_table_ref.production_key.as_bytes(),
        loot_table_ref.revision_ref.as_bytes(),
    ] {
        hasher.update((field.len() as u64).to_be_bytes());
        hasher.update(field);
    }
    hasher.update(draw_ordinal.to_be_bytes());
    hasher.update([match kind {
        DrawKind::Chance => 0_u8,
        DrawKind::Quantity => 1_u8,
    }]);
    let digest = hasher.finalize();
    let mut buf = [0_u8; 8];
    buf.copy_from_slice(&digest[0..8]);
    u64::from_be_bytes(buf)
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    fn item_ref(key: &str) -> LootDefinitionRef {
        LootDefinitionRef::new("Item", key, "definition-r1")
    }

    fn table_ref(key: &str) -> LootDefinitionRef {
        LootDefinitionRef::new("Loot", key, "definition-r1")
    }

    fn death_key(seed: u8) -> LootPlanDeathKey {
        LootPlanDeathKey::new(
            [seed; 16],
            [seed.wrapping_add(1); 16],
            u64::from(seed) + 1,
            u32::from(seed) + 1,
            u64::from(seed) + 2,
        )
    }

    fn one_entry_table(
        item_key: &str,
        min_count: u32,
        max_count: u32,
        probability_ppm: Option<u32>,
    ) -> LootTableDefinition {
        LootTableDefinition {
            algorithm: LootSelectionAlgorithm::IndependentBernoulliPpm,
            entries: vec![LootTableEntry {
                item: item_ref(item_key),
                min_count,
                max_count,
                probability_ppm,
            }],
        }
    }

    fn many_entries_table(count: usize, probability_ppm: u32) -> LootTableDefinition {
        LootTableDefinition {
            algorithm: LootSelectionAlgorithm::IndependentBernoulliPpm,
            entries: (0..count)
                .map(|index| LootTableEntry {
                    item: item_ref(&format!("oteryn:item.loot-bound-{index}")),
                    min_count: 1,
                    max_count: 1,
                    probability_ppm: Some(probability_ppm),
                })
                .collect(),
        }
    }

    fn rat_loot_table() -> LootTableDefinition {
        LootTableDefinition {
            algorithm: LootSelectionAlgorithm::IndependentBernoulliPpm,
            entries: vec![
                LootTableEntry {
                    item: item_ref("oteryn:item.tibia.i3607"),
                    min_count: 1,
                    max_count: 1,
                    probability_ppm: Some(356_433),
                },
                LootTableEntry {
                    item: item_ref("oteryn:item.tibia.i3031"),
                    min_count: 1,
                    max_count: 4,
                    probability_ppm: Some(903_036),
                },
            ],
        }
    }

    const GOLDEN_CHANCE: u64 = 10_348_850_681_732_701_394;
    const GOLDEN_QUANTITY: u64 = 9_517_304_501_960_542_980;
    const GOLDEN_RAT_PLAN: &str = r#"[("oteryn:item.tibia.i3031", 1, 1)]"#;

    #[test]
    fn deterministic_draw_is_pinned_by_golden_vectors() {
        // FND-03 §25: a change to framing, byte order, truncation or the hash must not
        // silently re-roll historical plans. These values are the canonical derivation.
        let table_ref = table_ref("oteryn:loot.creature.rat");
        let key = death_key(7);
        let chance = deterministic_draw(key, &table_ref, 0, DrawKind::Chance);
        let quantity = deterministic_draw(key, &table_ref, 1, DrawKind::Quantity);
        assert_eq!((chance, quantity), (GOLDEN_CHANCE, GOLDEN_QUANTITY));
        let plan = plan_creature_loot(key, &table_ref, &rat_loot_table()).expect("rat plan");
        let summary: Vec<(String, u32, u32)> = plan
            .entries
            .iter()
            .map(|entry| {
                (
                    entry.item.production_key.clone(),
                    entry.quantity,
                    entry.draw_ordinal,
                )
            })
            .collect();
        assert_eq!(format!("{summary:?}"), GOLDEN_RAT_PLAN);
    }

    #[test]
    fn distinct_reference_framings_do_not_collide() {
        let key = death_key(7);
        let left = LootDefinitionRef::new("Loot", "a:b", "cd");
        let right = LootDefinitionRef::new("Loot", "a:bc", "d");
        assert_ne!(
            deterministic_draw(key, &left, 0, DrawKind::Chance),
            deterministic_draw(key, &right, 0, DrawKind::Chance)
        );
    }

    #[test]
    fn same_death_key_produces_the_same_plan() {
        let table_ref = table_ref("oteryn:loot.creature.rat");
        let table = rat_loot_table();
        let key = death_key(7);
        let first = plan_creature_loot(key, &table_ref, &table).expect("first plan");
        let second = plan_creature_loot(key, &table_ref, &table).expect("second plan (replay)");
        assert_eq!(first, second);
    }

    #[test]
    fn different_death_keys_can_produce_a_different_plan() {
        let table_ref = table_ref("oteryn:loot.creature.rat");
        let table = rat_loot_table();
        let plans: Vec<LootPlan> = (0..8_u8)
            .map(|seed| {
                plan_creature_loot(death_key(seed), &table_ref, &table).expect("plan for seed")
            })
            .collect();
        assert!(
            plans.windows(2).any(|pair| pair[0] != pair[1]),
            "expected at least one differing plan across distinct death keys"
        );
    }

    #[test]
    fn sixteen_table_entries_accepted_seventeen_rejected() {
        let table_ref = table_ref("oteryn:loot.bound.table");
        let ok = plan_creature_loot(
            death_key(3),
            &table_ref,
            &many_entries_table(COMBAT01_LOOT_PLAN_ENTRIES_MAX, 0),
        );
        assert!(ok.is_ok());

        let rejected = plan_creature_loot(
            death_key(3),
            &table_ref,
            &many_entries_table(COMBAT01_LOOT_PLAN_ENTRIES_MAX + 1, 0),
        );
        assert_eq!(rejected, Err(LootPlanError::TooManyTableEntries));
    }

    #[test]
    fn sixteen_entries_use_exactly_the_registered_rng_draw_budget() {
        let table = many_entries_table(COMBAT01_LOOT_PLAN_ENTRIES_MAX, 0);
        assert_eq!(table.entries.len() * 2, COMBAT01_LOOT_RNG_DRAWS_MAX);
    }

    #[test]
    fn plan_bytes_budget_accepts_exact_limit_and_rejects_one_byte_over() {
        assert!(charge_plan_bytes(0, COMBAT01_LOOT_PLAN_BYTES_MAX).is_ok());
        assert_eq!(
            charge_plan_bytes(0, COMBAT01_LOOT_PLAN_BYTES_MAX + 1),
            Err(LootPlanError::PlanBytesExceeded)
        );
        assert_eq!(
            charge_plan_bytes(COMBAT01_LOOT_PLAN_BYTES_MAX, 1),
            Err(LootPlanError::PlanBytesExceeded)
        );
    }

    #[test]
    fn sixteen_maximum_width_entries_fit_exactly_at_the_registered_byte_ceiling() {
        // Mirrors VSL-COMBAT-01 §4.1.1's own worst case: every content key at
        // its FIRST_PRODUCTION_MAX_KEY_BYTES/ATOM_BYTES ceiling (512 B).
        let max_key = |prefix: &str| format!("{prefix}:{}", "k".repeat(512 - prefix.len() - 1));
        let max_revision = "r".repeat(512);
        let table_ref = LootDefinitionRef::new("Loot", max_key("loot"), max_revision.clone());
        let entries = (0..COMBAT01_LOOT_PLAN_ENTRIES_MAX)
            .map(|index| LootTableEntry {
                item: LootDefinitionRef::new(
                    "Item",
                    max_key(&format!("item{index}")),
                    max_revision.clone(),
                ),
                min_count: 1,
                max_count: 1,
                probability_ppm: Some(1_000_000),
            })
            .collect();
        let table = LootTableDefinition {
            algorithm: LootSelectionAlgorithm::IndependentBernoulliPpm,
            entries,
        };

        let plan = plan_creature_loot(death_key(9), &table_ref, &table).expect("max-width plan");
        assert_eq!(plan.entries.len(), COMBAT01_LOOT_PLAN_ENTRIES_MAX);

        let total_bytes = charge_plan_bytes(0, header_footprint_bytes(&table_ref))
            .and_then(|running| {
                plan.entries.iter().try_fold(running, |running, entry| {
                    charge_plan_bytes(
                        running,
                        entry_footprint_bytes(&entry.item, &entry.purpose_key),
                    )
                })
            })
            .expect("max-width plan fits the registered byte ceiling");
        assert_eq!(total_bytes, COMBAT01_LOOT_PLAN_BYTES_MAX);
    }

    #[test]
    fn zero_chance_entry_never_drops_and_max_chance_entry_always_drops() {
        let table_ref = table_ref("oteryn:loot.chance.table");
        let table = LootTableDefinition {
            algorithm: LootSelectionAlgorithm::IndependentBernoulliPpm,
            entries: vec![
                LootTableEntry {
                    item: item_ref("oteryn:item.chance.never"),
                    min_count: 1,
                    max_count: 1,
                    probability_ppm: Some(0),
                },
                LootTableEntry {
                    item: item_ref("oteryn:item.chance.always"),
                    min_count: 1,
                    max_count: 1,
                    probability_ppm: Some(1_000_000),
                },
            ],
        };
        for seed in 0..32_u8 {
            let plan = plan_creature_loot(death_key(seed), &table_ref, &table).expect("plan");
            assert!(
                plan.entries
                    .iter()
                    .all(|entry| entry.purpose_key != "oteryn:item.chance.never")
            );
            assert!(
                plan.entries
                    .iter()
                    .any(|entry| entry.purpose_key == "oteryn:item.chance.always")
            );
        }
    }

    #[test]
    fn quantity_draw_stays_within_the_entry_range() {
        let table_ref = table_ref("oteryn:loot.quantity.table");
        let table = one_entry_table("oteryn:item.quantity.stack", 3, 9, Some(1_000_000));
        for seed in 0..40_u8 {
            let plan = plan_creature_loot(death_key(seed), &table_ref, &table).expect("plan");
            let entry = plan.entries.first().expect("guaranteed entry");
            assert!((3..=9).contains(&entry.quantity));
        }
    }

    #[test]
    fn rat_loot_table_produces_a_valid_plan() {
        let table_ref = table_ref("oteryn:loot.creature.rat");
        let table = rat_loot_table();
        let plan = plan_creature_loot(death_key(42), &table_ref, &table).expect("rat plan");
        assert!(plan.entries.len() <= COMBAT01_LOOT_PLAN_ENTRIES_MAX);
        for entry in &plan.entries {
            assert!(entry.quantity >= 1);
            assert!(entry.quantity <= 4);
        }
    }

    #[test]
    fn unsupported_algorithm_is_rejected() {
        let table_ref = table_ref("oteryn:loot.unsupported.table");
        let table = LootTableDefinition {
            algorithm: LootSelectionAlgorithm::GuaranteedEntries,
            entries: vec![],
        };
        assert_eq!(
            plan_creature_loot(death_key(1), &table_ref, &table),
            Err(LootPlanError::UnsupportedAlgorithm)
        );
    }

    #[test]
    fn zero_or_inverted_quantity_range_is_rejected() {
        let table_ref = table_ref("oteryn:loot.invalid-quantity.table");
        let zero_min = one_entry_table("oteryn:item.invalid.zero", 0, 1, Some(1));
        assert_eq!(
            plan_creature_loot(death_key(1), &table_ref, &zero_min),
            Err(LootPlanError::InvalidQuantityRange)
        );

        let inverted = one_entry_table("oteryn:item.invalid.inverted", 5, 1, Some(1));
        assert_eq!(
            plan_creature_loot(death_key(1), &table_ref, &inverted),
            Err(LootPlanError::InvalidQuantityRange)
        );
    }

    #[test]
    fn missing_or_out_of_scale_probability_is_rejected() {
        let table_ref = table_ref("oteryn:loot.invalid-probability.table");
        let missing = one_entry_table("oteryn:item.invalid.missing-ppm", 1, 1, None);
        assert_eq!(
            plan_creature_loot(death_key(1), &table_ref, &missing),
            Err(LootPlanError::InvalidProbability)
        );

        let over_scale = one_entry_table("oteryn:item.invalid.over-ppm", 1, 1, Some(1_000_001));
        assert_eq!(
            plan_creature_loot(death_key(1), &table_ref, &over_scale),
            Err(LootPlanError::InvalidProbability)
        );
    }
}
