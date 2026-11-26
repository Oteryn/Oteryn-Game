//! The existing USE path consumes charges through the same durable command owner.
//! Outer None delegates an ordinary item; inner None preserves an unknown timed commit.
use super::timed_item_content::facts;
use super::{ComposedFreshAdmission, connection::UseCommand, item_view::ItemKey};
use crate::foundation::ExactActorRef;
use oteryn_protocol_oteryn::timed_item::*;
use sha2::{Digest, Sha256};
fn charge_use_facts(
    native: &crate::content::native_gameplay::NativeGameplayState,
    definition: &crate::durability::item_mint::TypedDefinitionRef,
) -> Option<()> {
    let tibia_id = definition
        .production_key
        .strip_prefix("oteryn:item.tibia.i")?
        .parse()
        .ok()?;
    if oteryn_gameplay_items::remaining_consumables::is_dispenser(tibia_id) {
        return None;
    }
    facts(native, definition)?.0.full.charges()?;
    Some(())
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn use_timed_item(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        source: &super::item_ref_admission::UseSourceHint,
    ) -> Option<Option<super::connection::UseOutcome>> {
        let Some(item @ ItemKey::Instance(id)) = source.key else {
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
        let reference = crate::foundation::CommandRef::new(
            command.game_session_id,
            crate::foundation::CommandId::new(command.command_id).ok()?,
        );
        let historical = match self
            .probe_plain_use_receipt(
                crate::durability::PlainUseReceipt::Timed(transaction),
                reference,
            )
            .await
        {
            Ok(present) => present,
            Err(_) => return Some(Some(super::use_receipt_refused(command, "timed_existence"))),
        };
        let replay = if historical {
            let budget = super::AdmissionRetryBudget::for_command(self.root);
            match budget
                .retry_command_guard(async || {
                    match self
                        .root
                        .read_timed_command(
                            self.character,
                            command.game_session_id,
                            transaction,
                            binding,
                        )
                        .await
                    {
                        Err(crate::durability::item_timed_state::TimedWriteError::Unavailable(
                            error,
                        )) => Err(error),
                        result => Ok(result),
                    }
                })
                .await
            {
                Ok(Ok(Some(replay))) => Some(replay),
                _ => return Some(Some(super::use_receipt_refused(command, "timed_replay"))),
            }
        } else {
            None
        };
        if replay.is_none() {
            match source
                .candidate(self, command, "timed_candidate", |definition| {
                    charge_use_facts(native, definition).is_some()
                })
                .await
            {
                Ok(true) => {}
                Ok(false) => return None,
                Err(refusal) => return Some(Some(refusal.into())),
            }
            let backpack = match self.read_use_backpack(fence).await {
                Ok(backpack) => backpack?,
                Err(_) => {
                    return Some(Some(super::use_receipt_refused(command, "timed_inventory")));
                }
            };
            let definition = &backpack
                .entries
                .iter()
                .find(|entry| entry.item.item_instance_id == id)?
                .item
                .definition;
            charge_use_facts(native, definition)?;
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
            reward_granted: None,
            gate_message_id: None,
            potion_utterance: None,
            refusal_detail: None,
        }))
    }
}
