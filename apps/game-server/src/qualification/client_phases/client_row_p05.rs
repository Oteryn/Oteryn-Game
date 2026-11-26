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
        evidence(&format!(
            "P05_CLIENT_CAPTURE_WRITE_ERROR file={name} error={error}"
        ));
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
    let (before, query) = send(&mut client, 4).await?;
    let (after, upgrade) = send(&mut client, 18).await?;
    let prior = before.talisman[0];
    let unchanged =
        after.bounty_points == before.bounty_points && after.talisman == before.talisman;
    let expected = match (
        prior.next_cost,
        before.bounty_points.checked_sub(u64::from(prior.next_cost)),
    ) {
        (0, _) => after.refusal == 6 && unchanged,
        (_, None) => after.refusal == 4 && unchanged,
        (_, Some(remaining)) => {
            after.refusal == 0
                && after.bounty_points == remaining
                && prior.level.checked_add(1) == Some(after.talisman[0].level)
                && after.talisman[1..] == before.talisman[1..]
        }
    };
    if before.refusal != 0 || !expected {
        return Err(format!("unexpected Bounty Point upgrade reply: {after:?}").into());
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
        "bounty_points_before={}\nbounty_points_after={}\ntalisman_before={}\ntalisman_after={}\nrefusal={}\n",
        before.bounty_points,
        after.bounty_points,
        level(&before),
        level(&after),
        after.refusal
    );
    capture("p05-expected.txt", expected.as_bytes());
    Ok(expected.replace('\n', ";"))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match verdict::indexed_timeout(Duration::from_secs(120), verdict::observe(journey(clients)))
        .await
    {
        Ok(Ok(expected)) => evidence(&format!(
            "e2e_all scenario=P05.client status=CAPTURED evidence={expected}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=P05.client status=FAIL evidence={error}"
        )),
        Err(_) => evidence("e2e_all scenario=P05.client status=FAIL evidence=timeout"),
    }
}
