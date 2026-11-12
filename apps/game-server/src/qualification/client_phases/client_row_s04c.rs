//! S04c client replay capture: public channels with mute. Character one opens World and
//! Advertising, speaks, is refused a second Advertising line (`Exhausted`), closes World and is
//! refused there (`RoomNotOpen`), reopens it and spams until `Muted`. `Session::chat` decodes the
//! `CommandResult` into `ChatOutcome` and the CHAT deltas into `ChatLog`, so the raw bytes are not
//! retained; the server-observed outcomes are written to `s04c-expected.txt` (one
//! `step status disposition wait speaker` line each) and the replay re-encodes them with the
//! production chat encoders. No `.bin` file is written or claimed as raw.
use super::*;
use oteryn_dev_client::{ChatDisposition, ChatIntent, ChatLine, ChatRoom, DevClientSession};

fn capture(name: &str, bytes: &[u8]) {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return;
    };
    if let Err(error) = std::fs::write(Path::new(&root).join(name), bytes) {
        evidence(&format!("S04C_CLIENT_CAPTURE_WRITE_ERROR file={name} error={error}"));
    }
}

fn speaker(client: &DevClientSession, room: ChatRoom, text: &str) -> Option<String> {
    client.chat_log()?.lines().find_map(|line| match line {
        ChatLine::Room { room: seen, speaker_name, text: seen_text }
            if *seen == room && seen_text == text => Some(speaker_name.clone()),
        _ => None,
    })
}

async fn settle(client: &mut DevClientSession) -> TestResult {
    for _ in 0..10 {
        client.service_liveness(Duration::from_millis(20)).await?;
        client.take_events();
    }
    Ok(())
}

async fn say(
    client: &mut DevClientSession,
    room: ChatRoom,
    text: &str,
    expected: ChatDisposition,
) -> TestResult<(CommandStatus, u32)> {
    let reply = client
        .chat(&ChatIntent::Room { room, text: text.to_owned() })
        .await?;
    if reply.disposition != expected {
        return Err(format!("step=say {room:?} observed {reply:?}, expected {expected:?}").into());
    }
    settle(client).await?;
    Ok((reply.status, reply.wait_seconds))
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let mut client = connect_character(clients, 0x63, 1).await?;
    let mut expected = String::new();
    let mut note = |step: &str, status: CommandStatus, disposition: ChatDisposition, wait: u32, who: &str| {
        expected.push_str(&format!("{step}\t{status:?}\t{disposition:?}\t{wait}\t{who}\n"));
    };
    for (step, intent, room) in [
        ("open-world", ChatIntent::OpenRoom(ChatRoom::World), ChatRoom::World),
        ("open-advertising", ChatIntent::OpenRoom(ChatRoom::Advertising), ChatRoom::Advertising),
    ] {
        let reply = client.chat(&intent).await?;
        if reply.disposition != ChatDisposition::Ok {
            return Err(format!("step={step} observed {reply:?}").into());
        }
        settle(&mut client).await?;
        if !client.chat_log().is_some_and(|log| log.rooms().contains(room)) {
            return Err(format!("step={step} room delta missing").into());
        }
        note(step, reply.status, reply.disposition, reply.wait_seconds, "-");
    }
    let (status, wait) = say(&mut client, ChatRoom::World, "S04c-client world", ChatDisposition::Ok).await?;
    let who = speaker(&client, ChatRoom::World, "S04c-client world").ok_or("step=line-world no CHAT line delta")?;
    note("line-world", status, ChatDisposition::Ok, wait, &who);
    let (status, wait) = say(&mut client, ChatRoom::Advertising, "S04c-client ad", ChatDisposition::Ok).await?;
    let who = speaker(&client, ChatRoom::Advertising, "S04c-client ad").ok_or("step=line-advertising no CHAT line delta")?;
    note("line-advertising", status, ChatDisposition::Ok, wait, &who);
    let (status, wait) = say(&mut client, ChatRoom::Advertising, "S04c-client ad again", ChatDisposition::Exhausted).await?;
    if wait == 0 || speaker(&client, ChatRoom::Advertising, "S04c-client ad again").is_some() {
        return Err("step=exhausted missing wait or produced a line".into());
    }
    note("exhausted-advertising", status, ChatDisposition::Exhausted, wait, "-");
    let close = client.chat(&ChatIntent::CloseRoom(ChatRoom::World)).await?;
    settle(&mut client).await?;
    if close.disposition != ChatDisposition::Ok || client.chat_log().is_some_and(|log| log.rooms().contains(ChatRoom::World)) {
        return Err(format!("step=close-world observed {close:?}").into());
    }
    note("close-world", close.status, close.disposition, close.wait_seconds, "-");
    let (status, wait) = say(&mut client, ChatRoom::World, "S04c-client closed", ChatDisposition::RoomNotOpen).await?;
    note("closed-world", status, ChatDisposition::RoomNotOpen, wait, "-");
    let reopen = client.chat(&ChatIntent::OpenRoom(ChatRoom::World)).await?;
    settle(&mut client).await?;
    if reopen.disposition != ChatDisposition::Ok {
        return Err(format!("step=reopen-world observed {reopen:?}").into());
    }
    note("reopen-world", reopen.status, reopen.disposition, reopen.wait_seconds, "-");
    let mut muted = None;
    for index in 0..32 {
        let text = format!("S04c-client spam {index}");
        let reply = client
            .chat(&ChatIntent::Room { room: ChatRoom::World, text: text.clone() })
            .await?;
        settle(&mut client).await?;
        match reply.disposition {
            ChatDisposition::Ok => {}
            ChatDisposition::Muted if reply.wait_seconds > 0 => {
                muted = Some((text, reply));
                break;
            }
            _ => return Err(format!("step=spam {index} observed {reply:?}").into()),
        }
    }
    let (text, reply) = muted.ok_or("step=muted 32 lines never reached the mute")?;
    if speaker(&client, ChatRoom::World, &text).is_some() {
        return Err("step=muted refused line produced a CHAT delta".into());
    }
    note("muted-world", reply.status, reply.disposition, reply.wait_seconds, "-");
    let _ = client.session_mut().logout().await;
    capture("s04c-expected.txt", expected.as_bytes());
    Ok(format!("steps={}", expected.lines().count()))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    if std::env::var_os("E2E_ALL_CAPTURE_DIR").is_none() {
        return;
    }
    match verdict::indexed_timeout(Duration::from_secs(90), verdict::observe(journey(clients))).await {
        Ok(Ok(observed)) => evidence(&format!(
            "e2e_all scenario=S04c.client status=CAPTURED evidence={observed}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=S04c.client status=FAIL evidence={}",
            error.to_string().replace(' ', "_")
        )),
        Err(_) => evidence("e2e_all scenario=S04c.client status=FAIL evidence=timeout"),
    }
}
