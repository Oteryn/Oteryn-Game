//! Legacy USE replay binding composed with the existing native retained-batch owner.
use super::super::actor_spell::NativeCastDispatch;
use super::super::actor_spell::replay_with_rune;
use super::{ComposedFreshAdmission, LegacyRuneUse};
use crate::foundation::{CharacterId, CommandRef, ExactActorRef};
use oteryn_protocol_oteryn::actor_spell::SpellCastDisposition;
use serde_json::Value;

impl ComposedFreshAdmission<'_, '_, '_> {
    /// Use the native retained batch before resolving a new source revision or handle.
    pub(in crate::gameplay_transport) async fn replay_legacy_rune(
        &self,
        actor: ExactActorRef,
        command: super::super::connection::UseCommand,
        original: &oteryn_protocol_oteryn::world_object::UseIntent,
    ) -> NativeCastDispatch {
        use oteryn_protocol_oteryn::actor_spell_item_v2::decode_item_spell_cast_intent;
        let Some(_lane) = self.spell_lane_permit().await else {
            return NativeCastDispatch::Pending;
        };
        let Some(fence) = command.item_fence else {
            return NativeCastDispatch::Outcome(
                super::super::actor_spell::SpellCastOutcome::rejected(),
            );
        };
        let Ok(character) = CharacterId::decode(fence.character_id.as_bytes()) else {
            return NativeCastDispatch::Outcome(
                super::super::actor_spell::SpellCastOutcome::rejected(),
            );
        };
        let Ok(id) = crate::foundation::CommandId::new(command.command_id) else {
            return NativeCastDispatch::Outcome(
                super::super::actor_spell::SpellCastOutcome::rejected(),
            );
        };
        let command_ref = CommandRef::new(command.game_session_id, id);
        let mut runtime = self.runtime.lock().await;
        let mut states = self.states.spell_states.lock().await;
        let rejected =
            || NativeCastDispatch::Outcome(super::super::actor_spell::SpellCastOutcome::rejected());
        if runtime
            .player_control_facts(actor, command.game_session_id)
            .is_err()
        {
            return rejected();
        }
        let batch = match runtime.retained_spell_batch(
            actor,
            character,
            fence.character_lease_generation,
            command_ref,
        ) {
            Ok(Some(batch)) => batch,
            Ok(None) => return NativeCastDispatch::NotApplicable,
            Err(_) => return rejected(),
        };
        let Ok(saved) = serde_json::from_slice::<Value>(&batch.binding) else {
            return rejected();
        };
        let Ok(bytes) = serde_json::from_value::<Vec<u8>>(saved["legacy_rune_cast"].clone()) else {
            return rejected();
        };
        let Ok(rune) = decode_item_spell_cast_intent(&bytes) else {
            return rejected();
        };
        let legacy = LegacyRuneUse {
            original: original.clone(),
            creature: None,
            fence,
        };
        match replay_with_rune(
            &mut runtime,
            &mut states,
            actor,
            character,
            fence.character_lease_generation,
            command_ref,
            &rune.intent,
            Some(&rune),
            Some(&legacy),
            None,
            &mut None,
        ) {
            Ok(Some(receipt)) => {
                self.record_spell_kills(&mut runtime, std::iter::once(&receipt))
                    .await;
                NativeCastDispatch::Outcome(super::super::actor_spell::SpellCastOutcome {
                    disposition: SpellCastDisposition::Cast,
                    vitals: super::super::actor_spell::observe_vitals(
                        &runtime,
                        &states,
                        actor,
                        command.game_session_id,
                    ),
                })
            }
            _ => rejected(),
        }
    }
}

/// Bind the decoded USE before the existing presentation owner seals the native batch.
pub(in crate::gameplay_transport) fn bind_batch(
    batch: &mut crate::spell::combat_batch::OwnerCombatBatch,
    legacy: &LegacyRuneUse,
    rune: &oteryn_protocol_oteryn::actor_spell_item_v2::ItemSpellCastIntent,
) -> Result<(), SpellCastDisposition> {
    let mut capture: Value =
        serde_json::from_slice(&batch.binding).map_err(|_| SpellCastDisposition::Rejected)?;
    capture["legacy_rune_use"] = serde_json::json!(
        legacy
            .binding()
            .map_err(|_| SpellCastDisposition::Rejected)?
    );
    capture["legacy_rune_cast"] = serde_json::json!(
        oteryn_protocol_oteryn::actor_spell_item_v2::encode_item_spell_cast_intent(rune)
            .map_err(|_| SpellCastDisposition::Rejected)?
    );
    batch.binding = serde_json::to_vec(&capture).map_err(|_| SpellCastDisposition::Rejected)?;
    Ok(())
}
