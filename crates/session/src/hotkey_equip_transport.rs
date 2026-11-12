//! Typed hotkey command exchange through the session sequencing and failure boundary.
use super::*;

impl<S: SessionStream> Session<S> {
    pub async fn equip_item_definition(
        &mut self,
        definition: std::num::NonZeroU32,
    ) -> Result<ItemMoveOutcomeResult, SessionError> {
        let payload = hotkey_equip::TieredRequest {
            request: hotkey_equip::Request { definition },
            // This definition-only helper targets an unforged copy on tiered sessions.
            upgrade_tier: self
                .is_selected(hotkey_equip::CAPABILITY_HOTKEY_EQUIP_UPGRADE_TIER_V1)
                .then_some(0),
        }
        .encode()
        .ok_or(SessionError::Protocol(
            FoundationProtocolError::MalformedFrame,
        ))?;
        let result = self
            .gated_command(
                hotkey_equip::CAPABILITY_HOTKEY_EQUIP_V1,
                hotkey_equip::COMMAND_TYPE_HOTKEY_EQUIP,
                &payload,
            )
            .await?;
        let decoded = if result.status != CommandStatus::Accepted {
            Err(SessionError::InconsistentCommandResult {
                command_id: result.command_id,
                status: result.status,
            })
        } else {
            hotkey_equip::decode_reply(&result.payload)
                .map_err(SessionError::ItemView)
                .map(|outcome| ItemMoveOutcomeResult {
                    command_id: result.command_id,
                    status: result.status,
                    outcome,
                    result_server_sequence: result.server_sequence,
                })
        };
        self.poison_on_error(decoded)
    }
}
