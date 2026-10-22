//! The command-31 transport boundary. The House owner must independently
//! resolve the active pair, reservation, ACL and fenced logout at its turn.

use super::connection::{FreshAdmissionAuthority, UseCommand, UseOutcome};
use super::item_view::{ItemKey, SessionItemView};
use crate::foundation::ExactActorRef;
use oteryn_protocol_oteryn::bed::BedSleepIntent;
use oteryn_protocol_oteryn::world_interaction::WorldInteractionWireError;
use oteryn_protocol_oteryn::world_object::UseDisposition;

pub(super) async fn dispatch(
    authority: &impl FreshAdmissionAuthority,
    actor: ExactActorRef,
    command: UseCommand,
    intent: Result<BedSleepIntent, WorldInteractionWireError>,
    view: Option<&SessionItemView>,
) -> UseOutcome {
    let Ok(intent) = intent else {
        return UseOutcome::rejected();
    };
    let Some(target) = view.and_then(|view| view.resolve(intent.target)) else {
        return UseOutcome {
            disposition: UseDisposition::StaleState,
            ..UseOutcome::rejected()
        };
    };
    if !matches!(target, ItemKey::MapBase { .. }) {
        return UseOutcome {
            disposition: UseDisposition::NotPossible,
            ..UseOutcome::rejected()
        };
    }
    authority.sleep_in_bed(actor, command, intent, target).await
}
