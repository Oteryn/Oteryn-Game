//! W10 client replay capture: the real Hazard query reply (`[unlocked, selected]`), the level-1
//! selection reply and the locked level-2 refusal. The consumer is `apply_result` in
//! `crates/client-ui/src/features/0117-boss-encounter/protocol.rs` (hazard summary in
//! `protocol/auxiliary.rs`).
use super::*;
use oteryn_dev_client::client_runtime::boss_encounter as wire;

fn capture(name: &str, bytes: &[u8]) -> TestResult {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return Ok(());
    };
    std::fs::write(Path::new(&root).join(name), bytes)?;

    Ok(())
}

async fn hazard(
    client: &mut oteryn_dev_client::DevClientSession,
    payload: &[u8],
) -> TestResult<oteryn_dev_client::client_runtime::server_mechanic::ServerMechanicResponse> {
    Ok(client
        .session_mut()
        .gated_command(
            wire::CAPABILITY_BOSS_ENCOUNTER_V1,
            wire::hazard::COMMAND_TYPE_HAZARD,
            payload,
        )
        .await?)
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let row_deadline = tokio::time::Instant::now() + Duration::from_secs(120);
    let mut client = connect_world(clients, 0x05).await?;
    let row_result: TestResult<_> = tokio::time::timeout_at(row_deadline, async {
        let query = hazard(&mut client, &[]).await?;
        let select = hazard(&mut client, &[1]).await?;
        let locked = hazard(&mut client, &[2]).await?;
        for (name, reply) in [("query", &query), ("select", &select)] {
            if reply.status != CommandStatus::Accepted || reply.payload.len() != 2 {
                return Err(format!("hazard {name} reply={reply:?}").into());
            }
        }
        if locked.status != CommandStatus::Rejected || !locked.payload.is_empty() {
            return Err(format!("locked hazard level was not refused: {locked:?}").into());
        }
        capture("w10-query.bin", &query.payload)?;
        capture("w10-select.bin", &select.payload)?;
        let expected = format!(
            "query_unlocked={}\nquery_selected={}\nselect_unlocked={}\nselect_selected={}\n",
            query.payload[0], query.payload[1], select.payload[0], select.payload[1]
        );
        capture("w10-expected.txt", expected.as_bytes())?;
        Ok(expected.replace('\n', ";"))
    })
    .await
    .unwrap_or_else(|error| Err(error.into()));
    let logout_result = row_logout(&mut client).await;
    logout_result?;
    row_result
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match row_complete(journey(clients)).await {
        Ok(Ok(observed)) => evidence(&format!(
            "e2e_all scenario=W10.client status=CAPTURED evidence={observed}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=W10.client status=FAIL evidence={error}"
        )),
        Err(_) => evidence("e2e_all scenario=W10.client status=FAIL evidence=timeout"),
    }
}
