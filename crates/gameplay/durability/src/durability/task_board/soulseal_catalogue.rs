//! Server catalogue: current native Task Board facts intersect the qualified Soulpit roster.
use super::super::soulpit::SoulpitWriteError;
use oteryn_gameplay_content::content::native_gameplay::NativeGameplayState;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SoulsealFight {
    pub race: u16,
    pub species: String,
    pub name: String,
    pub stars: u8,
    pub cost: u16,
}
pub fn soulseal_fights(native: &NativeGameplayState) -> Vec<SoulsealFight> {
    super::catalogue::creatures(native)
        .into_iter()
        .enumerate()
        .filter_map(|(index, creature)| {
            let policy = native
                .policies()
                .policies()
                .iter()
                .find(|p| p.definition_key == creature.race)?;
            if policy.is_familiar
                || crate::domain::soulpit::source_monster(&policy.display_name).is_none()
            {
                return None;
            }
            let cost = crate::domain::soulpit::soulseal_cost(creature.bestiary_stars).ok()?;
            Some(SoulsealFight {
                race: (index + 1).try_into().ok()?,
                species: creature.race,
                name: policy.display_name.clone(),
                stars: creature.bestiary_stars,
                cost,
            })
        })
        .collect()
}
pub(crate) fn soulseal_fight(
    native: &NativeGameplayState,
    species: &str,
) -> Result<SoulsealFight, SoulpitWriteError> {
    soulseal_fights(native)
        .into_iter()
        .find(|f| f.species == species)
        .ok_or(SoulpitWriteError::AuthorityRejected)
}
