use oteryn_gameplay_world::map::view::MapFacts;
use super::fixture::{Harness, debug, id, run};

use super::{Dependencies, TestResult, prepare_current_session};

use crate::content::channel_content_pin::ChannelContentPin;

use crate::foundation::ChannelRuntimeV1;

use crate::gameplay_transport::connection::FirstEntryOutcome;

use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Position};

use oteryn_protocol_oteryn::world_spatial::{
    COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT, StepDirection, encode_step_intent,
};

pub(super) const ENTRY: &str = "oteryn:quest-callback/grimvale-full-moon-entry";

pub(super) const TRIGGER: Position = Position {
    x: 33430,
    y: 31537,
    floor: -11,
};

pub(super) const DESTINATION: Position = Position {
    x: 33419,
    y: 31539,
    floor: -10,
};

pub(super) fn scenario_with_bindings(prove_bindings: bool) -> TestResult {
    // Fixture state is the real encounter's already-open source portal. This does not
    // assert that a full moon fires Feroxa or fabricate the donor's five-percent draw.
    let callback: Callback = serde_json::from_value(serde_json::json!({
        "key": ENTRY, "quest": "oteryn:quest.grimvale_quest", "trigger": "step_in",
        "selector": {"position":TRIGGER,"unique":null,"action":null,"appearance":22761},
        "lever":null,"reversible":false,"passages":[],"teleport":DESTINATION,
        "storage":[],"offerings":[],"item_on_target":null,"return_on_failure":true,
        "failure_message":null,
        "sources":["crystalserver:9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d:data-global/lib/quests/grimvale.lua:34-45"]
    }))?;
    let world = super::quest_world_callbacks_fixtures::world(&callback, TRIGGER)?;
    run(async |admin| {
        let start = std::time::Instant::now();
        let h =
            Harness::create_with_admission_seed(admin, "worldchg_grimvale", false, false).await?;
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
        let origin = super::quest_world_callbacks_fixtures::local(Position {
            x: TRIGGER.x - 1,
            ..TRIGGER
        });
        deps.combat_room = Some(
            activated
                .qualified_room()
                .bundle_view(
                    cells.clone(),
                    activated.frame_binding().digest(),
                    world.map_revision_digest(),
                    crate::content::NativeEntryStart {
                        x: origin.x,
                        y: origin.y,
                        floor: origin.floor,
                    },
                )
                .ok_or("qualified Grimvale bundle room")?,
        );
        let pin = ChannelContentPin::from_activation(
            deps.admitted.world_id,
            1,
            generation.identity().server_artifact_digest(),
            generation.identity().client_artifact_digest(),
            activated.frame_binding().digest(),
            world.map_revision_digest(),
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
            2,
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
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(generation);
        owner.movement_cells = (&cells).into();
        owner.quest_catalogue = Some(std::sync::Arc::new(
            oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
                .map_err(debug)?
                .catalogue()
                .clone(),
        ));
        assert!(
            owner
                .states
                .quest_callbacks
                .fixture
                .set(vec![callback.clone()])
                .is_ok()
        );
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
            .bind(activated.frame_binding().digest().as_slice()).execute(&h.pool).await?;
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
        let world_id = admitted.world_id;
        for (epoch, day) in [(0_i64, 12_i32), (1, 13), (2, 13)] {
            sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,$2,$3,'map-1',$3,$3,$3,'TIBIA_SERVER_SAVE_BERLIN_06',1,make_timestamptz(2031,6,$4,12,0,0,'UTC'),'2031-06')")
                .bind(world_id.as_bytes().as_slice()).bind(epoch).bind([1_u8;32].as_slice()).bind(day).execute(&h.pool).await?;
        }
        sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,0)")
            .bind(world_id.as_bytes().as_slice())
            .execute(&h.pool)
            .await?;
        let inactive = h
            .root
            .world_changes(world_id, 0)
            .await?
            .ok_or("inactive day")?;
        assert!(!inactive.grimvale_full_moon);
        owner.reset_epoch = 0;
        let source_key = crate::map::view::placement_key(
            crate::map::overlay::TilePos {
                x: TRIGGER.x,
                y: TRIGGER.y,
                floor: TRIGGER.floor,
            },
            1,
        )
        .ok_or("entry key")?;
        assert_eq!(
            owner
                .bundle_map()
                .ok_or("map")?
                .facts
                .object_revision(source_key),
            0
        );
        let before = h
            .root
            .read_character_quest_state(&authority, fence.character_id)
            .await
            .map_err(debug)?;
        let east = encode_step_intent(StepDirection::East);
        super::quest_world_callbacks_fixtures::wire(
            &owner,
            &mut admitted,
            1,
            COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
            east.clone(),
        )
        .await?;
        assert_eq!(
            owner
                .runtime
                .lock()
                .await
                .read_actor_position(deps.actor)
                .map_err(debug)?
                .position(),
            origin,
            "an already-open encounter portal cannot bypass the current moon predicate"
        );
        assert_eq!(receipt_count(&h).await?, 0);
        assert_eq!(
            owner
                .bundle_map()
                .ok_or("map")?
                .facts
                .object_revision(source_key),
            0,
            "refused entry cannot mutate its native map object"
        );

        sqlx::query("UPDATE game_world_reset_active SET reset_epoch=1 WHERE world_id=encode($1,'hex')::uuid")
            .bind(world_id.as_bytes().as_slice()).execute(&h.pool).await?;
        let active = h
            .root
            .world_changes(world_id, 1)
            .await?
            .ok_or("active day")?;
        assert!(active.grimvale_full_moon);
        if prove_bindings {
            super::engworldchg_fixtures::assert_binding_lifecycle(&h, world_id, 1).await?;
        }
        owner.reset_epoch = 1;
        super::quest_world_callbacks_fixtures::wire(
            &owner,
            &mut admitted,
            2,
            COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
            east.clone(),
        )
        .await?;
        assert_eq!(
            owner
                .runtime
                .lock()
                .await
                .read_actor_position(deps.actor)
                .map_err(debug)?
                .position(),
            super::quest_world_callbacks_fixtures::local(DESTINATION)
        );
        assert_eq!(
            receipt_count(&h).await?,
            0,
            "stateless world observation is not personal quest progress"
        );
        let after = h
            .root
            .read_character_quest_state(&authority, fence.character_id)
            .await
            .map_err(debug)?;
        assert_eq!(after.tracks(), before.tracks());
        // No second portal, reward or receipt is created by a repeated native command.
        super::quest_world_callbacks_fixtures::wire(
            &owner,
            &mut admitted,
            2,
            COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
            east,
        )
        .await?;
        assert_eq!(receipt_count(&h).await?, 0);
        assert_eq!(
            owner
                .runtime
                .lock()
                .await
                .read_actor_position(deps.actor)
                .map_err(debug)?
                .position(),
            super::quest_world_callbacks_fixtures::local(DESTINATION)
        );
        let query_fence = owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(|()| "query fence")?
            .ok_or("query fence")?;
        let context = {
            let runtime = owner.runtime.lock().await;
            let snapshot = runtime.read_actor_position(deps.actor).map_err(debug)?;
            crate::durability::quest_world_callback::QuestWorldContext {
                party: Vec::new(),
                map_revision: crate::map::overlay::map_revision(
                    owner.bundle_map().ok_or("map")?.overlay.base(),
                ),
                content_revision: format!(
                    "sha256:{}",
                    runtime
                        .content_pin()
                        .server_artifact_digest()
                        .iter()
                        .map(|b| format!("{b:02x}"))
                        .collect::<String>()
                ),
                placement_context: runtime.content_pin().frame_binding_digest().to_vec(),
                reset_epoch: 1,
                actor_placement: deps.actor.placement_identity(),
                movement_revision: snapshot.revision(),
            }
        };
        let permit = owner.spell_lane_permit().await.ok_or("query permit")?;
        for _ in 0..2 {
            assert!(
                h.root
                    .observe_world_callback_gate(
                        &authority,
                        &h.node,
                        &permit,
                        query_fence,
                        context.clone(),
                        ENTRY.into()
                    )
                    .await?
            );
        }
        sqlx::query("UPDATE game_world_reset_active SET reset_epoch=2 WHERE world_id=encode($1,'hex')::uuid")
            .bind(world_id.as_bytes().as_slice()).execute(&h.pool).await?;
        h.root.world_changes(world_id, 2).await?.ok_or("next day")?;
        assert!(
            !h.root
                .observe_world_callback_gate(
                    &authority,
                    &h.node,
                    &permit,
                    query_fence,
                    context,
                    ENTRY.into()
                )
                .await?,
            "a retained moon snapshot cannot authorize the next save"
        );
        assert_eq!(receipt_count(&h).await?, 0);
        drop(permit);
        let registered = oteryn_gameplay_quests::quest::world_callbacks::load()?
            .into_iter()
            .find(|c| c.key == ENTRY)
            .ok_or("production moon entry registration")?;
        assert_eq!(registered.selector, callback.selector);
        assert_eq!(registered.teleport, callback.teleport);
        assert!(registered.storage.is_empty());
        if prove_bindings {
            super::engworldchg_fixtures::assert_native_quest_transition(
                &h,
                &owner,
                admitted.game_session_id,
                "grimvale",
                "oteryn:quest.grimvale_mini_world_change",
            )
            .await?;
        }
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await?;
        assert!(start.elapsed().as_secs_f64() > 1.0);
        eprintln!(
            "WORLDCHG-Q PG EXECUTED: Grimvale native stateless entry, current moon refusal/allow, replay, quest state unchanged; elapsed={:?}",
            start.elapsed()
        );
        Ok(())
    })
}

pub(super) async fn receipt_count(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT count(*) FROM game_character_quest_receipts WHERE transition_key=$1",
    )
    .bind(ENTRY)
    .fetch_one(&h.pool)
    .await?)
}
