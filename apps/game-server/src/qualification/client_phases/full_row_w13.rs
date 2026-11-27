//! W13 player scenario, derived from the matrix/re-audit and the daily-boost,
//! kill-reward, spawn-realization, and world-respawn owners:
//! 1. Admit the provisioned character and send DAILY_BOOST_QUERY; expect an
//!    accepted view naming the day's creature, boss, and reset epoch.
//! 2. Send the query again; expect the identical selection (the stable-day path).
//! 3. Send a malformed DAILY_BOOST_QUERY payload; expect a rejected reply and no view.
//! 4. Walk to the selected creature's authored spawn, attack it until death, open
//!    its corpse and move real loot; expect combat/entity/container/inventory deltas.
//! 5. Observe the selected creature respawn, kill and loot it again; the owner must
//!    apply boosted creature XP and extra loot rolls without a player shortcut.
//! 6. Walk to the selected boss, fight and loot it; expect boss bonus loot while
//!    its unique drop is not duplicated, followed by its owner-driven respawn clock.
//! 7. Log out, obtain a fresh admission, query again, and expect the same durable
//!    reset epoch and creature/boss selection.
//!
//! A runtime failure records the first uncompleted step and the actual reply or
//! missing delta. The journey never substitutes a source-only verdict for an attempt.
use super::*;
use oteryn_dev_client::client_runtime::raw_feature;
use oteryn_dev_client::client_ui::daily_boost::{self, DailyBoostPanel, Intent};
use oteryn_protocol_oteryn::daily_boost as wire;

pub(super) async fn query(
    client: &mut oteryn_dev_client::DevClientSession,
    step: u8,
) -> TestResult<wire::DailyBoostView> {
    let mut panel = DailyBoostPanel {
        selected: true,
        ..Default::default()
    };
    let command = daily_boost::request(
        &mut panel,
        Intent {
            command: wire::COMMAND_TYPE_DAILY_BOOST_QUERY,
            payload: Vec::new(),
        },
    )
    .ok_or_else(|| format!("step-{step}: client did not encode DAILY_BOOST_QUERY"))?;
    let reply = raw_feature::send(client.session_mut(), &command).await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("step-{step}: DAILY_BOOST_QUERY observed reply {reply:?}").into());
    }
    daily_boost::apply_result(&mut panel, &reply);
    panel
        .boost
        .ok_or_else(|| format!("step-{step}: accepted query had no decoded view delta").into())
}

async fn required_hunt(
    client: &mut oteryn_dev_client::DevClientSession,
    step: u8,
    target: &str,
) -> TestResult<String> {
    if !client
        .selected_capabilities()
        .contains(&oteryn_protocol_oteryn::world_map::CAPABILITY_WORLD_MAP_VIEW_V1)
    {
        return Err(format!(
            "step-{step}: {target} hunt cannot run; admission omitted world-map capability"
        )
        .into());
    }
    let before = client
        .inventory()
        .cloned()
        .ok_or_else(|| format!("step-{step}: {target} hunt had no inventory before combat"))?;
    client_hunt::hunt(client).await.map_err(|error| {
        verdict::record_error(verdict::IndexedError::as_error(&error));
        verdict::context(
            format!("step-{step}: {target} combat/loot observed {error}"),
            error,
        )
    })?;
    let after = client
        .inventory()
        .cloned()
        .ok_or_else(|| format!("step-{step}: {target} hunt had no inventory after looting"))?;
    if after == before {
        return Err(format!(
            "step-{step}: {target} hunt returned Ok but observed no inventory delta; inventory={after:?}"
        )
        .into());
    }
    Ok(format!(
        "hunt=Ok inventory-before={before:?} inventory-after={after:?}"
    ))
}

async fn prepare_hunter(
    clients: &SeamClients<'_>,
    mut client: oteryn_dev_client::DevClientSession,
) -> TestResult<oteryn_dev_client::DevClientSession> {
    use oteryn_dev_client::ItemMoveDestination;
    use oteryn_dev_client::client_runtime::inventory::MoveRequest;
    use oteryn_protocol_oteryn::item_view::{EquipmentSlot, ItemMoveOutcome};

    let session = client.join_snapshot().game_session_id;
    request_fixture_control(uuid_text(session.as_bytes()), "--profile", "q_w13".into()).await?;
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("W13 hunter preparation logout refused".into());
    }
    drop(client);
    let mut hunter = connect_world(clients, 0xd5).await?;
    if hunter.join_snapshot().game_session_id == session {
        return Err("W13 hunter preparation reused its original session".into());
    }
    use oteryn_protocol_oteryn::cyclopedia_character as stats;
    let reply = hunter.session_mut().gated_command(
        stats::CAPABILITY_CYCLOPEDIA_CHARACTER_V1,
        stats::COMMAND_TYPE_CYCLOPEDIA_CHARACTER_TAB,
        &[stats::CharacterTab::GeneralStats as u8, 0, 0, 1],
    ).await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("W13 GeneralStats refused: {reply:?}").into());
    }
    let data = &reply.payload;
    let vocation = general_stats::validate(data, general_stats::selected(&hunter))?;
    let level = u32::from_le_bytes(data[9..13].try_into()?);
    let sword = u16::from_le_bytes(data[19..21].try_into()?);
    if level != 200 || vocation != stats::vocation_id("knight")
        || sword != crate::qualification_w13::W13_SWORD_SKILL
    {
        return Err(format!("W13 hunter build differs: vocation={vocation:?} level={level} sword={sword}").into());
    }
    let index = provisioning::item_index()?;
    let definition = provisioning::item_reference(
        &index,
        crate::qualification_w13::W13_HUNTER_SWORD_ID,
    )?;
    let source = hunter.inventory().and_then(|inventory| {
        inventory.entries.iter().find(|entry| entry.item_definition_ref == definition)
    }).ok_or("W13 Relic Sword missing from backpack after readmission")?.handle;
    let moved = hunter.session_mut().move_item_counted(MoveRequest {
        source,
        destination: ItemMoveDestination::Equipment(EquipmentSlot::RightHand),
        count: None,
    }).await?;
    if moved.outcome != ItemMoveOutcome::Moved {
        return Err(format!("W13 fixture sword equip observed {moved:?}").into());
    }
    full_row_p05::wait_for_inventory(&mut hunter, "w13-fixture-sword-equip", |client| {
        client.inventory().is_some_and(|inventory| inventory.equipment.iter().any(|entry| {
            entry.slot == EquipmentSlot::RightHand && entry.item.item_definition_ref == definition
        }))
    }).await?;
    Ok(hunter)
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let mut client = connect_world(clients, 0xd3).await?;
    if !client
        .selected_capabilities()
        .contains(&wire::CAPABILITY_DAILY_BOOST_V1)
    {
        return Err("step-1: admission did not offer daily-boost capability 151".into());
    }
    let selected = query(&mut client, 1).await?;
    evidence(&format!(
        "W13_CHECK step=1 accepted epoch={} creature={} boss={}",
        selected.reset_epoch, selected.creature_key, selected.boss_key
    ));
    let repeated = query(&mut client, 2).await?;
    if repeated != selected {
        return Err(format!(
            "step-2: repeated query changed view from {selected:?} to {repeated:?}"
        )
        .into());
    }
    let refused = client
        .session_mut()
        .gated_command(
            wire::CAPABILITY_DAILY_BOOST_V1,
            wire::COMMAND_TYPE_DAILY_BOOST_QUERY,
            &[0],
        )
        .await?;
    if refused.status != CommandStatus::Rejected
        || wire::DailyBoostRefusal::decode(&refused.payload)
            != Ok(wire::DailyBoostRefusal::MalformedQuery)
    {
        return Err(format!("step-3: malformed query observed reply {refused:?}").into());
    }
    evidence("W13_CHECK step=3 malformed-query-rejected");
    let mut client = prepare_hunter(clients, client).await?;

    travel_to_spawn(&mut client, world, &metadata.spawns, &selected.creature_key)
        .await
        .map_err(|error| {
            verdict::record_error(verdict::IndexedError::as_error(&error));
            verdict::context(
                format!("step-4: selected creature route/spawn observed {error}"),
                error,
            )
        })?;
    required_hunt(&mut client, 4, "selected creature").await?;
    evidence("W13_CHECK step=4 selected-creature-killed-and-looted");

    // A second real approach waits through the owner's respawn mechanism rather
    // than manufacturing an actor. The bounded helper reports the missing actor.
    travel_to_spawn(&mut client, world, &metadata.spawns, &selected.creature_key)
        .await
        .map_err(|error| {
            verdict::record_error(verdict::IndexedError::as_error(&error));
            verdict::context(
                format!("step-5: boosted creature respawn observed {error}"),
                error,
            )
        })?;
    let step_5_observed = required_hunt(&mut client, 5, "respawned selected creature").await?;

    travel_to_spawn(&mut client, world, &metadata.spawns, &selected.boss_key)
        .await
        .map_err(|error| {
            verdict::record_error(verdict::IndexedError::as_error(&error));
            verdict::context(
                format!("step-6: selected boss route/spawn observed {error}"),
                error,
            )
        })?;
    let step_6_observed = required_hunt(&mut client, 6, "selected boss").await?;

    // The player protocol currently exposes neither the XP award nor which
    // loot rolls were bonus/unique, and it publishes no boss-respawn clock.
    // A successful generic hunt therefore cannot prove either numbered step.
    let unobservable_combat_deltas = format!(
        "step-5: boosted XP and extra-loot deltas are not observable; observed {step_5_observed}; \
         step-6: boss bonus/unique-loot and respawn-clock deltas are not observable; observed {step_6_observed}"
    );

    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("step-7: logout reply was not Accepted".into());
    }
    let mut fresh = connect_world(clients, 0xd4).await?;
    let durable = query(&mut fresh, 7).await?;
    if durable != selected {
        return Err(format!(
            "step-7: fresh admission changed durable view from {selected:?} to {durable:?}"
        )
        .into());
    }
    if fresh.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("step-7: fresh-admission logout reply was not Accepted".into());
    }
    evidence("W13_CHECK step=7 durable-selection-rechecked");
    Err(unobservable_combat_deltas.into())
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result =
        verdict::indexed_timeout(Duration::from_secs(420), verdict::observe(journey(clients)))
            .await;
    let (status, gap) = match result {
        Ok(Ok(())) => ("PASS", None),
        Ok(Err(error)) => ("FAIL", Some(error.to_string())),
        Err(_) => (
            "FAIL",
            Some("step-timeout: no required reply/delta within 420s".into()),
        ),
    };
    if let Some(gap) = gap {
        evidence(&format!("W13_GAP {gap}"));
    }
    evidence(&format!(
        "e2e_all scenario=W13.full status={status} seconds={:.3}",
        started.elapsed().as_secs_f64()
    ));
}
