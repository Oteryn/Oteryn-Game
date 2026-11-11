//! P04 client replay capture: real TASK_BOARD replies for the Query and a refused StartWeekly with
//! a difficulty the board does not offer. The consumer is `apply_full_result` and `panel_text` in
//! `crates/client-ui/src/features/0072-task-board`.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_dev_client::client_runtime::task_board;
use task_board::task_board::full::{Reply, Request, decode_reply, encode_request};

fn capture(name: &str, bytes: &[u8]) -> TestResult {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return Ok(());
    };
    std::fs::write(Path::new(&root).join(name), bytes)?;

    Ok(())
}

async fn send(
    client: &mut DevClientSession,
    action: u8,
    selection: u16,
    name: &str,
) -> TestResult<Reply> {
    let payload = encode_request(&Request {
        action,
        selection,
        race: String::new(),
    })
    .map_err(|error| format!("request encoder: {error:?}"))?;
    let result = client
        .session_mut()
        .gated_command(
            task_board::task_board::CAPABILITY_TASK_BOARD_V1,
            task_board::task_board::COMMAND_TYPE_TASK_BOARD,
            &payload,
        )
        .await?;
    if !matches!(
        result.status,
        CommandStatus::Accepted | CommandStatus::DuplicateReplay
    ) {
        return Err(format!("task-board action={action} status={:?}", result.status).into());
    }
    capture(name, &result.payload)?;
    decode_reply(&result.payload).map_err(|error| format!("reply decode: {error:?}").into())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let row_deadline = tokio::time::Instant::now() + Duration::from_secs(120);
    let mut client = connect_world(clients, 0x26).await?;
    let row_result: TestResult<_> = tokio::time::timeout_at(row_deadline, async {
    let initial = send(&mut client, 4, 0, "p04-query.bin").await?;
    if initial.refusal != 0 || initial.shop.is_empty() {
        return Err(format!("query refused or without shop: {initial:?}").into());
    }
    let refused = send(&mut client, 5, 3, "p04-refused.bin").await?;
    if refused.refusal == 0 || refused.weekly != initial.weekly {
        return Err(format!("StartWeekly(3) was not refused unchanged: {refused:?}").into());
    }
    let offer = initial.shop.first().ok_or("shop offer")?;
    let expected = format!(
        "period={}\npoints={}\nbounty_points={}\nsoulseals={}\nstarted={}\nkills={}\ndeliveries={}\nshop_count={}\nshop_first={},{},{}\nrefusal={}\n",
        initial.period,
        initial.points,
        initial.bounty_points,
        initial.soulseals,
        initial.weekly.started,
        initial.weekly.kills.len(),
        initial.weekly.deliveries.len(),
        initial.shop.len(),
        offer.index,
        offer.kind,
        offer.price,
        refused.refusal
    );
    capture("p04-expected.txt", expected.as_bytes())?;
    Ok(expected.replace('\n', ";"))

    }).await.unwrap_or_else(|error| Err(error.into()));
    let logout_result = row_logout(&mut client).await;
    logout_result?;
    row_result
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match row_complete(journey(clients)).await {
        Ok(Ok(expected)) => evidence(&format!(
            "e2e_all scenario=P04.client status=CAPTURED evidence={expected}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=P04.client status=FAIL evidence={error}"
        )),
        Err(_) => evidence("e2e_all scenario=P04.client status=FAIL evidence=timeout"),
    }
}
