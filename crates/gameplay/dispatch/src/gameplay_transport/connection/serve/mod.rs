//! Ordered private phases of an admitted actor connection.
mod vip_presence;

use super::*;
use crate::gameplay_transport::operator_event;
use crate::gameplay_transport::record_join_progress;

mod achievement_notices;
mod activity_transfer;
mod analyser;
mod autowalk;
mod command;
mod creature_push;
mod cyclopedia_tracker;
mod forge;
mod guild_emblem;
mod hotkey_equip;
mod house_hireling;
mod house_transfer;
mod idle;
mod inbound;
mod integrity;
mod inventory_capacity;
mod party_membership;
mod party_skull;
mod player_trade;
mod poll;
mod pvp_expert;
mod refresh;
mod setup;
mod sidecar;
mod snapshot;
mod spell_catalog;
mod spell_repair;
mod target_move;
use command::CommandOutcome;
mod cooldowns;
mod effects;
mod native_cues;
mod outbound;
use outbound::write_frame;
mod result;
mod route;
mod use_effects;

/// Observe a slow owner without cancelling, retrying or extending its work. One pending
/// event is emitted even if it never returns; normal fast calls remain silent.
async fn observed<T>(phase: &'static str, work: impl std::future::Future<Output = T>) -> T {
    let started = tokio::time::Instant::now();
    let mut work = std::pin::pin!(work);
    let mut warning = std::pin::pin!(tokio::time::sleep(std::time::Duration::from_secs(1)));
    let mut reported = false;
    let result = std::future::poll_fn(|cx| {
        if let std::task::Poll::Ready(result) = work.as_mut().poll(cx) {
            return std::task::Poll::Ready(result);
        }
        if !reported && warning.as_mut().poll(cx).is_ready() {
            operator_event(&format!(
                "event=connection_phase_wait phase={phase} state=pending elapsed_ms={}",
                started.elapsed().as_millis(),
            ));
            reported = true;
        }
        std::task::Poll::Pending
    })
    .await;
    if reported {
        operator_event(&format!(
            "event=connection_phase_wait phase={phase} state=completed elapsed_ms={}",
            started.elapsed().as_millis(),
        ));
    }
    result
}

// Fields drop in the reverse order of the original connection locals. In particular,
// release the pacing claim before dropping the session command stream (which locks ingress).
struct ServeState {
    cooldowns: Option<cooldowns::CooldownFeed>,
    cast_chat_revision: u64,
    spell_catalog: Option<spell_catalog::SessionCatalog>,
    integrity: integrity::Integrity,
    target_walk: Option<target_move::PendingWalk>,
    target_move_window: crate::gameplay_transport::attack::IntentWindow,
    push_window: crate::gameplay_transport::attack::IntentWindow,
    push_due: Option<tokio::time::Instant>,
    chase_yielded: bool,
    input_after_refresh: bool,
    next_periodic_refresh: u8,
    chase_due: Option<tokio::time::Instant>,
    held: std::collections::VecDeque<(Vec<u8>, bool, tokio::time::Instant)>,
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
    activity_ack: Option<oteryn_protocol_oteryn::activity_scope::ActivityScopeAck>,
    reconcile_original_bank: bool,
    commands: crate::gameplay_transport::session_ingress::SessionCommandStream,
    sequence: u64,
    movement_effect_revision: u64,
    movement_speech_revision: u64,
    creature_visual_revision: u64,
    native_cue_actor: ExactActorRef,
    sound_cursor: u64,
    ambient_revision: u64,
    item_effects_revision: u64,
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
    managed_container_move_pending: Option<routing::managed_container_move::Pending>,
    stash_container_pending: Option<routing::stash_container::Pending>,
    stash_revision: u64,
    party_icons: Vec<oteryn_protocol_oteryn::party_skull::PartyIcon>,
    /// GUILD-EMBLEM-1: the emblems last sent on this connection under capability 509.
    guild_emblems: Vec<oteryn_protocol_oteryn::guild_emblem::GuildEmblem>,
    /// INVCAP-1: the capacity last sent on this connection under capability 508.
    inventory_capacity: Option<oteryn_protocol_oteryn::inventory_capacity::InventoryCapacity>,
    inventory_capacity_refresh_at: Option<tokio::time::Instant>,
    /// INVCAP-1: the maximum health and mana last observed; a change rereads the capacity.
    inventory_capacity_vitals_max: Option<(u32, u32)>,
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
    require_session_binding: bool,
    admitted: AdmittedSession,
    policy: LivenessPolicy,
}

/// A wire command belongs to one GameSession, even when a successor restarts
/// generation1/command1 on the same socket. Never acknowledge stale-session
/// bytes against the successor's fresh counter or charge its ingress window.
fn command_session_matches(
    frame: &[u8],
    admitted: AdmittedSession,
    required: bool,
) -> Result<bool, FoundationProtocolError> {
    let envelope = decode_wire_envelope(frame)?;
    if envelope.message_type() != MessageType::ClientCommand
        || envelope.connection_generation() != admitted.continuity.connection_generation
    {
        // Existing message/generation validators retain their original errors.
        return Ok(true);
    }
    Ok(match envelope.client_command_session_id()? {
        Some(session) => session == admitted.game_session_id.as_bytes(),
        None => !required,
    })
}

impl Drop for ServeState {
    fn drop(&mut self) {
        if let Some(buffered) = &self.buffered {
            // Only a frame still waiting here is proven not to have entered its
            // movement owner. An in-flight STEP has already taken this buffer.
            let _ =
                self.commands
                    .cancel_buffered_step(&buffered.frame, self.generation, self.sequence);
        }
    }
}

enum ServeEnd {
    Transferred(Box<AdmittedSession>),
    Server {
        cause: &'static str,
        source: &'static std::panic::Location<'static>,
    },
    Transport(TransportEnd),
    ControlLost(&'static str),
    LoggedOut,
    Closed(FoundationProtocolError),
}

enum TransportEnd {
    Read(std::io::ErrorKind),
    Write(std::io::ErrorKind),
}

impl From<super::super::tcp_tls::FrameFailure> for TransportEnd {
    fn from(failure: super::super::tcp_tls::FrameFailure) -> Self {
        use super::super::tcp_tls::FrameFailure;
        match failure {
            FrameFailure::Read(kind) => Self::Read(kind),
            FrameFailure::Write(kind) => Self::Write(kind),
        }
    }
}

impl TransportEnd {
    fn fields(&self) -> (&'static str, &'static str, std::io::ErrorKind) {
        use std::io::ErrorKind;
        let (path, kind) = match *self {
            Self::Read(kind) => ("read", kind),
            Self::Write(kind) => ("write", kind),
        };
        let reason = match (path, kind) {
            (_, ErrorKind::ConnectionReset) => "peer_reset",
            ("read", ErrorKind::UnexpectedEof) => "peer_eof",
            ("write", ErrorKind::TimedOut) => "io_deadline",
            ("read", _) => "read_error",
            _ => "write_error",
        };
        (path, reason, kind)
    }
}

impl ServeEnd {
    #[track_caller]
    fn server(cause: &'static str) -> Self {
        match super::super::tcp_tls::FrameFailure::take() {
            Some(io) => Self::Transport(io.into()),
            None => Self::Server {
                cause,
                source: std::panic::Location::caller(),
            },
        }
    }

    #[track_caller]
    fn finish(self, admitted: AdmittedSession) -> ConnectionEnd {
        self.finish_with_phase(admitted, None)
    }

    #[track_caller]
    fn finish_with_phase(
        self,
        admitted: AdmittedSession,
        phase: Option<&'static str>,
    ) -> ConnectionEnd {
        let (reason, detail, source) = match &self {
            Self::Transferred(_) => (
                "transferred",
                " initiator=server".into(),
                std::panic::Location::caller(),
            ),
            Self::Server { cause, source } => (
                "server_close",
                format!(" initiator=server cause={cause}"),
                *source,
            ),
            Self::Transport(transport) => {
                let (path, reason, kind) = transport.fields();
                let initiator = match reason {
                    "peer_eof" | "peer_reset" => "peer",
                    "io_deadline" => "server",
                    _ => "transport",
                };
                (
                    reason,
                    format!(" path={path} initiator={initiator} io_kind={kind:?}"),
                    std::panic::Location::caller(),
                )
            }
            Self::ControlLost(path) => (
                "control_lost",
                format!(" path={path} initiator=server"),
                std::panic::Location::caller(),
            ),
            Self::LoggedOut => (
                "logged_out",
                " initiator=server".into(),
                std::panic::Location::caller(),
            ),
            Self::Closed(_) => (
                "protocol_error",
                " initiator=server".into(),
                std::panic::Location::caller(),
            ),
        };
        let phase = phase.map_or_else(String::new, |phase| format!(" phase={phase}"));
        let code = match &self {
            Self::Closed(error) => format!(" reason_code={}", error.code()),
            _ => String::new(),
        };
        operator_event(&format!(
            "event=serve_end reason={reason}{detail}{phase}{code} source={source} next_command_id={} server_sequence={}",
            admitted.continuity.next_command_id, admitted.continuity.server_sequence,
        ));
        match self {
            Self::Transferred(next) => ConnectionEnd::AdmittedThenDisconnected(*next),
            Self::Server { .. } | Self::Transport(_) => {
                ConnectionEnd::AdmittedThenDisconnected(admitted)
            }
            Self::ControlLost(_) => ConnectionEnd::AdmittedThenControlLost(admitted),
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
    // Staff visibility shares the view cadence; input/timer ticks must not turn
    // best-effort reads into continuous contention for the gameplay DB holder.
    let mut next_staff_refresh = tokio::time::Instant::now() + VISIBILITY_REFRESH;
    let mut integrity = integrity::Integrity::default();
    let root_authority = authority;
    // Keep one guard across owner changes. It must also drop the transferred
    // recipient if a handoff write is cancelled before destination setup.
    let mut server_messages = None;
    let mut scoped_authority: Option<A> = None;
    let mut activity_input = None;
    let mut require_session_binding = false;
    let mut handoff_effect_start = None;
    loop {
        let authority = scoped_authority.as_ref().unwrap_or(root_authority);
        match authority.resolve_native_scope(admitted).await {
            Ok(Some(next)) => scoped_authority = Some(next),
            Ok(None) => {}
            Err(()) => return ServeEnd::server("scope_resolution_failed").finish(admitted),
        }
        let authority = scoped_authority.as_ref().unwrap_or(root_authority);
        // Resolve the admitted scope before checking its actor, including a
        // reconnect whose actor already belongs to an activity Instance.
        if !Box::pin(authority.staff_refresh(admitted)).await {
            return ServeEnd::server("staff_refresh_failed").finish(admitted);
        }
        if !authority.remember_activity_connection(admitted) {
            return ServeEnd::server("activity_connection_fence_failed").finish(admitted);
        }
        if !authority.prepare_quest_callback_resume(&mut admitted).await {
            return ServeEnd::server("quest_resume_unavailable").finish(admitted);
        }
        if admitted.item_fence.is_some_and(|fence| {
            matches!(
                fence.runtime_scope,
                crate::foundation::RuntimeScopeRefV1::Instance { .. }
            ) && admitted
                .runtime_actor
                .is_none_or(|actor| actor.runtime_scope() != fence.runtime_scope)
        }) {
            // House services carry their own routing and view futures. Keep that
            // branch off the Channel connection's inline future, as for autowalk.
            admitted = match Box::pin(house_transfer::serve(
                stream,
                admitted,
                authority,
                root_authority,
                &mut server_messages,
                policy,
                &mut integrity,
                require_session_binding,
            ))
            .await
            {
                Ok(next) => next,
                Err(end) => return *end,
            };
        }
        let repaired_io = if admitted.first_entry == FirstEntryOutcome::AwaitingSpellRepair
            && admitted.runtime_actor.is_some()
        {
            match spell_repair::wait(stream, &mut admitted, authority, policy).await {
                Ok(io) => Some(io),
                Err(end) => return end.finish(admitted),
            }
        } else {
            None
        };
        let playable = matches!(
            admitted.first_entry,
            FirstEntryOutcome::Positioned | FirstEntryOutcome::Reconciled
        );
        let Some(actor) = admitted.runtime_actor.filter(|_| playable) else {
            record_join_progress("held_unplayable", admitted.first_entry);
            return hold_refused(stream, admitted, policy).await;
        };
        record_join_progress("baseline", admitted.first_entry);
        let Some(baseline) = authority.observe(actor).await else {
            record_join_progress("held_no_baseline", admitted.first_entry);
            return hold_refused(stream, admitted, policy).await;
        };
        // Keep the join snapshot and its temporary session state out of the
        // connection coroutine, which also carries the live command path.
        let mut session = match Box::pin(setup::prepare(
            stream,
            &mut admitted,
            authority,
            policy,
            actor,
            baseline,
            integrity,
        ))
        .await
        {
            Ok(session) => session,
            Err(end) => return end.finish(admitted),
        };
        if let Some((frames, liveness, cadence)) = repaired_io {
            session.frames = frames;
            session.liveness = liveness;
            session.cadence = cadence;
        }
        if let Some((frames, expected)) = activity_input.take() {
            session.frames = frames;
            if let Some(expected) = expected
                && let Err(end) =
                    activity_transfer::acknowledge(stream, authority, &mut session, expected).await
            {
                return end.finish(session.admitted);
            }
        }
        if let Some(cursor) = handoff_effect_start.take() {
            session.movement_effect_revision = cursor;
        }
        session.require_session_binding = require_session_binding;
        if server_messages.is_none() {
            server_messages =
                crate::gameplay_transport::raid_notices::ServerMessageConnection::join(
                    root_authority,
                    session.admitted,
                );
            if server_messages.is_none() {
                return ServeEnd::server("admitted_owner_unavailable").finish(session.admitted);
            }
        }
        let mut live_started = false;
        loop {
            if !authority.remember_activity_connection(session.admitted) {
                return ServeEnd::server("admitted_owner_unavailable").finish(session.admitted);
            }
            match observed(
                "activity_switch",
                activity_transfer::pending(stream, authority, &mut session),
            )
            .await
            {
                Ok(Some((next, cursor))) => {
                    require_session_binding = true;
                    handoff_effect_start = cursor;
                    activity_input = Some((
                        std::mem::take(&mut session.frames),
                        session.activity_ack.take(),
                    ));
                    admitted = next;
                    integrity = std::mem::take(&mut session.integrity);
                    break;
                }
                Ok(None) => {}
                Err(end) => return end.finish(session.admitted),
            }
            // A committed handoff has retired the source actor. Cross its
            // barrier before refreshing staff facts through the new owner.
            if tokio::time::Instant::now() >= next_staff_refresh {
                match observed(
                    "staff_refresh",
                    Box::pin(activity_transfer::refresh_staff(
                        stream,
                        authority,
                        &mut session,
                    )),
                )
                .await
                {
                    Ok(Some((next, cursor))) => {
                        require_session_binding = true;
                        handoff_effect_start = cursor;
                        activity_input = Some((
                            std::mem::take(&mut session.frames),
                            session.activity_ack.take(),
                        ));
                        admitted = next;
                        integrity = std::mem::take(&mut session.integrity);
                        break;
                    }
                    Ok(None) => {}
                    Err(end) => return end.finish(session.admitted),
                }
                next_staff_refresh = tokio::time::Instant::now() + VISIBILITY_REFRESH;
            }
            if let Err(end) =
                observed("analyser", analyser::emit(stream, authority, &mut session)).await
            {
                authority.player_trade_disconnected(session.admitted.game_session_id);
                return end.finish(authority.current_activity_connection(session.admitted));
            }
            let next = poll::next(stream, authority, &mut session).await;
            if !live_started {
                operator_event(&format!(
                    "event=connection_live_started phase={}",
                    next.phase()
                ));
                live_started = true;
            }
            // A member who did not issue the lever may have changed owner while
            // waiting for input. Its buffered source command never reaches the
            // destination's fresh command stream.
            match observed(
                "activity_switch",
                activity_transfer::pending(stream, authority, &mut session),
            )
            .await
            {
                Ok(Some((next, cursor))) => {
                    require_session_binding = true;
                    handoff_effect_start = cursor;
                    activity_input = Some((
                        std::mem::take(&mut session.frames),
                        session.activity_ack.take(),
                    ));
                    admitted = next;
                    integrity = std::mem::take(&mut session.integrity);
                    break;
                }
                Ok(None) => {}
                Err(end) => return end.finish(session.admitted),
            }
            if tokio::time::Instant::now() >= next_staff_refresh {
                match observed(
                    "staff_refresh",
                    Box::pin(activity_transfer::refresh_staff(
                        stream,
                        authority,
                        &mut session,
                    )),
                )
                .await
                {
                    Ok(Some((next, cursor))) => {
                        require_session_binding = true;
                        handoff_effect_start = cursor;
                        activity_input = Some((
                            std::mem::take(&mut session.frames),
                            session.activity_ack.take(),
                        ));
                        admitted = next;
                        integrity = std::mem::take(&mut session.integrity);
                        break;
                    }
                    Ok(None) => {}
                    Err(end) => return end.finish(session.admitted),
                }
                next_staff_refresh = tokio::time::Instant::now() + VISIBILITY_REFRESH;
            }
            let inbound_phase = next.phase();
            let received = observed(
                inbound_phase,
                Box::pin(inbound::receive(stream, authority, &mut session, next)),
            )
            .await;
            let input = match received {
                Ok(Some(input)) => input,
                Ok(None) => continue,
                Err(end) => {
                    authority.player_trade_disconnected(session.admitted.game_session_id);
                    return end.finish_with_phase(
                        authority.current_activity_connection(session.admitted),
                        Some(inbound_phase),
                    );
                }
            };
            if let Err(end) = observed(
                "command",
                Box::pin(command::execute(stream, authority, &mut session, input)),
            )
            .await
            {
                if let ServeEnd::Transferred(next) = end {
                    activity_input = Some((
                        std::mem::take(&mut session.frames),
                        session.activity_ack.take(),
                    ));
                    admitted = *next;
                    integrity = std::mem::take(&mut session.integrity);
                    break;
                }
                let error = match &end {
                    ServeEnd::Closed(error) => format!("{error:?}"),
                    ServeEnd::ControlLost(path) => format!("ControlLost({path})"),
                    ServeEnd::Server { cause, .. } => format!("ServerClose({cause})"),
                    ServeEnd::LoggedOut => "LoggedOut".to_owned(),
                    ServeEnd::Transferred(_) => "Transferred".to_owned(),
                    ServeEnd::Transport(end) => {
                        let (path, reason, kind) = end.fields();
                        format!("Transport({path},{reason},{kind:?})")
                    }
                };
                operator_event(&format!(
                    "event=serve_command_end error={error} next_command_id={}",
                    session.admitted.continuity.next_command_id,
                ));
                authority.player_trade_disconnected(session.admitted.game_session_id);
                return end.finish_with_phase(
                    authority.current_activity_connection(session.admitted),
                    Some("command"),
                );
            }
        }
    }
}

#[cfg(test)]
#[test]
fn serve_end_preserves_reset_io_errors_and_server_cause() -> Result<(), Box<dyn std::error::Error>>
{
    use super::super::tcp_tls::{FRAME_FAILURE, FrameFailure};
    use std::io::ErrorKind;
    let runtime = tokio::runtime::Builder::new_current_thread().build()?;
    runtime.block_on(FRAME_FAILURE.scope(std::cell::Cell::default(), async {
        FrameFailure::Read(ErrorKind::ConnectionReset).record();
        let ServeEnd::Transport(reset) = ServeEnd::server("state_projection_unavailable") else {
            return Err("the projection dropped the original RST".into());
        };
        assert_eq!(
            reset.fields(),
            ("read", "peer_reset", ErrorKind::ConnectionReset)
        );
        assert_eq!(
            TransportEnd::Write(ErrorKind::BrokenPipe).fields(),
            ("write", "write_error", ErrorKind::BrokenPipe)
        );
        assert_eq!(
            TransportEnd::Read(ErrorKind::InvalidData).fields(),
            ("read", "read_error", ErrorKind::InvalidData)
        );
        assert!(matches!(
            ServeEnd::server("command_outcome_unavailable"),
            ServeEnd::Server {
                cause: "command_outcome_unavailable",
                ..
            }
        ));
        Ok(())
    }))
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
