//! Native trade and item-picker pushes use the existing capability-gated revisions.
use super::*;

impl<S: SessionStream> Session<S> {
    /// Latest authoritative trade projection, including unsolicited partner actions.
    pub fn player_trade_state(&self) -> Option<player_trade::State> {
        self.gated_snapshot(player_trade::STATE_DOMAIN_PLAYER_TRADE)
            .and_then(|snapshot| player_trade::decode_state(&snapshot.payload).ok())
    }

    pub(crate) fn record_player_trade_delta(
        &mut self,
        delta: &GatedDelta,
    ) -> Result<(), SessionError> {
        let state = player_trade::decode_state(&delta.payload)
            .map_err(|_| SessionError::SessionUnusable)?;
        if let Some(snapshot) = self
            .gated_snapshots
            .iter_mut()
            .find(|snapshot| snapshot.domain_id == player_trade::STATE_DOMAIN_PLAYER_TRADE)
        {
            snapshot.revision = delta.new_revision;
            snapshot.payload.clone_from(&delta.payload);
        }
        self.queue_event(SessionEvent::PlayerTrade(AppliedDelta {
            server_sequence: delta.server_sequence,
            base_revision: delta.base_revision,
            new_revision: delta.new_revision,
            value: state,
        }))
    }

    pub(crate) fn apply_trade_delta(
        &mut self,
        sequence: u64,
        delta: oteryn_protocol_oteryn::StateDeltaView<'_>,
    ) -> Result<SessionEvent, SessionError> {
        let domain_id = delta.domain_id;
        if !self.is_selected(60) {
            return Err(SessionError::UnselectedDomain { domain_id });
        }
        let revision = self
            .gated_revisions
            .iter()
            .find(|(id, _)| *id == domain_id)
            .map_or(0, |(_, revision)| *revision);
        check_delta(&delta, 1, revision)?;
        let applied = |value| AppliedDelta {
            server_sequence: sequence,
            base_revision: delta.base_revision,
            new_revision: delta.new_revision,
            value,
        };
        let event = SessionEvent::PlayerTrade(applied(
            player_trade::decode_state(delta.payload).map_err(|_| SessionError::SessionUnusable)?,
        ));
        match self
            .gated_revisions
            .iter_mut()
            .find(|(id, _)| *id == domain_id)
        {
            Some((_, revision)) => *revision = delta.new_revision,
            None => self.gated_revisions.push((domain_id, delta.new_revision)),
        }
        if let Some(snapshot) = self
            .gated_snapshots
            .iter_mut()
            .find(|snapshot| snapshot.domain_id == domain_id)
        {
            snapshot.revision = delta.new_revision;
            snapshot.payload = delta.payload.to_vec();
        }
        Ok(event)
    }
}
