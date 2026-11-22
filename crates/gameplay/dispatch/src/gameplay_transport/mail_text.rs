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
    let house_origin = owner.resident_origin(admitted).await;
    let native = {
        let runtime = owner.runtime.lock().await;
        let control = if house_origin.is_some() {
            if runtime
                .house_player_writes_fenced(actor, admitted.game_session_id)
                .ok()?
            {
                return Some(MailOutcome::Rejected);
            }
            runtime.house_player_control_facts(actor, admitted.game_session_id)
        } else {
            runtime.player_control_facts(actor, admitted.game_session_id)
        };
        if !control.is_ok_and(|facts| facts.control_loss.is_none()) {
            return Some(MailOutcome::Rejected);
        }
        let states = owner.states.spell_states.lock().await;
        let busy = if house_origin.is_some() {
            states.has_pending_spell_commit(actor, admitted.game_session_id)
        } else {
            !states.spell_idle(&runtime, actor, admitted.game_session_id)
        };
        let Some(native) = crate::gameplay_transport::equipment_move::native_for_move(
            &runtime,
            owner.active_generation,
            states.is_dead(actor),
            busy,
        ) else {
            return Some(MailOutcome::Rejected);
        };
        native
    };
    let definition = owner
        .root
        .read_carried_item_text(owner.character, owner.holder, fence, house_origin, item)
        .await
        .ok()?
        .map(|(definition, _)| definition);
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
    let result = if house_origin.is_some() {
        owner
            .root
            .commit_house_item_text_rewrite(
                &permit,
                owner.character,
                owner.holder,
                fence,
                policy,
                request,
                admitted.channel_id,
            )
            .await
    } else {
        owner
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
    };
    match result {
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
