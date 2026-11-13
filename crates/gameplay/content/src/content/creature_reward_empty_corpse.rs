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
