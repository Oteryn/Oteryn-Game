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
        // The measured floor-10 approach is 281 paced steps.
        Duration::from_secs(600),
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
    let mut client = connect_character(clients, 0x19, 1).await?;
    let preparation = client.join_snapshot().game_session_id;
    request_fixture_control(uuid_text(preparation.as_bytes()), "--profile", "knight".into()).await?;
    require_logout(&mut client, clients).await?;
    drop(client);
    let mut client = connect_character(clients, 0x19, 1).await?;
    client.session_mut().retain_qualification_travel_replay();
    if client.join_snapshot().game_session_id == preparation {
        return Err("P19 preparation reused the original session".into());
    }
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
    let summoner = character_prep::dedicated_character(clients, "NODE_BOOT_C16C_ACCOUNT").await?;
    let (accounts, characters) = summoner.replace_slot(clients, 1)?;
    let summoner_clients = SeamClients { accounts: &accounts, characters: &characters, ..*clients };
    let seed = 19;
    let summon = async {
        let mut summoner = character_prep::Seeded {
            profile: "c16c-sorcerer",
            reason: "P19 live owned summon census",
        }.connect(&summoner_clients, 0xd5, 1).await?;
        let owned = full_e2ef5::summon_rat_for_forge(&mut summoner, world).await?;
        evidence(&format!("P19 live summon={owned:?}"));
        Ok::<_, Box<dyn std::error::Error>>((summoner, owned))
    };
    let (mut summoner, owned) = while_servicing(&mut client, summon).await?;
    let draw = forge_population_draw(&mut client, &mut summoner, seed).await?;
    let mut selected = verify_forge_draw(&draw, seed, owned)?;
    selected.sort_by_key(|(_, _, stacks)| *stacks == 15);
    require_logout(&mut summoner, &summoner_clients).await?;
    drop(summoner);

        let before = rewards(&mut client, Path::new(&capture)).await?;
        if before
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
    let mut encounters = Vec::new();
    for (target, exact, stacks) in selected {
        let (classification, folder) = if stacks == 15 {
            ("fiendish", "fiendish")
        } else {
            ("influenced", "")
        };
        let creature_key = target["creature_key"].as_str().ok_or("P19 selected target lacks creature key")?;
        let position = &target["position"];
        let (x, y, floor) = (draw_number(position, "x")?, draw_number(position, "y")?,
            position["floor"].as_i64().ok_or("P19 selected target lacks floor")?);
        let origin = tile(client.world_spatial().actor_position)?;
        let path = forge_route(world, origin, |at| {
            u64::from(at.x).abs_diff(x) <= 1 && u64::from(at.y).abs_diff(y) <= 1
                && i64::from(at.floor) == -floor
                && (u64::from(at.x), u64::from(at.y)) != (x, y)
        }).ok_or_else(|| format!("P19 selected {creature_key} {exact:?} is unreachable from {origin:?} at {position}"))?;
        evidence(&format!("P19 approach entity={exact:?} origin={origin:?} target={position} steps={}", path.len()));
        let needs_entry_evidence = !encounters.iter().any(|(kind, _, _, _, _)| *kind == classification);
        let mut travel_events = move_first_attempt(&mut client, world, path, needs_entry_evidence.then_some(exact)).await?;
        if !needs_entry_evidence {
            tokio::time::timeout(Duration::from_secs(15), async {
                loop {
                    let appearances = client.session_mut().visible_appearances();
                    if let Some(entry) = appearances.iter().find(|entry| entry.entity.entity == exact) {
                        if entry.forge_stacks != Some(stacks) {
                            return Err(format!("P19 {creature_key} {exact:?} expected stacks={stacks}, got {:?}", entry.forge_stacks).into());
                        }
                        return TestResult::Ok(());
                    }
                    client.wait_for_events(Duration::from_secs(15)).await?;
                }
            }).await.map_err(|_| format!("P19 selected {creature_key} {exact:?} was not visible at {position}"))??;
            evidence(&format!("P19_FORGE creature_key={creature_key} entity={exact:?} forge_stacks={stacks}"));
            continue;
        }
        let first: u64 = tokio::time::timeout(Duration::from_secs(15), async {
            let mut first_binding = None;
            loop {
                for event in if travel_events.is_empty() { client.wait_for_events(Duration::from_secs(15)).await? } else { std::mem::take(&mut travel_events) } {
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
        evidence(&format!(
            "P19_FORGE creature_key={creature_key} entity={exact:?} forge_stacks={stacks} sequence={first}"
        ));
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
        let candidates = client.session_mut().visible_appearances();
        let entry = candidates.iter().find(|entry| entry.entity.entity == exact)
            .ok_or("P19 selected actor is missing from the live client")?;
        if entry.forge_stacks != Some(stacks)
        {
            return Err(format!("P19 wrong live creature or Forge marker for {exact:?}").into());
        }
        let selected = exact;
        encounters.push((classification, stacks, selected, creature_key, tile(client.world_spatial().actor_position)?));
    }
    for classification in ["influenced", "fiendish"] {
        if !encounters.iter().any(|(kind, _, _, _, _)| *kind == classification) {
            return Err(format!("P19 production draw selected no {classification} creature for the combat proof").into());
        }
    }
        let combat_tile = tile(client.world_spatial().actor_position)?;
        let retreat = route(world, combat_tile, |at| is_logout_rest_tile(world, at))
            .ok_or("P19 FIXTURE_PLACEMENT no rest tile for live-marker relog")?;
        move_first_attempt(&mut client, world, retreat, None).await?;
        let rest_position = client.world_spatial().actor_position;
        let previous = client.join_snapshot().game_session_id;
        require_logout(&mut client, clients).await?;
        drop(client);
        client = connect_character(clients, 0x19, 1).await?;
        client.session_mut().retain_qualification_travel_replay();
        if client.join_snapshot().game_session_id == previous
            || client.world_spatial().actor_position != rest_position
        {
            return Err("P19 live-marker relog did not restore the saved position in a new session".into());
        }
        let appearances = client.session_mut().visible_appearances();
        for &(_, stacks, exact, _, _) in &encounters {
        if !appearances.iter().any(|entry| {
            entry.entity.entity == exact && entry.forge_stacks == Some(stacks)
        }) {
            return Err(format!("P19 fresh admission lacks live marker for {exact:?} with stacks={stacks}").into());
        }
        evidence(&format!("P19 live-marker fresh admission entity={exact:?} stacks={stacks}"));
        }
        let mut before = before;
    let mut dead = Vec::new();
    for (classification, stacks, exact, creature_key, combat_tile) in encounters {
        let approach = forge_route(world, tile(client.world_spatial().actor_position)?, |at| at == combat_tile)
            .ok_or("P19 FIXTURE_PLACEMENT no return to combat tile")?;
        move_first_attempt(&mut client, world, approach, None).await?;
        let pre_attack = rewards(&mut client, Path::new(&capture)).await?;
        if pre_attack != before {
            return Err(format!("P19 rewards changed before the attack: before_spawn={before:?} before_attack={pre_attack:?}").into());
        }
        let corpse = kill_and_observe_clear(&mut client, exact).await?;
        let directory = Path::new(&capture).join(if stacks == 15 { "fiendish-death" } else { "influenced-death" });
        std::fs::create_dir_all(&directory)?;
        let files = client.session_mut().p19_departure_capture_files(exact, stacks)
            .ok_or("P19 exact death replay chain missing")?;
        for (name, bytes) in files {
            std::fs::write(directory.join(name), bytes)?;
        }

        let killed = rewards(&mut client, Path::new(&capture)).await?;
        dead.push(exact);
        require_dead_markers_absent(&mut client, &dead)?;
        let stopped = client.session_mut().attack_target(None).await?;
        require_mechanic_acceptance(stopped.status, "P19 stop attack")?;
        if stopped.disposition != AttackIntentDisposition::Ok {
            return Err(format!("P19 first attack stop refused: {stopped:?}").into());
        }
        let (corpse_entity, source) = corpse_contents(&mut client, corpse).await?;
        let corpse_slivers: u64 = source.iter().filter(|item| item.item_definition_ref == sliver)
            .map(|item| u64::from(item.count.get())).sum();
        let mut destination = backpack_quantities(&client)?;
        add_quantities(&mut destination, &source);
        let durable_corpse = full_e2ef3::e2ei_c::native_corpse_id(clients, &corpse_entity).await?;
        let occurrence = forge_kill_occurrence(clients, durable_corpse, creature_key).await?;
        let stacks_u64 = u64::from(stacks);
        let expected_dust = stacks_u64 + u64::from(occurrence[15]) % (2 * stacks_u64 + 1);
        let expected_slivers = slivers_for_kill(stacks, occurrence[14]);
        if !corpse_slivers_match(stacks, occurrence[14], corpse_slivers) {
            return Err(format!("P19 {classification} corpse slivers={corpse_slivers}, expected={expected_slivers}, occurrence={occurrence:?}").into());
        }
        let expected_items = corpse_item_ids(clients, durable_corpse, source.len()).await?;
        quick_loot_corpse(&mut client, corpse, &expected_items).await?;
        let after = rewards(&mut client, Path::new(&capture)).await?;
        let remaining_items = corpse_contents(&mut client, corpse).await?.1.len();
        if remaining_items != 0 {
            return Err(format!("P19 Quick Loot left {remaining_items} stacks in exact corpse {corpse:?}").into());
        }
        if backpack_quantities(&client)? != destination {
            return Err("P19 Quick Loot backpack quantities do not match all source stacks".into());
        }
        evidence(&format!("P19 Quick Loot source depleted corpse={corpse:?} before={} after=0", source.len()));
        let looted = after
            .slivers
            .checked_sub(killed.slivers)
            .ok_or("P19 backpack slivers decreased after loot")?;
        if !valid_reward_transition(&before, &killed, &after, expected_dust, expected_slivers)
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
        move_first_attempt(&mut client, world, retreat, None).await?;
        let at = client.world_spatial().actor_position;
        let previous = client.join_snapshot().game_session_id;
        require_logout(&mut client, clients).await?;
        drop(client);
        client = connect_character(clients, 0x19, 1).await?;
        client.session_mut().retain_qualification_travel_replay();
        if client.join_snapshot().game_session_id == previous
            || client.world_spatial().actor_position != at
        {
            return Err(
                "P19 fresh admission did not restore the saved position in a new session".into(),
            );
        }
        let restored = rewards(&mut client, Path::new(&capture)).await?;
        require_dead_markers_absent(&mut client, &dead)?;
        if restored != after || sliver_count(&client, sliver)? != restored.slivers
            || backpack_quantities(&client)? != destination
        {
            return Err(format!(
                "P19 fresh rewards mismatch: expected={after:?} observed={restored:?}"
            )
            .into());
        }
        evidence(&format!(
            "P19 rewards {classification} fresh_admission={restored:?}"
        ));
        before = restored;
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

async fn require_logout(client: &mut DevClientSession, clients: &SeamClients<'_>) -> TestResult {
    let session = uuid_text(client.join_snapshot().game_session_id.as_bytes());
    let reply = client.session_mut().logout().await?;
    if reply.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("P19 first logout refused: {reply:?}").into());
    }
    // Check before dropping the transport or entering the general fixture release wait.
    let released = tokio::time::timeout(Duration::from_secs(5), async {
        let mut db = sqlx::PgConnection::connect(clients.url).await?;
        sqlx::query_scalar::<_, bool>(
            "SELECT session_state=3 AND control_loss_epoch IS NULL
             FROM game_durability_reconnect_sessions WHERE game_session_id=$1::uuid",
        )
        .bind(&session)
        .fetch_one(&mut db)
        .await
    })
    .await??;
    if !released {
        return Err(format!("P19 accepted logout did not durably release session {session} without control loss").into());
    }
    Ok(())
}

// Forge selects across the loaded world, including creatures below the town search's
// two-floor limit. Each edge still uses the production collision and floor resolver.
fn forge_route(
    world: &BundleWorld,
    from: TilePos,
    goal: impl Fn(TilePos) -> bool,
) -> Option<Vec<StepDirection>> {
    find_route(from, goal, |at, direction| {
        let next = destination(world, at, direction)?;
        forge_route_cell(from, next).then_some(next)
    })
    .ok()
}

fn forge_route_cell(from: TilePos, next: TilePos) -> bool {
    next.x.abs_diff(from.x) <= 160 && next.y.abs_diff(from.y) <= 160
}

#[test]
fn p19_route_allows_three_floor_descent_within_horizontal_bound() {
    let temple = TilePos { x: 32369, y: 32238, floor: -7 };
    let shaman = TilePos { x: 32298, y: 32137, floor: -10 };
    assert!(forge_route_cell(temple, shaman));
    assert!(!forge_route_cell(temple, TilePos { x: temple.x + 161, ..shaman }));
}

fn collect_travel_events(
    client: &mut DevClientSession,
    target: Option<EntityRef>,
    events: &mut Vec<SessionEvent>,
) {
    events.extend(client.take_events().into_iter().filter(|event| {
        let Some(target) = target else { return false };
        match event {
            SessionEvent::Appearance(delta) => matches!(&delta.value, AppearanceUpdate::Mappings(mappings)
                if mappings.forgets.contains(&target) || mappings.upserts.iter().any(|entry| entry.entity == target)),
            SessionEvent::WorldSpatialEntities(delta) => delta.value.enter.iter().any(|entry| entry.entity == target),
            _ => false,
        }
    }));
}

async fn move_first_attempt(
    client: &mut DevClientSession,
    world: &BundleWorld,
    path: Vec<StepDirection>,
    entry_target: Option<EntityRef>,
) -> TestResult<Vec<SessionEvent>> {
    let start = tile(client.world_spatial().actor_position)?;
    let goal = path.iter().try_fold(start, |at, direction| {
        destination(world, at, *direction).ok_or("P19 route has no planned landing")
    })?;
    let mut remaining = std::collections::VecDeque::from(path);
    let mut blocked = Vec::new();
    let mut completed = 0;
    let mut entry_events = Vec::new();
    while let Some(direction) = remaining.pop_front() {
        collect_travel_events(client, entry_target, &mut entry_events);
        let origin = tile(client.world_spatial().actor_position)?;
        let expected = destination(world, origin, direction).ok_or("P19 route has no landing")?;
        let movement = client.step(direction).await?;
        let actual = tile(client.world_spatial().actor_position)?;
        if movement.disposition == StepDisposition::Blocked
            && actual == origin
            && blocked.len() < 4
        {
            blocked.push(expected);
            remaining = find_route(origin, |at| at == goal, |at, direction| {
                let next = destination(world, at, direction)?;
                (forge_route_cell(start, next) && !blocked.contains(&next)).then_some(next)
            })
            .map_err(|visited| format!("P19 no detour around {blocked:?} to {goal:?} after {visited} visited tiles"))?
            .into();
            evidence(&format!("P19 detour blocked={expected:?} goal={goal:?} remaining={}", remaining.len()));
            continue;
        }
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
        verdict::Harness::global().progress("walk");
        completed += 1;
        if completed % 32 == 0 || remaining.is_empty() {
            evidence(&format!("P19 walk confirmed={completed}/{} position={actual:?}", completed + remaining.len()));
        }
        if movement.step_wait_ms != 0 {
            let wait = Duration::from_millis(u64::from(movement.step_wait_ms));
            let until = tokio::time::Instant::now() + wait;
            while let Some(remaining) = until.checked_duration_since(tokio::time::Instant::now()) {
                client.service_liveness(remaining).await?;
            }
        }
    }
    collect_travel_events(client, entry_target, &mut entry_events);
    Ok(entry_events)
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
    expected_dust: u64,
    expected_slivers: u64,
) -> bool {
    // TibiaWiki, Dust and Slivers: Premium kills grant 1–3 dust per stack
    // (15–45 for Fiendish); only Fiendish corpses yield slivers when looted.
    // https://tibia.fandom.com/wiki/Fiendish_and_Influenced_Creatures#Dust_and_Slivers
    let dust_gain = killed.dust.checked_sub(before.dust);
    let remaining_dust = before.limit.checked_sub(before.dust);
    killed.limit == before.limit
        && after.limit == killed.limit
        && dust_gain.zip(remaining_dust).is_some_and(|(gain, remaining)| {
            gain == expected_dust && gain <= remaining
        })
        && after.dust == killed.dust
        && killed.slivers == before.slivers
        && after.slivers.checked_sub(killed.slivers) == Some(expected_slivers)
}

// Crystal 201189f8, configmanager.cpp:315–316 and monster.cpp:2528–2537.
const CRYSTAL_FIENDISH_SLIVER_MIN: u64 = 3;
const CRYSTAL_FIENDISH_SLIVER_MAX: u64 = 7;

fn slivers_for_kill(stacks: u8, roll: u8) -> u64 {
    if stacks == 15 {
        let span = CRYSTAL_FIENDISH_SLIVER_MAX - CRYSTAL_FIENDISH_SLIVER_MIN + 1;
        CRYSTAL_FIENDISH_SLIVER_MIN + u64::from(roll) % span
    } else {
        0
    }
}

fn corpse_slivers_match(stacks: u8, roll: u8, actual: u64) -> bool {
    // Parity gap: Global scales Fiendish Slivers with difficulty, but publishes
    // no numeric yields by tier. Use the approved pinned Crystal bounds here.
    // https://tibia.fandom.com/wiki/Fiendish_and_Influenced_Creatures#History
    if stacks == 15 {
        (CRYSTAL_FIENDISH_SLIVER_MIN..=CRYSTAL_FIENDISH_SLIVER_MAX).contains(&actual)
            && actual == slivers_for_kill(stacks, roll)
    } else {
        actual == 0
    }
}

#[test]
fn forge_corpse_sliver_oracle_rejects_wrong_counts_and_non_fiendish_drops() {
    for (roll, expected) in [(0, 3), (1, 4), (2, 5), (3, 6), (4, 7), (255, 3)] {
        for count in 0..=8 {
            assert_eq!(corpse_slivers_match(15, roll, count), count == expected);
        }
    }
    for stacks in 0..=5 {
        assert!(corpse_slivers_match(stacks, 2, 0));
        for count in 1..=8 {
            assert!(!corpse_slivers_match(stacks, 2, count));
        }
    }
}

async fn forge_kill_occurrence(clients: &SeamClients<'_>, corpse: [u8; 16], creature_key: &str) -> TestResult<[u8; 16]> {
    tokio::time::timeout(Duration::from_secs(5), async {
        let mut db = sqlx::PgConnection::connect(clients.url).await?;
        let (actor, generation, receipts): (i64, String, Vec<Vec<u8>>) = sqlx::query_as(
            "SELECT m.death_actor_local_id, m.death_actor_local_generation::text,
                    ARRAY(SELECT uuid_send(b.bestiary_occurrence_id)
                          FROM game_character_bestiary_kill_receipts b
                          WHERE b.character_id=m.corpse_top_damage_character_id
                            AND b.race_key IN ($3, split_part($3, 'oteryn:creature.', 2)))
             FROM game_item_mint_receipts m
             WHERE m.item_instance_id=encode($1,'hex')::uuid
               AND m.corpse_top_damage_character_id=encode($2,'hex')::uuid
               AND m.loot_purpose_key='CORPSE_MATERIALIZATION'",
        )
        .bind(corpse.as_slice())
        .bind(clients.characters[1].as_slice())
        .bind(creature_key)
        .fetch_one(&mut db)
        .await?;
        let actor = u32::try_from(actor)?;
        let generation = generation.parse::<u64>()?;
        let matches: Vec<_> = receipts.into_iter()
            .filter_map(|receipt| <[u8; 16]>::try_from(receipt).ok())
            .filter(|receipt| receipt_matches_death(
                [clients.world, clients.channel, clients.characters[1]],
                clients.scope_generation, actor, generation, *receipt,
            ))
            .collect();
        let [receipt] = matches.as_slice() else {
            return Err(format!("P19 exact Forge corpse {corpse:?} has {} matching Bestiary receipts", matches.len()).into());
        };
        Ok(*receipt)
    }).await?
}

fn receipt_matches_death(
    [world, channel, principal]: [[u8; 16]; 3],
    scope_generation: u64,
    actor: u32,
    generation: u64,
    receipt: [u8; 16],
) -> bool {
    use sha2::{Digest, Sha256};
    // The reward occurrence is UUIDv7-shaped. Its timestamp is public and the
    // digest binds it to the mint receipt's exact death actor and principal.
    let digest = Sha256::new()
        .chain_update(b"oteryn:combat-death-xp-occurrence:v1")
        .chain_update(world)
        .chain_update(channel)
        .chain_update(scope_generation.to_be_bytes())
        .chain_update(actor.to_be_bytes())
        .chain_update(generation.to_be_bytes())
        .chain_update(principal)
        .chain_update([0, 0])
        .chain_update(&receipt[..6])
        .finalize();
    receipt[6] == (0x70 | (digest[0] & 0x0f))
        && receipt[7] == digest[1]
        && receipt[8] == (0x80 | (digest[2] & 0x3f))
        && receipt[9..] == digest[3..10]
}

#[test]
fn forge_occurrence_rejects_another_same_race_death() {
    let scope = [[1; 16], [2; 16], [3; 16]];
    let receipt = [1, 149, 103, 120, 137, 154, 113, 95, 190, 59, 57, 105, 50, 163, 159, 58];
    assert!(receipt_matches_death(scope, 4, 42, 7, receipt));
    assert!(!receipt_matches_death(scope, 4, 43, 7, receipt));
    assert!(!receipt_matches_death(scope, 4, 42, 8, receipt));
    assert!(!receipt_matches_death(scope, 5, 42, 7, receipt));
}

#[test]
fn forge_rewards_reject_wrong_amount_or_phase() {
    let before = Rewards {
        dust: 0,
        limit: 100,
        slivers: 0,
    };
    let killed = Rewards { dust: 15, ..before };
    for gain in [0, 1, 2, 3, 4, 5, 6, 7, 8] {
        let after = Rewards {
            slivers: gain,
            ..killed
        };
        assert_eq!(
            valid_reward_transition(&before, &killed, &after, 15, slivers_for_kill(15, 2)),
            gain == 5,
            "fiendish sliver gain {gain}"
        );
    }
    let good = Rewards {
        slivers: 3,
        ..killed
    };
    assert!(!valid_reward_transition(&before, &before, &good, 15, 3));
    assert!(!valid_reward_transition(&before, &good, &good, 15, 3));
    assert!(!valid_reward_transition(
        &before,
        &killed,
        &Rewards { limit: 101, ..good },
        15,
        3,
    ));
    for dust in [14, 15, 24, 45, 46, 101] {
        let killed = Rewards { dust, ..before };
        let after = Rewards { slivers: 3, ..killed };
        assert_eq!(
            valid_reward_transition(&before, &killed, &after, 24, 3),
            dust == 24
        );
    }
    let near_cap = Rewards { dust: 90, ..before };
    let clipped = Rewards { dust: 100, ..near_cap };
    assert!(!valid_reward_transition(
        &near_cap,
        &clipped,
        &Rewards { slivers: 3, ..clipped },
        15,
        3,
    ));
    let influenced = Rewards { dust: 3, ..before };
    assert!(valid_reward_transition(
        &before,
        &influenced,
        &influenced,
        3,
        0,
    ));
    assert!(!valid_reward_transition(
        &before,
        &influenced,
        &Rewards {
            slivers: 1,
            ..influenced
        },
        3,
        0,
    ));
}

async fn forge_population_draw(
    client: &mut DevClientSession,
    summoner: &mut DevClientSession,
    seed: u64,
) -> TestResult<serde_json::Value> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let session = uuid_text(client.join_snapshot().game_session_id.as_bytes());
    let draw = tokio::time::timeout(Duration::from_secs(30), async {
        let mut socket = tokio::net::UnixStream::connect(required("NODE_BOOT_CONTROL_SOCKET")?).await?;
        socket.write_all(format!("forge-population-draw {session} {seed}\n").as_bytes()).await?;
        socket.shutdown().await?;
        let mut reply = String::new();
        socket.take(4 * 1024 * 1024).read_to_string(&mut reply).await?;
        let census = serde_json::from_str(&reply)
            .map_err(|error| format!("P19 production Forge draw: {error}; reply={reply:.200}"))?;
        evidence("P19 production Forge draw returned a census");
        Ok(census)
    });
    while_servicing(client, while_servicing(summoner, async { draw.await? })).await
}

async fn while_servicing<T>(
    client: &mut DevClientSession,
    operation: impl std::future::Future<Output = TestResult<T>>,
) -> TestResult<T> {
    use std::sync::atomic::{AtomicBool, Ordering::Relaxed};
    let done = AtomicBool::new(false);
    // Let a started frame or liveness acknowledgement finish before resuming commands.
    let (result, serviced) = tokio::join!(
        async {
            let result = operation.await;
            done.store(true, Relaxed);
            result
        },
        async {
            while !done.load(Relaxed) {
                client.service_liveness(Duration::from_millis(100)).await?;
            }
            TestResult::Ok(())
        },
    );
    serviced?;
    result
}

fn draw_number(value: &serde_json::Value, key: &str) -> TestResult<u64> {
    value[key].as_u64().ok_or_else(|| format!("P19 Forge draw missing numeric {key}").into())
}

fn draw_entity(value: &serde_json::Value) -> TestResult<EntityRef> {
    let generation = draw_number(value, "generation")?;
    let id = value["id"].as_str().ok_or("P19 Forge draw missing actor id")?;
    let identity = crate::node::operator_files::hex_bytes::<16>(&id.replace('-', ""))
        .map_err(|_| "P19 Forge draw invalid actor id")?;
    Ok(EntityRef { identity, generation })
}

fn verify_forge_draw(draw: &serde_json::Value, seed: u64, owned: EntityRef)
-> TestResult<Vec<(&serde_json::Value, EntityRef, u8)>> {
    let array = |key: &str| draw[key].as_array().ok_or_else(|| format!("P19 Forge draw missing {key}"));
    let eligible = array("eligible")?.iter()
        .map(|entry| Ok((draw_entity(entry)?, draw_number(entry, "spawn_identity")?)))
        .collect::<TestResult<Vec<_>>>()?;
    let selected = array("selected")?.iter()
        .map(|entry| Ok((entry, draw_entity(entry)?, u8::try_from(draw_number(entry, "stacks")?)?)))
        .collect::<TestResult<Vec<_>>>()?;
    let exclusions = array("excluded_owned_summons")?.iter()
        .filter_map(|entry| Some((crate::node::uuid_bytes(entry["id"].as_str()?)?, &entry["reason"])))
        .collect::<Vec<_>>();
    if !exclusions.iter().any(|(actor, reason)| *actor == owned.identity && *reason == "owned_summon")
        || selected.iter().any(|(_, actor, _)| *actor == owned)
        || eligible.iter().any(|(actor, _)| *actor == owned)
    {
        return Err("P19 live owned summon exclusion mismatch".into());
    }
    if draw_number(draw, "seed")? != seed {
        return Err("P19 Forge draw returned the wrong seed".into());
    }
    crate::qualification::forge_draw_replay::assert_replayed_selection(draw)?;
    evidence("P19 exact production draw and owned summon exclusion verified");
    Ok(selected)
}

async fn rewards(client: &mut DevClientSession, capture: &Path) -> TestResult<Rewards> {
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
    use oteryn_protocol_oteryn::item_forge::{
        CAPABILITY_ITEM_FORGE_V1, COMMAND_TYPE_FORGE_DUST_QUERY,
    };
    let native = client.session_mut()
        .gated_command(CAPABILITY_ITEM_FORGE_V1, COMMAND_TYPE_FORGE_DUST_QUERY, &[])
        .await?;
    require_mechanic_acceptance(native.status, "P19 Current Dust")?;
    let native_balance: serde_json::Value = serde_json::from_slice(&native.payload)?;
    if native_balance["current_dust"].as_u64() != Some(u64::from(page.resources.dust))
        || native_balance["dust_limit"].as_u64() != Some(u64::from(page.resources.dust_limit))
    {
        return Err(format!("P19 Current Dust disagrees with exact resources: {native_balance}").into());
    }
    use std::io::Write;
    let mut output = std::fs::OpenOptions::new().create(true).append(true)
        .open(capture.join("p19-current-dust.jsonl"))?;
    serde_json::to_writer(&mut output, &(
        page.resources.dust,
        page.resources.dust_limit,
        native.payload,
    ))?;
    writeln!(output)?;
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
                            if !delta.value.enter.iter().any(|entity| {
                                entity.kind == EntityKind::Corpse
                                    && entity.entity.identity == expected_corpse
                                    && matches!(
                                        entity.detail,
                                        EntityDetail::Object { item_handle: Some(_), .. }
                                            | EntityDetail::ObjectWithAppearance { item_handle: Some(_), .. }
                                    )
                            }) {
                                return Err(format!("P19 target left without the exact corpse entering in delta {}", delta.server_sequence).into());
                            }
                            left = true;
                            evidence(&format!(
                                "P19 exact corpse={corpse} sequence={}",
                                delta.server_sequence
                            ));
                        }
                    }
                    _ => {}
                }
            }
            if left && cleared.is_some() {
                return TestResult::Ok(expected_corpse);
            }
        }
    })
    .await
    .map_err(|_| format!("P19 exact kill, corpse or marker clear missing for {target:?}"))?
}

fn require_dead_markers_absent(client: &mut DevClientSession, dead: &[EntityRef]) -> TestResult {
    if client.session_mut().visible_appearances().iter()
        .any(|entry| dead.contains(&entry.entity.entity))
    {
        return Err("P19 appearance binding reappeared for a dead Forge creature".into());
    }
    Ok(())
}

type ItemQuantities = std::collections::BTreeMap<(NonZeroU32, u32), u64>;

fn add_quantities(
    quantities: &mut ItemQuantities,
    entries: &[oteryn_protocol_oteryn::item_view::ItemEntry],
) {
    for item in entries {
        *quantities.entry((item.item_definition_ref, item.sub_type)).or_default() +=
            u64::from(item.count.get());
    }
}

fn backpack_quantities(client: &DevClientSession) -> TestResult<ItemQuantities> {
    let mut quantities = ItemQuantities::new();
    add_quantities(&mut quantities, &client.inventory().ok_or("P19 inventory absent")?.entries);
    Ok(quantities)
}

async fn corpse_item_ids(
    clients: &SeamClients<'_>,
    corpse: [u8; 16],
    expected_count: usize,
) -> TestResult<std::collections::BTreeSet<[u8; 16]>> {
    // The legacy corpse projection exposes session handles. The Quick Loot
    // receipt uses durable IDs, so read the exact fixture source before moving it.
    let ids = tokio::time::timeout(Duration::from_secs(5), async {
        let mut db = sqlx::PgConnection::connect(clients.url).await?;
        sqlx::query_scalar::<_, Vec<u8>>(
            "SELECT uuid_send(e.item_instance_id) FROM game_item_corpse_container_entries e
             JOIN game_item_instances i USING(item_instance_id, world_id)
             WHERE e.parent_item_instance_id=encode($1,'hex')::uuid
               AND i.lifecycle=1 ORDER BY e.item_instance_id LIMIT $2",
        )
        .bind(corpse.as_slice())
        .bind(i64::try_from(expected_count).unwrap_or(i64::MAX - 1).saturating_add(1))
        .fetch_all(&mut db)
        .await
    }).await??;
    if ids.len() != expected_count {
        return Err(format!("P19 corpse wire stack count {expected_count} disagrees with {} durable source items", ids.len()).into());
    }
    ids.into_iter()
        .map(|id| id.try_into().map_err(|_| "P19 malformed corpse source item ID".into()))
        .collect()
}

fn exact_loot_receipt(
    expected: &std::collections::BTreeSet<[u8; 16]>,
    items: &[quick_loot::QuickLootItemResult],
) -> bool {
    items.len() == expected.len()
        && items.iter().all(|item| item.accepted)
        && items.iter().map(|item| item.item).collect::<std::collections::BTreeSet<_>>() == *expected
}

#[test]
fn quick_loot_receipt_rejects_duplicate_wrong_or_refused_items() {
    let expected = [[1; 16], [2; 16]].into_iter().collect();
    let item = |id, accepted| quick_loot::QuickLootItemResult { item: [id; 16], accepted };
    assert!(exact_loot_receipt(&expected, &[item(2, true), item(1, true)]));
    assert!(!exact_loot_receipt(&expected, &[item(1, true), item(1, true)]));
    assert!(!exact_loot_receipt(&expected, &[item(1, true), item(3, true)]));
    assert!(!exact_loot_receipt(&expected, &[item(1, true), item(2, false)]));
}

async fn quick_loot_corpse(
    client: &mut DevClientSession,
    corpse: [u8; 16],
    expected_items: &std::collections::BTreeSet<[u8; 16]>,
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
    if !exact_loot_receipt(expected_items, &result.items) {
        return Err(
            format!("P19 first Quick Loot did not collect the corpse items: {result:?}").into(),
        );
    }
    Ok(())
}

async fn corpse_contents(
    client: &mut DevClientSession,
    identity: [u8; 16],
) -> TestResult<(oteryn_protocol_oteryn::world_spatial_entities::WorldSpatialEntity, Vec<oteryn_protocol_oteryn::item_view::ItemEntry>)> {
    let corpse = client
        .world_entities()
        .and_then(|entities| {
            entities.iter().find(|entity| {
                entity.entity.identity == identity && entity.kind == EntityKind::Corpse
            })
        })
        .copied()
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
    client.take_events();
    let use_result = client.use_item(handle).await?;
    require_mechanic_acceptance(use_result.status, "P19 open exact corpse")?;
    if use_result.disposition != oteryn_protocol_oteryn::world_object::UseDisposition::Committed {
        return Err(format!("P19 first corpse open refused: {use_result:?}").into());
    }
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            for event in client.wait_for_events(Duration::from_secs(5)).await? {
                let SessionEvent::OpenContainer(delta) = event else {
                    continue;
                };
                let container = delta.value;
                if delta.server_sequence <= use_result.result_server_sequence
                    || container.container_handle != Some(handle)
                {
                    continue;
                }
                return TestResult::Ok((corpse, container.entries));
            }
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
