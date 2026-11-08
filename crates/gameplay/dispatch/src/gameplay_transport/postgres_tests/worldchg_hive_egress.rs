//! Native Hive USE proves safe egress without a live scent or open stage.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::content::channel_content_pin::ChannelContentPin;
use crate::foundation::ChannelRuntimeV1;
use crate::gameplay_transport::connection::FirstEntryOutcome;
use oteryn_gameplay_quests::quest::world_callbacks::Position;
use oteryn_gameplay_world::map::view::MapFacts;
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};
const TRIGGER: Position = Position {
    x: 33598,
    y: 31228,
    floor: -1,
};
#[test]
fn worldchg_hive_native_gate_closed_stage_and_missing_scent_allow_egress_only() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run("worldchg-hive-egress", scenario)
}
fn scenario() -> TestResult {
    let callback = oteryn_gameplay_quests::quest::hive_access::callbacks()
        .into_iter()
        .find(|callback| {
            callback.selector.position == Some(TRIGGER)
                && callback.selector.appearance == Some(13291)
        })
        .ok_or("native Hive gate")?;
    let world = super::quest_world_callbacks::world(&callback, TRIGGER)?;
    run(async |admin| {
        let start = std::time::Instant::now();
        let h = Harness::create_with_admission_seed(admin, "worldchg_hive_egress", false, false)
            .await?;
        super::reset();
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
        let origin = super::quest_world_callbacks::local(Position {
            x: TRIGGER.x + 1,
            ..TRIGGER
        });
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
        assert_eq!(
            inactive.counter_stages
                [crate::durability::world_event::changes::counters::CounterKind::HiveBorn.index()],
            1
        );
        owner.reset_epoch = 0;
        let source_key = crate::map::view::placement_key(
            crate::map::overlay::TilePos {
                x: TRIGGER.x,
                y: TRIGGER.y,
                floor: TRIGGER.floor,
            },
            1,
        )
        .ok_or("gate source")?;
        let map = owner.bundle_map().ok_or("map")?;
        let target = |revision| {
            let mut placement = map.overlay.base().digest().to_vec();
            placement.extend_from_slice(&source_key.to_be_bytes());
            WorldObjectTarget {
                placement,
                expected_revision: revision,
            }
        };
        let payload =
            oteryn_protocol_oteryn::world_object::encode_use_intent(&target(0)).map_err(debug)?;
        let result =
            super::quest_world_callbacks::wire(&owner, &mut admitted, 1, 2, payload).await?;
        assert_eq!(
            oteryn_protocol_oteryn::world_object::decode_use_result(&result).map_err(debug)?,
            UseDisposition::Committed
        );
        let outside = super::quest_world_callbacks::local(Position {
            x: TRIGGER.x - 1,
            ..TRIGGER
        });
        assert_eq!(
            owner
                .runtime
                .lock()
                .await
                .read_actor_position(deps.actor)
                .map_err(debug)?
                .position(),
            outside
        );
        let revision = map.facts.object_revision(source_key);
        let payload = oteryn_protocol_oteryn::world_object::encode_use_intent(&target(revision))
            .map_err(debug)?;
        let result =
            super::quest_world_callbacks::wire(&owner, &mut admitted, 2, 2, payload).await?;
        assert_eq!(
            oteryn_protocol_oteryn::world_object::decode_use_result(&result).map_err(debug)?,
            UseDisposition::Rejected
        );
        assert_eq!(
            owner
                .runtime
                .lock()
                .await
                .read_actor_position(deps.actor)
                .map_err(debug)?
                .position(),
            outside
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_character_quest_receipts")
                .fetch_one(&h.pool)
                .await?,
            0
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_world_change_credit_receipts")
                .fetch_one(&h.pool)
                .await?,
            0
        );
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await?;
        assert!(start.elapsed().as_secs_f64() > 1.0);
        println!(
            "WORLDCHG-Q PG EXECUTED Hive native USE current fenced egress, closed-stage/missing-permit entry refused, no quest/shared credit writes; elapsed={:?}",
            start.elapsed()
        );
        Ok(())
    })
}
