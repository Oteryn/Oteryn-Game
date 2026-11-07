//! House-only item visual events; command results and map mutations retain their owners.
use super::*;
impl State {
    pub(super) async fn item_effects<S: AsyncWrite + Unpin, A: FreshAdmissionAuthority>(
        &mut self,
        stream: &mut S,
        authority: &A,
        admitted: &mut AdmittedSession,
    ) -> Result<(), ServeEnd> {
        use oteryn_protocol_oteryn::presentation as wire;
        if !admitted
            .continuity
            .selected_capabilities
            .contains(wire::CAPABILITY_ITEM_EFFECTS_V1)
        {
            return Ok(());
        }
        let (cursor, events) = authority
            .observe_house_item_effects(*admitted, self.item_effect_cursor)
            .await
            .ok_or(ServeEnd::Disconnected)?;
        if events.is_empty() {
            self.item_effect_cursor = cursor;
            return Ok(());
        }
        let payload =
            wire::encode_item_effect_batch(&events).map_err(|_| ServeEnd::Disconnected)?;
        let revision = self
            .item_effect_revision
            .checked_add(1)
            .ok_or(ServeEnd::Disconnected)?;
        let sequence = admitted
            .continuity
            .server_sequence
            .checked_add(1)
            .ok_or(ServeEnd::Disconnected)?;
        let frame = encode_state_delta(
            admitted.continuity.connection_generation,
            sequence,
            wire::STATE_DOMAIN_ITEM_EFFECTS,
            self.item_effect_revision,
            revision,
            1,
            &payload,
        )
        .map_err(|_| ServeEnd::Disconnected)?;
        write_frame(stream, &frame)
            .await
            .map_err(|_| ServeEnd::Disconnected)?;
        self.item_effect_revision = revision;
        self.item_effect_cursor = cursor;
        admitted.continuity.server_sequence = sequence;
        Ok(())
    }
}
