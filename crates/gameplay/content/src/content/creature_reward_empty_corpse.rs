//! Empty physical reward-boss corpses retain their source container capacity.
//! Private loot rows still use the existing 30-entry admission and contents bounds.
use super::{
    BestiaryRace, CreatureRewardRow, HashMap, HashSet, NoSettlementReason,
    ProjectV2CreatureAuthoring, SectionItemFacts, SectionLootTable, SectionRef,
};

pub(super) fn reward_boss_row(
    creature: &str,
    profile: &ProjectV2CreatureAuthoring,
    tables: &HashMap<&SectionRef, &SectionLootTable>,
    items: &HashMap<&SectionRef, &SectionItemFacts>,
    race: Option<BestiaryRace>,
    verified_source_corpses: &HashSet<String>,
) -> Result<CreatureRewardRow, NoSettlementReason> {
    if !profile
        .details
        .as_ref()
        .is_some_and(|details| details.system_eligibility.reward_boss)
    {
        return Err(NoSettlementReason::CorpseItemInadmissible);
    }
    // No private table is supplied: this physical corpse always has zero contents.
    // The caller also requires the exact native loot binding before calling here.
    super::row(
        creature,
        profile,
        None,
        tables,
        items,
        race,
        super::CorpseAdmission {
            maximum_corpse_capacity: u32::from(u16::MAX),
            verified_source_corpses,
        },
    )
}

// Both Father corpses are plain Ground objects. Their verified Item
// profiles qualify only the empty physical body used by ordinary USE.
impl super::CreatureRewardTable {
    pub(crate) fn with_father_corpses(
        mut self,
        creatures: &super::CreatureProfilesDocument,
        loot: Option<&super::LootTablesSection>,
        items: Option<&super::super::native_gameplay::ItemProfilesDocument>,
    ) -> Self {
        use super::super::native_gameplay::{NativeItemDestination, NativeItemStackClass};
        use super::super::{ProjectV2AuthoringProfileData, ReferenceItemField as F};
        use std::sync::Arc;

        let (Some(loot), Some(items)) = (loot, items) else {
            return self;
        };
        for (creature, corpse_key) in [
            ("oteryn:creature.glitterscale", "oteryn:item.tibia.i11589"),
            ("oteryn:creature.heoni", "oteryn:item.tibia.i11590"),
        ] {
            let Some(record) = creatures
                .records
                .iter()
                .find(|row| row.profile.target.key == creature)
            else {
                continue;
            };
            let ProjectV2AuthoringProfileData::Creature(profile) = &record.profile.data else {
                continue;
            };
            let Some(corpse) = profile
                .details
                .as_ref()
                .filter(|details| details.system_eligibility.reward_boss)
                .and_then(|details| details.corpse_item.as_ref())
                .filter(|corpse| corpse.key == corpse_key && corpse.revision == "definition-r1")
            else {
                continue;
            };
            let Some(item) = items
                .records
                .iter()
                .find(|item| item.production_definition.production_key == corpse_key)
            else {
                continue;
            };
            let admitted = item.production_definition.family == "Item"
                && item.production_definition.revision_ref == corpse.revision
                && item.admission.materializable
                && item.admission.stack_class == NativeItemStackClass::NonStackable
                && item.admission.legal_destinations == [NativeItemDestination::Ground]
                && item.semantics.container == F::NotApplicable
                && matches!(&item.semantics.physical, F::Known(physical)
                    if physical.movable == F::Known(false) && physical.pickupable == F::Known(false))
                && loot
                    .creature_loot
                    .iter()
                    .any(|binding| binding.creature.key == creature);
            if !admitted {
                continue;
            }
            let Ok(xp) = i64::try_from(profile.experience.unwrap_or(0)) else {
                continue;
            };
            let item = SectionRef {
                family: "Item".into(),
                key: corpse.key.clone(),
                revision: corpse.revision.clone(),
            }
            .lower();
            self.reward_boss_corpses.insert(
                creature.into(),
                Arc::new(CreatureRewardRow {
                    xp_amount: super::ExactI64::new(xp),
                    corpse_item: Some(item.clone()),
                    loot_table_ref: Some(item),
                    loot_table: super::LootTableDefinition {
                        algorithm: super::LootSelectionAlgorithm::IndependentBernoulliPpm,
                        entries: Vec::new(),
                    },
                    race: super::canonical_races().get(creature).cloned(),
                    boss: self.bosses.get(creature).cloned(),
                }),
            );
        }
        self
    }
}
