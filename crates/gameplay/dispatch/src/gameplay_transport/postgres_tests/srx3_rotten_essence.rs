//! Actual carried essence USE reaches the production composed owner and paired writers.

use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::content::channel_content_pin::ChannelContentPin;
use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};

use oteryn_protocol_oteryn::world_object::UseDisposition;

#[test]
fn srx3_rotten_essence_channel_production_use_burns_and_sets_quest_once_postgres() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e_fixtures::scenario_thread::run("srx3-essence-use", || {
        run(async |admin| {
            let h = Harness::create_with_admission_seed(admin, "srx3_essence_use", false, false)
                .await?;
            prepare_current_session(&h).await?;
            // Historical inventory only; all new USE/Quest/Item guards remain active.
            super::item_look_fixtures::seed_main_backpack(&h, &[("oteryn:item.tibia.i43501", 2)]).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
            let (controller, pin) =
                super::death_ml_loss_composed_fixtures::production_generation_with_pin(world, "content-1")?;
            let active = controller.active().ok_or("active generation")?;
            let native = active.native_gameplay().ok_or("native gameplay")?;
            let room = pin.qualified_room().clone();
            let channel_pin = ChannelContentPin::from_activation(
                world,
                pin.activation_sequence(),
                pin.identity().server_artifact_digest(),
                pin.identity().client_artifact_digest(),
                pin.frame_binding().digest(),
                room.map_revision_digest(),
                (0, 0, 0),
            );
            let deps = Dependencies::with_content(
                channel_pin,
                room,
                crate::spell::cast::v1_spell_book().map_err(debug)?,
            )?;
            let mut owner = deps.owner(&h, &authority);
            owner.active_generation = Some(active);
            let lowered =
                oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
                    .map_err(debug)?;
            owner.quest_catalogue = Some(std::sync::Arc::new(lowered.catalogue().clone()));
            let mut admitted = deps.admitted;
            admitted.item_fence = crate::gameplay_transport::item_fence_of(
                crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                    .current_session_at(admitted.game_session_id)
                    .await?
                    .0,
            );
            Box::pin(owner.admit_progression_session(&mut admitted)).await;
            deps.initialize(&owner).await;
            owner
                .runtime
                .lock()
                .await
                .initialize_first_entry_position(deps.actor)
                .map_err(debug)?;
            let index = native.item_index().ok_or("source item index")?;
            let definition = index
                .definition_ref("Item", "oteryn:item.tibia.i43501", "definition-r1")
                .ok_or("real carried essence definition")?;
            let command = UseCommand {
                game_session_id: admitted.game_session_id,
                command_id: 1,
                item_fence: admitted.item_fence,
            };
            let first = owner
                .use_item_by_definition(deps.actor, command, definition)
                .await;
            assert_eq!(
                first.disposition,
                UseDisposition::Committed,
                "actual source essence USE must reach its paired owner"
            );
            let replay = owner
                .use_item_by_definition(deps.actor, command, definition)
                .await;
            assert_eq!(replay.disposition, first.disposition);
            let changed = index
                .definition_ref("Item", "oteryn:item.tibia.i43502", "definition-r1")
                .ok_or("other source essence")?;
            assert_eq!(
                owner
                    .use_item_by_definition(deps.actor, command, changed)
                    .await
                    .disposition,
                UseDisposition::Rejected,
                "same CommandRef cannot replay another essence"
            );

            // A new USE while the source flag is already held must not burn
            // another unit (rottenEssences.lua:16-20), independently of replay.
            let _already_held = owner
                .use_item_by_definition(
                    deps.actor,
                    UseCommand {
                        command_id: 2,
                        ..command
                    },
                    definition,
                )
                .await;
            let state:(i64,i64,i64)=sqlx::query_as("SELECT (SELECT quantity::bigint FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid), (SELECT value FROM game_character_quest_tracks WHERE character_id=encode($2,'hex')::uuid AND track_key='oteryn:quest-progress/storage/52280'), (SELECT count(*) FROM game_item_use_records WHERE cause_kind=4)")
                .bind(id(71).as_slice()).bind(id(41).as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(state, (1, 1, 1));
            assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_character_quest_receipts WHERE transition_key='oteryn:quest-transition/rotten-blood/essence-murcion'").fetch_one(&h.pool).await?,1);
            drop(owner);
            drop(authority);
            drop(seal);
            h.cleanup().await
        })
    });
    eprintln!(
        "SRX3_ESSENCE_CHANNEL_CASE_EXECUTED duration={:?} passed={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}
