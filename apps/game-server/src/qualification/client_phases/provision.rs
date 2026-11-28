//! Drive the root-only fixture command, then reload via normal logout/admission.
use super::*;

pub(super) async fn provision(
    client: oteryn_dev_client::DevClientSession,
    clients: &SeamClients<'_>,
    index: usize,
) -> TestResult<oteryn_dev_client::DevClientSession> {
    provision_at_level(client, clients, index, 100).await
}

pub(super) async fn provision_at_level(
    client: oteryn_dev_client::DevClientSession,
    clients: &SeamClients<'_>,
    index: usize,
    expected_level: u32,
) -> TestResult<oteryn_dev_client::DevClientSession> {
    let profile = std::env::var("NODE_BOOT_PROFILE").unwrap_or_else(|_| "sorcerer".into());
    provision_profile(client, clients, index, expected_level, &profile, false).await
}

pub(in crate::qualification) async fn provision_once(
    client: oteryn_dev_client::DevClientSession,
    clients: &SeamClients<'_>,
    index: usize,
    expected_level: u32,
) -> TestResult<oteryn_dev_client::DevClientSession> {
    let profile = std::env::var("NODE_BOOT_PROFILE").unwrap_or_else(|_| "sorcerer".into());
    provision_profile(client, clients, index, expected_level, &profile, true).await
}

async fn provision_profile(
    mut client: oteryn_dev_client::DevClientSession,
    clients: &SeamClients<'_>,
    index: usize,
    expected_level: u32,
    profile: &str,
    require_first_logout: bool,
) -> TestResult<oteryn_dev_client::DevClientSession> {
    let socket = required("NODE_BOOT_CONTROL_SOCKET")?;
    let vocation = if profile == "explorer" {
        "knight"
    } else {
        profile
    };
    let (max_health, max_mana, _) = oteryn_gameplay_characters::vitals::maxima(
        vocation,
        expected_level,
        oteryn_protocol_oteryn::actor_spell::MAX_VITAL_POOL,
        oteryn_protocol_oteryn::actor_spell::MAX_SOUL,
    )
    .ok_or("qualification profile has no source vitals")?;
    let session = uuid_text(client.join_snapshot().game_session_id.as_bytes());
    let script = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tools/qualification/node_boot/provision.py");
    let profile = profile.to_owned();
    let status = tokio::task::spawn_blocking(move || {
        crate::qualification::provision_python()
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
    let original_session = client.join_snapshot().game_session_id;
    if require_first_logout {
        let reply = client.session_mut().logout().await?;
        evidence(&format!("P02_SETUP_LOGOUT attempt=1 reply={reply:?}"));
        require_setup_logout(reply.result)?;
    } else {
        let (result, logout, attempts, busy_timed_out) =
            client_world::logout_retrying_busy(&mut client).await?;
        if result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
            return Err(format!("fixture setup logout refused: {logout}; attempts={attempts}; busy_timed_out={busy_timed_out}").into());
        }
    }
    drop(client);
    let mut client = connect_character(clients, 0x70, index).await?;
    if require_first_logout && client.join_snapshot().game_session_id == original_session {
        return Err("P02 setup did not obtain a fresh session".into());
    }
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
                .chain(inventory.equipment.iter().map(|slot| &slot.item))
                .any(|item| item.item_definition_ref == reference && item.count.get() == 1)
        }) {
            return Err(format!("fresh admission missing provisioned Item {id}").into());
        }
    }
    let reference = item_reference(&definitions, 3276)?;
    // Fenced provisioning installs an unforged copy; the session owns selected 485 encoding.
    if !client.inventory().is_some_and(|inventory| {
        inventory
            .equipment
            .iter()
            .any(|entry| entry.item.item_definition_ref == reference)
    }) {
        let response = client
            .session_mut()
            .equip_item_definition(reference)
            .await?;
        if response.status != CommandStatus::Accepted
            || response.outcome != oteryn_protocol_oteryn::item_view::ItemMoveOutcome::Moved
        {
            return Err("provisioned axe equip refused".into());
        }
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
    evidence(&format!(
        "provisioning build=fenced level={expected_level} items=axe,rope,shovel reload=fresh_admission"
    ));
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

fn require_setup_logout(
    result: Option<oteryn_protocol_oteryn::logout::LogoutResult>,
) -> TestResult {
    use oteryn_protocol_oteryn::logout::LogoutResult;
    if result != Some(LogoutResult::Accepted) {
        return Err(format!("P02 first setup logout refused: {result:?}").into());
    }
    Ok(())
}

#[test]
fn p02_setup_requires_first_reply_to_be_accepted() {
    use oteryn_protocol_oteryn::logout::LogoutResult;
    assert!(require_setup_logout(Some(LogoutResult::Busy)).is_err());
    assert!(require_setup_logout(None).is_err());
    assert!(require_setup_logout(Some(LogoutResult::Accepted)).is_ok());
}
