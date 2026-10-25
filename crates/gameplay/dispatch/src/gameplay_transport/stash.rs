//! Reachable native locker -> existing Character item fence -> stash conversion.
use super::ComposedFreshAdmission;
use super::connection::UseCommand;
use super::equipment_move::native_for_move;
use super::item_ref_admission::NativeItemWeights;
use super::item_view::ItemKey;
use crate::durability::character_stash::StashRequest;
use crate::foundation::{CommandId, CommandRef, ExactActorRef};
use oteryn_protocol_oteryn::stash::*;

pub(super) async fn execute(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    intent: StashIntent,
    selected: Option<ItemKey>,
    row_open: bool,
) -> Option<StashReply> {
    let mut reply = StashReply {
        outcome: StashOutcome::Rejected,
        stowed_quantity: 0,
        transaction: None,
        page: intent.page,
        has_more: false,
        rows: Vec::new(),
    };
    let Some(fence) = command.item_fence else {
        return Some(reply);
    };
    let current = super::FreshAdmissionStore::from_root(admission.root.clone())
        .current_session_at(command.game_session_id)
        .await
        .ok()?
        .0;
    if current.session_state() != super::GameSessionState::Active
        || super::item_fence_of(current) != Some(fence)
    {
        return Some(reply);
    }
    let permit = admission.spell_lane_permit().await?;
    let (native, reachable) = {
        let mut runtime = admission.runtime.lock().await;
        let states = admission.states.spell_states.lock().await;
        (
            native_for_move(
                &runtime,
                admission.active_generation,
                states.is_dead(actor),
                states.has_pending_spell_commit(actor, command.game_session_id),
            ),
            super::bundle_map_position(&mut runtime, admission.movement_cells, actor)
                .as_ref()
                .and_then(super::depot::locker)
                .is_some(),
        )
    };
    let live_allowed = native.is_some();
    // Historical receipt identity is independent of current gameplay eligibility.
    // Never use it to reopen a view or reacquire current mutation authority.
    let replay_content = admission.active_generation?.native_gameplay()?;
    let native = native.unwrap_or(replay_content);
    let request = StashRequest {
        command: CommandRef::new(
            command.game_session_id,
            CommandId::new(command.command_id).ok()?,
        ),
        intent,
        source: match selected {
            Some(ItemKey::Instance(item)) => Some(item),
            _ => None,
        },
        row_open,
        access_refusal: (!reachable).then_some(StashOutcome::NoLocker),
        premium: admission
            .session_premium_gate(command.game_session_id)
            .await,
    };
    if intent.action != StashAction::Browse {
        let previous = admission
            .root
            .stash_conversion(
                &permit,
                admission.character,
                admission.holder,
                fence,
                request.clone(),
                native,
                &NativeItemWeights(native),
                true,
            )
            .await
            .ok()?;
        let receipt = match previous {
            Some(previous) => previous,
            None if !live_allowed => return Some(reply),
            None => admission
                .root
                .stash_conversion(
                    &permit,
                    admission.character,
                    admission.holder,
                    fence,
                    request,
                    native,
                    &NativeItemWeights(native),
                    false,
                )
                .await
                .ok()??,
        };
        reply.outcome = receipt.outcome;
        reply.stowed_quantity = receipt.stowed_quantity;
        reply.transaction = Some(receipt.transaction);
        if receipt.outcome == StashOutcome::Committed {
            admission
                .states
                .attack
                .lock()
                .await
                .forget_weapon(actor, command.game_session_id);
        }
    } else {
        if !live_allowed {
            return Some(reply);
        }
        reply.outcome = if reachable {
            StashOutcome::Opened
        } else {
            StashOutcome::NoLocker
        };
    }
    if reachable && live_allowed {
        (reply.rows, reply.has_more) = admission
            .root
            .read_stash_page(admission.character, fence.character_id, intent.page, native)
            .await
            .ok()?;
    }
    Some(reply)
}
