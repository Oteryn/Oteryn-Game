//! E2E S04c — public channels with mute.
//!
//! Player scenario derived from the S04c audit/matrix row and the native CHAT/public-channel
//! owner (`guild_chat/public_channels.rs`):
//! 1. Open World, English, Help, and Advertising with native `OpenRoom` commands; require an OK
//!    result and a room-set delta containing each room.
//! 2. Send a native room line in every open channel; require an OK result and the exact room,
//!    speaker, and text in a CHAT line delta.
//! 3. Send a second Advertising line immediately; require `Exhausted` with a positive wait and
//!    no matching CHAT line delta (the two-minute advertising refusal path).
//! 4. Close World; require an OK result and a room-set delta removing it, then try to speak there
//!    and require `RoomNotOpen` with no matching line delta.
//! 5. Reopen World and send distinct player room lines until the shared spam gate returns `Muted`
//!    with a positive wait; require every OK reply to have its exact line delta and the refused
//!    line to produce none.
//! 6. Log out and freshly admit the same provisioned character; require a new game session, the
//!    World room still open, and another World line still refused as `Muted` with a positive wait.

use super::*;
use super::{
    row_connect_character as connect_character,
    row_finish as row_logout,
};
use oteryn_dev_client::{ChatDisposition, ChatIntent, ChatLine, ChatRoom, DevClientSession};

const ROOMS: [ChatRoom; 4] = [
    ChatRoom::World,
    ChatRoom::English,
    ChatRoom::Help,
    ChatRoom::Advertising,
];

fn has_line(client: &DevClientSession, room: ChatRoom, text: &str) -> bool {
    client.chat_log().is_some_and(|log| {
        log.lines().any(|line| {
            matches!(line, ChatLine::Room { room: seen, speaker_name, text: seen_text }
                if *seen == room && !speaker_name.is_empty() && seen_text == text)
        })
    })
}

async fn wait_for(
    client: &mut DevClientSession,
    predicate: impl Fn(&DevClientSession) -> bool,
) -> TestResult {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while !predicate(client) {
        if tokio::time::Instant::now() >= deadline {
            return Err("step reply had no expected CHAT state delta before timeout".into());
        }
        client.service_liveness(Duration::from_millis(20)).await?;
        client.take_events();
    }
    Ok(())
}

async fn require_no_line(client: &mut DevClientSession, room: ChatRoom, text: &str) -> TestResult {
    let deadline = tokio::time::Instant::now() + Duration::from_millis(200);
    while tokio::time::Instant::now() < deadline {
        client.service_liveness(Duration::from_millis(20)).await?;
        client.take_events();
        if has_line(client, room, text) {
            return Err("refused public-channel command produced a CHAT line delta".into());
        }
    }
    Ok(())
}

async fn open(client: &mut DevClientSession, room: ChatRoom) -> TestResult {
    let reply = client.chat(&ChatIntent::OpenRoom(room)).await?;
    if reply.disposition != ChatDisposition::Ok {
        return Err(format!(
            "step 1 open {room:?}: observed reply {reply:?}; server must admit the public room"
        )
        .into());
    }
    wait_for(client, |client| {
        client
            .chat_log()
            .is_some_and(|log| log.rooms().contains(room))
    })
    .await
}

async fn send_line(client: &mut DevClientSession, room: ChatRoom, text: &str) -> TestResult {
    let reply = client
        .chat(&ChatIntent::Room {
            room,
            text: text.to_owned(),
        })
        .await?;
    if reply.disposition != ChatDisposition::Ok {
        return Err(format!("step 2 send {room:?}: observed reply {reply:?}; server must relay an admitted public line").into());
    }
    wait_for(client, |client| has_line(client, room, text)).await
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    row_complete(async {
    let row_deadline = tokio::time::Instant::now() + Duration::from_secs(90);
    let mut client = connect_character(clients, 0xc4, 0).await?;
    let mut row_needs_logout_0 = true;
    let row_result: TestResult<_> = tokio::time::timeout_at(row_deadline, async {
    for room in ROOMS {
        open(&mut client, room).await?;
    }
    for (room, text) in [
        (ChatRoom::World, "S04c world"),
        (ChatRoom::English, "S04c english"),
        (ChatRoom::Help, "S04c help"),
        (ChatRoom::Advertising, "S04c advertising"),
    ] {
        send_line(&mut client, room, text).await?;
    }

    let refused_text = "S04c advertising refused";
    let reply = client
        .chat(&ChatIntent::Room {
            room: ChatRoom::Advertising,
            text: refused_text.into(),
        })
        .await?;
    if reply.disposition != ChatDisposition::Exhausted || reply.wait_seconds == 0 {
        return Err(format!("step 3 advertising refusal: observed reply {reply:?}, line_delta={}; server must return EXHAUSTED with wait and no line", has_line(&client, ChatRoom::Advertising, refused_text)).into());
    }
    require_no_line(&mut client, ChatRoom::Advertising, refused_text).await?;

    let reply = client.chat(&ChatIntent::CloseRoom(ChatRoom::World)).await?;
    if reply.disposition != ChatDisposition::Ok {
        return Err(format!("step 4 close World: observed reply {reply:?}").into());
    }
    wait_for(&mut client, |client| {
        client
            .chat_log()
            .is_some_and(|log| !log.rooms().contains(ChatRoom::World))
    })
    .await?;
    let closed_text = "S04c closed refusal";
    let reply = client
        .chat(&ChatIntent::Room {
            room: ChatRoom::World,
            text: closed_text.into(),
        })
        .await?;
    if reply.disposition != ChatDisposition::RoomNotOpen {
        return Err(format!("step 4 closed-room refusal: observed reply {reply:?}, line_delta={}; server must return ROOM_NOT_OPEN and no line", has_line(&client, ChatRoom::World, closed_text)).into());
    }
    require_no_line(&mut client, ChatRoom::World, closed_text).await?;
    open(&mut client, ChatRoom::World).await?;

    let mut muted = None;
    for index in 0..32 {
        let text = format!("S04c spam {index}");
        let reply = client
            .chat(&ChatIntent::Room {
                room: ChatRoom::World,
                text: text.clone(),
            })
            .await?;
        match reply.disposition {
            ChatDisposition::Ok => wait_for(&mut client, |client| has_line(client, ChatRoom::World, &text)).await?,
            ChatDisposition::Muted if reply.wait_seconds > 0 => { muted = Some((text, reply)); break; }
            _ => return Err(format!("step 5 spam attempt {index}: observed reply {reply:?}; server must relay or return MUTED with wait").into()),
        }
    }
    let (muted_text, muted_reply) = muted.ok_or("step 5: 32 real World lines never reached the spam mute; server must expose the shared limiter")?;
    require_no_line(&mut client, ChatRoom::World, &muted_text)
        .await
        .map_err(|error| format!("step 5 muted reply {muted_reply:?}: {error}"))?;

    let old_session = client.join_snapshot().game_session_id;
    if client.row_logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("step 6 logout: server refused native logout".into());
    }
    row_needs_logout_0 = false;
    if row_needs_logout_0 { tokio::time::timeout(Duration::from_secs(10), row_logout(&mut client)).await??; row_needs_logout_0 = false; }
    let mut client = connect_character(clients, 0xc5, 0).await?;
    let row_needs_logout_1 = true;
    let row_result: TestResult<_> = tokio::time::timeout_at(row_deadline, async {
    if client.join_snapshot().game_session_id == old_session
        || !client
            .chat_log()
            .is_some_and(|log| log.rooms().contains(ChatRoom::World))
    {
        return Err(
            "step 6 fresh admission: new session or retained World room state missing".into(),
        );
    }
    let text = "S04c relog muted";
    let reply = client
        .chat(&ChatIntent::Room {
            room: ChatRoom::World,
            text: text.into(),
        })
        .await?;
    if reply.disposition != ChatDisposition::Muted || reply.wait_seconds == 0 {
        return Err(format!("step 6 fresh-admission mute: observed reply {reply:?}, line_delta={}; server must retain mute and refuse without a line", has_line(&client, ChatRoom::World, text)).into());
    }
    require_no_line(&mut client, ChatRoom::World, text).await?;
    Ok(())

    }).await.unwrap_or_else(|error| Err(error.into()));
    if row_needs_logout_1 { tokio::time::timeout(Duration::from_secs(10), row_logout(&mut client)).await??; }
    row_result

    })
    .await
    .unwrap_or_else(|error| Err(error.into()));
    if row_needs_logout_0 {
        tokio::time::timeout(Duration::from_secs(10), row_logout(&mut client)).await??;
    }
    row_result

    }).await?
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = row_complete(journey(clients)).await;
    match result {
        Ok(Ok(())) => evidence(&format!(
            "e2e_all scenario=S04c.full status=PASS seconds={:.3}",
            started.elapsed().as_secs_f64()
        )),
        Ok(Err(error)) => {
            evidence(&format!("S04C_GAP {error}"));
            evidence(&format!(
                "e2e_all scenario=S04c.full status=FAIL seconds={:.3} evidence=observed-runtime-failure",
                started.elapsed().as_secs_f64()
            ));
        }
        Err(_) => {
            evidence(
                "S04C_GAP step=journey observed=timeout server-needed=bounded-public-chat-replies",
            );
            evidence(&format!(
                "e2e_all scenario=S04c.full status=FAIL seconds={:.3} evidence=journey-timeout",
                started.elapsed().as_secs_f64()
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn scenario_covers_all_four_public_rooms() {
        assert_eq!(ROOMS.len(), 4);
        for room in [
            ChatRoom::World,
            ChatRoom::English,
            ChatRoom::Help,
            ChatRoom::Advertising,
        ] {
            assert!(ROOMS.contains(&room));
        }
    }
}
