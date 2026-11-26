//! S09 live client qualification and optional replay capture: native logout and fresh login.
//! Capability 21 logout is not a gated command, so `Session::logout` decodes the `CommandResult` into `LogoutOutcome` and the
//! raw bytes are not retained. The capture therefore records the server-observed outcome
//! (`s09-expected.txt`) and the replay (`crates/session/src/tests/e2e_s09.rs`) encodes that
//! outcome with the production logout encoder; no `.bin` file is written or claimed as raw.
use super::*;
use oteryn_protocol_oteryn::logout::LogoutResult;

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let mut client = connect_character(clients, 0x43, 0).await?;
    let position = client.world_spatial().actor_position;
    let session = client.join_snapshot().game_session_id;
    let outcome = client.session_mut().logout().await?;
    if outcome.status != CommandStatus::Accepted || outcome.result != Some(LogoutResult::Accepted) {
        return Err(format!("step=logout observed {outcome:?}").into());
    }
    drop(client);
    let mut fresh = connect_character(clients, 0x46, 0).await?;
    let retained = fresh.world_spatial().actor_position == position
        && fresh.join_snapshot().game_session_id != session;
    if !retained {
        return Err(format!(
            "step=relog position {:?} expected {position:?} or session not renewed",
            fresh.world_spatial().actor_position
        )
        .into());
    }
    let cleanup = fresh.session_mut().logout().await?;
    if cleanup.status != CommandStatus::Accepted || cleanup.result != Some(LogoutResult::Accepted) {
        return Err(format!("step=relog-logout observed {cleanup:?}").into());
    }
    if let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        use std::io::Write;
        let mut capture = std::fs::File::create_new(Path::new(&root).join("s09-expected.txt"))?;
        writeln!(
            capture,
            "logout\tstatus={:?}\tresult={:?}",
            outcome.status, outcome.result
        )?;
    }
    Ok(format!("logout=Accepted relog_position={position:?}"))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    match verdict::indexed_timeout(Duration::from_secs(90), verdict::observe(journey(clients)))
        .await
    {
        Ok(Ok(observed)) => evidence(&format!(
            "e2e_all scenario=S09.client status=PASS seconds={:.3} evidence={observed}",
            started.elapsed().as_secs_f64()
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=S09.client status=FAIL evidence={}",
            error.to_string().replace(' ', "_")
        )),
        Err(_) => evidence("e2e_all scenario=S09.client status=FAIL evidence=timeout"),
    }
}
