use super::super::ComposedFreshAdmission;
use crate::domain::soulpit::{soul_core_for_monster, soulpit_entry_monster_name};
use crate::durability::character_progression::CurrentCharacterGameplayFence;
use crate::durability::soulpit::{SoulpitStepOutcome, SoulpitStepRequest, SoulpitWriteError};
use oteryn_protocol_oteryn::zones_nologout::ServerMessage;

impl ComposedFreshAdmission<'_, '_, '_> {
    /// A committed completion proves every survivor fence, including the
    /// unchanged survivor intent returned by a durable replay.
    pub(in crate::gameplay_transport) async fn commit_soulpit_progress(
        &self,
        species: &str,
        request: SoulpitStepRequest,
    ) -> Result<SoulpitStepOutcome, SoulpitWriteError> {
        let occurrence = request.occurrence;
        let survivors = request.survivors.clone();
        let outcome = self
            .root
            .advance_soulpit(self.character, self.holder, request)
            .await?;
        if matches!(
            outcome.effect,
            crate::domain::soulpit::EncounterEffect::Completed { .. }
        ) {
            self.publish_soulpit_completion(species, occurrence, &survivors);
        }
        Ok(outcome)
    }

    fn publish_soulpit_completion(
        &self,
        species: &str,
        occurrence: [u8; 16],
        survivors: &[CurrentCharacterGameplayFence],
    ) {
        let Some(facts) = self
            .active_generation
            .and_then(crate::content::ActiveGeneration::native_gameplay)
            .and_then(|native| native.reward_table().encounter_facts(species))
        else {
            return;
        };
        // Crystal 201189f, soulpit_fight.lua:86-88,100-106 uses the Core's
        // source spelling and reverses named variants before interpolating it.
        let core_name = soul_core_for_monster(&facts.display_name)
            .and_then(soulpit_entry_monster_name)
            .filter(|name| name.eq_ignore_ascii_case(&facts.display_name));
        let name = core_name.as_deref().unwrap_or(&facts.display_name);
        let message = ServerMessage::Advance(format!(
            "You have defeated the core of the {name} soul and unlocked its animus mastery!"
        ));
        for survivor in survivors {
            self.states.server_messages.send_soulpit_completion(
                survivor.game_session_id,
                survivor.connection_generation.get(),
                occurrence,
                &message,
            );
        }
    }
}
