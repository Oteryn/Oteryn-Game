//! G02: named Wheel builds and code exchange (apps/client 0148-wheel-allocation presets).
//! 1. Admit the node-boot character and refresh the native Wheel panel from the server.
//! 2. Propose a build in the panel, export its `OW1:` code, store it in a named preset, save the
//!    preset book to disk and load it back (client-local exchange), then reset the proposal and
//!    import the code from the reloaded preset.
//! 3. Press Save Wheel: the server must confirm the imported points with a revision advance.
//! 4. Log out, freshly admit, refresh: the durable allocation must equal the imported build.
//! 5. Reset the Wheel through the panel and confirm the zero allocation (cleanup, same route).
use super::full_row_p20::{approach_npc, talk};
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_dev_client::client_runtime::wheel_allocation;
use oteryn_dev_client::client_ui::wheel_allocation::{Intent, WheelPanel, apply_result, request};
use oteryn_protocol_oteryn::atelier::v2::CAPABILITY_GEM_ATELIER_V2;
use oteryn_protocol_oteryn::wheel::CAPABILITY_WHEEL_ALLOCATION_V1;

const CORE_SLOT: usize = 20;
const POINTS: u16 = 10;
const LEVEL: u32 = 60;

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result =
        verdict::indexed_timeout(Duration::from_secs(120), verdict::observe(journey(clients)))
            .await;
    let (status, detail) = match result {
        Ok(Ok(())) => ("PASS", "named-build-code-saved-and-durable".to_owned()),
        Ok(Err(error)) => ("FAIL", token(&error.to_string())),
        Err(_) => ("FAIL", "step=journey;observed=timeout".to_owned()),
    };
    evidence(&format!(
        "e2e_all scenario=G02.full status={status} seconds={:.3} evidence={detail}",
        started.elapsed().as_secs_f64()
    ));
}

fn token(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join("-")
}

/// Send one panel intent through the real session and apply the server answer to the panel.
async fn press(
    client: &mut DevClientSession,
    panel: &mut WheelPanel,
    intent: Intent,
    lane: u8,
    step: &str,
) -> TestResult {
    let command = request(panel, intent, v7(0x92, lane))
        .ok_or_else(|| format!("step={step} observed=panel-refused:{:?}", panel.feedback))?;
    let response = wheel_allocation::send(client.session_mut(), &command).await?;
    if !matches!(
        response.status,
        CommandStatus::Accepted | CommandStatus::DuplicateReplay
    ) {
        return Err(format!("step={step} observed=status:{:?}", response.status).into());
    }
    if !apply_result(panel, &response) {
        return Err(format!("step={step} observed=view-not-applied:{:?}", panel.feedback).into());
    }
    Ok(())
}

fn admitted_panel(client: &DevClientSession) -> WheelPanel {
    let capabilities = client.selected_capabilities();
    let selected = capabilities.contains(&CAPABILITY_WHEEL_ALLOCATION_V1);
    WheelPanel {
        selected,
        atelier_v2: selected && capabilities.contains(&CAPABILITY_GEM_ATELIER_V2),
        ..WheelPanel::default()
    }
}

async fn logout(client: &mut DevClientSession) -> TestResult {
    let reply = client.session_mut().logout().await?;
    if reply.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err("step=logout observed=refused".into());
    }
    Ok(())
}

async fn prepare(clients: &SeamClients<'_>) -> TestResult<DevClientSession> {
    character_prep::declare_seeded(
        "G02",
        "level-60-sorcerer,Premium,promotion-fee",
        "ordinary leveling is outside the named Wheel build journey budget",
    );
    let mut client = connect_character(clients, 0xdf, 0).await?;
    request_fixture_control(
        uuid_text(client.join_snapshot().game_session_id.as_bytes()),
        "--profile",
        "g02-wheel".into(),
    )
    .await?;
    character_prep::grant_item(&client, 3043, 2).await?;
    character_prep::grant_premium(&client).await?;
    logout(&mut client).await?;
    drop(client);

    let mut client = connect_character(clients, 0xe0, 0).await?;
    let vitals = client
        .actor_vitals()
        .ok_or("step=prepare observed=no-vitals")?;
    let (health, mana, _) = oteryn_gameplay_characters::vitals::maxima(
        "sorcerer",
        LEVEL,
        oteryn_protocol_oteryn::actor_spell::MAX_VITAL_POOL,
        oteryn_protocol_oteryn::actor_spell::MAX_SOUL,
    )
    .ok_or("step=prepare observed=no-level-60-vital-maxima")?;
    if (vitals.max_health, vitals.max_mana) != (health, mana) {
        return Err(format!(
            "step=prepare observed={vitals:?};needed=max-health:{health},max-mana:{mana}"
        )
        .into());
    }
    evidence(&format!(
        "G02_CHECK step=derived-vitals level={LEVEL} max_health={health} max_mana={mana}"
    ));
    let world = full_e2ef7::qualification_world(clients)?;
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let king = approach_npc(
        &mut client,
        world,
        &metadata.npcs,
        "oteryn:npc.king_tibianus",
    )
    .await?;
    if talk(&mut client, king, "hi").await?.is_empty()
        || talk(&mut client, king, "promotion").await?
            != ["Do you want to be promoted for 20000 gold coins?"]
        || talk(&mut client, king, "yes").await? != ["You are now promoted."]
    {
        return Err("step=prepare observed=promotion-dialogue-did-not-confirm".into());
    }
    logout(&mut client).await?;
    drop(client);
    connect_character(clients, 0xe1, 0).await
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let identity = character_prep::dedicated_character(clients, "NODE_BOOT_G02_ACCOUNT").await?;
    let clients = &identity.clients(clients);
    let mut client = prepare(clients).await?;
    let mut panel = admitted_panel(&client);
    if !panel.selected {
        return Err("step=admit observed=wheel-capability-not-selected".into());
    }
    press(&mut client, &mut panel, Intent::Refresh, 1, "refresh").await?;
    let initial = panel
        .confirmed_wheel
        .clone()
        .ok_or("step=refresh observed=no-confirmed-wheel")?;
    if initial.points != [0; 36] {
        return Err(format!(
            "step=refresh observed=non-zero-start:{:?};needed=clean-wheel",
            initial.points
        )
        .into());
    }

    panel.points[CORE_SLOT] = POINTS;
    let code = panel.build_code();
    panel.preset_book.insert("g02-build", &code)?;
    let path = std::env::temp_dir().join(format!("oteryn-g02-presets-{}.json", std::process::id()));
    panel.preset_book.save(&path)?;
    let loaded = WheelPanel::default().preset_book;
    let reloaded =
        oteryn_dev_client::client_ui::wheel_allocation::presets::PresetBook::load(&path)?;
    let _ = std::fs::remove_file(&path);
    if reloaded == loaded || reloaded.0.get("g02-build") != Some(&code) {
        return Err("step=preset-file observed=preset-not-round-tripped".into());
    }
    panel.points = initial.points;
    panel.import_build(
        reloaded
            .0
            .get("g02-build")
            .ok_or("step=import observed=no-preset")?,
    )?;
    if panel.points[CORE_SLOT] != POINTS {
        return Err("step=import observed=points-not-loaded".into());
    }
    // `apply_result` overwrites `panel.points` with the server answer, so keep the import.
    let imported = panel.points;
    press(&mut client, &mut panel, Intent::Save, 2, "save").await?;
    let saved = panel
        .confirmed_wheel
        .clone()
        .ok_or("step=save observed=no-wheel")?;
    if saved.refused || saved.revision != initial.revision + 1 || saved.points != imported {
        return Err(format!("step=save observed={saved:?};needed=imported-build-confirmed").into());
    }
    logout(&mut client).await?;
    drop(client);

    let mut fresh = connect_character(clients, 0xe2, 0).await?;
    let mut again = admitted_panel(&fresh);
    press(&mut fresh, &mut again, Intent::Refresh, 3, "fresh-refresh").await?;
    let durable = again
        .confirmed_wheel
        .clone()
        .ok_or("step=fresh observed=no-wheel")?;
    if durable.points != imported || durable.revision != saved.revision {
        return Err(format!("step=fresh observed={durable:?};needed={saved:?}").into());
    }
    press(&mut fresh, &mut again, Intent::Reset, 4, "reset").await?;
    let cleared = again
        .confirmed_wheel
        .clone()
        .ok_or("step=reset observed=no-wheel")?;
    logout(&mut fresh).await?;
    if cleared.refused || cleared.points != [0; 36] {
        return Err(format!("step=reset observed={cleared:?};needed=zero-allocation").into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_code_round_trips_through_preset_book() -> TestResult {
        let mut panel = WheelPanel::default();
        panel.points[CORE_SLOT] = POINTS;
        let code = panel.build_code();
        panel.preset_book.insert("named", &code)?;
        assert_eq!(panel.preset_book.0.get("named"), Some(&code));
        assert!(panel.preset_book.insert("bad", "not-a-code").is_err());
        Ok(())
    }
}
