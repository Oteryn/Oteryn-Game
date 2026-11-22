use super::super::ComposedFreshAdmission;
use super::runtime::{SoulpitActivityBinding, SoulpitRuntimeError};
use crate::domain::soulpit::{soul_core_for_monster, soulpit_entry_monster_name};
use crate::durability::soulpit::{SoulpitStepOutcome, SoulpitStepRequest, SoulpitWriteError};
use crate::foundation::GameSessionId;
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
            // The terminal pass retries any eligible recipient that was full.
            let _ = self.publish_soulpit_completion(
                species,
                occurrence,
                survivors
                    .iter()
                    .map(|f| (f.game_session_id, f.connection_generation.get())),
            );
        }
        Ok(outcome)
    }

    /// Read the committed completion even when the advancing writer lost its acknowledgement.
    /// Return is deferred while an eligible connection has no room for its notice.
    pub(in crate::gameplay_transport) async fn queue_committed_soulpit_completion(
        &self,
        binding: SoulpitActivityBinding,
    ) -> Result<(), SoulpitRuntimeError> {
        let stored = self
            .root
            .read_soulpit_encounter(
                self.character,
                self.holder,
                binding.handoff_id,
                binding.scope,
                binding.generation,
            )
            .await
            .map_err(super::owner_runtime::durable)?
            .ok_or(SoulpitRuntimeError::InvalidState)?;
        let completion = stored.completion.ok_or(SoulpitRuntimeError::InvalidState)?;
        self.publish_soulpit_completion(
            &stored.species,
            completion.occurrence,
            completion.recipients,
        )
    }

    fn publish_soulpit_completion(
        &self,
        species: &str,
        occurrence: [u8; 16],
        survivors: impl IntoIterator<Item = (GameSessionId, u64)>,
    ) -> Result<(), SoulpitRuntimeError> {
        let facts = self
            .active_generation
            .and_then(crate::content::ActiveGeneration::native_gameplay)
            .and_then(|native| native.reward_table().encounter_facts(species))
            .ok_or(SoulpitRuntimeError::MissingContent)?;
        // Crystal 201189f, soulpit_fight.lua:86-88,100-106 uses the Core's
        // source spelling and reverses named variants before interpolating it.
        let core_name = soul_core_for_monster(&facts.display_name)
            .and_then(soulpit_entry_monster_name)
            .filter(|name| name.eq_ignore_ascii_case(&facts.display_name));
        let name = core_name.as_deref().unwrap_or(&facts.display_name);
        let message = ServerMessage::Advance(format!(
            "You have defeated the core of the {name} soul and unlocked its animus mastery!"
        ));
        let mut pending = false;
        for (session, generation) in survivors {
            pending |= self
                .states
                .server_messages
                .send_soulpit_completion(session, generation, occurrence, &message)
                .is_err();
        }
        if pending {
            return Err(SoulpitRuntimeError::AdapterUnavailable);
        }
        Ok(())
    }
}
