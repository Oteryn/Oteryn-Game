use super::fixture::{Harness, debug, id, run};
use super::m39_bone_journey_fixtures::{carried, stage};
use super::quest_world_callbacks_fixtures::{input_item, local, wire};
use super::questpc8_authored_programmes_fixtures::step;
use super::{
    AdmittedSession, ComposedFreshAdmission, Dependencies, TestResult, prepare_current_session,
};
use crate::content::channel_content_pin::ChannelContentPin;
use crate::durability::item_mint::TypedDefinitionRef;
use crate::foundation::{
    ChannelRuntimeV1, MovementFacing, NpcPlacement, NpcPlacementTable, NpcPlacements,
};
use crate::gameplay_transport::connection::FirstEntryOutcome;
use crate::gameplay_transport::quest_log::QuestLogLines;
use crate::map::overlay::TilePos;
use oteryn_gameplay_quests::quest::{
    loader::load_embedded_quest_state, log::loader::load_embedded_quest_log,
    world_callbacks::{Offering, Position},
};
use oteryn_gameplay_world::map::view::MapFacts;
use oteryn_protocol_oteryn::{
    npc_service::{
        NpcIntentDisposition, NpcTalkIntent, decode_npc_intent_result, encode_npc_talk_intent,
    },
    quest_log::{QuestLogQuery, QuestLogView, decode_quest_log, encode_quest_log_query},
    world_object::{
        MapUseTarget, UseDisposition, UseIntent, UseTarget, WorldObjectTarget, decode_use_result,
        encode_item_use_intent, encode_use_intent,
    },
    world_spatial::StepDirection,
    world_spatial_entities::EntityRef,
};
use std::{
    path::Path,
    sync::Arc,
    time::{Duration, Instant},
};

const QUEST: &str = "oteryn:quest.an_interest_in_botany_quest";
const NPC: &str = "oteryn:npc.rabaz";
const STAGE: &str = "oteryn:quest-progress/an-interest-in-botany/stage";
const DOOR_TRACK: &str = "oteryn:quest-progress/quest/u8_6/an_interest_in_botany/chest_door";
const CHEST_CLAIM: &str = "oteryn:reward-claim.quest.u8_6.an_interest_in_botany.botany_almanach";
const FINISH: &str = "oteryn:quest-reward/an-interest-in-botany/s4";
const KNIFE: u32 = 5908;
const FIRE_BUG: u32 = 5467;
const ALMANACH: u32 = 11699;
const SEDGE_HAT: u32 = 11700;
const GOLD_COIN: u32 = 3031;
const START: Position = Position {
    x: 33009,
    y: 31534,
    floor: -10,
};
const DOOR: Position = Position {
    x: 33007,
    y: 31536,
    floor: -10,
};
const CHEST: Position = Position {
    x: 33004,
    y: 31530,
    floor: -10,
};
const DREADCOIL: Position = Position {
    x: 33132,
    y: 31396,
    floor: -8,
};
const WEB: Position = Position {
    x: 33209,
    y: 31376,
    floor: -8,
};
const VERMINOUS: Position = Position {
    x: 33257,
    y: 31325,
    floor: -8,
};

#[test]
fn an_interest_in_botany_full_journey() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    let started = Instant::now();
    super::kill_reward_e2e_fixtures::scenario_thread::run("an-interest-in-botany", || journey(false))?;
    println!(
        "POSTGRES_CASE_COMPLETED an_interest_in_botany_full_journey duration={:?}",
        started.elapsed()
    );
    Ok(())
}

#[test]
fn botany_sampling_accepts_a_web_already_burned_by_another_player() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run("botany-open-web", || journey(true))
}

fn journey(web_already_burned: bool) -> TestResult {
    run(async move |admin| {
        let lowered = load_embedded_quest_state("content-1").map_err(debug)?;
        let journal = load_embedded_quest_log(&lowered)?;
        let catalogue = Arc::new(lowered.catalogue().clone());
        assert!(
            catalogue.authored_bindings().is_some_and(|bindings| {
                bindings.quests.iter().any(|quest| quest.quest == QUEST)
            }),
            "Botany is missing its embedded native journey"
        );

        let h = Harness::create_with_admission_seed(admin, "botany_journey", false, false).await?;
        let result: TestResult = async {
            prepare_current_session(&h).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
            let mut deps = Dependencies::new()?;
            let (controller, activated_pin) =
                super::death_ml_loss_composed_fixtures::production_generation_with_pin(
                    deps.admitted.world_id, "content-1",
                )?;
            let generation = controller.active().ok_or("active generation")?;
            let native = generation.native_gameplay().ok_or("native generation")?;
            let (world, placement) = retained_world(native)?;
            let claims = crate::content::world_reward_claims::bind_embedded_reward_claims(
                world.base().as_ref(), &|_| true,
            )?;
            assert!(claims.served.iter().any(|claim| claim.key.as_str() == CHEST_CLAIM));

            let cells = world.movement_cells(deps.room.movement_cells()).map_err(debug)?;
            let start = local(START);
            let pin = ChannelContentPin::from_activation(
                deps.admitted.world_id,
                1,
                generation.identity().server_artifact_digest(),
                generation.identity().client_artifact_digest(),
                activated_pin.frame_binding().digest(),
                world.map_revision_digest(),
                (start.x, start.y, start.floor),
            );
            let mut runtime = ChannelRuntimeV1::from_committed_assignment(
                deps.admitted.world_id,
                deps.admitted.channel_id,
                crate::foundation::NodeId::decode(&id(1)).map_err(debug)?,
                1, 1, 1, "runtime-scope-assignment:1", 2, pin,
            ).map_err(debug)?;
            let reservation = runtime.reserve_fresh_session(deps.admitted.game_session_id).map_err(debug)?;
            deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
            runtime.initialize_pinned_test_position(deps.actor, start).map_err(debug)?;
            runtime.place_npcs(&NpcPlacementTable {
                npcs: vec![NpcPlacements { key: NPC.into(), placements: vec![placement] }],
            }, |key| key == NPC, true).map_err(debug)?;
            let rabaz = runtime.visible_entities().npcs.into_iter()
                .find(|npc| runtime.npc_key(npc.actor) == Some(NPC)).ok_or("live Rabaz")?;
            let npc = EntityRef { identity: rabaz.actor.placement_identity(), generation: rabaz.generation };
            deps.runtime = tokio::sync::Mutex::new(runtime);

            let current = crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                .current_session_at(deps.admitted.game_session_id).await?.0;
            let fence = crate::gameplay_transport::item_fence_of(current).ok_or("item fence")?;
            sqlx::query("INSERT INTO game_control_scope_grants VALUES (session_user, encode($1,'hex')::uuid, encode($2,'hex')::uuid, 4)")
                .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
            sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid, encode($2,'hex')::uuid, 1, NULL, $3, $4, $5)")
                .bind(id(42).as_slice()).bind(id(43).as_slice())
                .bind(generation.identity().server_artifact_digest().as_slice())
                .bind(generation.identity().client_artifact_digest().as_slice())
                .bind(activated_pin.frame_binding().digest().as_slice()).execute(&h.pool).await?;

            let fresh = super::fresh_fixture::fresh::Source::new(0)?.request()?;
            h.root.initialize_admission_equipment(&authority, &h.node, &fence,
                generation.identity().server_artifact_digest(), fresh.operation()).await.map_err(debug)?;
            let starter = crate::durability::starter_grant::StarterTemplate {
                definition: TypedDefinitionRef { family: "Item".into(),
                    production_key: "oteryn:item.tibia.i2854".into(), revision_ref: "definition-r1".into() },
                ..crate::durability::starter_grant::StarterTemplate::main()
            };
            h.root.grant_starter_items(&authority, &h.node, &fence,
                &crate::gameplay_transport::item_ref_admission::NativeItemWeights(native),
                &starter).await.map_err(debug)?;

            let model = crate::content::NpcDataCatalogue::load(
                &repository().join("content"), std::ffi::OsStr::new("world"),
                crate::content::npc_catalogue_preproduction_limits(),
            ).map_err(debug)?.service_model_for_project().map_err(debug)?;
            let talk_book = Arc::new(crate::gameplay_transport::npc_talk::NpcTalkBook::from_model_with_native_quests(&model, &[NPC]));
            let mut admitted = deps.admitted;
            admitted.runtime_actor = Some(deps.actor);
            admitted.first_entry = FirstEntryOutcome::Positioned;
            admitted.item_fence = Some(fence);
            let mut owner = deps.owner(&h, &authority);
            owner.active_generation = Some(generation);
            owner.movement_cells = (&cells).into();
            owner.quest_catalogue = Some(Arc::clone(&catalogue));
            let log_lines = QuestLogLines::new(journal.content).ok_or("Botany journal line bounds")?;
            owner.quest_log_content = Some(log_lines.clone());
            deps.initialize(&owner).await;
            admit(&mut owner, &mut admitted, Arc::clone(&talk_book)).await?;
            for (index, tool) in [KNIFE, FIRE_BUG].into_iter().enumerate() {
                input_item(&owner, fence, &Offering {
                    position: START, item: format!("oteryn:item.tibia.i{tool}"), count: 1,
                }, 600 + u64::try_from(index)?, true).await?;
            }
            let initial_log = log(&owner, &mut admitted).await?;
            exercise(&mut owner, &mut admitted, &h, npc, web_already_burned).await?;

            // A new admission owner rebuilds the quest and NPC projections for the reconnect.
            // The wire helper has already closed the first transport and retained its continuity.
            drop(owner);
            let mut reconnected = deps.owner(&h, &authority);
            reconnected.active_generation = Some(generation);
            reconnected.movement_cells = (&cells).into();
            reconnected.quest_catalogue = Some(catalogue);
            reconnected.quest_log_content = Some(log_lines);
            deps.initialize(&reconnected).await;
            admit(&mut reconnected, &mut admitted, talk_book).await?;
            assert_eq!(speak(&mut reconnected, &mut admitted, npc, "hi").await?, NpcIntentDisposition::Ok);
            speak(&mut reconnected, &mut admitted, npc, "mission").await?;
            assert_eq!(speak(&mut reconnected, &mut admitted, npc, "yes").await?, NpcIntentDisposition::Rejected);
            checkpoint(&h, Some(5), 0, 1, 1000).await?;
            assert_eq!(finish_receipts(&h).await?, 1);
            assert_eq!(log(&reconnected, &mut admitted).await?, initial_log, "the completed quest stays hidden");
            Ok(())
        }.await;
        h.cleanup().await?;
        result
    })
}

fn repository() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..")
}

fn retained_world(
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> TestResult<(crate::map::boot::BundleWorld, NpcPlacement)> {
    use oteryn_world_bundle_compiler::{bundle, compile, source};
    let root = repository();
    let project = source::project_regions(&root, &[(128, 123), (129, 122)])?;
    let (registry, palette) = source::registry(&root, &project)?;
    let identity: bundle::Identity = serde_json::from_slice(&std::fs::read(
        root.join("content/world/pins/oteryn.identity.json"),
    )?)?;
    let mut pins = crate::map::BundlePins {
        digest: [0; 32],
        project_format_version: identity.project_format_version.clone(),
        world_schema_version: identity.world_schema_version.clone(),
        content_revision: identity.content_revision.clone(),
        production: false,
    };
    let compiled = compile::compile(
        &compile::Input {
            regions: &project.regions,
            palette: &palette,
            identity,
            world: source::world_extent(&root)?,
            build_class: bundle::BuildClass::NonProduction,
            draft_areas: oteryn_world_bundle_compiler::project::draft_areas(&project.index)?,
            families: &project.families,
        },
        &registry,
    )?;
    pins.digest = compiled.digest;
    let bundle = oteryn_world_bundle::bundle::read(&compiled.bytes)?;
    let rabaz = bundle
        .npcs
        .npcs
        .iter()
        .find(|row| row.key == NPC)
        .ok_or("Rabaz must be in the compiled World")?;
    let [source] = rabaz.placements.as_slice() else {
        return Err("one Rabaz placement required".into());
    };
    assert_eq!((source.x, source.y, source.floor), (33009, 31532, -10));
    assert_eq!(
        source.direction,
        oteryn_world_bundle_compiler::npc::Direction::North
    );
    let placement = NpcPlacement {
        x: source.x,
        y: source.y,
        floor: source.floor,
        direction: MovementFacing::North,
    };
    let base = crate::map::load(&compiled.bytes, &pins).map_err(debug)?;
    let world = crate::map::boot::boot(
        &compiled.bytes,
        &crate::map::boot::BootPins {
            map_revision: crate::map::overlay::map_revision(&base),
            bundle: pins,
            start: tile(START),
        },
        crate::foundation::WorldId::decode(&id(42)).map_err(debug)?,
        crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?,
        |key| crate::map::boot::native_item_definition(Some(native), key),
    )
    .map_err(debug)?;
    Ok((world, placement))
}

async fn admit(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    talk_book: Arc<crate::gameplay_transport::npc_talk::NpcTalkBook>,
) -> TestResult {
    owner.admit_progression_session(admitted).await;
    owner.admit_quest_session(admitted).await;
    owner.admit_premium(id(40), admitted.game_session_id);
    owner.states.chat.install_npc_talk(talk_book);
    owner.states.chat.admit(
        admitted.game_session_id,
        "Botany witness",
        owner.owner_now(),
    );
    owner.states.chat.set_npc_service(admitted.game_session_id);
    super::qd_world_fixtures::select_talk_capabilities(owner, admitted, &[3, 4, 6, 11, 15, 16])
}

async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    npc: EntityRef,
    web_already_burned: bool,
) -> TestResult {
    checkpoint(h, None, 0, 0, 0).await?;
    assert_eq!(stage(h, DOOR_TRACK).await?, None);
    for word in ["hi", "mission"] {
        assert_eq!(
            speak(owner, admitted, npc, word).await?,
            NpcIntentDisposition::Ok
        );
        checkpoint(h, None, 0, 0, 0).await?;
    }
    assert_eq!(
        speak(owner, admitted, npc, "yes").await?,
        NpcIntentDisposition::Ok
    );
    checkpoint(h, Some(1), 0, 0, 0).await?;
    assert_eq!(stage(h, DOOR_TRACK).await?, Some(0));

    walk(
        owner,
        admitted,
        &[StepDirection::South, StepDirection::South],
    )
    .await?;
    assert_eq!(
        use_object(owner, admitted, DOOR, 1, None).await?,
        UseDisposition::Committed
    );
    checkpoint(h, Some(1), 0, 0, 0).await?;
    walk(
        owner,
        admitted,
        &[
            StepDirection::West,
            StepDirection::West,
            StepDirection::West,
            StepDirection::North,
            StepDirection::North,
            StepDirection::North,
            StepDirection::North,
            StepDirection::North,
        ],
    )
    .await?;
    checkpoint(h, Some(1), 0, 0, 0).await?;
    assert_eq!(
        use_object(owner, admitted, CHEST, 1, None).await?,
        UseDisposition::Committed
    );
    checkpoint(h, Some(1), 1, 0, 0).await?;

    arrive(
        owner,
        admitted,
        Position {
            x: 33133,
            ..DREADCOIL
        },
    )
    .await?;
    assert_eq!(
        use_object(owner, admitted, DREADCOIL, 1, Some(KNIFE)).await?,
        UseDisposition::Committed
    );
    checkpoint(h, Some(2), 1, 0, 0).await?;

    arrive(owner, admitted, Position { y: 31377, ..WEB }).await?;
    if web_already_burned {
        let map = owner.bundle_map().ok_or("Botany map")?;
        let key = crate::map::view::placement_key(tile(WEB), 4).ok_or("web placement")?;
        assert!(map.facts.transform_movement_item(key, 188));
        checkpoint(h, Some(2), 1, 0, 0).await?;
    } else {
        let before = target(owner, WEB, 4)?.expected_revision;
        assert_eq!(
            use_object(owner, admitted, WEB, 4, Some(FIRE_BUG)).await?,
            UseDisposition::Committed
        );
        assert!(
            target(owner, WEB, 4)?.expected_revision > before,
            "the real web changed on the map"
        );
        checkpoint(h, Some(3), 1, 0, 0).await?;
    }

    arrive(
        owner,
        admitted,
        Position {
            y: 31326,
            ..VERMINOUS
        },
    )
    .await?;
    assert_eq!(
        use_object(owner, admitted, VERMINOUS, 1, Some(KNIFE)).await?,
        UseDisposition::Committed
    );
    checkpoint(h, Some(4), 1, 0, 0).await?;

    arrive(owner, admitted, START).await?;
    for word in ["hi", "mission"] {
        assert_eq!(
            speak(owner, admitted, npc, word).await?,
            NpcIntentDisposition::Ok
        );
        checkpoint(h, Some(4), 1, 0, 0).await?;
    }
    let command = admitted.continuity.next_command_id;
    let confirmation = speech(npc, "yes")?;
    let reward = wire(owner, admitted, command, 7, confirmation.clone()).await?;
    assert_eq!(
        decode_npc_intent_result(&reward).map_err(debug)?,
        NpcIntentDisposition::Ok
    );
    checkpoint(h, Some(5), 0, 1, 1000).await?;
    assert_eq!(
        wire(owner, admitted, command, 7, confirmation).await?,
        reward
    );
    checkpoint(h, Some(5), 0, 1, 1000).await?;
    assert_eq!(
        finish_receipts(h).await?,
        1,
        "terminal replay pays only once"
    );
    Ok(())
}

fn tile(pos: Position) -> TilePos {
    TilePos {
        x: pos.x,
        y: pos.y,
        floor: pos.floor,
    }
}

fn target(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    pos: Position,
    ordinal: u8,
) -> TestResult<WorldObjectTarget> {
    let map = owner.bundle_map().ok_or("Botany map")?;
    let key = crate::map::view::placement_key(tile(pos), ordinal).ok_or("Botany placement")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    Ok(WorldObjectTarget {
        placement,
        expected_revision: map.facts.object_revision(key),
    })
}

async fn use_object(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    pos: Position,
    ordinal: u8,
    tool: Option<u32>,
) -> TestResult<UseDisposition> {
    owner.clock_origin -= Duration::from_secs(2);
    let target = target(owner, pos, ordinal)?;
    let payload = if let Some(tool) = tool {
        let item = owner
            .active_generation
            .and_then(|g| g.native_gameplay())
            .and_then(|n| n.item_index())
            .and_then(|index| {
                index.definition_ref(
                    "Item",
                    &format!("oteryn:item.tibia.i{tool}"),
                    "definition-r1",
                )
            })
            .ok_or("native tool definition")?;
        encode_item_use_intent(&UseIntent {
            target: UseTarget::ItemByDefinition(item),
            use_with: None,
            use_with_map_item: Some(MapUseTarget::Object(target)),
            use_with_position: None,
        })
        .map_err(debug)?
    } else {
        encode_use_intent(&target).map_err(debug)?
    };
    let command = admitted.continuity.next_command_id;
    decode_use_result(&wire(owner, admitted, command, 2, payload).await?).map_err(debug)
}

async fn walk(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    steps: &[StepDirection],
) -> TestResult {
    for direction in steps {
        let command = admitted.continuity.next_command_id;
        step(owner, admitted, command, *direction).await?;
    }
    Ok(())
}

// Position the fixture between geographically separate quest sites. There is no
// quest, inventory, or callback write here; the storage-room route is stepped.
async fn arrive(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    pos: Position,
) -> TestResult {
    let map = owner.bundle_map().ok_or("Botany map")?;
    let doors = crate::map::floor::BundleDoors(map.facts.doors());
    let actor = admitted.runtime_actor.ok_or("Botany actor")?;
    let mut runtime = owner.runtime.lock().await;
    let previous = runtime.read_actor_position(actor).map_err(debug)?;
    let proof = crate::map::floor::prepare_quest_callback_teleport(
        &runtime,
        &owner.movement_cells,
        actor,
        admitted.game_session_id,
        previous,
        local(pos),
        false,
        &doors,
    )
    .map_err(debug)?;
    runtime.commit_source_step(proof).map_err(debug)?;
    Ok(())
}

fn speech(npc_actor: EntityRef, text: &str) -> TestResult<Vec<u8>> {
    encode_npc_talk_intent(&NpcTalkIntent {
        npc_actor,
        text: text.into(),
    })
    .map_err(debug)
}

async fn speak(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    npc: EntityRef,
    text: &str,
) -> TestResult<NpcIntentDisposition> {
    owner.clock_origin -= Duration::from_secs(2);
    let command = admitted.continuity.next_command_id;
    decode_npc_intent_result(&wire(owner, admitted, command, 7, speech(npc, text)?).await?)
        .map_err(debug)
}

async fn log(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
) -> TestResult<oteryn_protocol_oteryn::quest_log::QuestLog> {
    let command = admitted.continuity.next_command_id;
    let query = encode_quest_log_query(&QuestLogQuery::ListOnly).map_err(debug)?;
    let reply = wire(owner, admitted, command, 22, query).await?;
    let log = decode_quest_log(&reply).map_err(debug)?;
    if !matches!(&log.view, Some(QuestLogView::List(_))) {
        return Err("Botany journal list response".into());
    }
    Ok(log)
}

async fn checkpoint(
    h: &Harness,
    progress: Option<i64>,
    book: i64,
    hat: i64,
    gold: i64,
) -> TestResult {
    assert_eq!(stage(h, STAGE).await?, progress, "Botany stage");
    for (item, count) in [
        (ALMANACH, book),
        (SEDGE_HAT, hat),
        (GOLD_COIN, gold),
        (KNIFE, 1),
        (FIRE_BUG, 1),
        (3035, 0),
    ] {
        assert_eq!(
            carried(h, item).await?,
            count,
            "carried item {item} at stage {progress:?}"
        );
    }
    let complete: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_character_quest_states WHERE quest_key=$1 AND completed_character_revision IS NOT NULL)")
        .bind(QUEST).fetch_one(&h.pool).await?;
    assert_eq!(complete, progress == Some(5), "durable completion");
    Ok(())
}

async fn finish_receipts(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT count(*) FROM game_character_quest_receipts WHERE transition_key=$1",
    )
    .bind(FINISH)
    .fetch_one(&h.pool)
    .await?)
}
