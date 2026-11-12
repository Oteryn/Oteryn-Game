//! S09 client replay capture: a native logout and a fresh login. Capability 21 logout is not a
//! gated command, so `Session::logout` decodes the `CommandResult` into `LogoutOutcome` and the
//! raw bytes are not retained. The capture therefore records the server-observed outcome
//! (`s09-expected.txt`) and the replay (`crates/session/src/tests/e2e_s09.rs`) encodes that
//! outcome with the production logout encoder; no `.bin` file is written or claimed as raw.
use super::*;
use oteryn_protocol_oteryn::logout::LogoutResult;

fn capture(name: &str, bytes: &[u8]) {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return;
    };
    if let Err(error) = std::fs::write(Path::new(&root).join(name), bytes) {
        evidence(&format!("S09_CLIENT_CAPTURE_WRITE_ERROR file={name} error={error}"));
    }
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let mut client = connect_character(clients, 0x43, 0).await?;
    let position = client.world_spatial().actor_position;
    let session = client.join_snapshot().game_session_id;
    let outcome = client.session_mut().logout().await?;
    if outcome.status != CommandStatus::Accepted || outcome.result != Some(LogoutResult::Accepted)
    {
        return Err(format!("step=logout observed {outcome:?}").into());
    }
    drop(client);
    let mut fresh = connect_character(clients, 0x46, 0).await?;
    let retained = fresh.world_spatial().actor_position == position
        && fresh.join_snapshot().game_session_id != session;
    let _ = fresh.session_mut().logout().await;
    if !retained {
        return Err(format!(
            "step=relog position {:?} expected {position:?} or session not renewed",
            fresh.world_spatial().actor_position
        )
        .into());
    }
    capture(
        "s09-expected.txt",
        format!(
            "logout\tstatus={:?}\tresult={:?}\n",
            outcome.status, outcome.result
        )
        .as_bytes(),
    );
    Ok(format!("logout=Accepted relog_position={position:?}"))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    if std::env::var_os("E2E_ALL_CAPTURE_DIR").is_none() {
        return;
    }
    match verdict::indexed_timeout(Duration::from_secs(90), verdict::observe(journey(clients))).await {
        Ok(Ok(observed)) => evidence(&format!(
            "e2e_all scenario=S09.client status=CAPTURED evidence={observed}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=S09.client status=FAIL evidence={}",
            error.to_string().replace(' ', "_")
        )),
        Err(_) => evidence("e2e_all scenario=S09.client status=FAIL evidence=timeout"),
    }
}
