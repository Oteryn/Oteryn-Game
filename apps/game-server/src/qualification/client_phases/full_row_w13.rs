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

async fn query(
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
    client_hunt::hunt(client)
        .await
        .map_err(|error| format!("step-{step}: {target} combat/loot observed {error}"))?;
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
    let mut client = row_connect_world(clients, 0xd3).await?;
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
    if refused.status != CommandStatus::Rejected || !refused.payload.is_empty() {
        return Err(format!("step-3: malformed query observed reply {refused:?}").into());
    }
    evidence("W13_CHECK step=3 malformed-query-rejected");

    travel_to_spawn(
        &mut client,
        world,
        &metadata.spawns,
        &selected.creature_key,
    )
    .await
    .map_err(|error| format!("step-4: selected creature route/spawn observed {error}"))?;
    required_hunt(&mut client, 4, "selected creature").await?;
    evidence("W13_CHECK step=4 selected-creature-killed-and-looted");

    // A second real approach waits through the owner's respawn mechanism rather
    // than manufacturing an actor. The bounded helper reports the missing actor.
    travel_to_spawn(
        &mut client,
        world,
        &metadata.spawns,
        &selected.creature_key,
    )
    .await
    .map_err(|error| format!("step-5: boosted creature respawn observed {error}"))?;
    let step_5_observed = required_hunt(&mut client, 5, "respawned selected creature").await?;

    travel_to_spawn(&mut client, world, &metadata.spawns, &selected.boss_key)
        .await
        .map_err(|error| format!("step-6: selected boss route/spawn observed {error}"))?;
    let step_6_observed = required_hunt(&mut client, 6, "selected boss").await?;

    // The player protocol currently exposes neither the XP award nor which
    // loot rolls were bonus/unique, and it publishes no boss-respawn clock.
    // A successful generic hunt therefore cannot prove either numbered step.
    let unobservable_combat_deltas = format!(
        "step-5: boosted XP and extra-loot deltas are not observable; observed {step_5_observed}; \
         step-6: boss bonus/unique-loot and respawn-clock deltas are not observable; observed {step_6_observed}"
    );

    if client.row_logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("step-7: logout reply was not Accepted".into());
    }
    let mut fresh = row_connect_world(clients, 0xd4).await?;
    let durable = query(&mut fresh, 7).await?;
    if durable != selected {
        return Err(format!(
            "step-7: fresh admission changed durable view from {selected:?} to {durable:?}"
        )
        .into());
    }
    if fresh.row_logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("step-7: fresh-admission logout reply was not Accepted".into());
    }
    evidence("W13_CHECK step=7 durable-selection-rechecked");
    Err(unobservable_combat_deltas.into())
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = row_complete_with_timeout(Duration::from_secs(420), journey(clients)).await;
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
