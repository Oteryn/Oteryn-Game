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
    let logout = client.session_mut().logout().await?;
    if logout.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err("fixture setup logout refused".into());
    }
    drop(client);
    let client = connect_character(clients, 0x70, index).await?;
    let vitals = client.actor_vitals().ok_or("provisioned vitals missing")?;
    if vitals.max_health != max_health || vitals.max_mana != max_mana || vitals.mana == 0 {
        return Err("fresh admission did not load the provisioned build and mana".into());
    }
    evidence("provisioning build=fenced level=100 reload=fresh_admission");
    Ok(client)
}
