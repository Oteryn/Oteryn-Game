//! The command-31 transport boundary. The House owner must independently
//! resolve the active pair, reservation, ACL and fenced logout at its turn.

use super::connection::{FreshAdmissionAuthority, UseCommand, UseOutcome};
use super::item_view::{ItemKey, SessionItemView};
use crate::foundation::ExactActorRef;
use oteryn_protocol_oteryn::bed::BedSleepIntent;
use oteryn_protocol_oteryn::world_interaction::WorldInteractionWireError;
use oteryn_protocol_oteryn::world_object::UseDisposition;
use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Clone)]
pub struct PreparedBedSleep {
    pub actor: ExactActorRef,
    pub target: ItemKey,
    pub request: crate::durability::bed_sleep::BedSleepRequest,
}

#[derive(Default)]
pub struct BedSleepPreparations(Mutex<HashMap<crate::foundation::GameSessionId, PreparedBedSleep>>);

impl BedSleepPreparations {
    pub fn install(&self, prepared: PreparedBedSleep) -> bool {
        self.0.lock().is_ok_and(|mut entries| {
            entries
                .insert(prepared.request.fence.game_session_id, prepared)
                .is_none()
        })
    }

    pub(super) fn take(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        target: ItemKey,
    ) -> Option<PreparedBedSleep> {
        let mut entries = self.0.lock().ok()?;
        let prepared = entries.get(&command.game_session_id)?;
        if prepared.actor != actor
            || prepared.target != target
            || prepared.request.command_id != command.command_id
        {
            return None;
        }
        entries.remove(&command.game_session_id)
    }

    pub(super) fn forget(&self, session: crate::foundation::GameSessionId) {
        if let Ok(mut entries) = self.0.lock() {
            entries.remove(&session);
        }
    }
}

impl super::ComposedFreshAdmission<'_, '_, '_> {
    pub fn prepare_production_bed_sleep(&self, prepared: PreparedBedSleep) -> bool {
        self.states.bed_sleep.install(prepared)
    }

    pub(super) fn production_bed_sleep_owner(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        target: ItemKey,
    ) -> Option<crate::durability::bed_sleep::BedSleepOwner<'_, '_, '_>> {
        let request = self.states.bed_sleep.take(actor, command, target)?.request;
        Some(crate::durability::bed_sleep::BedSleepOwner {
            root: self.root,
            recovery: self.character,
            node: self.holder,
            presence: crate::durability::bed_sleep::ResolvedHousePresence {
                world: request.house.world_id(),
                house_key: request.house.house_key().to_owned(),
                actor_session: request.fence.game_session_id,
                scope_generation: request.fence.scope_ownership_generation.get(),
            },
            request,
        })
    }
}

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
