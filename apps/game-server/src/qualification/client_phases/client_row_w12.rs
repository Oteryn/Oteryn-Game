//! W12 client replay capture: the real Cyclopedia map QUERY reply, the FIND-before-START refusal
//! (code 4), the START reply (code 0 premium or 2 non-premium) and a malformed intent refusal.
//! The consumer is `apply` in `apps/client/src/features/0057-cyclopedia-map/protocol.rs`.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_dev_client::client_runtime::cyclopedia_map as wire;

fn capture(name: &str, bytes: &[u8]) {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return;
    };
    if let Err(error) = std::fs::write(Path::new(&root).join(name), bytes) {
        evidence(&format!("W12_CLIENT_CAPTURE_WRITE_ERROR file={name} error={error}"));
    }
}

async fn exchange(
    client: &mut DevClientSession,
    operation: u8,
    file: &str,
) -> TestResult<wire::MapResult> {
    let (kind, payload) = wire::request(operation, 0).map_err(|_| "map request encoder")?;
    let reply = client
        .session_mut()
        .gated_command(wire::CAPABILITY_CYCLOPEDIA_MAP_V1, kind, &payload)
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("map operation {operation} status={:?}", reply.status).into());
    }
    capture(file, &reply.payload);
    wire::decode_result(&reply.payload).map_err(|_| format!("operation {operation} result malformed").into())
}

fn describe(prefix: &str, result: &wire::MapResult) -> String {
    format!(
        "{prefix}_code={}\n{prefix}_revision={}\n{prefix}_discovered={}\n{prefix}_found={}\n{prefix}_percent={}\n{prefix}_pool={}\n{prefix}_epoch={}\n{prefix}_improved={}\n{prefix}_visible={:?}\n",
        result.code,
        result.revision,
        result.discovered,
        result.found,
        result.area_percent,
        result.pool,
        result.epoch,
        result.improved,
        result.visible
    )
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let mut client = connect_world(clients, 0xfc).await?;
    let query = exchange(&mut client, 0, "w12-query.bin").await?;
    let find = exchange(&mut client, 3, "w12-find.bin").await?;
    if find.code != 4 {
        return Err(format!("FIND before START was not refused with code 4: {find:?}").into());
    }
    let start = exchange(&mut client, 1, "w12-start.bin").await?;
    if !matches!(start.code, 0 | 2) {
        return Err(format!("START returned neither success nor premium refusal: {start:?}").into());
    }
    let refused = client
        .session_mut()
        .gated_command(
            wire::CAPABILITY_CYCLOPEDIA_MAP_V1,
            wire::COMMAND_TYPE_CYCLOPEDIA_INTENT,
            &[1, 2, 0, 0, 0, 0, 0, 0, 0],
        )
        .await?;
    if refused.status != CommandStatus::Rejected || !refused.payload.is_empty() {
        return Err(format!("malformed intent was not refused: {refused:?}").into());
    }
    let expected = format!(
        "{}{}{}",
        describe("query", &query),
        describe("find", &find),
        describe("start", &start)
    );
    capture("w12-expected.txt", expected.as_bytes());
    Ok(expected.replace('\n', ";").replace(' ', ""))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match verdict::indexed_timeout(Duration::from_secs(120), verdict::observe(journey(clients))).await {
        Ok(Ok(observed)) => evidence(&format!(
            "e2e_all scenario=W12.client status=CAPTURED evidence={observed}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=W12.client status=FAIL evidence={error}"
        )),
        Err(_) => evidence("e2e_all scenario=W12.client status=FAIL evidence=timeout"),
    }
}
