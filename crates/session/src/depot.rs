//! Typed depot request/reply through the existing sequenced session exchange.
use super::*;
use oteryn_protocol_oteryn::depot::{
    CAPABILITY_CROSS_TOWN_DEPOT_V1, CAPABILITY_DEPOT_V1, COMMAND_TYPE_DEPOT_INTENT,
    decode_depot_reply, encode_depot_request,
};

impl<S: SessionStream> Session<S> {
    /// Resolve a nearby locker, browse a box, or move one whole item/stack.
    /// Capability absence and ordinary command rejection leave play usable.
    pub async fn depot(&mut self, intent: DepotIntent) -> Result<Option<DepotReply>, SessionError> {
        self.depot_at(intent, None).await
    }

    /// [`Self::depot`] with Manual Sort Mode's box slot for a deposit. The slot is sent only
    /// when capability 510 was negotiated; without it the deposit takes the first slot.
    pub async fn depot_at(
        &mut self,
        intent: DepotIntent,
        slot: Option<u32>,
    ) -> Result<Option<DepotReply>, SessionError> {
        self.depot_request(DepotRequest::local(intent, slot)).await
    }

    /// An explicit cross-town request requires capability 627.
    pub async fn depot_request(
        &mut self,
        mut request: DepotRequest,
    ) -> Result<Option<DepotReply>, SessionError> {
        self.ensure_usable()?;
        if !self.is_selected(CAPABILITY_DEPOT_V1)
            || (request.cross_town && !self.is_selected(CAPABILITY_CROSS_TOWN_DEPOT_V1))
        {
            return Ok(None);
        }
        request.slot = request.slot.filter(|_| {
            self.is_selected(oteryn_protocol_oteryn::item_view::CAPABILITY_CONTAINER_MANUAL_SORT_V1)
        });
        let payload = encode_depot_request(request)
            .map_err(|_| SessionError::Protocol(FoundationProtocolError::MalformedEnvelope))?;
        let revision = self.inventory_revision;
        let container_revision = self.container_revision;
        let bag_revision = self.gated_snapshot(14).map(|s| s.revision);
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
            // A nested-bag deposit may change only domain 14, leaving inventory unchanged.
            // Wait for the authoritative item view update before accepting another input.
            let until = tokio::time::Instant::now() + self.deadline;
            while self.inventory_revision == revision
                && self.container_revision == container_revision
                && self.gated_snapshot(14).map(|s| s.revision) == bag_revision
            {
                if tokio::time::Instant::now() >= until {
                    return self.poison_on_error(Err(SessionError::Timeout("depot inventory")));
                }
                self.service_liveness(Duration::from_millis(20)).await?;
            }
        }
        Ok(Some(reply))
    }
}
