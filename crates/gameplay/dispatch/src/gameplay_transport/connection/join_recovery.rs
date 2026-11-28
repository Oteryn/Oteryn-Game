//! The socket hands unpublished commits to the loss owner before closing.
use super::*;
use crate::durability::house_transfer::HouseTransferRecovery;
use crate::durability::house_transfer::HouseTransferRequest;
use crate::foundation::fresh_admission_durability::FreshAdmissionCommitRequestV1;
use crate::foundation::{
    CompleteReconnectDurabilityOperationV1, ControlLossEpochRefV1, ReconnectIdentityV1,
};
use oteryn_gameplay_entities::carrier::PlayerActorReservation;
use oteryn_gameplay_entities::carrier::house_custody::HouseActorState;

#[doc(hidden)]
pub struct UnpublishedJoin {
    pub(in crate::gameplay_transport) source: AdmittedSession,
    pub(in crate::gameplay_transport) action: JoinAction,
    pub(in crate::gameplay_transport) house: Option<PendingHouseJoin>,
    pub(in crate::gameplay_transport) committed: bool,
}

pub(in crate::gameplay_transport) enum JoinAction {
    Fresh {
        request: Box<FreshAdmissionCommitRequestV1>,
        reservation: PlayerActorReservation,
        channel_entry_started: bool,
    },
    Resume {
        identity: Box<ReconnectIdentityV1>,
        operation: Option<Box<CompleteReconnectDurabilityOperationV1>>,
        epoch: ControlLossEpochRefV1,
        house: bool,
    },
    NativeHouse(Box<HouseTransferRecovery>),
    CapabilityMismatch {
        epoch: ControlLossEpochRefV1,
        activity_store: Option<Box<crate::durability::fresh_admission::FreshAdmissionStore>>,
    },
    // The native Activity registry owns reconciliation as soon as its socket guard drops.
    NativeActivity,
}

pub(in crate::gameplay_transport) struct PendingHouseJoin {
    pub request: HouseTransferRequest,
    pub placement: HouseActorState,
    pub stage: HouseJoinStage,
}

pub(in crate::gameplay_transport) enum HouseJoinStage {
    Prepared,
    Issuing,
    Committed(Box<AdmittedSession>),
}

#[derive(Default)]
pub(in crate::gameplay_transport) struct JoinRecovery {
    deadline: Option<tokio::time::Instant>,
    fresh_selection: std::sync::Mutex<Option<SelectedCapabilities>>,
    pending: std::sync::Mutex<Option<Box<UnpublishedJoin>>>,
    published: std::sync::atomic::AtomicBool,
    failed: std::sync::atomic::AtomicBool,
    name: std::sync::Mutex<
        Option<(
            ExactActorRef,
            oteryn_protocol_oteryn::entity_names::EntityDisplayName,
        )>,
    >,
}

tokio::task_local! {
    pub(in crate::gameplay_transport) static JOIN_RECOVERY: JoinRecovery;
}

impl JoinRecovery {
    pub(in crate::gameplay_transport) fn by(deadline: tokio::time::Instant) -> Self {
        Self {
            deadline: Some(deadline),
            ..Self::default()
        }
    }

    pub(in crate::gameplay_transport) fn take_pending(&self) -> Option<Box<UnpublishedJoin>> {
        (!self.published.load(std::sync::atomic::Ordering::Relaxed))
            .then(|| self.pending.lock().ok()?.take())
            .flatten()
    }
    pub(in crate::gameplay_transport) fn published(&self) -> bool {
        self.published.load(std::sync::atomic::Ordering::Relaxed)
    }
}

pub(super) fn join_deadline() -> Option<tokio::time::Instant> {
    JOIN_RECOVERY.try_with(|join| join.deadline).ok().flatten()
}

pub(super) fn retain_fresh_selection(selected: SelectedCapabilities) {
    let _ = JOIN_RECOVERY.try_with(|join| {
        if let Ok(mut selection) = join.fresh_selection.lock() {
            *selection = Some(selected);
        }
    });
}

pub(in crate::gameplay_transport) fn fresh_selection() -> Option<SelectedCapabilities> {
    JOIN_RECOVERY
        .try_with(|join| *join.fresh_selection.lock().ok()?)
        .ok()
        .flatten()
}

pub(in crate::gameplay_transport) fn retain_join(
    source: AdmittedSession,
    action: JoinAction,
) -> bool {
    JOIN_RECOVERY
        .try_with(|join| {
            let Ok(mut pending) = join.pending.lock() else {
                return false;
            };
            *pending = Some(Box::new(UnpublishedJoin {
                source,
                action,
                house: None,
                committed: false,
            }));
            true
        })
        .unwrap_or(false)
}

pub(in crate::gameplay_transport) fn clear_join() {
    let _ = JOIN_RECOVERY.try_with(|join| {
        if let Ok(mut pending) = join.pending.lock() {
            *pending = None;
        }
    });
}

pub(in crate::gameplay_transport) fn retain_committed_session(source: AdmittedSession) {
    update_pending(|join| {
        join.source = source;
        join.committed = true;
    });
}

pub(super) fn committed_join() -> Option<AdmittedSession> {
    JOIN_RECOVERY
        .try_with(|join| {
            let pending = join.pending.lock().ok()?;
            let pending = pending.as_ref()?;
            if let Some(PendingHouseJoin {
                stage: HouseJoinStage::Committed(known),
                ..
            }) = &pending.house
            {
                return Some(**known);
            }
            pending.committed.then_some(pending.source)
        })
        .ok()
        .flatten()
}

pub(in crate::gameplay_transport) fn retain_channel_entry_started() {
    update_pending(|join| {
        if let JoinAction::Fresh {
            channel_entry_started,
            ..
        } = &mut join.action
        {
            *channel_entry_started = true;
        }
    });
}

pub(in crate::gameplay_transport) fn retain_resume_operation(
    operation: &CompleteReconnectDurabilityOperationV1,
) {
    update_pending(|join| {
        if let JoinAction::Resume {
            operation: pending, ..
        } = &mut join.action
        {
            *pending = Some(Box::new(operation.clone()));
        }
    });
}

fn update_pending(f: impl FnOnce(&mut UnpublishedJoin)) {
    let _ = JOIN_RECOVERY.try_with(|join| {
        if let Ok(mut pending) = join.pending.lock()
            && let Some(record) = pending.as_deref_mut()
        {
            f(record);
        }
    });
}

pub(in crate::gameplay_transport) fn retain_house_join(
    source: AdmittedSession,
    request: &HouseTransferRequest,
    placement: &HouseActorState,
) {
    update_pending(|join| {
        join.source = source;
        join.house = Some(PendingHouseJoin {
            request: request.clone(),
            placement: placement.clone(),
            stage: HouseJoinStage::Prepared,
        });
    });
}

pub(in crate::gameplay_transport) fn retain_house_stage(stage: HouseJoinStage) {
    update_pending(|join| {
        if let Some(house) = join.house.as_mut() {
            house.stage = stage;
        }
    });
}

pub(super) fn join_published() {
    let _ = JOIN_RECOVERY.try_with(|join| {
        join.published
            .store(true, std::sync::atomic::Ordering::Relaxed);
        if let Ok(mut pending) = join.pending.lock() {
            *pending = None;
        }
    });
}

pub(in crate::gameplay_transport) fn retain_join_name(
    actor: ExactActorRef,
    name: oteryn_protocol_oteryn::entity_names::EntityDisplayName,
) {
    let _ = JOIN_RECOVERY.try_with(|join| {
        if let Ok(mut saved) = join.name.lock() {
            *saved = Some((actor, name));
        }
    });
}

pub(in crate::gameplay_transport) fn join_name(
    actor: ExactActorRef,
) -> Option<oteryn_protocol_oteryn::entity_names::EntityDisplayName> {
    JOIN_RECOVERY
        .try_with(|join| {
            if join.published() {
                return None;
            }
            let (owner, name) = join.name.lock().ok()?.as_ref().copied()?;
            (actor == owner).then_some(name)
        })
        .ok()
        .flatten()
}

pub(in crate::gameplay_transport) fn require_join_recovery() {
    let _ = JOIN_RECOVERY.try_with(|join| {
        join.failed
            .store(true, std::sync::atomic::Ordering::Relaxed)
    });
}

pub(super) fn join_requires_recovery() -> bool {
    JOIN_RECOVERY
        .try_with(|join| join.failed.load(std::sync::atomic::Ordering::Relaxed))
        .unwrap_or(false)
}
