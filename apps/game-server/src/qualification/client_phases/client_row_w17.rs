//! W17 client replay capture: the player opens the Svargrond catalogue and status, sends an
//! out-of-state Advance (refused with `WrongState`), then asks the Tibiadrome for its status and
//! tries to enter it. The raw `CommandResult` payloads the server sent are written byte for byte
//! (`Session::gated_command`); the native arena panel consumes them through `apply_result`
//! (`crates/client-ui/src/features/0170-arena/protocol.rs`, `tests/e2e_w17.rs`).
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::arena::{
    self as wire, ArenaAction, ArenaDisposition, DromeAction, DromeResult,
};

fn capture(name: &str, bytes: &[u8]) -> TestResult {
    use std::io::Write;
    let root = required("E2E_ALL_CAPTURE_DIR")?;
    std::fs::File::create_new(Path::new(&root).join(name))?.write_all(bytes)?;
    Ok(())
}

async fn request(
    client: &mut DevClientSession,
    kind: u32,
    payload: &[u8],
    step: &str,
) -> TestResult<Vec<u8>> {
    let reply = client
        .session_mut()
        .gated_command(wire::CAPABILITY_ARENA_V1, kind, payload)
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("step={step} status={:?} payload={:?}", reply.status, reply.payload).into());
    }
    let mut bytes = Vec::new();
    bytes.extend_from_slice(client.join_snapshot().game_session_id.as_bytes());
    bytes.extend_from_slice(&reply.command_id.to_be_bytes());
    bytes.extend_from_slice(&reply.server_sequence.to_be_bytes());
    bytes.extend_from_slice(&(reply.status as u32).to_be_bytes());
    bytes.extend_from_slice(&reply.payload);
    capture(&format!("w17-{step}.bin"), &bytes)?;
    Ok(reply.payload)
}

async fn action(client: &mut DevClientSession, action: ArenaAction, step: &str) -> TestResult<Vec<u8>> {
    let payload = wire::encode_match_request(action).map_err(|error| format!("step={step} encode: {error:?}"))?;
    request(client, wire::COMMAND_TYPE_ARENA_MATCH, &payload, step).await
}

fn state_text(result: &wire::ArenaResult) -> String {
    let s = result.state;
    format!(
        "{:?}|{}|{}|{}|{}|{}|{},{},{}",
        result.disposition,
        s.completed_rank,
        s.active_rank,
        s.pit,
        s.phase,
        s.remaining_seconds,
        s.claimed_masks[0],
        s.claimed_masks[1],
        s.claimed_masks[2]
    )
}

fn drome_text(result: &DromeResult) -> String {
    format!(
        "{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
        result.disposition,
        result.level,
        result.start,
        result.points,
        result.phase,
        result.wave,
        result.remaining,
        result.cooldown,
        result.rank,
        result.potions,
        result.rotation_ends
    )
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let mut client = connect_world(clients, 0x8d).await?;
    if !client.selected_capabilities().contains(&wire::CAPABILITY_ARENA_V1) {
        return Err("step=capability arena-v1 not selected".into());
    }
    let catalogue = request(&mut client, wire::COMMAND_TYPE_ARENA_CATALOGUE, &[], "catalogue").await?;
    let challenges = wire::decode_catalogue(&catalogue).map_err(|error| format!("step=catalogue decode: {error:?}"))?;
    if challenges != wire::SVARGROND_CHALLENGES {
        return Err(format!("step=catalogue observed={challenges:?}").into());
    }
    let status = action(&mut client, ArenaAction::Status, "status").await?;
    let initial = wire::decode_match_result(&status).map_err(|error| format!("step=status decode: {error:?}"))?;
    if initial.disposition != ArenaDisposition::Applied {
        return Err(format!("step=status observed={initial:?}").into());
    }
    let advance = action(&mut client, ArenaAction::Advance, "advance").await?;
    let refused = wire::decode_match_result(&advance).map_err(|error| format!("step=advance decode: {error:?}"))?;
    if refused.disposition != ArenaDisposition::WrongState || refused.state != initial.state {
        return Err(format!("step=advance observed={refused:?} before={initial:?}").into());
    }
    let drome_status = action(&mut client, ArenaAction::Drome(DromeAction::Status), "drome-status").await?;
    let before = DromeResult::decode(&drome_status).map_err(|error| format!("step=drome-status decode: {error:?}"))?;
    let drome_enter = action(&mut client, ArenaAction::Drome(DromeAction::Enter), "drome-enter").await?;
    let entered = DromeResult::decode(&drome_enter).map_err(|error| format!("step=drome-enter decode: {error:?}"))?;
    if entered != (DromeResult { disposition: 3, ..before }) {
        return Err(format!("step=drome-enter before={before:?} observed={entered:?}").into());
    }
    let drome_after = action(&mut client, ArenaAction::Drome(DromeAction::Status), "drome-after-refusal").await?;
    let after = DromeResult::decode(&drome_after)
        .map_err(|error| format!("step=drome-after-refusal decode: {error:?}"))?;
    if after != before {
        return Err(format!("step=drome-after-refusal before={before:?} observed={after:?}").into());
    }
    let mut expected = String::new();
    for challenge in &challenges {
        expected.push_str(&format!(
            "challenge={:?}|{}|{}|{}\n",
            challenge.rank, challenge.fee, challenge.rooms, challenge.room_seconds
        ));
    }
    expected.push_str(&format!(
        "status={}\nadvance={}\ndrome_status={}\ndrome_enter={}\n",
        state_text(&initial),
        state_text(&refused),
        drome_text(&before),
        drome_text(&entered)
    ));
    let session = client.join_snapshot().game_session_id;
    let logout = client.session_mut().logout().await?;
    if logout.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("step=logout observed={logout:?}").into());
    }
    capture("w17-expected.txt", expected.as_bytes())?;
    Ok(format!(
        "session={} advance={:?} drome_enter={}",
        uuid_text(session.as_bytes()),
        refused.disposition, entered.disposition
    ))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match verdict::indexed_timeout(Duration::from_secs(90), verdict::observe(journey(clients))).await {
        Ok(Ok(observed)) => evidence(&format!(
            "e2e_all scenario=W17.client status=CAPTURED evidence={observed}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=W17.client status=FAIL evidence={}",
            error.to_string().replace(' ', "_")
        )),
        Err(_) => evidence("e2e_all scenario=W17.client status=FAIL evidence=timeout"),
    }
}
