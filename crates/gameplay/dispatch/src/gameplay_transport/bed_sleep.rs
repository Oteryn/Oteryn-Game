//! The command-31 transport boundary. The House owner must independently
//! resolve the active pair, reservation, ACL and fenced logout at its turn.

use super::connection::{FreshAdmissionAuthority, UseCommand, UseOutcome};
use super::item_view::{ItemKey, SessionItemView};
use crate::foundation::ExactActorRef;
use oteryn_protocol_oteryn::bed::BedSleepIntent;
use oteryn_protocol_oteryn::world_interaction::WorldInteractionWireError;
use oteryn_protocol_oteryn::world_object::UseDisposition;

pub(super) async fn sleep_with_owner(
    authority: &(impl FreshAdmissionAuthority + ?Sized),
    actor: ExactActorRef,
    command: UseCommand,
    intent: BedSleepIntent,
    target: ItemKey,
) -> Result<UseOutcome, crate::durability::DurabilityError> {
    use crate::durability::bed_sleep::BedSleepOutcome;

    let Some(owner) = authority
        .prepare_bed_sleep(actor, command, intent, target)
        .await
    else {
        return Ok(UseOutcome::rejected());
    };
    if command.item_fence != Some(owner.request.fence)
        || command.game_session_id != owner.request.fence.game_session_id
        || command.command_id != owner.request.command_id
        || intent.skill != owner.request.skill
    {
        return Ok(UseOutcome::rejected());
    }
    let disposition = match owner.commit().await? {
        BedSleepOutcome::Slept => UseDisposition::OfflineTraining,
        BedSleepOutcome::Occupied => UseDisposition::BedOccupied,
        BedSleepOutcome::Stale | BedSleepOutcome::Conflict => UseDisposition::StaleState,
        BedSleepOutcome::Denied => UseDisposition::Rejected,
    };
    Ok(UseOutcome {
        disposition,
        ..UseOutcome::rejected()
    })
}

pub(super) async fn dispatch(
    authority: &impl FreshAdmissionAuthority,
    actor: ExactActorRef,
    command: UseCommand,
    intent: Result<BedSleepIntent, WorldInteractionWireError>,
    view: Option<&SessionItemView>,
) -> Result<UseOutcome, crate::durability::DurabilityError> {
    let Ok(intent) = intent else {
        return Ok(UseOutcome::rejected());
    };
    let Some(target) = view.and_then(|view| view.resolve(intent.target)) else {
        return Ok(UseOutcome {
            disposition: UseDisposition::StaleState,
            ..UseOutcome::rejected()
        });
    };
    if !matches!(target, ItemKey::MapBase { .. }) {
        return Ok(UseOutcome {
            disposition: UseDisposition::NotPossible,
            ..UseOutcome::rejected()
        });
    }
    authority.sleep_in_bed(actor, command, intent, target).await
}
