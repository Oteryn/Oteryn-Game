//! Optional capability command exchange through the existing command sequencing owner.
use super::*;

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
        Ok(result)
    }
}
