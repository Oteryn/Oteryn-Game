//! P04 client replay capture: real TASK_BOARD replies for the Query and a refused StartWeekly with
//! a difficulty above the character’s level. The consumer is `apply_full_result` and `panel_text` in
//! `crates/client-ui/src/features/0072-task-board`.
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
            "P04_CLIENT_CAPTURE_WRITE_ERROR file={name} error={error}"
        ));
    }
}

async fn send(
    client: &mut DevClientSession,
    action: u8,
    selection: u16,
    name: Option<&str>,
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
    if result.status != CommandStatus::Accepted {
        return Err(format!("task-board action={action} status={:?}", result.status).into());
    }
    let session = client.join_snapshot().game_session_id;
    let mut captured = format!(
        "{} {} {} {}\n",
        uuid_text(session.as_bytes()),
        result.command_id,
        result.server_sequence,
        result.status as u8,
    )
    .into_bytes();
    captured.extend_from_slice(&result.payload);
    if let Some(name) = name {
        capture(name, &captured);
    }
    decode_reply(&result.payload).map_err(|error| format!("reply decode: {error:?}").into())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let mut client = connect_world(clients, 0x26).await?;
    let initial = send(&mut client, 4, 0, Some("p04-query.bin")).await?;
    if initial.refusal != 0 || initial.shop.is_empty() || initial.weekly.started {
        return Err(format!("query refused or without shop: {initial:?}").into());
    }
    let refused = send(&mut client, 5, 3, Some("p04-refused.bin")).await?;
    let mut unchanged = refused.clone();
    unchanged.refusal = 0;
    if refused.refusal != 3 || unchanged != initial {
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
    capture("p04-expected.txt", expected.as_bytes());
    let admitted = client.join_snapshot().game_session_id;
    let session = uuid_text(admitted.as_bytes());
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("P04 client capture logout was refused".into());
    }
    drop(client);
    let mut fresh = connect_world(clients, 0x26).await?;
    if fresh.join_snapshot().game_session_id == admitted
        || send(&mut fresh, 4, 0, None).await? != initial
    {
        return Err("P04 client refusal changed the board after readmission".into());
    }
    if fresh.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("P04 client verification logout was refused".into());
    }
    Ok(format!("session={session};{}", expected.replace('\n', ";")))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match verdict::indexed_timeout(Duration::from_secs(120), verdict::observe(journey(clients)))
        .await
    {
        Ok(Ok(expected)) => evidence(&format!(
            "e2e_all scenario=P04.client status=CAPTURED evidence={expected}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=P04.client status=FAIL evidence={error}"
        )),
        Err(_) => evidence("e2e_all scenario=P04.client status=FAIL evidence=timeout"),
    }
}
