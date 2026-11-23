//! Crystal 201189f8: Eroth, the tower scrying ball, and source chests 6253–6256.
use super::fixture::{debug, id};
use super::{AdmittedSession, ComposedFreshAdmission, Dependencies, TestResult};
use crate::content::NativeEntryStart;
use crate::content::channel_content_pin::ChannelContentPin;
use crate::foundation::{
    ChannelRuntimeV1, MovementFacing, NodeId, NpcPlacement, NpcPlacementTable, NpcPlacements,
};
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{
    ControllerBinding, FirstEntryOutcome, FreshAdmissionAuthority, SessionContinuity,
};
use crate::map::overlay::TilePos;
use crate::map::view::MapFacts;
use crate::movement::CardinalStep;
use oteryn_gameplay_quests::quest::{
    loader, log,
    world_callbacks::{Offering, Position},
};
use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTalkIntent};
use oteryn_protocol_oteryn::quest_log::{self, QuestLogQuery, QuestLogView};
use oteryn_protocol_oteryn::world_object::{
    self, MapUseTarget, UseDisposition, UseIntent, UseTarget, WorldObjectTarget,
};
use oteryn_protocol_oteryn::world_spatial::StepDirection;
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use std::collections::{BTreeMap, VecDeque};
use std::sync::Arc;

const QUEST: &str = "oteryn:quest.to_blind_the_enemy_quest";
const PHASE: &str = "oteryn:quest-progress/to-blind-the-enemy/mission";
const NPC: &str = "oteryn:npc.eroth";
const MIRROR: u16 = 3463;
const PLATINUM: u16 = 3035;
const AMULET: u16 = 3082;
const TOOLS: [u16; 3] = [3003, 3457, 3148];
const START: Position = at(32664, 31682, 6);
const TOWER: Position = at(32593, 31644, 2);
const BALL: Position = at(32592, 31645, 2);

const fn at(x: u16, y: u16, z: i8) -> Position {
    Position { x, y, floor: -z }
}

#[test]
fn to_blind_the_enemy_full_journey_earns_eroth_reward() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e_fixtures::scenario_thread::run_async(async move |_| {
        scenario(admin).await
    });
    eprintln!(
        "POSTGRES_CASE_COMPLETED to_blind_the_enemy_full_journey_earns_eroth_reward duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

async fn scenario(admin: String) -> TestResult {
    let h =
        crate::bestiary_postgres_harness::Harness::create(admin, "blind_the_enemy", false).await?;
    let mut deps = Dependencies::new()?;
    let (controller, pin) = super::death_ml_loss_composed_fixtures::production_generation_with_pin(
        deps.admitted.world_id,
        "content-1",
    )?;
    let generation = controller.active().ok_or("active production generation")?;
    let native = generation.native_gameplay().ok_or("native gameplay")?;
    let lowered = loader::load_embedded_quest_state("content-1").map_err(debug)?;
    assert!(
        lowered
            .catalogue()
            .authored_bindings()
            .is_some_and(|bindings| bindings.quests.iter().any(|quest| quest.quest == QUEST)),
        "Eroth's production mission must be registered"
    );
    let journal = log::loader::load_embedded_quest_log(&lowered)?;
    let quest_index = u32::try_from(
        journal
            .content
            .active()
            .quests()
            .iter()
            .position(|entry| entry.key == QUEST)
            .ok_or("quest journal")?,
    )?;

    let (world, npcs) = super::annihilator_route_fixtures::source_world(native, START, false)?;
    let npc = npcs
        .npcs
        .iter()
        .find(|npc| npc.key == NPC)
        .ok_or("compiled Eroth placement")?;
    let [placement] = npc.placements.as_slice() else {
        return Err("expected exactly one compiled Eroth placement".into());
    };
    assert_eq!(
        (placement.x, placement.y, placement.floor),
        (32662, 31682, -6)
    );
    assert_eq!(
        placement.direction,
        oteryn_world_bundle::npc::Direction::North
    );

    let cells = world
        .movement_cells(pin.qualified_room().movement_cells())
        .map_err(debug)?;
    let frame = world.frame_binding_digest(deps.admitted.world_id);
    let room = pin
        .qualified_room()
        .bundle_view(
            cells.clone(),
            frame,
            world.map_revision_digest(),
            NativeEntryStart {
                x: START.x.into(),
                y: START.y.into(),
                floor: i16::from(-START.floor),
            },
        )
        .ok_or("source journey room")?;
    deps.combat_room = Some(room);
    sqlx::query("INSERT INTO game_control_scope_grants VALUES (session_user, encode($1,'hex')::uuid, encode($2,'hex')::uuid, 4)")
        .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid, encode($2,'hex')::uuid, 1, NULL, $3, $4, $5)")
        .bind(id(42).as_slice()).bind(id(43).as_slice())
        .bind(generation.identity().server_artifact_digest().as_slice())
        .bind(generation.identity().client_artifact_digest().as_slice())
        .bind(frame.as_slice()).execute(&h.pool).await?;
    let now = sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
        .fetch_one(&h.pool)
        .await?;
    let (current, operation) =
        crate::world_party_runtime_postgres_cases::committed_fresh_player_with_operation(
            &crate::world_party_runtime_postgres_cases::FreshPlayerOwners {
                root: h.root.clone(),
                node: h.node.clone(),
                recovery: &h.recovery,
            },
            60,
            80,
            70,
            now,
        )
        .await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let mut runtime = ChannelRuntimeV1::from_committed_assignment(
        deps.admitted.world_id,
        deps.admitted.channel_id,
        NodeId::decode(&id(1)).map_err(debug)?,
        1,
        1,
        1,
        "runtime-scope-assignment:1",
        16,
        ChannelContentPin::from_activation(
            deps.admitted.world_id,
            1,
            generation.identity().server_artifact_digest(),
            generation.identity().client_artifact_digest(),
            frame,
            world.map_revision_digest(),
            (START.x.into(), START.y.into(), i16::from(-START.floor)),
        ),
    )
    .map_err(debug)?;
    let session = current.current_game_session_id();
    let reserved = runtime.reserve_fresh_session(session).map_err(debug)?;
    let actor = runtime.commit_fresh_session(reserved).map_err(debug)?;
    runtime
        .bind_attacker_lease(actor, session, current.current_character_lease())
        .map_err(debug)?;
    runtime
        .initialize_pinned_test_position(actor, super::quest_world_callbacks_fixtures::local(START))
        .map_err(debug)?;
    runtime
        .place_npcs(
            &NpcPlacementTable {
                npcs: vec![NpcPlacements {
                    key: NPC.into(),
                    placements: vec![NpcPlacement {
                        x: placement.x,
                        y: placement.y,
                        floor: placement.floor,
                        direction: MovementFacing::North,
                    }],
                }],
            },
            |_| true,
            true,
        )
        .map_err(debug)?;
    let npc = runtime
        .visible_entities()
        .npcs
        .into_iter()
        .find(|npc| runtime.npc_key(npc.actor) == Some(NPC))
        .map(|npc| EntityRef {
            identity: npc.actor.placement_identity(),
            generation: npc.generation,
        })
        .ok_or("live Eroth")?;
    deps.actor = actor;
    deps.runtime = tokio::sync::Mutex::new(runtime);
    let mut member = AdmittedSession {
        game_session_id: session,
        world_id: deps.admitted.world_id,
        channel_id: deps.admitted.channel_id,
        runtime_actor: Some(actor),
        first_entry: FirstEntryOutcome::Positioned,
        controller: Some(ControllerBinding {
            transport: current.current_transport().ok_or("fresh transport")?,
            account_id: id(80),
        }),
        continuity: SessionContinuity::FRESH,
        item_fence: None,
    };
    deps.admitted = member;
    let mut owner = deps.owner_from(&h.root, &h.node, &authority);
    owner.active_generation = Some(generation);
    owner.test_step_wait_receipts = Some(true);
    owner.movement_cells = (&cells).into();
    owner.quest_catalogue = Some(Arc::new(lowered.catalogue().clone()));
    owner.quest_log_content = Some(
        crate::gameplay_transport::quest_log::QuestLogLines::new(journal.content)
            .ok_or("journal line bounds")?,
    );
    owner.states.chat.install_npc_talk(Arc::new(
        crate::gameplay_transport::npc_talk::NpcTalkBook::from_model_with_native_quests(
            &crate::content::NpcDataCatalogue::load(
                std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content")),
                std::ffi::OsStr::new("world"),
                crate::content::npc_catalogue_preproduction_limits(),
            )
            .map_err(debug)?
            .service_model_for_project()
            .map_err(debug)?,
            &oteryn_gameplay_quests::quest::authored_bindings::native_dialogue_npcs()?
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>(),
        ),
    ));
    owner
        .states
        .chat
        .admit(session, "Quest witness", owner.owner_now());
    owner.states.chat.set_npc_service(session);
    let offered = crate::gameplay_transport::appearance_change::offer::offered(
        owner.offered_capabilities(),
        owner.appearance_available(),
    );
    let mut pending = vec![3, 11, 13, 15, 16];
    let mut selected = std::collections::BTreeSet::new();
    while let Some(id) = pending.pop() {
        if selected.insert(id)
            && let Some(capability) = offered.iter().find(|capability| capability.id == id)
        {
            pending.extend_from_slice(capability.requires);
        }
    }
    member.continuity.selected_capabilities =
        SelectedCapabilities::select(&offered, &selected.into_iter().collect::<Vec<_>>())
            .ok_or("quest capabilities")?;
    owner.admit_progression_session(&mut member).await;
    let fence = owner
        .current_quest_fence(session)
        .await
        .map_err(debug)?
        .ok_or("fresh quest fence")?;
    member.item_fence = Some(fence.item_fence());
    owner
        .root
        .initialize_admission_equipment(
            owner.character,
            owner.holder,
            &fence.item_fence(),
            native.source_digest(),
            &operation,
        )
        .await
        .map_err(debug)?;
    owner
        .root
        .grant_starter_items(
            owner.character,
            owner.holder,
            &fence.item_fence(),
            &crate::gameplay_transport::item_ref_admission::NativeItemWeights(native),
            &crate::durability::starter_grant::StarterTemplate::main(),
        )
        .await
        .map_err(debug)?;
    owner.admit_quest_session(&member).await;
    for (sequence, tool) in TOOLS.into_iter().enumerate() {
        super::quest_world_callbacks_fixtures::input_item(
            &owner,
            fence.item_fence(),
            &Offering {
                position: START,
                item: item_key(tool),
                count: 1,
            },
            53000 + u64::try_from(sequence)?,
            true,
        )
        .await?;
    }
    journey(
        &mut owner,
        &mut member,
        &h.pool,
        *fence.character_id.as_bytes(),
        npc,
        quest_index,
    )
    .await
}

fn item_key(item: u16) -> String {
    format!("oteryn:item.tibia.i{item}")
}

async fn checkpoint(
    pool: &sqlx::PgPool,
    character: [u8; 16],
    phase: i64,
    expected: &BTreeMap<u16, i64>,
) -> TestResult {
    let actual: Option<i64> = sqlx::query_scalar(
        "SELECT value FROM game_character_quest_tracks WHERE quest_key=$1 AND track_key=$2 AND character_id=encode($3,'hex')::uuid",
    )
    .bind(QUEST)
    .bind(PHASE)
    .bind(character.as_slice())
    .fetch_optional(pool)
    .await?;
    assert_eq!(actual.unwrap_or(0), phase);
    let completed: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_character_quest_receipts WHERE quest_key=$1 AND completes AND character_id=encode($2,'hex')::uuid",
    )
    .bind(QUEST)
    .bind(character.as_slice())
    .fetch_one(pool)
    .await?;
    assert_eq!(completed, i64::from(phase == 4));
    let actual: Vec<(String, i64)> = sqlx::query_as(
        "WITH RECURSIVE owned(item_instance_id) AS (
             SELECT item_instance_id FROM game_item_container_entries WHERE character_id=encode($1,'hex')::uuid
             UNION
             SELECT e.item_instance_id FROM game_item_nested_entries e
             JOIN owned ON e.parent_item_instance_id=owned.item_instance_id
         ) SELECT definition_production_key, sum(quantity)::bigint FROM owned JOIN game_item_instances USING (item_instance_id)
         WHERE lifecycle=1 AND definition_production_key=ANY($2::text[]) GROUP BY definition_production_key",
    ).bind(character.as_slice()).bind(expected.keys().map(|item| item_key(*item)).collect::<Vec<_>>())
        .fetch_all(pool).await?;
    let actual = actual.into_iter().collect::<BTreeMap<_, _>>();
    for (&item, &quantity) in expected {
        assert_eq!(
            actual.get(&item_key(item)).copied().unwrap_or(0),
            quantity,
            "item {item} at mission phase {phase}"
        );
    }
    Ok(())
}

async fn journey(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    member: &mut AdmittedSession,
    pool: &sqlx::PgPool,
    character: [u8; 16],
    npc: EntityRef,
    quest_index: u32,
) -> TestResult {
    use super::annihilator_route_fixtures::use_target;
    use super::qengine_rewards_fixtures::wire_conversation;
    use super::quest_world_callbacks_fixtures::wire;
    use NpcIntentDisposition::{Ok as Spoken, Rejected};
    let callbacks = oteryn_gameplay_quests::quest::reward_callbacks::load_for(
        owner.quest_catalogue.as_ref().ok_or("quest catalogue")?,
    )
    .map_err(debug)?;
    let callback = |key| {
        callbacks
            .iter()
            .find(|callback| callback.key == key)
            .ok_or("Eroth callback")
    };
    let offer = callback("oteryn:quest-reward/to-blind-the-enemy/offer")?;
    let accept = callback("oteryn:quest-reward/to-blind-the-enemy/accept")?;
    let report = callback("oteryn:quest-reward/to-blind-the-enemy/report")?;
    let mut bag = BTreeMap::from([
        (MIRROR, 0),
        (PLATINUM, 0),
        (AMULET, 0),
        (TOOLS[0], 1),
        (TOOLS[1], 1),
        (TOOLS[2], 1),
        (2853, 0),
        (3425, 0),
        (3282, 0),
        (237, 0),
        (3147, 0),
        (3059, 0),
        (3028, 0),
        (3031, 0),
    ]);
    checkpoint(pool, character, 0, &bag).await?;
    assert_eq!(
        wire_conversation(owner, member, npc, offer, false, vec!["hi", "mission"])
            .await?
            .0,
        Spoken
    );
    checkpoint(pool, character, 0, &bag).await?;
    assert_eq!(
        wire_conversation(
            owner,
            member,
            npc,
            accept,
            false,
            vec!["hi", "mission", "yes"]
        )
        .await?
        .0,
        Spoken
    );
    bag.insert(MIRROR, 1);
    checkpoint(pool, character, 1, &bag).await?;
    journal(owner, member, quest_index, false, "Take Eroth's mirror").await?;
    assert_eq!(
        wire_conversation(owner, member, npc, report, false, vec!["hi", "mission"])
            .await?
            .0,
        Rejected
    );
    checkpoint(pool, character, 1, &bag).await?;

    // The ladder is a native USE target. The other floor changes resolve
    // through ordinary paced STEP on the compiled source region.
    walk(owner, member, at(32593, 31644, 5)).await?;
    assert_eq!(
        use_target(owner, member, &target(owner, at(32593, 31643, 5), 1)?).await?,
        UseDisposition::Committed
    );
    checkpoint(pool, character, 1, &bag).await?;
    walk(owner, member, TOWER).await?;
    checkpoint(pool, character, 2, &bag).await?;
    journal(owner, member, quest_index, false, "Use the mirror").await?;
    let ball = target(owner, BALL, 2)?;
    assert_ne!(
        use_target(owner, member, &ball).await?,
        UseDisposition::Committed
    );
    checkpoint(pool, character, 2, &bag).await?;
    assert_eq!(
        tool_use(owner, member, TOOLS[1], ball.clone()).await?,
        UseDisposition::NothingToUse
    );
    checkpoint(pool, character, 2, &bag).await?;
    let key = crate::map::view::placement_key(
        TilePos { x: BALL.x, y: BALL.y, floor: BALL.floor }, 2,
    ).ok_or("scrying ball placement")?;
    owner.states.quest_callbacks.interrupt_after_commit
        .store(true, std::sync::atomic::Ordering::Release);
    assert!(tool_use(owner, member, MIRROR, ball).await.is_err());
    assert_eq!(owner.states.quest_callbacks.reserved_target_for_test(member.game_session_id), Some(key));
    assert!(!owner.bundle_map().ok_or("World")?.facts.base_hidden(key));
    owner.clock_origin -= std::time::Duration::from_secs(61);
    assert!(owner.reconcile_quest_callbacks(member.game_session_id).await);
    let published = owner.owner_now().get();
    let map = owner.bundle_map().ok_or("World")?;
    assert!(map.facts.base_hidden(key));
    map.facts.expire_movement_items(published + 59_000_000);
    assert!(map.facts.base_hidden(key), "reconciliation starts a fresh reset window");
    map.facts.expire_movement_items(published + 60_000_000);
    assert!(!map.facts.base_hidden(key));
    bag.insert(MIRROR, 0);
    checkpoint(pool, character, 3, &bag).await?;
    journal(
        owner,
        member,
        quest_index,
        false,
        "Recover the four tower treasures",
    )
    .await?;

    for (chest, approach, additions) in [
        (
            at(32588, 31645, 3),
            at(32589, 31645, 3),
            &[(2853, 1), (237, 1), (3147, 1), (3059, 1)][..],
        ),
        (
            at(32588, 31644, 3),
            at(32589, 31644, 3),
            &[(2853, 1), (3028, 2), (3031, 100)][..],
        ),
        (at(32591, 31647, 3), at(32591, 31646, 3), &[(3425, 1)][..]),
        (at(32590, 31647, 3), at(32590, 31646, 3), &[(3282, 1)][..]),
    ] {
        walk(owner, member, approach).await?;
        assert_eq!(
            use_target(owner, member, &target(owner, chest, 1)?).await?,
            UseDisposition::Committed
        );
        for &(item, count) in additions {
            *bag.entry(item).or_default() += count;
        }
        checkpoint(pool, character, 3, &bag).await?;
    }
    walk(owner, member, START).await?;
    let (outcome, _, command) =
        wire_conversation(owner, member, npc, report, false, vec!["hi", "mission"]).await?;
    assert_eq!(outcome, Spoken);
    bag.insert(PLATINUM, 10);
    bag.insert(AMULET, 1);
    checkpoint(pool, character, 4, &bag).await?;
    journal(owner, member, quest_index, true, "received Eroth's reward").await?;

    // Each wire helper starts a new duplex connection with retained continuity.
    let payload = oteryn_protocol_oteryn::npc_service::encode_npc_talk_intent(&NpcTalkIntent {
        npc_actor: npc,
        text: "mission".into(),
    })
    .map_err(debug)?;
    let reply = wire(owner, member, command, 7, payload).await?;
    assert_eq!(
        oteryn_protocol_oteryn::npc_service::decode_npc_intent_result(&reply).map_err(debug)?,
        Spoken
    );
    checkpoint(pool, character, 4, &bag).await?;
    assert_eq!(
        wire_conversation(owner, member, npc, report, false, vec!["hi", "mission"])
            .await?
            .0,
        Rejected
    );
    checkpoint(pool, character, 4, &bag).await?;
    journal(owner, member, quest_index, true, "received Eroth's reward").await
}

fn target(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    position: Position,
    ordinal: u8,
) -> TestResult<WorldObjectTarget> {
    let map = owner.bundle_map().ok_or("source map")?;
    let key = crate::map::view::placement_key(
        TilePos {
            x: position.x,
            y: position.y,
            floor: position.floor,
        },
        ordinal,
    )
    .ok_or("source object placement")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    Ok(WorldObjectTarget {
        placement,
        expected_revision: map.facts.object_revision(key),
    })
}

async fn tool_use(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    member: &mut AdmittedSession,
    item: u16,
    target: WorldObjectTarget,
) -> TestResult<UseDisposition> {
    let index = owner
        .active_generation
        .and_then(|generation| generation.native_gameplay())
        .and_then(|native| native.item_index())
        .ok_or("active item index")?;
    let definition = index
        .definition_ref("Item", &item_key(item), "definition-r1")
        .ok_or("selected tool")?;
    member.item_fence = Some(
        owner
            .current_quest_fence(member.game_session_id)
            .await
            .map_err(debug)?
            .ok_or("tool fence")?
            .item_fence(),
    );
    let command = member.continuity.next_command_id;
    let reply = super::quest_world_callbacks_fixtures::wire(
        owner,
        member,
        command,
        world_object::COMMAND_TYPE_USE_INTENT,
        world_object::encode_item_use_intent(&UseIntent {
            target: UseTarget::ItemByDefinition(definition),
            use_with: None,
            use_with_map_item: Some(MapUseTarget::Object(target)),
            use_with_position: None,
        })
        .map_err(debug)?,
    )
    .await?;
    world_object::decode_use_result(&reply).map_err(debug)
}

async fn journal(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    member: &mut AdmittedSession,
    quest_index: u32,
    completed: bool,
    text: &str,
) -> TestResult {
    use crate::gameplay_transport::connection::{ConnectionEnd, IDLE_LIVENESS, serve_admitted};
    use oteryn_protocol_oteryn::{MessageType, decode_state_delta, decode_wire_envelope};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let payload =
        quest_log::encode_quest_log_query(&QuestLogQuery::Quest { quest_index }).map_err(debug)?;
    let frame = oteryn_protocol_oteryn::encode_client_command(
        member.continuity.connection_generation,
        &oteryn_protocol_oteryn::ClientCommandValue {
            command_id: member.continuity.next_command_id,
            command_type: quest_log::COMMAND_TYPE_QUEST_LOG_QUERY,
            payload: &payload,
        },
    )
    .map_err(debug)?;
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let peer = async {
        client
            .write_all(&u32::try_from(frame.len())?.to_be_bytes())
            .await?;
        client.write_all(&frame).await?;
        client.shutdown().await?;
        let mut output = Vec::new();
        client.read_to_end(&mut output).await?;
        Ok::<_, Box<dyn std::error::Error>>(output)
    };
    let serving = async {
        let result = serve_admitted(&mut server, *member, owner, IDLE_LIVENESS).await;
        drop(server);
        result
    };
    let (end, output) = tokio::join!(serving, peer);
    let ConnectionEnd::AdmittedThenDisconnected(next) = end else {
        return Err(format!("LOG connection ended {end:?}").into());
    };
    *member = next;
    let output = output?;
    let mut reader = output.as_slice();
    let mut found = false;
    while !reader.is_empty() {
        let frame = crate::gameplay_transport::tcp_tls::read_frame(&mut reader).await?;
        let envelope = decode_wire_envelope(&frame).map_err(debug)?;
        if envelope.message_type() == MessageType::StateDelta {
            let delta = decode_state_delta(envelope.payload()).map_err(debug)?;
            if delta.domain_id == quest_log::STATE_DOMAIN_QUEST_LOG
                && let Some(QuestLogView::Line(line)) = quest_log::decode_quest_log(delta.payload)
                    .map_err(debug)?
                    .view
            {
                assert_eq!((line.quest_index, line.completed), (quest_index, completed));
                let [mission] = line.missions.as_slice() else {
                    return Err("expected one mission line".into());
                };
                assert_eq!(mission.done, completed);
                assert!(
                    mission.text.contains(text),
                    "mission text was {:?}",
                    mission.text
                );
                found = true;
            }
        }
    }
    assert!(found, "quest line arrived through protocol LOG");
    Ok(())
}

async fn walk(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    member: &mut AdmittedSession,
    goal: Position,
) -> TestResult {
    use crate::content::{CollisionClass, LogicalCell};
    use crate::map::floor::{Ground, resolve_landing};
    let at = owner
        .runtime
        .lock()
        .await
        .read_actor_position(member.runtime_actor.ok_or("actor")?)
        .map_err(debug)?
        .position();
    let start = (at.x, at.y, i32::from(at.floor));
    let end = (i32::from(goal.x), i32::from(goal.y), -i32::from(goal.floor));
    let map = owner.bundle_map().ok_or("source map")?;
    let cells = &owner.movement_cells;
    let table = crate::map::floor::table_of(cells).ok_or("source floors")?;
    let ground = |(x, y, z): (i32, i32, i32)| {
        if let (Ok(x), Ok(y), Ok(floor)) = (u16::try_from(x), u16::try_from(y), i8::try_from(-z))
            && map.facts.doors().blocks(TilePos { x, y, floor })
        {
            return Ground::Blocked;
        }
        match cells.index().lookup(cells.scope(), LogicalCell { x, y, z }) {
            Ok(CollisionClass::Walkable) => Ground::Walkable,
            Ok(CollisionClass::Blocked) => Ground::Blocked,
            Err(_) => Ground::Absent,
        }
    };
    let directions = [
        (CardinalStep::North, StepDirection::North),
        (CardinalStep::East, StepDirection::East),
        (CardinalStep::South, StepDirection::South),
        (CardinalStep::West, StepDirection::West),
    ];
    let mut pending = VecDeque::from([start]);
    let mut previous = BTreeMap::new();
    while let Some(cell) = pending.pop_front() {
        if cell == end {
            break;
        }
        for (direction, command) in directions {
            if let Ok(next) = resolve_landing(table, ground, |_| true, cell, direction)
                && next != start
                && let std::collections::btree_map::Entry::Vacant(entry) = previous.entry(next)
            {
                entry.insert((cell, command));
                pending.push_back(next);
            }
        }
    }
    let mut path = Vec::new();
    let mut cell = end;
    while cell != start {
        let (prior, direction) = previous
            .get(&cell)
            .copied()
            .ok_or_else(|| format!("no source walk from {start:?} to {end:?}"))?;
        path.push(direction);
        cell = prior;
    }
    for direction in path.into_iter().rev() {
        super::desert_earned_route_fixtures::step(owner, member, direction).await?;
    }
    Ok(())
}
