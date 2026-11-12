//! Drive the root-only fixture command, then reload via normal logout/admission.
use super::*;

pub(super) async fn provision(
    mut client: oteryn_dev_client::DevClientSession,
    clients: &SeamClients<'_>,
    index: usize,
) -> TestResult<oteryn_dev_client::DevClientSession> {
    let socket = required("NODE_BOOT_CONTROL_SOCKET")?;
    let profile = std::env::var("NODE_BOOT_PROFILE").unwrap_or_else(|_| "sorcerer".into());
    let vocation = if profile == "explorer" {
        "knight"
    } else {
        &profile
    };
    let (max_health, max_mana, _) = oteryn_gameplay_characters::vitals::maxima(
        vocation,
        100,
        oteryn_protocol_oteryn::actor_spell::MAX_VITAL_POOL,
        oteryn_protocol_oteryn::actor_spell::MAX_SOUL,
    )
    .ok_or("qualification profile has no source vitals")?;
    let session = uuid_text(client.join_snapshot().game_session_id.as_bytes());
    let script = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tools/qualification/node_boot/provision.py");
    let status = tokio::task::spawn_blocking(move || {
        std::process::Command::new("sudo")
            .args(["-n", "python3"])
            .arg(script)
            .args([
                "--socket",
                &socket,
                "--session",
                &session,
                "--profile",
                &profile,
            ])
            .status()
    })
    .await??;
    if !status.success() {
        return Err("node-boot fenced build provisioning failed".into());
    }
    let (result, logout, attempts, busy_timed_out) =
        client_world::logout_retrying_busy(&mut client).await?;
    if result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("fixture setup logout refused: {logout}; attempts={attempts}; busy_timed_out={busy_timed_out}").into());
    }
    drop(client);
    let mut client = connect_character(clients, 0x70, index).await?;
    let vitals = client.actor_vitals().ok_or("provisioned vitals missing")?;
    if vitals.max_health != max_health || vitals.max_mana != max_mana || vitals.mana == 0 {
        return Err("fresh admission did not load the provisioned build and mana".into());
    }
    let definitions = item_index()?;
    for id in [3276, 3003, 3457] {
        let reference = item_reference(&definitions, id)?;
        if !client.inventory().is_some_and(|inventory| {
            inventory
                .entries
                .iter()
                .any(|item| item.item_definition_ref == reference && item.count.get() == 1)
        }) {
            return Err(format!("fresh admission missing provisioned Item {id}").into());
        }
    }
    let reference = item_reference(&definitions, 3276)?;
    // Fenced provisioning installs an unforged copy; the session owns selected 485 encoding.
    let response = client
        .session_mut()
        .equip_item_definition(reference)
        .await?;
    if response.status != CommandStatus::Accepted
        || response.outcome != oteryn_protocol_oteryn::item_view::ItemMoveOutcome::Moved
    {
        return Err("provisioned axe equip refused".into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        if client.inventory().is_some_and(|inventory| {
            inventory
                .equipment
                .iter()
                .any(|entry| entry.item.item_definition_ref == reference)
        }) {
            break;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("provisioned axe equipment delta missing".into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
        client.take_events();
    }
    evidence("provisioning build=fenced level=100 items=axe,rope,shovel reload=fresh_admission");
    Ok(client)
}

pub(super) fn item_index() -> TestResult<crate::content::item_ref::ItemDefinitionIndex> {
    let keys = native_input()?
        .item_keys
        .as_ref()
        .ok_or("qualification Item keys missing")?;
    Ok(crate::content::item_ref::ItemDefinitionIndex::decode(
        &keys.bytes,
    )?)
}

pub(super) fn item_reference(
    index: &crate::content::item_ref::ItemDefinitionIndex,
    id: u32,
) -> TestResult<std::num::NonZeroU32> {
    let key = format!("oteryn:item.tibia.i{id}");
    let revision = index.revision(&key).ok_or("qualification Item absent")?;
    index
        .definition_ref("Item", &key, revision)
        .ok_or_else(|| "qualification Item ref missing".into())
}
