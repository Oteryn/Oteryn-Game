//! Guarded paid NPC exchanges through the real dispatcher and PostgreSQL owner.
//! This explicit engine fixture does not qualify the complete Lumberjack quest.
use super::fixture::{Harness, debug, id, run};
use super::{
    AdmittedSession, ComposedFreshAdmission, Dependencies, TestResult, prepare_current_session,
};
use crate::content::channel_content_pin::ChannelContentPin;
use crate::foundation::{
    ChannelRuntimeV1, MovementFacing, NpcPlacement, NpcPlacementTable, NpcPlacements,
};
use crate::gameplay_transport::connection::{FirstEntryOutcome, FreshAdmissionAuthority};
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_gameplay_quests::quest::{
    QuestStateCatalogue,
    authored_bindings::Bindings,
    world_callbacks::{Callback, Offering, Position},
};
use oteryn_protocol_oteryn::{
    npc_service::NpcIntentDisposition, world_spatial::StepDirection,
    world_spatial_entities::EntityRef,
};
use sha2::{Digest, Sha256};
use std::sync::Arc;

const CHIP: &str = "oteryn:npc.chip";
const QUEST: &str = "oteryn:quest.fixture.guarded_wood_exchange";
const ENTRY: &str = "oteryn:quest-callback/worldchg-m31/fixture-guarded-sale/entry";
const WORK: &str = "oteryn:quest-reward/worldchg-m31/fixture-guarded-sale/work";
const PHASE: &str = "oteryn:quest-progress/fixture/guarded-wood-exchange/phase";
const TRADE: &str = "oteryn:quest-reward/worldchg-m31/fixture-guarded-sale/wood";
const WOOD: &str = "oteryn:item.tibia.i5901";
const GOLD: &str = "oteryn:item.tibia.i3031";

fn fixture() -> TestResult<(Callback, QuestStateCatalogue)> {
    let source: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../../content/quests/world-change-journals.json"
    ))?;
    let p = source["programs"]
        .as_array()
        .ok_or("programmes")?
        .iter()
        .find(|p| p["key"] == "lumberjack")
        .ok_or("lumberjack")?;
    // Keep the source mechanics, but do not borrow the held canonical quest's identity.
    // The World callback prefix retains the existing journal-source admission contract.
    let mut entry = p["world_callbacks"][0].clone();
    entry["key"] = ENTRY.into();
    entry["quest"] = QUEST.into();
    entry["storage"][0]["track"] = PHASE.into();
    let mut work = p["npc_callbacks"][0].clone();
    work["key"] = WORK.into();
    work["quest"] = QUEST.into();
    work["storage"][0]["track"] = PHASE.into();
    let mut barter = work.clone();
    barter["key"] = TRADE.into();
    barter["keywords"] = serde_json::json!(["sell wood"]);
    barter["repeatable"] = true.into();
    barter["storage"][0] =
        serde_json::json!({"track":PHASE,"minimum":2,"maximum":4,"set":2,"preserve":true});
    barter["take"] = serde_json::json!([{"item":WOOD,"count":1}]);
    barter["give"] = serde_json::json!({"item":GOLD,"count":50,"contents":[]});
    let stages = serde_json::json!([
        {"key":"s1","completes":false,"trigger":{"kind":"step_in","callback":entry}},
        {"key":"s2","completes":false,"trigger":{"kind":"npc_dialogue","callback":work}},
        {"key":"s3","completes":false,"trigger":{"kind":"npc_dialogue","callback":barter}}
    ]);
    let digest = Sha256::digest(serde_json::to_vec(&stages)?)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    let [phase] = p["tracks"].as_array().ok_or("tracks")?.as_slice() else {
        return Err("guarded exchange fixture requires one source phase track".into());
    };
    let tracks = serde_json::json!([{
        "key":PHASE,"initial":phase["initial"],"minimum":phase["min"],"maximum":phase["max"]
    }]);
    let document = serde_json::json!({"schema":1,"quests":[{
        "quest":QUEST,"recipe_sha256":digest,"tracks":tracks,"stages":stages,
        "sources":["M31 guarded exchange engine fixture; not canonical quest qualification",
            "https://tibiopedia.pl/npcs/Chip","https://tibiopedia.pl/items/Wood"]
    }]});
    let bindings = Bindings::parse(&serde_json::to_vec(&document)?).map_err(debug)?;
    let base = QuestStateCatalogue::empty("content-1").ok_or("content revision")?;
    Ok((
        serde_json::from_value(entry)?,
        bindings.attach(&base).map_err(debug)?,
    ))
}

async fn install_chip(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
) -> TestResult<EntityRef> {
    let catalogue = crate::content::NpcDataCatalogue::load(
        std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content")),
        std::ffi::OsStr::new("world"),
        crate::content::npc_catalogue_preproduction_limits(),
    )
    .map_err(debug)?;
    let model = catalogue.service_model_for_project().map_err(debug)?;
    owner.states.chat.install_npc_talk(Arc::new(
        crate::gameplay_transport::npc_talk::NpcTalkBook::from_model_with_native_quests(
            &model,
            &[CHIP],
        ),
    ));
    owner
        .states
        .chat
        .admit(admitted.game_session_id, "Wood trader", owner.owner_now());
    owner.states.chat.set_npc_service(admitted.game_session_id);
    let mut capabilities = admitted
        .continuity
        .selected_capabilities
        .as_slice()
        .to_vec();
    capabilities.push(oteryn_protocol_oteryn::npc_service::CAPABILITY_NPC_SERVICE_V1);
    capabilities.sort_unstable();
    capabilities.dedup();
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(
            owner.offered_capabilities(),
            &capabilities,
        )
        .ok_or("NPC capabilities")?;
    let placements: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../../content/world/npc-placements/placements-00000-00055.json"
    ))?;
    let row = placements["records"]
        .as_array()
        .ok_or("placements")?
        .iter()
        .map(|r| &r["declaration"])
        .find(|r| r["npc"] == CHIP)
        .ok_or("Chip source placement")?;
    assert_eq!(
        row["cell"],
        serde_json::json!({"x":32418,"y":31702,"floor":7})
    );
    assert_eq!(row["direction"], "north");
    let mut runtime = owner.runtime.lock().await;
    let placed = runtime
        .place_npcs(
            &NpcPlacementTable {
                npcs: vec![NpcPlacements {
                    key: CHIP.into(),
                    placements: vec![NpcPlacement {
                        x: 32418,
                        y: 31702,
                        floor: -7,
                        direction: MovementFacing::North,
                    }],
                }],
            },
            |key| model.npcs().keys().any(|r| r.key == key),
            true,
        )
        .map_err(debug)?;
    let actor = *placed.actors.first().ok_or("Chip actor")?;
    let npc = runtime
        .visible_entities()
        .npcs
        .into_iter()
        .find(|n| n.actor == actor)
        .ok_or("Chip generation")?;
    Ok(EntityRef {
        identity: actor.placement_identity(),
        generation: npc.generation,
    })
}

async fn quantity(h: &Harness, item: &str) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT coalesce(sum(quantity),0)::bigint FROM game_item_instances WHERE lifecycle=1 AND definition_production_key=$1")
        .bind(item).fetch_one(&h.pool).await?)
}

#[test]
fn engworldchg_guarded_wood_exchange_real_dispatch_postgres() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e::scenario_thread::run("engworldchg-guarded-sale", move || {
        let (entry_callback, catalogue) = fixture()?;
        let entry = entry_callback.selector.position.ok_or("entry")?;
        let world = super::engworldchg_journals::scene(&entry_callback, &entry_callback)?;
        run(async move |admin| {
            let h = Harness::create_with_admission_seed(
                admin,
                "engworldchg_guarded_sale",
                false,
                false,
            )
            .await?;
            prepare_current_session(&h).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let mut deps = Dependencies::new()?;
            let (controller, activated) =
                super::death_ml_loss_composed::production_generation_with_pin(
                    deps.admitted.world_id,
                    "content-1",
                )?;
            let generation = controller.active().ok_or("generation")?;
            let cells = world
                .movement_cells(activated.qualified_room().movement_cells())
                .map_err(debug)?;
            let origin = super::quest_world_callbacks::local(
                entry_callback
                    .effects
                    .encounter
                    .as_ref()
                    .filter(|_| {
                        entry_callback.trigger
                            == oteryn_gameplay_quests::quest::world_callbacks::Trigger::Use
                    })
                    .and_then(|encounter| encounter.players.first())
                    .map_or(
                        Position {
                            x: entry.x + 1,
                            ..entry
                        },
                        |member| member.from,
                    ),
            );
            let room = super::rust_lottery::bundle_action_room(
                activated.qualified_room(),
                &world,
                deps.admitted.world_id,
                &cells,
            )?;
            let frame = room.frame_binding().digest();
            let pin = ChannelContentPin::from_activation(
                deps.admitted.world_id,
                1,
                generation.identity().server_artifact_digest(),
                generation.identity().client_artifact_digest(),
                frame,
                room.map_revision_digest(),
                (origin.x, origin.y, origin.floor),
            );
            let mut runtime = ChannelRuntimeV1::from_committed_assignment(
                deps.admitted.world_id,
                deps.admitted.channel_id,
                crate::foundation::NodeId::decode(&id(1)).map_err(debug)?,
                1,
                1,
                1,
                "runtime-scope-assignment:1",
                64,
                pin,
            )
            .map_err(debug)?;
            let reservation = runtime
                .reserve_fresh_session(deps.admitted.game_session_id)
                .map_err(debug)?;
            deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
            runtime
                .initialize_pinned_test_position(deps.actor, origin)
                .map_err(debug)?;
            deps.runtime = tokio::sync::Mutex::new(runtime);
            deps.combat_room = Some(room);
            let mut owner = deps.owner(&h, &authority);
            owner.active_generation = Some(generation);
            owner.movement_cells = (&cells).into();
            owner.quest_catalogue = Some(Arc::new(catalogue));
            let mut admitted = deps.admitted;
            admitted.first_entry = FirstEntryOutcome::Positioned;
            let fence = owner
                .current_quest_fence(admitted.game_session_id)
                .await
                .map_err(|()| "fence")?
                .ok_or("fence")?;
            admitted.item_fence = Some(fence.item_fence());
            owner
                .runtime
                .lock()
                .await
                .bind_attacker_lease(
                    deps.actor,
                    admitted.game_session_id,
                    crate::foundation::CharacterLease::new(
                        crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                            .map_err(debug)?,
                        fence.character_lease_generation,
                    )
                    .map_err(debug)?,
                )
                .map_err(debug)?;

            sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
            sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
            .bind(id(42).as_slice()).bind(id(43).as_slice())
            .bind(generation.identity().server_artifact_digest().as_slice())
            .bind(generation.identity().client_artifact_digest().as_slice())
            .bind(frame.as_slice()).execute(&h.pool).await?;
            let fresh = super::fresh_fixture::fresh::Source::new(0)?.request()?;
            h.root
                .initialize_admission_equipment(
                    &authority,
                    &h.node,
                    &fence.item_fence(),
                    generation.identity().server_artifact_digest(),
                    fresh.operation(),
                )
                .await
                .map_err(debug)?;
            let native = generation.native_gameplay().ok_or("native")?;
            h.root
                .grant_starter_items(
                    &authority,
                    &h.node,
                    &fence.item_fence(),
                    &crate::gameplay_transport::item_ref_admission::NativeItemWeights(native),
                    &crate::durability::starter_grant::StarterTemplate {
                        definition: crate::durability::item_mint::TypedDefinitionRef {
                            family: "Item".into(),
                            production_key: "oteryn:item.tibia.i2854".into(),
                            revision_ref: "definition-r1".into(),
                        },
                        ..crate::durability::starter_grant::StarterTemplate::main()
                    },
                )
                .await
                .map_err(debug)?;
            owner.admit_progression_session(&mut admitted).await;
            deps.initialize(&owner).await;
            owner.admit_quest_session(&admitted).await;
            native
                .install_companion_policies(&mut *owner.runtime.lock().await)
                .map_err(debug)?;
            let npc = install_chip(&owner, &mut admitted).await?;
            let start_gold = quantity(&h, GOLD).await?;
            assert_eq!(quantity(&h, WOOD).await?, 0);
            let fence = owner
                .current_quest_fence(admitted.game_session_id)
                .await
                .map_err(|()| "quest fence")?
                .ok_or("quest fence")?;
            super::quest_world_callbacks::input_item(
                &owner,
                fence.item_fence(),
                &Offering {
                    position: Position {
                        x: entry.x + 1,
                        ..entry
                    },
                    item: WOOD.into(),
                    count: 2,
                },
                30100,
                true,
            )
            .await?;
            assert_eq!(quantity(&h, WOOD).await?, 2);
            assert_eq!(
                super::qengine_rewards::talk(&mut owner, &mut admitted, npc, 31000, "hi").await?,
                NpcIntentDisposition::Ok
            );
            assert_eq!(
                super::qengine_rewards::talk(&mut owner, &mut admitted, npc, 31001, "sell wood")
                    .await?,
                NpcIntentDisposition::Rejected
            );
            assert_eq!(
                quantity(&h, WOOD).await?,
                2,
                "knowledge gate cannot consume inputs"
            );
            assert_eq!(quantity(&h, GOLD).await?, start_gold);
            super::engworldchg_journals::step(&owner, &mut admitted, 1, StepDirection::West)
                .await?;
            assert_eq!(super::engworldchg_journals::value(&h, PHASE).await?, 1);
            super::engworldchg_journals::step(&owner, &mut admitted, 2, StepDirection::East)
                .await?;
            owner.open_npc_session(admitted.game_session_id).await;
            assert_eq!(
                super::qengine_rewards::talk(&mut owner, &mut admitted, npc, 31002, "hi").await?,
                NpcIntentDisposition::Ok
            );
            assert_eq!(
                super::qengine_rewards::talk(&mut owner, &mut admitted, npc, 31003, "work").await?,
                NpcIntentDisposition::Ok
            );
            assert_eq!(super::engworldchg_journals::value(&h, PHASE).await?, 2);
            assert_eq!(
                super::qengine_rewards::talk(&mut owner, &mut admitted, npc, 31004, "sell wood")
                    .await?,
                NpcIntentDisposition::Ok
            );
            assert_eq!(quantity(&h, WOOD).await?, 1);
            assert_eq!(quantity(&h, GOLD).await?, start_gold + 50);
            assert_eq!(
                super::qengine_rewards::talk(&mut owner, &mut admitted, npc, 31004, "sell wood")
                    .await?,
                NpcIntentDisposition::Ok
            );
            assert_eq!(
                quantity(&h, WOOD).await?,
                1,
                "replay cannot consume a second input"
            );
            assert_eq!(
                quantity(&h, GOLD).await?,
                start_gold + 50,
                "replay cannot mint a second payout"
            );
            assert_eq!(
                super::qengine_rewards::talk(&mut owner, &mut admitted, npc, 31005, "sell wood")
                    .await?,
                NpcIntentDisposition::Ok
            );
            assert_eq!(quantity(&h, WOOD).await?, 0);
            assert_eq!(quantity(&h, GOLD).await?, start_gold + 100);
            assert_eq!(
                super::qengine_rewards::talk(&mut owner, &mut admitted, npc, 31005, "sell wood")
                    .await?,
                NpcIntentDisposition::Ok,
                "a committed sale replays after all inputs were consumed"
            );
            assert_eq!(quantity(&h, WOOD).await?, 0);
            assert_eq!(quantity(&h, GOLD).await?, start_gold + 100);
            assert_eq!(
                super::qengine_rewards::talk(&mut owner, &mut admitted, npc, 31006, "sell wood")
                    .await?,
                NpcIntentDisposition::Rejected
            );
            assert_eq!(
                quantity(&h, GOLD).await?,
                start_gold + 100,
                "missing inputs cannot mint"
            );
            assert_eq!(
                super::engworldchg_journals::value(&h, PHASE).await?,
                2,
                "sales must retain investigation progress"
            );
            let receipts: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM game_character_quest_receipts WHERE transition_key=$1",
            )
            .bind(TRADE)
            .fetch_one(&h.pool)
            .await?;
            assert_eq!(receipts, 2, "one receipt for each distinct committed sale");
            let consumed:i64=sqlx::query_scalar("SELECT coalesce(sum(quantity),0)::bigint FROM game_quest_callback_consumptions WHERE transition_key=$1").bind(TRADE).fetch_one(&h.pool).await?;
            assert_eq!(consumed, 2);
            let mints:i64=sqlx::query_scalar("SELECT count(*) FROM game_reward_claim_mint_receipts WHERE claim_production_key LIKE 'oteryn:quest-exchange/%'").fetch_one(&h.pool).await?;
            assert_eq!(mints, 2);
            let fence = owner
                .current_quest_fence(admitted.game_session_id)
                .await
                .map_err(|()| "quest fence")?
                .ok_or("quest fence")?;
            let cold = h
                .root
                .read_character_quest_state(&authority, fence.character_id)
                .await
                .map_err(debug)?;
            assert_eq!(cold.tracks().get(PHASE), Some(&2));
            assert!(started.elapsed() > std::time::Duration::from_millis(10));
            eprintln!(
                "ENGWORLDCHG_GUARDED_SALE_PG_EXECUTED duration={:?}",
                started.elapsed()
            );
            drop(owner);
            drop(authority);
            drop(seal);
            h.cleanup().await
        })
    })
}
