//! Optional capability command exchange through the existing command sequencing owner.
use super::*;

#[path = "features/0011-world-interaction/protocol.rs"]
pub mod world_context;

#[path = "features/0064-target-move/protocol.rs"]
mod target_move;

impl<S: SessionStream> Session<S> {
    /// Sends a command type owned by `capability` and returns its raw `CommandResult`. Refused
    /// before anything is sent when the capability is not selected or does not own the type.
    pub async fn gated_command(
        &mut self,
        capability: u32,
        command_type: u32,
        payload: &[u8],
    ) -> Result<GatedCommandResult, SessionError> {
        self.ensure_usable()?;
        let parameter_cast = capability == actor_spell_v2::CAPABILITY_ACTOR_SPELL_PARAMETERS_V2
            && command_type == actor_spell_v2::COMMAND_TYPE_WORLD_ACTOR_SPELL_CAST_PARAMETERS_V2;
        let owned = parameter_cast
            || gated_routes()
                .any(|(id, commands, _)| id == capability && commands.contains(&command_type));
        if !owned || !self.is_selected(capability) {
            return Err(SessionError::CapabilityNotSelected { capability });
        }
        let outcome = self.send_and_read_result(command_type, payload).await;
        let result = self.poison_on_error(outcome)?;
        let result = GatedCommandResult {
            command_id: result.command_id,
            status: result.status,
            server_sequence: result.server_sequence,
            payload: result.payload,
        };
        self.poison_on_error(appearance_state::validate_result(command_type, &result))?;
        if command_type == property_services::house_transfer::COMMAND_TYPE_HOUSE_TRANSFER {
            let handoff = self.house_handoff(payload, &result).await;
            self.poison_on_error(handoff)?;
        }
        if parameter_cast {
            let outcome = self.publish_parameter_spell_result(&result);
            self.poison_on_error(outcome)?;
        }
        Ok(result)
    }
    fn publish_parameter_spell_result(
        &mut self,
        reply: &GatedCommandResult,
    ) -> Result<(), SessionError> {
        use actor_spell_v2::ParameterSpellCastResult;

        let result = if reply.status == CommandStatus::Rejected && reply.payload.is_empty() {
            ParameterSpellCastResult {
                disposition: SpellCastDisposition::Rejected,
                feedback: None,
                editor: None,
            }
        } else {
            actor_spell_v2::decode_parameter_spell_cast_result(&reply.payload)?
        };
        check_status_pairing(
            reply.command_id,
            reply.status,
            result.disposition == SpellCastDisposition::Rejected,
        )?;
        self.queue_event(SessionEvent::ParameterSpellCast {
            result,
            position: self.world_spatial().actor_position,
        })
    }
}
