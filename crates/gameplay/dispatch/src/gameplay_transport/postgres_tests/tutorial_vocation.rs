//! The Newhaven vocation offer and selection through admitted command dispatch.
use super::fixture::{Harness, debug, id, run};
use super::tutorial_vitals_fixture::{admit, native_asset_revisions, publish_native_activation};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::gameplay_transport::capabilities::{OfferedCapability, SelectedCapabilities};
use oteryn_protocol_oteryn::CommandStatus;
use crate::gameplay_transport::connection::{ControlLossResult, FreshAdmissionAuthority, ResumeAttempt};

#[test]
fn tutorial_vocation_offer_and_submit_real_dispatch() -> TestResult {
    vocation_scenario(false, false, false)
}

#[test]
fn tutorial_vocation_waits_for_main_backpack() -> TestResult {
    vocation_scenario(true, false, false)
}

#[test]
fn tutorial_vocation_resumes_retained_request() -> TestResult {
    vocation_scenario(false, true, false)
}

#[test]
fn tutorial_vocation_logout_keeps_refill_when_live_installation_is_pending() -> TestResult {
    vocation_scenario(false, true, true)
}

fn vocation_scenario(missing_bag: bool, retained: bool, post_commit: bool) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run("tutorial-vocation", move || {
        run(async |admin| {
            let h = Harness::create_newhaven(admin, "tutorial_vocation").await?;
            let recovery = if retained {
                Some(super::house_transfer::RecoverySource::start(&h).await?)
            } else {
                None
            };
            let request = if retained {
                super::house_transfer::prepare_owned_session_request(&h).await?
            } else {
                prepare_current_session(&h).await?
            };
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
            if let Some(recovery) = &recovery {
                owner.evidence = &recovery.evidence;
            }
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
            const OFFERS: &[OfferedCapability] = &[
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
                SelectedCapabilities::select(OFFERS, &[31, 504]).ok_or("select vocation")?;
            owner.required_capabilities = crate::gameplay_transport::world_required_capabilities::BoundRequiredCapabilities::for_channel_boot(
                &[], current.current_runtime_scope(), current.current_scope_generation().get(), "policy-1", OFFERS,
            ).map_err(debug)?;
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
                fill_fixture_backpack(&h, true).await?;
                assert!(owner.tutorial_vocation_wire(admitted, &[4]).await.is_none());
                let build = h.root.read_character_build_state(&authority, fence.character_id).await.map_err(debug)?;
                assert_eq!(build.vocation(), crate::durability::character_build::NO_VOCATION);
                fill_fixture_backpack(&h, false).await?;

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
                assert!(owner.track_restored(
                    admitted.game_session_id,
                    deps.actor,
                    &crate::gameplay_transport::last_position::AdmissionPosition::FirstLogin,
                    fence,
                ));
                if post_commit {
                    use crate::gameplay_transport::vocation_choice::commit_retained_npc_vocation;
                    let session = admitted.game_session_id;
                    {
                        let runtime = owner.runtime.lock().await;
                        let mut states = owner.states.spell_states.lock().await;
                        states.get_mut(&runtime, deps.actor, session).ok_or("live spell state")?.set_health_for_test(1);
                    }
                    let retained = owner.states.tutorial_sessions.vocation(session)
                        .map_err(|()| "retained choice")?.ok_or("retained choice")?;
                    assert!(owner.save_first_vocation_vitals(deps.actor, session, old, &retained.request.before).await);
                    let mut slot = owner.revision_sequencer.acquire(old.character_id).await;
                    commit_retained_npc_vocation(
                        &mut slot, owner.root, owner.character, owner.holder, &retained, formula,
                    ).await.map_err(debug)?;
                    drop(slot);
                    // Stop at the same state a failed post-commit read leaves behind:
                    // the refill has committed while the live actor still has 1 HP.
                    let refill = saved_vitals(&h).await?;
                    assert!(refill.0 > 1, "first vocation refilled the wounded actor");
                    assert!(owner.last_position_capture_is_held(session).await);
                    assert_eq!(owner.write_due_last_positions(true).await, 0);
                    assert_eq!(saved_vitals(&h).await?, refill);
                    let _ = owner.logout_session(admitted).await;
                    assert_eq!(saved_vitals(&h).await?, refill, "logout must preserve the committed refill");
                    drop(owner);
                    drop(authority);
                    drop(seal);
                    return h.cleanup().await;
                }
                assert!(
                    owner.last_position_capture_is_held(admitted.game_session_id).await,
                    "the retained choice must fence stale vitals even if post-commit installation fails"
                );
                owner.states.command_ingress.admit(admitted.game_session_id);
                assert_eq!(
                    owner.lose_control(admitted, std::time::Duration::ZERO, None).await,
                    ControlLossResult::Recorded,
                );
                let token = super::house_transfer::RecoverySource::token(88)?;
                let prior = admitted;
                admitted = owner.resume_session(ResumeAttempt {
                    game_session_id: prior.game_session_id,
                    recovery_material: token.as_bytes(),
                    transport: crate::foundation::AuthenticatedTransportRefV1::decode(&[88; 16]).map_err(debug)?,
                    last_applied_server_sequence: prior.continuity.server_sequence,
                    supported_capabilities: prior.continuity.selected_capabilities.as_slice(),
                }).await.map_err(debug)?;
                assert_eq!(admitted.game_session_id, prior.game_session_id);
                assert_eq!(admitted.continuity.connection_generation, prior.continuity.connection_generation + 1);
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
            let occurrences: Vec<Vec<u8>> = sqlx::query_scalar(
                "SELECT uuid_send(build_occurrence_id) FROM game_character_build_receipts WHERE cause='vocation_choice'",
            )
            .fetch_all(&h.pool)
            .await?;
            assert_eq!(occurrences.len(), 1);
            if retained {
                assert_eq!(occurrences[0], id(88));
            }
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

async fn fill_fixture_backpack(h: &Harness, fill: bool) -> TestResult {
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
    if fill {
        sqlx::query("WITH bag AS (SELECT character_id,item_instance_id,world_id FROM game_item_container_slots WHERE character_id=encode($1,'hex')::uuid),
            items AS (INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id)
                SELECT game_character_uuid_v7(),world_id,'Item','oteryn:item.tibia.i3031','definition-r1',1,1,game_character_uuid_v7() FROM bag CROSS JOIN generate_series(1,20)
                RETURNING item_instance_id,world_id,minted_transaction_id)
            INSERT INTO game_item_container_entries(character_id,parent_item_instance_id,item_instance_id,world_id,placement_ordinal,placed_transaction_id)
                SELECT bag.character_id,bag.item_instance_id,items.item_instance_id,items.world_id,100+row_number() OVER (ORDER BY items.item_instance_id),items.minted_transaction_id FROM items CROSS JOIN bag")
            .bind(id(41).as_slice()).execute(&mut *tx).await?;
    } else {
        sqlx::query("WITH removed AS (DELETE FROM game_item_container_entries WHERE character_id=encode($1,'hex')::uuid AND placement_ordinal>100 RETURNING item_instance_id)
            DELETE FROM game_item_instances WHERE item_instance_id IN (SELECT item_instance_id FROM removed)")
            .bind(id(41).as_slice()).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    Ok(())
}


async fn saved_vitals(h: &Harness) -> TestResult<(i64, i64, i64)> {
    Ok(sqlx::query_as("SELECT health::bigint,mana::bigint,write_sequence::bigint FROM game_character_last_positions WHERE character_id=encode($1,'hex')::uuid")
        .bind(id(41).as_slice()).fetch_one(&h.pool).await?)
}
