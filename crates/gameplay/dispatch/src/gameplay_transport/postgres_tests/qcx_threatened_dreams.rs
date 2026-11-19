//! Global dialogue and Fairy Treasure through the fenced NPC and world-object routes.
use super::classic_quests_fixtures::{install, progress, talk};
use super::fixture::{Harness, debug, id, run};
use super::{
    AdmittedSession, ComposedFreshAdmission, Dependencies, TestResult, prepare_current_session,
};
use crate::foundation::{ChannelContentPin, ChannelRuntimeV1};
use crate::gameplay_transport::connection::{FirstEntryOutcome, UseCommand};
use oteryn_gameplay_quests::quest::{threatened_dreams as quest, world_callbacks::Position};
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};

const MOSAIC: Position = Position {
    x: 32363,
    y: 32416,
    floor: -7,
};
const STONE: &str = "oteryn:quest-progress/quest/u11_40/threatened_dreams/mission04/grumpy_stone";

#[test]
fn global_dialogue_and_fairy_treasure() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e_fixtures::scenario_thread::run("qcx-threatened-dreams", scenario);
    eprintln!(
        "PG qcx_threatened_dreams {:?}, passed={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

fn scenario() -> TestResult {
    let trigger = MOSAIC;
    // Use the exact source target independently of whether the quest registers it.
    let mut callback = oteryn_gameplay_quests::quest::world_callbacks::load()?
        .into_iter()
        .find(|callback| callback.key.ends_with("behemoth"))
        .ok_or("terrain fixture")?;
    callback.selector.position = Some(trigger);
    callback.selector.appearance = Some(25380);
    callback.selector.unique = None;
    callback.selector.action = None;
    let world = super::quest_world_callbacks_fixtures::world(&callback, trigger)?;
    let local = super::quest_world_callbacks_fixtures::local;
    run(async |admin| {
        let h = Harness::create_with_admission_seed(admin, "quest_world_callbacks", false, false)
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
            super::death_ml_loss_composed_fixtures::production_generation_with_pin(
                deps.admitted.world_id,
                "content-1",
            )?;
        let generation = controller.active().ok_or("generation")?;
        let cells = world
            .movement_cells(activated.qualified_room().movement_cells())
            .map_err(debug)?;
        let pin = ChannelContentPin::from_activation(
            deps.admitted.world_id,
            1,
            generation.identity().server_artifact_digest(),
            generation.identity().client_artifact_digest(),
            activated.frame_binding().digest(),
            world.map_revision_digest(),
            (
                i32::from(trigger.x) - 1,
                i32::from(trigger.y),
                -i16::from(trigger.floor),
            ),
        );
        let room = activated
            .qualified_room()
            .bundle_view(
                cells.clone(),
                activated.frame_binding().digest(),
                world.map_revision_digest(),
                crate::content::NativeEntryStart {
                    x: i32::from(trigger.x) - 1,
                    y: i32::from(trigger.y),
                    floor: -i16::from(trigger.floor),
                },
            )
            .ok_or("mosaic bundle room")?;
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            deps.admitted.world_id,
            deps.admitted.channel_id,
            crate::foundation::NodeId::decode(&id(1)).map_err(debug)?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            8,
            pin,
        )
        .map_err(debug)?;
        let reservation = runtime
            .reserve_fresh_session(deps.admitted.game_session_id)
            .map_err(debug)?;
        deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        let origin = local(Position {
            x: trigger.x - 1,
            ..trigger
        });
        runtime
            .initialize_pinned_test_position(deps.actor, origin)
            .map_err(debug)?;
        deps.runtime = tokio::sync::Mutex::new(runtime);
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(generation);
        owner.movement_cells = (&cells).into();
        owner.rooms = crate::gameplay_transport::SeamRooms::new(None, Some(&room));
        owner.quest_catalogue = Some(std::sync::Arc::new(
            oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
                .map_err(debug)?
                .catalogue()
                .clone(),
        ));
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        let fence = owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(|()| "fence")?
            .ok_or("fence")?;
        admitted.item_fence = Some(fence.item_fence());
        sqlx::query("INSERT INTO game_control_scope_grants VALUES (session_user, encode($1,'hex')::uuid, encode($2,'hex')::uuid, 4)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid, encode($2,'hex')::uuid, 1, NULL, $3, $4, $5)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).bind(generation.identity().server_artifact_digest().as_slice())
            .bind(generation.identity().client_artifact_digest().as_slice()).bind(activated.frame_binding().digest().as_slice()).execute(&h.pool).await?;
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
        let template = crate::durability::starter_grant::StarterTemplate {
            definition: crate::durability::item_mint::TypedDefinitionRef {
                family: "Item".into(),
                production_key: "oteryn:item.tibia.i2854".into(),
                revision_ref: "definition-r1".into(),
            },
            ..crate::durability::starter_grant::StarterTemplate::main()
        };
        h.root
            .grant_starter_items(
                &authority,
                &h.node,
                &fence.item_fence(),
                &crate::gameplay_transport::item_ref_admission::NativeItemWeights(native),
                &template,
            )
            .await
            .map_err(debug)?;
        owner.admit_progression_session(&mut admitted).await;
        deps.initialize(&owner).await;
        owner.admit_quest_session(&admitted).await;

        let fairy_key = "oteryn:npc.dancing_fairy";
        let wolf_key = "oteryn:npc.ghostly_wolf";
        let tree_key = "oteryn:npc.tired_tree";
        let [alkestios, aurita, fairy, wolf, tree] = install(
            &owner,
            &admitted,
            [
                quest::ALKESTIOS,
                "oteryn:npc.aurita",
                fairy_key,
                wolf_key,
                tree_key,
            ],
        )
        .await?;

        let early = use_mosaic(&mut owner, &admitted, 2199).await?;
        talk(&mut owner, &admitted, alkestios, 2200, "hi").await?;
        let offer = talk(&mut owner, &admitted, alkestios, 2201, "mission").await?;
        talk(&mut owner, &admitted, alkestios, 2202, "yes").await?;
        let animals = progress(&owner, &admitted, &authority, quest::ANIMALS).await?;

        talk(&mut owner, &admitted, aurita, 2203, "hi").await?;
        talk(&mut owner, &admitted, aurita, 2204, "mission").await?;
        talk(&mut owner, &admitted, aurita, 2205, "yes").await?;
        talk(&mut owner, &admitted, fairy, 2206, "hi").await?;
        let instructions = talk(&mut owner, &admitted, fairy, 2207, "fishtail").await?;
        let couple = progress(&owner, &admitted, &authority,
            "oteryn:quest-progress/crystalserver/quest/u11_40/threatened_dreams/mission03/unlikely_couple").await?;

        // Resume at the item-dependent prerequisites so only the NPC
        // commands below can commit the transitions under test.
        let tree_track =
            "oteryn:quest-progress/quest/u11_40/threatened_dreams/mission04/tired_tree";
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx)
            .await?;
        for (track, value) in [
            (quest::ANIMALS, 9_i64),
            (
                "oteryn:quest-progress/quest/u11_40/threatened_dreams/mission04/tooth_fairy",
                2,
            ),
            (STONE, 4),
        ] {
            sqlx::query("INSERT INTO game_character_quest_tracks(character_id,track_key,quest_key,value,committed_character_revision) SELECT character_id,$2,$3,$4,character_revision FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid ON CONFLICT(character_id,track_key) DO UPDATE SET value=EXCLUDED.value,committed_character_revision=EXCLUDED.committed_character_revision")
                .bind(id(41).as_slice())
                .bind(track)
                .bind("oteryn:quest.threatened_dreams_quest")
                .bind(value)
                .execute(&mut *tx).await?;
        }
        tx.commit().await?;
        owner
            .refresh_quest_session(admitted.game_session_id, false)
            .await;

        talk(&mut owner, &admitted, wolf, 2208, "hi").await?;
        let peace = talk(&mut owner, &admitted, wolf, 2209, "whelp").await?;
        let wolf_stage = progress(&owner, &admitted, &authority, quest::ANIMALS).await?;

        talk(&mut owner, &admitted, tree, 2210, "hi").await?;
        let map = talk(&mut owner, &admitted, tree, 2211, "map").await?;
        talk(&mut owner, &admitted, tree, 2212, "yes").await?;
        let tree_stage = progress(&owner, &admitted, &authority, tree_track).await?;

        let claimed = use_mosaic(&mut owner, &admitted, 2214).await?;
        let repeated = use_mosaic(&mut owner, &admitted, 2215).await?;
        let treasure: Vec<(String, i64)> = sqlx::query_as("SELECT definition_production_key,sum(quantity)::bigint FROM game_item_instances WHERE lifecycle=1 AND definition_production_key IN ('oteryn:item.tibia.i24390','oteryn:item.tibia.i25698','oteryn:item.tibia.i25737') GROUP BY definition_production_key ORDER BY definition_production_key")
            .fetch_all(&h.pool).await?;

        assert_ne!(
            early,
            UseDisposition::Committed,
            "the map must be assembled"
        );
        assert_eq!(
            (
                claimed,
                repeated,
                progress(&owner, &admitted, &authority, STONE).await?,
                [animals, couple, wolf_stage, tree_stage],
            ),
            (
                UseDisposition::Committed,
                UseDisposition::Rejected,
                5,
                [1, 2, 10, 1]
            ),
            "Global dialogue advances each route and the mosaic pays exactly once",
        );
        assert_eq!(
            treasure,
            vec![
                ("oteryn:item.tibia.i24390".into(), 5),
                ("oteryn:item.tibia.i25698".into(), 1),
                ("oteryn:item.tibia.i25737".into(), 5),
            ]
        );
        assert_eq!(
            (
                offer.as_slice(),
                instructions.as_slice(),
                peace.as_slice(),
                map.as_slice()
            ),
            (
                quest::reply(quest::ALKESTIOS, 80).ok_or("offer")?,
                quest::reply(fairy_key, 58).ok_or("instructions")?,
                quest::reply(wolf_key, 74).ok_or("peace")?,
                quest::reply(tree_key, 66).ok_or("map offer")?,
            ),
        );
        Ok(())
    })
}

async fn use_mosaic(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    command_id: u64,
) -> TestResult<UseDisposition> {
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let map = owner.bundle_map().ok_or("mosaic map")?;
    let key = oteryn_world_bundle::bundle::placement_key(MOSAIC.floor, MOSAIC.x, MOSAIC.y, 1)
        .ok_or("mosaic key")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    Ok(owner
        .use_world_object(
            admitted.runtime_actor.ok_or("actor")?,
            UseCommand {
                game_session_id: admitted.game_session_id,
                command_id,
                item_fence: Some(fence.item_fence()),
            },
            WorldObjectTarget {
                placement,
                expected_revision: 0,
            },
        )
        .await
        .disposition)
}
