//! Existing item-move selection, stack-count and refusal-detail client adapter.
use crate::*;
pub use oteryn_protocol_oteryn::item_view::{EquipmentSlot, EquippedItem, WorldTilePosition};
use oteryn_protocol_oteryn::{item_view as wire, refusal_detail};
pub use refusal_detail::RefusalDetail;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MoveRequest {
    pub source: ItemHandle,
    pub destination: ItemMoveDestination,
    pub count: Option<u32>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MoveResponse {
    pub outcome: ItemMoveOutcome,
    pub detail: Option<RefusalDetail>,
}

fn encode(request: MoveRequest, selected: &[u32]) -> Result<Vec<u8>, SessionError> {
    let mut payload = wire::encode_item_move_intent_for(
        &ItemMoveIntent {
            source: request.source,
            destination: request.destination,
        },
        selection(selected),
    )
    .map_err(SessionError::ItemView)?;
    if let Some(count) = request.count {
        if !selected.contains(&61) {
            return Err(SessionError::CapabilityNotSelected { capability: 61 });
        }
        if !(1..=100).contains(&count) {
            return Err(SessionError::ItemView(wire::ItemViewWireError::Malformed));
        }
        // Existing command-9 field 6; the protocol decoder enforces the same range.
        payload.extend([0x30, count as u8]);
    }
    Ok(payload)
}
fn selection(selected: &[u32]) -> wire::ItemMoveSelection {
    wire::ItemMoveSelection {
        equip_drop: selected.contains(&12),
        container_tree: selected.contains(&14),
        world_interaction: selected.contains(&11),
    }
}

impl<S: SessionStream> Session<S> {
    pub async fn move_item_counted(
        &mut self,
        request: MoveRequest,
    ) -> Result<MoveResponse, SessionError> {
        self.ensure_usable()?;
        self.ensure_item_view()?;
        let payload = encode(request, self.selected_capabilities())?;
        let result = self.exchange_counted_move(&payload).await;
        self.poison_on_error(result)
    }
    async fn exchange_counted_move(
        &mut self,
        payload: &[u8],
    ) -> Result<MoveResponse, SessionError> {
        let result = self
            .send_and_read_result(wire::COMMAND_TYPE_ITEM_MOVE_INTENT, payload)
            .await?;
        let (payload, detail) = self.item_result_payload(&result.payload)?;
        let outcome =
            wire::decode_item_move_result_for(payload, selection(self.selected_capabilities()))
                .map_err(SessionError::ItemView)?;
        check_status_pairing(
            result.command_id,
            result.status,
            outcome == ItemMoveOutcome::Rejected,
        )?;
        if outcome == ItemMoveOutcome::Moved && detail.is_some() {
            return Err(SessionError::ItemView(wire::ItemViewWireError::Malformed));
        }
        Ok(MoveResponse { outcome, detail })
    }
    pub(crate) fn item_result_payload<'a>(
        &self,
        payload: &'a [u8],
    ) -> Result<(&'a [u8], Option<RefusalDetail>), SessionError> {
        if self.is_selected(66) {
            refusal_detail::split_refusal_detail(payload).map_err(SessionError::ItemView)
        } else {
            Ok((payload, None))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn count_and_destinations_follow_negotiation() -> Result<(), SessionError> {
        let request = MoveRequest {
            source: std::num::NonZeroU64::MIN,
            destination: ItemMoveDestination::Container(std::num::NonZeroU64::MIN),
            count: Some(7),
        };
        assert!(encode(request, &[4, 6]).is_err());
        assert!(encode(request, &[4, 6, 12, 14]).is_err());
        let selected = &[4, 6, 12, 14, 61];
        let payload = encode(request, selected)?;
        let (intent, count) =
            wire::decode_item_move_intent_with_count_for(&payload, selection(selected))
                .map_err(SessionError::ItemView)?;
        assert_eq!(intent.destination, request.destination);
        assert_eq!(count, Some(7));
        for count in [0, 101, u32::MAX] {
            assert!(
                encode(
                    MoveRequest {
                        count: Some(count),
                        ..request
                    },
                    selected
                )
                .is_err()
            );
        }
        Ok(())
    }
}
