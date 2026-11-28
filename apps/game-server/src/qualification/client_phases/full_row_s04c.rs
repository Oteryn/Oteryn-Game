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
//! 6. Log out and freshly admit the same character. Require a new game session with its
//!    own room membership. Reopen World and verify that the fresh limiter admits speech.

use super::*;
use oteryn_dev_client::{ChatDisposition, ChatIntent, ChatLine, ChatRoom, DevClientSession};

const ROOMS: [ChatRoom; 4] = [
    ChatRoom::World,
    ChatRoom::English,
    ChatRoom::Help,
    ChatRoom::Advertising,
];

pub(super) async fn connect_speaker(
    clients: &SeamClients<'_>,
    row: &str,
    tag: u8,
    index: usize,
) -> TestResult<DevClientSession> {
    const LEVEL: u32 = 100;
    character_prep::declare_seeded(
        row,
        "level-100-sorcerer",
        "Public room speech requires level 20 or Premium, and leveling is outside this journey.",
    );
    let mut client = connect_character(clients, tag, index).await?;
    let old_session = client.join_snapshot().game_session_id;
    request_fixture_control(
        uuid_text(old_session.as_bytes()),
        "--profile",
        "sorcerer".into(),
    )
    .await?;
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("speaker fixture logout was refused".into());
    }
    drop(client);

    let mut client = connect_character(clients, tag, index).await?;
    if client.join_snapshot().game_session_id == old_session {
        return Err("speaker fixture reused its original game session".into());
    }
    let level = full_e2ef7::player_level(&mut client).await?;
    let (max_health, max_mana, _) = oteryn_gameplay_characters::vitals::maxima(
        "sorcerer",
        LEVEL,
        oteryn_protocol_oteryn::actor_spell::MAX_VITAL_POOL,
        oteryn_protocol_oteryn::actor_spell::MAX_SOUL,
    )
    .ok_or("speaker fixture source vitals are missing")?;
    if level != LEVEL
        || !client.actor_vitals().is_some_and(|vitals| {
            vitals.max_health == max_health && vitals.max_mana == max_mana
        })
    {
        return Err(format!(
            "speaker fixture expected level {LEVEL} and maxima {max_health}/{max_mana}, observed level {level} and {:?}",
            client.actor_vitals()
        ).into());
    }
    Ok(client)
}

fn has_line(client: &DevClientSession, room: ChatRoom, text: &str) -> bool {
    client.chat_log().is_some_and(|log| {
        log.lines().any(|line| {
            matches!(line, ChatLine::Room { room: seen, speaker_name, text: seen_text }
                if *seen == room && !speaker_name.is_empty() && seen_text == text)
        })
    })
}

pub(super) async fn wait_for(
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

pub(super) async fn require_no_line(client: &mut DevClientSession, room: ChatRoom, text: &str) -> TestResult {
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

fn same_chat_session(before: &[u8; 16], after: &[u8; 16]) -> bool {
    before == after
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let mut client = connect_speaker(clients, "S04c.full", 0xc4, 0).await?;
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
        .map_err(|error| { verdict::record_error(verdict::IndexedError::as_error(&error)); verdict::context(format!("step 5 muted reply {muted_reply:?}: {error}"), error) })?;

    let old_session = client.join_snapshot().game_session_id;
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("step 6 logout: server refused native logout".into());
    }
    drop(client);
    let mut client = connect_character(clients, 0xc5, 0).await?;
    if same_chat_session(
        old_session.as_bytes(),
        client.join_snapshot().game_session_id.as_bytes(),
    ) {
        return Err("step 6 fresh admission: reused the muted game session".into());
    }
    if client
        .chat_log()
        .is_none_or(|log| log.rooms().contains(ChatRoom::World))
    {
        return Err(
            "step 6 fresh admission: expected a fresh CHAT room set with World closed".into(),
        );
    }
    open(&mut client, ChatRoom::World).await?;
    send_line(&mut client, ChatRoom::World, "S04c fresh-session message").await?;
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("step 6 logout: server refused fresh-session logout".into());
    }
    Ok(())
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = verdict::indexed_timeout(Duration::from_secs(90), verdict::observe(journey(clients))).await;
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
    fn fresh_chat_session_is_distinct_from_the_muted_session() {
        assert!(same_chat_session(&[1; 16], &[1; 16]));
        assert!(!same_chat_session(&[1; 16], &[2; 16]));
    }

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
