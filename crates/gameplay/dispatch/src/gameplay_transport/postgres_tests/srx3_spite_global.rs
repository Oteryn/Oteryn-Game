//! Actual actor-free GlobalEvent through the existing physical Item transaction.
use super::fixture::{Harness, debug, run};
use super::srx3_activity_source_fixture::{SourceScene, install_scene, prepare_player};
use super::{TestResult, reset};
use crate::content::bosses::activity_template::PinnedActivitySourceRoom;
use crate::durability::creature_source_items::SpiteGlobalFireCallback;
use crate::foundation::MovementLocalPosition;

#[test]
fn srx3_spite_registered_global_fire_uses_real_scope_without_creature_postgres() -> TestResult {
    registered_global_case(false, false)
}

#[test]
fn srx3_spite_registered_global_reserves_current_native_expiry_target_postgres() -> TestResult {
    registered_global_case(true, false)
}

#[test]
fn srx3_spite_registered_global_original_five_second_expiry_removes_current_fire_postgres()
-> TestResult {
    registered_global_case(false, true)
}

fn registered_global_case(reserve_target: bool, expire: bool) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run("srx3-spite-global", move || {
        run(async move |admin| {
            reset();
            let scene = SourceScene::new(
                PinnedActivitySourceRoom::GoshnarsSpite,
                MovementLocalPosition {
                    x: 33742,
                    y: 31639,
                    floor: 14,
                },
            )?;
            let deps = scene.dependencies()?;
            let h = Harness::create_with_admission_seed(admin, "srx3_spite_global", false, false)
                .await?;
            let evidence = super::house_transfer::RecoverySource::start(&h).await?;
            super::house_transfer::prepare_owned_session(&h).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let mut owner = deps.owner(&h, &authority);
            owner.evidence = &evidence.evidence;
            install_scene(&mut owner, &scene, PinnedActivitySourceRoom::GoshnarsSpite)?;
            super::boss_activity::issue_content_and_reset(&h, &scene.pin).await?;
            let mut admitted = deps.admitted;
            prepare_player(&h, &owner, &mut admitted, 500).await?;
            let native = scene
                .controller
                .active()
                .and_then(|active| active.native_gameplay())
                .ok_or("actual native GlobalEvent generation")?;
            assert_eq!(
                scene.room.pinned_activity_source_room(),
                Some(PinnedActivitySourceRoom::GoshnarsSpite)
            );
            assert!(
                scene.room.source_world().is_none(),
                "production retained source recipe uses the actual Bundle collision frame"
            );
            assert!(native.qualifies_soul_war_callbacks(scene.pin.server_artifact_digest()));
            let proof = native
                .qualify_spite_global_event(&scene.room)
                .ok_or("actual retained registered Spite GlobalEvent DATA")?;
            let permit = owner
                .spell_lane_permit()
                .await
                .ok_or("actual physical lane")?;
            let mut runtime = owner.runtime.lock().await;
            assert!(
                runtime
                    .source_registered_creature_census()
                    .map_err(debug)?
                    .into_iter()
                    .all(|(actor, _)| {
                        runtime.companion_snapshot(actor).is_ok_and(|snapshot| {
                            snapshot.state.policy.definition_key
                                != "oteryn:creature.goshnar_s_spite"
                        })
                    }),
                "GlobalEvent executes without a Spite boss parent"
            );
            let callback = SpiteGlobalFireCallback::from_current_registration(
                &mut runtime,
                &scene.room,
                native,
                &proof,
                1,
                0,
                owner.owner_now().get(),
            )
            .map_err(debug)?;
            assert!(callback.current_for(&runtime));
            assert_eq!(
                callback.position(),
                MovementLocalPosition {
                    x: 33743,
                    y: 31628,
                    floor: 14
                }
            );
            let prepared = callback.prepare_current_write(&runtime).map_err(debug)?;
            let target = if reserve_target || expire {
                oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt::install_companion_policies(native, &mut runtime).map_err(debug)?;
                let key = "oteryn:creature.goshnar_s_spite";
                let policy = runtime.companion_policy(key).map_err(debug)?;
                let target = runtime
                    .admit_pinned_monster_lab_creature(
                        MovementLocalPosition {
                            x: 33742,
                            y: 31630,
                            floor: 14,
                        },
                        key,
                        policy.maximum_health,
                    )
                    .map_err(debug)?;
                runtime
                    .install_creature_policy(target, key)
                    .map_err(debug)?;
                assert!(
                    runtime
                        .source_creature_is_registered(target)
                        .map_err(debug)?
                );
                assert_eq!(
                    runtime.companion_snapshot(target).map_err(debug)?.health,
                    policy.maximum_health
                );
                assert!(prepared.current_for(&runtime));
                let snapshot = runtime.companion_snapshot(target).map_err(debug)?;
                let defense = runtime
                    .effective_creature_defense(&snapshot)
                    .map_err(debug)?;
                if reserve_target {
                    eprintln!("SRX3_REGISTERED_GLOBAL_CURRENT_NATIVE_RESERVATION_REACHED");
                    let reservation = runtime.reserve_source_global_action(&prepared, &[target]);
                    assert!(
                        reservation.is_ok(),
                        "real actor-free Global proof must reserve current native target: {reservation:?}"
                    );
                    let reservation = reservation.map_err(debug)?;
                    assert!(runtime.assert_actor_spell_unreserved(target).is_err());
                    runtime
                        .validate_source_global_action(&reservation, &prepared)
                        .map_err(debug)?;
                    runtime
                        .complete_spite_global_action(
                            &reservation,
                            &prepared,
                            Some((&snapshot, defense)),
                            false,
                        )
                        .map_err(debug)?;
                    assert_eq!(
                        runtime
                            .effective_creature_defense(&snapshot)
                            .map_err(debug)?,
                        defense
                    );
                    runtime
                        .assert_actor_spell_unreserved(target)
                        .map_err(debug)?;
                    assert!(
                        runtime
                            .validate_source_global_action(&reservation, &prepared)
                            .is_err()
                    );
                    assert!(
                        runtime
                            .complete_spite_global_action(
                                &reservation,
                                &prepared,
                                Some((&snapshot, defense)),
                                true,
                            )
                            .is_err(),
                        "a released reservation cannot publish a later duplicate defense change"
                    );
                    return Ok(());
                }
                Some((snapshot, defense))
            } else {
                None
            };
            drop(runtime);
            let receipt = owner
                .root
                .commit_spite_global_fire(
                    &permit,
                    owner.character,
                    owner.holder,
                    &callback,
                    &prepared,
                )
                .await
                .map_err(|error| {
                    format!("actual registered GlobalEvent physical boundary: {error:?}")
                })?;
            let replay_prepared = {
                let runtime = owner.runtime.lock().await;
                assert!(
                    prepared.current_for(&runtime),
                    "current scope/pins after known commit"
                );
                callback.prepare_current_write(&runtime).map_err(debug)?
            };
            let replay = owner
                .root
                .commit_spite_global_fire(
                    &permit,
                    owner.character,
                    owner.holder,
                    &callback,
                    &replay_prepared,
                )
                .await
                .map_err(debug)?;
            {
                let runtime = owner.runtime.lock().await;
                assert!(
                    replay_prepared.current_for(&runtime),
                    "current scope/pins after receipt replay"
                );
            }
            assert!(!receipt.removed);
            assert!(!replay.removed);
            assert_eq!(
                receipt.receipt.transaction_id,
                replay.receipt.transaction_id
            );
            assert!(replay.receipt.already_committed);
            let row:(i16,Option<Vec<u8>>,Option<String>,i32)=sqlx::query_as(
                "SELECT cause_kind,creature_placement,creature_definition_key,operation_count FROM game_spell_item_receipts WHERE transaction_id=encode($1,'hex')::uuid")
                .bind(receipt.receipt.transaction_id.as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(row, (4, None, None, 1));
            let count:i64=sqlx::query_scalar("SELECT count(*) FROM game_spell_item_lines WHERE transaction_id=encode($1,'hex')::uuid AND operation_kind=1 AND definition_production_key='oteryn:item.tibia.i33877'")
                .bind(receipt.receipt.transaction_id.as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(count, 1);
            if expire {
                // A later real MINT at the same anchor becomes the current
                // top-down Thing. The old callback retains a position, not its UUID.
                let (later, later_prepared) = {
                    let mut runtime = owner.runtime.lock().await;
                    let later = SpiteGlobalFireCallback::from_current_registration(
                        &mut runtime,
                        &scene.room,
                        native,
                        &proof,
                        2,
                        0,
                        owner.owner_now().get(),
                    )
                    .map_err(debug)?;
                    let prepared = later.prepare_current_write(&runtime).map_err(debug)?;
                    (later, prepared)
                };
                let later_receipt = owner
                    .root
                    .commit_spite_global_fire(
                        &permit,
                        owner.character,
                        owner.holder,
                        &later,
                        &later_prepared,
                    )
                    .await
                    .map_err(debug)?;
                let original_item: String = sqlx::query_scalar(
                    "SELECT item_instance_id::text FROM game_spell_item_lines WHERE transaction_id=encode($1,'hex')::uuid AND operation_kind=1",
                ).bind(receipt.receipt.transaction_id.as_slice()).fetch_one(&h.pool).await?;
                let later_item: String = sqlx::query_scalar(
                    "SELECT item_instance_id::text FROM game_spell_item_lines WHERE transaction_id=encode($1,'hex')::uuid AND operation_kind=1",
                ).bind(later_receipt.receipt.transaction_id.as_slice()).fetch_one(&h.pool).await?;
                assert_ne!(original_item, later_item);
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                let expiry = {
                    let mut runtime = owner.runtime.lock().await;
                    assert!(
                        owner.owner_now().get() >= callback.due_us(),
                        "real source execution clock crossed original5s deadline"
                    );
                    callback
                        .prepare_current_expiry(&mut runtime, &scene.room, owner.owner_now().get())
                        .map_err(debug)?
                };
                let (prepared, reservation) = {
                    let mut runtime = owner.runtime.lock().await;
                    let prepared = expiry.prepare_current_write(&runtime).map_err(debug)?;
                    let (snapshot, _) = target.as_ref().ok_or("actual expiry target")?;
                    let reservation = runtime
                        .reserve_source_global_action(&prepared, &[snapshot.actor])
                        .map_err(debug)?;
                    (prepared, reservation)
                };
                eprintln!("SRX3_REGISTERED_GLOBAL_ORIGINAL_FIVE_SECOND_EXPIRY_REACHED");
                let expired = owner
                    .root
                    .commit_spite_global_fire(
                        &permit,
                        owner.character,
                        owner.holder,
                        &expiry,
                        &prepared,
                    )
                    .await
                    .map_err(debug)?;
                assert!(expired.removed);
                {
                    let mut runtime = owner.runtime.lock().await;
                    let (snapshot, defense) = target.as_ref().ok_or("actual expiry target")?;
                    runtime
                        .complete_spite_global_action(
                            &reservation,
                            &prepared,
                            Some((snapshot, *defense)),
                            expired.removed,
                        )
                        .map_err(debug)?;
                    assert_eq!(
                        runtime
                            .effective_creature_defense(snapshot)
                            .map_err(debug)?,
                        defense + 10
                    );
                    assert_eq!(
                        runtime
                            .companion_snapshot(snapshot.actor)
                            .map_err(debug)?
                            .health,
                        snapshot.health
                    );
                }
                let removed: i32 = sqlx::query_scalar("SELECT operation_count FROM game_spell_item_receipts WHERE cause_kind=4 AND creature_occurrence LIKE '%:expire'")
                    .fetch_one(&h.pool).await?;
                assert_eq!(
                    removed, 1,
                    "actual same-tile current33877 is removed by the existing Item writer"
                );
                let removed_item: String = sqlx::query_scalar(
                    "SELECT item_instance_id::text FROM game_spell_item_lines WHERE transaction_id=encode($1,'hex')::uuid AND operation_kind=2",
                ).bind(expired.receipt.transaction_id.as_slice()).fetch_one(&h.pool).await?;
                assert_eq!(
                    removed_item, later_item,
                    "expiry removes the current matching Thing, not the originally minted UUID"
                );
                let replay = owner
                    .root
                    .commit_spite_global_fire(
                        &permit,
                        owner.character,
                        owner.holder,
                        &expiry,
                        &prepared,
                    )
                    .await
                    .map_err(debug)?;
                assert!(replay.receipt.already_committed);
                assert!(
                    replay.removed,
                    "the immutable receipt retains the original current-field outcome"
                );
                let original_still_current: bool = sqlx::query_scalar(
                    "SELECT EXISTS(SELECT 1 FROM game_item_ground_locations WHERE item_instance_id=$1::uuid)",
                ).bind(&original_item).fetch_one(&h.pool).await?;
                assert!(
                    original_still_current,
                    "receipt replay cannot remove the next current field at the retained position"
                );
                {
                    let mut runtime = owner.runtime.lock().await;
                    let (snapshot, defense) = target.as_ref().ok_or("actual expiry target")?;
                    assert!(
                        runtime
                            .complete_spite_global_action(
                                &reservation,
                                &prepared,
                                Some((snapshot, *defense)),
                                replay.removed,
                            )
                            .is_err()
                    );
                    assert_eq!(
                        runtime
                            .effective_creature_defense(snapshot)
                            .map_err(debug)?,
                        defense + 10
                    );
                }
            }
            Ok(())
        })
    });
    eprintln!(
        "SRX3_SPITE_GLOBAL_ACTUAL_POSTGRES elapsed={:?}",
        started.elapsed()
    );
    result
}

#[test]
fn srx3_spite_registered_global_qualifies_actual_published_world_without_private_alias()
-> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run("srx3-spite-global-world", || {
        let scene = super::srx3_soul_war_world_fixture::source_world(MovementLocalPosition {
            x: 33742,
            y: 31639,
            floor: 14,
        })?;
        let active = scene.controller.active().ok_or("actual World activation")?;
        let native = active
            .native_gameplay()
            .ok_or("actual native registered source closure")?;
        let world = active
            .qualify_published_world_context(&scene.room, &scene.pin)
            .ok_or("actual published World frame/pin proof")?;
        assert!(world.matches(&scene.room, &scene.pin));
        assert!(scene.room.pinned_activity_source_room().is_none());
        assert!(scene.room.source_world().is_none());
        assert!(native.qualifies_soul_war_callbacks(scene.pin.server_artifact_digest()));
        eprintln!("SRX3_REGISTERED_GLOBAL_ACTUAL_PUBLISHED_WORLD_QUALIFIER_REACHED");
        let proof = native.qualify_spite_global_event_in_published_world(&scene.room, &scene.pin, &world)
            .ok_or("registered source GlobalEvent must qualify real World without a private scope alias")?;
        assert_eq!(proof.interval_ms(), 14_000);
        assert_eq!(proof.expiry_ms(), 5_000);
        assert_eq!(proof.content(), scene.pin.server_artifact_digest());
        assert_ne!(proof.body_digest(), [0; 32]);
        Ok(())
    });
    eprintln!(
        "SRX3_SPITE_GLOBAL_PUBLISHED_WORLD elapsed={:?}",
        started.elapsed()
    );
    result
}

#[test]
fn srx3_spite_registered_world_owner_emits_one_selected_field_without_a_boss_after_fourteen_seconds_postgres()
-> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run("srx3-spite-global-owner", || {
        run(async |admin| {
            reset();
            let scene = super::srx3_soul_war_world_fixture::source_world(MovementLocalPosition {
                x: 33742,
                y: 31639,
                floor: 14,
            })?;
            let deps = scene.dependencies()?;
            let h =
                Harness::create_with_admission_seed(admin, "srx3_spite_global_owner", false, false)
                    .await?;
            let evidence = super::house_transfer::RecoverySource::start(&h).await?;
            super::house_transfer::prepare_owned_session(&h).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let mut owner = deps.owner(&h, &authority);
            owner.evidence = &evidence.evidence;
            owner.active_generation = scene.controller.active();
            owner.rooms =
                crate::gameplay_transport::SeamRooms::new(Some(&scene.room), Some(&scene.room));
            super::boss_activity::issue_content_and_reset(&h, &scene.pin).await?;
            let mut admitted = deps.admitted;
            prepare_player(&h, &owner, &mut admitted, 500).await?;
            let registered = owner.owner_now().get();
            assert!(
                owner
                    .register_world_spite_global(registered)
                    .await
                    .map_err(debug)?,
                "actual published World registered callback prerequisite"
            );
            {
                let runtime = owner.runtime.lock().await;
                assert!(
                    runtime
                        .source_registered_creature_census()
                        .map_err(debug)?
                        .is_empty(),
                    "the Global owner requires no boss or fake Creature parent"
                );
                let states = owner.states.spell_states.try_lock().map_err(debug)?;
                assert_eq!(
                    states.spite_global.next_due_us(),
                    Some(registered + 14_000_000)
                );
            }
            owner.drain_monster_melee().await;
            let before: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM game_spell_item_receipts WHERE cause_kind=4",
            )
            .fetch_one(&h.pool)
            .await?;
            assert_eq!(
                before, 0,
                "no first-Think or boss-birth shortcut before source14s"
            );
            tokio::time::sleep(std::time::Duration::from_secs(14)).await;
            assert!(owner.owner_now().get() >= registered + 14_000_000);
            eprintln!("SRX3_REGISTERED_WORLD_GLOBAL_ACTUAL_FOURTEEN_SECOND_OWNER_REACHED");
            owner.drain_monster_melee().await;
            let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_spell_item_receipts WHERE cause_kind=4 AND creature_occurrence LIKE '%:create' AND operation_count=1 AND creature_placement IS NULL").fetch_one(&h.pool).await?;
            assert_eq!(
                count, 1,
                "one actual source dispatch creates one field at its selected literal anchor without a boss"
            );
            owner.drain_monster_melee().await;
            let replay: i64 = sqlx::query_scalar("SELECT count(*) FROM game_spell_item_receipts WHERE cause_kind=4 AND creature_occurrence LIKE '%:create'").fetch_one(&h.pool).await?;
            assert_eq!(replay, 1, "one dispatch, no reroll or catchup batch");
            let expiry_due = owner
                .states
                .spell_states
                .try_lock()
                .map_err(debug)?
                .spite_global
                .next_due_us()
                .ok_or("retained original expiry")?;
            assert!(expiry_due > registered + 14_000_000);
            assert!(expiry_due <= owner.owner_now().get() + 5_000_000);
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            assert!(owner.owner_now().get() >= expiry_due);
            owner.drain_monster_melee().await;
            let expired: i64 = sqlx::query_scalar("SELECT count(*) FROM game_spell_item_receipts WHERE cause_kind=4 AND creature_occurrence LIKE '%:expire' AND operation_count=1 AND creature_placement IS NULL")
                .fetch_one(&h.pool).await?;
            assert_eq!(
                expired, 1,
                "the actual owner removes the current field after its retained five-second deadline without a boss"
            );
            owner.drain_monster_melee().await;
            let expired_replay: i64 = sqlx::query_scalar("SELECT count(*) FROM game_spell_item_receipts WHERE cause_kind=4 AND creature_occurrence LIKE '%:expire'")
                .fetch_one(&h.pool).await?;
            assert_eq!(
                expired_replay, 1,
                "completed expiry is not repeated by another owner pass"
            );
            Ok(())
        })
    });
    eprintln!(
        "SRX3_SPITE_GLOBAL_REAL_WORLD_OWNER elapsed={:?}",
        started.elapsed()
    );
    result
}
