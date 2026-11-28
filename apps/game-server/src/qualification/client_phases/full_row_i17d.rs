//! I17d: hotkey equip, jewelry delay and the feared gate.
//! The disposable character receives its amulet before the wire checks.
//! The row requires an authored player-facing fear source to complete.
use super::*;
use oteryn_dev_client::{DevClientSession, EntityKind, SessionEvent};
use oteryn_protocol_oteryn::CommandStatus;
use oteryn_protocol_oteryn::attack::AttackIntentDisposition;
use oteryn_protocol_oteryn::condition_status::{
    CAPABILITY_CONDITION_STATUS_V1, COMMAND_TYPE_CONDITION_STATUS_QUERY, ConditionStatus,
};
use oteryn_protocol_oteryn::item_view::{EquipmentSlot, ItemEntry};

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
    use oteryn_protocol_oteryn::logout::{LogoutResult, MAX_LOGOUT_RETRY_AFTER_MS};

    let deadline =
        tokio::time::Instant::now() + Duration::from_millis(u64::from(MAX_LOGOUT_RETRY_AFTER_MS));
    loop {
        let combat = client
            .session_mut()
            .combat_state()
            .ok_or("logout requires combat state")?;
        if let Some(target) = combat.target {
            return Err(
                format!("logout requires leaving the combat target first: {target:?}").into(),
            );
        }
        if !combat.in_fight {
            break;
        }
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            return Err("combat lock remained active through its logout bound".into());
        }
        verdict::Harness::global().bounded_wait(deadline, "logout-combat-lock");
        client
            .service_liveness(remaining.min(Duration::from_millis(100)))
            .await?;
        client.take_events();
    }
    let reply = client.session_mut().logout().await?;
    if reply.result != Some(LogoutResult::Accepted) {
        return Err(format!(
            "logout refused status={:?} result={:?} command={} server_sequence={}",
            reply.status, reply.result, reply.command_id, reply.result_server_sequence
        )
        .into());
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

fn necklace(client: &DevClientSession) -> Option<ItemEntry> {
    client
        .inventory()?
        .equipment
        .iter()
        .find(|entry| entry.slot == EquipmentSlot::Necklace)
        .map(|entry| entry.item)
}

fn amulet_at(client: &DevClientSession, item: ItemEntry, equipped: bool) -> TestResult<()> {
    let inventory = client.inventory().ok_or("missing inventory")?;
    // Wire handles are session-local. The remaining fields describe the persisted item.
    let matches = |observed: ItemEntry| {
        ItemEntry {
            handle: item.handle,
            ..observed
        } == item
    };
    let carried = quantity(client, item.item_definition_ref)?;
    let correct = if equipped {
        carried == 0 && necklace(client).is_some_and(matches)
    } else {
        carried == 1
            && necklace(client).is_none()
            && worn(client, item.item_definition_ref).is_none()
            && inventory.entries.iter().copied().any(matches)
    };
    if !correct {
        return Err(format!(
            "amulet custody mismatch equipped={equipped} carried={carried} necklace={:?}",
            necklace(client)
        )
        .into());
    }
    Ok(())
}

async fn settle(client: &mut DevClientSession, item: ItemEntry, equipped: bool) -> TestResult<()> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(6);
    loop {
        let result = amulet_at(client, item, equipped);
        if result.is_ok() || tokio::time::Instant::now() >= deadline {
            return result;
        }
        client.service_liveness(Duration::from_millis(20)).await?;
        client.take_events();
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
    item: ItemEntry,
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
    if start == COMBAT_APPROACH || path.is_empty() {
        return Err("combat route starts at its destination".into());
    }
    for direction in path {
        let before = tile(client.world_spatial().actor_position)?;
        let expected = destination(world, before, direction)
            .filter(|at| *at != before)
            .ok_or("combat step lacks a distinct expected landing")?;
        let mut step = client.step(direction).await?;
        if step.disposition == StepDisposition::Blocked
            && tile(client.world_spatial().actor_position)? == before
        {
            let occupant = client.world_entities().into_iter()
                .flat_map(|view| view.iter())
                .find(|entry| entry.kind == EntityKind::Creature
                    && tile(entry.position).is_ok_and(|position| position == expected))
                .map(|entry| entry.entity)
                .ok_or_else(|| format!("BLOCKER-SERVER blocked landing without visible creature occupant: {expected:?}; frame={step:?}"))?;
            evidence(&format!(
                "I17D_CHECK blocked-landing occupant={occupant:?} tile={expected:?} frame={step:?}"
            ));
            let deadline = tokio::time::Instant::now() + Duration::from_secs(2);
            let position = loop {
                let actor = client
                    .world_entities()
                    .and_then(|view| view.get(&occupant))
                    .ok_or("blocking creature left view before its position could be verified")?;
                let position = tile(actor.position)?;
                if position != expected {
                    break position;
                }
                let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
                if remaining.is_zero() {
                    return Err(format!("creature {occupant:?} still blocks {expected:?}").into());
                }
                client
                    .service_liveness(remaining.min(Duration::from_millis(50)))
                    .await?;
                client.take_events();
            };
            evidence(&format!(
                "I17D_CHECK blocked-landing cleared occupant={occupant:?} position={position:?}"
            ));
            step = client.step(direction).await?;
        }
        let actual = tile(client.world_spatial().actor_position)?;
        if step.disposition != StepDisposition::Moved
            || step.world_spatial_delta.is_none()
            || actual != expected
        {
            return Err(format!("combat step {direction:?} from={before:?} expected={expected:?} actual={actual:?} result={step:?}").into());
        }
        verdict::Harness::global().progress("walk");
        client.take_events();
    }
    if tile(client.world_spatial().actor_position)? != COMBAT_APPROACH {
        return Err("combat route missed its fixed approach".into());
    }
    let target = client
        .world_entities()
        .into_iter()
        .flat_map(|entities| entities.iter())
        .filter(|entity| {
            entity.kind == EntityKind::Creature
                && entity
                    .display_name
                    .is_some_and(|name| name.as_str() == Ok("Shrieking Cry-Stal"))
                && tile(entity.position).is_ok_and(|at| at.floor == COMBAT_APPROACH.floor)
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
    if client
        .session_mut()
        .combat_state()
        .ok_or("missing initial combat state")?
        .target
        .is_some()
    {
        return Err("attack pre-state already has a combat target".into());
    }
    let attack = client.session_mut().attack_target(Some(&target)).await?;
    if attack.status != CommandStatus::Accepted || attack.disposition != AttackIntentDisposition::Ok
    {
        return Err(format!("Shrieking Cry-Stal attack refused: {attack:?}").into());
    }
    let target_deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    'observe_target: loop {
        for event in client.take_events() {
            if let SessionEvent::ActorCombatState(delta) = event
                && delta.server_sequence > attack.result_server_sequence
            {
                if delta.value.target != Some(target) {
                    return Err(format!(
                        "attack published the wrong combat target: {:?}",
                        delta.value.target
                    )
                    .into());
                }
                break 'observe_target;
            }
        }
        if tokio::time::Instant::now() >= target_deadline {
            return Err("attack did not publish the live caster as its combat target".into());
        }
        client.service_liveness(Duration::from_millis(20)).await?;
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(120);
    verdict::Harness::global().bounded_wait(deadline, "authored-fear-cast");
    while !feared(client).await? {
        if tokio::time::Instant::now() >= deadline {
            return Err("Shrieking Cry-Stal combat did not produce wire Feared icon 12".into());
        }
        client.service_liveness(Duration::from_millis(100)).await?;
    }
    amulet_at(client, item, false)?;
    let before = client
        .inventory()
        .ok_or("missing feared pre-inventory")?
        .clone();
    expect(
        client,
        item.item_definition_ref,
        "feared-equip",
        oteryn_dev_client::ItemMoveOutcome::Rejected,
    )
    .await?;
    if !feared(client).await? || client.inventory() != Some(&before) {
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
    let item = client
        .inventory()
        .ok_or("missing fixture inventory")?
        .entries
        .iter()
        .find(|entry| entry.item_definition_ref == amulet)
        .copied()
        .ok_or("missing carried fixture amulet")?;
    amulet_at(&client, item, false)?;
    evidence("I17D_CHECK prerequisite PASS fixture=one-carried-amulet-after-relog");

    expect(&mut client, amulet, "equip-amulet", Out::Moved).await?;
    settle(&mut client, item, true).await?;
    let equipped = client
        .inventory()
        .ok_or("missing cooldown pre-inventory")?
        .clone();
    let equip_state_sequence = client.last_server_sequence();
    let refused = hotkey(&mut client, amulet).await?;
    let cooldown_failure = match refused {
        Out::Rejected => {
            if client.inventory() != Some(&equipped) {
                return Err("immediate cooldown refusal changed inventory".into());
            }
            // Crystal 201189f8 configmanager.cpp:377 defaults the necklace clock to 200 ms.
            wait_for(&mut client, Duration::from_millis(200)).await?;
            if client.inventory() != Some(&equipped) {
                return Err("cooldown refusal changed inventory before recovery".into());
            }
            expect(&mut client, amulet, "after-delay", Out::Moved).await?;
            None
        }
        Out::Moved => {
            let failure = format!(
                "step=immediate-repeat observed=Moved;equip_state_sequence={equip_state_sequence};repeat_sequence={};needed=Rejected-and-still-worn",
                client.last_server_sequence()
            );
            evidence(&format!("I17D_CHECK jewelry-delay FAIL {failure}"));
            Some(failure)
        }
        other => {
            return Err(format!("step=immediate-repeat observed={other:?};needed=Rejected").into());
        }
    };
    settle(&mut client, item, false).await?;
    wait_for(&mut client, Duration::from_millis(200)).await?;
    expect(&mut client, amulet, "equip-for-durable-reload", Out::Moved).await?;
    settle(&mut client, item, true).await?;
    let mut client = relog(client, clients).await?;
    amulet_at(&client, item, true)?;
    expect(&mut client, amulet, "unequip-after-reload", Out::Moved).await?;
    settle(&mut client, item, false).await?;
    if cooldown_failure.is_none() {
        evidence("I17D_CHECK jewelry-delay PASS immediate=Rejected recovery=Moved relog=necklace");
    }
    let combat = async {
        character_prep::declare_seeded(
            "I17d.full",
            "level 100 knight",
            "survive authored fear caster attacks",
        );
        request_fixture_control(
            uuid_text(client.join_snapshot().game_session_id.as_bytes()),
            "--profile",
            "knight".into(),
        )
        .await?;
        let mut client = relog(client, clients).await?;
        let vitals = client
            .actor_vitals()
            .ok_or("seeded combat vitals missing")?;
        let (health, mana, _) = oteryn_gameplay_characters::vitals::maxima(
            "knight",
            100,
            oteryn_protocol_oteryn::actor_spell::MAX_VITAL_POOL,
            oteryn_protocol_oteryn::actor_spell::MAX_SOUL,
        )
        .ok_or("missing level 100 knight stat formula")?;
        if (vitals.max_health, vitals.max_mana) != (health, mana) {
            return Err(format!("seeded combat vitals mismatch: {vitals:?}").into());
        }
        fight_fear_caster(&mut client, clients, item).await?;
        drop(client);
        wait_character_released(clients, 0).await?;
        Ok::<(), Box<dyn std::error::Error>>(())
    }
    .await;
    if let Some(failure) = cooldown_failure {
        return Err(match combat {
            Ok(()) => failure,
            Err(error) => format!("{failure};combat={error}"),
        }
        .into());
    }
    combat?;
    Ok("jewelry-delay-and-live-feared-gate-proven".to_owned())
}
