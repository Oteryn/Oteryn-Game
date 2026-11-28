//! Live Exiva whitelist, relog and client replay.
use super::*;
use oteryn_dev_client::{ChatDisposition, DevClientSession};
use oteryn_protocol_oteryn::{exiva, logout::LogoutResult};

const INITIAL_MODES: u8 = 30;
const WHITELIST_ONLY: u8 = 2;
const REPLAY_TEST: &str =
    "exiva_settings::e2e_c05::e2e_c05_replays_exiva_mode_change_and_player_list";

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let result =
        verdict::indexed_timeout(Duration::from_secs(60), verdict::observe(journey(clients))).await;
    match result.unwrap_or_else(|_| Err("C05 client qualification timed out".into())) {
        Ok(()) => evidence(
            "e2e_all scenario=C05.client status=PASS evidence=exiva-whitelist-relog-and-live-ui-replay",
        ),
        Err(error) => evidence(&format!(
            "e2e_all scenario=C05.client status=FAIL evidence={error}"
        )),
    }
}

async fn settings(
    client: &mut DevClientSession,
    payload: &[u8],
) -> TestResult<oteryn_dev_client::native_session::GatedCommandResult> {
    let reply = client
        .session_mut()
        .gated_command(
            exiva::CAPABILITY_EXIVA_SETTINGS_V1,
            exiva::COMMAND_TYPE_EXIVA_SETTINGS,
            payload,
        )
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!(
            "C05 settings opcode={} refused: {:?}",
            payload[0], reply.status
        )
        .into());
    }
    Ok(reply)
}

async fn require_list(
    client: &mut DevClientSession,
    modes: u8,
    players: &[([u8; 16], String)],
) -> TestResult<oteryn_dev_client::native_session::GatedCommandResult> {
    let reply = settings(client, &[5]).await?;
    let list = exiva::PlayerList::decode(&reply.payload).ok_or("C05 server List undecodable")?;
    if list.modes != modes || list.players != players {
        return Err(format!("C05 expected modes={modes} players={players:?}; got {list:?}").into());
    }
    Ok(reply)
}

async fn logout(client: &mut DevClientSession) -> TestResult {
    let reply = client.session_mut().logout().await?;
    if reply.result != Some(LogoutResult::Accepted) {
        return Err(format!("C05 expected accepted logout, got {:?}", reply.result).into());
    }
    Ok(())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let capture_root =
        std::env::var_os("E2E_ALL_CAPTURE_DIR").ok_or("C05 capture directory required")?;
    let replay_binary =
        std::env::var_os("OTERYN_C05_REPLAY_BIN").ok_or("C05 replay executable required")?;
    let caster_identity =
        character_prep::dedicated_character(clients, "NODE_BOOT_EXIVA_ACCOUNT_ID").await?;
    let target_identity =
        character_prep::dedicated_character(clients, "NODE_BOOT_C05_TARGET_ACCOUNT").await?;
    let caster_clients = caster_identity.clients(clients);
    let target_clients = target_identity.clients(clients);
    character_prep::declare_seeded("C05.client", "level-8-exiva-build", "learn and cast Exiva");
    let mut caster = full_row_c05::connect_caster(&caster_clients).await?;
    let caster_name = full_row_c05::own_name(&caster)?;
    let caster_id = caster_clients.characters[0];
    let mut target = connect_character(&target_clients, 0xc6, 0).await?;
    let target_name = full_row_c05::own_name(&target)?;
    let old_session = target.join_snapshot().game_session_id;
    require_list(&mut target, INITIAL_MODES, &[]).await?;
    if settings(&mut target, &[0, WHITELIST_ONLY]).await?.payload != [1] {
        return Err("C05 SetModes lacked an accepted receipt".into());
    }
    let modes = require_list(&mut target, WHITELIST_ONLY, &[]).await?;
    let before_refusal = caster
        .actor_vitals()
        .copied()
        .ok_or("C05 caster vitals absent")?;
    let (refusal, feedback) = full_row_c05::cast(&mut caster, &target_name).await?;
    if refusal.status != CommandStatus::Rejected
        || refusal.disposition != ChatDisposition::Rejected
        || caster.actor_vitals() != Some(&before_refusal)
        || feedback
            != [
                "The character you are trying to find with Exiva is currently protected from your spell.",
            ]
    {
        return Err(format!("C05 empty whitelist expected protected refusal and unchanged vitals; got {refusal:?} {feedback:?}").into());
    }

    let mut add = vec![6];
    add.extend(caster_name.as_bytes());
    if settings(&mut target, &add).await?.payload != [1] {
        return Err("C05 AddName lacked an accepted receipt".into());
    }
    let expected = [(caster_id, caster_name.clone())];
    let added = require_list(&mut target, WHITELIST_ONLY, &expected).await?;
    if modes.server_sequence >= added.server_sequence {
        return Err("C05 List sequence did not advance".into());
    }
    logout(&mut target).await?;
    drop(target);
    let mut target = connect_character(&target_clients, 0xc7, 0).await?;
    if target.join_snapshot().game_session_id == old_session {
        return Err("C05 relog reused the settings session".into());
    }
    require_list(&mut target, WHITELIST_ONLY, &expected).await?;

    let caster_position = caster.world_spatial().actor_position;
    let target_position = target.world_spatial().actor_position;
    if caster_position.floor != target_position.floor
        || caster_position
            .x
            .abs_diff(target_position.x)
            .max(caster_position.y.abs_diff(target_position.y))
            >= 5
    {
        return Err(format!(
            "C05 expected adjacent spawn positions, got {caster_position:?} and {target_position:?}"
        )
        .into());
    }
    let before = caster
        .actor_vitals()
        .copied()
        .ok_or("C05 caster vitals absent")?;
    let (locate, feedback) = full_row_c05::cast(&mut caster, &target_name).await?;
    let after = caster
        .actor_vitals()
        .copied()
        .ok_or("C05 post-cast vitals absent")?;
    if locate.status != CommandStatus::Accepted
        || locate.disposition != ChatDisposition::Ok
        || before.mana.checked_sub(20) != Some(after.mana)
        || feedback != [format!("{target_name} is standing next to you.")]
    {
        return Err(format!(
            "C05 persisted whitelist locate failed: {locate:?} mana={}->{} {feedback:?}",
            before.mana, after.mana
        )
        .into());
    }

    let mut remove = vec![2];
    remove.extend(caster_id);
    if settings(&mut target, &remove).await?.payload != [1] {
        return Err("C05 RemovePlayer lacked an accepted receipt".into());
    }
    require_list(&mut target, WHITELIST_ONLY, &[]).await?;
    if settings(&mut target, &[0, INITIAL_MODES]).await?.payload != [1] {
        return Err("C05 mode restore lacked an accepted receipt".into());
    }
    require_list(&mut target, INITIAL_MODES, &[]).await?;
    logout(&mut target).await?;
    logout(&mut caster).await?;

    let session = uuid_text(old_session.as_bytes());
    let capture = Path::new(&capture_root).join(format!("c05-{session}"));
    std::fs::create_dir(&capture)?;
    std::fs::write(capture.join("c05-modes.bin"), modes.payload)?;
    std::fs::write(capture.join("c05-list.bin"), added.payload)?;
    let hex: String = caster_id.iter().map(|byte| format!("{byte:02x}")).collect();
    std::fs::write(
        capture.join("c05-expected.txt"),
        format!(
            "modes={WHITELIST_ONLY}\nplayer={hex} {caster_name}\nsession={session}\nsequences={}:{}\n",
            modes.server_sequence, added.server_sequence
        ),
    )?;
    let sequences = format!("{}:{}", modes.server_sequence, added.server_sequence);
    let output = tokio::task::spawn_blocking(move || {
        std::process::Command::new(replay_binary)
            .env("E2E_ALL_CAPTURE_DIR", capture)
            .env("OTERYN_C05_REPLAY_SESSION", session)
            .env("OTERYN_C05_REPLAY_SEQUENCES", sequences)
            .args([REPLAY_TEST, "--exact", "--ignored"])
            .output()
    })
    .await??;
    let stdout = String::from_utf8(output.stdout)?;
    if !output.status.success()
        || !stdout.contains("test result: ok. 1 passed; 0 failed; 0 ignored;")
    {
        return Err(format!(
            "C05 UI replay failed: {stdout} {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    evidence(&format!("C05_REPLAY PASS test={REPLAY_TEST}"));
    Ok(())
}
