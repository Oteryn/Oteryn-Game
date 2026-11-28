//! W05a captures the admitted domain-40 snapshot and the original countdown frame after
//! checking the local command-ownership refusal. The mandatory replay applies the captured
//! delta through the production session domain dispatcher.
use super::*;
use oteryn_dev_client::SessionEvent;
use oteryn_protocol_oteryn::server_save_countdown as wire;

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let mut client = connect_world(clients, 0xfe).await?;
    if !client
        .selected_capabilities()
        .contains(&wire::CAPABILITY_SERVER_SAVE_COUNTDOWN_V1)
    {
        return Err("admission did not select capability 75".into());
    }
    let snapshot = client
        .session_mut()
        .gated_snapshot(wire::STATE_DOMAIN_SERVER_SAVE_COUNTDOWN)
        .ok_or("domain-40 snapshot missing")?
        .clone();
    if snapshot.revision != 0 || snapshot.payload != wire::SERVER_SAVE_COUNTDOWN_SNAPSHOT {
        return Err(format!("unexpected countdown snapshot {snapshot:?}").into());
    }
    // Capability 75 is snapshot-only. Pairing it with core STEP (type 1)
    // deliberately probes local ownership rejection; this is no reset command
    // and must consume neither a command id nor any server sequence.
    let next_command = client.next_command_id();
    let sequence = client.last_server_sequence();
    let position = client.world_spatial().actor_position;
    let countdown_before = client
        .session_mut()
        .gated_snapshot(wire::STATE_DOMAIN_SERVER_SAVE_COUNTDOWN)
        .cloned();
    let refusal = client
        .session_mut()
        .gated_command(wire::CAPABILITY_SERVER_SAVE_COUNTDOWN_V1, 1, &[])
        .await;
    if !matches!(refusal, Err(oteryn_dev_client::SessionError::CapabilityNotSelected { capability })
        if capability == wire::CAPABILITY_SERVER_SAVE_COUNTDOWN_V1)
        || client.next_command_id() != next_command
        || client.last_server_sequence() != sequence
        || client.world_spatial().actor_position != position
        || client
            .session_mut()
            .gated_snapshot(wire::STATE_DOMAIN_SERVER_SAVE_COUNTDOWN)
            .cloned()
            != countdown_before
    {
        return Err(format!("step=local-route-refusal observed={refusal:?}").into());
    }
    evidence(
        "W05a_CHECK step=player-reset-boundary observed=local-ownership-refusal wire_submission=false command-and-sequence=unchanged countdown-and-position=unchanged",
    );

    client.take_events();
    client.session_mut().capture_server_save_frame();
    client.service_liveness(Duration::from_secs(7)).await?;
    let delta = client
        .take_events()
        .into_iter()
        .find_map(|event| match event {
            SessionEvent::ServerSaveCountdown(delta) => Some(delta),
            _ => None,
        })
        .ok_or("step=countdown observed=no-live-domain-40-delta")?;
    if delta.base_revision != snapshot.revision
        || delta.base_revision.checked_add(1) != Some(delta.new_revision)
        || delta.value.ending_epoch == 0
        || !(1..=wire::SERVER_SAVE_COUNTDOWN_MAX_MINUTES).contains(&delta.value.minutes_remaining)
    {
        return Err(format!("invalid countdown delta {delta:?}").into());
    }
    let frame = client
        .session_mut()
        .take_server_save_frame()
        .ok_or("countdown frame missing")?;
    let root = std::env::var_os("E2E_ALL_CAPTURE_DIR").ok_or("capture directory missing")?;
    let directory = Path::new(&root).join("W05a.client");
    std::fs::create_dir(&directory)?;
    let session = u128::from_be_bytes(*client.join_snapshot().game_session_id.as_bytes());
    let expected = format!(
        "snapshot_revision={}\ncountdown=present\nbase_revision={}\nnew_revision={}\nending_epoch={}\nminutes_remaining={}\nserver_sequence={}\nsession={session:032x}\n",
        snapshot.revision,
        delta.base_revision,
        delta.new_revision,
        delta.value.ending_epoch,
        delta.value.minutes_remaining,
        delta.server_sequence
    );
    std::fs::write(directory.join("w05a-snapshot.bin"), snapshot.payload)?;
    std::fs::write(directory.join("w05a-countdown.bin"), frame)?;
    std::fs::write(directory.join("w05a-expected.txt"), expected.as_bytes())?;
    Ok(expected.replace('\n', ";"))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match verdict::indexed_timeout(Duration::from_secs(120), verdict::observe(journey(clients)))
        .await
    {
        Ok(Ok(observed)) => evidence(&format!(
            "e2e_all scenario=W05a.client status=CAPTURED evidence={observed}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=W05a.client status=FAIL evidence={error}"
        )),
        Err(_) => evidence("e2e_all scenario=W05a.client status=FAIL evidence=timeout"),
    }
}
