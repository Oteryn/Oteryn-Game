//! The socket hands unpublished commits to the loss owner before closing.
use super::*;
use crate::durability::house_transfer::HouseTransferRequest;
use crate::foundation::fresh_admission_durability::FreshAdmissionOperationV1;
use oteryn_gameplay_entities::carrier::house_custody::HouseActorState;

#[doc(hidden)]
pub struct UnpublishedJoin {
    pub(in crate::gameplay_transport) source: AdmittedSession,
    pub(in crate::gameplay_transport) operation: FreshAdmissionOperationV1,
    pub(in crate::gameplay_transport) house: Option<PendingHouseJoin>,
}

pub(in crate::gameplay_transport) struct PendingHouseJoin {
    pub request: HouseTransferRequest,
    pub placement: HouseActorState,
    pub stage: HouseJoinStage,
}

pub(in crate::gameplay_transport) enum HouseJoinStage {
    Prepared,
    Issuing,
    Committed(AdmittedSession),
}

#[derive(Default)]
pub(in crate::gameplay_transport) struct JoinRecovery {
    pending: std::sync::Mutex<Option<Box<UnpublishedJoin>>>,
    published: std::sync::atomic::AtomicBool,
    failed: std::sync::atomic::AtomicBool,
    abandoned_resume: std::sync::atomic::AtomicBool,
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
    pub(in crate::gameplay_transport) fn take_pending(&self) -> Option<Box<UnpublishedJoin>> {
        (!self.published.load(std::sync::atomic::Ordering::Relaxed))
            .then(|| self.pending.lock().ok()?.take())
            .flatten()
    }
    pub(in crate::gameplay_transport) fn abandoned_resume(&self) -> bool {
        self.abandoned_resume
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    pub(in crate::gameplay_transport) fn published(&self) -> bool {
        self.published.load(std::sync::atomic::Ordering::Relaxed)
    }
}

pub(in crate::gameplay_transport) fn retain_committed_join(
    source: AdmittedSession,
    operation: &FreshAdmissionOperationV1,
) {
    let _ = JOIN_RECOVERY.try_with(|join| {
        if let Ok(mut pending) = join.pending.lock() {
            *pending = Some(Box::new(UnpublishedJoin {
                source,
                operation: operation.clone(),
                house: None,
            }));
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

pub(in crate::gameplay_transport) fn abandon_unpublished_resume() {
    let _ = JOIN_RECOVERY.try_with(|join| {
        join.abandoned_resume
            .store(true, std::sync::atomic::Ordering::Relaxed);
    });
}
