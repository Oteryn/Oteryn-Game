//! Production MAIL text ingress using the fenced WRITE-0 owner; no client dialog here.
use crate::durability::item_text::{ItemTextWriteError, ItemTextWriteRequest};
use crate::durability::mail_text_policy::NativeGameplayMailTextPolicyExt;
use crate::foundation::{CommandId, CommandRef, ExactActorRef};
use crate::gameplay_transport::{
    ComposedFreshAdmission, connection::AdmittedSession, item_view::ItemKey,
};
use oteryn_protocol_oteryn::mail::MailOutcome;

pub(super) async fn execute(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    admitted: AdmittedSession,
    command: u64,
    item: Option<ItemKey>,
    text: String,
) -> Option<MailOutcome> {
    let (Some(fence), Some(ItemKey::Instance(item)), Ok(command)) =
        (admitted.item_fence, item, CommandId::new(command))
    else {
        return Some(MailOutcome::Rejected);
    };
    let command = CommandRef::new(admitted.game_session_id, command);
    match owner
        .root
        .read_write_once_source(owner.character, command, fence.character_id, item)
        .await
    {
        Ok(Some(definition)) => {
            let request = ItemTextWriteRequest {
                command,
                item_instance_id: item,
                definition,
                text,
            };
            let binding = request.freeze_receipt_binding(fence.character_id);
            return match owner
                .root
                .reconcile_item_text_rewrite(owner.character, &binding)
                .await
            {
                Ok(Some(_)) => Some(MailOutcome::TextWritten),
                Ok(None) => None,
                Err(error) => refusal(error),
            };
        }
        Ok(None) => {}
        Err(error) => return refusal(error),
    }
    let permit = owner.spell_lane_permit().await?;
    if !owner
        .states
        .player_trades
        .physical_service(admitted.game_session_id)
    {
        return Some(MailOutcome::Rejected);
    }
    let native = {
        let runtime = owner.runtime.lock().await;
        runtime
            .player_control_facts(actor, admitted.game_session_id)
            .ok()?;
        let states = owner.states.spell_states.lock().await;
        crate::gameplay_transport::equipment_move::native_for_move(
            &runtime,
            owner.active_generation,
            states.is_dead(actor),
            !states.spell_idle(&runtime, actor, admitted.game_session_id),
        )?
    };
    let inventory = owner
        .root
        .read_item_use_inventory(owner.character, owner.holder, fence)
        .await
        .ok()?;
    let definition = inventory
        .equipment
        .iter()
        .find(|entry| entry.item_instance_id == item)
        .or_else(|| {
            inventory
                .backpack
                .as_ref()?
                .entries
                .iter()
                .map(|entry| &entry.item)
                .find(|entry| entry.item_instance_id == item)
        })
        .or_else(|| {
            inventory
                .nested
                .iter()
                .map(|(_, entry)| &entry.item)
                .find(|entry| entry.item_instance_id == item)
        })
        .map(|entry| entry.definition.clone());
    let Some(definition) = definition else {
        return Some(MailOutcome::Rejected);
    };
    let policy = match native.mail_text_policy(&definition) {
        Ok(policy) => policy,
        Err(error) => return refusal(error),
    };
    let request = ItemTextWriteRequest {
        command,
        item_instance_id: item,
        definition,
        text,
    };
    match owner
        .root
        .commit_item_text_rewrite(
            &permit,
            owner.character,
            owner.holder,
            fence,
            policy,
            request,
            None,
            None,
        )
        .await
    {
        Ok(_) => Some(MailOutcome::TextWritten),
        Err(error) => refusal(error),
    }
}

fn refusal(error: ItemTextWriteError) -> Option<MailOutcome> {
    Some(match error {
        ItemTextWriteError::Unavailable(_) => return None,
        ItemTextWriteError::NotWritable => MailOutcome::NotWritable,
        ItemTextWriteError::TooLong => MailOutcome::TooLong,
        ItemTextWriteError::Exhausted => MailOutcome::Exhausted,
        _ => MailOutcome::Rejected,
    })
}
