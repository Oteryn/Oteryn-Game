//! W12 client replay capture: automatic discovery QUERY/FIND, idempotent legacy START
//! (or already-discovered code 3), and a malformed intent refusal.
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
    record: &mut impl FnMut(&str, &[u8]),
) -> TestResult<wire::MapResult> {
    let (kind, payload) = wire::request(operation, 0).map_err(|_| "map request encoder")?;
    let reply = client
        .session_mut()
        .gated_command(wire::CAPABILITY_CYCLOPEDIA_MAP_V1, kind, &payload)
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("map operation {operation} status={:?}", reply.status).into());
    }
    record(file, &reply.payload);
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

async fn observe_initial_discovery<F, Fut>(
    client: &mut DevClientSession,
    record: &mut impl FnMut(&str, &[u8]),
    witness: F,
) -> TestResult<(wire::MapResult, wire::MapResult, wire::MapResult)>
where
    F: FnOnce(ActorPosition, wire::MapResult) -> Fut,
    Fut: Future<Output = TestResult<u8>>,
{
    let query = exchange(client, 0, "w12-query.bin", record).await?;
    let position = client.world_spatial().actor_position;
    let find = exchange(client, 3, "w12-find.bin", record).await?;
    if client.world_spatial().actor_position != position {
        return Err("position changed during stationary discovery check".into());
    }
    super::full_row_w12::validate_initial_find(&query, &find, position, witness).await?;
    let start = exchange(client, 1, "w12-start.bin", record).await?;
    if client.world_spatial().actor_position != position {
        return Err("position changed during stationary discovery check".into());
    }
    super::full_row_w12::validate_legacy_start(&find, &start)?;
    Ok((query, find, start))
}

#[cfg(test)]
pub(super) async fn observe_initial_discovery_for_test<F, Fut>(
    client: &mut DevClientSession,
    record: &mut impl FnMut(&str, &[u8]),
    witness: F,
) -> TestResult<(wire::MapResult, wire::MapResult, wire::MapResult)>
where
    F: FnOnce(ActorPosition, wire::MapResult) -> Fut,
    Fut: Future<Output = TestResult<u8>>,
{
    observe_initial_discovery(client, record, witness).await
}

async fn journey(clients: &SeamClients<'_>, world: &BundleWorld) -> TestResult<String> {
    let mut client = connect_world(clients, 0xfc).await?;
    let mut record = capture;
    let (query, find, start) = observe_initial_discovery(&mut client, &mut record, |position, observed| async move {
        super::full_row_w12::completed_area_percentage(clients, world, position, &observed).await
    }).await?;
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

pub(super) async fn run_packet(clients: &SeamClients<'_>, world: &BundleWorld) {
    match verdict::indexed_timeout(Duration::from_secs(120), verdict::observe(journey(clients, world))).await {
        Ok(Ok(observed)) => evidence(&format!(
            "e2e_all scenario=W12.client status=CAPTURED evidence={observed}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=W12.client status=FAIL evidence={error}"
        )),
        Err(_) => evidence("e2e_all scenario=W12.client status=FAIL evidence=timeout"),
    }
}
