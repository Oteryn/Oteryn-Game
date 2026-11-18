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
    // The Wrath bosses are marked as reward bosses in Crystal, but their
    // standalone remains have no container or private loot. Keep the source
    // reward-boss metadata and qualify only the shared physical projection.
    let plain_profile = profile.details.as_ref().and_then(|details| {
        let corpse = details.corpse_item.as_ref()?;
        if !matches!(
            corpse.key.as_str(),
            "oteryn:item.tibia.i11361" | "oteryn:item.tibia.i11429"
        ) || !verified_source_corpses.contains(&corpse.key)
            || !super::source_plain_corpse_pair(
                creature,
                &SectionRef {
                    family: "Item".into(),
                    key: corpse.key.clone(),
                    revision: corpse.revision.clone(),
                },
            )
        {
            return None;
        }
        let mut physical = profile.clone();
        physical.details.as_mut()?.system_eligibility.reward_boss = false;
        Some(physical)
    });
    // No private table is supplied: this physical corpse always has zero contents.
    // The caller also requires the exact native loot binding before calling here.
    super::row(
        creature,
        plain_profile.as_ref().unwrap_or(profile),
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
