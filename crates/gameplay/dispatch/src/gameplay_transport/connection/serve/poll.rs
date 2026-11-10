//! Cancel-safe polling with a bounded input turn after periodic work.

use super::*;

pub(super) enum Next {
    Frame(std::io::Result<Vec<u8>>),
    /// A frame read while a step waited in the buffer, or while a logout ran.
    Held(Vec<u8>, bool, tokio::time::Instant),
    StepDue,
    Probe,
    ServerMessage,
    Serene,
    QuestLog,
    Chat,
    Visibility,
    Combat,
    Chase,
    TargetMove,
    Npc,
}

impl Next {
    /// A closed diagnostic label, independent of client frames and database errors.
    pub(super) const fn phase(&self) -> &'static str {
        match self {
            Self::Frame(Ok(_)) => "frame",
            Self::Frame(Err(_)) => "frame_error",
            Self::Held(..) => "held",
            Self::StepDue => "step_due",
            Self::Probe => "probe",
            Self::ServerMessage => "server_message",
            Self::Serene => "serene",
            Self::QuestLog => "quest_log",
            Self::Chat => "chat",
            Self::Visibility => "visibility",
            Self::Combat => "combat",
            Self::Chase => "chase",
            Self::TargetMove => "target_move",
            Self::Npc => "npc",
        }
    }
}

pub(super) async fn next<S: AsyncRead + AsyncWrite + Unpin, A: FreshAdmissionAuthority>(
    stream: &mut S,
    authority: &A,
    session: &mut ServeState,
) -> Next {
    // All futures are cancel-safe: the frame reader keeps partial bytes and a dropped
    // interval tick is not consumed. After periodic work, give ready input one turn
    // before timers: a slow refresh can otherwise remain due and starve even probe ACKs.
    // Only one input turn is granted, so flooding cannot starve the probe cadence.
    let held_frame = if session.buffered.is_none() {
        session.held.pop_front()
    } else {
        None
    };
    if let Some((frame, behind_step, received_at)) = held_frame {
        session.input_after_refresh = false;
        Next::Held(frame, behind_step, received_at)
    } else {
        let due = session.buffered.as_ref().map(|step| step.due);
        let reading = session.held.len() < MAX_FRAMES_HELD_BEHIND_A_BUFFERED_STEP;
        let mut step_due = std::pin::pin!(async {
            match due {
                Some(due) => tokio::time::sleep_until(due).await,
                None => std::future::pending().await,
            }
        });
        let mut read = std::pin::pin!(session.frames.next(stream));
        let mut tick = std::pin::pin!(session.cadence.tick());
        let mut server_message =
            std::pin::pin!(authority.server_messages_changed(session.admitted));
        let mut serene_tick = std::pin::pin!(async {
            match session.serene.as_mut() {
                Some(timer) => timer.tick().await,
                None => std::future::pending().await,
            }
        });
        let mut quest_log_tick = std::pin::pin!(async {
            match session.quest_log_refresh.as_mut() {
                Some(refresh) => refresh.tick().await,
                None => std::future::pending().await,
            }
        });
        let mut chat_tick = std::pin::pin!(async {
            match session.chat_refresh.as_mut() {
                Some(refresh) => refresh.tick().await,
                None => std::future::pending().await,
            }
        });
        let mut visibility_tick = std::pin::pin!(async {
            match session.visibility_refresh.as_mut() {
                Some(refresh) => refresh.tick().await,
                None => std::future::pending().await,
            }
        });
        let mut npc_tick = std::pin::pin!(async {
            match session.npc_refresh.as_mut() {
                Some(refresh) => refresh.tick().await,
                None => std::future::pending().await,
            }
        });
        let mut combat_tick = std::pin::pin!(async {
            match session.combat_refresh.as_mut() {
                Some(refresh) => refresh.tick().await,
                None => std::future::pending().await,
            }
        });
        let mut chase_tick = std::pin::pin!(async {
            match session.chase_due {
                Some(due) => tokio::time::sleep_until(due).await,
                None => std::future::pending().await,
            }
        });
        let mut target_tick = std::pin::pin!(async {
            match session.target_walk.as_ref().map(|walk| walk.due) {
                Some(due) => tokio::time::sleep_until(due).await,
                None => std::future::pending().await,
            }
        });
        let input_after_refresh = session.input_after_refresh;
        let mut refresh_cursor = session.refresh_cursor;
        let mut input = |context: &mut std::task::Context<'_>| {
            if step_due.as_mut().poll(context).is_ready() {
                return Some(Next::StepDue);
            }
            // CHASE-1: one ready client frame is read before a due chase step, so a client
            // step wins over the chase (RANGED-0 §8) and is never paced behind it; after
            // that frame the due chase step runs before the next one.
            let target_ready = target_tick.as_mut().poll(context).is_ready();
            if target_ready && session.chase_yielded {
                return Some(Next::TargetMove);
            }
            let chase_ready = chase_tick.as_mut().poll(context).is_ready();
            if chase_ready && session.chase_yielded {
                return Some(Next::Chase);
            }
            if reading && let std::task::Poll::Ready(read) = read.as_mut().poll(context) {
                session.chase_yielded = chase_ready || target_ready;
                return Some(Next::Frame(read));
            }
            if target_ready {
                return Some(Next::TargetMove);
            }
            if chase_ready {
                return Some(Next::Chase);
            }
            None
        };
        let next = std::future::poll_fn(|context| {
            if input_after_refresh && let Some(next) = input(context) {
                return std::task::Poll::Ready(next);
            }
            if tick.as_mut().poll(context).is_ready() {
                return std::task::Poll::Ready(Next::Probe);
            }
            if server_message.as_mut().poll(context).is_ready() {
                return std::task::Poll::Ready(Next::ServerMessage);
            }
            // A refresh may outlast its own period. Give every ready periodic owner
            // a turn before selecting that same owner again; input and probes retain
            // their existing precedence.
            for offset in 0..6 {
                let owner = (refresh_cursor + offset) % 6;
                let ready = match owner {
                    0 => serene_tick.as_mut().poll(context).is_ready(),
                    1 => quest_log_tick.as_mut().poll(context).is_ready(),
                    2 => chat_tick.as_mut().poll(context).is_ready(),
                    3 => visibility_tick.as_mut().poll(context).is_ready(),
                    4 => combat_tick.as_mut().poll(context).is_ready(),
                    _ => npc_tick.as_mut().poll(context).is_ready(),
                };
                if ready {
                    refresh_cursor = (owner + 1) % 6;
                    return std::task::Poll::Ready(match owner {
                        0 => Next::Serene,
                        1 => Next::QuestLog,
                        2 => Next::Chat,
                        3 => Next::Visibility,
                        4 => Next::Combat,
                        _ => Next::Npc,
                    });
                }
            }
            if let Some(next) = input(context) {
                return std::task::Poll::Ready(next);
            }
            std::task::Poll::Pending
        })
        .await;
        session.refresh_cursor = refresh_cursor;
        session.input_after_refresh = matches!(
            next,
            Next::Probe
                | Next::ServerMessage
                | Next::Serene
                | Next::QuestLog
                | Next::Chat
                | Next::Visibility
                | Next::Combat
                | Next::Npc
        );
        next
    }
}
