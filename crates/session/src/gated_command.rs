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
        let owned = gated_routes()
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
        Ok(result)
    }
}
