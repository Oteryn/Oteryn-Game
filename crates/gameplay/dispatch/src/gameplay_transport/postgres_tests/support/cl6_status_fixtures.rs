use super::*;

use crate::gameplay_transport::capabilities::{OfferedCapability, SelectedCapabilities};

use oteryn_protocol_oteryn::mechanics_status::{
    CAPABILITY_MECHANICS_STATUS_V1, MechanicsStatus,
};

pub(super) async fn query_actor(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    actor: crate::foundation::ExactActorRef,
    session: crate::foundation::GameSessionId,
) -> TestResult<MechanicsStatus> {
    use crate::gameplay_transport::connection::{
        AdmittedSession, FirstEntryOutcome, SessionContinuity,
    };
    let fence = owner
        .current_quest_fence(session)
        .await
        .map_err(debug)?
        .ok_or("CL6 current fence")?
        .item_fence();
    let mut continuity = SessionContinuity::FRESH;
    continuity.connection_generation = fence.connection_generation.get();
    continuity.selected_capabilities = SelectedCapabilities::select(
        &[OfferedCapability {
            id: CAPABILITY_MECHANICS_STATUS_V1,
            requires: &[],
        }],
        &[CAPABILITY_MECHANICS_STATUS_V1],
    )
    .ok_or("CL6 capability")?;
    let admitted = AdmittedSession {
        game_session_id: session,
        world_id: owner.world_id,
        channel_id: owner.channel_id,
        runtime_actor: Some(actor),
        first_entry: FirstEntryOutcome::Positioned,
        controller: None,
        continuity,
        item_fence: Some(fence),
    };
    MechanicsStatus::decode(
        &owner
            .mechanics_status_page(admitted)
            .await
            .ok_or("CL6 projection")?,
    )
    .ok_or_else(|| "CL6 projection decode".into())
}

pub(super) async fn check_timer(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    actor: crate::foundation::ExactActorRef,
    session: crate::foundation::GameSessionId,
    key: &str,
    active: bool,
    max_ms: u64,
) -> TestResult {
    let status = query_actor(owner, actor, session).await?;
    let timer = status.timers.iter().find(|timer| timer.key == key);
    if active {
        let timer = timer.ok_or("CL6 active timer")?;
        assert!((1..=max_ms).contains(&timer.remaining_ms));
        assert_eq!(timer.online_only, key == "wealth_duplex");
    } else {
        assert!(timer.is_none(), "expired/instant timer: {key}");
    }
    eprintln!("CL6_TIMER POSTGRES_ASSERTION_EXECUTED key={key} active={active}");
    Ok(())
}
