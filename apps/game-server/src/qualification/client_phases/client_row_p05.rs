//! P05 client replay capture: real TASK_BOARD replies for the Query, a Talisman purchase and the
//! repeated purchase that the server refuses. The consumer is `talisman::request`,
//! `apply_full_result` and `panel_text` in `crates/client-ui/src/features/0072-task-board`.
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

async fn send(client: &mut DevClientSession, action: u8, name: &str) -> TestResult<Reply> {
    let payload = encode_request(&Request {
        action,
        selection: 0,
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
    if result.status != CommandStatus::Accepted {
        return Err(format!("task-board action={action} status={:?}", result.status).into());
    }
    capture(name, &result.payload)?;
    decode_reply(&result.payload).map_err(|error| format!("reply decode: {error:?}").into())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let row_deadline = tokio::time::Instant::now() + Duration::from_secs(120);
    let mut client = connect_character(clients, 0x27, 0).await?;
    let row_result: TestResult<_> = tokio::time::timeout_at(row_deadline, async {
    let before = send(&mut client, 4, "p05-query.bin").await?;
    let bought = send(&mut client, 17, "p05-buy.bin").await?;
    if bought.refusal != 0 {
        return Err(format!("Talisman purchase refused: {bought:?}").into());
    }
    let repeated = send(&mut client, 17, "p05-repeat.bin").await?;
    if repeated.refusal == 0 || repeated.talisman != bought.talisman {
        return Err(format!("repeat purchase was not refused unchanged: {repeated:?}").into());
    }
    let level = |reply: &Reply| reply.talisman.iter().map(|p| format!("{},{},{}", p.level, p.bonus, p.next_cost)).collect::<Vec<_>>().join("|");
    let expected = format!(
        "bounty_points_before={}\nbounty_points_after={}\ntalisman_before={}\ntalisman_after={}\nrefusal={}\n",
        before.bounty_points,
        bought.bounty_points,
        level(&before),
        level(&bought),
        repeated.refusal
    );
    capture("p05-expected.txt", expected.as_bytes())?;
    Ok(expected.replace('\n', ";"))

    }).await.unwrap_or_else(|error| Err(error.into()));
    let logout_result = row_logout(&mut client).await;
    logout_result?;
    row_result
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match row_complete(journey(clients)).await {
        Ok(Ok(expected)) => evidence(&format!(
            "e2e_all scenario=P05.client status=CAPTURED evidence={expected}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=P05.client status=FAIL evidence={error}"
        )),
        Err(_) => evidence("e2e_all scenario=P05.client status=FAIL evidence=timeout"),
    }
}
