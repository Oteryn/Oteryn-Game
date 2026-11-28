//! Bounded inventory evidence for the dev-client qualification harness.
use super::*;

#[derive(Clone, Debug)]
pub struct InventoryCapture {
    pub session_id: [u8; 16],
    pub snapshot_sequence: u64,
    pub snapshot_revision: u64,
    pub snapshot: Vec<u8>,
    delta: Option<(u64, Vec<u8>)>,
}

impl InventoryCapture {
    pub(super) fn new(
        body: &[u8],
        session_id: [u8; 16],
        snapshot_sequence: u64,
    ) -> Result<Self, SessionError> {
        let inventory = decode_snapshot_body(body)?.into_iter().find(|domain| {
            domain.domain_id == STATE_DOMAIN_CHARACTER_INVENTORY
                && domain.snapshot_type == item_view::SNAPSHOT_TYPE_CHARACTER_INVENTORY_V1
        });
        let (snapshot_revision, snapshot) = inventory.map_or((0, Vec::new()), |domain| {
            (domain.revision, domain.payload.to_vec())
        });
        Ok(Self {
            session_id,
            snapshot_sequence,
            snapshot_revision,
            snapshot,
            delta: None,
        })
    }

    pub(super) fn record_delta(&mut self, sequence: u64, wire_payload: &[u8]) {
        self.delta = Some((sequence, wire_payload.to_vec()));
    }

    pub fn delta(&self, sequence: u64) -> Option<&[u8]> {
        self.delta
            .as_ref()
            .filter(|(captured, _)| *captured == sequence)
            .map(|(_, bytes)| bytes.as_slice())
    }

    /// Replay the original domain payloads with the negotiated inventory layout and revision.
    pub fn replay(
        &self,
        snapshot: &[u8],
        delta: &[u8],
        sequence: u64,
        selected: &[u32],
    ) -> Result<(CharacterInventory, CharacterInventory), SessionError> {
        if snapshot.is_empty()
            || snapshot != self.snapshot
            || self.delta(sequence) != Some(delta)
            || sequence <= self.snapshot_sequence
            || !selected.contains(&item_view::CAPABILITY_ITEM_VIEW_MOVE_V1)
        {
            return Err(FoundationProtocolError::MalformedEnvelope.into());
        }
        let delta = decode_state_delta(delta)?;
        if delta.domain_id != STATE_DOMAIN_CHARACTER_INVENTORY
            || delta.new_revision <= self.snapshot_revision
        {
            return Err(FoundationProtocolError::MalformedEnvelope.into());
        }
        framing::check_delta(
            &delta,
            item_view::DELTA_TYPE_CHARACTER_INVENTORY_V1,
            self.snapshot_revision,
        )?;
        let decode = |payload| {
            item_view::decode_character_inventory_with_appearances(
                payload,
                selected.contains(&item_view::CAPABILITY_ITEM_EQUIP_DROP_V1),
                selected.contains(&item_view::CAPABILITY_ITEM_APPEARANCE_V1),
            )
            .map_err(SessionError::ItemView)
        };
        Ok((decode(snapshot)?, decode(delta.payload)?))
    }
}

impl<S: SessionStream> Session<S> {
    pub fn inventory_capture(&self) -> &InventoryCapture {
        &self.inventory_capture
    }
}
