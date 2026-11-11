//! I10 client replay capture: real house-furniture wrap / unwrap / stale-revision replies.
//!
//! The replies are replayed through the production reducer `apply` in
//! `apps/client/src/features/0207-house-furniture-wrap/protocol.rs`, which turns the command
//! status and payload into the player-visible feedback. The requests use the zero item id, so the
//! server's own refusal for each action is what is captured.
use super::*;
use oteryn_protocol_oteryn::house_wrap;

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let result = row_complete(journey(clients)).await;
    match result.unwrap_or_else(|_| Err("I10 client capture timed out".into())) {
        Ok(()) => evidence(
            "e2e_all scenario=I10.client status=PASS evidence=house-wrap-unwrap-and-stale-refusals",
        ),
        Err(error) => evidence(&format!(
            "e2e_all scenario=I10.client status=FAIL evidence={error}"
        )),
    }
}

async fn wrap_result(
    client: &mut oteryn_dev_client::DevClientSession,
    action: house_wrap::Action,
    revision: u64,
) -> TestResult<(CommandStatus, Vec<u8>)> {
    let position = client.world_spatial().actor_position;
    let request = house_wrap::Request {
        action,
        item: [0; 16],
        expected_revision: revision,
        acl_revision: 0,
        position: [position.x, position.y, i32::from(position.floor)],
    };
    let result = client
        .session_mut()
        .gated_command(
            house_wrap::CAPABILITY_HOUSE_FURNITURE_WRAP_V1,
            house_wrap::COMMAND_WRAP,
            &request.encode(),
        )
        .await?;
    // The client reducer shows a refusal only for a non-accepted command carrying a refusal byte.
    match house_wrap::Reply::decode(&result.payload) {
        Some(house_wrap::Reply::Refused(_)) if result.status != CommandStatus::Accepted => {
            Ok((result.status, result.payload))
        }
        other => Err(format!(
            "I10 {action:?} reply is not a refused refusal: status={:?} reply={other:?}",
            result.status
        )
        .into()),
    }
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let row_deadline = tokio::time::Instant::now() + Duration::from_secs(90);
    let mut client = connect_world(clients, 0xb2).await?;
    let row_result: TestResult<_> = tokio::time::timeout_at(row_deadline, async {
        let wrap = wrap_result(&mut client, house_wrap::Action::Wrap, 0).await?;
        let unwrap = wrap_result(&mut client, house_wrap::Action::Unwrap, 0).await?;
        let stale = wrap_result(&mut client, house_wrap::Action::Wrap, u64::MAX).await?;
        if let Some(directory) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
            let directory = Path::new(&directory);
            let hex = |bytes: &[u8]| bytes.iter().map(|b| format!("{b:02x}")).collect::<String>();
            for (name, reply) in [
                ("i10-wrap", &wrap),
                ("i10-unwrap", &unwrap),
                ("i10-refusal", &stale),
            ] {
                std::fs::write(directory.join(format!("{name}.bin")), &reply.1)?;
            }
            std::fs::write(
                directory.join("i10-expected.txt"),
                format!(
                    "wrap={}\nunwrap={}\nrefusal={}\n",
                    hex(&wrap.1),
                    hex(&unwrap.1),
                    hex(&stale.1)
                ),
            )?;
        }
        Ok(())
    })
    .await
    .unwrap_or_else(|error| Err(error.into()));
    let logout_result = row_logout(&mut client).await;
    logout_result?;
    row_result
}
