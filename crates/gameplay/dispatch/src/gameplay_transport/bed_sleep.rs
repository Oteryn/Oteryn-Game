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
    let Some(owner) = authority
        .prepare_bed_sleep(actor, command, intent, target)
        .await
    else {
        return Ok(UseOutcome::rejected());
    };
    finish_owner(command, intent, owner).await
}

pub(super) async fn finish_owner(
    command: UseCommand,
    intent: BedSleepIntent,
    owner: crate::durability::bed_sleep::BedSleepOwner<'_, '_, '_>,
) -> Result<UseOutcome, crate::durability::DurabilityError> {
    finish_owner_with_house_position(command, intent, owner, None).await
}

pub(super) async fn finish_owner_with_house_position(
    command: UseCommand,
    intent: BedSleepIntent,
    owner: crate::durability::bed_sleep::BedSleepOwner<'_, '_, '_>,
    house: Option<(
        crate::durability::character_last_position::HouseLastPosition,
        String,
    )>,
) -> Result<UseOutcome, crate::durability::DurabilityError> {
    finish_skill_with_house_position(command, intent.skill, owner, house).await
}

pub(super) async fn finish_skill_with_house_position(
    command: UseCommand,
    skill: u8,
    owner: crate::durability::bed_sleep::BedSleepOwner<'_, '_, '_>,
    house: Option<(
        crate::durability::character_last_position::HouseLastPosition,
        String,
    )>,
) -> Result<UseOutcome, crate::durability::DurabilityError> {
    if command.item_fence != Some(owner.request.fence)
        || command.game_session_id != owner.request.fence.game_session_id
        || command.command_id != owner.request.command_id
        || skill != owner.request.skill
    {
        return Ok(UseOutcome::rejected());
    }
    let disposition = bed_sleep_disposition(owner.commit_with_house_position(house).await?);
    Ok(UseOutcome {
        disposition,
        ..UseOutcome::rejected()
    })
}

/// Keep the durable ACL/reservation decisions on the real USE response path.
/// Crystal 201189f `src/items/bed.cpp:126-142,162-175` distinguishes an
/// authorized wake/sleep from a bed occupied by another character.
fn bed_sleep_disposition(outcome: crate::durability::bed_sleep::BedSleepOutcome) -> UseDisposition {
    use crate::durability::bed_sleep::BedSleepOutcome;
    match outcome {
        BedSleepOutcome::Slept => UseDisposition::OfflineTraining,
        BedSleepOutcome::Occupied => UseDisposition::BedOccupied,
        BedSleepOutcome::Stale | BedSleepOutcome::Conflict => UseDisposition::StaleState,
        BedSleepOutcome::Denied => UseDisposition::Rejected,
    }
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
    // C10: the production bed owner (House fence re-proof, bed head and House
    // position capture) made this future large; boxing keeps it off the stack of
    // every connection command future that only might route a bed USE.
    Box::pin(authority.sleep_in_bed(actor, command, intent, target)).await
}
