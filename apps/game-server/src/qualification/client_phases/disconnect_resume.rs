//! Recovery over a fresh real TLS connection using the existing server protocol.
use super::*;

pub(super) async fn recover(
    client: oteryn_dev_client::DevClientSession,
    clients: &SeamClients<'_>,
    expected_after_step: Option<ActorPosition>,
) -> TestResult<oteryn_dev_client::DevClientSession> {
    let session = *client.join_snapshot().game_session_id.as_bytes();
    let position = client.world_spatial().actor_position;
    let next = client.next_command_id();
    let sequence = client.last_server_sequence();
    let admissions = committed_admissions(clients.url).await?;
    let previous = client.disconnect_after_step(StepDirection::West).await?;
    evidence("mechanic=disconnect_resume phase=pending_step_written_transport_dropped");
    let deadline = tokio::time::Instant::now() + Duration::from_secs(60);
    loop {
        if matches!(
            session_loss_row(clients.url, session).await?,
            Some((1, _, _))
        ) {
            break;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("dev client never became reconnectable".into());
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    evidence("mechanic=disconnect_resume phase=durable_loss_observed");
    let generation = platform_generation(clients.descriptor, &clients.accounts[1]).await?;
    let token = sign_recovery(
        clients.recovery,
        &clients.accounts[1],
        &clients.characters[1],
        &clients.world,
        generation,
        now_seconds()?,
        [0x72; 32],
    );
    let mut resumed = oteryn_dev_client::resume_session(
        oteryn_dev_client::RecoveryTransport {
            address: clients.address,
            server_name: "localhost",
            root_certificate: clients.certificate,
            deadline: Duration::from_secs(20),
        },
        previous,
        oteryn_dev_client::Recovery {
            reconnect_material: token.as_bytes(),
            schema_revision: 1,
            client_build_id: "oteryn-dev-client/recovery-qualification",
            deadline: Duration::from_secs(20),
            appearance_items: None,
        },
    )
    .await?;
    evidence("mechanic=disconnect_resume phase=replacement_transport_joined");
    if resumed.join_snapshot().game_session_id.as_bytes() != &session
        || (resumed.world_spatial().actor_position != position
            && Some(resumed.world_spatial().actor_position) != expected_after_step)
        || resumed.next_command_id() != next.checked_add(1).ok_or("pending step id exhausted")?
        || resumed.last_server_sequence() < sequence
        || committed_admissions(clients.url).await? != admissions
    {
        return Err(
            "dev client recovery changed session, position, command continuity or admissions"
                .into(),
        );
    }
    client_party::party(&mut resumed).await?;
    if resumed.next_command_id() <= next || resumed.last_server_sequence() <= sequence {
        return Err("resumed command did not advance the original sequence".into());
    }
    evidence(&format!(
        "mechanic=disconnect_resume state=pending_step_resumed position={:?} next_command_id={} server_sequence={} admissions={admissions}",
        resumed.world_spatial().actor_position,
        resumed.next_command_id(),
        resumed.last_server_sequence()
    ));
    Ok(resumed)
}
