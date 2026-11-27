//! Verify live Forge marker lifecycle, combat rewards and fresh-admission custody.
use super::*;
use oteryn_dev_client::native_session::AppearanceUpdate;
use oteryn_dev_client::{DevClientSession, EntityDetail, EntityKind, EntityRef, SessionEvent};
use oteryn_protocol_oteryn::appearance::CAPABILITY_FORGE_CREATURE_V1;
use oteryn_protocol_oteryn::{
    attack::AttackIntentDisposition, forge_preview, item_view as items, quick_loot,
};
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
    let dedicated = character_prep::dedicated_character(clients, "NODE_BOOT_P19_ACCOUNT").await?;
    let (accounts, characters) = dedicated.replace_slot(clients, 1)?;
    let clients = &SeamClients {
        accounts: &accounts,
        characters: &characters,
        ..*clients
    };
    if !requires_platform_grant(&clients.accounts[1]) {
        return Err("P19 requires Platform-issued admission for its Premium fixture".into());
    }
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
    evidence("P19 FIXTURE premium=platform_snapshot");
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
        let before = rewards(&mut client).await?;
        if slot == 0
            && before
                != (Rewards {
                    dust: 0,
                    limit: 100,
                    slivers: 0,
                })
        {
            return Err(format!("P19 expected a fresh Forge balance, got {before:?}").into());
        }
        if sliver_count(&client, sliver)? != before.slivers {
            return Err("P19 pre-combat backpack and Forge sliver counts disagree".into());
        }
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
        let stopped = client.session_mut().attack_target(None).await?;
        require_mechanic_acceptance(stopped.status, "P19 stop attack")?;
        if stopped.disposition != AttackIntentDisposition::Ok {
            return Err(format!("P19 first attack stop refused: {stopped:?}").into());
        }
        let killed = rewards(&mut client).await?;
        let (expected_items, expected_slivers) =
            corpse_contents(&mut client, corpse, sliver).await?;
        quick_loot_corpse(&mut client, corpse, expected_items).await?;
        let after = rewards(&mut client).await?;
        let looted = after
            .slivers
            .checked_sub(killed.slivers)
            .ok_or("P19 backpack slivers decreased after loot")?;
        if !valid_reward_transition(&before, &killed, &after, stacks)
            || looted != expected_slivers
            || sliver_count(&client, sliver)? != after.slivers
        {
            return Err(format!("P19 {classification} reward mismatch: before={before:?} killed={killed:?} looted={after:?}").into());
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
        let restored = rewards(&mut client).await?;
        if restored != after || sliver_count(&client, sliver)? != restored.slivers {
            return Err(format!(
                "P19 fresh rewards mismatch: expected={after:?} observed={restored:?}"
            )
            .into());
        }
        evidence(&format!(
            "P19 rewards {classification} fresh_admission={restored:?}"
        ));
    }
    // Both reward proofs have completed. Release the fixture character for the runner.
    evidence("P19 FIXTURE_CLEANUP step=logout");
    let (result, detail, attempts, _) = fixture_logout_retrying_busy(&mut client).await?;
    if result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!(
            "P19 fixture cleanup logout failed after {attempts} attempt(s): {detail}"
        )
        .into());
    }
    Ok(())
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

#[derive(Debug, PartialEq, Eq)]
struct Rewards {
    dust: u64,
    limit: u64,
    slivers: u64,
}

fn valid_reward_transition(
    before: &Rewards,
    killed: &Rewards,
    after: &Rewards,
    stacks: u8,
) -> bool {
    // TibiaWiki, Dust and Slivers: Premium kills grant 1–3 dust per stack
    // (15–45 for Fiendish); only Fiendish corpses yield slivers when looted.
    // https://tibia.fandom.com/wiki/Fiendish_and_Influenced_Creatures#Dust_and_Slivers
    let stacks = u64::from(stacks);
    let dust_gain = killed.dust.checked_sub(before.dust);
    let remaining_dust = before.limit.checked_sub(before.dust);
    killed.limit == before.limit
        && after.limit == killed.limit
        && dust_gain.zip(remaining_dust).is_some_and(|(gain, remaining)| {
            (stacks..=3 * stacks).contains(&gain) && gain <= remaining
        })
        && after.dust == killed.dust
        && killed.slivers == before.slivers
        && if stacks == 15 {
            after.slivers > killed.slivers
        } else {
            after.slivers == killed.slivers
        }
}

#[test]
fn forge_rewards_reject_wrong_amount_or_phase() {
    let before = Rewards {
        dust: 0,
        limit: 100,
        slivers: 0,
    };
    let killed = Rewards { dust: 15, ..before };
    for gain in [0, 2, 3, 7, 8] {
        let after = Rewards {
            slivers: gain,
            ..killed
        };
        assert_eq!(
            valid_reward_transition(&before, &killed, &after, 15),
            gain > 0
        );
    }
    let good = Rewards {
        slivers: 3,
        ..killed
    };
    assert!(!valid_reward_transition(&before, &before, &good, 15));
    assert!(!valid_reward_transition(&before, &good, &good, 15));
    assert!(!valid_reward_transition(
        &before,
        &killed,
        &Rewards { limit: 101, ..good },
        15
    ));
    for dust in [14, 15, 45, 46, 101] {
        let killed = Rewards { dust, ..before };
        let after = Rewards { slivers: 1, ..killed };
        assert_eq!(
            valid_reward_transition(&before, &killed, &after, 15),
            (15..=45).contains(&dust)
        );
    }
    let near_cap = Rewards { dust: 90, ..before };
    let clipped = Rewards { dust: 100, ..near_cap };
    assert!(!valid_reward_transition(
        &near_cap,
        &clipped,
        &Rewards { slivers: 1, ..clipped },
        15
    ));
    let influenced = Rewards { dust: 3, ..before };
    assert!(valid_reward_transition(
        &before,
        &influenced,
        &influenced,
        3
    ));
    assert!(!valid_reward_transition(
        &before,
        &influenced,
        &Rewards {
            slivers: 1,
            ..influenced
        },
        3
    ));
}

async fn rewards(client: &mut DevClientSession) -> TestResult<Rewards> {
    let request = forge_preview::Request {
        nonce: 1,
        selection: forge_preview::Selection::Browse { after: 0 },
    };
    let bytes = request
        .encode()
        .ok_or("P19 invalid Forge resource request")?;
    let reply = client
        .session_mut()
        .gated_command(
            forge_preview::CAPABILITY_FORGE_PREVIEW_V1,
            forge_preview::COMMAND_TYPE_FORGE_PREVIEW,
            &bytes,
        )
        .await?;
    require_mechanic_acceptance(reply.status, "P19 Forge resources")?;
    let reply =
        forge_preview::Reply::decode(&reply.payload).ok_or("P19 malformed Forge resource reply")?;
    let forge_preview::Payload::Page(page) = reply.payload else {
        return Err("P19 Forge resource page absent".into());
    };
    if reply.nonce != request.nonce
        || page.after != 0
        || page.resources.dust > page.resources.dust_limit
    {
        return Err("P19 invalid Forge resource page".into());
    }
    Ok(Rewards {
        dust: u64::from(page.resources.dust),
        limit: u64::from(page.resources.dust_limit),
        slivers: page.resources.slivers,
    })
}

fn sliver_count(client: &DevClientSession, sliver: NonZeroU32) -> TestResult<u64> {
    Ok(client
        .inventory()
        .ok_or("P19 inventory absent")?
        .entries
        .iter()
        .filter(|entry| entry.item_definition_ref == sliver)
        .map(|entry| u64::from(entry.count.get()))
        .sum())
}

async fn kill_and_observe_clear(
    client: &mut DevClientSession,
    target: EntityRef,
) -> TestResult<[u8; 16]> {
    // Resolve the expected identity while the exact creature is alive. The
    // fixture supplies the expectation; only live frames can prove the kill.
    let identity: String = target
        .identity
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    let answer = request_fixture_control(
        uuid_text(client.join_snapshot().game_session_id.as_bytes()),
        "--inspect-damage",
        format!("{identity} {}", target.generation),
    )
    .await?;
    let mut fields = answer.split_ascii_whitespace();
    let (Some("damage"), Some("0"), Some(corpse), None) =
        (fields.next(), fields.next(), fields.next(), fields.next())
    else {
        return Err(format!("P19 expected untouched fixture creature, got {answer:?}").into());
    };
    let expected_corpse = crate::node::operator_files::hex_bytes::<16>(corpse)
        .map_err(|_| "P19 invalid expected corpse identity")?;
    evidence(&format!(
        "P19 FIXTURE target={identity} expected_corpse={corpse}"
    ));
    let entities = client.world_entities().ok_or("P19 spatial view absent")?;
    if entities.get(&target).is_none()
        || entities
            .iter()
            .any(|entity| entity.entity.identity == expected_corpse)
    {
        return Err("P19 expected a live target and no matching corpse before attack".into());
    }
    client.take_events();
    let attack = client.session_mut().attack_target(Some(&target)).await?;
    require_mechanic_acceptance(attack.status, "P19 attack")?;
    if attack.disposition != AttackIntentDisposition::Ok {
        return Err(format!("P19 first attack refused: {attack:?}").into());
    }
    tokio::time::timeout(Duration::from_secs(55), async {
        let mut left = false;
        let mut cleared = None;
        let mut observed_corpse = false;
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
                        if delta.value.leave.contains(&target) {
                            left = true;
                            evidence(&format!(
                                "P19 target-left entity={target:?} sequence={}",
                                delta.server_sequence
                            ));
                        }
                        if !observed_corpse
                            && delta
                                .value
                                .enter
                                .iter()
                                .chain(&delta.value.update)
                                .any(|entity| {
                                    entity.kind == EntityKind::Corpse
                                        && entity.entity.identity == expected_corpse
                                        && matches!(
                                            entity.detail,
                                            EntityDetail::Object {
                                                item_handle: Some(_),
                                                ..
                                            } | EntityDetail::ObjectWithAppearance {
                                                item_handle: Some(_),
                                                ..
                                            }
                                        )
                                })
                        {
                            observed_corpse = true;
                            evidence(&format!(
                                "P19 exact corpse={corpse} sequence={}",
                                delta.server_sequence
                            ));
                        }
                    }
                    _ => {}
                }
            }
            if left && cleared.is_some() && observed_corpse {
                return TestResult::Ok(expected_corpse);
            }
        }
    })
    .await
    .map_err(|_| format!("P19 exact kill, corpse or marker clear missing for {target:?}"))?
}

async fn quick_loot_corpse(
    client: &mut DevClientSession,
    corpse: [u8; 16],
    expected_items: usize,
) -> TestResult {
    let payload = quick_loot::encode_quick_loot_request(&quick_loot::QuickLootRequest {
        action: quick_loot::QuickLootAction::Click,
        corpse: Some(corpse),
        category: None,
        recipient: None,
        recipient_id: None,
        fallback_to_main: false,
        filter_mode: None,
        filter_definition_ref: None,
    })
    .map_err(|error| format!("P19 Quick Loot input: {error:?}"))?;
    let reply = client
        .session_mut()
        .gated_command(
            quick_loot::CAPABILITY_QUICK_LOOT_V1,
            quick_loot::COMMAND_TYPE_QUICK_LOOT,
            &payload,
        )
        .await?;
    require_mechanic_acceptance(reply.status, "P19 Quick Loot")?;
    let result = quick_loot::decode_quick_loot_result(&reply.payload)
        .map_err(|error| format!("P19 Quick Loot result: {error:?}"))?;
    // Global grants slivers only to fiendish creatures, with quantity based on difficulty.
    // https://tibia.fandom.com/wiki/Fiendish_and_Influenced_Creatures#Dust_and_Slivers
    if result.items.len() != expected_items || result.items.iter().any(|item| !item.accepted) {
        return Err(
            format!("P19 first Quick Loot did not collect the corpse items: {result:?}").into(),
        );
    }
    Ok(())
}

async fn corpse_contents(
    client: &mut DevClientSession,
    identity: [u8; 16],
    sliver: NonZeroU32,
) -> TestResult<(usize, u64)> {
    let corpse = client
        .world_entities()
        .and_then(|entities| {
            entities.iter().find(|entity| {
                entity.entity.identity == identity && entity.kind == EntityKind::Corpse
            })
        })
        .ok_or("P19 exact corpse absent before open")?;
    let (EntityDetail::Object {
        item_handle: Some(handle),
        ..
    }
    | EntityDetail::ObjectWithAppearance {
        item_handle: Some(handle),
        ..
    }) = corpse.detail
    else {
        return Err("P19 exact corpse has no item handle".into());
    };
    let use_result = client.use_item(handle).await?;
    require_mechanic_acceptance(use_result.status, "P19 open exact corpse")?;
    if use_result.disposition != oteryn_protocol_oteryn::world_object::UseDisposition::Committed {
        return Err(format!("P19 first corpse open refused: {use_result:?}").into());
    }
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if let Some(container) = client
                .open_container()
                .filter(|view| view.container_handle == Some(handle))
            {
                let quantity = container
                    .entries
                    .iter()
                    .filter(|entry| entry.item_definition_ref == sliver)
                    .map(|entry| u64::from(entry.count.get()))
                    .sum();
                return TestResult::Ok((container.entries.len(), quantity));
            }
            client.service_liveness(Duration::from_millis(50)).await?;
        }
    })
    .await
    .map_err(|_| "P19 exact corpse contents were not projected")?
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
