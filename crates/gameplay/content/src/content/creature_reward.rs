//! ARCH-KILL-REWARD-LOGOUT-1 §1.1: the immutable reward rows of one active generation, keyed
//! by the creature definition reference the runtime uses for its creature policies. A creature
//! without a settleable row fails closed: its kill settles nothing and logs one
//! `kill_reward_refused` line with the reason.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::{Arc, LazyLock};

use oteryn_simulation_determinism::ExactI64;
use serde::Deserialize;

use super::native_gameplay::CreatureProfilesDocument;
use super::{
    ContentError, ProjectEvidenceLimits, ProjectV2AuthoringProfileData, ProjectV2CreatureAuthoring,
    ProjectV2DefinitionRef,
};
use crate::domain::bestiary::BestiaryRace;
use crate::domain::bosstiary::{BosstiaryBoss, progress};
use oteryn_gameplay_domain::domain::loot_data::{
    GAMEITEM01_CORPSE_CONTAINER_ENTRIES_MAX, LootDefinitionRef, LootSelectionAlgorithm,
    LootTableDefinition, LootTableEntry,
};

pub const LOOT_TABLES_SCHEMA: &str = "OTERYN_NATIVE_LOOT_TABLES/v1";
/// The record bound of each list of the loot section.
const LOOT_SECTION_RECORDS_MAX: usize = 4096;
/// The record bound of the section's `items` list: one fact row per Item definition at most.
const LOOT_SECTION_ITEMS_MAX: usize = 8192;

/// What one creature's kill settles.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreatureRewardRow {
    pub xp_amount: ExactI64,
    pub corpse_item: LootDefinitionRef,
    pub loot_table_ref: LootDefinitionRef,
    pub loot_table: LootTableDefinition,
    pub race: Option<BestiaryRace>,
    pub boss: Option<BosstiaryBoss>,
}

/// Facts of admitted native profiles used by Soulpit and Echo Warden. Conversion draws use
/// the separate pinned source roster, which does not admit additional native identities.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreatureEncounterFacts {
    pub display_name: String,
    pub bestiary_stars: Option<u8>,
    pub has_boss_race: bool,
    pub maximum_health: Option<u64>,
    pub experience: Option<u64>,
    pub soul_core: Option<LootDefinitionRef>,
    source_soul_core: Option<(u32, LootDefinitionRef)>,
}

/// Why a creature's kill settles nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoSettlementReason {
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
    pub const fn as_str(self) -> &'static str {
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
pub struct CreatureRewardTable {
    rows: HashMap<String, Result<Arc<CreatureRewardRow>, NoSettlementReason>>,
    bosses: HashMap<String, BosstiaryBoss>,
    forge_sliver: Option<LootDefinitionRef>,
    daily_boss_equipment: HashSet<String>,
    encounter_facts: BTreeMap<String, CreatureEncounterFacts>,
    admitted_source_items: HashMap<u32, LootDefinitionRef>,
}

impl CreatureRewardTable {
    /// Native encounter facts, independent of ordinary corpse loot admission.
    pub fn encounter_facts(&self, creature: &str) -> Option<&CreatureEncounterFacts> {
        self.encounter_facts.get(creature)
    }

    /// Mintable Item references already admitted by this generation's existing loot owner.
    pub fn admitted_source_item(&self, source_id: u32) -> Option<&LootDefinitionRef> {
        self.admitted_source_items.get(&source_id)
    }

    /// Additional callback Items are admitted by the same validated native Item profiles as
    /// carried equipment and use. Regular monster loot tables are not the complete Item set.
    pub(crate) fn with_encounter_items(
        mut self,
        items: Option<&super::native_gameplay::ItemProfilesDocument>,
    ) -> Self {
        if let Some(items) = items {
            for record in &items.records {
                if !record.admission.materializable {
                    continue;
                }
                let definition = &record.production_definition;
                if definition.family != "Item"
                    || definition.production_key != record.production_binding.target.key
                    || definition.revision_ref != record.production_binding.target.revision
                {
                    continue;
                }
                let Ok(source_id) = record.production_binding.external_id.parse::<u32>() else {
                    continue;
                };
                if source_id == 0 {
                    continue;
                }
                self.admitted_source_items.insert(
                    source_id,
                    LootDefinitionRef::new(
                        &definition.family,
                        &definition.production_key,
                        &definition.revision_ref,
                    ),
                );
            }
        }
        for facts in self.encounter_facts.values_mut() {
            facts.soul_core = facts.source_soul_core.as_ref().and_then(|(id, expected)| {
                self.admitted_source_items
                    .get(id)
                    .filter(|actual| *actual == expected)
                    .cloned()
            });
        }
        self
    }

    /// Boss credit is independent of corpse and loot materialization admission.
    pub fn boss(&self, creature: &str) -> Option<&BosstiaryBoss> {
        self.bosses.get(creature)
    }

    /// Equipment membership comes from this generation's admitted Item policies.
    pub fn is_daily_boss_equipment(&self, key: &str) -> bool {
        self.daily_boss_equipment.contains(key)
    }

    pub(crate) fn with_daily_boss_equipment(
        mut self,
        items: Option<&super::native_gameplay::ItemProfilesDocument>,
    ) -> Self {
        use super::{
            ReferenceEquipmentSlot as Slot, ReferenceItemField as F, ReferenceWeaponType as Weapon,
        };
        if let Some(items) = items {
            for item in &items.records {
                let weapon = matches!(&item.semantics.weapon, F::Known(weapon)
                    if matches!(weapon.weapon_type, F::Known(Weapon::Axe | Weapon::Club | Weapon::Distance | Weapon::Shield | Weapon::Spellbook | Weapon::Sword | Weapon::Wand)));
                let quiver = matches!(&item.semantics.container, F::Known(container) if matches!(container.capacity, F::Known(1..)));
                let equipment = matches!(&item.semantics.equipment, F::Known(equipment)
                if matches!(&equipment.patterns, F::Known(patterns)
                    if patterns.iter().any(|p| match p.primary_slot {
                        F::Known(Slot::Head | Slot::Torso | Slot::Legs | Slot::Feet | Slot::Amulet | Slot::Ring) => true,
                        F::Known(Slot::Weapon | Slot::Shield) => weapon,
                        F::Known(Slot::Extra) => quiver,
                        _ => false,
                    })));
                // Canary monster.lua equipmentBags, constants.lua:8-10 @47dfd51f.
                let bag = matches!(
                    item.production_definition.production_key.as_str(),
                    "oteryn:item.tibia.i34109"
                        | "oteryn:item.tibia.i39546"
                        | "oteryn:item.tibia.i43895"
                );
                if equipment || bag {
                    self.daily_boss_equipment
                        .insert(item.production_definition.production_key.clone());
                }
            }
        }
        self
    }

    /// The existing corpse loot admission owns the materializable Forge resource too.
    pub fn forge_sliver(&self) -> Option<&LootDefinitionRef> {
        self.forge_sliver.as_ref()
    }
    #[cfg(any(test, feature = "test-support"))]
    pub fn insert(
        &mut self,
        creature: impl Into<String>,
        row: Result<CreatureRewardRow, NoSettlementReason>,
    ) {
        self.rows.insert(creature.into(), row.map(Arc::new));
    }

    pub fn row(&self, creature: &str) -> Result<Arc<CreatureRewardRow>, NoSettlementReason> {
        self.rows
            .get(creature)
            .cloned()
            .unwrap_or(Err(NoSettlementReason::NoLootBinding))
    }
}

/// One definition reference of the loot section, as the producer copies it.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SectionRef {
    pub family: String,
    pub key: String,
    pub revision: String,
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
pub struct CreatureLootRow {
    pub creature: SectionRef,
    pub loot: Option<SectionRef>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SectionLootEntry {
    pub item: SectionRef,
    pub min_count: u32,
    pub max_count: u32,
    pub probability_ppm: Option<u32>,
    #[serde(default)]
    pub children: Vec<SectionLootEntry>,
    #[serde(default)]
    pub unique: bool,
    #[serde(default)]
    pub subtype: Option<u16>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SectionLootTable {
    pub identity: SectionRef,
    pub algorithm: String,
    pub entries: Vec<SectionLootEntry>,
}

/// The admission facts of one Item, copied by the producer from the Item definitions (CP D929).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SectionItemFacts {
    pub item: SectionRef,
    pub materializable: bool,
    pub container_capacity: Option<u32>,
    #[serde(default)]
    pub charges: Option<u32>,
    #[serde(default)]
    pub stackable: bool,
    #[serde(default)]
    pub creature_product: bool,
}

/// The pinned `loot_tables` section (`OTERYN_NATIVE_LOOT_TABLES/v1`).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LootTablesSection {
    pub schema: String,
    pub creature_loot: Vec<CreatureLootRow>,
    pub tables: Vec<SectionLootTable>,
    pub items: Vec<SectionItemFacts>,
}

fn refuse(reason: &'static str) -> ContentError {
    ContentError::InvalidArtifact(reason)
}

fn lower_loot_entries(
    entries: &[SectionLootEntry],
    items: &HashMap<&SectionRef, &SectionItemFacts>,
    depth: usize,
) -> Result<Vec<LootTableEntry>, NoSettlementReason> {
    if depth > 8 || entries.len() > LOOT_SECTION_RECORDS_MAX {
        return Err(NoSettlementReason::LootItemInadmissible);
    }
    entries
        .iter()
        .map(|entry| {
            let facts = items
                .get(&entry.item)
                .filter(|facts| facts.materializable)
                .ok_or(NoSettlementReason::LootItemInadmissible)?;
            if !entry.children.is_empty() && facts.container_capacity.is_none() {
                return Err(NoSettlementReason::LootItemInadmissible);
            }
            Ok(LootTableEntry {
                item: entry.item.lower(),
                min_count: entry.min_count,
                max_count: entry.max_count,
                probability_ppm: entry.probability_ppm,
                details: oteryn_gameplay_domain::domain::loot_data::LootEntryDetails {
                    children: lower_loot_entries(&entry.children, items, depth + 1)?,
                    container_capacity: facts.container_capacity,
                    charges: facts.charges,
                    stackable: facts.stackable,
                    subtype: entry.subtype,
                    unique: entry.unique,
                    creature_product: facts.creature_product,
                    gut_bonus_hundredths: 0,
                },
            })
        })
        .collect()
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
    pub fn decode(
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
    pub fn from_pin(
        creatures: &CreatureProfilesDocument,
        section: Option<&LootTablesSection>,
    ) -> Self {
        section.map_or_else(Self::default, |section| {
            Self::build(creatures, section, canonical_races())
        })
    }

    /// Pure: one row per pinned creature, keyed by its definition key. A row that cannot settle
    /// carries its refusal reason; the generation still activates.
    pub fn build(
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
                        row(profile, *loot, &tables, &items, race).map(|mut row| {
                            row.boss = boss_metadata(profile, &target.key);
                            row
                        })
                    })
                    .map(Arc::new);
                (target.key.clone(), row)
            })
            .collect();
        let bosses = creatures
            .records
            .iter()
            .filter_map(|record| {
                let ProjectV2AuthoringProfileData::Creature(profile) = &record.profile.data else {
                    return None;
                };
                let key = &record.profile.target.key;
                Some((key.clone(), boss_metadata(profile, key)?))
            })
            .collect();
        let forge_sliver = section
            .items
            .iter()
            .find(|facts| facts.item.key == "oteryn:item.tibia.i37109" && facts.materializable)
            .map(|facts| facts.item.lower());
        let admitted_source_items = section
            .items
            .iter()
            .filter(|facts| facts.materializable)
            .filter_map(|facts| {
                let source_id = facts
                    .item
                    .key
                    .strip_prefix("oteryn:item.tibia.i")?
                    .parse()
                    .ok()?;
                Some((source_id, facts.item.lower()))
            })
            .collect::<HashMap<u32, LootDefinitionRef>>();
        let encounter_facts = creatures
            .records
            .iter()
            .filter_map(|record| {
                let ProjectV2AuthoringProfileData::Creature(profile) = &record.profile.data else {
                    return None;
                };
                let details = profile.details.as_ref()?;
                let source_soul_core = details.soul_core_item.as_ref().and_then(|core| {
                    let source_id = core.key.strip_prefix("oteryn:item.tibia.i")?.parse().ok()?;
                    (core.family == super::ProjectV2Family::Item).then(|| {
                        (
                            source_id,
                            LootDefinitionRef::new("Item", &core.key, &core.revision),
                        )
                    })
                });
                let soul_core = source_soul_core.as_ref().and_then(|(id, expected)| {
                    admitted_source_items
                        .get(id)
                        .filter(|actual| *actual == expected)
                        .cloned()
                });
                Some((
                    record.profile.target.key.clone(),
                    CreatureEncounterFacts {
                        display_name: details.display_name.clone(),
                        bestiary_stars: source_bestiary_stars(profile),
                        has_boss_race: details
                            .bosstiary
                            .as_ref()
                            .is_some_and(|boss| !boss.category.is_empty())
                            || profile
                                .bosstiary
                                .as_ref()
                                .is_some_and(|boss| !boss.category.is_empty()),
                        maximum_health: profile.health,
                        experience: profile.experience,
                        soul_core,
                        source_soul_core,
                    },
                ))
            })
            .collect();
        Self {
            rows,
            bosses,
            forge_sliver,
            daily_boss_equipment: HashSet::new(),
            encounter_facts,
            admitted_source_items,
        }
    }
}

/// W07/Echo rules read exact Crystal 201189f stars. Native Bestiary progression is
/// separately authored and intentionally remains unchanged by this encounter metadata.
fn source_bestiary_stars(profile: &ProjectV2CreatureAuthoring) -> Option<u8> {
    let name = &profile.details.as_ref()?.display_name;
    oteryn_gameplay_domain::domain::soulpit::source_monster(name).map(|monster| monster.stars)
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
            let entries = lower_loot_entries(&table.entries, items, 0)?;
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
        boss: None,
    })
}

fn boss_metadata(profile: &ProjectV2CreatureAuthoring, key: &str) -> Option<BosstiaryBoss> {
    let category = profile
        .details
        .as_ref()
        .and_then(|details| {
            details
                .bosstiary
                .as_ref()
                .map(|boss| boss.category.as_str())
        })
        .or_else(|| {
            profile
                .bosstiary
                .as_ref()
                .map(|boss| boss.category.as_str())
        })?;
    progress(category, 0)?;
    Some(BosstiaryBoss {
        key: key.to_owned(),
        category: category.to_owned(),
    })
}

#[cfg(test)]
mod encounter_source_tests {
    use super::*;

    #[test]
    fn encounter_stars_follow_pinned_source_without_rewriting_native_bestiary()
    -> Result<(), serde_json::Error> {
        for (name, native, expected) in [
            ("Chocolate Blob", 0, Some(1)),
            ("Corrupted Skeleton", 1, Some(2)),
            ("fixture:unknown-source", 5, None),
        ] {
            let profile: ProjectV2CreatureAuthoring = serde_json::from_value(serde_json::json!({
                "details": {
                    "display_name": name, "inspection": "", "defense": 0, "critical_chance_ppm": 0,
                    "flags": {"attackable": true, "illusionable": false, "health_hidden": false},
                    "summoning": {"summonable": false, "convinceable": false, "is_familiar": false},
                    "system_eligibility": {"prey": false, "exclusive_prey": false, "forge": false, "reward_boss": false},
                    "spawn_eligibility": {"period": "All", "ignore_period_underground": false, "blocked_by_nearby_players": false},
                    "bestiary": {"class": "fixture", "taxonomy": "fixture", "stars": native}
                }
            }))?;
            assert_eq!(source_bestiary_stars(&profile), expected, "{name}");
            assert_eq!(
                profile
                    .details
                    .as_ref()
                    .and_then(|details| details.bestiary.as_ref())
                    .and_then(|bestiary| bestiary.stars),
                Some(native)
            );
        }
        Ok(())
    }
}
