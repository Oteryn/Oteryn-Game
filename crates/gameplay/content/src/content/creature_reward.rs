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
    LootDefinitionRef, LootSelectionAlgorithm, LootTableDefinition, LootTableEntry,
};

#[path = "creature_reward_empty_corpse.rs"]
mod empty_corpse;

pub const LOOT_TABLES_SCHEMA: &str = "OTERYN_NATIVE_LOOT_TABLES/v1";
/// The record bound of each list of the loot section.
const LOOT_SECTION_RECORDS_MAX: usize = 4096;
/// The record bound of the section's `items` list: one fact row per Item definition at most.
const LOOT_SECTION_ITEMS_MAX: usize = 8192;

/// What one creature's kill settles.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreatureRewardRow {
    pub xp_amount: ExactI64,
    /// Absent only for the closed source-qualified corpse-free encounter children.
    pub corpse_item: Option<LootDefinitionRef>,
    /// Absent with corpse_item for those children; no synthetic Item/Loot identity.
    pub loot_table_ref: Option<LootDefinitionRef>,
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
    /// The corpse Item is not admitted or exceeds this settlement path's container bound.
    /// Declared slots are independent of the actual entry/loot-plan count budgets.
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
    reward_boss_corpses: HashMap<String, Arc<CreatureRewardRow>>,
    forge_sliver: Option<LootTableEntry>,
    daily_boss_equipment: HashSet<String>,
    encounter_facts: BTreeMap<String, CreatureEncounterFacts>,
    admitted_source_items: HashMap<u32, LootDefinitionRef>,
}

impl CreatureRewardTable {
    /// Active native decode supplies the same validated effective policies used
    /// for physical births. Keep raw authoring in CreatureProfilesDocument.
    pub(crate) fn with_qualified_creature_health(
        mut self,
        policies: &[super::creature_policy::CompiledCreaturePolicy],
    ) -> Result<Self, ContentError> {
        let policies: HashMap<_, _> = policies
            .iter()
            .map(|policy| (policy.definition_key.as_str(), policy))
            .collect();
        for (key, facts) in &mut self.encounter_facts {
            let policy = policies
                .get(key.as_str())
                .ok_or(ContentError::InvalidArtifact(
                    "native encounter facts missing effective creature policy",
                ))?;
            facts.maximum_health = Some(
                u64::try_from(policy.maximum_health)
                    .ok()
                    .filter(|health| *health > 0)
                    .ok_or(ContentError::InvalidArtifact(
                        "native encounter facts effective health",
                    ))?,
            );
        }
        Ok(self)
    }

    /// Native encounter facts, independent of ordinary corpse loot admission.
    pub fn encounter_facts(&self, creature: &str) -> Option<&CreatureEncounterFacts> {
        self.encounter_facts.get(creature)
    }

    /// Empty physical corpse admission for reward bosses, independent of private loot.
    /// This never admits a personal reward table or an ordinary creature's loot.
    pub fn reward_boss_corpse(&self, creature: &str) -> Option<Arc<CreatureRewardRow>> {
        self.reward_boss_corpses.get(creature).cloned()
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
    pub fn forge_sliver(&self) -> Option<&LootTableEntry> {
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
    /// Ground custody qualified for ordinary loot only, even when a private boss shares the Item.
    #[serde(default)]
    pub ordinary_corpse_only: bool,
    #[serde(default)]
    pub charges: Option<u32>,
    #[serde(default)]
    pub stackable: bool,
    #[serde(default)]
    pub stack_maximum: Option<std::num::NonZeroU32>,
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

fn quest_loot_items() -> &'static HashSet<&'static str> {
    static ITEMS: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
        include_str!(concat!(env!("OUT_DIR"), "/quest-loot-items.txt"))
            .lines()
            .collect()
    });
    &ITEMS
}

fn is_boss(profile: &ProjectV2CreatureAuthoring) -> bool {
    profile.bosstiary.is_some()
        || profile.details.as_ref().is_some_and(|details| {
            details.bosstiary.is_some() || details.system_eligibility.reward_boss
        })
}

fn has_quest_loot(entry: &SectionLootEntry) -> bool {
    quest_loot_items().contains(entry.item.key.as_str())
        || entry.children.iter().any(has_quest_loot)
}

fn lower_loot_entries(
    entries: &[SectionLootEntry],
    items: &HashMap<&SectionRef, &SectionItemFacts>,
    depth: usize,
    admits_tusk_shield: bool,
    allow_unavailable_yields: bool,
) -> Result<Vec<LootTableEntry>, NoSettlementReason> {
    if depth > 8 || entries.len() > LOOT_SECTION_RECORDS_MAX {
        return Err(NoSettlementReason::LootItemInadmissible);
    }
    let mut lowered = Vec::with_capacity(entries.len());
    for entry in entries {
        // A shared native Item profile grants this drop only to source-bound tables.
        if !admits_tusk_shield && entry.item.key == "oteryn:item.tibia.i3443" {
            continue;
        }
        let Some(facts) = items.get(&entry.item).filter(|facts| facts.materializable) else {
            if allow_unavailable_yields && !has_quest_loot(entry) {
                continue;
            }
            return Err(NoSettlementReason::LootItemInadmissible);
        };
        if !entry.children.is_empty() && facts.container_capacity.is_none() {
            return Err(NoSettlementReason::LootItemInadmissible);
        }
        lowered.push(LootTableEntry {
            item: entry.item.lower(),
            min_count: entry.min_count,
            max_count: entry.max_count,
            probability_ppm: entry.probability_ppm,
            details: oteryn_gameplay_domain::domain::loot_data::LootEntryDetails {
                children: lower_loot_entries(
                    &entry.children,
                    items,
                    depth + 1,
                    admits_tusk_shield,
                    allow_unavailable_yields,
                )?,
                container_capacity: facts.container_capacity,
                charges: facts.charges,
                stackable: facts.stackable,
                corpse_stack_maximum: facts.stack_maximum,
                subtype: entry.subtype,
                unique: entry.unique,
                direct_corpse_drop: false,
                creature_product: facts.creature_product,
                gut_bonus_hundredths: 0,
                rated_probability_ppm: None,
            },
        });
    }
    Ok(lowered)
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

    /// Actual native Item proofs qualify the closed Soul War48-capacity and
    /// plain-body cases. Legacy callers retain the existing30-capacity rule.
    pub(crate) fn from_pin_with_native_corpses(
        creatures: &CreatureProfilesDocument,
        section: Option<&LootTablesSection>,
        items: Option<&super::native_gameplay::ItemProfilesDocument>,
    ) -> Self {
        let verified = verified_source_corpses(items);
        section.map_or_else(Self::default, |section| {
            Self::build_with_native_corpses(creatures, section, canonical_races(), &verified)
        })
    }

    /// Pure: one row per pinned creature, keyed by its definition key. A row that cannot settle
    /// carries its refusal reason; the generation still activates.
    pub fn build(
        creatures: &CreatureProfilesDocument,
        section: &LootTablesSection,
        races: &HashMap<String, BestiaryRace>,
    ) -> Self {
        Self::build_with_native_corpses(creatures, section, races, &HashSet::new())
    }

    fn build_with_native_corpses(
        creatures: &CreatureProfilesDocument,
        section: &LootTablesSection,
        races: &HashMap<String, BestiaryRace>,
        verified_source_corpses: &HashSet<String>,
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
                        row(
                            &target.key,
                            profile,
                            *loot,
                            &tables,
                            &items,
                            race,
                            CorpseAdmission {
                                maximum_corpse_capacity: oteryn_gameplay_domain::domain::loot_data::CORPSE_CONTAINER_ENTRIES_MAX,
                                verified_source_corpses,
                            },
                        )
                        .map(|mut row| {
                            row.boss = boss_metadata(profile, &target.key);
                            row
                        })
                    })
                    .map(Arc::new);
                (target.key.clone(), row)
            })
            .collect();
        let reward_boss_corpses = creatures
            .records
            .iter()
            .filter_map(|record| {
                let ProjectV2AuthoringProfileData::Creature(profile) = &record.profile.data else {
                    return None;
                };
                if !profile.details.as_ref()?.system_eligibility.reward_boss {
                    return None;
                }
                let target = &record.profile.target;
                // Require an explicit native binding even though physical reward-boss
                // corpses never carry that binding's private loot.
                bindings.get(target.key.as_str())?;
                let race = races
                    .get(&target.key)
                    .filter(|race| race.definition_revision() == target.revision)
                    .cloned();
                let mut corpse = empty_corpse::reward_boss_row(
                    &target.key,
                    profile,
                    &tables,
                    &items,
                    race,
                    verified_source_corpses,
                )
                .ok()?;
                corpse.boss = boss_metadata(profile, &target.key);
                Some((target.key.clone(), Arc::new(corpse)))
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
            .map(|facts| LootTableEntry {
                item: facts.item.lower(),
                min_count: 1,
                max_count: 1,
                probability_ppm: None,
                details: oteryn_gameplay_domain::domain::loot_data::LootEntryDetails {
                    stackable: facts.stackable,
                    corpse_stack_maximum: facts.stack_maximum,
                    direct_corpse_drop: true,
                    ..Default::default()
                },
            });
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
            reward_boss_corpses,
            forge_sliver,
            daily_boss_equipment: HashSet::new(),
            encounter_facts,
            admitted_source_items,
        }
    }
}

fn source_soul_war_corpse_pair(
    creature: &str,
    profile: &ProjectV2CreatureAuthoring,
    corpse: &SectionRef,
) -> bool {
    let expected_reward_boss = match (creature, corpse.key.as_str()) {
        ("oteryn:creature.goshnar_s_cruelty", "oteryn:item.tibia.i33859")
        | ("oteryn:creature.goshnar_s_greed", "oteryn:item.tibia.i33863")
        | ("oteryn:creature.goshnar_s_spite", "oteryn:item.tibia.i33867")
        | ("oteryn:creature.goshnar_s_malice", "oteryn:item.tibia.i33871")
        | ("oteryn:creature.goshnar_s_hatred", "oteryn:item.tibia.i33875")
        | (
            "oteryn:creature.goshnar_s_megalomania_blue"
            | "oteryn:creature.goshnar_s_megalomania_green",
            "oteryn:item.tibia.i33889",
        ) => true,
        ("oteryn:creature.bony_sea_devil", "oteryn:item.tibia.i33797")
        | ("oteryn:creature.cloak_of_terror", "oteryn:item.tibia.i33801")
        | ("oteryn:creature.many_faces", "oteryn:item.tibia.i33805")
        | ("oteryn:creature.branchy_crawler", "oteryn:item.tibia.i33809")
        | ("oteryn:creature.vibrant_phantom", "oteryn:item.tibia.i33813")
        | ("oteryn:creature.brachiodemon", "oteryn:item.tibia.i33817")
        | ("oteryn:creature.rotten_golem", "oteryn:item.tibia.i33897")
        | ("oteryn:creature.infernal_demon", "oteryn:item.tibia.i33901")
        | ("oteryn:creature.turbulent_elemental", "oteryn:item.tibia.i33905")
        | ("oteryn:creature.courage_leech", "oteryn:item.tibia.i33909") => false,
        _ => return false,
    };
    corpse.family == "Item"
        && corpse.revision == "definition-r1"
        && profile.details.as_ref().is_some_and(|details| {
            details.system_eligibility.reward_boss == expected_reward_boss
                && details.corpse_item.as_ref().is_some_and(|declared| {
                    matches!(declared.family, super::ProjectV2Family::Item)
                        && declared.key == corpse.key
                        && declared.revision == corpse.revision
                })
        })
}

fn source_questpc7_corpse_capacity(creature: &str, corpse: &SectionRef) -> Option<u32> {
    if corpse.family != "Item" || corpse.revision != "definition-r1" {
        return None;
    }
    match (creature, corpse.key.as_str()) {
        ("oteryn:creature.hydra", "oteryn:item.tibia.i6048") => Some(36),
        ("oteryn:creature.glooth_golem", "oteryn:item.tibia.i20972") => Some(38),
        ("oteryn:creature.werehyaena", "oteryn:item.tibia.i33821") => Some(48),
        _ => None,
    }
}

const SOURCE_PLAIN_CORPSES: &[(&str, &str)] = &[
    (
        "oteryn:creature.aspect_of_power",
        "oteryn:item.tibia.i33949",
    ),
    ("oteryn:creature.blue_butterfly", "oteryn:item.tibia.i4993"),
    ("oteryn:creature.hateful_soul", "oteryn:item.tibia.i33793"),
    (
        "oteryn:creature.necromantic_focus",
        "oteryn:item.tibia.i33984",
    ),
    ("oteryn:creature.poor_soul", "oteryn:item.tibia.i33891"),
    (
        "oteryn:creature.purple_butterfly",
        "oteryn:item.tibia.i4992",
    ),
    ("oteryn:creature.red_butterfly", "oteryn:item.tibia.i4991"),
    (
        "oteryn:creature.symbol_of_hatred",
        "oteryn:item.tibia.i33792",
    ),
    ("oteryn:creature.weeping_soul", "oteryn:item.tibia.i33876"),
];

fn source_plain_corpse_pair(creature: &str, corpse: &SectionRef, private_reward: bool) -> bool {
    corpse.family == "Item"
        && corpse.revision == "definition-r1"
        && if private_reward {
            (creature, corpse.key.as_str())
                == ("oteryn:creature.glitterscale", "oteryn:item.tibia.i11589")
        } else {
            SOURCE_PLAIN_CORPSES.contains(&(creature, corpse.key.as_str()))
        }
}

fn verified_source_corpses(
    items: Option<&super::native_gameplay::ItemProfilesDocument>,
) -> HashSet<String> {
    use super::native_gameplay::{NativeItemDestination, NativeItemStackClass};
    use super::{ReferenceItemField as F, ReferenceItemType};
    let Some(items) = items else {
        return HashSet::new();
    };
    items.records.iter().filter(|record| {
        let definition = &record.production_definition;
        let common = definition.family == "Item" && definition.revision_ref == "definition-r1"
            && record.production_binding.disposition == super::ProjectV2SourceIdentityDisposition::Exact
            && matches!(record.production_binding.target.family, super::ProjectV2Family::Item)
            && record.production_binding.target.key == definition.production_key
            && record.production_binding.target.revision == definition.revision_ref
            && record.admission.materializable
            && record.admission.stack_class == NativeItemStackClass::NonStackable
            && record.admission.legal_destinations.contains(&NativeItemDestination::Ground)
            && matches!(&record.semantics.stack, F::Known(value) if value.stackable == F::Known(false));
        if !common { return false; }
        let source_48 = matches!(definition.production_key.as_str(),
            "oteryn:item.tibia.i33797" | "oteryn:item.tibia.i33801" | "oteryn:item.tibia.i33805"
            | "oteryn:item.tibia.i33809" | "oteryn:item.tibia.i33813" | "oteryn:item.tibia.i33817"
            | "oteryn:item.tibia.i33859" | "oteryn:item.tibia.i33863" | "oteryn:item.tibia.i33867"
            | "oteryn:item.tibia.i33871" | "oteryn:item.tibia.i33875" | "oteryn:item.tibia.i33889"
            | "oteryn:item.tibia.i33897" | "oteryn:item.tibia.i33901" | "oteryn:item.tibia.i33905" | "oteryn:item.tibia.i33909")
            && record.admission.legal_destinations == [NativeItemDestination::Ground]
            && matches!(&record.semantics.classification, F::Known(value) if value.item_type == F::Known(ReferenceItemType::Container))
            && matches!(&record.semantics.physical, F::Known(value)
                if value.weight == F::Known(0) && value.movable == F::Known(false) && value.pickupable == F::Known(false))
            && matches!(&record.semantics.container, F::Known(value) if value.capacity == F::Known(48));
        let source_plain = (SOURCE_PLAIN_CORPSES.iter().any(|(_, key)| *key == definition.production_key)
            || definition.production_key == "oteryn:item.tibia.i11589")
            && matches!(&record.semantics.classification, F::Known(value) if value.item_type == F::NotApplicable)
            && record.semantics.container == F::NotApplicable;
        let questpc7_capacity = match definition.production_key.as_str() {
            "oteryn:item.tibia.i6048" => Some(36),
            "oteryn:item.tibia.i20972" => Some(38),
            "oteryn:item.tibia.i33821" => Some(48),
            _ => None,
        };
        let source_questpc7 = questpc7_capacity.is_some_and(|capacity|
            record.admission.legal_destinations == [NativeItemDestination::Ground]
            && matches!(&record.semantics.classification, F::Known(value) if value.item_type == F::Known(ReferenceItemType::Container))
            && matches!(&record.semantics.physical, F::Known(value)
                if value.weight == F::Known(0) && value.movable == F::Known(false) && value.pickupable == F::Known(false))
            && matches!(&record.semantics.container, F::Known(value) if value.capacity == F::Known(capacity)));
        source_48 || source_plain || source_questpc7
    }).map(|record| record.production_definition.production_key.clone()).collect()
}

/// W07/Echo rules read exact Crystal 201189f stars. Native Bestiary progression is
/// separately authored and intentionally remains unchanged by this encounter metadata.
fn source_bestiary_stars(profile: &ProjectV2CreatureAuthoring) -> Option<u8> {
    let name = &profile.details.as_ref()?.display_name;
    oteryn_gameplay_domain::domain::soulpit::source_monster(name).map(|monster| monster.stars)
}

fn source_tusk_shield_pair(creature: &str, loot: &SectionRef) -> bool {
    loot.revision == "definition-r1"
        && matches!(
            (creature, loot.key.as_str()),
            (
                "oteryn:creature.animated_clomp",
                "oteryn:loot.creature.animated_clomp"
            ) | ("oteryn:creature.clomp", "oteryn:loot.creature.clomp")
                | (
                    "oteryn:creature.terrified_elephant",
                    "oteryn:loot.creature.terrified_elephant"
                )
        )
}

struct CorpseAdmission<'a> {
    maximum_corpse_capacity: u32,
    verified_source_corpses: &'a HashSet<String>,
}

fn row(
    creature: &str,
    profile: &ProjectV2CreatureAuthoring,
    loot: Option<&SectionRef>,
    tables: &HashMap<&SectionRef, &SectionLootTable>,
    items: &HashMap<&SectionRef, &SectionItemFacts>,
    race: Option<BestiaryRace>,
    corpse_admission: CorpseAdmission<'_>,
) -> Result<CreatureRewardRow, NoSettlementReason> {
    let CorpseAdmission {
        maximum_corpse_capacity,
        verified_source_corpses,
    } = corpse_admission;
    let xp_amount = i64::try_from(profile.experience.unwrap_or(0))
        .map(ExactI64::new)
        .map_err(|_| NoSettlementReason::XpOutOfRange)?;
    let declared_corpse = profile
        .details
        .as_ref()
        .and_then(|details| details.corpse_item.as_ref());
    if declared_corpse.is_none()
        && loot.is_none()
        && profile
            .details
            .as_ref()
            .is_some_and(|details| !details.system_eligibility.reward_boss)
        && matches!(
            creature,
            "oteryn:creature.mushroom"
                | "oteryn:creature.pillar_of_dark_energy"
                | "oteryn:creature.spiteful_spitter"
        )
    {
        return Ok(CreatureRewardRow {
            xp_amount,
            corpse_item: None,
            loot_table_ref: None,
            loot_table: LootTableDefinition {
                algorithm: LootSelectionAlgorithm::IndependentBernoulliPpm,
                entries: Vec::new(),
            },
            race,
            boss: None,
        });
    }
    let corpse = declared_corpse.ok_or(NoSettlementReason::CorpseItemMissing)?;
    let corpse = SectionRef {
        family: "Item".to_owned(),
        key: corpse.key.clone(),
        revision: corpse.revision.clone(),
    };
    let private_reward = profile
        .details
        .as_ref()
        .is_some_and(|details| details.system_eligibility.reward_boss);
    let source_plain = verified_source_corpses.contains(&corpse.key)
        && source_plain_corpse_pair(creature, &corpse, private_reward);
    if !source_plain {
        items
            .get(&corpse)
            .filter(|facts| facts.materializable && !(private_reward && facts.ordinary_corpse_only))
            .and_then(|facts| facts.container_capacity)
            .filter(|capacity| {
                // Private reward custody has its own loot budget; only the
                // materializable physical container's positive capacity is admitted.
                (*capacity > 0 && private_reward)
                    || (1..=maximum_corpse_capacity).contains(capacity)
                    // Source capacity36 belongs only to this exact Seacrest
                    // body; the independent runtime loot-entry limit stays30.
                    || (*capacity == 36
                        && corpse.key == "oteryn:item.tibia.i21893"
                        && corpse.revision == "definition-r1")
                    || (*capacity == 48
                        && verified_source_corpses.contains(&corpse.key)
                        && source_soul_war_corpse_pair(creature, profile, &corpse))
                    || (verified_source_corpses.contains(&corpse.key)
                        && source_questpc7_corpse_capacity(creature, &corpse) == Some(*capacity))
            })
            .ok_or(NoSettlementReason::CorpseItemInadmissible)?;
    }
    // The legacy Loot-section materializable flag describes containers/carried
    // yields. A plain corpse has neither a capacity nor a fabricated yield;
    // its exact complete native Item proof above is its Ground authority.
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
            // Independent filler can be omitted. Boss and coupled tables
            // require the entire table, and every quest yield is mandatory.
            let algorithm =
                algorithm(&table.algorithm).unwrap_or(LootSelectionAlgorithm::NestedGroups);
            let allow_unavailable_yields = !source_plain
                && algorithm == LootSelectionAlgorithm::IndependentBernoulliPpm
                && !is_boss(profile);
            let admits_tusk_shield = source_tusk_shield_pair(creature, reference);
            // Keep the exact source roll in the Loot section, but exclude this
            // fixed scenery from Zamulosh's private Item settlement.
            let carried_entries;
            let entries = if private_reward
                && creature == "oteryn:creature.zamulosh"
                && reference.key == "oteryn:loot.creature.zamulosh"
                && reference.revision == "definition-r1"
            {
                carried_entries = table
                    .entries
                    .iter()
                    .filter(|entry| {
                        entry.item.key != "oteryn:item.tibia.i22555"
                            || entry.item.revision != "definition-r1"
                    })
                    .cloned()
                    .collect::<Vec<_>>();
                carried_entries.as_slice()
            } else {
                table.entries.as_slice()
            };
            let entries = lower_loot_entries(
                entries,
                items,
                0,
                admits_tusk_shield,
                allow_unavailable_yields,
            )?;
            (
                reference.lower(),
                LootTableDefinition { algorithm, entries },
            )
        }
    };
    // Private rewards go to personal chests. Ordinary loot needs corpse storage.
    if source_plain && !private_reward && !loot_table.entries.is_empty() {
        return Err(NoSettlementReason::CorpseItemInadmissible);
    }
    Ok(CreatureRewardRow {
        xp_amount,
        corpse_item: Some(corpse_item),
        loot_table_ref: Some(loot_table_ref),
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
mod tusk_shield_tests {
    use super::*;

    #[test]
    fn clomps_keep_the_donor_tusk_shield_roll() -> Result<(), Box<dyn std::error::Error>> {
        let (creatures, section) = super::protected_loot_tests::documents()?;
        let rewards = CreatureRewardTable::from_pin(&creatures, Some(&section));
        for creature in ["oteryn:creature.animated_clomp", "oteryn:creature.clomp"] {
            let reward = rewards.row(creature).map_err(NoSettlementReason::as_str)?;
            let shield = reward
                .loot_table
                .entries
                .iter()
                .find(|entry| entry.item.production_key == "oteryn:item.tibia.i3443")
                .ok_or("Clomp Tusk Shield missing")?;
            assert_eq!(shield.probability_ppm, Some(4_000));
            assert_eq!((shield.min_count, shield.max_count), (1, 1));
        }
        Ok(())
    }

    #[test]
    fn ordinary_elephant_cannot_roll_tusk_shield() -> Result<(), Box<dyn std::error::Error>> {
        let creatures = serde_json::from_slice(include_bytes!(
            "../../../../../content/creatures/definitions/spell-native-profiles.json"
        ))?;
        let section = LootTablesSection::decode(
            include_bytes!("../../../../../tools/content-schema/native-gameplay/loot-tables.json"),
            &creatures,
        )?;
        let rewards = CreatureRewardTable::from_pin(&creatures, Some(&section));
        for (creature, expected_probability) in [
            ("oteryn:creature.elephant", None),
            ("oteryn:creature.terrified_elephant", Some(800)),
        ] {
            let reward = rewards.row(creature).map_err(|reason| reason.as_str())?;
            assert_eq!(
                reward.loot_table.entries.iter().find_map(|entry| {
                    (entry.item.production_key == "oteryn:item.tibia.i3443")
                        .then_some(entry.probability_ppm)
                }),
                expected_probability.map(Some),
                "{creature}"
            );
        }
        Ok(())
    }
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

#[cfg(test)]
mod private_loot_capacity_tests {
    use super::*;

    #[test]
    fn private_boss_corpse_capacity_does_not_admit_ordinary_or_unmaterializable_corpses()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut profile: ProjectV2CreatureAuthoring = serde_json::from_value(serde_json::json!({
            "details": {
                "display_name": "fixture", "inspection": "", "defense": 0, "critical_chance_ppm": 0,
                "flags": {"attackable": true, "illusionable": false, "health_hidden": false},
                "summoning": {"summonable": false, "convinceable": false, "is_familiar": false},
                "system_eligibility": {"prey": false, "exclusive_prey": false, "forge": false, "reward_boss": true},
                "spawn_eligibility": {"period": "All", "ignore_period_underground": false, "blocked_by_nearby_players": false},
                "corpse_item": {"family": "Item", "key": "oteryn:item.fixture", "revision": "definition-r1"}
            }
        }))?;
        let corpse = SectionRef {
            family: "Item".into(),
            key: "oteryn:item.fixture".into(),
            revision: "definition-r1".into(),
        };
        let mut facts = SectionItemFacts {
            item: corpse.clone(),
            materializable: true,
            container_capacity: Some(64),
            ordinary_corpse_only: false,
            charges: None,
            stackable: false,
            stack_maximum: None,
            creature_product: false,
        };
        assert!(
            row(
                "oteryn:creature.fixture",
                &profile,
                None,
                &HashMap::new(),
                &HashMap::from([(&corpse, &facts)]),
                None,
                CorpseAdmission {
                    maximum_corpse_capacity: 30,
                    verified_source_corpses: &HashSet::new(),
                },
            )
            .is_ok()
        );
        profile
            .details
            .as_mut()
            .ok_or("details")?
            .system_eligibility
            .reward_boss = false;
        assert_eq!(
            row(
                "oteryn:creature.fixture",
                &profile,
                None,
                &HashMap::new(),
                &HashMap::from([(&corpse, &facts)]),
                None,
                CorpseAdmission {
                    maximum_corpse_capacity: 30,
                    verified_source_corpses: &HashSet::new(),
                },
            ),
            Err(NoSettlementReason::CorpseItemInadmissible)
        );
        profile
            .details
            .as_mut()
            .ok_or("details")?
            .system_eligibility
            .reward_boss = true;
        for capacity in [None, Some(0)] {
            facts.container_capacity = capacity;
            assert_eq!(
                row(
                    "oteryn:creature.fixture",
                    &profile,
                    None,
                    &HashMap::new(),
                    &HashMap::from([(&corpse, &facts)]),
                    None,
                    CorpseAdmission {
                        maximum_corpse_capacity: 30,
                        verified_source_corpses: &HashSet::new(),
                    },
                ),
                Err(NoSettlementReason::CorpseItemInadmissible)
            );
        }
        facts.container_capacity = Some(64);
        facts.materializable = false;
        assert_eq!(
            row(
                "oteryn:creature.fixture",
                &profile,
                None,
                &HashMap::new(),
                &HashMap::from([(&corpse, &facts)]),
                None,
                CorpseAdmission {
                    maximum_corpse_capacity: 30,
                    verified_source_corpses: &HashSet::new(),
                },
            ),
            Err(NoSettlementReason::CorpseItemInadmissible)
        );
        Ok(())
    }
}

#[cfg(test)]
mod private_loot_tests {
    use super::*;

    #[test]
    fn private_boss_corpse_capacity_does_not_admit_ordinary_or_unmaterializable_corpses()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut profile: ProjectV2CreatureAuthoring = serde_json::from_value(serde_json::json!({
            "details": {
                "display_name": "fixture", "inspection": "", "defense": 0, "critical_chance_ppm": 0,
                "flags": {"attackable": true, "illusionable": false, "health_hidden": false},
                "summoning": {"summonable": false, "convinceable": false, "is_familiar": false},
                "system_eligibility": {"prey": false, "exclusive_prey": false, "forge": false, "reward_boss": true},
                "spawn_eligibility": {"period": "All", "ignore_period_underground": false, "blocked_by_nearby_players": false},
                "corpse_item": {"family": "Item", "key": "oteryn:item.fixture", "revision": "definition-r1"}
            }
        }))?;
        let corpse = SectionRef {
            family: "Item".into(),
            key: "oteryn:item.fixture".into(),
            revision: "definition-r1".into(),
        };
        let mut facts = SectionItemFacts {
            item: corpse.clone(),
            materializable: true,
            container_capacity: Some(64),
            ordinary_corpse_only: false,
            charges: None,
            stackable: false,
            stack_maximum: None,
            creature_product: false,
        };
        assert!(
            row(
                "oteryn:creature.fixture",
                &profile,
                None,
                &HashMap::new(),
                &HashMap::from([(&corpse, &facts)]),
                None,
                CorpseAdmission {
                    maximum_corpse_capacity: 30,
                    verified_source_corpses: &HashSet::new(),
                },
            )
            .is_ok()
        );
        profile
            .details
            .as_mut()
            .ok_or("details")?
            .system_eligibility
            .reward_boss = false;
        assert_eq!(
            row(
                "oteryn:creature.fixture",
                &profile,
                None,
                &HashMap::new(),
                &HashMap::from([(&corpse, &facts)]),
                None,
                CorpseAdmission {
                    maximum_corpse_capacity: 30,
                    verified_source_corpses: &HashSet::new(),
                },
            ),
            Err(NoSettlementReason::CorpseItemInadmissible)
        );
        profile
            .details
            .as_mut()
            .ok_or("details")?
            .system_eligibility
            .reward_boss = true;
        for capacity in [None, Some(0)] {
            facts.container_capacity = capacity;
            assert_eq!(
                row(
                    "oteryn:creature.fixture",
                    &profile,
                    None,
                    &HashMap::new(),
                    &HashMap::from([(&corpse, &facts)]),
                    None,
                    CorpseAdmission {
                        maximum_corpse_capacity: 30,
                        verified_source_corpses: &HashSet::new(),
                    },
                ),
                Err(NoSettlementReason::CorpseItemInadmissible)
            );
        }
        facts.container_capacity = Some(64);
        facts.materializable = false;
        assert_eq!(
            row(
                "oteryn:creature.fixture",
                &profile,
                None,
                &HashMap::new(),
                &HashMap::from([(&corpse, &facts)]),
                None,
                CorpseAdmission {
                    maximum_corpse_capacity: 30,
                    verified_source_corpses: &HashSet::new(),
                },
            ),
            Err(NoSettlementReason::CorpseItemInadmissible)
        );
        Ok(())
    }
}

#[cfg(test)]
mod source_48_corpse_tests {
    use super::*;

    #[test]
    fn soul_war_48_capacity_requires_exact_native_ground_profile_and_actual_boss_pair()
    -> Result<(), String> {
        use crate::content::ReferenceItemField as F;
        use crate::content::native_gameplay::{ItemProfilesDocument, NativeItemDestination};
        let captured: serde_json::Value = serde_json::from_slice(include_bytes!(
            "../../../../../tools/content-schema/native-gameplay/boss-corpse-item-profiles.json"
        ))
        .map_err(|error| error.to_string())?;
        let mut items = ItemProfilesDocument {
            schema: "OTERYN_NATIVE_ITEM_PROFILES/v1".into(),
            records: serde_json::from_value(captured["records"].clone())
                .map_err(|error| error.to_string())?,
        };
        // The exact source closure includes sixteen48-capacity bodies and
        // five plain bodies; unrelated admitted Items remain excluded.
        assert_eq!(
            verified_source_corpses(Some(&items)),
            std::collections::HashSet::from(
                [
                    "oteryn:item.tibia.i33792",
                    "oteryn:item.tibia.i33797",
                    "oteryn:item.tibia.i33801",
                    "oteryn:item.tibia.i33805",
                    "oteryn:item.tibia.i33809",
                    "oteryn:item.tibia.i33813",
                    "oteryn:item.tibia.i33817",
                    "oteryn:item.tibia.i33859",
                    "oteryn:item.tibia.i33863",
                    "oteryn:item.tibia.i33867",
                    "oteryn:item.tibia.i33871",
                    "oteryn:item.tibia.i33875",
                    "oteryn:item.tibia.i33876",
                    "oteryn:item.tibia.i33889",
                    "oteryn:item.tibia.i33891",
                    "oteryn:item.tibia.i33897",
                    "oteryn:item.tibia.i33901",
                    "oteryn:item.tibia.i33905",
                    "oteryn:item.tibia.i33909",
                    "oteryn:item.tibia.i33949",
                    "oteryn:item.tibia.i33984",
                ]
                .map(str::to_owned)
            )
        );
        let greed_key = "oteryn:item.tibia.i33863";
        let original = items
            .records
            .iter()
            .position(|record| record.production_definition.production_key == greed_key)
            .ok_or("source Greed corpse")?;
        let record = items.records[original].clone();
        items.records[original]
            .admission
            .legal_destinations
            .push(NativeItemDestination::CharacterInventory);
        assert!(!verified_source_corpses(Some(&items)).contains(greed_key));
        items.records[original] = record.clone();
        if let F::Known(container) = &mut items.records[original].semantics.container {
            container.capacity = F::Known(47);
        }
        assert!(!verified_source_corpses(Some(&items)).contains(greed_key));
        items.records[original] = record;
        assert!(verified_source_corpses(Some(&items)).contains(greed_key));
        let mut profile: ProjectV2CreatureAuthoring = serde_json::from_value(serde_json::json!({
            "details": {
                "display_name":"Goshnar's Greed","inspection":"","defense":0,"critical_chance_ppm":0,
                "corpse_item":{"family":"Item","key":"oteryn:item.tibia.i33863","revision":"definition-r1"},
                "flags":{"attackable":true,"illusionable":false,"health_hidden":false},
                "summoning":{"summonable":false,"convinceable":false,"is_familiar":false},
                "system_eligibility":{"prey":false,"exclusive_prey":false,"forge":false,"reward_boss":true},
                "spawn_eligibility":{"period":"All","ignore_period_underground":false,"blocked_by_nearby_players":false}
            }
        })).map_err(|error| error.to_string())?;
        let corpse = SectionRef {
            family: "Item".into(),
            key: greed_key.into(),
            revision: "definition-r1".into(),
        };
        assert!(source_soul_war_corpse_pair(
            "oteryn:creature.goshnar_s_greed",
            &profile,
            &corpse
        ));
        assert!(!source_soul_war_corpse_pair(
            "oteryn:creature.rat",
            &profile,
            &corpse
        ));
        assert!(!source_soul_war_corpse_pair(
            "oteryn:creature.goshnar_s_malice",
            &profile,
            &corpse
        ));
        profile
            .details
            .as_mut()
            .ok_or("source details")?
            .system_eligibility
            .reward_boss = false;
        assert!(!source_soul_war_corpse_pair(
            "oteryn:creature.goshnar_s_greed",
            &profile,
            &corpse
        ));
        assert_eq!(
            oteryn_gameplay_domain::domain::loot_data::GAMEITEM01_CORPSE_CONTAINER_ENTRIES_MAX,
            48,
            "physical capacity never raises the existing loot/work bound"
        );
        Ok(())
    }
}

#[cfg(test)]
mod protected_loot_tests {
    use super::*;

    pub(super) fn documents()
    -> Result<(CreatureProfilesDocument, LootTablesSection), serde_json::Error> {
        let creatures = serde_json::from_str(include_str!(
            "../../../../../content/creatures/definitions/spell-native-profiles.json"
        ))?;
        let loot = serde_json::from_str(include_str!(
            "../../../../../tools/content-schema/native-gameplay/loot-tables.json"
        ))?;
        Ok((creatures, loot))
    }

    #[test]
    fn slice_three_tables_load_every_qualified_yield() -> Result<(), Box<dyn std::error::Error>> {
        fn missing_yields<'a>(
            entries: &'a [SectionLootEntry],
            admitted: &HashSet<&SectionRef>,
            missing: &mut Vec<&'a str>,
        ) {
            for entry in entries {
                if !admitted.contains(&entry.item) {
                    missing.push(&entry.item.key);
                }
                missing_yields(&entry.children, admitted, missing);
            }
        }

        let (creatures, _) = documents()?;
        let section = LootTablesSection::decode(
            include_bytes!("../../../../../tools/content-schema/native-gameplay/loot-tables.json"),
            &creatures,
        )?;
        let bindings: HashMap<_, _> = section
            .creature_loot
            .iter()
            .map(|row| (row.creature.key.as_str(), row.loot.as_ref()))
            .collect();
        let tables: HashMap<_, _> = section
            .tables
            .iter()
            .map(|table| (&table.identity, table.entries.as_slice()))
            .collect();
        let admitted = section
            .items
            .iter()
            .filter(|fact| fact.materializable)
            .map(|fact| &fact.item)
            .collect();
        let mut missing = Vec::new();
        let mut incomplete_tables = Vec::new();
        let creatures = include_str!(
            "../../../../../tools/content-schema/native-gameplay/cloot3-creatures.txt"
        );
        assert_eq!(creatures.lines().count(), 321);
        for creature in creatures.lines() {
            let table = bindings
                .get(creature)
                .and_then(|table| *table)
                .ok_or_else(|| format!("missing loot binding for {creature}"))?;
            let entries = tables
                .get(table)
                .ok_or_else(|| format!("missing table for {creature}"))?;
            let previous = missing.len();
            missing_yields(entries, &admitted, &mut missing);
            if missing.len() != previous {
                incomplete_tables.push(creature);
            }
        }
        // Zamulosh's exact stone wall is immovable and requires a settlement
        // destination the carried-loot loader does not support.
        assert_eq!(missing, ["oteryn:item.tibia.i22555"]);
        assert_eq!(incomplete_tables, ["oteryn:creature.zamulosh"]);
        for creature in [
            "oteryn:creature.ratmiral_blackwhiskers",
            "oteryn:creature.tentugly_s_head",
            "oteryn:creature.the_dread_maiden",
        ] {
            let reference = bindings[creature].ok_or("missing boss loot binding")?;
            assert!(
                tables[reference]
                    .iter()
                    .all(|entry| entry.item.key != "oteryn:item.tibia.i51302")
            );
        }
        let thawing = bindings["oteryn:creature.thawing_dragon_lord"]
            .ok_or("missing Thawing Dragon Lord binding")?;
        assert!(tables[thawing].iter().any(|entry| {
            entry.item.key == "oteryn:item.tibia.i7441"
                && entry.probability_ppm == Some(55_600)
                && (entry.min_count, entry.max_count) == (1, 1)
        }));
        Ok(())
    }

    #[test]
    fn loot_slice_one_loads_materializable_yields() -> Result<(), Box<dyn std::error::Error>> {
        let (creatures, _) = documents()?;
        let section = LootTablesSection::decode(
            include_bytes!("../../../../../tools/content-schema/native-gameplay/loot-tables.json"),
            &creatures,
        )?;
        let bindings: HashMap<_, _> = section
            .creature_loot
            .iter()
            .map(|row| (row.creature.key.as_str(), row.loot.as_ref()))
            .collect();
        let tables: HashMap<_, _> = section
            .tables
            .iter()
            .map(|row| (&row.identity, row))
            .collect();
        let items: HashMap<_, _> = section.items.iter().map(|row| (&row.item, row)).collect();
        let selected: Vec<_> = include_str!(
            "../../../../../tools/content-schema/native-gameplay/loot-slice-1-creatures.txt"
        )
        .lines()
        .collect();
        assert_eq!(selected.len(), 323);
        assert!(selected.windows(2).all(|pair| pair[0] < pair[1]));
        let native_items = serde_json::from_slice(include_bytes!(
            "../../../../../content/items/definitions/spell-native-profiles.json"
        ))?;
        let rewards = CreatureRewardTable::from_pin_with_native_corpses(
            &creatures,
            Some(&section),
            Some(&native_items),
        );
        let mut refused = Vec::new();
        let mut settled = 0;
        for creature in selected {
            let reference =
                bindings[creature].ok_or_else(|| format!("{creature}: missing loot"))?;
            let entries = &tables[reference].entries;
            match lower_loot_entries(
                entries,
                &items,
                0,
                source_tusk_shield_pair(creature, reference),
                false,
            ) {
                Ok(lowered) => {
                    assert_eq!(lowered.len(), entries.len(), "{creature}");
                    if matches!(
                        creature,
                        "oteryn:creature.crazed_dwarf" | "oteryn:creature.dwarf"
                    ) {
                        // Letter 3505 finding: both Dwarfs must admit their corpse before rolling loot.
                        let row = rewards.row(creature).map_err(|reason| {
                            format!("{creature}: Letter reward row refused: {reason:?}")
                        })?;
                        assert_eq!(
                            row.loot_table
                                .entries
                                .iter()
                                .filter(
                                    |entry| entry.item.production_key == "oteryn:item.tibia.i3505"
                                )
                                .count(),
                            1,
                            "{creature}: expected one materializable Letter roll"
                        );
                    }
                    match rewards.row(creature) {
                        Ok(row) => {
                            assert_eq!(row.loot_table.entries, lowered, "{creature}");
                            settled += 1;
                        }
                        Err(
                            NoSettlementReason::CorpseItemMissing
                            | NoSettlementReason::CorpseItemInadmissible,
                        ) => {}
                        Err(reason) => refused.push((creature, reason)),
                    }
                }
                Err(reason) => refused.push((creature, reason)),
            }
        }
        assert!(refused.is_empty(), "unavailable loot: {refused:?}");
        assert!(settled > 0, "expected pre-existing corpse admission");
        let reference = bindings["oteryn:creature.general_murius"]
            .ok_or("General Murius loot binding missing")?;
        let horn = lower_loot_entries(&tables[reference].entries, &items, 0, false, false)
            .map_err(NoSettlementReason::as_str)?
            .into_iter()
            .find(|entry| entry.item.production_key == "oteryn:item.tibia.i11472")
            .ok_or("Minotaur horn missing")?;
        assert!(horn.details.stackable);
        assert_eq!((horn.min_count, horn.max_count), (1, 2));
        for (creature, id, chance) in [
            ("animated_clomp", 3443, 4_000),
            ("clomp", 3443, 4_000),
            ("corrupted_ghost", 3565, 30_400),
            ("corrupted_ghost", 266, 213_900),
            ("corrupted_ghost", 268, 200_100),
            ("corrupted_skeleton", 266, 262_800),
            ("dharalion", 3103, 79_200),
        ] {
            let key = format!("oteryn:creature.{creature}");
            let reference = bindings[key.as_str()].ok_or("corrected loot binding missing")?;
            let item = format!("oteryn:item.tibia.i{id}");
            let matches: Vec<_> = tables[reference]
                .entries
                .iter()
                .filter(|entry| entry.item.key == item)
                .collect();
            assert_eq!(matches.len(), 1, "{creature}: {item} source roll count");
            let entry = matches[0];
            assert_eq!(entry.probability_ppm, Some(chance), "{creature}: {item}");
            assert_eq!((entry.min_count, entry.max_count), (1, 1));
        }
        Ok(())
    }

    #[test]
    fn corrupted_skeleton_settlement_keeps_the_corrected_potion()
    -> Result<(), Box<dyn std::error::Error>> {
        let (creatures, section) = documents()?;
        let rewards = CreatureRewardTable::from_pin(&creatures, Some(&section));
        let reward = rewards
            .row("oteryn:creature.corrupted_skeleton")
            .map_err(NoSettlementReason::as_str)?;
        assert!(reward.loot_table.entries.iter().any(|entry| {
            entry.item.production_key == "oteryn:item.tibia.i266"
                && entry.probability_ppm == Some(262_800)
        }));
        Ok(())
    }

    #[test]
    fn glitterscale_pins_flexible_dragon_scale() -> Result<(), Box<dyn std::error::Error>> {
        let (creatures, _) = documents()?;
        let loot = LootTablesSection::decode(
            include_bytes!("../../../../../tools/content-schema/native-gameplay/loot-tables.json"),
            &creatures,
        )?;
        let creature = "oteryn:creature.glitterscale";
        let reference = loot
            .creature_loot
            .iter()
            .find(|row| row.creature.key == creature)
            .and_then(|row| row.loot.as_ref())
            .ok_or("Glitterscale must bind the Crystal loot table")?;
        let table = loot
            .tables
            .iter()
            .find(|table| &table.identity == reference)
            .ok_or("Glitterscale loot table missing")?;
        assert_eq!(table.algorithm, "IndependentBernoulliPpm");
        let items = loot.items.iter().map(|row| (&row.item, row)).collect();
        let entries = lower_loot_entries(&table.entries, &items, 0, false, false)
            .map_err(NoSettlementReason::as_str)?;
        assert_eq!(entries.len(), 1);
        let entry = &entries[0];
        assert_eq!(entry.item.production_key, "oteryn:item.tibia.i11550");
        assert_eq!((entry.min_count, entry.max_count), (1, 1));
        assert_eq!(entry.probability_ppm, Some(700_000));
        assert_eq!(
            CreatureRewardTable::from_pin(&creatures, Some(&loot)).row(creature),
            Err(NoSettlementReason::CorpseItemInadmissible)
        );
        let native_items = serde_json::from_slice(include_bytes!(
            "../../../../../content/items/definitions/spell-native-profiles.json"
        ))?;
        let rewards = CreatureRewardTable::from_pin_with_native_corpses(
            &creatures,
            Some(&loot),
            Some(&native_items),
        );
        let reward = rewards.row(creature).map_err(NoSettlementReason::as_str)?;
        assert_eq!(reward.loot_table.entries, entries);
        Ok(())
    }

    #[test]
    fn active_private_raid_loot_admits_exact_silver_token_drops()
    -> Result<(), Box<dyn std::error::Error>> {
        let (creatures, _) = documents()?;
        let loot = LootTablesSection::decode(
            include_bytes!("../../../../../tools/content-schema/native-gameplay/loot-tables.json"),
            &creatures,
        )?;
        let table = CreatureRewardTable::from_pin(&creatures, Some(&loot));
        for (creature, corpse, chance) in [
            ("furyosa", 18118, 450_000),
            ("hirintror", 7282, 2_000),
            ("ocyakao", 7320, 2_000),
            ("the_welter", 18974, 2_000),
            ("tyrn", 18970, 50_000),
            ("weakened_shlorg", 18982, 30_000),
            ("white_pale", 18978, 2_000),
            ("willi_wasp", 5989, 50_000),
            ("zushuka", 18265, 920_000),
        ] {
            let key = format!("oteryn:creature.{creature}");
            let row = table
                .row(&key)
                .map_err(|reason| format!("{key}: {}", reason.as_str()))?;
            assert_eq!(
                row.corpse_item
                    .as_ref()
                    .map(|item| item.production_key.as_str()),
                Some(format!("oteryn:item.tibia.i{corpse}").as_str()),
            );
            let tokens: Vec<_> = row
                .loot_table
                .entries
                .iter()
                .filter(|entry| entry.item.production_key == "oteryn:item.tibia.i19083")
                .map(|entry| (entry.min_count, entry.max_count, entry.probability_ppm))
                .collect();
            assert_eq!(tokens, [(1, 1, Some(chance))], "{key}");
        }
        Ok(())
    }

    #[test]
    fn explorer_society_butterflies_admit_the_authored_plain_corpses()
    -> Result<(), Box<dyn std::error::Error>> {
        let (creatures, _) = documents()?;
        let loot = LootTablesSection::decode(
            include_bytes!("../../../../../tools/content-schema/native-gameplay/loot-tables.json"),
            &creatures,
        )?;
        let items = serde_json::from_str(include_str!(
            "../../../../../content/items/definitions/spell-native-profiles.json"
        ))?;
        let table = CreatureRewardTable::from_pin_with_native_corpses(
            &creatures,
            Some(&loot),
            Some(&items),
        );
        let callbacks: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../../content/quests/npc-callbacks/explorer-society.json"
        ))?;
        let mut count = 0;
        for callback in callbacks["callbacks"].as_array().ok_or("callbacks")? {
            let Some(species) = callback["creature_target"].as_str() else {
                continue;
            };
            if !species.ends_with("_butterfly") {
                continue;
            }
            let appearance = callback["corpse_target"]["appearance"]
                .as_u64()
                .ok_or("appearance")?;
            let reward = table.row(species).map_err(NoSettlementReason::as_str)?;
            let corpse = reward.corpse_item.as_ref().ok_or("missing native corpse")?;
            assert_eq!(
                corpse.production_key,
                format!("oteryn:item.tibia.i{appearance}")
            );
            assert!(reward.loot_table.entries.is_empty());
            count += 1;
        }
        assert_eq!(count, 3);
        Ok(())
    }

    #[test]
    fn crystal_boss_loot_resolves_named_source_items() -> Result<(), Box<dyn std::error::Error>> {
        let (creatures, _) = documents()?;
        let loot = LootTablesSection::decode(
            include_bytes!("../../../../../tools/content-schema/native-gameplay/loot-tables.json"),
            &creatures,
        )?;
        let rewards = CreatureRewardTable::from_pin(&creatures, Some(&loot));
        for (creature, item, chance) in [
            (
                "ayana_the_crimson_curse",
                "oteryn:item.tibia.i3039",
                263_600,
            ),
            (
                "ayana_the_crimson_curse",
                "oteryn:item.tibia.i43888",
                178_300,
            ),
            ("malvaroth", "oteryn:item.tibia.i49909", 70_000),
        ] {
            let reward = rewards
                .row(&format!("oteryn:creature.{creature}"))
                .map_err(NoSettlementReason::as_str)?;
            let entry = reward
                .loot_table
                .entries
                .iter()
                .find(|entry| entry.item.production_key == item)
                .ok_or_else(|| format!("{creature} lacks admitted {item}"))?;
            assert_eq!(
                (entry.probability_ppm, entry.min_count, entry.max_count),
                (Some(chance), 1, 1)
            );
        }
        Ok(())
    }

    #[test]
    fn arbaziloth_loads_admitted_crystal_loot() -> Result<(), Box<dyn std::error::Error>> {
        let (creatures, _) = documents()?;
        let loot = LootTablesSection::decode(
            include_bytes!("../../../../../tools/content-schema/native-gameplay/loot-tables.json"),
            &creatures,
        )?;
        let table = CreatureRewardTable::from_pin(&creatures, Some(&loot));
        let row = table
            .row("oteryn:creature.arbaziloth")
            .map_err(NoSettlementReason::as_str)?;
        assert_eq!(
            row.corpse_item
                .as_ref()
                .map(|item| item.production_key.as_str()),
            Some("oteryn:item.tibia.i50029"),
        );
        let donor: Vec<(u32, u32, u32)> = serde_json::from_slice(include_bytes!(
            "../../../../../tools/content-schema/monster-authoring/samples/arbaziloth-crystal-201189f8-loot.json"
        ))?;
        let expected: Vec<_> = donor
            .into_iter()
            .map(|(id, chance, max)| {
                (
                    format!("oteryn:item.tibia.i{id}"),
                    Some(chance * 10),
                    1,
                    max,
                )
            })
            .collect();
        let actual: Vec<_> = row
            .loot_table
            .entries
            .iter()
            .map(|entry| {
                (
                    entry.item.production_key.clone(),
                    entry.probability_ppm,
                    entry.min_count,
                    entry.max_count,
                )
            })
            .collect();
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn omruc_settles_mini_mummy_at_authored_probability() -> Result<(), Box<dyn std::error::Error>>
    {
        let (creatures, loot) = documents()?;
        let row = CreatureRewardTable::from_pin(&creatures, Some(&loot))
            .row("oteryn:creature.omruc")
            .map_err(NoSettlementReason::as_str)?;
        assert!(
            row.loot_table.entries.iter().any(|entry| {
                entry.item.production_key == "oteryn:item.tibia.i10290"
                    && entry.probability_ppm == Some(11429)
            }),
            "Omruc's 11,429 ppm mini mummy is missing"
        );
        Ok(())
    }

    #[test]
    fn merlkin_settles_ape_fur_for_myra() -> Result<(), Box<dyn std::error::Error>> {
        let (creatures, loot) = documents()?;
        let row = CreatureRewardTable::from_pin(&creatures, Some(&loot))
            .row("oteryn:creature.merlkin")
            .map_err(NoSettlementReason::as_str)?;
        assert!(
            row.loot_table.entries.iter().any(|entry| {
                entry.item.production_key == "oteryn:item.tibia.i5883"
                    && entry.probability_ppm == Some(10000)
            }),
            "Merlkin's 10,000 ppm ape fur is missing"
        );
        Ok(())
    }

    #[test]
    fn source_quest_loot_settles_the_backpack_and_hunter_music_sheets()
    -> Result<(), Box<dyn std::error::Error>> {
        let (creatures, _) = documents()?;
        let loot = LootTablesSection::decode(
            include_bytes!("../../../../../tools/content-schema/native-gameplay/loot-tables.json"),
            &creatures,
        )?;
        let table = CreatureRewardTable::from_pin(&creatures, Some(&loot));
        for (creature, corpse, item, chance) in [
            ("orc_sambackpack", 5966, 3244, 1_000_000),
            ("novice_of_the_cult", 18186, 6087, 9_700),
            ("acolyte_of_the_cult", 18038, 6088, 4_800),
            ("adept_of_the_cult", 18030, 6089, 7_000),
            ("enlightened_of_the_cult", 18110, 6090, 4_900),
        ] {
            let row = table
                .row(&format!("oteryn:creature.{creature}"))
                .map_err(|error| format!("{creature}: {}", error.as_str()))?;
            let corpse_key = format!("oteryn:item.tibia.i{corpse}");
            assert_eq!(
                row.corpse_item
                    .as_ref()
                    .map(|item| item.production_key.as_str()),
                Some(corpse_key.as_str()),
                "{creature} source corpse"
            );
            let key = format!("oteryn:item.tibia.i{item}");
            let entry = row
                .loot_table
                .entries
                .iter()
                .find(|entry| entry.item.production_key == key)
                .ok_or_else(|| format!("{creature} drops no {key}"))?;
            assert_eq!(
                (entry.min_count, entry.max_count, entry.probability_ppm),
                (1, 1, Some(chance)),
                "{creature} source loot"
            );
        }
        Ok(())
    }

    #[test]
    fn unmaterializable_protected_loot_refuses_settlement() -> Result<(), Box<dyn std::error::Error>>
    {
        let (creatures, mut loot) = documents()?;
        loot.items.retain(|facts| {
            !matches!(
                facts.item.key.as_str(),
                "oteryn:item.tibia.i10290" | "oteryn:item.tibia.i5883"
            )
        });
        let table = CreatureRewardTable::from_pin(&creatures, Some(&loot));
        for key in ["oteryn:creature.omruc", "oteryn:creature.merlkin"] {
            assert_eq!(
                table.row(key),
                Err(NoSettlementReason::LootItemInadmissible),
                "{key}"
            );
        }
        Ok(())
    }
}

#[cfg(test)]
pub(super) fn expected_zamulosh_private_loot(
    source: &SectionLootTable,
    items: &[SectionItemFacts],
) -> Result<Vec<SectionLootEntry>, &'static str> {
    assert_eq!(
        (
            source.identity.family.as_str(),
            source.identity.key.as_str(),
            source.identity.revision.as_str(),
        ),
        ("Loot", "oteryn:loot.creature.zamulosh", "definition-r1")
    );
    let wall = SectionRef {
        family: "Item".into(),
        key: "oteryn:item.tibia.i22555".into(),
        revision: "definition-r1".into(),
    };
    let (scenery, carried): (Vec<_>, Vec<_>) = source
        .entries
        .iter()
        .cloned()
        .partition(|entry| entry.item == wall);
    let [entry] = scenery.as_slice() else {
        return Err("Zamulosh must retain exactly one source Stone Wall roll");
    };
    assert_eq!(
        (entry.min_count, entry.max_count, entry.probability_ppm),
        (1, 1, Some(5_000))
    );
    assert!(entry.children.is_empty());
    assert!(
        !items
            .iter()
            .find(|facts| facts.item == wall)
            .ok_or("Zamulosh Stone Wall custody facts")?
            .materializable
    );
    Ok(carried)
}

#[cfg(test)]
mod protected_loot_sweep {
    use super::*;

    #[test]
    fn settleable_boss_and_quest_loot_keeps_every_protected_entry()
    -> Result<(), Box<dyn std::error::Error>> {
        fn has_pruned_protected(
            authored: &[SectionLootEntry],
            lowered: &[LootTableEntry],
            boss: bool,
            missing: &mut Vec<String>,
        ) {
            // The committed quest corpus is the source of protection.
            for entry in authored {
                let row = lowered.iter().find(|row| {
                    row.item == entry.item.lower()
                        && row.min_count == entry.min_count
                        && row.max_count == entry.max_count
                        && row.probability_ppm == entry.probability_ppm
                });
                if row.is_none() && (boss || quest_loot_items().contains(entry.item.key.as_str())) {
                    missing.push(entry.item.key.clone());
                }
                has_pruned_protected(
                    &entry.children,
                    row.map_or(&[], |row| row.details.children.as_slice()),
                    boss,
                    missing,
                );
            }
        }

        let (creatures, loot) = super::protected_loot_tests::documents()?;
        let profile_items = serde_json::from_str(include_str!(
            "../../../../../content/items/definitions/spell-native-profiles.json"
        ))?;
        let table = CreatureRewardTable::from_pin_with_native_corpses(
            &creatures,
            Some(&loot),
            Some(&profile_items),
        );
        let bosses: HashSet<_> = creatures
            .records
            .iter()
            .filter_map(|record| {
                let ProjectV2AuthoringProfileData::Creature(profile) = &record.profile.data else {
                    return None;
                };
                is_boss(profile).then_some(&record.profile.target.key)
            })
            .collect();
        // Every row known to have pruned a protected drop must still settle.
        for creature in [
            "ashmunrah",
            "bane_of_light",
            "behemoth",
            "bonebeast",
            "dipthrah",
            "ghost",
            "goldhanded_cultist",
            "goldhanded_cultist_bride",
            "horestis",
            "lich",
            "mahrdis",
            "merlkin",
            "morguthis",
            "omruc",
            "rahemos",
            "ribstride",
            "thalas",
            "vashresamun",
        ] {
            let key = format!("oteryn:creature.{creature}");
            assert!(table.row(&key).is_ok(), "{key} must still settle");
        }
        let authored: HashMap<_, _> = loot.tables.iter().map(|t| (&t.identity, t)).collect();
        let mut missing = Vec::new();
        for binding in &loot.creature_loot {
            let key = &binding.creature.key;
            let (Ok(row), Some(reference)) = (table.row(key), &binding.loot) else {
                continue;
            };
            let source = authored[reference];
            let carried;
            let expected = if key == "oteryn:creature.zamulosh" {
                carried = expected_zamulosh_private_loot(source, &loot.items)?;
                carried.as_slice()
            } else {
                source.entries.as_slice()
            };
            let mut pruned = Vec::new();
            has_pruned_protected(
                expected,
                &row.loot_table.entries,
                bosses.contains(key),
                &mut pruned,
            );
            if !pruned.is_empty() {
                missing.push((key, pruned));
            }
        }
        assert!(missing.is_empty(), "protected loot pruned: {missing:?}");
        Ok(())
    }
}
