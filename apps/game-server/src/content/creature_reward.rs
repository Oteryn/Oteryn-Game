//! ARCH-KILL-REWARD-LOGOUT-1 §1.1: the immutable reward rows of one active generation, keyed
//! by the creature definition reference the runtime uses for its creature policies. A creature
//! without a settleable row fails closed: its kill settles nothing and logs one
//! `kill_reward_refused` line with the reason.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, LazyLock};

use oteryn_simulation_determinism::ExactI64;
use serde::Deserialize;

use super::native_gameplay::CreatureProfilesDocument;
use super::{
    ContentError, ProjectEvidenceLimits, ProjectV2AuthoringProfileData, ProjectV2CreatureAuthoring,
    ProjectV2DefinitionRef,
};
use crate::combat::{
    GAMEITEM01_CORPSE_CONTAINER_ENTRIES_MAX, LootDefinitionRef, LootSelectionAlgorithm,
    LootTableDefinition, LootTableEntry,
};
use crate::domain::bestiary::BestiaryRace;

pub(crate) const LOOT_TABLES_SCHEMA: &str = "OTERYN_NATIVE_LOOT_TABLES/v1";
/// The record bound of each list of the loot section.
const LOOT_SECTION_RECORDS_MAX: usize = 4096;
/// The record bound of the section's `items` list: one fact row per Item definition at most.
const LOOT_SECTION_ITEMS_MAX: usize = 8192;

/// What one creature's kill settles.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CreatureRewardRow {
    pub(crate) xp_amount: ExactI64,
    pub(crate) corpse_item: LootDefinitionRef,
    pub(crate) loot_table_ref: LootDefinitionRef,
    pub(crate) loot_table: LootTableDefinition,
    pub(crate) race: Option<BestiaryRace>,
}

/// Why a creature's kill settles nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NoSettlementReason {
    /// The pin carries no `creature_loot` row for the creature.
    NoLootBinding,
    /// The bound loot reference has no pinned table.
    LootTableMissing,
    /// The creature profile names no corpse Item.
    CorpseItemMissing,
    /// The corpse Item is not an admitted, materializable container of capacity 1 to
    /// `GAMEITEM01-CORPSE-CONTAINER-ENTRIES`.
    CorpseItemInadmissible,
    /// A loot table entry names an Item that is not admitted and materializable.
    LootItemInadmissible,
    /// `profile.experience` is above `i64::MAX`.
    XpOutOfRange,
}

impl NoSettlementReason {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::NoLootBinding => "no_loot_binding",
            Self::LootTableMissing => "loot_table_missing",
            Self::CorpseItemMissing => "corpse_item_missing",
            Self::CorpseItemInadmissible => "corpse_item_inadmissible",
            Self::LootItemInadmissible => "loot_item_inadmissible",
            Self::XpOutOfRange => "xp_out_of_range",
        }
    }
}

/// The reward rows of one active generation. A creature with no entry is `no_loot_binding`.
#[derive(Debug, Clone, Default)]
pub(crate) struct CreatureRewardTable {
    rows: HashMap<String, Result<Arc<CreatureRewardRow>, NoSettlementReason>>,
}

impl CreatureRewardTable {
    #[cfg(test)]
    pub(crate) fn insert(
        &mut self,
        creature: impl Into<String>,
        row: Result<CreatureRewardRow, NoSettlementReason>,
    ) {
        self.rows.insert(creature.into(), row.map(Arc::new));
    }

    pub(crate) fn row(&self, creature: &str) -> Result<Arc<CreatureRewardRow>, NoSettlementReason> {
        self.rows
            .get(creature)
            .cloned()
            .unwrap_or(Err(NoSettlementReason::NoLootBinding))
    }
}

/// One definition reference of the loot section, as the producer copies it.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SectionRef {
    pub(crate) family: String,
    pub(crate) key: String,
    pub(crate) revision: String,
}

impl SectionRef {
    fn matches(&self, reference: &ProjectV2DefinitionRef, family: &str) -> bool {
        self.family == family && self.key == reference.key && self.revision == reference.revision
    }

    fn lower(&self) -> LootDefinitionRef {
        LootDefinitionRef::new(&self.family, &self.key, &self.revision)
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CreatureLootRow {
    pub(crate) creature: SectionRef,
    pub(crate) loot: Option<SectionRef>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SectionLootEntry {
    pub(crate) item: SectionRef,
    pub(crate) min_count: u32,
    pub(crate) max_count: u32,
    pub(crate) probability_ppm: Option<u32>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SectionLootTable {
    pub(crate) identity: SectionRef,
    pub(crate) algorithm: String,
    pub(crate) entries: Vec<SectionLootEntry>,
}

/// The admission facts of one Item, copied by the producer from the Item definitions (CP D929).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SectionItemFacts {
    pub(crate) item: SectionRef,
    pub(crate) materializable: bool,
    pub(crate) container_capacity: Option<u32>,
}

/// The pinned `loot_tables` section (`OTERYN_NATIVE_LOOT_TABLES/v1`).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct LootTablesSection {
    pub(crate) schema: String,
    pub(crate) creature_loot: Vec<CreatureLootRow>,
    pub(crate) tables: Vec<SectionLootTable>,
    pub(crate) items: Vec<SectionItemFacts>,
}

fn refuse(reason: &'static str) -> ContentError {
    ContentError::InvalidArtifact(reason)
}

fn algorithm(name: &str) -> Option<LootSelectionAlgorithm> {
    Some(match name {
        "IndependentBernoulliPpm" => LootSelectionAlgorithm::IndependentBernoulliPpm,
        "WeightedSingleSelection" => LootSelectionAlgorithm::WeightedSingleSelection,
        "GuaranteedEntries" => LootSelectionAlgorithm::GuaranteedEntries,
        "NestedGroups" => LootSelectionAlgorithm::NestedGroups,
        _ => return None,
    })
}

impl LootTablesSection {
    /// Decodes the section against the pinned creature profiles. Refuses the manifest when the
    /// section is unbounded or malformed, a pinned creature has no `creature_loot` row, a row
    /// names a creature that is not pinned, a row, table or Item is given twice, or a table is
    /// referenced by no row.
    pub(crate) fn decode(
        bytes: &[u8],
        creatures: &CreatureProfilesDocument,
    ) -> Result<Self, ContentError> {
        let section: Self = serde_json::from_slice(bytes)
            .map_err(|_| refuse("native gameplay loot section encoding"))?;
        if section.schema != LOOT_TABLES_SCHEMA
            || section.creature_loot.len() > LOOT_SECTION_RECORDS_MAX
            || section.tables.len() > LOOT_SECTION_RECORDS_MAX
            || section.items.len() > LOOT_SECTION_ITEMS_MAX
        {
            return Err(refuse("native gameplay loot section schema or bounds"));
        }
        let pinned: HashMap<&str, &ProjectV2DefinitionRef> = creatures
            .records
            .iter()
            .map(|record| (record.profile.target.key.as_str(), &record.profile.target))
            .collect();
        let mut bound = HashSet::new();
        let mut referenced = HashSet::new();
        for row in &section.creature_loot {
            let exact = pinned
                .get(row.creature.key.as_str())
                .is_some_and(|target| row.creature.matches(target, "Creature"));
            if !exact || !bound.insert(row.creature.key.as_str()) {
                return Err(refuse(
                    "native gameplay loot binding is unpinned or repeated",
                ));
            }
            if let Some(loot) = &row.loot {
                if loot.family != "Loot" {
                    return Err(refuse("native gameplay loot binding family"));
                }
                referenced.insert(loot);
            }
        }
        if bound.len() != pinned.len() {
            return Err(refuse(
                "native gameplay pinned creature has no loot binding",
            ));
        }
        let mut tables = HashSet::new();
        for table in &section.tables {
            if algorithm(&table.algorithm).is_none()
                || !referenced.contains(&table.identity)
                || !tables.insert(table.identity.key.as_str())
            {
                return Err(refuse(
                    "native gameplay loot table is unreferenced or repeated",
                ));
            }
        }
        let mut items = HashSet::new();
        if section
            .items
            .iter()
            .any(|facts| facts.item.family != "Item" || !items.insert(facts.item.key.as_str()))
        {
            return Err(refuse("native gameplay loot Item facts repeated"));
        }
        Ok(section)
    }
}

/// The canonical Bestiary races by creature key; empty when the canonical source does not
/// decode, so every row then has no race.
fn canonical_races() -> &'static HashMap<String, BestiaryRace> {
    static RACES: LazyLock<HashMap<String, BestiaryRace>> = LazyLock::new(|| {
        let limits = ProjectEvidenceLimits {
            max_documents: 5,
            max_document_bytes: 2_500_000,
            max_total_bytes: 8_000_000,
            max_json_depth: 10,
            max_decoded_fields: 65_000,
            max_string_bytes: 1_000_000,
            max_locator_bytes: 128,
            max_locator_segments: 5,
            max_reference_records: LOOT_SECTION_RECORDS_MAX,
            max_import_records: 1,
            max_reimport_states: 1,
        };
        super::project::canonical_bestiary_rows(limits)
            .map(|rows| {
                rows.into_iter()
                    .map(|(race, _)| (race.key().to_owned(), race))
                    .collect()
            })
            .unwrap_or_default()
    });
    &RACES
}

impl CreatureRewardTable {
    /// The reward rows of the pinned creatures under the canonical Bestiary races (§1.1).
    pub(crate) fn from_pin(
        creatures: &CreatureProfilesDocument,
        section: Option<&LootTablesSection>,
    ) -> Self {
        section.map_or_else(Self::default, |section| {
            Self::build(creatures, section, canonical_races())
        })
    }

    /// Pure: one row per pinned creature, keyed by its definition key. A row that cannot settle
    /// carries its refusal reason; the generation still activates.
    pub(crate) fn build(
        creatures: &CreatureProfilesDocument,
        section: &LootTablesSection,
        races: &HashMap<String, BestiaryRace>,
    ) -> Self {
        let bindings: HashMap<&str, Option<&SectionRef>> = section
            .creature_loot
            .iter()
            .map(|row| (row.creature.key.as_str(), row.loot.as_ref()))
            .collect();
        let tables: HashMap<&SectionRef, &SectionLootTable> = section
            .tables
            .iter()
            .map(|table| (&table.identity, table))
            .collect();
        let items: HashMap<&SectionRef, &SectionItemFacts> = section
            .items
            .iter()
            .map(|facts| (&facts.item, facts))
            .collect();
        let rows = creatures
            .records
            .iter()
            .map(|record| {
                let target = &record.profile.target;
                let row = bindings
                    .get(target.key.as_str())
                    .ok_or(NoSettlementReason::NoLootBinding)
                    .and_then(|loot| {
                        let ProjectV2AuthoringProfileData::Creature(profile) = &record.profile.data
                        else {
                            return Err(NoSettlementReason::CorpseItemMissing);
                        };
                        let race = races
                            .get(&target.key)
                            .filter(|race| race.definition_revision() == target.revision)
                            .cloned();
                        row(profile, *loot, &tables, &items, race)
                    })
                    .map(Arc::new);
                (target.key.clone(), row)
            })
            .collect();
        Self { rows }
    }
}

fn row(
    profile: &ProjectV2CreatureAuthoring,
    loot: Option<&SectionRef>,
    tables: &HashMap<&SectionRef, &SectionLootTable>,
    items: &HashMap<&SectionRef, &SectionItemFacts>,
    race: Option<BestiaryRace>,
) -> Result<CreatureRewardRow, NoSettlementReason> {
    let xp_amount = i64::try_from(profile.experience.unwrap_or(0))
        .map(ExactI64::new)
        .map_err(|_| NoSettlementReason::XpOutOfRange)?;
    let corpse = profile
        .details
        .as_ref()
        .and_then(|details| details.corpse_item.as_ref())
        .ok_or(NoSettlementReason::CorpseItemMissing)?;
    let corpse = SectionRef {
        family: "Item".to_owned(),
        key: corpse.key.clone(),
        revision: corpse.revision.clone(),
    };
    let admitted = |item: &SectionRef| items.get(item).filter(|facts| facts.materializable);
    admitted(&corpse)
        .and_then(|facts| facts.container_capacity)
        .filter(|capacity| {
            (1..=GAMEITEM01_CORPSE_CONTAINER_ENTRIES_MAX as u64).contains(&u64::from(*capacity))
        })
        .ok_or(NoSettlementReason::CorpseItemInadmissible)?;
    let corpse_item = corpse.lower();
    let (loot_table_ref, loot_table) = match loot {
        // A `null` binding mints the corpse with no loot: an empty table under the corpse's own
        // definition, the reference the corpse's mint cause already uses in place of a table.
        None => (
            corpse_item.clone(),
            LootTableDefinition {
                algorithm: LootSelectionAlgorithm::IndependentBernoulliPpm,
                entries: Vec::new(),
            },
        ),
        Some(reference) => {
            let table = tables
                .get(reference)
                .ok_or(NoSettlementReason::LootTableMissing)?;
            let entries = table
                .entries
                .iter()
                .map(|entry| {
                    admitted(&entry.item)
                        .map(|_| LootTableEntry {
                            item: entry.item.lower(),
                            min_count: entry.min_count,
                            max_count: entry.max_count,
                            probability_ppm: entry.probability_ppm,
                        })
                        .ok_or(NoSettlementReason::LootItemInadmissible)
                })
                .collect::<Result<_, _>>()?;
            (
                reference.lower(),
                LootTableDefinition {
                    // Decode admits only the named algorithms.
                    algorithm: algorithm(&table.algorithm)
                        .unwrap_or(LootSelectionAlgorithm::NestedGroups),
                    entries,
                },
            )
        }
    };
    Ok(CreatureRewardRow {
        xp_amount,
        corpse_item,
        loot_table_ref,
        loot_table,
        race,
    })
}

#[cfg(test)]
#[path = "creature_reward_tests.rs"]
mod tests;
