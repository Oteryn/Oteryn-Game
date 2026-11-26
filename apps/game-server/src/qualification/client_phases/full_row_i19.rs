//! Crystal 201189f8 enchanting.lua: raw ruby -> fire shrine -> flame weapon.
use super::*;
use crate::map::view::MapFacts;
use oteryn_dev_client::{DevClientSession, SessionEvent};
use oteryn_protocol_oteryn::actor_spell::ActorVitals;
use oteryn_protocol_oteryn::item_view::ItemHandle;
use oteryn_protocol_oteryn::timed_item as timed;
use oteryn_protocol_oteryn::world_map::MapDefinition;
use oteryn_protocol_oteryn::world_object::{
    MapUseTarget, UseDisposition, UseIntent, UseTarget, WorldObjectTarget, decode_use_result,
    encode_item_use_intent,
};
use std::num::NonZeroU32;

const RUBY: u32 = 3030;
const WEAPON: u32 = 3271;
const ALTAR: u32 = 146;
const ENCHANTED_RUBY: u32 = 676;
const ENCHANTED_WEAPON: u32 = 660;
const SHRINE: TilePos = TilePos {
    x: 32342,
    y: 32269,
    floor: -11,
};
const APPROACH: TilePos = TilePos {
    x: 32343,
    y: 32268,
    floor: -11,
};

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result =
        verdict::indexed_timeout(Duration::from_secs(420), verdict::observe(journey(clients)))
            .await;
    let (status, detail) = match result {
        Ok(Ok(detail)) => ("PASS", detail),
        Ok(Err(error)) => ("FAIL", error.to_string()),
        Err(_) => ("FAIL", "step=journey;observed=timeout".to_owned()),
    };
    let detail = detail.split_whitespace().collect::<Vec<_>>().join("-");
    evidence(&format!(
        "e2e_all scenario=I19.full status={status} seconds={:.3} evidence={detail}",
        started.elapsed().as_secs_f64()
    ));
}

fn inventory(client: &DevClientSession, refs: [NonZeroU32; 4]) -> TestResult<[u64; 4]> {
    let inventory = client
        .inventory()
        .ok_or("enchanting inventory snapshot absent")?;
    Ok(refs.map(|reference| {
        inventory
            .entries
            .iter()
            .filter(|item| item.item_definition_ref == reference)
            .map(|item| u64::from(item.count.get()))
            .sum()
    }))
}

fn handle(client: &DevClientSession, reference: NonZeroU32) -> TestResult<ItemHandle> {
    client
        .inventory()
        .and_then(|inventory| {
            inventory
                .entries
                .iter()
                .find(|item| item.item_definition_ref == reference)
        })
        .map(|item| item.handle)
        .ok_or_else(|| format!("missing item handle for {reference}").into())
}

fn vitals(client: &DevClientSession) -> TestResult<ActorVitals> {
    client
        .actor_vitals()
        .copied()
        .ok_or_else(|| "enchanting vitals snapshot absent".into())
}

async fn relog(
    mut client: DevClientSession,
    clients: &SeamClients<'_>,
    tag: u8,
) -> TestResult<DevClientSession> {
    let session = client.join_snapshot().game_session_id;
    logout(&mut client).await?;
    drop(client);
    let fresh = connect_character(clients, tag, 0).await?;
    if fresh.join_snapshot().game_session_id == session {
        return Err("enchanting relog reused the previous session".into());
    }
    Ok(fresh)
}

async fn logout(client: &mut DevClientSession) -> TestResult {
    let reply = client.session_mut().logout().await?;
    if reply.status != CommandStatus::Accepted
        || reply.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err(format!("enchanting logout refused: {reply:?}").into());
    }
    Ok(())
}

fn shrine_target(world: &BundleWorld, reference: NonZeroU32) -> TestResult<MapUseTarget> {
    let tile = world
        .base()
        .tile(SHRINE.x, SHRINE.y, SHRINE.floor)
        .ok_or("authored fire shrine tile absent")?;
    let ordinal = 1;
    let id = tile
        .ids()
        .iter()
        .zip(tile.depths())
        .filter(|(_, depth)| **depth == 0)
        .nth(usize::from(ordinal))
        .map(|(id, _)| *id)
        .ok_or("authored fire altar layer absent")?;
    if world
        .facts()
        .base_entry(SHRINE, ordinal, id)
        .is_none_or(|entry| entry.definition != MapDefinition::Item(reference))
    {
        return Err("authored fire altar 146 absent from its shrine layer".into());
    }
    let key = crate::map::view::placement_key(SHRINE, ordinal)
        .ok_or("fire altar placement key absent")?;
    let mut placement = world.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    Ok(MapUseTarget::Object(WorldObjectTarget {
        placement,
        expected_revision: 0,
    }))
}

async fn walk_to_shrine(client: &mut DevClientSession, world: &BundleWorld) -> TestResult {
    let from = tile(client.world_spatial().actor_position)?;
    let world_for_route = world.clone();
    let path = route_work(client, move || {
        find_route(
            from,
            |at| at == APPROACH,
            |at, direction| {
                let next = destination(&world_for_route, at, direction)?;
                (next.x.abs_diff(TOWN.x) <= 100
                    && next.y.abs_diff(TOWN.y) <= 100
                    && (-11..=-7).contains(&next.floor))
                .then_some(next)
            },
        )
        .map_err(|visited| format!("no walk route to fire shrine from {from:?}; visited={visited}"))
    })
    .await?;
    for direction in path {
        let from = tile(client.world_spatial().actor_position)?;
        let expected =
            destination(world, from, direction).ok_or("fire shrine route lost its landing")?;
        let reply = client.step(direction).await?;
        let actual = tile(client.world_spatial().actor_position)?;
        if reply.status != CommandStatus::Accepted
            || reply.disposition != StepDisposition::Moved
            || reply.world_spatial_delta.is_none()
            || actual != expected
        {
            return Err(format!("fire shrine step {direction:?} from={from:?} expected={expected:?} actual={actual:?} reply={reply:?}").into());
        }
        verdict::Harness::global().progress("fire-shrine-step");
        client.take_events();
        // Pace the next new command using the receipt for the successful step.
        tokio::time::sleep(Duration::from_millis(u64::from(reply.step_wait_ms))).await;
    }
    if tile(client.world_spatial().actor_position)? != APPROACH {
        return Err("fire shrine approach was not reached".into());
    }
    Ok(())
}

async fn enchant(
    client: &mut DevClientSession,
    source: ItemHandle,
    target: MapUseTarget,
    refs: [NonZeroU32; 4],
    expected_items: [u64; 4],
    mana: u32,
    soul: u32,
) -> TestResult<u64> {
    let before = vitals(client)?;
    let expected_pools = (before.mana.checked_sub(mana), before.soul.checked_sub(soul));
    client.take_events();
    let payload = encode_item_use_intent(&UseIntent {
        target: UseTarget::Item(source),
        use_with: None,
        use_with_map_item: Some(target),
        use_with_position: None,
    })
    .map_err(|error| format!("enchanting intent encode: {error:?}"))?;
    let reply = client.session_mut().world_context_use(&payload).await?;
    let disposition = decode_use_result(&reply.payload)
        .map_err(|error| format!("enchanting result decode: {error:?}"))?;
    if reply.status != CommandStatus::Accepted || disposition != UseDisposition::Committed {
        return Err(format!(
            "enchanting intent status={:?} disposition={disposition:?}",
            reply.status
        )
        .into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    let mut inventory_delta = false;
    let mut vitals_delta = mana == 0 && soul == 0;
    loop {
        for event in client.take_events() {
            match event {
                SessionEvent::Inventory(delta) if delta.server_sequence > reply.server_sequence => {
                    inventory_delta = true;
                }
                SessionEvent::ActorVitals(delta)
                    if delta.server_sequence > reply.server_sequence =>
                {
                    vitals_delta |=
                        (Some(delta.value.mana), Some(delta.value.soul)) == expected_pools;
                }
                _ => {}
            }
        }
        let current_items = inventory(client, refs)?;
        let current = vitals(client)?;
        if inventory_delta
            && vitals_delta
            && current_items == expected_items
            && (Some(current.mana), Some(current.soul)) == expected_pools
        {
            return Ok(reply.command_id);
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("enchanting deltas command={} inventory={current_items:?} expected={expected_items:?} pools={:?} expected_pools={expected_pools:?} inventory_delta={inventory_delta} vitals_delta={vitals_delta}", reply.command_id, (current.mana, current.soul)).into());
        }
        client.service_liveness(Duration::from_millis(20)).await?;
    }
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let world = full_e2ef7::qualification_world(clients)?;
    let index = provisioning::item_index()?;
    let reference = |id| provisioning::item_reference(&index, id);
    let refs = [
        reference(RUBY)?,
        reference(WEAPON)?,
        reference(ENCHANTED_RUBY)?,
        reference(ENCHANTED_WEAPON)?,
    ];
    let shrine = shrine_target(world, reference(ALTAR)?)?;
    let identity = character_prep::dedicated_named_character(
        clients,
        "NODE_BOOT_I19_ACCOUNT",
        Some("Fire Shrine Tester"),
    )
    .await?;
    let scoped = identity.clients(clients);
    let mut client = connect_character(&scoped, 0xdf, 0).await?;
    character_prep::declare_seeded(
        "I19.full",
        "sorcerer-level-32,ruby-3030,weapon-3271",
        "travel and both enchanting commands are under test",
    );
    request_fixture_control(
        uuid_text(client.join_snapshot().game_session_id.as_bytes()),
        "--profile",
        "i19-sorcerer".into(),
    )
    .await?;
    character_prep::grant_item(&client, RUBY, 1).await?;
    character_prep::grant_item(&client, WEAPON, 1).await?;
    client = relog(client, &scoped, 0xdf).await?;

    let initial = vitals(&client)?;
    if (initial.max_health, initial.max_mana, initial.soul) != (305, 810, 100)
        || initial.mana < 300
        || inventory(&client, refs)? != [1, 1, 0, 0]
    {
        return Err(format!(
            "enchanting fixture state vitals={initial:?} items={:?}",
            inventory(&client, refs)?
        )
        .into());
    }
    let before_refusal = client.inventory().cloned();
    let refused = client.use_item(handle(&client, refs[0])?).await?;
    if refused.status != CommandStatus::Accepted
        || refused.disposition != UseDisposition::NothingToUse
        || client.inventory() != before_refusal.as_ref()
        || vitals(&client)? != initial
    {
        return Err(format!(
            "ruby without altar changed state or returned the wrong refusal: {refused:?}"
        )
        .into());
    }
    evidence("I19_CHECK no-altar PASS ruby-retained=1");

    walk_to_shrine(&mut client, world).await?;
    if inventory(&client, refs)? != [1, 1, 0, 0] {
        return Err("enchanting inputs changed before reaching the shrine".into());
    }
    let ruby = handle(&client, refs[0])?;
    let shrine_command = enchant(&mut client, ruby, shrine, refs, [0, 1, 1, 0], 300, 2).await?;
    evidence(&format!(
        "I19_CHECK fire-shrine PASS command={shrine_command} mana-debit=300 soul-debit=2"
    ));
    let ruby = handle(&client, refs[2])?;
    let weapon = handle(&client, refs[1])?;
    let weapon_command = enchant(
        &mut client,
        ruby,
        MapUseTarget::Item(weapon),
        refs,
        [0, 0, 0, 1],
        0,
        0,
    )
    .await?;
    evidence(&format!(
        "I19_CHECK fire-weapon PASS command={weapon_command} output=660"
    ));
    let paid = vitals(&client)?;
    let mut fresh = relog(client, &scoped, 0xdf).await?;
    let saved = vitals(&fresh)?;
    if inventory(&fresh, refs)? != [0, 0, 0, 1]
        || (saved.mana, saved.soul) != (paid.mana, paid.soul)
    {
        return Err(format!(
            "enchanting relog state vitals={saved:?} expected_pools={:?} items={:?}",
            (paid.mana, paid.soul),
            inventory(&fresh, refs)?
        )
        .into());
    }
    let payload = timed::encode_timed_item_intent(timed::TimedItemIntent {
        action: timed::TimedAction::Inspect,
        target: handle(&fresh, refs[3])?,
        expected_revision: 0,
        npc: None,
    })
    .map_err(|error| format!("enchanted weapon inspect encode: {error:?}"))?;
    let reply = fresh
        .session_mut()
        .gated_command(
            timed::CAPABILITY_TIMED_ITEM_V1,
            timed::COMMAND_TYPE_TIMED_ITEM_INTENT,
            &payload,
        )
        .await?;
    let inspected = timed::decode_timed_item_result(&reply.payload)
        .map_err(|error| format!("enchanted weapon inspect decode: {error:?}"))?;
    if reply.status != CommandStatus::Accepted
        || inspected.disposition != timed::TimedDisposition::Ok
        || inspected.charges != Some(1000)
        || inspected.retired
    {
        return Err(format!(
            "enchanted weapon after relog status={:?} inspect={inspected:?}",
            reply.status
        )
        .into());
    }
    logout(&mut fresh).await?;
    Ok(format!(
        "ruby=3030->676 shrine=146 weapon=3271->660 mana-debit=300 soul-debit=2 shrine-command={shrine_command} weapon-command={weapon_command} durable-relog=verified weapon-charges=1000"
    ))
}
