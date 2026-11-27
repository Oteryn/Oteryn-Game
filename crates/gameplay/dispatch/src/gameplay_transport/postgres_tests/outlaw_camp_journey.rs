//! Chosen Outlaw Camp route through the shipped map, USE, STEP and the quest journal.
use super::fixture::{Harness, debug, id};
use super::outlaw_camp_treasure_fixtures::{TREASURE_KEY, seed_treasure_key};
use super::quest_world_callbacks_fixtures::wire;
use super::questpc8_authored_programmes_fixtures::use_target as use_object;
use super::{
    AdmittedSession, ComposedFreshAdmission, Dependencies, TestResult, prepare_current_session,
};
use crate::content::channel_content_pin::ChannelContentPin;
use crate::content::{CollisionClass, LogicalCell};
use crate::durability::{fresh_admission::FreshAdmissionStore, item_mint::TypedDefinitionRef};
use crate::foundation::ChannelRuntimeV1;
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{FirstEntryOutcome, FreshAdmissionAuthority};
use crate::gameplay_transport::item_ref_admission::NativeItemWeights;
use crate::gameplay_transport::quest_log::QuestLogLines;
use crate::map::{
    overlay::TilePos,
    view::{MapFacts, placement_key},
};
use oteryn_protocol_oteryn::quest_log::{
    COMMAND_TYPE_QUEST_LOG_QUERY, QuestLogQuery, QuestLogView, decode_quest_log,
    encode_quest_log_query,
};
use oteryn_protocol_oteryn::world_object::{
    MapUseTarget, UseDisposition, UseIntent, UseTarget, WorldObjectTarget, encode_item_use_intent,
};
use oteryn_protocol_oteryn::world_spatial::{StepDirection, StepDirection::*};
use std::collections::{BTreeMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};

const QUEST: &str = "oteryn:quest.the_outlaw_camp_quest";
const STAGE: &str = "oteryn:quest-progress/authored/outlaw-camp/stage";
const ENTRANCE: &str = "oteryn:quest-progress/authored/outlaw-camp/entrance";
const TREASURE: &str = "oteryn:quest-progress/quest/u6_4/outlaw_camp_quest";
const TERMINAL: &str = "oteryn:quest-callback/authored-outlaw-camp-s3-treasure";
const START: TilePos = TilePos {
    x: 32620,
    y: 32247,
    floor: -7,
};
const SURFACE_ENTRANCE: TilePos = TilePos {
    x: 32619,
    y: 32246,
    floor: -7,
};
const MINE: TilePos = TilePos {
    x: 32620,
    y: 32200,
    floor: -10,
};
const PASSAGE: TilePos = TilePos {
    x: 32620,
    y: 32199,
    floor: -10,
};
const CHEST: TilePos = TilePos {
    x: 32620,
    y: 32198,
    floor: -10,
};

#[test]
fn outlaw_camp_full_journey() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    let started = Instant::now();
    super::kill_reward_e2e_fixtures::scenario_thread::run("outlaw-camp", move || {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?
            .block_on(journey(admin))
    })?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED outlaw_camp_full_journey duration={:?}",
        started.elapsed()
    );
    Ok(())
}

async fn journey(admin: String) -> TestResult {
    use oteryn_gameplay_quests::quest::{
        loader::load_embedded_quest_state, log::loader::load_embedded_quest_log,
    };
    let loaded = load_embedded_quest_state("content-1").map_err(debug)?;
    let catalogue = Arc::new(loaded.catalogue().clone());
    assert!(
        catalogue.transition(TERMINAL).is_some(),
        "Outlaw Camp terminal binding is absent"
    );
    let journal = load_embedded_quest_log(&loaded)?.content;
    let journal_index = u32::try_from(
        journal
            .active()
            .quests()
            .iter()
            .position(|q| q.key == QUEST)
            .ok_or("canonical Outlaw Treasure journal")?,
    )?;
    let log = QuestLogLines::new(journal).ok_or("quest log bounds")?;
    let h = Harness::create_with_admission_seed(admin, "outlaw_camp_journey", false, false).await?;
    let request = prepare_current_session(&h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let mut deps = Dependencies::new()?;
    let mut admitted = deps.admitted;
    let (controller, activated) = super::death_ml_loss_composed::production_generation_with_pin(
        admitted.world_id,
        "content-1",
    )?;
    let generation = controller.active().ok_or("active generation")?;
    let native = generation.native_gameplay().ok_or("native gameplay")?;
    // The World compiler loads all floors in the region; the exact anchors
    // and passability are checked below by the live claims and movement.
    let world = super::source_world_crop::compiled_region(START, START, START, native)?;
    let claims = crate::content::world_reward_claims::bind_embedded_reward_claims(
        world.base().as_ref(),
        &|_| true,
    )?;
    let cells = world
        .movement_cells(activated.qualified_room().movement_cells())
        .map_err(debug)?;
    let server_digest = generation.identity().server_artifact_digest();
    let client_digest = generation.identity().client_artifact_digest();
    let frame = world.frame_binding_digest(admitted.world_id);
    let start = crate::content::NativeEntryStart {
        x: i32::from(START.x),
        y: i32::from(START.y),
        floor: -i16::from(START.floor),
    };
    deps.combat_room = Some(
        activated
            .qualified_room()
            .bundle_view(cells.clone(), frame, world.map_revision_digest(), start)
            .ok_or("Outlaw Camp bundle room")?,
    );
    let pin = ChannelContentPin::from_activation(
        admitted.world_id,
        1,
        server_digest,
        client_digest,
        frame,
        world.map_revision_digest(),
        (start.x, start.y, start.floor),
    );
    let mut runtime = ChannelRuntimeV1::from_committed_assignment(
        admitted.world_id,
        admitted.channel_id,
        crate::foundation::NodeId::decode(&id(1)).map_err(debug)?,
        1,
        1,
        1,
        "runtime-scope-assignment:1",
        2,
        pin,
    )
    .map_err(debug)?;
    let reservation = runtime
        .reserve_fresh_session(admitted.game_session_id)
        .map_err(debug)?;
    deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
    runtime
        .initialize_first_entry_position(deps.actor)
        .map_err(debug)?;
    let current = FreshAdmissionStore::from_root(h.root.clone())
        .current_session_at(admitted.game_session_id)
        .await?
        .0;
    runtime
        .bind_attacker_lease(
            deps.actor,
            admitted.game_session_id,
            current.current_character_lease(),
        )
        .map_err(debug)?;
    deps.runtime = tokio::sync::Mutex::new(runtime);
    let fence = crate::gameplay_transport::item_fence_of(current).ok_or("item fence")?;
    sqlx::query("INSERT INTO game_control_scope_grants VALUES (session_user, encode($1,'hex')::uuid, encode($2,'hex')::uuid, 4)")
        .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid, encode($2,'hex')::uuid, 1, NULL, $3, $4, $5)")
        .bind(id(42).as_slice()).bind(id(43).as_slice()).bind(server_digest.as_slice())
        .bind(client_digest.as_slice()).bind(frame.as_slice()).execute(&h.pool).await?;
    h.root
        .initialize_admission_equipment(
            &authority,
            &h.node,
            &fence,
            server_digest,
            request.operation(),
        )
        .await
        .map_err(debug)?;
    h.root
        .grant_starter_items(
            &authority,
            &h.node,
            &fence,
            &NativeItemWeights(native),
            &crate::durability::starter_grant::StarterTemplate {
                definition: TypedDefinitionRef {
                    family: "Item".into(),
                    production_key: "oteryn:item.tibia.i2854".into(),
                    revision_ref: "definition-r1".into(),
                },
                ..crate::durability::starter_grant::StarterTemplate::main()
            },
        )
        .await
        .map_err(debug)?;
    admitted.runtime_actor = Some(deps.actor);
    admitted.first_entry = FirstEntryOutcome::Positioned;
    admitted.item_fence = Some(fence);

    let mut owner = deps.owner(&h, &authority);
    owner.active_generation = Some(generation);
    owner.quest_catalogue = Some(Arc::clone(&catalogue));
    owner.quest_log_content = Some(log.clone());
    owner.movement_cells = (&cells).into();
    admitted.continuity.selected_capabilities =
        SelectedCapabilities::select(owner.offered_capabilities(), &[4, 6, 11, 13, 15, 16, 18])
            .ok_or("journey capabilities")?;
    owner.admit_progression_session(&mut admitted).await;
    deps.initialize(&owner).await;
    owner.admit_quest_session(&admitted).await;
    super::questpc8_magic_sword_fixtures::raise_level(&owner, &admitted, 45, 201).await?;
    assert_snapshot(&h, &owner, &admitted, &[], (0, 0, false)).await?;

    for number in [3301, 3302, 3303] {
        let claim = format!("oteryn:reward-claim.quest.key.id{number}");
        let [placement] = claims
            .served
            .iter()
            .find(|entry| entry.key.as_str() == claim)
            .ok_or_else(|| format!("missing production World claim {number}"))?
            .placements
            .as_slice()
        else {
            return Err(format!("ambiguous key placement {number}").into());
        };
        let (cell, _) = crate::map::view::placement_of(placement.bundle_placement_key)
            .ok_or("key source coordinates")?;
        let path = surface_path(&owner, actor_cell(&owner, &admitted).await?, cell, true)?;
        for direction in path {
            step(&mut owner, &mut admitted, direction).await?;
        }
        let target = target(&owner, placement.bundle_placement_key)?;
        let command = admitted.continuity.next_command_id;
        assert_eq!(
            use_object(&mut owner, &mut admitted, command, &target).await?,
            UseDisposition::Committed
        );
        let keys = match number {
            3301 => &[3301][..],
            3302 => &[3301, 3302][..],
            _ => &[3301, 3302, 3303][..],
        };
        assert_snapshot(
            &h,
            &owner,
            &admitted,
            keys,
            (i64::from(number == 3303), 0, false),
        )
        .await?;
    }

    let keys = [3301, 3302, 3303];
    let path = surface_path(
        &owner,
        actor_cell(&owner, &admitted).await?,
        SURFACE_ENTRANCE,
        false,
    )?;
    for direction in path {
        step(&mut owner, &mut admitted, direction).await?;
    }
    assert_eq!(
        actor_cell(&owner, &admitted).await?,
        MINE,
        "the guarded entrance reaches the mine"
    );
    assert_snapshot(&h, &owner, &admitted, &keys, (1, 1, false)).await?;

    let door = target(
        &owner,
        placement_key(PASSAGE, 1).ok_or("passage placement")?,
    )?;
    let command = admitted.continuity.next_command_id;
    assert_eq!(
        use_object(&mut owner, &mut admitted, command, &door).await?,
        UseDisposition::Locked
    );
    assert_snapshot(&h, &owner, &admitted, &keys, (1, 1, false)).await?;
    // The treasure key comes from a separate mine pickup outside this route.
    seed_treasure_key(&h).await?;
    let keys = [3301, 3302, 3303, 3304];
    unlock(&mut owner, &mut admitted, door).await?;
    assert_snapshot(&h, &owner, &admitted, &keys, (1, 1, false)).await?;
    step(&mut owner, &mut admitted, North).await?;
    assert_eq!(actor_cell(&owner, &admitted).await?, PASSAGE);
    assert_snapshot(&h, &owner, &admitted, &keys, (2, 1, false)).await?;

    let chest = target(&owner, placement_key(CHEST, 2).ok_or("treasure placement")?)?;
    let command = admitted.continuity.next_command_id;
    assert_eq!(
        use_object(&mut owner, &mut admitted, command, &chest).await?,
        UseDisposition::Committed
    );
    assert_snapshot(&h, &owner, &admitted, &keys, (3, 1, true)).await?;
    let commands = std::mem::take(
        &mut Arc::get_mut(&mut owner.states)
            .ok_or("isolated Outlaw owner state")?
            .command_ingress,
    );
    drop(owner);

    // Reload the durable quest copy while retaining the Channel's command stream.
    // The reconnecting client resends the original command after losing its reply.
    let mut reconnect = deps.owner(&h, &authority);
    reconnect.active_generation = Some(generation);
    reconnect.quest_catalogue = Some(catalogue);
    reconnect.quest_log_content = Some(log);
    reconnect.movement_cells = (&cells).into();
    Arc::get_mut(&mut reconnect.states)
        .ok_or("isolated Outlaw reconnect state")?
        .command_ingress = commands;
    reconnect.admit_progression_session(&mut admitted).await;
    deps.initialize(&reconnect).await;
    reconnect.admit_quest_session(&admitted).await;
    assert_eq!(
        use_object(&mut reconnect, &mut admitted, command, &chest).await?,
        UseDisposition::Committed
    );
    assert_snapshot(&h, &reconnect, &admitted, &keys, (3, 1, true)).await?;
    let fresh_command = admitted.continuity.next_command_id;
    assert_eq!(
        use_object(&mut reconnect, &mut admitted, fresh_command, &chest).await?,
        UseDisposition::Rejected
    );
    assert_snapshot(&h, &reconnect, &admitted, &keys, (3, 1, true)).await?;

    let cmd = admitted.continuity.next_command_id;
    let result = wire(
        &reconnect,
        &mut admitted,
        cmd,
        COMMAND_TYPE_QUEST_LOG_QUERY,
        encode_quest_log_query(&QuestLogQuery::ListOnly).map_err(debug)?,
    )
    .await?;
    let Some(QuestLogView::List(entries)) = decode_quest_log(&result).map_err(debug)?.view else {
        return Err("expected Quest Log list".into());
    };
    assert!(
        entries
            .iter()
            .any(|entry| entry.quest_index == journal_index && entry.completed)
    );
    Ok(())
}

async fn actor_cell(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
) -> TestResult<TilePos> {
    let cell = owner
        .runtime
        .lock()
        .await
        .read_actor_position(admitted.runtime_actor.ok_or("journey actor")?)
        .map_err(debug)?
        .position();
    Ok(TilePos {
        x: cell.x.try_into()?,
        y: cell.y.try_into()?,
        floor: (-cell.floor).try_into()?,
    })
}

fn surface_path(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    start: TilePos,
    goal: TilePos,
    adjacent: bool,
) -> TestResult<Vec<StepDirection>> {
    let cells = owner.effective_movement_cells();
    let mut parents = BTreeMap::from([(start, (start, North))]);
    let mut pending = VecDeque::from([start]);
    while let Some(cell) = pending.pop_front() {
        if cell == goal || adjacent && cell.x.abs_diff(goal.x) <= 1 && cell.y.abs_diff(goal.y) <= 1
        {
            let mut path = Vec::new();
            let mut current = cell;
            while current != start {
                let (parent, direction) = parents[&current];
                path.push(direction);
                current = parent;
            }
            path.reverse();
            return Ok(path);
        }
        for direction in [North, East, South, West] {
            let (dx, dy) = direction.delta();
            let x = i32::from(cell.x) + dx;
            let y = i32::from(cell.y) + dy;
            if !(32595..=32665).contains(&x) || !(32225..=32265).contains(&y) {
                continue;
            }
            let next = TilePos {
                x: x.try_into()?,
                y: y.try_into()?,
                floor: start.floor,
            };
            if adjacent && next == SURFACE_ENTRANCE {
                continue;
            }
            let logical = LogicalCell {
                x,
                y,
                z: -i32::from(next.floor),
            };
            if !matches!(
                cells.index().lookup(cells.scope(), logical),
                Ok(CollisionClass::Walkable)
            ) {
                continue;
            }
            if let std::collections::btree_map::Entry::Vacant(entry) = parents.entry(next) {
                entry.insert((cell, direction));
                pending.push_back(next);
            }
        }
    }
    Err(format!("no surface route from {start:?} to {goal:?}").into())
}

async fn step(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    direction: StepDirection,
) -> TestResult {
    let command = admitted.continuity.next_command_id;
    super::questpc8_authored_programmes_fixtures::step(owner, admitted, command, direction).await
}

fn target(owner: &ComposedFreshAdmission<'_, '_, '_>, key: u64) -> TestResult<WorldObjectTarget> {
    let map = owner.bundle_map().ok_or("native World")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    Ok(WorldObjectTarget {
        placement,
        expected_revision: map.facts.object_revision(key),
    })
}

async fn unlock(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    door: WorldObjectTarget,
) -> TestResult {
    let index = owner
        .active_generation
        .and_then(|generation| generation.native_gameplay())
        .and_then(|native| native.item_index())
        .ok_or("key definitions")?;
    let copper = index
        .definition_ref("Item", "oteryn:item.tibia.i2970", "definition-r1")
        .ok_or("copper key definition")?;
    let treasure = index
        .definition_ref("Item", TREASURE_KEY, "definition-r1")
        .ok_or("treasure key definition")?;
    let payload = encode_item_use_intent(&UseIntent {
        target: UseTarget::Item(std::num::NonZeroU64::MIN),
        use_with: None,
        use_with_map_item: Some(MapUseTarget::Object(door)),
        use_with_position: None,
    })
    .map_err(debug)?;
    // The collected copper keys refuse the treasure lock without changing its revision.
    for (reference, offset) in [(copper, 0), (copper, 1), (treasure, 0)] {
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(Duration::from_secs(2))
            .ok_or("clock")?;
        let (next, result, _) = super::item_craft_fixtures::exchange_selected(
            owner,
            *admitted,
            admitted.continuity.next_command_id,
            &payload,
            Some(reference),
            offset,
        )
        .await?;
        *admitted = next;
        if reference == treasure {
            assert_eq!(result, UseDisposition::Committed);
        } else {
            assert_eq!(result, UseDisposition::KeyMismatch);
        }
    }
    Ok(())
}

async fn assert_snapshot(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    expected_keys: &[u16],
    expected: (i64, i64, bool),
) -> TestResult {
    let fence = admitted.item_fence.ok_or("journey item fence")?;
    let state = h
        .root
        .read_character_quest_state(owner.character, fence.character_id)
        .await
        .map_err(debug)?;
    let (stage, entrance, completed) = expected;
    let track = |key| state.tracks().get(key).copied().unwrap_or_default();
    assert_eq!(track(STAGE), stage);
    assert_eq!(track(ENTRANCE), entrance);
    assert_eq!(track(TREASURE), i64::from(completed));
    assert_eq!(
        state.states().get(QUEST).is_some_and(|s| s.completed),
        completed
    );
    let inventory = h
        .root
        .read_item_use_inventory(owner.character, owner.holder, fence)
        .await
        .map_err(debug)?;
    let mut numbers: Vec<_> = inventory.keys.iter().map(|(_, n)| *n).collect();
    numbers.sort_unstable();
    assert_eq!(numbers, expected_keys);

    let rewards: Vec<(String, i64)> = sqlx::query_as(
        "SELECT i.definition_production_key,sum(i.quantity)::bigint
         FROM game_quest_callback_carried(encode($1,'hex')::uuid,encode($2,'hex')::uuid) c
         JOIN game_item_instances i ON i.item_instance_id=c.item
         WHERE i.lifecycle=1 AND i.definition_production_key IN
            ('oteryn:item.tibia.i3039','oteryn:item.tibia.i3295','oteryn:item.tibia.i2853')
         GROUP BY i.definition_production_key ORDER BY i.definition_production_key",
    )
    .bind(fence.character_id.as_bytes().as_slice())
    .bind(owner.world_id.as_bytes().as_slice())
    .fetch_all(&h.pool)
    .await?;
    let wanted = if completed {
        vec![
            ("oteryn:item.tibia.i2853".into(), 1),
            ("oteryn:item.tibia.i3039".into(), 1),
            ("oteryn:item.tibia.i3295".into(), 1),
        ]
    } else {
        Vec::new()
    };
    assert_eq!(rewards, wanted);
    let bag_contents: Vec<(String, i64)> = sqlx::query_as(
        "SELECT child.definition_production_key, child.quantity::bigint
         FROM game_quest_callback_carried(encode($1,'hex')::uuid,encode($2,'hex')::uuid) c
         JOIN game_item_nested_entries entry ON entry.item_instance_id=c.item
         JOIN game_item_instances child USING (item_instance_id)
         JOIN game_item_instances bag ON bag.item_instance_id=entry.parent_item_instance_id
         WHERE child.lifecycle=1 AND bag.definition_production_key='oteryn:item.tibia.i2853'
         ORDER BY child.definition_production_key",
    )
    .bind(fence.character_id.as_bytes().as_slice())
    .bind(owner.world_id.as_bytes().as_slice())
    .fetch_all(&h.pool)
    .await?;
    assert_eq!(bag_contents, if completed { wanted[1..].to_vec() } else { Vec::new() });
    let terminal_count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_character_quest_receipts
         WHERE character_id=encode($1,'hex')::uuid AND transition_key=$2",
    )
    .bind(fence.character_id.as_bytes().as_slice())
    .bind(TERMINAL)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(terminal_count, i64::from(completed));
    Ok(())
}
