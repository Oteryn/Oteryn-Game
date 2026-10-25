use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::gameplay_transport::connection::FirstEntryOutcome;
use oteryn_gameplay_progression::loop_training::TrainingRates;
use std::sync::Arc;
#[path = "ranged_training/world.rs"]
mod world;
use crate::content::channel_content_pin::ChannelContentPin;
use crate::foundation::{ChannelRuntimeV1, MovementLocalPosition};
use crate::gameplay_transport::{
    capabilities::SelectedCapabilities,
    connection::{FreshAdmissionAuthority, UseCommand},
    item_view::{ItemKey, ItemViewContinuity, SessionItemView},
};
use std::time::Duration;

fn field(out: &mut Vec<u8>, key: u8, mut value: u64) {
    out.push(key);
    while value >= 128 {
        out.push((value as u8 & 127) | 128);
        value >>= 7;
    }
    out.push(value as u8);
}
#[test]
fn ranged_attack_postgres_dummy_bow_wand_and_rod_train_at_distance_five() -> TestResult {
    super::kill_reward_e2e::scenario_thread::run("ranged-dummy", || {
        scenario(28555, 5)?;
        scenario(28556, 0)?;
        scenario(28557, 0)
    })
}
fn scenario(item: u32, family: usize) -> TestResult {
    run(async |admin| {
        let h = Harness::create_with_admission_seed(admin, "loop_training_exercise", true, false)
            .await?;
        eprintln!("POSTGRES_STARTED LOOP-TRAIN-1 exercise");
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut deps = Dependencies::new()?;
        let (controller, pin) =
            super::death_ml_loss_composed::production_generation_with_pin(
                deps.admitted.world_id,
                "content-1",
            )?;
        let active = controller.active().ok_or("active")?;
        let native = active.native_gameplay().ok_or("native")?;
        let index = native.item_index().ok_or("item index")?;
        let world = world::training_world(index, deps.admitted)?;
        let cells = world
            .movement_cells(deps.room.movement_cells())
            .map_err(debug)?;
        let content_pin = ChannelContentPin::from_activation(
            deps.admitted.world_id,
            1,
            active.identity().server_artifact_digest(),
            active.identity().client_artifact_digest(),
            pin.frame_binding().digest(),
            world.map_revision_digest(),
            (1, 0, 7),
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
            content_pin,
        )
        .map_err(debug)?;
        let reservation = runtime
            .reserve_fresh_session(deps.admitted.game_session_id)
            .map_err(debug)?;
        deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .initialize_pinned_test_position(
                deps.actor,
                MovementLocalPosition {
                    x: 1,
                    y: 0,
                    floor: 7,
                },
            )
            .map_err(debug)?;
        deps.runtime = tokio::sync::Mutex::new(runtime);
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(active);
        owner.movement_cells = &cells;
        deps.initialize(&owner).await;
        let mut rates = TrainingRates::from_json(include_bytes!(
            "../../../../../../rulesets/training/provisional-v1.json"
        ))
        .map_err(debug)?;
        rates.enabled = true;
        owner.states.training.config = Some(Arc::new(rates));
        let mut admitted = deps.admitted;
        admitted.runtime_actor = Some(deps.actor);
        admitted.first_entry = FirstEntryOutcome::Positioned;
        let current =
            crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                .current_session(admitted.game_session_id)
                .await?;
        let fence = crate::gameplay_transport::item_fence_of(current).ok_or("fence")?;
        admitted.item_fence = Some(fence);
        admitted.continuity.selected_capabilities =
            SelectedCapabilities::select(owner.offered_capabilities(), &[4, 6, 18, 21, 54])
                .ok_or("selection")?;
        assert!(admitted.continuity.selected_capabilities.contains(54));
        // A pre-training build may already have non-default 0141 stamina. Establish
        // its ordinary build through the real writer before seeding that legacy value.
        let gameplay = crate::durability::character_progression::CurrentCharacterGameplayFence {
            character_id: fence.character_id,
            game_session_id: fence.game_session_id,
            connection_generation: fence.connection_generation,
            character_lease_generation: fence.character_lease_generation,
            runtime_scope: fence.runtime_scope,
            scope_ownership_generation: fence.scope_ownership_generation,
            expected_character_revision: h
                .root
                .read_current_character(&authority, fence.character_id)
                .await
                .map_err(debug)?
                .revision,
        };
        let before = h
            .root
            .read_character_build_state(&authority, fence.character_id)
            .await
            .map_err(debug)?;
        let formula = native.training_formula().ok_or("training formula")?;
        let after =
            crate::durability::character_build::convert_vocation(formula, &before, "knight")
                .map_err(debug)?;
        let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
        slot.commit_build(
            &h.root,
            &authority,
            &h.node,
            gameplay,
            crate::durability::character_build::BuildChangeRequest {
                occurrence: crate::durability::character_build::BuildOccurrence::from_bytes(id(78))
                    .map_err(debug)?,
                cause: crate::durability::character_build::BuildCause::VocationChoice,
                before,
                after,
                pruned_stance: None,
            },
            formula,
            None,
        )
        .await
        .map_err(debug)?;
        drop(slot);
        sqlx::query("UPDATE game_character_build_state SET stamina_minutes=2339 WHERE character_id=encode($1,'hex')::uuid")
            .bind(fence.character_id.as_bytes().as_slice()).execute(&h.pool).await?;
        owner.admit_loop_training(&mut admitted).await;
        assert_eq!(admitted.first_entry, FirstEntryOutcome::Positioned);
        let stamina: i32 =
            sqlx::query_scalar("SELECT stamina_minutes FROM game_character_build_state")
                .fetch_one(&h.pool)
                .await?;
        assert_eq!(
            stamina, 2339,
            "first training marker preserves legacy stamina"
        );
        // Existing fixture builders provision custody; production dispatch owns every tested write.
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx)
            .await?;
        sqlx::raw_sql(sqlx::AssertSqlSafe(format!(
            "{}{}",
            crate::durability::item_fee_burn::test_support::item(
                70,
                "oteryn:item.tibia.i2854",
                "definition-r1",
                1
            ),
            crate::durability::item_fee_burn::test_support::item(
                72,
                &format!("oteryn:item.tibia.i{item}"),
                "definition-r1",
                1
            )
        )))
        .execute(&mut *tx)
        .await?;
        sqlx::query("INSERT INTO game_item_container_slots(character_id,item_instance_id,world_id,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid)").bind(id(41).as_slice()).bind(id(70).as_slice()).bind(id(42).as_slice()).bind(id(74).as_slice()).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_item_container_entries(character_id,parent_item_instance_id,item_instance_id,world_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,1,encode($5,'hex')::uuid)").bind(id(41).as_slice()).bind(id(70).as_slice()).bind(id(72).as_slice()).bind(id(42).as_slice()).bind(id(75).as_slice()).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)").bind(id(41).as_slice()).execute(&mut *tx).await?;
        tx.commit().await?;
        let dummy = ItemKey::MapBase {
            bundle_digest: world.base().digest(),
            placement_key: crate::map::view::placement_key(
                crate::map::overlay::TilePos {
                    x: 6,
                    y: 0,
                    floor: -7,
                },
                2,
            )
            .ok_or("placement")?,
            reset_epoch: 0,
        };
        let weapon = ItemKey::Instance(id(72));
        let mut items = SessionItemView::resume(ItemViewContinuity::default()).with_map_view();
        let (wh, dh) = items
            .map_view(&[weapon, dummy], |table| {
                Ok((
                    table.handle(&weapon).ok_or(
                        crate::gameplay_transport::item_view::ItemViewError::LimitExceeded,
                    )?,
                    table.handle(&dummy).ok_or(
                        crate::gameplay_transport::item_view::ItemViewError::LimitExceeded,
                    )?,
                ))
            })
            .map_err(debug)?;
        let mut start = vec![8, 1];
        field(&mut start, 16, wh.get());
        field(&mut start, 24, dh.get());
        let command = UseCommand {
            game_session_id: admitted.game_session_id,
            command_id: 1,
            item_fence: Some(fence),
        };
        let mut without = admitted;
        without.continuity.selected_capabilities = SelectedCapabilities::NONE;
        assert_eq!(
            crate::gameplay_transport::loop_training::wire::dispatch(
                &owner,
                without,
                command,
                67,
                &start,
                Some(&items)
            )
            .await
            .ok_or("capability refusal")?
            .disposition,
            4
        );
        let mut stale = admitted;
        let mut stale_fence = fence;
        stale_fence.connection_generation =
            crate::foundation::ConnectionGeneration::new(2).map_err(debug)?;
        stale.item_fence = Some(stale_fence);
        assert_eq!(
            crate::gameplay_transport::loop_training::wire::dispatch(
                &owner,
                stale,
                UseCommand {
                    item_fence: Some(stale_fence),
                    ..command
                },
                67,
                &start,
                Some(&items)
            )
            .await
            .ok_or("stale refusal")?
            .disposition,
            4
        );
        let result = crate::gameplay_transport::loop_training::wire::dispatch(
            &owner,
            admitted,
            command,
            67,
            &start,
            Some(&items),
        )
        .await
        .ok_or("known start")?;
        assert_eq!(result.disposition, 1, "{result:?}");
        assert_eq!(result.charges, 500);
        owner.clock_origin -= Duration::from_secs(2);
        let stopped = crate::gameplay_transport::loop_training::wire::dispatch(&owner, admitted,
            UseCommand { command_id: 2, ..command }, 67, &[8, 3], Some(&items)).await.ok_or("stop")?;
        assert_eq!(stopped.disposition, 2);
        assert_eq!(stopped.charges, 499);
        let stored: (i64, i64) = sqlx::query_as("SELECT distance_tries,mana_spent FROM game_character_build_state").fetch_one(&h.pool).await?;
        if family == 5 { assert_eq!(stored.0, 7, "remote bow dummy training is durable"); }
        else { assert!(stored.1 > 0, "remote wand/rod training is durable"); }
        drop(owner); drop(authority); drop(seal); h.cleanup().await
    })
}
