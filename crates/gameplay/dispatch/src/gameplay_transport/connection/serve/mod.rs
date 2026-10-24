//! Ordered private phases of an admitted actor connection.

use super::*;

mod command;
mod inbound;
mod poll;
mod refresh;
mod setup;
mod snapshot;
use command::CommandOutcome;
mod effects;
mod outbound;
mod result;
mod route;
mod use_effects;

// Fields drop in the reverse order of the original connection locals. In particular,
// release the pacing claim before dropping the session command stream (which locks ingress).
struct ServeState {
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
    party_view: crate::gameplay_transport::party_command::SessionPartyView,
    notices_selected: bool,
    overlay_served: bool,
    revision: u64,
    actor: ExactActorRef,
    generation: u64,
    admitted: AdmittedSession,
    policy: LivenessPolicy,
}

enum ServeEnd {
    Disconnected,
    ControlLost,
    LoggedOut,
    Closed(FoundationProtocolError),
}

impl ServeEnd {
    fn finish(self, admitted: AdmittedSession) -> ConnectionEnd {
        match self {
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
    let generation = admitted.continuity.connection_generation;
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
    let revision = admitted.continuity.spatial_revision;
    let mut session = match setup::prepare(
        stream,
        &mut admitted,
        authority,
        policy,
        actor,
        baseline,
        (generation, revision),
    )
    .await
    {
        Ok(session) => session,
        Err(end) => return end.finish(admitted),
    };
    loop {
        let next = poll::next(stream, &mut session).await;
        let input = match inbound::receive(stream, authority, &mut session, next).await {
            Ok(Some(input)) => input,
            Ok(None) => continue,
            Err(end) => return end.finish(session.admitted),
        };
        if let Err(end) = command::execute(stream, authority, &mut session, input).await {
            return end.finish(session.admitted);
        }
    }
}
