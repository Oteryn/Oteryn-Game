//! Typed depot request/reply through the existing sequenced session exchange.
use super::*;
use oteryn_protocol_oteryn::depot::{
    CAPABILITY_DEPOT_V1, COMMAND_TYPE_DEPOT_INTENT, decode_depot_reply, encode_depot_intent,
};

impl<S: SessionStream> Session<S> {
    /// Resolve a nearby locker, browse a box, or move one whole item/stack.
    /// Capability absence and ordinary command rejection leave play usable.
    pub async fn depot(&mut self, intent: DepotIntent) -> Result<Option<DepotReply>, SessionError> {
        self.ensure_usable()?;
        if !self.is_selected(CAPABILITY_DEPOT_V1) {
            return Ok(None);
        }
        let payload = encode_depot_intent(intent)
            .map_err(|_| SessionError::Protocol(FoundationProtocolError::MalformedEnvelope))?;
        let revision = self.inventory_revision;
        let result = self
            .gated_command(CAPABILITY_DEPOT_V1, COMMAND_TYPE_DEPOT_INTENT, &payload)
            .await?;
        if result.status == CommandStatus::Rejected && result.payload.is_empty() {
            return Ok(None);
        }
        if result.status != CommandStatus::Accepted {
            return self.poison_on_error(Err(SessionError::InconsistentCommandResult {
                command_id: result.command_id,
                status: result.status,
            }));
        }
        let reply = decode_depot_reply(&result.payload)
            .map_err(|_| SessionError::Protocol(FoundationProtocolError::MalformedEnvelope));
        let reply = self.poison_on_error(reply)?;
        if reply.outcome == DepotOutcome::Moved {
            // The result precedes the inventory delta; wait before accepting another input.
            let until = tokio::time::Instant::now() + self.deadline;
            while self.inventory_revision == revision {
                if tokio::time::Instant::now() >= until {
                    return self.poison_on_error(Err(SessionError::Timeout("depot inventory")));
                }
                self.service_liveness(Duration::from_millis(20)).await?;
            }
        }
        Ok(Some(reply))
    }
}
