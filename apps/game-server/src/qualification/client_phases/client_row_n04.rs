//! N04 client replay capture: a real player bug report (category "Other issue", no map position)
//! and an immediate second report. Client owner: `apps/client/src/features/0076-player-report`
//! (`bug.rs` builds the request, `protocol.rs` `apply` turns the reply into feedback).
//!
//! The request bytes the server accepted are captured too, so the client test can require its own
//! request to be byte-identical.
use super::*;
use oteryn_protocol_oteryn::player_report as wire;

const COMMENT: &str = "N04 bug report: the depot chest closed by itself";

fn capture(name: &str, bytes: &[u8]) -> TestResult {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return Ok(());
    };
    std::fs::write(Path::new(&root).join(name), bytes)?;

    Ok(())
}

async fn report(
    client: &mut oteryn_dev_client::DevClientSession,
    payload: &[u8],
) -> TestResult<(wire::PlayerReportReply, Vec<u8>)> {
    let response = client
        .session_mut()
        .gated_command(
            wire::CAPABILITY_PLAYER_REPORT_V1,
            wire::COMMAND_TYPE_PLAYER_REPORT,
            payload,
        )
        .await?;
    if response.status != CommandStatus::Accepted {
        return Err(format!("report status {:?}", response.status).into());
    }
    let reply = wire::decode_player_report_reply(&response.payload)
        .map_err(|error| format!("report reply decode {error:?}"))?;
    Ok((reply, response.payload))
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let row_deadline = tokio::time::Instant::now() + Duration::from_secs(90);
    let mut client = connect_world(clients, 0xae).await?;
    let row_result: TestResult<_> = tokio::time::timeout_at(row_deadline, async {
    let request = wire::PlayerReportRequest {
        kind: wire::KIND_BUG,
        category: 3,
        reason: 0,
        action: 0,
        target: String::new(),
        comment: COMMENT.into(),
        map_x: 0,
        map_y: 0,
        map_floor: 0,
    };
    wire::validate_player_report(&request).map_err(|code| format!("invalid request {code:?}"))?;
    let payload = wire::encode_player_report_request(&request)
        .map_err(|error| format!("report encode {error:?}"))?;
    let (first, first_bytes) = report(&mut client, &payload).await?;
    if first.code != wire::PlayerReportCode::Accepted || first.report == 0 || first.wait_seconds != 0 {
        return Err(format!("first report not accepted: {first:?}").into());
    }
    let (second, second_bytes) = report(&mut client, &payload).await?;
    if !matches!(
        second.code,
        wire::PlayerReportCode::Accepted | wire::PlayerReportCode::RateLimited
    ) {
        return Err(format!("second report unexpected reply: {second:?}").into());
    }
    let expected = format!(
        "comment={COMMENT}\nfirst_code={:?}\nfirst_report={}\nsecond_code={:?}\nsecond_report={}\n\
         second_wait={}\n",
        first.code, first.report, second.code, second.report, second.wait_seconds
    );
    capture("n04-request.bin", &payload)?;
    capture("n04-first.bin", &first_bytes)?;
    capture("n04-second.bin", &second_bytes)?;
    capture("n04-expected.txt", expected.as_bytes())?;

    Ok(format!(
        "first={:?}#{}|second={:?}#{}wait{}",
        first.code, first.report, second.code, second.report, second.wait_seconds
    ))

    }).await.unwrap_or_else(|error| Err(error.into()));
    let logout_result = row_logout(&mut client).await;
    logout_result?;
    row_result
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match row_complete(journey(clients)).await {
        Ok(Ok(observed)) => evidence(&format!(
            "e2e_all scenario=N04.client status=CAPTURED evidence={observed}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=N04.client status=FAIL evidence={}",
            error.to_string().replace(' ', "_")
        )),
        Err(_) => evidence("e2e_all scenario=N04.client status=FAIL evidence=timeout"),
    }
}
