//! The Newhaven vocation offer and selection through admitted command dispatch.
use super::fixture::{Harness, debug, id, run};
use super::tutorial_vitals_fixture::{admit, native_asset_revisions, publish_native_activation};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::gameplay_transport::capabilities::{OfferedCapability, SelectedCapabilities};
use oteryn_protocol_oteryn::CommandStatus;

#[test]
fn tutorial_vocation_offer_and_submit_real_dispatch() -> TestResult {
    vocation_scenario(false, false)
}

#[test]
fn tutorial_vocation_waits_for_main_backpack() -> TestResult {
    vocation_scenario(true, false)
}

#[test]
fn tutorial_vocation_resumes_retained_request() -> TestResult {
    vocation_scenario(false, true)
}

fn vocation_scenario(missing_bag: bool, retained: bool) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run("tutorial-vocation", move || {
        run(async |admin| {
            let h = Harness::create_newhaven(admin, "tutorial_vocation").await?;
            let request = prepare_current_session(&h).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
            let (controller, pin) =
                super::death_ml_loss_composed::production_generation_with_pin(world, "content-1")?;
            publish_native_activation(&h, &pin).await?;
            let active = controller.active().ok_or("active")?;
            native_asset_revisions(&h, active).await?;
            let room = pin.qualified_room().clone();
            // The native test scene is small; use the accepted Newhaven entry start,
            // as the existing Corruptor qualification does, for this owner fixture.
            let channel_pin = crate::foundation::ChannelContentPin::from_activation(
                world,
                pin.activation_sequence(),
                pin.identity().server_artifact_digest(),
                pin.identity().client_artifact_digest(),
                pin.frame_binding().digest(),
                room.map_revision_digest(),
                (32534, 32513, 7),
            );
            let deps = Dependencies::with_content(
                channel_pin,
                room,
                crate::spell::cast::v1_spell_book().map_err(debug)?,
            )?;
            let mut owner = deps.owner(&h, &authority);
            owner.active_generation = Some(active);
            let mut admitted = deps.admitted;
            Box::pin(owner.admit_progression_session(&mut admitted)).await;
            admit(&owner, &deps, &request).await?;
            let current =
                crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                    .current_session_at(admitted.game_session_id)
                    .await?
                    .0;
            admitted.item_fence = crate::gameplay_transport::item_fence_of(current);
            admitted.first_entry =
                crate::gameplay_transport::connection::FirstEntryOutcome::Positioned;
            assert_eq!(
                super::cyclopedia::support::exchange(&owner, &mut admitted, 1, 6243, &[])
                    .await?
                    .0,
                CommandStatus::Rejected,
                "504 must be selected"
            );
            let offers = [
                OfferedCapability {
                    id: 31,
                    requires: &[],
                },
                OfferedCapability {
                    id: 504,
                    requires: &[31],
                },
            ];
            admitted.continuity.selected_capabilities =
                SelectedCapabilities::select(&offers, &[31, 504]).ok_or("select vocation")?;
            let (status, bytes) =
                super::cyclopedia::support::exchange(&owner, &mut admitted, 2, 6243, &[]).await?;
            assert_eq!(
                status,
                CommandStatus::Accepted,
                "tutorial vocation observation"
            );
            assert_eq!(bytes, [0, 0], "no offer before entering the portal");
            assert_eq!(
                super::cyclopedia::support::exchange(&owner, &mut admitted, 3, 6243, &[4])
                    .await?
                    .0,
                CommandStatus::Rejected,
                "cannot choose before the portal"
            );
            let landing = crate::foundation::MovementLocalPosition {
                x: 32544,
                y: 32515,
                floor: 7,
            };
            {
                let mut runtime = deps.runtime.lock().await;
                let mut movement = runtime.borrow_movement_position();
                let mut at = movement.read(deps.actor).map_err(debug)?;
                for x in 32535..=32544 {
                    let next = crate::foundation::MovementLocalPosition { x, ..at.position() };
                    at = movement.commit_cardinal(at, next).map_err(debug)?;
                }
                for y in 32514..=32515 {
                    let next = crate::foundation::MovementLocalPosition { y, ..at.position() };
                    at = movement.commit_cardinal(at, next).map_err(debug)?;
                }
                assert_eq!(at.position(), landing);
            }
            let fence = owner
                .current_quest_fence(admitted.game_session_id)
                .await
                .map_err(|()| "tutorial fence")?
                .ok_or("tutorial fence")?;
            owner
                .reconcile_tutorial_position(fence, landing)
                .await
                .map_err(|()| "portal tutorial transition")?;
            let (status, offer) =
                super::cyclopedia::support::exchange(&owner, &mut admitted, 4, 6243, &[]).await?;
            assert_eq!(status, CommandStatus::Accepted);
            assert_eq!(offer, [31, 0]);
            assert_eq!(
                super::cyclopedia::support::exchange(&owner, &mut admitted, 5, 6243, &[6])
                    .await?
                    .0,
                CommandStatus::Rejected
            );
            if missing_bag {
                move_fixture_bag(&h, 41, 98).await?;
                assert!(
                    owner.tutorial_vocation_wire(admitted, &[4]).await.is_none(),
                    "kit could not be placed"
                );
                assert!(
                    owner.tutorial_vocation_wire(admitted, &[]).await.is_none(),
                    "observation retries the kit"
                );
                move_fixture_bag(&h, 98, 41).await?;
            }
            if retained {
                use crate::durability::character_build::{BuildFormula, BuildOccurrence};
                use crate::gameplay_transport::vocation_choice::{
                    RetainedNpcVocation, vocation_choice_request,
                };
                let formula = active
                    .native_gameplay()
                    .ok_or("active native gameplay")?
                    .training_formula()
                    .ok_or("training formula")?;
                let before = h
                    .root
                    .read_character_build_state(&authority, fence.character_id)
                    .await
                    .map_err(debug)?;
                let request = vocation_choice_request(
                    BuildOccurrence::from_bytes(id(88)).map_err(debug)?,
                    before,
                    "paladin",
                    formula,
                    None,
                )
                .map_err(debug)?;
                let old = owner
                    .npc_vocation_fence(admitted.game_session_id)
                    .await
                    .ok_or("old fence")?;
                assert!(owner.states.tutorial_sessions.retain_vocation(
                    admitted.game_session_id,
                    RetainedNpcVocation {
                        fence: old,
                        request,
                        formula_digest: formula.digest()
                    }
                ));
                sqlx::query("UPDATE game_durability_reconnect_sessions SET current_generation=current_generation+1 WHERE game_session_id=encode($1,'hex')::uuid").bind(admitted.game_session_id.as_bytes().as_slice()).execute(&h.pool).await?;
                admitted.continuity.connection_generation += 1;
                admitted.item_fence = Some(
                    owner
                        .npc_vocation_fence(admitted.game_session_id)
                        .await
                        .ok_or("resumed fence")?
                        .item_fence(),
                );
            }
            let (status, selected) = super::cyclopedia::support::exchange(
                &owner,
                &mut admitted,
                6,
                6243,
                if retained { &[] } else { &[4] },
            )
            .await?;
            assert_eq!(status, CommandStatus::Accepted, "submit Paladin");
            assert_eq!(selected, [0, 4]);
            let build = h
                .root
                .read_character_build_state(&authority, fence.character_id)
                .await
                .map_err(debug)?;
            assert_eq!(build.vocation(), "paladin");
            assert_eq!(
                super::cyclopedia::support::exchange(&owner, &mut admitted, 7, 6243, &[])
                    .await?
                    .1,
                selected
            );
            assert_eq!(
                super::cyclopedia::support::exchange(&owner, &mut admitted, 8, 6243, &[1])
                    .await?
                    .0,
                CommandStatus::Rejected,
                "a selected vocation cannot be replaced"
            );
            let mut stale = admitted;
            stale.continuity.connection_generation += 1;
            assert!(
                owner.tutorial_vocation_wire(stale, &[]).await.is_none(),
                "a non-current generation cannot observe an offer"
            );
            // The connection owner rejects this generation before it can return a
            // command result. Keep the handler refusal and the transport closure covered.
            let refusal = super::cyclopedia::support::exchange(&owner, &mut stale, 9, 6243, &[])
                .await
                .err()
                .ok_or("non-current connection returned a command result")?;
            let refusal = refusal.to_string();
            assert!(
                refusal.starts_with("cyclopedia command 9/6243 ended AdmittedThenDisconnected(")
            );
            assert!(refusal.ends_with("early eof"));
            let receipts: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM game_character_build_receipts WHERE cause='vocation_choice'",
            )
            .fetch_one(&h.pool)
            .await?;
            assert_eq!(receipts, 1);
            let grants: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_starter_grants WHERE template_key LIKE 'oteryn:starter.newhaven.%'").fetch_one(&h.pool).await?;
            assert!(grants > 0, "the accepted profession kit is granted");
            if let Some(directory) = std::env::var_os("OTERYN_TUTORIAL_VOCATION_CAPTURE") {
                let directory = std::path::Path::new(&directory);
                std::fs::create_dir_all(directory)?;
                std::fs::write(directory.join("offer.bin"), offer)?;
                std::fs::write(directory.join("selected.bin"), selected)?;
            }
            drop(owner);
            drop(authority);
            drop(seal);
            h.cleanup().await
        })
    })
}

async fn move_fixture_bag(h: &Harness, from: u8, to: u8) -> TestResult {
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    let moved=sqlx::query("UPDATE game_item_container_slots SET character_id=encode($2,'hex')::uuid WHERE character_id=encode($1,'hex')::uuid").bind(id(from).as_slice()).bind(id(to).as_slice()).execute(&mut *tx).await?;
    assert_eq!(moved.rows_affected(), 1);
    tx.commit().await?;
    Ok(())
}
