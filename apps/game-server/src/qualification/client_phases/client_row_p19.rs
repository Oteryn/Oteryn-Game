//! Verify live Forge marker lifecycle, combat rewards and fresh-admission custody.
use super::*;
use oteryn_dev_client::native_session::AppearanceUpdate;
use oteryn_dev_client::{DevClientSession, EntityDetail, EntityKind, EntityRef, SessionEvent};
use oteryn_protocol_oteryn::appearance::CAPABILITY_FORGE_CREATURE_V1;
use oteryn_protocol_oteryn::{container_tree as containers, item_forge, item_view as items};
use std::num::NonZeroU32;

pub(super) async fn run_packet(clients: &SeamClients<'_>, world: &BundleWorld) {
    let result = verdict::indexed_timeout(
        Duration::from_secs(180),
        verdict::observe(prove(clients, world)),
    )
    .await;
    match result {
        Ok(Ok(())) => evidence(
            "e2e_all scenario=P19.client status=PASS evidence=first-marker-kill-clear-and-exact-durable-dust-and-slivers",
        ),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=P19.client status=FAIL evidence={error}"
        )),
        Err(_) => evidence("e2e_all scenario=P19.client status=FAIL evidence=timeout"),
    }
}

async fn prove(clients: &SeamClients<'_>, world: &BundleWorld) -> TestResult {
    let capture = required("E2E_ALL_CAPTURE_DIR")?;
    let dedicated =
        character_prep::dedicated_character(clients, "NODE_BOOT_C11_KNIGHT_ACCOUNT").await?;
    let (accounts, characters) = dedicated.replace_slot(clients, 1)?;
    let clients = &SeamClients {
        accounts: &accounts,
        characters: &characters,
        ..*clients
    };
    character_prep::declare_seeded("P19.client", "knight,premium", "Forge combat and rewards");
    let mut client = character_prep::Seeded {
        profile: "knight",
        reason: "Forge combat qualification",
    }
    .connect(clients, 0x19, 1)
    .await?;
    let index = provisioning::item_index()?;
    let axe = provisioning::item_reference(&index, 3276)?;
    equip_fixture_axe(&mut client, axe).await?;
    let sliver = provisioning::item_reference(&index, 37109)?;
    if !client
        .selected_capabilities()
        .contains(&CAPABILITY_FORGE_CREATURE_V1)
    {
        return Err("P19 Forge appearance capability absent".into());
    }
    use crate::qualification_wild_spawn::adjacent_cells;
    let cells = |at: TilePos| {
        adjacent_cells(crate::foundation::MovementLocalPosition {
            x: i32::from(at.x),
            y: i32::from(at.y),
            floor: -i16::from(at.floor),
        })
        .into_iter()
        .filter(|candidate| {
            let (Ok(x), Ok(y)) = (u16::try_from(candidate.x), u16::try_from(candidate.y)) else {
                return false;
            };
            let target = TilePos {
                x,
                y,
                floor: at.floor,
            };
            world
                .base()
                .tile(x, y, at.floor)
                .is_some_and(|tile| tile.walkable() && !tile.protection_zone())
                && world.enterable(target)
                && !world.floors().contains(target)
                && world.floors().teleport(target).is_none()
        })
        .take(2)
        .collect::<Vec<_>>()
    };
    let approach = |at: TilePos| {
        cells(at).len() == 2
            && world
                .base()
                .tile(at.x, at.y, at.floor)
                .is_some_and(|tile| !tile.protection_zone())
    };
    for (classification, stacks, folder, slot) in
        [("influenced 3", 3, "", 0), ("fiendish", 15, "fiendish", 1)]
    {
        let path = route(
            world,
            tile(client.world_spatial().actor_position)?,
            approach,
        )
        .ok_or("P19 FIXTURE_PLACEMENT no approach with two open spawn tiles")?;
        move_first_attempt(&mut client, world, path).await?;
        let positions = cells(tile(client.world_spatial().actor_position)?);
        let position = *positions
            .get(slot)
            .ok_or("P19 FIXTURE_PLACEMENT spawn tile absent")?;
        if slot == 0 {
            let refused = request_fixture_control(
                uuid_text(client.join_snapshot().game_session_id.as_bytes()),
                "--spawn-forge",
                format!(
                    "oteryn:creature.druid_familiar {} {} {} fiendish",
                    position.x, position.y, position.floor
                ),
            )
            .await;
            if refused.as_ref().err().map(String::as_str) != Some("ineligible_forge_creature") {
                return Err(
                    format!("P19 expected ineligible_forge_creature, got {refused:?}").into(),
                );
            }
        }
        let session = uuid_text(client.join_snapshot().game_session_id.as_bytes());
        character_prep::grant_premium(&client).await?;
        evidence("P19 FIXTURE premium=granted");
        let before = rewards(&mut client, sliver).await?;
        client.take_events();
        evidence(&format!(
            "P19 FIXTURE classification={classification} placement={position:?}"
        ));
        let reply = request_fixture_control(
            session.clone(),
            "--spawn-forge",
            format!(
                "oteryn:creature.rat {} {} {} {classification}",
                position.x, position.y, position.floor
            ),
        )
        .await?;
        let exact = parse_spawned_entity(&reply)?;
        let first: u64 = tokio::time::timeout(Duration::from_secs(15), async {
            let mut first_binding = None;
            loop {
                for event in client.wait_for_events(Duration::from_secs(15)).await? {
                    match event {
                        SessionEvent::Appearance(delta) => {
                            if let AppearanceUpdate::Mappings(mappings) = delta.value {
                                if mappings.forgets.contains(&exact) {
                                    first_binding = None;
                                }
                                if let Some(binding) = mappings.upserts.iter().find(|entry| entry.entity == exact) {
                                    if binding.forge_stacks != Some(stacks) {
                                        return Err(format!("P19 {classification} wire binding was {:?}", binding.forge_stacks).into());
                                    }
                                    first_binding.get_or_insert(delta.server_sequence);
                                }
                            }
                        }
                        SessionEvent::WorldSpatialEntities(delta)
                            if delta.value.enter.iter().any(|entry| entry.entity == exact) =>
                        {
                            return TestResult::Ok(first_binding.ok_or_else(|| format!(
                                "P19 {classification} entity {exact:?} entered at sequence {} before its marker",
                                delta.server_sequence,
                            ))?);
                        }
                        _ => {}
                    }
                }
            }
        })
        .await??;
        let files = tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                if let Some(files) = client.session_mut().p19_capture_files(exact) {
                    return TestResult::Ok(files);
                }
                client.service_liveness(Duration::from_millis(50)).await?;
            }
        })
        .await
        .map_err(|_| format!("P19 {classification} exact creature replay capture missing"))??;
        let directory = Path::new(&capture).join(folder);
        std::fs::create_dir_all(&directory)?;
        for (name, bytes) in files {
            std::fs::write(directory.join(name), bytes)?;
        }
        evidence(&format!(
            "P19_FORGE entity={exact:?} forge_stacks={stacks} sequence={}",
            first
        ));
        let corpse = kill_and_observe_clear(&mut client, exact).await?;
        let looted = loot_slivers(&mut client, corpse, sliver, stacks == 15).await?;
        let after = rewards(&mut client, sliver).await?;
        // Global dust table: https://tibia.fandom.com/wiki/Fiendish_and_Influenced_Creatures#Dust_and_Slivers
        let minimum = (before.dust + u64::from(stacks)).min(before.limit);
        let maximum = (before.dust + 3 * u64::from(stacks)).min(before.limit);
        if before.dust == before.limit
            || !(minimum..=maximum).contains(&after.dust)
            || after.limit != before.limit
            || before.slivers.checked_add(looted) != Some(after.slivers)
        {
            return Err(format!("P19 {classification} reward mismatch: {before:?} -> {after:?}, slivers looted={looted}").into());
        }
        evidence(&format!(
            "P19 rewards {classification} before={before:?} after={after:?} looted={looted}"
        ));
        let retreat = route(world, tile(client.world_spatial().actor_position)?, |at| {
            is_logout_rest_tile(world, at)
        })
        .ok_or("P19 FIXTURE_PLACEMENT no route to protection zone for fresh admission")?;
        move_first_attempt(&mut client, world, retreat).await?;
        let at = client.world_spatial().actor_position;
        let previous = client.join_snapshot().game_session_id;
        require_logout(&mut client).await?;
        drop(client);
        client = connect_character(clients, 0x19, 1).await?;
        if client.join_snapshot().game_session_id == previous
            || client.world_spatial().actor_position != at
        {
            return Err(
                "P19 fresh admission did not restore the saved position in a new session".into(),
            );
        }
        let restored = rewards(&mut client, sliver).await?;
        if restored != after {
            return Err(format!(
                "P19 fresh rewards mismatch: expected={after:?} observed={restored:?}"
            )
            .into());
        }
        evidence(&format!(
            "P19 rewards {classification} fresh_admission={restored:?}"
        ));
    }
    require_logout(&mut client).await
}

async fn equip_fixture_axe(client: &mut DevClientSession, axe: NonZeroU32) -> TestResult {
    let inventory = client.inventory().ok_or("P19 fixture inventory absent")?;
    if inventory
        .equipment
        .iter()
        .any(|entry| entry.item.item_definition_ref == axe)
    {
        return Ok(());
    }
    evidence("P19 FIXTURE weapon=axe");
    let reply = client.session_mut().equip_item_definition(axe).await?;
    require_mechanic_acceptance(reply.status, "P19 equip fixture axe")?;
    if reply.outcome != items::ItemMoveOutcome::Moved {
        return Err(format!("P19 fixture axe equip refused: {reply:?}").into());
    }
    tokio::time::timeout(Duration::from_secs(5), async {
        while !client.inventory().is_some_and(|inventory| {
            inventory
                .equipment
                .iter()
                .any(|entry| entry.item.item_definition_ref == axe)
        }) {
            client.service_liveness(Duration::from_millis(100)).await?;
        }
        TestResult::Ok(())
    })
    .await?
}

async fn require_logout(client: &mut DevClientSession) -> TestResult {
    let reply = client.session_mut().logout().await?;
    if reply.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("P19 first logout refused: {reply:?}").into());
    }
    Ok(())
}

async fn move_first_attempt(
    client: &mut DevClientSession,
    world: &BundleWorld,
    path: Vec<StepDirection>,
) -> TestResult {
    for direction in path {
        let origin = tile(client.world_spatial().actor_position)?;
        let expected = destination(world, origin, direction).ok_or("P19 route has no landing")?;
        let movement = client.step(direction).await?;
        let actual = tile(client.world_spatial().actor_position)?;
        if !confirmed_step(
            movement.disposition,
            movement.world_spatial_delta.is_some(),
            actual,
            expected,
        ) {
            return Err(format!(
                "P19 FIXTURE_PLACEMENT first movement {direction:?} from {origin:?}: disposition={:?} expected={expected:?} actual={actual:?}",
                movement.disposition,
            ).into());
        }
        if movement.step_wait_ms != 0 {
            let wait = Duration::from_millis(u64::from(movement.step_wait_ms));
            let until = tokio::time::Instant::now() + wait;
            while let Some(remaining) = until.checked_duration_since(tokio::time::Instant::now()) {
                client.service_liveness(remaining).await?;
            }
        }
    }
    Ok(())
}

#[derive(Debug, PartialEq, Eq, serde::Deserialize)]
struct Rewards {
    #[serde(rename = "current_dust")]
    dust: u64,
    #[serde(rename = "dust_limit")]
    limit: u64,
    #[serde(skip)]
    slivers: u64,
}

async fn rewards(client: &mut DevClientSession, sliver: NonZeroU32) -> TestResult<Rewards> {
    let reply = client
        .session_mut()
        .gated_command(
            item_forge::CAPABILITY_ITEM_FORGE_V1,
            item_forge::COMMAND_TYPE_FORGE_DUST_QUERY,
            &[],
        )
        .await?;
    require_mechanic_acceptance(reply.status, "P19 Forge dust balance")?;
    let mut result: Rewards = serde_json::from_slice(&reply.payload)?;
    result.slivers = client
        .inventory()
        .ok_or("P19 inventory absent")?
        .entries
        .iter()
        .filter(|entry| entry.item_definition_ref == sliver)
        .map(|entry| u64::from(entry.count.get()))
        .sum();
    Ok(result)
}

async fn kill_and_observe_clear(
    client: &mut DevClientSession,
    target: EntityRef,
) -> TestResult<items::ItemHandle> {
    use oteryn_protocol_oteryn::attack::AttackIntentDisposition;
    let entities = client.world_entities().ok_or("P19 spatial view absent")?;
    let mut position = entities
        .get(&target)
        .ok_or("P19 target absent before attack")?
        .position;
    let existing: std::collections::BTreeSet<_> =
        entities.iter().map(|entity| entity.entity).collect();
    client.take_events();
    let attack = client.session_mut().attack_target(Some(&target)).await?;
    require_mechanic_acceptance(attack.status, "P19 attack")?;
    if attack.disposition != AttackIntentDisposition::Ok {
        return Err(format!("P19 first attack refused: {attack:?}").into());
    }
    tokio::time::timeout(Duration::from_secs(55), async {
        let mut left = false;
        let mut cleared = None;
        let mut corpse = None;
        loop {
            for event in client.wait_for_events(Duration::from_secs(55)).await? {
                match event {
                    SessionEvent::Appearance(delta) => {
                        if let AppearanceUpdate::Mappings(mappings) = delta.value
                            && mappings.forgets.contains(&target)
                        {
                            if !left {
                                return Err(format!(
                                    "P19 marker for {target:?} cleared before its spatial leave"
                                )
                                .into());
                            }
                            cleared = Some(delta.server_sequence);
                            evidence(&format!(
                                "P19 marker-cleared entity={target:?} sequence={}",
                                delta.server_sequence
                            ));
                        }
                    }
                    SessionEvent::WorldSpatialEntities(delta) => {
                        if let Some(entity) = delta
                            .value
                            .update
                            .iter()
                            .find(|entry| entry.entity == target)
                        {
                            position = entity.position;
                        }
                        if delta.value.leave.contains(&target) {
                            left = true;
                            evidence(&format!(
                                "P19 target-left entity={target:?} sequence={}",
                                delta.server_sequence
                            ));
                        }
                        if left {
                            for entity in delta.value.enter.iter().chain(&delta.value.update) {
                                if entity.kind == EntityKind::Corpse
                                    && entity.position == position
                                    && !existing.contains(&entity.entity)
                                    && let (EntityDetail::Object {
                                        item_handle: Some(handle),
                                        ..
                                    }
                                    | EntityDetail::ObjectWithAppearance {
                                        item_handle: Some(handle),
                                        ..
                                    }) = entity.detail
                                {
                                    corpse = Some(handle);
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
            if let (Some(handle), Some(_)) = (corpse, cleared) {
                return TestResult::Ok(handle);
            }
        }
    })
    .await
    .map_err(|_| format!("P19 exact kill, corpse or marker clear missing for {target:?}"))?
}

async fn loot_slivers(
    client: &mut DevClientSession,
    corpse: items::ItemHandle,
    sliver: NonZeroU32,
    fiendish: bool,
) -> TestResult<u64> {
    let outcome = client
        .session_mut()
        .container_view(containers::ContainerViewIntent::Open {
            handle: corpse,
            replace_view: None,
        })
        .await?;
    if outcome != Some(containers::ContainerViewOutcome::Opened) {
        return Err(format!("P19 first corpse-open refused: {outcome:?}").into());
    }
    let (view_id, loot) = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if let Some(view) = client.session_mut().container_views().and_then(|views| {
                views
                    .views
                    .iter()
                    .find(|view| view.container_handle == corpse)
            }) {
                return TestResult::Ok((
                    view.view_id,
                    view.entries
                        .iter()
                        .copied()
                        .filter(|entry| entry.item_definition_ref == sliver)
                        .collect::<Vec<_>>(),
                ));
            }
            client.service_liveness(Duration::from_millis(100)).await?;
        }
    })
    .await??;
    let count: u64 = loot.iter().map(|entry| u64::from(entry.count.get())).sum();
    // Global grants slivers only to fiendish creatures, with quantity based on difficulty.
    // https://tibia.fandom.com/wiki/Fiendish_and_Influenced_Creatures#Dust_and_Slivers
    if (count > 0) != fiendish {
        return Err(
            format!("P19 unexpected corpse sliver count {count}, fiendish={fiendish}").into(),
        );
    }
    for entry in loot {
        let reply = client
            .move_item(&items::ItemMoveIntent {
                source: entry.handle,
                destination: items::ItemMoveDestination::MainBackpack,
            })
            .await?;
        require_mechanic_acceptance(reply.status, "P19 loot sliver")?;
        if reply.outcome != items::ItemMoveOutcome::Moved {
            return Err(format!("P19 first sliver move refused: {reply:?}").into());
        }
    }
    let outcome = client
        .session_mut()
        .container_view(containers::ContainerViewIntent::Close { view_id })
        .await?;
    if outcome != Some(containers::ContainerViewOutcome::Closed) {
        return Err(format!("P19 first corpse-close refused: {outcome:?}").into());
    }
    Ok(count)
}

fn confirmed_step(
    disposition: StepDisposition,
    has_delta: bool,
    actual: TilePos,
    expected: TilePos,
) -> bool {
    disposition == StepDisposition::Moved && has_delta && actual == expected
}

#[test]
fn first_step_refusals_and_unconfirmed_landings_fail() {
    let expected = TilePos {
        x: 1,
        y: 1,
        floor: 7,
    };
    let wrong = TilePos { x: 2, ..expected };
    for refused in [
        StepDisposition::Blocked,
        StepDisposition::Rejected,
        StepDisposition::TooEarly,
    ] {
        assert!(!confirmed_step(refused, true, expected, expected));
    }
    assert!(!confirmed_step(
        StepDisposition::Moved,
        false,
        expected,
        expected
    ));
    assert!(!confirmed_step(
        StepDisposition::Moved,
        true,
        wrong,
        expected
    ));
    assert!(confirmed_step(
        StepDisposition::Moved,
        true,
        expected,
        expected
    ));
}
