//! P07 client replay capture: the real Atelier current-state frames (gems, quotes, resource
//! balances) and a stale-revision Reveal refusal. The consumer is `apply_result` in
//! `crates/client-ui/src/features/0148-wheel-allocation/protocol.rs`.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_dev_client::client_runtime::wheel_allocation::{self, WheelCommand, WheelResponse};
use oteryn_protocol_oteryn::{atelier, wheel};

fn capture(name: &str, bytes: &[u8]) -> TestResult {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return Ok(());
    };
    std::fs::write(Path::new(&root).join(name), bytes)?;

    Ok(())
}

async fn send(
    client: &mut DevClientSession,
    kind: u32,
    payload: Vec<u8>,
    name: &str,
) -> TestResult<(WheelResponse, atelier::AtelierReply)> {
    let command = WheelCommand::new(kind, payload).ok_or("Wheel runtime rejected the command")?;
    let response = wheel_allocation::send(client.session_mut(), &command).await?;
    if !matches!(
        response.status,
        CommandStatus::Accepted | CommandStatus::DuplicateReplay
    ) {
        return Err(format!("{name} status={:?}", response.status).into());
    }
    capture(&format!("p07-{name}-receipt.bin"), &response.payload)?;
    for (suffix, frame) in [
        ("wheel", &response.wheel),
        ("atelier", &response.atelier),
        ("resources", &response.resources),
    ] {
        if let Some(bytes) = frame {
            capture(&format!("p07-{name}-{suffix}.bin"), bytes)?;
        }
    }
    let current = response
        .atelier
        .as_deref()
        .ok_or("current Atelier frame absent")?;
    let current = atelier::decode_reply(current)
        .map_err(|error| format!("{name} current Atelier: {error:?}"))?;
    Ok((response, current))
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let row_deadline = tokio::time::Instant::now() + Duration::from_secs(120);
    let mut client = connect_world(clients, 0x29).await?;
    let row_result: TestResult<_> = tokio::time::timeout_at(row_deadline, async {
    if !client
        .selected_capabilities()
        .contains(&wheel::CAPABILITY_WHEEL_ALLOCATION_V1)
    {
        return Err("admission omitted Atelier capability 148".into());
    }
    let (query, initial) = send(
        &mut client,
        wheel::COMMAND_TYPE_WHEEL_ALLOCATION,
        vec![8, 0],
        "query",
    )
    .await?;
    if initial.refused {
        return Err(format!("Atelier query refused: {initial:?}").into());
    }
    let balances = query
        .resources
        .as_deref()
        .and_then(|bytes| atelier::resources::decode_reply(bytes).ok())
        .ok_or("resource balance frame absent or malformed")?;
    let mut occurrence = [0x29; 16];
    occurrence[6] = 0x70;
    occurrence[8] = 0x80;
    let payload = atelier::encode_mutate(&atelier::MutateRequest {
        occurrence,
        expected_revision: initial.revision + 99,
        mutation: atelier::Mutation::Reveal { quality: 0 },
    })
    .map_err(|error| format!("reveal encoder: {error:?}"))?;
    let (stale, after) = send(
        &mut client,
        atelier::COMMAND_TYPE_ATELIER_MUTATE,
        payload,
        "stale",
    )
    .await?;
    let receipt = atelier::decode_reply(&stale.payload)
        .map_err(|error| format!("stale receipt: {error:?}"))?;
    if !receipt.refused || after != initial {
        return Err(format!("stale Reveal was not refused unchanged: {receipt:?}").into());
    }
    let quote = initial.quality_quotes.first();
    let expected = format!(
        "revision={}\ngems={}\ngrades={}\nreveal_gold_q0={}\ncarried_gold={}\nstale_refused={}\nstale_current_revision={}\n",
        initial.revision,
        initial.gems.len(),
        initial.grades.len(),
        quote.map_or(0, |q| q.reveal_gold),
        balances.carried_gold,
        receipt.refused,
        after.revision
    );
    capture("p07-expected.txt", expected.as_bytes())?;
    Ok(expected.replace('\n', ";"))

    }).await.unwrap_or_else(|error| Err(error.into()));
    let logout_result = row_logout(&mut client).await;
    logout_result?;
    row_result
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match row_complete(journey(clients)).await {
        Ok(Ok(expected)) => evidence(&format!(
            "e2e_all scenario=P07.client status=CAPTURED evidence={expected}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=P07.client status=FAIL evidence={error}"
        )),
        Err(_) => evidence("e2e_all scenario=P07.client status=FAIL evidence=timeout"),
    }
}
