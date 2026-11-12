//! Existing world-context codecs exposed at the client session boundary.
pub use oteryn_protocol_oteryn::actor_spell::SpellTargetPosition;
pub use oteryn_protocol_oteryn::{bed, item_look, world_object};

impl<S: crate::SessionStream> crate::Session<S> {
    /// World-context Use keeps its payload decoding local to the UI flow. Only exchange
    /// failures poison the connection; an unknown result payload still consumes its result.
    pub async fn world_context_use(
        &mut self,
        payload: &[u8],
    ) -> Result<crate::GatedCommandResult, crate::SessionError> {
        self.ensure_usable()?;
        let result = self
            .send_and_read_result(world_object::COMMAND_TYPE_USE_INTENT, payload)
            .await;
        let result = self.poison_on_error(result)?;
        Ok(crate::GatedCommandResult {
            command_id: result.command_id,
            status: result.status,
            server_sequence: result.server_sequence,
            payload: result.payload,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::capability_support::{CLIENT_SUPPORTED_CAPABILITIES, gated_routes};
    #[test]
    fn look_and_bed_are_advertised_and_routed() {
        for (capability, command) in [(28, 30), (11, 31)] {
            assert!(CLIENT_SUPPORTED_CAPABILITIES.contains(&capability));
            assert!(
                gated_routes()
                    .any(|(id, commands, _)| id == capability && commands.contains(&command))
            );
        }
    }
}
