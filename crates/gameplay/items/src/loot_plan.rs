//! Deterministic creature loot planning. Independent rolls retain their original entropy;
//! nested children are flattened after their parent so the kill owner can reserve every MINT
//! and commit a container tree atomically. A failed container roll suppresses its descendants.

use sha2::{Digest, Sha256};

use crate::stacking::stack_maximum;
use oteryn_gameplay_domain::domain::item_definition::ItemStackClass;

#[cfg(test)]
use crate::item_transfer::GAMEITEM01_STACK_QUANTITY_MAX;

mod boss;
pub use boss::{plan_boss_loot, plan_boss_loot_with_bonus};

/// Source entries per death, including descendants (the native section record bound).
pub const COMBAT01_LOOT_PLAN_ENTRIES_MAX: usize = 4096;
/// Maximum MINTs per death, including nested children and additional daily rolls.
pub const COMBAT01_LOOT_PLAN_ITEMS_MAX: usize = 4096;
/// Maximum typed-identity footprint: 4,096 maximum-width entries plus the header.
pub const COMBAT01_LOOT_PLAN_BYTES_MAX: usize = 4096 * 1792 + 2048;
/// Two deterministic draws per source entry: the chance (which also yields the count) and the
/// loot factor.
pub const COMBAT01_LOOT_RNG_DRAWS_MAX: usize = 8192;

// The one-shot direct drop keeps its MINT cause across ordinary table changes.
// Successful ordinary plans cannot reach this ordinal, even with split stacks.
const DIRECT_CORPSE_DROP_MINT_ORDINAL: u32 = u32::MAX;

const DAILY_BOOSTED_EXTRA_ROLLS_MAX: u8 = 3;
// Reserve a disjoint ordinal range for each additional stack, including across boosted rolls.
const STACK_MINT_ORDINAL_STRIDE: u32 =
    COMBAT01_LOOT_PLAN_ENTRIES_MAX as u32 * (1 + DAILY_BOOSTED_EXTRA_ROLLS_MAX as u32);

/// Content schema `probability_ppm` scale (parts per million), matching
/// `content::reference_playable::REFERENCE_LOOT_PROBABILITY_PPM_SCALE`.
const LOOT_PROBABILITY_PPM_SCALE: u32 = 1_000_000;
/// Donor `MAX_LOOTCHANCE` (100,000) is a tenth of the ppm scale; the count is taken from the
/// chance draw at the donor's resolution.
const DONOR_LOOT_CHANCE_DIVISOR: u32 = 10;
/// Donor per-entry loot factor `math.random(95, 105) / 100`, inclusive.
///
const LOOT_FACTOR_PERCENT_MIN: u32 = 95;
const LOOT_FACTOR_PERCENT_SPAN: u64 = 11;

/// Global's blue and red anniversary surprise bags. Crystal can inject these
/// into an arbitrary monster's corpse when its server-wide switch is enabled;
/// Oteryn keeps them attached to the admitted Global event creature tables.
const GLOBAL_EVENT_SURPRISE_BAGS: [&str; 2] =
    ["oteryn:item.tibia.i6570", "oteryn:item.tibia.i6571"];

/// Loot owners whose creatures are realized by Global event/raid content.
/// The raid schedule remains the authority that may realize those creatures;
/// this list only prevents ordinary creature loot from becoming a second,
/// all-year event-reward producer.
const GLOBAL_EVENT_SURPRISE_BAG_TABLES: [&str; 8] = [
    "oteryn:loot.creature.bones",
    "oteryn:loot.creature.chayenne",
    "oteryn:loot.creature.fluffy",
    "oteryn:loot.creature.hacker",
    "oteryn:loot.creature.pinata_dragon",
    "oteryn:loot.creature.primitive",
    "oteryn:loot.creature.the_mutated_pumpkin",
    "oteryn:loot.creature.undead_minion",
];

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

pub use oteryn_gameplay_domain::domain::loot_data::{
    LootDefinitionRef, LootSelectionAlgorithm, LootTableDefinition, LootTableEntry,
};

/// One accepted output of the loot plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LootPlanEntry {
    pub item: LootDefinitionRef,
    pub quantity: u32,
    /// The MINT ordinal of an ordinary roll, a split stack, or the one-shot direct drop.
    pub draw_ordinal: u32,
    pub purpose_key: String,
    /// Index of the parent in this flattened, parent-before-child plan; None means corpse.
    pub parent: Option<usize>,
    pub placement_ordinal: u32,
    pub subtype: Option<u16>,
    /// Full charges from the admitted Item definition (lazy timed-state default).
    pub charges: Option<u32>,
    pub unique: bool,
    /// Whether the admitted Item stacks; a stack takes its count and ignores the subtype.
    pub stackable: bool,
}

impl LootPlanEntry {
    /// The fluid this entry is minted with. The donor drops a stackable item with its count and a
    /// charged one with its full charges, and applies `subType` only to any other item, the
    /// fluid of a fluid container; subtype 0 is the empty container (Crystal 201189f8
    /// `data/libs/functions/container.lua:15-51`). Full charges are the lazy timed-state
    /// default and an empty container has no fluid row, so both give `None`.
    pub fn fluid_subtype(&self) -> Option<u8> {
        match self.subtype {
            Some(subtype) if self.charges.is_none() && !self.stackable => {
                u8::try_from(subtype).ok().filter(|fluid| *fluid != 0)
            }
            _ => None,
        }
    }

    /// Whether the subtype names a fluid beyond the client's one-byte fluid range, which the
    /// fluid state cannot hold.
    pub fn needs_subtype_state(&self) -> bool {
        self.subtype
            .is_some_and(|subtype| subtype > u16::from(u8::MAX))
            && self.charges.is_none()
            && !self.stackable
    }
}

/// The complete, bounded loot plan of one death: at most
/// `COMBAT01_LOOT_PLAN_ENTRIES_MAX` entries, already within
/// `COMBAT01_LOOT_PLAN_BYTES_MAX` of typed-identity footprint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LootPlan {
    pub loot_table: LootDefinitionRef,
    pub entries: Vec<LootPlanEntry>,
}

/// Fails closed. Every variant is checked before allocating plan storage; a
/// rejected plan is never truncated to fit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LootPlanError {
    /// `COMBAT01-LOOT-PLAN-ENTRIES`: the source loot table has more entries
    /// than one death may ever plan.
    TooManyTableEntries,
    /// `COMBAT01-LOOT-RNG-DRAWS`: planning the table would need more than the registered budget of
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
    /// The loot subtype of an uncharged, unstacked Item is beyond the one-byte fluid range.
    UnsupportedItemSubtype,
    /// An entry has an invalid roll quantity or Item stack maximum.
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
            Self::UnsupportedItemSubtype => {
                formatter.write_str("loot instance subtype requires durable state support")
            }
            Self::InvalidQuantityRange => {
                formatter.write_str("loot entry has an invalid quantity range or stack maximum")
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
pub struct LootPlanDeathKey {
    world_id: [u8; 16],
    scope_id: [u8; 16],
    instance: bool,
    scope_ownership_generation: u64,
    actor_local_id: u32,
    actor_local_generation: u64,
}

impl LootPlanDeathKey {
    /// Instance entropy has a separate namespace; the legacy Channel constructor
    /// retains its original byte framing and deterministic draws.
    pub const fn new_instance(
        world_id: [u8; 16],
        instance_id: [u8; 16],
        scope_ownership_generation: u64,
        actor_local_id: u32,
        actor_local_generation: u64,
    ) -> Self {
        Self {
            world_id,
            scope_id: instance_id,
            instance: true,
            scope_ownership_generation,
            actor_local_id,
            actor_local_generation,
        }
    }

    pub const fn new(
        world_id: [u8; 16],
        channel_id: [u8; 16],
        scope_ownership_generation: u64,
        actor_local_id: u32,
        actor_local_generation: u64,
    ) -> Self {
        Self {
            world_id,
            scope_id: channel_id,
            instance: false,
            scope_ownership_generation,
            actor_local_id,
            actor_local_generation,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DrawKind {
    Chance,
    Factor,
    BoostedChance(u8),
    BoostedFactor(u8),
}

/// Plan a death's loot from its already-loaded, already-validated loot table.
/// Rejects before allocating plan storage; never truncates a plan that would
/// exceed a registered ceiling. Deterministic: the same `death_key`,
/// `loot_table_ref` and `loot_table` always produce the same [`LootPlan`], so
/// replay is idempotent.
pub fn plan_creature_loot(
    death_key: LootPlanDeathKey,
    loot_table_ref: &LootDefinitionRef,
    loot_table: &LootTableDefinition,
) -> Result<LootPlan, LootPlanError> {
    if let LootSelectionAlgorithm::DailyBoosted {
        extra_rolls,
        boosted_entries,
    } = loot_table.algorithm
    {
        return plan_daily_boosted(
            death_key,
            loot_table_ref,
            loot_table,
            extra_rolls,
            boosted_entries,
        );
    }
    plan_roll(death_key, loot_table_ref, loot_table, 0)
}

fn plan_roll(
    death_key: LootPlanDeathKey,
    loot_table_ref: &LootDefinitionRef,
    loot_table: &LootTableDefinition,
    roll: u8,
) -> Result<LootPlan, LootPlanError> {
    if !matches!(
        loot_table.algorithm,
        LootSelectionAlgorithm::IndependentBernoulliPpm | LootSelectionAlgorithm::NestedGroups
    ) {
        return Err(LootPlanError::UnsupportedAlgorithm);
    }
    let sources = validate_tree(&loot_table.entries, 0)?;
    if sources > COMBAT01_LOOT_PLAN_ENTRIES_MAX {
        return Err(LootPlanError::TooManyTableEntries);
    }
    let mut state = RollState {
        entries: Vec::new(),
        unique: std::collections::HashSet::new(),
        ordinal: 0,
        bytes: charge_plan_bytes(0, header_footprint_bytes(loot_table_ref))?,
    };
    roll_entries(
        death_key,
        loot_table_ref,
        &loot_table.entries,
        roll,
        None,
        &mut state,
    )?;
    Ok(LootPlan {
        loot_table: loot_table_ref.clone(),
        entries: state.entries,
    })
}

fn validate_tree(entries: &[LootTableEntry], depth: usize) -> Result<usize, LootPlanError> {
    if depth > 8 || entries.len() > COMBAT01_LOOT_PLAN_ENTRIES_MAX {
        return Err(LootPlanError::TooManyTableEntries);
    }
    let mut count = entries.len();
    for entry in entries {
        validate_entry_shape(entry)?;
        if !entry.details.children.is_empty() && entry.details.container_capacity.is_none() {
            return Err(LootPlanError::UnsupportedAlgorithm);
        }
        count = count
            .checked_add(validate_tree(&entry.details.children, depth + 1)?)
            .filter(|count| *count <= COMBAT01_LOOT_PLAN_ENTRIES_MAX)
            .ok_or(LootPlanError::TooManyTableEntries)?;
    }
    Ok(count)
}

struct RollState {
    entries: Vec<LootPlanEntry>,
    unique: std::collections::HashSet<String>,
    ordinal: u32,
    bytes: usize,
}

impl RollState {
    fn push_instances(
        &mut self,
        entry: &LootTableEntry,
        chance: u64,
        draw_ordinal: u32,
        parent: Option<usize>,
        placement_ordinal: &mut u32,
    ) -> Result<(), LootPlanError> {
        let stack = if entry.details.stackable {
            ItemStackClass::Stackable {
                proven_maximum: entry
                    .details
                    .corpse_stack_maximum
                    .map(std::num::NonZeroU32::get),
            }
        } else {
            ItemStackClass::NonStackable
        };
        let (maximum, stackable) =
            stack_maximum(stack).map_err(|_| LootPlanError::InvalidQuantityRange)?;
        let quantity = match entry.details.charges {
            _ if !stackable || entry.details.container_capacity.is_some() => 1,
            Some(charges) => charges,
            None => quantity_in_range(chance, entry.min_count, entry.max_count),
        };
        if quantity == 0 || (entry.details.direct_corpse_drop && quantity > maximum) {
            return Err(LootPlanError::InvalidQuantityRange);
        }
        for stack_index in 0..quantity.div_ceil(maximum) {
            if self.entries.len() >= COMBAT01_LOOT_PLAN_ITEMS_MAX {
                return Err(LootPlanError::TooManyPlanItems);
            }
            let purpose = &entry.item.production_key;
            self.bytes =
                charge_plan_bytes(self.bytes, entry_footprint_bytes(&entry.item, purpose))?;
            *placement_ordinal += 1;
            self.entries.push(LootPlanEntry {
                item: entry.item.clone(),
                quantity: (quantity - stack_index * maximum).min(maximum),
                draw_ordinal: draw_ordinal + stack_index * STACK_MINT_ORDINAL_STRIDE,
                purpose_key: purpose.clone(),
                parent,
                placement_ordinal: *placement_ordinal,
                subtype: entry.details.subtype,
                charges: entry.details.charges,
                unique: entry.details.unique,
                stackable,
            });
        }
        Ok(())
    }
}

fn roll_entries(
    death: LootPlanDeathKey,
    reference: &LootDefinitionRef,
    source: &[LootTableEntry],
    roll: u8,
    parent: Option<usize>,
    state: &mut RollState,
) -> Result<(), LootPlanError> {
    let mut placement_ordinal = 0;
    for entry in source {
        let draw_ordinal = state.ordinal;
        state.ordinal += 1;
        if entry.details.direct_corpse_drop {
            if roll == 0 {
                if parent.is_some()
                    || state
                        .entries
                        .iter()
                        .any(|entry| entry.draw_ordinal == DIRECT_CORPSE_DROP_MINT_ORDINAL)
                {
                    return Err(LootPlanError::UnsupportedAlgorithm);
                }
                state.push_instances(
                    entry,
                    0,
                    DIRECT_CORPSE_DROP_MINT_ORDINAL,
                    None,
                    &mut placement_ordinal,
                )?;
            }
            continue;
        }
        let probability = entry
            .probability_ppm
            .ok_or(LootPlanError::InvalidProbability)?;
        let chance = deterministic_draw(
            death,
            reference,
            draw_ordinal,
            if roll == 0 {
                DrawKind::Chance
            } else {
                DrawKind::BoostedChance(roll)
            },
        );
        let factor = loot_factor_percent(deterministic_draw(
            death,
            reference,
            draw_ordinal,
            if roll == 0 {
                DrawKind::Factor
            } else {
                DrawKind::BoostedFactor(roll)
            },
        ));
        if (is_surprise_bag(entry) && !is_global_event_loot_table(reference))
            || state.unique.contains(&entry.item.production_key)
            || !factored_success(
                chance,
                entry.details.rated_probability_ppm.unwrap_or(probability),
                factor,
                entry.details.gut_bonus_hundredths,
            )
        {
            state.ordinal += validate_tree(&entry.details.children, 0)? as u32;
            continue;
        }
        let index = state.entries.len();
        state.push_instances(entry, chance, draw_ordinal, parent, &mut placement_ordinal)?;
        if entry.details.unique {
            state.unique.insert(entry.item.production_key.clone());
        }
        roll_entries(
            death,
            reference,
            &entry.details.children,
            roll,
            Some(index),
            state,
        )?;
        if let Some(capacity) = entry.details.container_capacity {
            let children = state.entries[index + 1..]
                .iter()
                .filter(|child| child.parent == Some(index))
                .count();
            if children > capacity as usize {
                return Err(LootPlanError::TooManyPlanItems);
            }
        }
    }
    Ok(())
}

/// W13: order a boosted boss's table so the entries its bonus rerolls lead, and return their
/// count (the `boosted_entries` of [`LootSelectionAlgorithm::DailyBoosted`]). The bonus rolls
/// take equipment only and never a unique entry (Crystal 201189f8
/// `data/libs/functions/monster.lua:219-221`, `equipmentOnly` returns `not unique and
/// isEquipment(itemType)`; rolls from `data/scripts/systems/reward_chest.lua:86-103`). The sort
/// is stable, so replay ordinals stay deterministic.
pub fn partition_daily_boss_bonus_entries(
    entries: &mut [LootTableEntry],
    is_equipment: impl Fn(&str) -> bool,
) -> usize {
    let bonus =
        |entry: &LootTableEntry| !entry.details.unique && is_equipment(&entry.item.production_key);
    entries.sort_by_key(|entry| !bonus(entry));
    entries.iter().take_while(|entry| bonus(entry)).count()
}

// All rolls share the existing per-death draw, Item and plan-byte ceilings.
// Distinct draw kinds domain-separate the extra rolls without widening those bounds.
fn plan_daily_boosted(
    death_key: LootPlanDeathKey,
    table_ref: &LootDefinitionRef,
    table: &LootTableDefinition,
    extra_rolls: u8,
    boosted_entries: usize,
) -> Result<LootPlan, LootPlanError> {
    if !(1..=DAILY_BOOSTED_EXTRA_ROLLS_MAX).contains(&extra_rolls)
        || boosted_entries > table.entries.len()
    {
        return Err(LootPlanError::UnsupportedAlgorithm);
    }
    if table.entries.len() > COMBAT01_LOOT_PLAN_ENTRIES_MAX {
        return Err(LootPlanError::TooManyTableEntries);
    }
    // Include descendants in the preflight before any additional roll is allocated.
    let draws = 2
        * (validate_tree(&table.entries, 0)?
            + validate_tree(&table.entries[..boosted_entries], 0)? * usize::from(extra_rolls));
    if draws > COMBAT01_LOOT_RNG_DRAWS_MAX {
        return Err(LootPlanError::TooManyRngDraws);
    }
    let mut ordinary = table.clone();
    ordinary.algorithm = LootSelectionAlgorithm::IndependentBernoulliPpm;
    let mut plan = plan_creature_loot(death_key, table_ref, &ordinary)?;
    ordinary.entries.truncate(boosted_entries);
    // Preserve previously admitted small tables' durable boosted MINT ordinals.
    let stride = validate_tree(&table.entries, 0)?.max(16) as u32;
    for roll in 1..=extra_rolls {
        let extra = plan_roll(death_key, table_ref, &ordinary, roll)?;
        let mut remapped = vec![None; extra.entries.len()];
        // W13: each extra roll is its own donor `generateLootRoll` call, whose unique set starts
        // empty (Crystal 201189f8 `data/libs/functions/monstertype.lua:10`, `:25`, `:124`;
        // Canary 47dfd51f `:10`, `:25`, `:74`): uniqueness holds inside one roll, not across.
        for (index, mut entry) in extra.entries.into_iter().enumerate() {
            if let Some(parent) = entry.parent {
                let Some(parent) = remapped[parent] else {
                    continue;
                };
                entry.parent = Some(parent);
            }
            entry.placement_ordinal = 1 + plan
                .entries
                .iter()
                .filter(|old| old.parent == entry.parent)
                .count() as u32;
            remapped[index] = Some(plan.entries.len());
            if plan.entries.len() >= COMBAT01_LOOT_PLAN_ITEMS_MAX {
                return Err(LootPlanError::TooManyPlanItems);
            }
            // Each successful roll mints its own Item. Combining quantities would
            // manufacture stacks of non-stackable equipment. The ordinal is also
            // part of the durable MINT occurrence, so retries keep each roll distinct.
            entry.draw_ordinal += u32::from(roll) * stride;
            plan.entries.push(entry);
        }
    }
    let mut bytes = charge_plan_bytes(0, header_footprint_bytes(table_ref))?;
    for entry in &plan.entries {
        bytes = charge_plan_bytes(
            bytes,
            entry_footprint_bytes(&entry.item, &entry.purpose_key),
        )?;
    }
    if plan.entries.len() > COMBAT01_LOOT_PLAN_ITEMS_MAX {
        return Err(LootPlanError::TooManyPlanItems);
    }
    Ok(plan)
}

fn is_surprise_bag(entry: &LootTableEntry) -> bool {
    GLOBAL_EVENT_SURPRISE_BAGS.contains(&entry.item.production_key.as_str())
}

fn is_global_event_loot_table(loot_table_ref: &LootDefinitionRef) -> bool {
    GLOBAL_EVENT_SURPRISE_BAG_TABLES.contains(&loot_table_ref.production_key.as_str())
}

fn validate_entry_shape(entry: &LootTableEntry) -> Result<(), LootPlanError> {
    if entry.min_count == 0 || entry.max_count < entry.min_count {
        return Err(LootPlanError::InvalidQuantityRange);
    }
    if entry.details.direct_corpse_drop {
        return if entry.min_count == entry.max_count
            && entry.details.stackable
            && entry.details.charges.is_none()
            && entry.details.container_capacity.is_none()
            && entry.details.children.is_empty()
        {
            Ok(())
        } else {
            Err(LootPlanError::InvalidQuantityRange)
        };
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

fn draw_ppm(draw: u64) -> u64 {
    draw % u64::from(LOOT_PROBABILITY_PPM_SCALE)
}

/// The donor's per-entry loot factor, `95..=105` percent (Crystal and Canary
/// `MonsterType:generateLootRoll`, `dynamicFactor = factor * math.random(95, 105) / 100`).
fn loot_factor_percent(draw: u64) -> u32 {
    LOOT_FACTOR_PERCENT_MIN + (draw % LOOT_FACTOR_PERCENT_SPAN) as u32
}

/// `success = draw_ppm < probability_ppm * factor / 100`, compared exactly, `draw_ppm` uniform
/// over `0..1,000,000`: `probability_ppm == 0` never succeeds. The factor also applies at the full
/// scale, matching Crystal 201189f8 `data/libs/functions/monstertype.lua:89-100`; therefore its
/// 95% draw can make a nominal 100% entry miss. A Gut bonus raises the factored chance
/// by `ceil(chance * bonus)` (Crystal and Canary `generateLootRoll`, `adjustedChance +=
/// math.ceil(adjustedChance * charmChance / 100)`); the donor rounds on its 1/100,000 scale,
/// Oteryn on the exact ppm-percent product.
/// W22c: the extra full loot rolls of `activators` live Wealth Duplex concoctions (Crystal
/// ff7ede59 `data/scripts/eventcallbacks/monster/ondroploot_wealth_duplex.lua:29-49`). Each adds
/// the concoction's rate of 100 (`data/scripts/systems/concoctions.lua:22`), the sum is divided
/// by `activators^0.7` (`config.lua.dist:304`), and the fractional roll is taken when the
/// inclusive `0..=100` draw falls below it. The donor's shipped config shares no loot boost
/// across a party (`config.lua.dist:303`), so the settle passes the corpse owner alone.
#[must_use]
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub fn wealth_duplex_extra_rolls(activators: u32, draw_0_to_100: u8) -> u32 {
    if activators == 0 {
        return 0;
    }
    let count = f64::from(activators);
    let rolls = 100.0 * count / count.powf(0.7) / 100.0;
    let whole = rolls.floor();
    whole as u32 + u32::from(f64::from(draw_0_to_100) < (rolls - whole) * 100.0)
}

fn factored_success(
    draw: u64,
    probability_ppm: u32,
    factor_percent: u32,
    gut_bonus_hundredths: u32,
) -> bool {
    let chance = u64::from(probability_ppm) * u64::from(factor_percent);
    let gut = (chance * u64::from(gut_bonus_hundredths)).div_ceil(10_000);
    draw_ppm(draw) * 100 < chance + gut
}

/// Applies the configured loot-rate percentage before a table reaches the deterministic planner.
/// Crystal 201189f8 `data/libs/functions/functions.lua:68-71` divides the random threshold by
/// `RATE_LOOT`; the capped chance remains available to other consumers, while the uncapped value
/// feeds the planner's per-entry factor.
pub fn apply_configured_loot_rate(table: &mut LootTableDefinition, rate_percent: u32) {
    apply_entry_loot_rate(&mut table.entries, rate_percent);
}

fn apply_entry_loot_rate(entries: &mut [LootTableEntry], rate_percent: u32) {
    for entry in entries {
        if let Some(probability) = entry.probability_ppm {
            let rated = u64::from(probability).saturating_mul(u64::from(rate_percent)) / 100;
            entry.probability_ppm = Some(rated.min(u64::from(LOOT_PROBABILITY_PPM_SCALE)) as u32);
            entry.details.rated_probability_ppm = (rated > u64::from(LOOT_PROBABILITY_PPM_SCALE))
                .then(|| rated.min(2_000_000) as u32);
        }
        apply_entry_loot_rate(&mut entry.details.children, rate_percent);
    }
}

/// The donor takes the count from the same random value as the chance:
/// `randValue % (maxCount - minCount + 1) + minCount`, `randValue` on its 0..100,000 scale.
fn quantity_in_range(chance_draw: u64, min_count: u32, max_count: u32) -> u32 {
    let span = u64::from(max_count - min_count) + 1;
    let donor_value = draw_ppm(chance_draw) / u64::from(DONOR_LOOT_CHANCE_DIVISOR);
    min_count + (donor_value % span) as u32
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
    if death_key.instance {
        hasher.update(b"oteryn:instance-creature-loot:v1\0");
    }
    hasher.update(death_key.world_id);
    hasher.update(death_key.scope_id);
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
        DrawKind::Factor => 1_u8,
        DrawKind::BoostedChance(roll) => 2 * roll,
        DrawKind::BoostedFactor(roll) => 2 * roll + 1,
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

    #[test]
    fn instance_loot_entropy_is_separate_and_preserves_channel_draws() {
        let channel = LootPlanDeathKey::new([1; 16], [2; 16], 3, 4, 5);
        let instance = LootPlanDeathKey::new_instance([1; 16], [2; 16], 3, 4, 5);
        let table = table_ref("oteryn:loot.fixture");
        let before = deterministic_draw(channel, &table, 7, DrawKind::Chance);
        assert_eq!(before, 2203622670569259222);
        assert_eq!(
            deterministic_draw(instance, &table, 7, DrawKind::Chance),
            11225786138931470376
        );
        assert_ne!(channel, instance);
    }

    fn item_ref(key: &str) -> LootDefinitionRef {
        LootDefinitionRef::new("Item", key, "definition-r1")
    }

    pub(super) fn table_ref(key: &str) -> LootDefinitionRef {
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

    fn ordinary_stack_count(
        death: LootPlanDeathKey,
        reference: &LootDefinitionRef,
        ordinal: u32,
        source: &LootTableEntry,
        probability: u32,
    ) -> u32 {
        let chance = deterministic_draw(death, reference, ordinal, DrawKind::Chance);
        let factor = loot_factor_percent(deterministic_draw(
            death,
            reference,
            ordinal,
            DrawKind::Factor,
        ));
        if factored_success(chance, probability, factor, 0) {
            quantity_in_range(chance, source.min_count, source.max_count)
        } else {
            0
        }
    }

    #[test]
    fn fiendish_sliver_mint_ordinal_survives_wealth_duplex_expiry()
    -> Result<(), Box<dyn std::error::Error>> {
        use oteryn_gameplay_content::content::creature_reward::{
            CreatureRewardTable, LootTablesSection,
        };

        let profiles = serde_json::from_slice(include_bytes!(
            "../../../../content/creatures/definitions/spell-native-profiles.json"
        ))?;
        let section = LootTablesSection::decode(
            include_bytes!("../../../../tools/content-schema/native-gameplay/loot-tables.json"),
            &profiles,
        )?;
        let rewards = CreatureRewardTable::from_pin(&profiles, Some(&section));
        let rat = rewards
            .row("oteryn:creature.rat")
            .map_err(|error| error.as_str())?;
        let reference = rat.loot_table_ref.as_ref().ok_or("Rat loot table")?;
        let mut duplex = rat.loot_table.clone();
        duplex.entries.extend_from_slice(&rat.loot_table.entries);
        let death = (1..=4096)
            .map(|generation| LootPlanDeathKey::new([1; 16], [2; 16], 1, 1, generation))
            .find(|&death| {
                plan_creature_loot(death, reference, &duplex)
                    .is_ok_and(|plan| plan.entries.is_empty())
            })
            .ok_or("death where all ordinary draws miss")?;
        let sliver = rewards.forge_sliver().ok_or("Fiendish sliver")?;
        let count = u32::from(oteryn_gameplay_domain::domain::forge_creature::slivers(2));
        let plan = |mut table: LootTableDefinition| {
            table.entries.push(LootTableEntry {
                min_count: count,
                max_count: count,
                ..sliver.clone()
            });
            plan_creature_loot(death, reference, &table)
        };
        let first = plan(duplex)?;
        assert_eq!(first.entries.len(), 1);
        assert_eq!(first, plan(rat.loot_table.clone())?);
        Ok(())
    }

    #[test]
    fn committed_fiendish_sliver_survives_a_missed_loot_draw()
    -> Result<(), Box<dyn std::error::Error>> {
        use oteryn_gameplay_content::content::creature_reward::{
            CreatureRewardTable, LootTablesSection,
        };
        use oteryn_gameplay_domain::domain::forge_creature::slivers;

        let profiles = serde_json::from_slice(include_bytes!(
            "../../../../content/creatures/definitions/spell-native-profiles.json"
        ))?;
        let section = LootTablesSection::decode(
            include_bytes!("../../../../tools/content-schema/native-gameplay/loot-tables.json"),
            &profiles,
        )?;
        let rewards = CreatureRewardTable::from_pin(&profiles, Some(&section));
        let rat = rewards
            .row("oteryn:creature.rat")
            .map_err(|error| error.as_str())?;
        let reference = rat
            .loot_table_ref
            .as_ref()
            .ok_or("missing rat loot table")?;
        let mut table = rat.loot_table.clone();
        let ordinal = u32::try_from(table.entries.len())?;
        let deaths = || {
            (1..=4096).map(|generation| LootPlanDeathKey::new([1; 16], [2; 16], 1, 1, generation))
        };
        let factor =
            |death, kind| loot_factor_percent(deterministic_draw(death, reference, ordinal, kind));
        let succeeds = |death, kind, factor| {
            factored_success(
                deterministic_draw(death, reference, ordinal, kind),
                LOOT_PROBABILITY_PPM_SCALE,
                factor,
                0,
            )
        };
        let miss = deaths()
            .find(|&death| {
                let factor = factor(death, DrawKind::Factor);
                factor == 95 && !succeeds(death, DrawKind::Chance, factor)
            })
            .ok_or("missing low-factor miss witness")?;
        let double = deaths()
            .find(|&death| {
                succeeds(death, DrawKind::Chance, factor(death, DrawKind::Factor))
                    && succeeds(
                        death,
                        DrawKind::BoostedChance(1),
                        factor(death, DrawKind::BoostedFactor(1)),
                    )
            })
            .ok_or("missing Double Loot duplicate witness")?;
        let sliver = rewards.forge_sliver().ok_or("missing admitted sliver")?;
        let count = u32::from(slivers(2));
        table.entries.push(LootTableEntry {
            min_count: count,
            max_count: count,
            ..sliver.clone()
        });
        let sliver_quantities = |plan: LootPlan| {
            plan.entries
                .into_iter()
                .filter(|entry| entry.item == sliver.item)
                .map(|entry| entry.quantity)
                .collect::<Vec<_>>()
        };
        let missed = sliver_quantities(plan_creature_loot(miss, reference, &table)?);
        // Match the scheduled-rate path applying Double Loot to the complete ordinary table.
        table.algorithm = LootSelectionAlgorithm::DailyBoosted {
            extra_rolls: 1,
            boosted_entries: table.entries.len(),
        };
        let doubled = sliver_quantities(plan_creature_loot(double, reference, &table)?);
        assert_eq!((missed, doubled), (vec![count], vec![count]));
        Ok(())
    }

    #[test]
    fn native_loot_rolls_honor_the_active_item_stack_class()
    -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        std::thread::Builder::new()
            .stack_size(64 << 20)
            .spawn(check_native_loot_stack_classes)?
            .join()
            .map_err(|_| "native loot loader thread panicked")?
    }

    fn check_native_loot_stack_classes() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        use oteryn_gameplay_content::content::native_gameplay::{
            NativeItemStackClass, test_support::activated_with_loot_tables,
        };
        use oteryn_gameplay_content::content::native_gameplay_test_world_id;

        let (controller, _) = activated_with_loot_tables(native_gameplay_test_world_id()?, None);
        let native = controller
            .active()
            .and_then(|generation| generation.native_gameplay())
            .ok_or("missing active native gameplay")?;
        let rewards = native.reward_table();
        let mut actual = Vec::new();
        let mut expected = Vec::new();
        for (creature, items, stack_class) in [
            (
                "evil_prospector",
                &[8092][..],
                NativeItemStackClass::NonStackable,
            ),
            (
                "glooth_fairy",
                &[21143, 21144][..],
                NativeItemStackClass::StackCapable,
            ),
            (
                "vok_the_freakish",
                &[1781][..],
                NativeItemStackClass::StackCapable,
            ),
        ] {
            let creature_key = format!("oteryn:creature.{creature}");
            let private_reward = native
                .policies()
                .policies()
                .iter()
                .find(|policy| policy.definition_key == creature_key)
                .and_then(|policy| policy.reward_boss)
                .ok_or("missing reward-boss policy")?;
            assert_eq!(private_reward, creature != "evil_prospector", "{creature}");
            let row = rewards
                .row(&creature_key)
                .map_err(|reason| reason.as_str())?;
            let table_ref = row.loot_table_ref.as_ref().ok_or("missing loot table")?;
            for item in items {
                let key = format!("oteryn:item.tibia.i{item}");
                let source = row
                    .loot_table
                    .entries
                    .iter()
                    .find(|entry| entry.item.production_key == key)
                    .ok_or("missing loot entry")?;
                let policy = native
                    .item_policy(&key, &source.item.revision_ref)
                    .ok_or("missing native Item policy")?;
                assert_eq!(policy.record().admission.stack_class, stack_class, "{key}");
                assert!(
                    source.max_count > 1 && source.details.charges.is_none(),
                    "{key}"
                );
                let mut largest = 0;
                for seed in 0..2048_u32 {
                    let plan = if private_reward {
                        plan_boss_loot(
                            u128::from(seed).to_be_bytes(),
                            [2; 16],
                            table_ref,
                            &row.loot_table,
                            1.0,
                            true,
                        )?
                    } else {
                        plan_creature_loot(
                            LootPlanDeathKey::new([1; 16], [2; 16], 1, seed, 1),
                            table_ref,
                            &row.loot_table,
                        )?
                    };
                    let mut entries = plan
                        .entries
                        .iter()
                        .filter(|entry| entry.item == source.item);
                    if let Some(entry) = entries.next() {
                        assert!(entries.next().is_none(), "{creature} {key}, seed {seed}");
                        largest = largest.max(entry.quantity);
                    }
                }
                actual.push((creature, *item, largest));
                let maximum = if stack_class == NativeItemStackClass::NonStackable {
                    1
                } else {
                    source.max_count
                };
                expected.push((creature, *item, maximum));
            }
        }
        assert_eq!(
            actual, expected,
            "largest minted quantity by creature and Item"
        );
        Ok(())
    }

    #[test]
    fn clomp_loot_keeps_the_committed_roll_in_collectable_stacks()
    -> Result<(), Box<dyn std::error::Error>> {
        use oteryn_gameplay_content::content::creature_reward::{
            CreatureRewardTable, LootTablesSection,
        };

        let profiles = serde_json::from_str(include_str!(
            "../../../../content/creatures/definitions/spell-native-profiles.json"
        ))?;
        let section = LootTablesSection::decode(
            include_bytes!("../../../../tools/content-schema/native-gameplay/loot-tables.json"),
            &profiles,
        )?;
        let rewards = CreatureRewardTable::from_pin(&profiles, Some(&section));
        let limit = GAMEITEM01_STACK_QUANTITY_MAX;
        for creature in ["oteryn:creature.animated_clomp", "oteryn:creature.clomp"] {
            let reward = rewards.row(creature).map_err(|reason| reason.as_str())?;
            let reference = reward.loot_table_ref.as_ref().ok_or("missing loot table")?;
            let (ordinal, source) = reward
                .loot_table
                .entries
                .iter()
                .enumerate()
                .find(|(_, entry)| entry.item.production_key == "oteryn:item.tibia.i3031")
                .ok_or("missing gold entry")?;
            assert!(source.details.stackable);
            let ordinal = u32::try_from(ordinal)?;
            let probability = source.probability_ppm.ok_or("missing gold probability")?;
            let (death, total) = (0..=u8::MAX)
                .map(death_key)
                .find_map(|death| {
                    let total =
                        ordinary_stack_count(death, reference, ordinal, source, probability);
                    (total > limit).then_some((death, total))
                })
                .ok_or("no successful oversized gold roll")?;
            let plan = plan_creature_loot(death, reference, &reward.loot_table)?;
            let gold: Vec<_> = plan
                .entries
                .iter()
                .filter(|entry| entry.item == source.item)
                .collect();
            assert_eq!(
                gold.iter().map(|entry| entry.quantity).collect::<Vec<_>>(),
                [limit, total - limit],
                "{creature}"
            );
            assert_eq!(gold[0].draw_ordinal, ordinal);
            assert_ne!(gold[0].draw_ordinal, gold[1].draw_ordinal);
            assert_eq!(gold[0].placement_ordinal + 1, gold[1].placement_ordinal);
            assert_eq!(
                plan,
                plan_creature_loot(death, reference, &reward.loot_table)?
            );
        }
        Ok(())
    }

    #[test]
    fn behemoth_corpse_gold_fits_the_pickup_stack_limit() -> Result<(), Box<dyn std::error::Error>>
    {
        use crate::item_transfer::{
            BackpackEntry, InventoryItem, ItemTransferDestination, PlanInput, TransferPlan,
            plan_transfer,
        };
        use oteryn_gameplay_content::content::creature_reward::{
            CreatureRewardTable, LootTablesSection,
        };
        use oteryn_gameplay_domain::domain::currency::Coin;
        use oteryn_gameplay_domain::domain::item_definition::{
            ItemDefinitionFacts, ItemStackClass, TypedDefinitionRef,
        };

        let profiles = serde_json::from_slice(include_bytes!(
            "../../../../content/creatures/definitions/spell-native-profiles.json"
        ))?;
        let section = LootTablesSection::decode(
            include_bytes!("../../../../tools/content-schema/native-gameplay/loot-tables.json"),
            &profiles,
        )?;
        let rewards = CreatureRewardTable::from_pin(&profiles, Some(&section));
        let reward = rewards
            .row("oteryn:creature.behemoth")
            .map_err(|reason| reason.as_str())?;
        let reference = reward.loot_table_ref.as_ref().ok_or("missing loot table")?;
        let death = death_key(1);
        let plan = plan_creature_loot(death, reference, &reward.loot_table)?;
        let gold = Coin::Gold.production_key();
        let backpack = ItemDefinitionFacts {
            definition: TypedDefinitionRef {
                family: "Item".into(),
                production_key: "fixture:backpack".into(),
                revision_ref: "definition-r1".into(),
            },
            stack: ItemStackClass::NonStackable,
            container_capacity: Some(2),
            container_slot_equip_pattern: true,
            weight: None,
        };
        let slot = InventoryItem {
            item_instance_id: [1; 16],
            definition: backpack.definition.clone(),
            quantity: 1,
        };
        let mut entries = Vec::new();
        for entry in plan
            .entries
            .iter()
            .filter(|entry| entry.parent.is_none() && entry.item.production_key == gold)
        {
            let item = ItemDefinitionFacts {
                definition: TypedDefinitionRef {
                    family: entry.item.family.clone(),
                    production_key: entry.item.production_key.clone(),
                    revision_ref: entry.item.revision_ref.clone(),
                },
                stack: ItemStackClass::Stackable {
                    proven_maximum: None,
                },
                container_capacity: None,
                container_slot_equip_pattern: false,
                weight: None,
            };
            let source = InventoryItem {
                item_instance_id: [u8::try_from(entries.len())? + 2; 16],
                definition: item.definition.clone(),
                quantity: entry.quantity,
            };
            let TransferPlan::NewEntry { position } = plan_transfer(&PlanInput {
                destination: ItemTransferDestination::MainBackpack,
                item: &item,
                backpack: Some(&backpack),
                source: &source,
                source_has_entries: false,
                slot: Some(&slot),
                entries: &entries,
            })
            .map_err(|reason| format!("{} gold: {reason:?}", source.quantity))?
            else {
                return Err("expected a new backpack entry".into());
            };
            entries.push(BackpackEntry {
                item: source,
                placement_ordinal: position.placement_ordinal,
            });
        }
        let (ordinal, source) = reward
            .loot_table
            .entries
            .iter()
            .enumerate()
            .find(|(_, entry)| entry.item.production_key == gold)
            .ok_or("missing gold entry")?;
        assert!(source.details.stackable && source.details.charges.is_none());
        let probability = source.probability_ppm.ok_or("missing gold probability")?;
        let total = ordinary_stack_count(
            death,
            reference,
            u32::try_from(ordinal)?,
            source,
            probability,
        );
        let maximum = GAMEITEM01_STACK_QUANTITY_MAX;
        assert!((maximum + 1..=2 * maximum).contains(&total));
        assert_eq!(
            entries
                .iter()
                .map(|entry| entry.item.quantity)
                .collect::<Vec<_>>(),
            [maximum, total - maximum],
        );
        Ok(())
    }

    #[test]
    fn daily_nested_loot_gives_collectable_stacks_distinct_slots_and_causes()
    -> Result<(), LootPlanError> {
        let limit = GAMEITEM01_STACK_QUANTITY_MAX;
        let total = 2 * limit + 1;
        let mut table = one_entry_table("oteryn:item.fixture.container", 1, 1, Some(1_000_000));
        let container = &mut table.entries[0].details;
        container.container_capacity = Some(3);
        container.children =
            one_entry_table("oteryn:item.fixture.stack", total, total, Some(1_000_000)).entries;
        container.children[0].details.stackable = true;
        container.children[0].details.corpse_stack_maximum = std::num::NonZeroU32::new(limit);
        apply_configured_loot_rate(&mut table, 200);
        table.algorithm = LootSelectionAlgorithm::DailyBoosted {
            extra_rolls: 1,
            boosted_entries: 1,
        };

        let plan = plan_creature_loot(death_key(0), &table_ref("oteryn:loot.fixture"), &table)?;
        assert_eq!(plan.entries.len(), 8);
        let mut ordinals = std::collections::HashSet::new();
        for (parent, entries) in plan.entries.as_chunks::<4>().0.iter().enumerate() {
            assert_eq!(entries[0].parent, None);
            for (slot, (entry, quantity)) in entries[1..].iter().zip([limit, limit, 1]).enumerate()
            {
                assert_eq!(entry.parent, Some(parent * 4));
                assert_eq!(entry.placement_ordinal as usize, slot + 1);
                assert_eq!(entry.quantity, quantity);
            }
            assert!(
                entries
                    .iter()
                    .all(|entry| ordinals.insert(entry.draw_ordinal))
            );
        }
        Ok(())
    }

    pub(super) fn one_entry_table(
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
                details: Default::default(),
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
                    details: Default::default(),
                })
                .collect(),
        }
    }

    #[test]
    fn clomp_corpse_gold_preserves_rolls_in_collectable_stacks()
    -> Result<(), Box<dyn std::error::Error>> {
        use oteryn_gameplay_content::content::creature_reward::{
            CreatureRewardTable, LootTablesSection, NoSettlementReason,
        };
        use oteryn_gameplay_domain::domain::currency::COIN_STACK_MAXIMUM;

        let creatures = serde_json::from_slice(include_bytes!(
            "../../../../content/creatures/definitions/spell-native-profiles.json"
        ))?;
        let section = LootTablesSection::decode(
            include_bytes!("../../../../tools/content-schema/native-gameplay/loot-tables.json"),
            &creatures,
        )?;
        let rewards = CreatureRewardTable::from_pin(&creatures, Some(&section));
        let gold = oteryn_gameplay_domain::domain::currency::Coin::Gold.production_key();
        let mut oversized = Vec::new();
        for creature in ["oteryn:creature.animated_clomp", "oteryn:creature.clomp"] {
            let reward = rewards.row(creature).map_err(NoSettlementReason::as_str)?;
            let reference = reward.loot_table_ref.as_ref().ok_or("missing loot table")?;
            let (ordinal, source) = reward
                .loot_table
                .entries
                .iter()
                .enumerate()
                .find(|(_, entry)| entry.item.production_key == gold)
                .ok_or("missing gold roll")?;
            assert!(source.details.stackable && source.details.charges.is_none());
            let ordinal = u32::try_from(ordinal)?;
            let probability = source.probability_ppm.ok_or("missing gold probability")?;
            let source_maximum = source.max_count;
            let mut largest = 0;
            for seed in 0..=u8::MAX {
                let death = death_key(seed);
                let expected = ordinary_stack_count(death, reference, ordinal, source, probability);
                let plan = plan_creature_loot(death, reference, &reward.loot_table)?;
                let gold_stacks: Vec<_> = plan
                    .entries
                    .iter()
                    .filter(|entry| entry.item.production_key == gold)
                    .collect();
                let total: u32 = gold_stacks.iter().map(|entry| entry.quantity).sum();
                assert_eq!(total, expected, "{creature}, seed {seed}");
                assert_eq!(
                    plan,
                    plan_creature_loot(death, reference, &reward.loot_table)?
                );
                assert!(gold_stacks.windows(2).all(|pair| {
                    pair[0].draw_ordinal != pair[1].draw_ordinal
                        && pair[0].purpose_key == pair[1].purpose_key
                        && pair[0].placement_ordinal + 1 == pair[1].placement_ordinal
                }));
                largest = largest.max(total);
                if let Some(entry) = gold_stacks
                    .iter()
                    .find(|entry| entry.quantity > COIN_STACK_MAXIMUM)
                {
                    oversized.push((creature, entry.quantity));
                }
            }
            assert_eq!(largest, source_maximum, "{creature}");
            assert!(largest > COIN_STACK_MAXIMUM, "{creature}");
        }
        assert!(oversized.is_empty(), "uncollectable gold: {oversized:?}");
        Ok(())
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
                    details: Default::default(),
                },
                LootTableEntry {
                    item: item_ref("oteryn:item.tibia.i3031"),
                    min_count: 1,
                    max_count: 4,
                    probability_ppm: Some(903_036),
                    details: oteryn_gameplay_domain::domain::loot_data::LootEntryDetails {
                        stackable: true,
                        ..Default::default()
                    },
                },
            ],
        }
    }

    const GOLDEN_CHANCE: u64 = 10_348_850_681_732_701_394;
    const GOLDEN_QUANTITY: u64 = 9_517_304_501_960_542_980;
    // W22a: the donor count comes from the chance draw, so the pinned rat plan re-rolled 1 -> 3.
    const GOLDEN_RAT_PLAN: &str = r#"[("oteryn:item.tibia.i3031", 3, 1)]"#;

    #[test]
    fn deterministic_draw_is_pinned_by_golden_vectors() {
        // FND-03 §25: a change to framing, byte order, truncation or the hash must not
        // silently re-roll historical plans. These values are the canonical derivation.
        let table_ref = table_ref("oteryn:loot.creature.rat");
        let key = death_key(7);
        let chance = deterministic_draw(key, &table_ref, 0, DrawKind::Chance);
        let quantity = deterministic_draw(key, &table_ref, 1, DrawKind::Factor);
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
    fn maximum_entries_use_exactly_the_registered_rng_draw_budget() {
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
    fn maximum_width_entries_fit_exactly_at_the_registered_byte_ceiling() {
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
                details: oteryn_gameplay_domain::domain::loot_data::LootEntryDetails {
                    gut_bonus_hundredths: 1_000,
                    ..Default::default()
                },
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
    fn zero_chance_never_drops_and_nominal_max_chance_still_uses_donor_factor() {
        let table_ref = table_ref("oteryn:loot.chance.table");
        let table = LootTableDefinition {
            algorithm: LootSelectionAlgorithm::IndependentBernoulliPpm,
            entries: vec![
                LootTableEntry {
                    item: item_ref("oteryn:item.chance.never"),
                    min_count: 1,
                    max_count: 1,
                    probability_ppm: Some(0),
                    details: Default::default(),
                },
                LootTableEntry {
                    item: item_ref("oteryn:item.chance.always"),
                    min_count: 1,
                    max_count: 1,
                    probability_ppm: Some(1_000_000),
                    details: Default::default(),
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
        }
        // Crystal 201189f8 `data/libs/functions/monstertype.lua:89-100` applies the 95..105%
        // factor before comparing every entry, including MAX_LOOTCHANCE.
        assert!(!factored_success(999_999, 1_000_000, 95, 0));
        assert!(factored_success(999_999, 1_000_000, 105, 0));
    }

    #[test]
    fn configured_loot_rate_reaches_every_descendant() {
        let mut table = one_entry_table("oteryn:item.root", 1, 1, None);
        let mut child = one_entry_table("oteryn:item.child", 1, 1, None)
            .entries
            .remove(0);
        child.probability_ppm = Some(400_000);
        let mut grandchild = one_entry_table("oteryn:item.grandchild", 1, 1, None)
            .entries
            .remove(0);
        grandchild.probability_ppm = Some(600_000);
        child.details.children.push(grandchild);
        table.entries[0].details.children.push(child);
        apply_configured_loot_rate(&mut table, 200);
        let child = &table.entries[0].details.children[0];
        assert_eq!(table.entries[0].probability_ppm, None);
        assert_eq!(child.probability_ppm, Some(800_000));
        assert_eq!(child.details.rated_probability_ppm, None);
        assert_eq!(child.details.children[0].probability_ppm, Some(1_000_000));
        assert_eq!(
            child.details.children[0].details.rated_probability_ppm,
            Some(1_200_000)
        );
    }

    #[test]
    fn configured_loot_rate_scales_and_caps_entry_chances() {
        // Crystal 201189f8 `data/libs/functions/functions.lua:68-70` multiplies the effective
        // chance by config `RATE_LOOT`; config.lua.dist ships the neutral multiplier.
        let mut table = one_entry_table("oteryn:item.rate", 1, 1, Some(600_000));
        apply_configured_loot_rate(&mut table, 200);
        assert_eq!(table.entries[0].probability_ppm, Some(1_000_000));
    }

    #[test]
    fn configured_loot_rate_keeps_an_in_scale_chance_without_a_rated_overflow() {
        let mut below_scale = one_entry_table("oteryn:item.rate.below", 1, 1, Some(400_000));
        apply_configured_loot_rate(&mut below_scale, 200);
        assert_eq!(below_scale.entries[0].probability_ppm, Some(800_000));
        assert_eq!(below_scale.entries[0].details.rated_probability_ppm, None);

        let mut at_scale = one_entry_table("oteryn:item.rate.at", 1, 1, Some(500_000));
        apply_configured_loot_rate(&mut at_scale, 200);
        assert_eq!(at_scale.entries[0].probability_ppm, Some(1_000_000));
        assert_eq!(at_scale.entries[0].details.rated_probability_ppm, None);

        let mut neutral = one_entry_table("oteryn:item.rate.neutral", 1, 1, Some(600_000));
        apply_configured_loot_rate(&mut neutral, 100);
        assert_eq!(neutral.entries[0].probability_ppm, Some(600_000));
        assert_eq!(neutral.entries[0].details.rated_probability_ppm, None);
    }

    #[test]
    fn configured_loot_rate_above_certainty_survives_the_lowest_loot_factor() {
        let reference = table_ref("oteryn:loot.rate.overflow");
        let capped = one_entry_table("oteryn:item.rate.overflow", 1, 1, Some(1_000_000));
        // A nominal 100% entry misses on some 95..99% factor draw.
        let miss = (0..4096_u32)
            .map(|actor| LootPlanDeathKey::new([7; 16], [8; 16], 1, actor, 1))
            .find(|key| {
                plan_creature_loot(*key, &reference, &capped)
                    .expect("capped plan")
                    .entries
                    .is_empty()
            })
            .expect("a low factor draw misses a nominal 100% entry");
        let mut rated = one_entry_table("oteryn:item.rate.overflow", 1, 1, Some(600_000));
        apply_configured_loot_rate(&mut rated, 200);
        assert_eq!(rated.entries[0].probability_ppm, Some(1_000_000));
        assert_eq!(
            rated.entries[0].details.rated_probability_ppm,
            Some(1_200_000)
        );
        // Crystal: 60% x 200% x 95% = 114% is certain.
        assert_eq!(
            plan_creature_loot(miss, &reference, &rated)
                .expect("rated plan")
                .entries
                .len(),
            1
        );
    }

    #[test]
    fn configured_loot_rate_overflow_is_bounded_at_twice_the_scale() {
        let reference = table_ref("oteryn:loot.rate.bound");
        let mut table = one_entry_table("oteryn:item.rate.bound", 1, 1, Some(1_000_000));
        apply_configured_loot_rate(&mut table, u32::MAX);
        assert_eq!(table.entries[0].probability_ppm, Some(1_000_000));
        assert_eq!(
            table.entries[0].details.rated_probability_ppm,
            Some(2_000_000)
        );

        for actor in 0..64_u32 {
            let key = LootPlanDeathKey::new([3; 16], [4; 16], 1, actor, 1);
            assert_eq!(
                plan_creature_loot(key, &reference, &table)
                    .expect("bounded plan")
                    .entries
                    .len(),
                1
            );
        }
    }

    #[test]
    fn surprise_bags_are_not_random_all_year_loot() {
        let ordinary = table_ref("oteryn:loot.creature.rat");
        let table = one_entry_table(
            "oteryn:item.tibia.i6570",
            1,
            1,
            Some(LOOT_PROBABILITY_PPM_SCALE),
        );

        let plan = plan_creature_loot(death_key(7), &ordinary, &table).expect("valid plan");

        assert!(plan.entries.is_empty());
    }

    #[test]
    fn surprise_bags_remain_in_global_event_creature_loot() {
        let event = table_ref("oteryn:loot.creature.pinata_dragon");
        let table = one_entry_table(
            "oteryn:item.tibia.i6571",
            1,
            1,
            Some(LOOT_PROBABILITY_PPM_SCALE),
        );

        let plan = plan_creature_loot(death_key(7), &event, &table).expect("valid plan");

        assert_eq!(plan.entries.len(), 1);
        assert_eq!(plan.entries[0].purpose_key, "oteryn:item.tibia.i6571");
    }

    #[test]
    fn quantity_draw_stays_within_the_entry_range() {
        let table_ref = table_ref("oteryn:loot.quantity.table");
        let mut table = one_entry_table("oteryn:item.quantity.stack", 3, 9, Some(1_000_000));
        table.entries[0].details.stackable = true;
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

#[cfg(test)]
mod daily_boost_tests {
    use super::*;

    #[test]
    fn daily_boost_extra_rolls_preserve_bounds_and_do_not_multiply_forge_drops()
    -> Result<(), LootPlanError> {
        let item = |key| LootDefinitionRef::new("Item", key, "definition-r1");
        let reference = LootDefinitionRef::new("Loot", "oteryn:loot.rat", "definition-r1");
        let mut table = LootTableDefinition {
            algorithm: LootSelectionAlgorithm::DailyBoosted {
                extra_rolls: 1,
                boosted_entries: 1,
            },
            entries: vec![
                LootTableEntry {
                    item: item("oteryn:item.gold"),
                    min_count: 1,
                    max_count: 1,
                    probability_ppm: Some(1_000_000),
                    details: Default::default(),
                },
                LootTableEntry {
                    item: item("oteryn:item.sliver"),
                    min_count: 3,
                    max_count: 3,
                    probability_ppm: Some(1_000_000),
                    details: oteryn_gameplay_domain::domain::loot_data::LootEntryDetails {
                        stackable: true,
                        ..Default::default()
                    },
                },
            ],
        };
        let death = LootPlanDeathKey::new([1; 16], [2; 16], 1, 1, 1);
        let plan = plan_creature_loot(death, &reference, &table)?;
        assert_eq!(
            plan.entries.iter().map(|e| e.quantity).collect::<Vec<_>>(),
            vec![1, 3, 1]
        );
        assert_eq!(plan_creature_loot(death, &reference, &table)?, plan);
        table.algorithm = LootSelectionAlgorithm::DailyBoosted {
            extra_rolls: 3,
            boosted_entries: 1,
        };
        assert_eq!(
            plan_creature_loot(death, &reference, &table)?
                .entries
                .iter()
                .filter(|e| e.item.production_key == "oteryn:item.gold")
                .map(|e| e.quantity)
                .sum::<u32>(),
            4
        );
        table.algorithm = LootSelectionAlgorithm::DailyBoosted {
            extra_rolls: 4,
            boosted_entries: 1,
        };
        assert_eq!(
            plan_creature_loot(death, &reference, &table),
            Err(LootPlanError::UnsupportedAlgorithm)
        );
        table.algorithm = LootSelectionAlgorithm::DailyBoosted {
            extra_rolls: 1,
            boosted_entries: 1,
        };
        table.entries = vec![table.entries[0].clone(); COMBAT01_LOOT_PLAN_ENTRIES_MAX];
        assert_eq!(
            plan_creature_loot(death, &reference, &table),
            Err(LootPlanError::TooManyRngDraws)
        );
        table.entries = vec![table.entries[0].clone(); COMBAT01_LOOT_PLAN_ENTRIES_MAX + 1];
        assert_eq!(
            plan_creature_loot(death, &reference, &table),
            Err(LootPlanError::TooManyTableEntries)
        );
        Ok(())
    }

    #[test]
    fn boosted_boss_bonus_rerolls_equipment_but_never_a_unique_entry() -> Result<(), LootPlanError>
    {
        let entry = |key: &str, unique: bool| {
            let details = oteryn_gameplay_domain::domain::loot_data::LootEntryDetails {
                unique,
                ..Default::default()
            };
            LootTableEntry {
                item: LootDefinitionRef::new("Item", key, "definition-r1"),
                min_count: 1,
                max_count: 1,
                probability_ppm: Some(1_000_000),
                details,
            }
        };
        let mut entries = vec![
            entry("oteryn:item.gold", false),
            entry("oteryn:item.unique_amulet", true),
            entry("oteryn:item.ring", false),
        ];
        let equipment = |key: &str| key != "oteryn:item.gold";
        let boosted = partition_daily_boss_bonus_entries(&mut entries, equipment);
        assert_eq!(boosted, 1);
        assert_eq!(
            entries
                .iter()
                .map(|e| e.item.production_key.as_str())
                .collect::<Vec<_>>(),
            [
                "oteryn:item.ring",
                "oteryn:item.gold",
                "oteryn:item.unique_amulet"
            ]
        );
        let table = LootTableDefinition {
            algorithm: LootSelectionAlgorithm::DailyBoosted {
                extra_rolls: 3,
                boosted_entries: boosted,
            },
            entries,
        };
        let plan = plan_creature_loot(
            LootPlanDeathKey::new([1; 16], [2; 16], 1, 1, 1),
            &LootDefinitionRef::new("Loot", "oteryn:loot.boss", "definition-r1"),
            &table,
        )?;
        let count = |key: &str| {
            plan.entries
                .iter()
                .filter(|e| e.item.production_key == key)
                .count()
        };
        assert_eq!(count("oteryn:item.ring"), 4);
        assert_eq!(count("oteryn:item.gold"), 1);
        assert_eq!(count("oteryn:item.unique_amulet"), 1);
        Ok(())
    }

    #[test]
    fn a_boosted_creature_extra_roll_keeps_uniqueness_inside_its_own_roll()
    -> Result<(), LootPlanError> {
        // Crystal and Canary `generateLootRoll` start each call with an empty unique set, so the
        // boosted creature's extra call may drop a unique item its base call already dropped.
        let unique = LootTableEntry {
            item: LootDefinitionRef::new("Item", "oteryn:item.unique", "definition-r1"),
            min_count: 1,
            max_count: 1,
            probability_ppm: Some(1_000_000),
            details: oteryn_gameplay_domain::domain::loot_data::LootEntryDetails {
                unique: true,
                ..Default::default()
            },
        };
        let table = LootTableDefinition {
            algorithm: LootSelectionAlgorithm::DailyBoosted {
                extra_rolls: 1,
                boosted_entries: 2,
            },
            entries: vec![unique.clone(), unique],
        };
        let plan = plan_creature_loot(
            LootPlanDeathKey::new([1; 16], [2; 16], 1, 1, 1),
            &LootDefinitionRef::new("Loot", "oteryn:loot.rat", "definition-r1"),
            &table,
        )?;
        assert_eq!(plan.entries.len(), 2);
        Ok(())
    }

    #[test]
    fn daily_boost_uses_independent_rolls_instead_of_doubling_probability()
    -> Result<(), LootPlanError> {
        let reference = LootDefinitionRef::new("Loot", "oteryn:loot.rat", "definition-r1");
        let table = LootTableDefinition {
            algorithm: LootSelectionAlgorithm::DailyBoosted {
                extra_rolls: 1,
                boosted_entries: 1,
            },
            entries: vec![LootTableEntry {
                item: LootDefinitionRef::new("Item", "oteryn:item.gold", "definition-r1"),
                min_count: 1,
                max_count: 1,
                probability_ppm: Some(500_000),
                details: Default::default(),
            }],
        };
        let mut seen = [false; 3];
        for actor in 1..=100 {
            let plan = plan_creature_loot(
                LootPlanDeathKey::new([1; 16], [2; 16], 1, actor, 1),
                &reference,
                &table,
            )?;
            let quantity: u32 = plan.entries.iter().map(|e| e.quantity).sum();
            seen[quantity as usize] = true;
        }
        assert_eq!(seen, [true; 3]);
        Ok(())
    }
}

#[cfg(test)]
mod w22a_tests {
    use super::*;

    fn entry(key: &str) -> LootTableEntry {
        LootTableEntry {
            item: LootDefinitionRef::new("Item", key, "definition-r1"),
            min_count: 1,
            max_count: 1,
            probability_ppm: Some(1_000_000),
            details: Default::default(),
        }
    }

    #[test]
    fn every_canonical_loot_table_can_be_planned() -> Result<(), Box<dyn std::error::Error>> {
        use oteryn_gameplay_content::content::creature_reward::SectionLootTable;
        let directory =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../content/loot");
        let mut tables = 0;
        let mut large = 0;
        for path in std::fs::read_dir(directory)? {
            let path = path?.path();
            if !path
                .file_name()
                .and_then(|s| s.to_str())
                .is_some_and(|s| s.starts_with("loot-") && s.ends_with(".json"))
            {
                continue;
            }
            let document: serde_json::Value = serde_json::from_slice(&std::fs::read(path)?)?;
            for record in document["records"].as_array().ok_or("canonical records")? {
                let mut definition = record["definition"].clone();
                definition
                    .as_object_mut()
                    .ok_or("canonical definition")?
                    .remove("kind");
                let table: SectionLootTable = serde_json::from_value(definition)?;
                let lower = |r: &oteryn_gameplay_content::content::creature_reward::SectionRef| {
                    LootDefinitionRef::new(&r.family, &r.key, &r.revision)
                };
                let entries = table
                    .entries
                    .iter()
                    .map(|e| {
                        assert!(
                            e.children.is_empty(),
                            "canonical descendants require Item facts in this fixture"
                        );
                        LootTableEntry {
                            item: lower(&e.item),
                            min_count: e.min_count,
                            max_count: e.max_count,
                            probability_ppm: e.probability_ppm,
                            details: Default::default(),
                        }
                    })
                    .collect();
                assert_eq!(table.algorithm, "IndependentBernoulliPpm");
                let source = LootTableDefinition {
                    algorithm: LootSelectionAlgorithm::IndependentBernoulliPpm,
                    entries,
                };
                let reference = lower(&table.identity);
                let death = LootPlanDeathKey::new([1; 16], [2; 16], 1, 1, 1);
                let plan = plan_creature_loot(death, &reference, &source)?;
                assert_eq!(plan, plan_creature_loot(death, &reference, &source)?);
                tables += 1;
                large += usize::from(source.entries.len() > 16);
            }
        }
        assert!(tables > 0 && large > 0);
        Ok(())
    }

    #[test]
    fn nested_container_rolls_children_inside_their_parent_deterministically()
    -> Result<(), LootPlanError> {
        let mut bag = entry("oteryn:item.bag");
        bag.min_count = 2;
        bag.max_count = 2;
        bag.details.container_capacity = Some(8);
        bag.details.children = vec![entry("oteryn:item.gold"), entry("oteryn:item.apple")];
        let table = LootTableDefinition {
            algorithm: LootSelectionAlgorithm::NestedGroups,
            entries: vec![bag],
        };
        let death = LootPlanDeathKey::new([1; 16], [2; 16], 1, 1, 1);
        let reference = LootDefinitionRef::new("Loot", "oteryn:loot.nested", "definition-r1");
        let plan = plan_creature_loot(death, &reference, &table)?;
        assert_eq!(
            plan.entries[0].quantity, 1,
            "a non-stackable container is one Item"
        );
        assert_eq!(
            plan.entries.iter().map(|e| e.parent).collect::<Vec<_>>(),
            vec![None, Some(0), Some(0)]
        );
        assert_eq!(
            plan.entries
                .iter()
                .map(|e| e.placement_ordinal)
                .collect::<Vec<_>>(),
            vec![1, 1, 2]
        );
        assert_eq!(plan, plan_creature_loot(death, &reference, &table)?);
        let mut failed = table;
        failed.entries[0].probability_ppm = Some(0);
        assert!(
            plan_creature_loot(death, &reference, &failed)?
                .entries
                .is_empty()
        );
        Ok(())
    }

    #[test]
    fn charged_item_uses_full_charges_and_unique_entry_rolls_once() -> Result<(), LootPlanError> {
        let mut rune = entry("oteryn:item.rune");
        rune.details.charges = Some(10);
        rune.details.subtype = Some(10);
        rune.details.unique = true;
        let table = LootTableDefinition {
            algorithm: LootSelectionAlgorithm::IndependentBernoulliPpm,
            entries: vec![rune.clone(), rune],
        };
        let plan = plan_creature_loot(
            LootPlanDeathKey::new([1; 16], [2; 16], 1, 1, 1),
            &LootDefinitionRef::new("Loot", "oteryn:loot.charges", "definition-r1"),
            &table,
        )?;
        assert_eq!(plan.entries.len(), 1);
        assert_eq!(plan.entries[0].quantity, 1);
        assert_eq!(plan.entries[0].charges, Some(10));
        assert_eq!(plan.entries[0].subtype, Some(10));
        Ok(())
    }

    #[test]
    fn forty_entry_table_plans_every_drop_when_bonus_covers_the_factor() -> Result<(), LootPlanError>
    {
        let table = LootTableDefinition {
            algorithm: LootSelectionAlgorithm::IndependentBernoulliPpm,
            entries: (0..40)
                .map(|index| LootTableEntry {
                    item: LootDefinitionRef::new(
                        "Item",
                        format!("oteryn:item.drop{index}"),
                        "definition-r1",
                    ),
                    min_count: 1,
                    max_count: 1,
                    probability_ppm: Some(1_000_000),
                    details: oteryn_gameplay_domain::domain::loot_data::LootEntryDetails {
                        gut_bonus_hundredths: 1_000,
                        ..Default::default()
                    },
                })
                .collect(),
        };
        let death = LootPlanDeathKey::new([1; 16], [2; 16], 1, 1, 1);
        let reference = LootDefinitionRef::new("Loot", "oteryn:loot.forty", "definition-r1");
        let plan = plan_creature_loot(death, &reference, &table)?;
        assert_eq!(plan.entries.len(), 40);
        assert_eq!(plan, plan_creature_loot(death, &reference, &table)?);
        Ok(())
    }

    #[test]
    fn loot_factor_covers_the_donor_inclusive_95_to_105_percent() {
        assert_eq!(loot_factor_percent(0), 95);
        assert_eq!(loot_factor_percent(10), 105);
        assert_eq!(loot_factor_percent(11), 95);
        let seen: std::collections::BTreeSet<u32> = (0..64).map(loot_factor_percent).collect();
        assert_eq!(seen, (95..=105).collect());
    }

    #[test]
    fn loot_factor_scales_the_chance_exactly_at_its_boundaries() {
        // 50%: a 95% factor admits draws below 475,000 ppm, a 105% factor below 525,000.
        assert!(factored_success(474_999, 500_000, 95, 0));
        assert!(!factored_success(475_000, 500_000, 95, 0));
        assert!(factored_success(524_999, 500_000, 105, 0));
        assert!(!factored_success(525_000, 500_000, 105, 0));
        assert!(factored_success(499_999, 500_000, 100, 0));
        assert!(!factored_success(500_000, 500_000, 100, 0));
        // A fractional threshold (356,433 * 0.95 = 338,611.35) is compared without rounding.
        assert!(factored_success(338_611, 356_433, 95, 0));
        assert!(!factored_success(338_612, 356_433, 95, 0));
        assert!(!factored_success(0, 0, 105, 0));
        assert!(!factored_success(999_999, 1_000_000, 95, 0));
        // The draw is reduced to ppm first.
        assert!(!factored_success(1_000_000 + 475_000, 500_000, 95, 0));
        assert!(factored_success(1_000_000 + 474_999, 500_000, 95, 0));
    }

    #[test]
    fn count_comes_from_the_chance_draw_at_the_donor_resolution() {
        // draw 123,456 ppm is donor value 12,345; 12,345 % 4 = 1.
        assert_eq!(quantity_in_range(123_456, 1, 4), 2);
        assert_eq!(quantity_in_range(123_459, 1, 4), 2);
        assert_eq!(quantity_in_range(123_460, 1, 4), 3);
        assert_eq!(quantity_in_range(999_999, 3, 9), 3 + 99_999 % 7);
        assert_eq!(quantity_in_range(5, 7, 7), 7);
    }

    #[test]
    fn planned_rolls_apply_the_loot_factor_and_take_the_count_from_the_chance_draw()
    -> Result<(), LootPlanError> {
        let reference = super::tests::table_ref("oteryn:loot.factor.table");
        let mut table =
            super::tests::one_entry_table("oteryn:item.factor.coin", 1, 4, Some(500_000));
        table.entries[0].details.stackable = true;
        let (mut above, mut below) = (false, false);
        for actor in 0..4_096_u32 {
            let death = LootPlanDeathKey::new([3; 16], [4; 16], 1, actor, 1);
            let chance = deterministic_draw(death, &reference, 0, DrawKind::Chance);
            let ppm = draw_ppm(chance);
            let plan = plan_creature_loot(death, &reference, &table)?;
            let factor =
                loot_factor_percent(deterministic_draw(death, &reference, 0, DrawKind::Factor));
            assert_eq!(
                plan.entries.len() == 1,
                ppm * 100 < 500_000 * u64::from(factor),
                "actor {actor}"
            );
            if let Some(entry) = plan.entries.first() {
                assert_eq!(entry.quantity, 1 + ((ppm / 10) % 4) as u32);
                above |= ppm >= 500_000;
            } else {
                below |= ppm < 500_000;
            }
        }
        // Some draw above the nominal chance drops (factor > 100%), some below misses (< 100%).
        assert!(above && below);
        Ok(())
    }

    #[test]
    fn an_uncharged_unstacked_subtype_is_the_minted_fluid() -> Result<(), LootPlanError> {
        let reference = super::tests::table_ref("oteryn:loot.subtype.table");
        let mut table = super::tests::one_entry_table("oteryn:item.subtype", 1, 1, Some(1_000_000));
        let plan = |table: &LootTableDefinition| {
            plan_creature_loot(
                LootPlanDeathKey::new([1; 16], [2; 16], 1, 1, 1),
                &reference,
                table,
            )
        };
        table.entries[0].details.subtype = Some(5);
        let fluid = plan(&table)?;
        assert_eq!(fluid.entries[0].fluid_subtype(), Some(5));
        assert!(!fluid.entries[0].needs_subtype_state());
        // Subtype 0 is the empty container: no fluid row.
        table.entries[0].details.subtype = Some(0);
        assert_eq!(plan(&table)?.entries[0].fluid_subtype(), None);
        // A subtype beyond the one-byte fluid range cannot be held and stays refused.
        table.entries[0].details.subtype = Some(300);
        let wide = plan(&table)?;
        assert_eq!(wide.entries[0].fluid_subtype(), None);
        assert!(wide.entries[0].needs_subtype_state());
        // A stack takes its count and ignores the subtype (container.lua:15-27).
        table.entries[0].details.stackable = true;
        let stack = plan(&table)?;
        assert_eq!(stack.entries[0].fluid_subtype(), None);
        assert!(!stack.entries[0].needs_subtype_state());
        table.entries[0].details.stackable = false;
        table.entries[0].details.subtype = Some(5);
        table.entries[0].details.charges = Some(5);
        let charged = plan(&table)?;
        assert_eq!(charged.entries[0].fluid_subtype(), None);
        assert!(!charged.entries[0].needs_subtype_state());
        table.entries[0].details.subtype = None;
        table.entries[0].details.charges = None;
        let plain = plan(&table)?;
        assert_eq!(plain.entries[0].fluid_subtype(), None);
        assert!(!plain.entries[0].needs_subtype_state());
        Ok(())
    }
}

#[cfg(test)]
mod w22b_tests {
    use super::*;

    #[test]
    fn gut_raises_the_factored_chance_by_its_ceiled_share() {
        // 50% at factor 100 with Gut 6%: 50,000,000 + 3,000,000 ppm-percent, draws below 530,000.
        assert!(factored_success(529_999, 500_000, 100, 600));
        assert!(!factored_success(530_000, 500_000, 100, 600));
        // 12% at factor 105: 52,500,000 + 6,300,000.
        assert!(factored_success(587_999, 500_000, 105, 1_200));
        assert!(!factored_success(588_000, 500_000, 105, 1_200));
        // The share is ceiled: 1 ppm at factor 95 with 6% is 95 + ceil(5.7) = 101.
        assert!(factored_success(1, 1, 95, 600));
        assert!(!factored_success(1, 1, 95, 0));
        assert!(!factored_success(0, 0, 105, 1_200));
    }

    #[test]
    fn a_gut_bonus_on_the_entry_widens_only_that_entry_roll() -> Result<(), LootPlanError> {
        let reference = super::tests::table_ref("oteryn:loot.gut.table");
        let plain = super::tests::one_entry_table("oteryn:item.gut.product", 1, 1, Some(500_000));
        let mut gut = plain.clone();
        gut.entries[0].details.creature_product = true;
        gut.entries[0].details.gut_bonus_hundredths = 1_200;
        let mut widened = false;
        for actor in 0..4_096_u32 {
            let death = LootPlanDeathKey::new([5; 16], [6; 16], 1, actor, 1);
            let ppm = draw_ppm(deterministic_draw(death, &reference, 0, DrawKind::Chance));
            let factor =
                loot_factor_percent(deterministic_draw(death, &reference, 0, DrawKind::Factor));
            let base = plan_creature_loot(death, &reference, &plain)?.entries.len();
            let boosted = plan_creature_loot(death, &reference, &gut)?.entries.len();
            let chance = 500_000 * u64::from(factor);
            assert_eq!(
                boosted == 1,
                ppm * 100 < chance + (chance * 1_200).div_ceil(10_000),
                "actor {actor}"
            );
            assert!(boosted >= base, "actor {actor}");
            widened |= boosted > base;
        }
        assert!(widened);
        Ok(())
    }
}

#[cfg(test)]
mod w22c_tests {
    use super::*;

    #[test]
    fn wealth_duplex_rolls_follow_the_donor_party_diminishing_rule() {
        // No live concoction rolls nothing; the corpse owner alone always rolls once more.
        assert_eq!(wealth_duplex_extra_rolls(0, 0), 0);
        assert_eq!(wealth_duplex_extra_rolls(1, 0), 1);
        assert_eq!(wealth_duplex_extra_rolls(1, 100), 1);
        // Two activators: 200 / 2^0.7 = 123.11, a second roll below a draw of 23.11.
        assert_eq!(wealth_duplex_extra_rolls(2, 23), 2);
        assert_eq!(wealth_duplex_extra_rolls(2, 24), 1);
        // Three activators: 300 / 3^0.7 = 139.04.
        assert_eq!(wealth_duplex_extra_rolls(3, 39), 2);
        assert_eq!(wealth_duplex_extra_rolls(3, 40), 1);
    }
}

#[cfg(test)]
mod admitted_loot_tests {
    use super::*;
    use crate::item_ref_admission::generation_item_facts;
    use crate::item_transfer::{
        InventoryItem, ItemTransferDestination, PlanInput, plan_transfer, stack_maximum,
    };
    use oteryn_gameplay_content::content::native_gameplay::test_support::activated_with_loot_tables;
    use oteryn_gameplay_content::content::native_gameplay_test_world_id;
    use oteryn_gameplay_domain::domain::item_definition::{
        ItemDefinitionFacts, ItemStackClass, TypedDefinitionRef,
    };

    fn yields(entries: &[LootTableEntry]) -> impl Iterator<Item = &LootTableEntry> {
        let mut pending = entries.iter().rev().collect::<Vec<_>>();
        std::iter::from_fn(move || {
            let entry = pending.pop()?;
            pending.extend(entry.details.children.iter().rev());
            Some(entry)
        })
    }

    #[test]
    fn admitted_loot_preserves_maximum_rolls_in_transferable_instances()
    -> Result<(), Box<dyn std::error::Error>> {
        let (controller, _) = activated_with_loot_tables(native_gameplay_test_world_id()?, None);
        let native = controller
            .active()
            .and_then(|generation| generation.native_gameplay())
            .ok_or("missing active native gameplay")?;
        let rewards = native.reward_table();
        let slivers = LootTableEntry {
            max_count: (0..=u8::MAX)
                .map(oteryn_gameplay_domain::domain::forge_creature::slivers)
                .map(u32::from)
                .max()
                .ok_or("missing Fiendish sliver count")?,
            ..rewards
                .forge_sliver()
                .ok_or("missing admitted sliver")?
                .clone()
        };
        let backpack = ItemDefinitionFacts {
            definition: TypedDefinitionRef {
                family: "Item".into(),
                production_key: "fixture:backpack".into(),
                revision_ref: "definition-r1".into(),
            },
            stack: ItemStackClass::NonStackable,
            container_capacity: Some(1),
            container_slot_equip_pattern: true,
            weight: None,
        };
        let slot = InventoryItem {
            item_instance_id: [1; 16],
            definition: backpack.definition.clone(),
            quantity: 1,
        };
        let mut failures = Vec::new();
        let mut coverage = [0, 0];
        let mut reviewed_yields = vec![
            (
                "oteryn:creature.evil_prospector",
                "oteryn:item.tibia.i8092",
                false,
            ),
            (
                "oteryn:creature.glooth_fairy",
                "oteryn:item.tibia.i21143",
                true,
            ),
            (
                "oteryn:creature.glooth_fairy",
                "oteryn:item.tibia.i21144",
                true,
            ),
            (
                "oteryn:creature.vok_the_freakish",
                "oteryn:item.tibia.i1781",
                true,
            ),
            ("oteryn:creature.rat", "oteryn:item.tibia.i37109", false),
        ];
        for record in &native.creature_profiles().records {
            let creature = &record.profile.target.key;
            let Ok(row) = rewards.row(creature) else {
                continue;
            };
            let Some(reference) = &row.loot_table_ref else {
                continue;
            };
            let personal = rewards.reward_boss_corpse(creature).is_some();
            coverage[usize::from(personal)] += 1;
            // The Fiendish award uses the same stack and pickup policy as ordinary loot.
            let fiendish = (creature == "oteryn:creature.rat").then_some(&slivers);
            for entry in yields(&row.loot_table.entries).chain(fiendish) {
                if entry.probability_ppm == Some(0)
                    || (is_surprise_bag(entry) && !is_global_event_loot_table(reference))
                {
                    continue;
                }
                let definition = TypedDefinitionRef {
                    family: entry.item.family.clone(),
                    production_key: entry.item.production_key.clone(),
                    revision_ref: entry.item.revision_ref.clone(),
                };
                let Ok(item) = generation_item_facts(native, &definition) else {
                    // Legacy materialization alone cannot supply an active pickup policy.
                    continue;
                };
                if item.stack == ItemStackClass::Unknown {
                    continue;
                }
                let (maximum, stackable) = stack_maximum(item.stack).map_err(|reason| {
                    format!("{creature}, {}: {reason:?}", definition.production_key)
                })?;
                reviewed_yields.retain(|&(required_creature, required_item, required_personal)| {
                    (required_creature, required_item, required_personal)
                        != (
                            creature.as_str(),
                            definition.production_key.as_str(),
                            personal,
                        )
                });
                let expected = match entry.details.charges {
                    _ if !stackable || entry.details.container_capacity.is_some() => 1,
                    Some(charges) => charges,
                    None => entry.max_count,
                };
                // Isolate each yield at its maximum so other entries cannot exhaust
                // a corpse or suppress a unique drop.
                let mut source = entry.clone();
                source.min_count = source.max_count;
                source.probability_ppm = Some(1_000_000);
                source.details.gut_bonus_hundredths = 1_000;
                source.details.children.clear();
                let table = LootTableDefinition {
                    algorithm: row.loot_table.algorithm,
                    entries: vec![source],
                };
                let plan = if personal {
                    plan_boss_loot([1; 16], [2; 16], reference, &table, 2.0, true)?
                } else {
                    plan_creature_loot(
                        LootPlanDeathKey::new([1; 16], [2; 16], 1, 1, 1),
                        reference,
                        &table,
                    )?
                };
                let mut total = 0;
                let mut ordinals = std::collections::HashSet::new();
                for planned in &plan.entries {
                    let source = InventoryItem {
                        item_instance_id: [2; 16],
                        definition: definition.clone(),
                        quantity: planned.quantity,
                    };
                    if let Err(reason) = plan_transfer(&PlanInput {
                        destination: ItemTransferDestination::MainBackpack,
                        item: &item,
                        backpack: Some(&backpack),
                        source: &source,
                        source_has_entries: false,
                        slot: Some(&slot),
                        entries: &[],
                    }) {
                        failures.push(format!(
                            "{creature}, {}: {} units refused with {reason:?}",
                            definition.production_key, planned.quantity
                        ));
                    }
                    assert!(ordinals.insert(planned.draw_ordinal), "{creature}");
                    assert_eq!(planned.charges, entry.details.charges, "{creature}");
                    total += planned.quantity;
                }
                if total != expected
                    || plan.entries.len() != expected.div_ceil(maximum) as usize
                    || entry.details.stackable != stackable
                {
                    failures.push(format!(
                        "{creature}, {}: {} units in {} instances (stackable {}), expected {expected} units in {} instances (stackable {stackable})",
                        definition.production_key,
                        total,
                        plan.entries.len(),
                        entry.details.stackable,
                        expected.div_ceil(maximum),
                    ));
                }
            }
        }
        assert!(coverage.iter().all(|count| *count > 0), "{coverage:?}");
        assert!(
            reviewed_yields.is_empty(),
            "missing admitted yields: {reviewed_yields:?}"
        );
        assert!(failures.is_empty(), "{}", failures.join("\n"));
        Ok(())
    }
}
