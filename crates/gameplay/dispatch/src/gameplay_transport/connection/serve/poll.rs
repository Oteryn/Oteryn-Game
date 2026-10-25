//! Cancel-safe input polling; branch priority is deliberately unchanged.

use super::*;

pub(super) enum Next {
    Frame(std::io::Result<Vec<u8>>),
    /// A frame read while a step waited in the buffer, or while a logout ran.
    Held(Vec<u8>, bool),
    StepDue,
    Probe,
    Serene,
    QuestLog,
    Chat,
    Visibility,
    Combat,
    Chase,
    TargetMove,
    Npc,
}

pub(super) async fn next<S: AsyncRead + AsyncWrite + Unpin>(
    stream: &mut S,
    session: &mut ServeState,
) -> Next {
    // All futures are cancel-safe: the frame reader keeps partial bytes and a dropped
    // interval tick is not consumed. The ticks are polled first so a client that keeps
    // frames flowing cannot starve a cadence; each is ready at most once per interval.
    let held_frame = if session.buffered.is_none() {
        session.held.pop_front()
    } else {
        None
    };
    if let Some((frame, behind_step)) = held_frame {
        Next::Held(frame, behind_step)
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
        std::future::poll_fn(|context| {
            if tick.as_mut().poll(context).is_ready() {
                return std::task::Poll::Ready(Next::Probe);
            }
            if serene_tick.as_mut().poll(context).is_ready() {
                return std::task::Poll::Ready(Next::Serene);
            }
            if quest_log_tick.as_mut().poll(context).is_ready() {
                return std::task::Poll::Ready(Next::QuestLog);
            }
            if chat_tick.as_mut().poll(context).is_ready() {
                return std::task::Poll::Ready(Next::Chat);
            }
            if visibility_tick.as_mut().poll(context).is_ready() {
                return std::task::Poll::Ready(Next::Visibility);
            }
            if combat_tick.as_mut().poll(context).is_ready() {
                return std::task::Poll::Ready(Next::Combat);
            }
            if npc_tick.as_mut().poll(context).is_ready() {
                return std::task::Poll::Ready(Next::Npc);
            }
            if step_due.as_mut().poll(context).is_ready() {
                return std::task::Poll::Ready(Next::StepDue);
            }
            // CHASE-1: one ready client frame is read before a due chase step, so a client
            // step wins over the chase (RANGED-0 §8) and is never paced behind it; after
            // that frame the due chase step runs before the next one.
            let target_ready = target_tick.as_mut().poll(context).is_ready();
            if target_ready && session.chase_yielded {
                return std::task::Poll::Ready(Next::TargetMove);
            }
            let chase_ready = chase_tick.as_mut().poll(context).is_ready();
            if chase_ready && session.chase_yielded {
                return std::task::Poll::Ready(Next::Chase);
            }
            if reading && let std::task::Poll::Ready(read) = read.as_mut().poll(context) {
                session.chase_yielded = chase_ready || target_ready;
                return std::task::Poll::Ready(Next::Frame(read));
            }
            if target_ready {
                return std::task::Poll::Ready(Next::TargetMove);
            }
            if chase_ready {
                return std::task::Poll::Ready(Next::Chase);
            }
            std::task::Poll::Pending
        })
        .await
    }
}
