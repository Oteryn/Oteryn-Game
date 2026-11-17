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

fn capture(name: &str, bytes: &[u8]) {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return;
    };
    if let Err(error) = std::fs::write(Path::new(&root).join(name), bytes) {
        evidence(&format!("W17_CLIENT_CAPTURE_WRITE_ERROR file={name} error={error}"));
    }
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
    if entered.disposition == 1 {
        // Leave again so later rows find the Tibiadrome idle.
        let left = action(&mut client, ArenaAction::Drome(DromeAction::Leave), "drome-leave").await?;
        let left = DromeResult::decode(&left).map_err(|error| format!("step=drome-leave decode: {error:?}"))?;
        if left.disposition != 1 {
            return Err(format!("step=drome-leave observed={left:?}").into());
        }
    } else if entered.disposition != 3 || entered != before {
        return Err(format!("step=drome-enter before={before:?} observed={entered:?}").into());
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
    capture("w17-catalogue.bin", &catalogue);
    capture("w17-status.bin", &status);
    capture("w17-advance.bin", &advance);
    capture("w17-drome-status.bin", &drome_status);
    capture("w17-drome-enter.bin", &drome_enter);
    capture("w17-expected.txt", expected.as_bytes());
    let _ = client.session_mut().logout().await;
    Ok(format!(
        "advance={:?} drome_enter={}",
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
