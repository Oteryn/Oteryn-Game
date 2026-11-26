//! Capture real Task Board query and Bounty Point upgrade replies for the client replay.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_dev_client::client_runtime::task_board;
use task_board::task_board::full::{Reply, Request, decode_reply, encode_request};

fn capture(name: &str, bytes: &[u8]) {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return;
    };
    if let Err(error) = std::fs::write(Path::new(&root).join(name), bytes) {
        evidence(&format!("P05_CLIENT_CAPTURE_WRITE_ERROR file={name} error={error}"));
    }
}

async fn send(client: &mut DevClientSession, action: u8) -> TestResult<(Reply, Vec<u8>)> {
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
    let reply =
        decode_reply(&result.payload).map_err(|error| format!("reply decode: {error:?}"))?;
    Ok((reply, result.payload))
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let mut client = connect_character(clients, 0x27, 0).await?;
    let (mut before, mut query) = send(&mut client, 4).await?;
    for _ in 0..2 {
        if before.bounty_points >= u64::from(before.talisman[0].next_cost) {
            break;
        }
        let (opened, _) = send(&mut client, 7).await?;
        full_row_p05::complete_bounty(&mut client, clients, &opened).await?;
        (before, query) = send(&mut client, 4).await?;
    }
    let (upgraded, upgrade) = send(&mut client, 18).await?;
    if before.talisman[0].level.checked_add(1) != Some(upgraded.talisman[0].level)
        || before
            .bounty_points
            .checked_sub(u64::from(before.talisman[0].next_cost))
            != Some(upgraded.bounty_points)
        || upgraded.refusal != 0
    {
        return Err(format!("Bounty Point upgrade did not commit: {upgraded:?}").into());
    }
    capture("p05-query.bin", &query);
    capture("p05-upgrade.bin", &upgrade);
    let level = |reply: &Reply| {
        reply
            .talisman
            .iter()
            .map(|p| format!("{},{},{}", p.level, p.bonus, p.next_cost))
            .collect::<Vec<_>>()
            .join("|")
    };
    let expected = format!(
        "bounty_points_before={}\nbounty_points_after={}\ntalisman_before={}\ntalisman_after={}\n",
        before.bounty_points,
        upgraded.bounty_points,
        level(&before),
        level(&upgraded)
    );
    capture("p05-expected.txt", expected.as_bytes());
    Ok(expected.replace('\n', ";"))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match verdict::indexed_timeout(Duration::from_secs(120), verdict::observe(journey(clients))).await {
        Ok(Ok(expected)) => evidence(&format!(
            "e2e_all scenario=P05.client status=CAPTURED evidence={expected}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=P05.client status=FAIL evidence={error}"
        )),
        Err(_) => evidence("e2e_all scenario=P05.client status=FAIL evidence=timeout"),
    }
}
