//! Native-only source admission; no reconstructed Draft or name-based callback grant.
use super::*;
use crate::content::native_gameplay::CreatureProfilesDocument;
pub(super) struct Plan {
    pub(super) ability: Ref,
    pub(super) presentation: Box<crate::content::ProjectV2AbilityDetails>,
}
pub(super) fn plan(
    document: &CreatureProfilesDocument,
    caster: &Ref,
) -> Result<Option<Plan>, AttackError> {
    if caster.family != Family::Creature
        || caster.key != CREATURE
        || caster.revision != "definition-r1"
    {
        return Ok(None);
    }
    use crate::content::native_monster_callbacks::{
        NativeCallbackList, qualify_native_callback_document,
    };
    let Some(entry) =
        qualify_native_callback_document(document, caster, 0, NativeCallbackList::Attack)
            .map_err(|_| AttackError::InvalidSource)?
    else {
        return Ok(None);
    };
    if entry.ability.key != PARENT
        || entry.ability.revision != "definition-r1"
        || entry.encounter.key != "oteryn:encounter.crystal_smelly_cheese"
    {
        return Err(AttackError::InvalidSource);
    }
    // The repository callback-witness producer compares EVERY profile in this
    // reviewed fixture with actual canonical declarations before emitting the
    // full root checked above. Child pulse bodies are not reconstructed Drafts
    // or invented native Ability registrations.
    static PRESENTATION: std::sync::OnceLock<Option<Box<crate::content::ProjectV2AbilityDetails>>> =
        std::sync::OnceLock::new();
    let presentation = PRESENTATION
        .get_or_init(|| {
            let witness: Witness = serde_json::from_str(include_str!(
                "../../../../../../apps/game-server/src/smelly_encounter_source.json"
            ))
            .ok()?;
            let Data::Ability(profile) = witness
                .profiles
                .get("oteryn:ability.creature.smelly_cheese.berserk_pulse_2")?
            else {
                return None;
            };
            profile.details.clone()
        })
        .as_ref()
        .ok_or(AttackError::InvalidSource)?
        .clone();
    Ok(Some(Plan {
        ability: entry.ability,
        presentation,
    }))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_smelly_document_retains_actual_callback_and_reviewed_pulse_bodies()
    -> Result<(), Box<dyn std::error::Error>> {
        let raw = std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../content/creatures/definitions/spell-native-profiles.json"
        ))?;
        let mut document: CreatureProfilesDocument = serde_json::from_slice(&raw)?;
        let caster = Ref {
            family: Family::Creature,
            key: CREATURE.into(),
            revision: "definition-r1".into(),
        };
        assert!(
            plan(&document, &caster)
                .map_err(|error| format!("{error:?}"))?
                .is_some(),
            "real registered Smelly Creature/Behavior/parentAbility/root must qualify its native two-pulse source"
        );
        document.source_definitions_sha256 = Some("00".repeat(32));
        assert!(
            plan(&document, &caster).is_err(),
            "same named profiles under a different canonical root cannot qualify"
        );
        Ok(())
    }
}
