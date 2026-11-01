//! Ordered private phases of an admitted actor connection.
mod vip_presence;

use super::*;

mod achievement_notices;
mod activity_transfer;
mod autowalk;
mod command;
mod creature_push;
mod cyclopedia_tracker;
mod hotkey_equip;
mod house_transfer;
mod idle;
mod inbound;
mod integrity;
mod party_skull;
mod player_trade;
mod poll;
mod pvp_expert;
mod refresh;
mod setup;
mod snapshot;
mod target_move;
use command::CommandOutcome;
mod effects;
mod outbound;
mod result;
mod route;
mod use_effects;

// Fields drop in the reverse order of the original connection locals. In particular,
// release the pacing claim before dropping the session command stream (which locks ingress).
struct ServeState {
    integrity: integrity::Integrity,
    target_walk: Option<target_move::PendingWalk>,
    target_move_window: crate::gameplay_transport::attack::IntentWindow,
    push_window: crate::gameplay_transport::attack::IntentWindow,
    push_due: Option<tokio::time::Instant>,
    chase_yielded: bool,
    chase_due: Option<tokio::time::Instant>,
    held: std::collections::VecDeque<(Vec<u8>, bool)>,
    buffered: Option<BufferedStep>,
    step_wait_receipts: bool,
    step_holder: std::sync::Arc<()>,
    pacer: StepPacer,
    visibility_refresh: Option<tokio::time::Interval>,
    combat_refresh: Option<tokio::time::Interval>,
    npc_shown: Option<WireEntityRef>,
    npc_refresh: Option<tokio::time::Interval>,
    chat_refresh: Option<tokio::time::Interval>,
    quest_log_refresh: Option<tokio::time::Interval>,
    serene: Option<tokio::time::Interval>,
    cadence: tokio::time::Interval,
    liveness: Liveness,
    frames: FrameReader,
    reconcile_original_bank: bool,
    commands: crate::gameplay_transport::session_ingress::SessionCommandStream,
    sequence: u64,
    weather_sent: Option<WorldWeather>,
    world_clock_pacer: WorldClockDeltaPacer,
    world_clock: WorldClockSource,
    world_map: Option<SessionMapView>,
    quest_log: Option<QuestLogState>,
    container_views: Option<ContainerViewState>,
    visibility: Option<SessionVisibility>,
    appearances: SessionAppearance,
    selected_capabilities: SelectedCapabilities,
    item_view: Option<SessionItemView>,
    stash_rows: Option<Vec<[u8; 16]>>,
    stash_revision: u64,
    party_icons: Vec<oteryn_protocol_oteryn::party_skull::PartyIcon>,
    party_view: crate::gameplay_transport::party_command::SessionPartyView,
    team_finder_view: crate::gameplay_transport::team_finder_members::SessionTeamFinderView,
    market_revision: u64,
    /// P22d: the domain 98 revision this connection has seen; 1 after the join snapshot.
    cyclopedia_tracker_revision: u64,
    notices_selected: bool,
    achievement_notices: Option<achievement_notices::SeenAchievements>,
    overlay_served: bool,
    revision: u64,
    actor: ExactActorRef,
    generation: u64,
    admitted: AdmittedSession,
    policy: LivenessPolicy,
}

enum ServeEnd {
    Transferred(Box<AdmittedSession>),
    Disconnected,
    ControlLost,
    LoggedOut,
    Closed(FoundationProtocolError),
}

impl ServeEnd {
    fn finish(self, admitted: AdmittedSession) -> ConnectionEnd {
        match self {
            Self::Transferred(next) => ConnectionEnd::AdmittedThenDisconnected(*next),
            Self::Disconnected => ConnectionEnd::AdmittedThenDisconnected(admitted),
            Self::ControlLost => ConnectionEnd::AdmittedThenControlLost(admitted),
            Self::LoggedOut => ConnectionEnd::AdmittedThenLoggedOut(admitted),
            Self::Closed(error) => ConnectionEnd::AdmittedThenClosed(admitted, error),
        }
    }
}

async fn close<S: AsyncWrite + Unpin>(
    stream: &mut S,
    admitted: AdmittedSession,
    error: FoundationProtocolError,
) -> ServeEnd {
    let _ = close_admitted(stream, admitted, error).await;
    ServeEnd::Closed(error)
}

pub(super) async fn serve_admitted_actor<S, A>(
    stream: &mut S,
    mut admitted: AdmittedSession,
    authority: &A,
    policy: LivenessPolicy,
) -> ConnectionEnd
where
    S: AsyncRead + AsyncWrite + Unpin,
    A: FreshAdmissionAuthority,
{
    let mut integrity = integrity::Integrity::default();
    let root_authority = authority;
    let mut scoped_authority: Option<A> = None;
    loop {
        let authority = scoped_authority.as_ref().unwrap_or(root_authority);
        match authority.resolve_native_scope(admitted).await {
            Ok(Some(next)) => scoped_authority = Some(next),
            Ok(None) => {}
            Err(()) => return ConnectionEnd::AdmittedThenDisconnected(admitted),
        }
        let authority = scoped_authority.as_ref().unwrap_or(root_authority);
        if !authority.remember_activity_connection(admitted) {
            return ConnectionEnd::AdmittedThenDisconnected(admitted);
        }
        if !authority.prepare_quest_callback_resume(&mut admitted).await {
            return ConnectionEnd::AdmittedThenDisconnected(admitted);
        }
        if admitted.item_fence.is_some_and(|fence| {
            matches!(
                fence.runtime_scope,
                crate::foundation::RuntimeScopeRefV1::Instance { .. }
            ) && admitted
                .runtime_actor
                .is_none_or(|actor| actor.runtime_scope() != fence.runtime_scope)
        }) {
            admitted =
                match house_transfer::serve(stream, admitted, authority, policy, &mut integrity)
                    .await
                {
                    Ok(next) => next,
                    Err(end) => return *end,
                };
        }
        let playable = matches!(
            admitted.first_entry,
            FirstEntryOutcome::Positioned | FirstEntryOutcome::Reconciled
        );
        let Some(actor) = admitted.runtime_actor.filter(|_| playable) else {
            return hold_refused(stream, admitted, policy).await;
        };
        let Some(baseline) = authority.observe(actor).await else {
            return hold_refused(stream, admitted, policy).await;
        };
        let mut session = match setup::prepare(
            stream,
            &mut admitted,
            authority,
            policy,
            actor,
            baseline,
            integrity,
        )
        .await
        {
            Ok(session) => session,
            Err(end) => return end.finish(admitted),
        };
        loop {
            if !authority.remember_activity_connection(session.admitted) {
                return ConnectionEnd::AdmittedThenDisconnected(session.admitted);
            }
            match activity_transfer::pending(stream, authority, &mut session).await {
                Ok(Some(next)) => {
                    admitted = next;
                    integrity = std::mem::take(&mut session.integrity);
                    break;
                }
                Ok(None) => {}
                Err(end) => return end.finish(session.admitted),
            }
            let next = poll::next(stream, &mut session).await;
            // A member who did not issue the lever may have changed owner while
            // waiting for input. Its buffered source command never reaches the
            // destination's fresh command stream.
            match activity_transfer::pending(stream, authority, &mut session).await {
                Ok(Some(next)) => {
                    admitted = next;
                    integrity = std::mem::take(&mut session.integrity);
                    break;
                }
                Ok(None) => {}
                Err(end) => return end.finish(session.admitted),
            }
            let input = match inbound::receive(stream, authority, &mut session, next).await {
                Ok(Some(input)) => input,
                Ok(None) => continue,
                Err(end) => {
                    authority.player_trade_disconnected(session.admitted.game_session_id);
                    return end.finish(authority.current_activity_connection(session.admitted));
                }
            };
            if let Err(end) = command::execute(stream, authority, &mut session, input).await {
                if let ServeEnd::Transferred(next) = end {
                    admitted = *next;
                    integrity = std::mem::take(&mut session.integrity);
                    break;
                }
                authority.player_trade_disconnected(session.admitted.game_session_id);
                return end.finish(authority.current_activity_connection(session.admitted));
            }
        }
    }
}

/// Drive the actual STEP router with a production admission snapshot in dispatch regressions.
#[cfg(test)]
pub(in crate::gameplay_transport) async fn regression_routed_step<A: FreshAdmissionAuthority>(
    authority: &A,
    mut admitted: AdmittedSession,
    actor: ExactActorRef,
    direction: StepDirection,
) -> Result<StepOutcome, &'static str> {
    let (mut stream, _client) = tokio::io::duplex(1 << 22);
    let baseline = authority.observe(actor).await.ok_or("step baseline")?;
    let mut session = setup::prepare(
        &mut stream,
        &mut admitted,
        authority,
        IDLE_LIVENESS,
        actor,
        baseline,
        integrity::Integrity::default(),
    )
    .await
    .map_err(|_| "step setup")?;
    let payload = oteryn_protocol_oteryn::world_spatial::encode_step_intent(direction);
    let command = oteryn_protocol_oteryn::ClientCommandView {
        command_id: admitted.continuity.next_command_id,
        command_type: oteryn_protocol_oteryn::world_spatial::COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
        payload: &payload,
    };
    loop {
        match route::step(authority, &mut session, command, false).await {
            route::StepAction::Dispatch(Dispatch::Step(outcome, _)) => return Ok(outcome),
            route::StepAction::Wait(due) => tokio::time::sleep_until(due).await,
            _ => return Err("step router did not dispatch"),
        }
    }
}
