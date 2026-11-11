//! S09 client replay capture: a native logout and a fresh login. Capability 21 logout is not a
//! gated command, so `Session::logout` decodes the `CommandResult` into `LogoutOutcome` and the
//! raw bytes are not retained. The capture therefore records the server-observed outcome
//! (`s09-expected.txt`) and the replay (`crates/session/src/tests/e2e_s09.rs`) encodes that
//! outcome with the production logout encoder; no `.bin` file is written or claimed as raw.
use super::*;
use oteryn_protocol_oteryn::logout::LogoutResult;

fn capture(name: &str, bytes: &[u8]) -> TestResult {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return Ok(());
    };
    std::fs::write(Path::new(&root).join(name), bytes)?;

    Ok(())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let row_deadline = tokio::time::Instant::now() + Duration::from_secs(90);
    let mut client = connect_character(clients, 0x43, 0).await?;
    let mut row_needs_logout_0 = true;
    let row_result: TestResult<_> = async {
        let position = client.world_spatial().actor_position;
        let session = client.join_snapshot().game_session_id;
        let outcome = client.session_mut().logout().await?;
        if outcome.status != CommandStatus::Accepted
            || outcome.result != Some(LogoutResult::Accepted)
        {
            return Err(format!("step=logout observed {outcome:?}").into());
        }
        row_needs_logout_0 = false;
        let mut fresh = connect_character(clients, 0x46, 0).await?;
        let mut row_needs_logout_1 = true;
        let row_result: TestResult<_> = tokio::time::timeout_at(row_deadline, async {
            let retained = fresh.world_spatial().actor_position == position
                && fresh.join_snapshot().game_session_id != session;
            row_logout(&mut fresh).await?;
            row_needs_logout_1 = false;
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
            )?;
            Ok(format!("logout=Accepted relog_position={position:?}"))
        })
        .await
        .unwrap_or_else(|error| Err(error.into()));
        if row_needs_logout_1 {
            row_logout(&mut fresh).await?;
        }
        row_result
    }
    .await;
    if row_needs_logout_0 {
        row_logout(&mut client).await?;
    }
    row_result
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    if std::env::var_os("E2E_ALL_CAPTURE_DIR").is_none() {
        return;
    }
    match row_complete(journey(clients)).await {
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
