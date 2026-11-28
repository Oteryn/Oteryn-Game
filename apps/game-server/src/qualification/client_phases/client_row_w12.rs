//! Live Cyclopedia map replies for the production client reducer replay.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_dev_client::client_runtime::cyclopedia_map as wire;

fn capture(root: &Path, name: &str, bytes: &[u8]) -> TestResult {
    std::fs::write(root.join(name), bytes)?;
    Ok(())
}

async fn exchange(
    client: &mut DevClientSession,
    operation: u8,
    root: &Path,
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
    let mut frame = reply.command_id.to_le_bytes().to_vec();
    frame.extend_from_slice(&reply.server_sequence.to_le_bytes());
    frame.extend_from_slice(&reply.payload);
    capture(root, file, &frame)?;
    wire::decode_result(&reply.payload)
        .map_err(|_| format!("operation {operation} result malformed").into())
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
    let root = std::path::PathBuf::from(
        std::env::var_os("E2E_ALL_CAPTURE_DIR").ok_or("missing capture directory")?,
    )
    .join("w12");
    std::fs::create_dir(&root)?;
    let query = exchange(&mut client, 0, &root, "w12-query.bin").await?;
    let find = exchange(&mut client, 3, &root, "w12-find.bin").await?;
    if query.code != 0 || find != query {
        return Err(format!(
            "stationary FIND changed the automatic exploration state: {query:?} -> {find:?}"
        )
        .into());
    }
    let start = exchange(&mut client, 1, &root, "w12-start.bin").await?;
    if start != find {
        return Err(
            format!("legacy START changed active exploration: {find:?} -> {start:?}").into(),
        );
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
    let mut frame = refused.command_id.to_le_bytes().to_vec();
    frame.extend_from_slice(&refused.server_sequence.to_le_bytes());
    frame.extend_from_slice(&refused.payload);
    capture(&root, "w12-refused.bin", &frame)?;
    let session = format!("{:?}", client.join_snapshot().game_session_id);
    capture(
        &root,
        "w12-expected.txt",
        format!("session={session}\n{expected}").as_bytes(),
    )?;
    Ok(format!(
        "session={session} sequence={}",
        refused.server_sequence
    ))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match verdict::indexed_timeout(Duration::from_secs(120), verdict::observe(journey(clients)))
        .await
    {
        Ok(Ok(observed)) => evidence(&format!(
            "e2e_capture scenario=W12.client status=CAPTURED {observed}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=W12.client status=FAIL evidence={error}"
        )),
        Err(_) => evidence("e2e_all scenario=W12.client status=FAIL evidence=timeout"),
    }
}
