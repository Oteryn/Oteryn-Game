//! I17d: hotkey equip, jewelry delay and the feared gate.
//! The disposable character receives its amulet before the wire checks.
//! The row requires an authored player-facing fear source to complete.
use super::*;
use oteryn_dev_client::{DevClientSession, EntityKind};
use oteryn_protocol_oteryn::CommandStatus;
use oteryn_protocol_oteryn::attack::AttackIntentDisposition;
use oteryn_protocol_oteryn::condition_status::{
    CAPABILITY_CONDITION_STATUS_V1, COMMAND_TYPE_CONDITION_STATUS_QUERY, ConditionStatus,
};
use oteryn_protocol_oteryn::item_view::EquipmentSlot;

const COMBAT_APPROACH: TilePos = TilePos {
    x: 33760,
    y: 32830,
    floor: -14,
};

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result =
        verdict::indexed_timeout(Duration::from_secs(300), verdict::observe(journey(clients)))
            .await;
    let (status, detail) = match result {
        Ok(Ok(detail)) => ("PASS", token(&detail)),
        Ok(Err(error)) => ("FAIL", token(&error.to_string())),
        Err(_) => ("FAIL", "step=journey;observed=timeout".to_owned()),
    };
    evidence(&format!(
        "e2e_all scenario=I17d.full status={status} seconds={:.3} evidence={detail}",
        started.elapsed().as_secs_f64()
    ));
}

fn token(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join("-")
}

fn quantity(client: &DevClientSession, definition: std::num::NonZeroU32) -> TestResult<u64> {
    Ok(client
        .inventory()
        .ok_or("inventory snapshot absent")?
        .entries
        .iter()
        .filter(|item| item.item_definition_ref == definition)
        .map(|item| u64::from(item.count.get()))
        .sum())
}

async fn logout(client: &mut DevClientSession) -> TestResult {
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("logout refused".into());
    }
    Ok(())
}

fn reference(id: u32) -> TestResult<std::num::NonZeroU32> {
    let index = provisioning::item_index()?;
    provisioning::item_reference(&index, id)
}

fn worn(client: &DevClientSession, definition: std::num::NonZeroU32) -> Option<EquipmentSlot> {
    client
        .inventory()?
        .equipment
        .iter()
        .find(|entry| entry.item.item_definition_ref == definition)
        .map(|entry| entry.slot)
}

async fn settle(
    client: &mut DevClientSession,
    definition: std::num::NonZeroU32,
    want_worn: bool,
) -> TestResult<()> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(6);
    loop {
        client.take_events();
        if worn(client, definition).is_some() == want_worn {
            return Ok(());
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!(
                "step=settle observed=worn:{:?};wanted={want_worn}",
                worn(client, definition)
            )
            .into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
    }
}

async fn hotkey(
    client: &mut DevClientSession,
    definition: std::num::NonZeroU32,
) -> TestResult<oteryn_dev_client::ItemMoveOutcome> {
    client.take_events();
    Ok(
        oteryn_dev_client::client_runtime::hotkey_equip::equip_definition(
            client.session_mut(),
            definition,
        )
        .await?,
    )
}

async fn expect(
    client: &mut DevClientSession,
    definition: std::num::NonZeroU32,
    step: &str,
    want: oteryn_dev_client::ItemMoveOutcome,
) -> TestResult<()> {
    let outcome = hotkey(client, definition).await?;
    if outcome != want {
        return Err(format!("step={step} observed={outcome:?};needed={want:?}").into());
    }
    Ok(())
}

async fn relog(
    mut client: DevClientSession,
    clients: &SeamClients<'_>,
) -> TestResult<DevClientSession> {
    let original = client.join_snapshot().game_session_id;
    logout(&mut client).await?;
    drop(client);
    wait_character_released(clients, 0).await?;
    let fresh = connect_character(clients, 0xe4, 0).await?;
    if fresh.join_snapshot().game_session_id == original {
        return Err("relog reused the original session".into());
    }
    Ok(fresh)
}

async fn wait_for(client: &mut DevClientSession, window: Duration) -> TestResult<()> {
    let deadline = tokio::time::Instant::now() + window;
    verdict::Harness::global().bounded_wait(deadline, "bounded-observation");
    while tokio::time::Instant::now() < deadline {
        client.service_liveness(Duration::from_millis(100)).await?;
    }
    Ok(())
}

async fn feared(client: &mut DevClientSession) -> TestResult<bool> {
    let reply = client
        .session_mut()
        .gated_command(
            CAPABILITY_CONDITION_STATUS_V1,
            COMMAND_TYPE_CONDITION_STATUS_QUERY,
            &[],
        )
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("condition status refused: {:?}", reply.status).into());
    }
    let status = ConditionStatus::decode(&reply.payload).ok_or("invalid condition status")?;
    Ok(status
        .conditions
        .iter()
        .any(|condition| condition.icon == 12))
}

async fn fight_fear_caster(
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    amulet: std::num::NonZeroU32,
) -> TestResult<()> {
    let world = load_world(
        std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?,
        &serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let start = tile(client.world_spatial().actor_position)?;
    let path = route(world, start, |at| at == COMBAT_APPROACH)
        .ok_or_else(|| format!("no walkable route from {start:?} to {COMBAT_APPROACH:?}"))?;
    walk(client, world, &path).await?;
    let target = client
        .world_entities()
        .into_iter()
        .flatten()
        .filter(|entity| {
            entity.kind == EntityKind::Creature
                && entity
                    .display_name
                    .is_some_and(|name| name.as_str() == Ok("Shrieking Cry-Stal"))
                && entity.position.floor == i16::from(COMBAT_APPROACH.floor)
        })
        .min_by_key(|entity| {
            entity
                .position
                .x
                .abs_diff(i32::from(COMBAT_APPROACH.x))
                .max(entity.position.y.abs_diff(i32::from(COMBAT_APPROACH.y)))
        })
        .map(|entity| entity.entity)
        .ok_or("no live Shrieking Cry-Stal at authored approach")?;
    if feared(client).await? {
        return Err("Feared was already active before attack selection".into());
    }
    let attack = client.session_mut().attack_target(Some(&target)).await?;
    if attack.status != CommandStatus::Accepted || attack.disposition != AttackIntentDisposition::Ok
    {
        return Err(format!("Shrieking Cry-Stal attack refused: {attack:?}").into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(120);
    verdict::Harness::global().bounded_wait(deadline, "authored-fear-cast");
    while !feared(client).await? {
        if tokio::time::Instant::now() >= deadline {
            return Err("Shrieking Cry-Stal combat did not produce wire Feared icon 12".into());
        }
        client.service_liveness(Duration::from_millis(100)).await?;
    }
    expect(
        client,
        amulet,
        "feared-equip",
        oteryn_dev_client::ItemMoveOutcome::Rejected,
    )
    .await?;
    if !feared(client).await? || worn(client, amulet).is_some() || quantity(client, amulet)? != 1 {
        return Err(
            "Feared equip refusal did not preserve carried custody within the condition window"
                .into(),
        );
    }
    evidence("I17D_CHECK feared-equip PASS caster=Shrieking-Cry-Stal icon=12 outcome=Rejected");
    Ok(())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    use oteryn_dev_client::ItemMoveOutcome as Out;
    let identity = character_prep::dedicated_character(clients, "NODE_BOOT_I17D_ACCOUNT").await?;
    let clients = &identity.clients(clients);
    let amulet = reference(3572)?;
    let client = connect_character(clients, 0xe4, 0).await?;
    if quantity(&client, amulet)? != 0 || worn(&client, amulet).is_some() {
        return Err("step=prerequisite observed=amulet-already-present".into());
    }
    character_prep::grant_item(&client, 3572, 1).await?;
    let mut client = relog(client, clients).await?;
    if quantity(&client, amulet)? != 1 || worn(&client, amulet).is_some() {
        return Err("step=prerequisite needed=one-carried-amulet-after-relog".into());
    }
    evidence("I17D_CHECK prerequisite PASS fixture=one-carried-amulet-after-relog");

    expect(&mut client, amulet, "equip-amulet", Out::Moved).await?;
    let repeated_at = tokio::time::Instant::now();
    let refused = hotkey(&mut client, amulet).await?;
    if refused != Out::Rejected {
        return Err(format!(
            "step=immediate-repeat observed={refused:?};elapsed_ms={};needed=Rejected-and-still-worn",
            repeated_at.elapsed().as_millis()
        ).into());
    }
    settle(&mut client, amulet, true).await?;
    let slot = worn(&client, amulet);
    if slot != Some(EquipmentSlot::Necklace) || quantity(&client, amulet)? != 0 {
        return Err(format!(
            "step=immediate-repeat observed=slot:{slot:?};needed=Necklace-and-no-carried-copy"
        )
        .into());
    }
    // Crystal 201189f8 configmanager.cpp:377 defaults the necklace clock to 200 ms.
    wait_for(&mut client, Duration::from_millis(200)).await?;
    expect(&mut client, amulet, "after-delay", Out::Moved).await?;
    settle(&mut client, amulet, false).await?;
    let mut client = relog(client, clients).await?;
    if worn(&client, amulet).is_some() || quantity(&client, amulet)? != 1 {
        return Err("step=durable-reload needed=one-carried-amulet-and-empty-necklace-slot".into());
    }
    evidence("I17D_CHECK jewelry-delay PASS immediate=Rejected recovery=Moved relog=carried");
    fight_fear_caster(&mut client, clients, amulet).await?;
    drop(client);
    wait_character_released(clients, 0).await?;
    Ok("jewelry-delay-and-live-feared-gate-proven".to_owned())
}
