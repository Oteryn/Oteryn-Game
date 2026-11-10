//! Actual GlobalEvent and Player STEP through the existing physical Item/Quest owners.
use super::fixture::{Harness, debug, run};
use super::srx3_activity_source_fixture::{SourceScene, install_scene, prepare_player};
use super::{TestResult, reset};
use crate::content::bosses::activity_template::PinnedActivitySourceRoom;
use crate::durability::creature_source_items::{SpiteGlobalFireCallback, SpiteGlobalFireOutcome};
use crate::foundation::MovementLocalPosition;

#[path = "srx3_spite_global/capacity.rs"]
mod capacity;

#[test]
fn srx3_spite_private_registration_uses_actual_ready_scope_before_entry_postgres() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run("srx3-spite-private-global", || {
        run(async |admin| {
            use crate::foundation::RuntimeScopeRefV1;
            use oteryn_simulation_determinism::SemanticTimeMicros;
            reset();
            let h = Harness::create_with_admission_seed(
                admin,
                "srx3_spite_private_global",
                false,
                false,
            )
            .await?;
            let evidence = super::house_transfer::RecoverySource::start(&h).await?;
            super::house_transfer::prepare_owned_session(&h).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let scene = SourceScene::new(
                PinnedActivitySourceRoom::GoshnarsSpite,
                MovementLocalPosition {
                    x: 33774,
                    y: 31634,
                    floor: 14,
                },
            )?;
            let deps = scene.dependencies()?;
            let mut owner = deps.owner(&h, &authority);
            owner.evidence = &evidence.evidence;
            install_scene(&mut owner, &scene, PinnedActivitySourceRoom::GoshnarsSpite)?;
            super::boss_activity::issue_content_and_reset(&h, &scene.pin).await?;
            let mut admitted = deps.admitted;
            prepare_player(&h, &owner, &mut admitted, 500).await?;
            let scope = super::activity_runtime::allocate_ready_instance(&h, 218).await?;
            let started = owner.owner_now().get();
            // Exercise the actual ready-runtime registration owner without
            // manufacturing a callback, accepting a lever or opening the gate.
            let ready = owner
                .ensure_pinned_source_runtime(
                    PinnedActivitySourceRoom::GoshnarsSpite,
                    Some((
                        "oteryn:creature.goshnar_s_spite",
                        MovementLocalPosition {
                            x: 33743,
                            y: 31632,
                            floor: 14,
                        },
                    )),
                    1024,
                )
                .await
                .map_err(debug)?;
            assert_eq!(ready.scope, scope);
            assert!(matches!(scope, RuntimeScopeRefV1::Instance { .. }));
            let view = owner.runtime_view(&ready);
            let first_due = {
                let runtime = view.runtime.lock().await;
                assert!(
                    runtime
                        .activity_participant_sessions()
                        .map_err(debug)?
                        .is_empty()
                );
                assert!(
                    runtime.spawn_point_creatures().is_empty(),
                    "prewarming cannot start the lever's XML respawn clock"
                );
                let states = view.states.spell_states.lock().await;
                assert!(
                    states.soul_war_callbacks.requires_committed_entry(),
                    "Spite's exact XML plan waits for the actual handoff even without a boss-specific Think owner"
                );
                states
                    .spite_global
                    .next_due_us()
                    .ok_or("production private Global registration")?
            };
            assert!(first_due >= started + 14_000_000);
            assert!(first_due <= owner.owner_now().get() + 14_000_000);
            let again = owner
                .ensure_pinned_source_runtime(
                    PinnedActivitySourceRoom::GoshnarsSpite,
                    Some((
                        "oteryn:creature.goshnar_s_spite",
                        MovementLocalPosition {
                            x: 33743,
                            y: 31632,
                            floor: 14,
                        },
                    )),
                    1024,
                )
                .await
                .map_err(debug)?;
            assert_eq!(again.scope, ready.scope);
            assert_eq!(
                again
                    .states
                    .spell_states
                    .lock()
                    .await
                    .spite_global
                    .next_due_us(),
                Some(first_due),
                "rediscovering the same ready owner cannot restart its independent Global clock"
            );

            // The existing test port supplies exact semantic owner turns; every
            // Item write below is still the real fenced native/SQL producer.
            view.drain_monster_melee_at(SemanticTimeMicros::from_micros(first_due - 1))
                .await;
            assert_eq!(
                sqlx::query_scalar::<_, i64>(
                    "SELECT count(*) FROM game_spell_item_receipts WHERE cause_kind=4"
                )
                .fetch_one(&h.pool)
                .await?,
                0,
                "no Global MINT before its original14s"
            );
            view.drain_monster_melee_at(SemanticTimeMicros::from_micros(first_due))
                .await;
            assert_eq!(sqlx::query_scalar::<_, i64>(
                "SELECT count(*) FROM game_spell_item_receipts WHERE cause_kind=4 AND creature_occurrence LIKE '%:create' AND operation_count=1"
            ).fetch_one(&h.pool).await?, 1, "one real private-scope MINT at the registered deadline");
            let expiry_due = view
                .states
                .spell_states
                .lock()
                .await
                .spite_global
                .next_due_us()
                .ok_or("original private expiry")?;
            assert_eq!(expiry_due, first_due + 5_000_000);
            view.drain_monster_melee_at(SemanticTimeMicros::from_micros(first_due))
                .await;
            assert_eq!(
                view.states
                    .spell_states
                    .lock()
                    .await
                    .spite_global
                    .next_due_us(),
                Some(expiry_due)
            );
            view.drain_monster_melee_at(SemanticTimeMicros::from_micros(expiry_due))
                .await;
            assert_eq!(sqlx::query_scalar::<_, i64>(
                "SELECT count(*) FROM game_spell_item_receipts WHERE cause_kind=4 AND creature_occurrence LIKE '%:expire' AND operation_count=1"
            ).fetch_one(&h.pool).await?, 1, "existing private owner expires the same current field once");
            assert!(
                view.runtime.lock().await.spawn_point_creatures().is_empty(),
                "Global ticks never substitute for committed XML activation"
            );
            Ok(())
        })
    })
}

#[test]
fn srx3_spite_actual_player_step_consumes_once_with_fifty_six_second_clock_postgres() -> TestResult
{
    spite_source_step_case(false)
}

#[test]
fn srx3_spite_cancelled_field_cleanup_retains_original_translation_postgres() -> TestResult {
    spite_source_step_case(true)
}

fn spite_source_step_case(cancel_during_cleanup: bool) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run("srx3-spite-source-step", move || {
        run(async move |admin| {
            use crate::durability::character_progression::CharacterProgressionError;
            use crate::durability::quest_state::{
                QuestTransitionOutcome, soul_war_source::SoulWarSourceAction,
            };
            use oteryn_gameplay_quests::quest::soul_war::source_items as rule;
            use oteryn_protocol_oteryn::world_spatial::{StepDirection, StepDisposition};
            reset();
            let h =
                Harness::create_with_admission_seed(admin, "srx3_spite_step", false, false).await?;
            let evidence = super::house_transfer::RecoverySource::start(&h).await?;
            super::house_transfer::prepare_owned_session(&h).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let scene = SourceScene::new(
                PinnedActivitySourceRoom::GoshnarsSpite,
                MovementLocalPosition {
                    x: 33743,
                    y: 31629,
                    floor: 14,
                },
            )?;
            let deps = scene.dependencies()?;
            let mut owner = deps.owner(&h, &authority);
            owner.evidence = &evidence.evidence;
            install_scene(&mut owner, &scene, PinnedActivitySourceRoom::GoshnarsSpite)?;
            super::boss_activity::issue_content_and_reset(&h, &scene.pin).await?;
            let mut admitted = deps.admitted;
            prepare_player(&h, &owner, &mut admitted, 500).await?;
            h.root.set_test_quest_clock(1_800_000_000).map_err(debug)?;
            let receipt_clock_before: i64 = sqlx::query_scalar(
                "SELECT floor(extract(epoch FROM statement_timestamp())*1000)::bigint",
            )
            .fetch_one(&h.pool)
            .await?;
            let actor = admitted.runtime_actor.ok_or("actual source Player")?;
            let native = scene
                .controller
                .active()
                .and_then(|active| active.native_gameplay())
                .ok_or("actual source generation")?;
            let registration = native
                .qualify_spite_global_event(&scene.room)
                .ok_or("real registered Spite source")?;
            let permit = owner
                .spell_lane_permit()
                .await
                .ok_or("physical Item lane")?;
            let (callback, prepared) = {
                let mut runtime = owner.runtime.lock().await;
                let callback = SpiteGlobalFireCallback::from_current_registration(
                    &mut runtime,
                    &scene.room,
                    native,
                    &registration,
                    1,
                    0,
                    owner.owner_now().get(),
                )
                .map_err(debug)?;
                let prepared = callback.prepare_current_write(&runtime).map_err(debug)?;
                (callback, prepared)
            };
            let SpiteGlobalFireOutcome::Committed(created) = owner
                .root
                .commit_spite_global_fire(
                    &permit,
                    owner.character,
                    owner.holder,
                    &callback,
                    &prepared,
                )
                .await
                .map_err(debug)?
            else {
                return Err("the Source STEP fixture must create its first physical field".into());
            };
            let first:String=sqlx::query_scalar("SELECT item_instance_id::text FROM game_spell_item_lines WHERE transaction_id=encode($1,'hex')::uuid AND operation_kind=1")
                .bind(created.receipt.transaction_id.as_slice()).fetch_one(&h.pool).await?;
            drop(permit);

            // A known unavailable precondition parks the real committed movement
            // before its Quest call. Resuming must use that retained selection,
            // without deriving a fresh event from a later standing observation.
            assert!(owner.quest_catalogue.is_none());
            if cancel_during_cleanup {
                use crate::gameplay_transport::spite_step::SPITE_STEP_CAPTURED_TEST_BARRIER;
                use std::future::{Future, poll_fn};
                use std::task::Poll;
                let captured = std::sync::Arc::new(tokio::sync::Notify::new());
                {
                    let mut movement = std::pin::pin!(SPITE_STEP_CAPTURED_TEST_BARRIER.scope(
                        std::sync::Arc::clone(&captured),
                        owner.ordinary_paced_step(
                            actor,
                            admitted.game_session_id,
                            StepDirection::North,
                            None,
                            None,
                        ),
                    ));
                    let mut entered = std::pin::pin!(captured.notified());
                    tokio::time::timeout(
                        std::time::Duration::from_secs(10),
                        poll_fn(|cx| {
                            if entered.as_mut().poll(cx).is_ready() {
                                return Poll::Ready(Ok(()));
                            }
                            if let Poll::Ready(result) = movement.as_mut().poll(cx) {
                                return Poll::Ready(Err(format!(
                                    "movement ended before retained cleanup barrier: {result:?}"
                                )));
                            }
                            Poll::Pending
                        }),
                    )
                    .await??;
                    // Dropping the real future cancels field transaction cleanup,
                    // after movement but before any Source STEP SQL/read began.
                }
                let original = owner
                    .retained_spite_movement_for_test()
                    .await
                    .ok_or("physical movement retained before field cleanup")?;
                assert_eq!(
                    original.position(),
                    MovementLocalPosition {
                        x: 33743,
                        y: 31628,
                        floor: 14
                    }
                );
                assert_eq!(original.revision(), original.last_translation_revision());
                {
                    let runtime = owner.runtime.lock().await;
                    assert_eq!(runtime.read_actor_position(actor).map_err(debug)?, original);
                    assert!(runtime.assert_actor_spell_unreserved(actor).is_err());
                }
                assert!(owner.retained_spite_step_for_test().await.is_none());
                // Observe the cancelled SQL holder's real return; no fixed sleep
                // or synthetic receipt substitutes for the retained source read.
                tokio::time::timeout(std::time::Duration::from_secs(10), async {
                    loop {
                        assert!(owner.spell_lane_permit().await.is_none());
                        if owner.retained_spite_step_for_test().await.is_some() {
                            break;
                        }
                        tokio::task::yield_now().await;
                    }
                })
                .await?;
                let (input, _) = owner
                    .retained_spite_step_for_test()
                    .await
                    .ok_or("same source after cancelled field cleanup")?;
                assert_eq!(input.context().movement_revision, original.revision());
            } else {
                let (moved, _) = owner
                    .ordinary_paced_step(
                        actor,
                        admitted.game_session_id,
                        StepDirection::North,
                        None,
                        None,
                    )
                    .await;
                assert_eq!(moved.disposition, StepDisposition::Moved);
            }
            let (input, fence) = owner
                .retained_spite_step_for_test()
                .await
                .ok_or("actual retained Source STEP")?;
            assert_eq!(
                input.item_instance_id(),
                sqlx::types::Uuid::parse_str(&first)?.into_bytes()
            );
            assert!(
                owner
                    .runtime
                    .lock()
                    .await
                    .assert_actor_spell_unreserved(actor)
                    .is_err()
            );
            let live:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_item_ground_locations WHERE item_instance_id=$1::uuid)").bind(&first).fetch_one(&h.pool).await?;
            assert!(live, "retention alone never removes the field");
            let lowered =
                oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
                    .map_err(debug)?;
            let catalogue = std::sync::Arc::new(lowered.catalogue().clone());
            // Change one context invariant while retaining the actual source,
            // session, Item, catalogue and movement. None is a fresh authority.
            let direct_permit = owner
                .spell_lane
                .acquire()
                .await
                .map_err(|_| "unresolved physical lane")?;
            for mutation in 0..4 {
                let mut wrong = input.context();
                match mutation {
                    0 => {
                        wrong.reset_epoch = wrong.reset_epoch.checked_add(1).ok_or("reset epoch")?
                    }
                    1 => {
                        wrong.movement_revision = wrong
                            .movement_revision
                            .checked_add(1)
                            .ok_or("movement revision")?
                    }
                    2 => wrong.actor_placement[0] ^= 1,
                    _ => {
                        *wrong
                            .placement_context
                            .first_mut()
                            .ok_or("native frame context")? ^= 1
                    }
                }
                let result = owner
                    .root
                    .commit_soul_war_source_action(
                        owner.character,
                        owner.holder,
                        fence,
                        SoulWarSourceAction::Step(input.clone()),
                        std::sync::Arc::clone(&catalogue),
                        Some((&direct_permit, wrong)),
                    )
                    .await;
                assert!(
                    matches!(result, Err(CharacterProgressionError::InvalidInput)),
                    "a changed source context cannot consume its current Item"
                );
            }
            drop(direct_permit);
            owner.quest_catalogue = Some(std::sync::Arc::clone(&catalogue));
            let permit = owner
                .spell_lane_permit()
                .await
                .ok_or("retained Source STEP resolves through sole writer")?;
            assert!(owner.retained_spite_step_for_test().await.is_none());
            owner
                .runtime
                .lock()
                .await
                .assert_actor_spell_unreserved(actor)
                .map_err(debug)?;
            let row:(i64,i64)=sqlx::query_as("SELECT (SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2),(SELECT count(*) FROM game_character_quest_receipts WHERE character_id=encode($1,'hex')::uuid AND cause_kind='source_step')")
                .bind(fence.character_id.as_bytes().as_slice()).bind(rule::SPITE_FIRE).fetch_one(&h.pool).await?;
            assert_eq!(row, (1_800_000_000, 1));
            let retired:(i16,i64,bool)=sqlx::query_as("SELECT lifecycle,quantity,EXISTS(SELECT 1 FROM game_item_ground_locations g WHERE g.item_instance_id=i.item_instance_id) FROM game_item_instances i WHERE item_instance_id=$1::uuid")
                .bind(&first).fetch_one(&h.pool).await?;
            assert_eq!(retired, (2, 0, false));

            let (later, later_write) = {
                let mut runtime = owner.runtime.lock().await;
                let later = SpiteGlobalFireCallback::from_current_registration(
                    &mut runtime,
                    &scene.room,
                    native,
                    &registration,
                    2,
                    0,
                    owner.owner_now().get(),
                )
                .map_err(debug)?;
                let write = later.prepare_current_write(&runtime).map_err(debug)?;
                (later, write)
            };
            let SpiteGlobalFireOutcome::Committed(later_receipt) = owner
                .root
                .commit_spite_global_fire(
                    &permit,
                    owner.character,
                    owner.holder,
                    &later,
                    &later_write,
                )
                .await
                .map_err(debug)?
            else {
                return Err("the Source STEP replay fixture must create the later field".into());
            };
            let later_item:String=sqlx::query_scalar("SELECT item_instance_id::text FROM game_spell_item_lines WHERE transaction_id=encode($1,'hex')::uuid AND operation_kind=1")
                .bind(later_receipt.receipt.transaction_id.as_slice()).fetch_one(&h.pool).await?;
            let replay = owner
                .root
                .commit_soul_war_source_action(
                    owner.character,
                    owner.holder,
                    fence,
                    SoulWarSourceAction::Step(input.clone()),
                    std::sync::Arc::clone(&catalogue),
                    Some((&permit, input.context())),
                )
                .await
                .map_err(debug)?;
            assert!(matches!(
                replay,
                QuestTransitionOutcome::AlreadyCommitted(_)
            ));
            let later_live:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_item_ground_locations WHERE item_instance_id=$1::uuid)").bind(&later_item).fetch_one(&h.pool).await?;
            assert!(
                later_live,
                "immutable replay never selects a later current field"
            );
            let bare = owner
                .root
                .commit_character_quest_transition(
                    owner.character,
                    owner.holder,
                    fence,
                    SoulWarSourceAction::Step(input.clone())
                        .request(&fence)
                        .map_err(debug)?,
                    std::sync::Arc::clone(&catalogue),
                )
                .await;
            assert!(
                matches!(bare, Err(CharacterProgressionError::InvalidInput)),
                "a bare source cause is not the sealed Item participant"
            );

            let shape:bool=sqlx::query_scalar("SELECT game_spite_source_step_receipt_shape(r) FROM game_character_quest_receipts r WHERE character_id=encode($1,'hex')::uuid AND cause_kind='source_step'")
                .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
            assert!(shape);
            for patch in [
                serde_json::json!({"cause_ordinal":1}),
                serde_json::json!({"cause_id":sqlx::types::Uuid::from_bytes(*fence.game_session_id.as_bytes()).to_string()}),
                serde_json::json!({"completes":true}),
                serde_json::json!({"track_keys":[rule::MAW]}),
                serde_json::json!({"values_before":[1_799_999_945]}),
            ] {
                let accepted:bool=sqlx::query_scalar("SELECT game_spite_source_step_receipt_shape(jsonb_populate_record(NULL::game_character_quest_receipts,to_jsonb(r)||$2::jsonb)) FROM game_character_quest_receipts r WHERE character_id=encode($1,'hex')::uuid AND cause_kind='source_step'")
                    .bind(fence.character_id.as_bytes().as_slice()).bind(patch.clone()).fetch_one(&h.pool).await?;
                assert!(!accepted, "one invalid source receipt invariant: {patch}");
            }
            let clock_proof: (bool, serde_json::Value, i64, i64, bool) = sqlx::query_as(
                "SELECT game_spite_source_step_consumption_matches(c,r),convert_from(c.proof,'UTF8')::jsonb->'quest_clock_seconds',r.committed_at,floor(extract(epoch FROM statement_timestamp())*1000)::bigint,c.proof_sha256=sha256(c.proof) FROM game_quest_callback_consumptions c JOIN game_character_quest_receipts r USING(character_id,cause_id,cause_ordinal,transition_key) WHERE c.item_instance_id=$1::uuid",
            )
            .bind(&first)
            .fetch_one(&h.pool)
            .await?;
            assert!(
                clock_proof.0,
                "the consumed field proves the exact quest clock"
            );
            assert_eq!(clock_proof.1, serde_json::json!(1_800_000_000));
            assert!(
                (receipt_clock_before..=clock_proof.3).contains(&clock_proof.2),
                "receipt time remains database wall time in milliseconds"
            );
            assert!(clock_proof.4, "the immutable proof hash covers its clock");
            let changed_clock: (bool, bool) = sqlx::query_as(
                "SELECT game_spite_source_step_receipt_shape(jsonb_populate_record(r,$2::jsonb)),game_spite_source_step_consumption_matches(c,jsonb_populate_record(r,$2::jsonb)) FROM game_quest_callback_consumptions c JOIN game_character_quest_receipts r USING(character_id,cause_id,cause_ordinal,transition_key) WHERE c.item_instance_id=$1::uuid",
            )
            .bind(&first)
            .bind(serde_json::json!({"values_after":[1_800_000_001]}))
            .fetch_one(&h.pool)
            .await?;
            assert_eq!(
                changed_clock,
                (true, false),
                "a structurally valid receipt still needs the sampled quest clock"
            );
            for patch in [
                serde_json::json!({}),
                serde_json::json!({"quest_clock_seconds":null}),
                serde_json::json!({"quest_clock_seconds":"1800000000"}),
                serde_json::json!({"quest_clock_seconds":[1_800_000_000]}),
                serde_json::json!({"quest_clock_seconds":1_800_000_001}),
                serde_json::json!({"quest_clock_seconds":1_800_000_000_000_i64}),
            ] {
                let accepted: bool = sqlx::query_scalar(
                    "SELECT game_spite_source_step_consumption_matches(jsonb_populate_record(c,jsonb_build_object('proof',convert_to(p.proof::text,'UTF8'),'proof_sha256',sha256(convert_to(p.proof::text,'UTF8')))),r) FROM game_quest_callback_consumptions c JOIN game_character_quest_receipts r USING(character_id,cause_id,cause_ordinal,transition_key) CROSS JOIN LATERAL (SELECT (convert_from(c.proof,'UTF8')::jsonb-'quest_clock_seconds')||$2::jsonb AS proof) p WHERE c.item_instance_id=$1::uuid",
                )
                .bind(&first)
                .bind(patch.clone())
                .fetch_one(&h.pool)
                .await?;
                assert!(
                    !accepted,
                    "one invalid sampled clock with a matching hash: {patch}"
                );
            }
            drop(permit);
            for (seconds, expected) in [(55, 1_i64), (1, 2_i64)] {
                for direction in [StepDirection::South, StepDirection::North] {
                    if direction == StepDirection::North {
                        h.root.advance_test_quest_clock(seconds).map_err(debug)?;
                    }
                    owner.clock_origin = owner
                        .clock_origin
                        .checked_sub(std::time::Duration::from_millis(500))
                        .ok_or("monotonic step pace")?;
                    let (moved, _) = owner
                        .ordinary_paced_step(actor, admitted.game_session_id, direction, None, None)
                        .await;
                    assert_eq!(moved.disposition, StepDisposition::Moved);
                }
                let count:i64=sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts WHERE character_id=encode($1,'hex')::uuid AND cause_kind='source_step'")
                    .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
                assert_eq!(count, expected);
                let consumed:i64=sqlx::query_scalar("SELECT count(*) FROM game_quest_callback_consumptions WHERE character_id=encode($1,'hex')::uuid AND transition_key='oteryn:quest-transition/soul-war/source-items/spite-fire'")
                    .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
                assert_eq!(
                    consumed, expected,
                    "a cooldown refusal leaves no consumption row"
                );
                let clock:i64=sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2")
                    .bind(fence.character_id.as_bytes().as_slice()).bind(rule::SPITE_FIRE).fetch_one(&h.pool).await?;
                assert_eq!(
                    clock,
                    if seconds == 55 {
                        1_800_000_000
                    } else {
                        1_800_000_056
                    }
                );
                let live:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_item_ground_locations WHERE item_instance_id=$1::uuid)").bind(&later_item).fetch_one(&h.pool).await?;
                assert_eq!(
                    live,
                    seconds == 55,
                    "cooldown refusal preserves the same field; exactly 56 seconds admits it"
                );
            }
            let last:i64=sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2")
                .bind(fence.character_id.as_bytes().as_slice()).bind(rule::SPITE_FIRE).fetch_one(&h.pool).await?;
            assert_eq!(last, 1_800_000_056);
            Ok(())
        })
    })
}

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
            let SpiteGlobalFireOutcome::Committed(receipt) = owner
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
                })?
            else {
                return Err("the original creation must commit one physical field".into());
            };
            let replay_prepared = {
                let runtime = owner.runtime.lock().await;
                assert!(
                    prepared.current_for(&runtime),
                    "current scope/pins after known commit"
                );
                callback.prepare_current_write(&runtime).map_err(debug)?
            };
            let SpiteGlobalFireOutcome::Committed(replay) = owner
                .root
                .commit_spite_global_fire(
                    &permit,
                    owner.character,
                    owner.holder,
                    &callback,
                    &replay_prepared,
                )
                .await
                .map_err(debug)?
            else {
                return Err("creation replay must retain its physical receipt".into());
            };
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
                let SpiteGlobalFireOutcome::Committed(later_receipt) = owner
                    .root
                    .commit_spite_global_fire(
                        &permit,
                        owner.character,
                        owner.holder,
                        &later,
                        &later_prepared,
                    )
                    .await
                    .map_err(debug)?
                else {
                    return Err("the later creation must commit its own field".into());
                };
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
                let SpiteGlobalFireOutcome::Committed(expired) = owner
                    .root
                    .commit_spite_global_fire(
                        &permit,
                        owner.character,
                        owner.holder,
                        &expiry,
                        &prepared,
                    )
                    .await
                    .map_err(debug)?
                else {
                    return Err("expiry must retain its physical receipt".into());
                };
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
                let SpiteGlobalFireOutcome::Committed(replay) = owner
                    .root
                    .commit_spite_global_fire(
                        &permit,
                        owner.character,
                        owner.holder,
                        &expiry,
                        &prepared,
                    )
                    .await
                    .map_err(debug)?
                else {
                    return Err("expiry replay must retain its physical receipt".into());
                };
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
