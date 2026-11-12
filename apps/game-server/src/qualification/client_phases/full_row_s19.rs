//! S19 player typing indicator, derived from the capability-233 owners
//! (`crates/protocol-oteryn/src/player_typing.rs`, dispatch `player_typing.rs`).
//! 1. Admit the node-boot character and require capability 233 to be selected.
//! 2. Send the typing command with `typing=true`; expect `Accepted` and a CHAT delta 4
//!    (`SessionEvent::PlayerTyping`) from the server naming the own actor (identity equal to the
//!    session's own entity identity, non-zero generation) with `typing=true`.
//! 3. Send `typing=false`; expect `Accepted` and a second server delta for the same speaker with
//!    `typing=false` and a later revision.
//! 4. Log out. The state is transient, so nothing durable is re-checked.
use super::*;
use oteryn_dev_client::client_runtime::player_typing::{
    PlayerTypingCommand, PlayerTypingOrigin, send,
};
use oteryn_dev_client::{DevClientSession, SessionEvent};
use oteryn_protocol_oteryn::player_typing::{CAPABILITY_PLAYER_TYPING_V1, TypingUpdate};

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = verdict::indexed_timeout(Duration::from_secs(60), verdict::observe(journey(clients))).await;
    let (status, detail) = match result {
        Ok(Ok(())) => ("PASS", "typing-on-off-deltas-from-server".to_owned()),
        Ok(Err(error)) => ("FAIL", compact(&error.to_string())),
        Err(_) => ("FAIL", "step=journey-observed=timeout".to_owned()),
    };
    evidence(&format!(
        "e2e_all scenario=S19.full status={status} seconds={:.3} evidence={detail}",
        started.elapsed().as_secs_f64()
    ));
}

fn compact(value: &str) -> String {
    let token: String = value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("-")
        .chars()
        .take(300)
        .collect();
    if token.is_empty() {
        "unknown".to_owned()
    } else {
        token
    }
}

/// The speaker must be the session's own actor and the state must be the one just requested.
fn judge(update: &TypingUpdate, own: Option<&[u8; 16]>, typing: bool) -> Result<(), String> {
    if update.typing != typing {
        return Err(format!(
            "step=typing observed=typing:{}-needed:{typing}",
            update.typing
        ));
    }
    match own {
        Some(identity) if *identity == update.speaker.identity => Ok(()),
        other => Err(format!(
            "step=typing observed=speaker-{:?}-own-{other:?}",
            update.speaker.identity
        )),
    }
}

async fn typing_step(
    client: &mut DevClientSession,
    typing: bool,
) -> TestResult<(TypingUpdate, u64)> {
    client.take_events();
    let reply = send(
        client.session_mut(),
        &PlayerTypingCommand {
            typing,
            origin: PlayerTypingOrigin::Manual,
        },
    )
    .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("step=typing-{typing} observed=status:{:?}", reply.status).into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        for event in client.take_events() {
            if let SessionEvent::PlayerTyping(delta) = event {
                return Ok((delta.value, delta.new_revision));
            }
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(
                format!("step=typing-{typing} observed=accepted-but-no-chat-delta-4").into(),
            );
        }
        client.service_liveness(Duration::from_millis(50)).await?;
    }
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let mut client = connect_world(clients, 0xc1).await?;
    if !client
        .selected_capabilities()
        .contains(&CAPABILITY_PLAYER_TYPING_V1)
    {
        return Err("step=admission observed=capability-233-not-selected".into());
    }
    let own = client
        .world_entities()
        .map(|entities| *entities.own_identity());
    let (on, on_revision) = typing_step(&mut client, true).await?;
    judge(&on, own.as_ref(), true)?;
    let (off, off_revision) = typing_step(&mut client, false).await?;
    judge(&off, own.as_ref(), false)?;
    if off.speaker != on.speaker || off_revision <= on_revision {
        return Err(format!(
            "step=typing-off observed=speaker-or-revision-changed-{on_revision}-{off_revision}"
        )
        .into());
    }
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("step=logout observed=refused".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use oteryn_protocol_oteryn::chat::ChatSpeaker;
    use std::num::NonZeroU64;

    fn update(typing: bool) -> TypingUpdate {
        TypingUpdate {
            speaker: ChatSpeaker {
                identity: [3; 16],
                generation: NonZeroU64::MIN,
            },
            typing,
        }
    }

    #[test]
    fn typing_update_must_name_own_actor_and_requested_state() {
        assert!(judge(&update(true), Some(&[3; 16]), true).is_ok());
        assert!(judge(&update(true), Some(&[4; 16]), true).is_err());
        assert!(judge(&update(true), None, true).is_err());
        assert!(judge(&update(false), Some(&[3; 16]), true).is_err());
    }

    #[test]
    fn compact_is_one_token() {
        assert!(!compact("a b\nc").contains(char::is_whitespace));
        assert_eq!(compact("  "), "unknown");
    }
}
