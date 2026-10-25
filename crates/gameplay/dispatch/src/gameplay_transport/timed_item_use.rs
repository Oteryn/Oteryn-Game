//! The existing USE path consumes charges through the same durable command owner.
//! Outer None delegates an ordinary item; inner None preserves an unknown timed commit.
use super::timed_item_content::facts;
use super::{ComposedFreshAdmission, connection::UseCommand, item_view::ItemKey};
use crate::foundation::ExactActorRef;
use oteryn_protocol_oteryn::timed_item::*;
use sha2::{Digest, Sha256};
impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn use_timed_item(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        item: ItemKey,
    ) -> Option<Option<super::connection::UseOutcome>> {
        let ItemKey::Instance(id) = item else {
            return None;
        };
        let native = self.active_generation?.native_gameplay()?;
        let fence = command.item_fence?;
        let transaction = super::npc_trade_command::derived(
            b"oteryn:timed-command:v1",
            command.game_session_id,
            command.command_id,
        );
        let binding = Sha256::new()
            .chain_update(b"oteryn:timed-standard-use:v1")
            .chain_update(id)
            .finalize()
            .into();
        let replay = self
            .root
            .read_timed_command(
                self.character,
                command.game_session_id,
                transaction,
                binding,
            )
            .await;
        if matches!(
            replay,
            Err(crate::durability::item_timed_state::TimedWriteError::Unavailable(_))
        ) {
            return Some(None);
        }
        if !matches!(replay, Ok(Some(_))) {
            let backpack = self
                .root
                .read_character_backpack(self.character, fence.character_id)
                .await
                .ok()??;
            let definition = &backpack
                .entries
                .iter()
                .find(|entry| entry.item.item_instance_id == id)?
                .item
                .definition;
            facts(native, definition)?.0.full.charges()?;
        }
        let intent = TimedItemIntent {
            action: TimedAction::Use,
            target: oteryn_protocol_oteryn::item_view::ItemHandle::new(1)?,
            expected_revision: 0,
            npc: None,
        };
        let payload = encode_timed_item_intent(intent).ok()?;
        let Some(bytes) = self
            .timed_command(actor, command, &payload, Some(item), true)
            .await
        else {
            return Some(None);
        };
        let result = decode_timed_item_result(&bytes).ok();
        Some(Some(super::connection::UseOutcome {
            disposition: if result.is_some_and(|result| result.disposition == TimedDisposition::Ok)
            {
                oteryn_protocol_oteryn::world_object::UseDisposition::Committed
            } else {
                oteryn_protocol_oteryn::world_object::UseDisposition::Rejected
            },
            committed: None,
            earned: super::connection::EarnedNotice::NoneEarned,
            gate_message_id: None,
            refusal_detail: None,
        }))
    }
}
