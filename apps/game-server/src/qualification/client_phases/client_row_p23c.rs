//! P23c client replay capture: the real legacy TASK_BOARD replies for the Query and for selecting
//! the first offered hunting task. The consumer is `apply_result` and `panel_text` in
//! `crates/client-ui/src/features/0072-task-board`.
use super::*;
use oteryn_protocol_oteryn::task_board::{
    CAPABILITY_TASK_BOARD_V1, COMMAND_TYPE_TASK_BOARD, TaskBoardReply, TaskBoardRequest,
    decode_task_board_reply, encode_task_board_request,
};

fn capture(name: &str, bytes: &[u8]) -> TestResult {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return Ok(());
    };
    std::fs::write(Path::new(&root).join(name), bytes)?;

    Ok(())
}

async fn send(
    client: &mut oteryn_dev_client::DevClientSession,
    action: u8,
    task: u8,
) -> TestResult<(Vec<u8>, TaskBoardReply)> {
    let payload = encode_task_board_request(&TaskBoardRequest { action, task })
        .map_err(|error| format!("request encoder: {error:?}"))?;
    let result = client
        .session_mut()
        .gated_command(CAPABILITY_TASK_BOARD_V1, COMMAND_TYPE_TASK_BOARD, &payload)
        .await?;
    if result.status != CommandStatus::Accepted {
        return Err(format!("task-board action={action} status={:?}", result.status).into());
    }
    let reply = decode_task_board_reply(&result.payload)
        .map_err(|error| format!("reply decode: {error:?}"))?;
    Ok((result.payload, reply))
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let row_deadline = tokio::time::Instant::now() + Duration::from_secs(90);
    let mut client = connect_character(clients, 0x45, 1).await?;
    let row_result: TestResult<_> = tokio::time::timeout_at(row_deadline, async {
        let (query_bytes, query) = send(&mut client, 0, 0).await?;
        if query.refused || query.tasks.is_empty() {
            return Err(format!("query refused or without tasks: {query:?}").into());
        }
        capture("p23c-query.bin", &query_bytes)?;
        let (select_bytes, select) = send(&mut client, 1, 1).await?;
        capture("p23c-select.bin", &select_bytes)?;
        let mut expected = format!(
            "points={}\nbounty_points={}\ntask_count={}\n",
            query.points,
            query.bounty_points,
            query.tasks.len()
        );
        for (index, task) in query.tasks.iter().enumerate() {
            expected.push_str(&format!(
                "task_{}={},{},{}\n",
                index + 1,
                task.race,
                task.required,
                task.claimed_at_unix
            ));
        }
        expected.push_str(&format!(
            "select_refused={}\nselect_selected={}\nselect_kills={}\nselect_points={}\n",
            select.refused, select.selected, select.kills, select.points
        ));
        capture("p23c-expected.txt", expected.as_bytes())?;

        Ok(expected.replace('\n', ";").replace(' ', "_"))
    })
    .await
    .unwrap_or_else(|error| Err(error.into()));
    let logout_result = row_logout(&mut client).await;
    logout_result?;
    row_result
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match row_complete(journey(clients)).await {
        Ok(Ok(expected)) => evidence(&format!(
            "e2e_all scenario=P23c.client status=CAPTURED evidence={expected}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=P23c.client status=FAIL evidence={error}"
        )),
        Err(_) => evidence("e2e_all scenario=P23c.client status=FAIL evidence=timeout"),
    }
}
