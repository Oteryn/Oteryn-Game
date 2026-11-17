//! STAGPOS: the positional Order of the Stag callbacks through the real USE and
//! STEP callers, the fenced quest writer and the reward MINT owner.
//!
//! Crystal 201189f871d23597b82caf88fd611454a2ddefb8:
//! data-global/scripts/quests/the_order_of_the_stag/bounac_cathedral.lua:134-153
//! (frozen chest) and isle_of_ada.lua:56-80 (mines area).
use super::*;
use oteryn_protocol_oteryn::world_spatial::{StepDirection, StepDisposition};

const QUESTLINE: &str =
    "oteryn:quest-progress/crystalserver/quest/u15_20/the_order_of_the_stag/questline";
const WRIT: &str = "oteryn:item.tibia.i52586";
const MINES_MESSAGE: &str =
    "Your questlog has been updated. You have found your way through the mines.";

#[derive(Clone, Copy, PartialEq, Eq)]
enum Act {
    Use,
    StepEast,
}

#[test]
fn stag_certificate_use_grants_writ_once_and_sets_stage_two() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::super::kill_reward_e2e::scenario_thread::run("stag-certificate-use", || {
        let callback = callback("order-of-the-stag-certificate")?;
        let chest = callback.selector.position.ok_or("chest")?;
        assert_eq!(chest, position((32401, 32469, -5)));
        for stage in [None, Some(0), Some(1), Some(2), Some(3)] {
            exercise(&callback, chest, Act::Use, stage)?;
        }
        Ok(())
    })
}

#[test]
fn stag_mines_step_in_sets_stage_five_only_inside_the_area() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::super::kill_reward_e2e::scenario_thread::run("stag-mines-step", || {
        let callback = callback("order-of-the-stag-mines")?;
        for (tile, stage) in [
            ((33870, 32190, -7), Some(4)),
            ((33877, 32195, -6), Some(4)),
            // An excluded ramp and a tile just outside the rectangle.
            ((33873, 32192, -7), Some(4)),
            ((33878, 32190, -7), Some(4)),
            ((33870, 32190, -7), Some(3)),
            ((33870, 32190, -7), Some(5)),
            ((33870, 32190, -7), None),
        ] {
            exercise(&callback, position(tile), Act::StepEast, stage)?;
        }
        Ok(())
    })
}

fn position((x, y, floor): (u16, u16, i8)) -> Position {
    Position { x, y, floor }
}

async fn writs(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT coalesce(sum(quantity),0)::bigint FROM game_item_instances \
         WHERE definition_production_key=$1 AND lifecycle=1",
    )
    .bind(WRIT)
    .fetch_one(&h.pool)
    .await?)
}

async fn receipts(h: &Harness) -> TestResult<i64> {
    Ok(
        sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts")
            .fetch_one(&h.pool)
            .await?,
    )
}

#[allow(
    clippy::too_many_lines,
    reason = "one literal session: admission, stage seed, command and every durable check"
)]
fn exercise(callback: &Callback, trigger: Position, act: Act, stage: Option<i64>) -> TestResult {
    let started = std::time::Instant::now();
    let world = world_with_cells(callback, trigger, &[])?;
    let callback = callback.clone();
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
            super::super::death_ml_loss_composed::production_generation_with_pin(
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
        let embedded = oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
            .map_err(debug)?
            .catalogue()
            .clone();
        let track = embedded.track(QUESTLINE).ok_or("Stag questline")?.clone();
        owner.quest_catalogue = Some(std::sync::Arc::new(embedded.clone()));
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
        sqlx::query("INSERT INTO game_control_scope_grants VALUES (session_user, encode($1,'hex')::uuid, encode($2,'hex')::uuid, 4)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid, encode($2,'hex')::uuid, 1, NULL, $3, $4, $5)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).bind(generation.identity().server_artifact_digest().as_slice())
            .bind(generation.identity().client_artifact_digest().as_slice()).bind(activated.frame_binding().digest().as_slice()).execute(&h.pool).await?;
        let fresh = super::super::fresh_fixture::fresh::Source::new(0)?.request()?;
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
        owner
            .states
            .chat
            .admit(admitted.game_session_id, "Traveller", owner.owner_now());
        if let Some(stage) = stage {
            use oteryn_gameplay_quests::quest::{
                QuestComparison, QuestEffect, QuestEffectKind, QuestTransition,
            };
            let seed = "oteryn:quest-transition/fixture/stagpos-stage";
            let catalogue = std::sync::Arc::new(
                embedded
                    .with_fixture_transition(QuestTransition {
                        world_change: None,
                        key: seed.into(),
                        quest: callback.quest.clone(),
                        effects: vec![QuestEffect {
                            track: QUESTLINE.into(),
                            from: QuestComparison::Between(track.min, track.max),
                            effect: QuestEffectKind::Set(stage),
                        }],
                        completes: false,
                        experience: None,
                    })
                    .map_err(debug)?,
            );
            let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
            let seeded = slot
                .commit_quest_transition(
                    owner.root,
                    owner.character,
                    owner.holder,
                    fence,
                    crate::durability::quest_state::QuestTransitionRequest {
                        transition_key: seed.into(),
                        cause: crate::durability::quest_state::QuestCause::Command(
                            crate::foundation::CommandRef::new(
                                admitted.game_session_id,
                                crate::foundation::CommandId::new(900).map_err(debug)?,
                            ),
                        ),
                    },
                    catalogue.clone(),
                )
                .await
                .map_err(debug)?;
            assert!(matches!(
                seeded,
                crate::durability::quest_state::QuestTransitionOutcome::Committed(_)
            ));
            drop(slot);
            owner.quest_catalogue = Some(catalogue);
        }
        let stage_of = async |owner: &crate::gameplay_transport::ComposedFreshAdmission<
            '_,
            '_,
            '_,
        >|
               -> TestResult<i64> {
            Ok(owner
                .root
                .read_character_quest_state(&authority, fence.character_id)
                .await
                .map_err(debug)?
                .tracks()
                .get(QUESTLINE)
                .copied()
                .unwrap_or(track.initial))
        };
        let before = stage_of(&owner).await?;
        assert_eq!(before, stage.unwrap_or(track.initial));
        let writs_before = writs(&h).await?;
        let receipts_before = receipts(&h).await?;
        let allowed;
        let expected_message;
        match act {
            Act::Use => {
                allowed = stage == Some(1);
                expected_message = callback.success_message.clone();
                let key = oteryn_world_bundle::bundle::placement_key(
                    trigger.floor,
                    trigger.x,
                    trigger.y,
                    0,
                )
                .ok_or("key")?;
                let mut placement = owner
                    .bundle_map()
                    .ok_or("map")?
                    .overlay
                    .base()
                    .digest()
                    .to_vec();
                placement.extend_from_slice(&key.to_be_bytes());
                let payload =
                    oteryn_protocol_oteryn::world_object::encode_use_intent(&WorldObjectTarget {
                        placement,
                        expected_revision: 0,
                    })
                    .map_err(debug)?;
                // The same command twice is one durable claim; a fresh command
                // afterwards finds stage 2 and is refused.
                for (command, expected) in [
                    (1, allowed),
                    (1, allowed),
                    (2, false),
                ] {
                    let reply = wire(&owner, &mut admitted, command, 2, payload.clone()).await?;
                    assert_eq!(
                        oteryn_protocol_oteryn::world_object::decode_use_result(&reply)
                            .map_err(debug)?,
                        if expected {
                            UseDisposition::Committed
                        } else {
                            UseDisposition::Rejected
                        },
                        "{} stage={stage:?} command={command}",
                        callback.key
                    );
                    if !allowed {
                        break;
                    }
                }
                assert_eq!(writs(&h).await?, writs_before + i64::from(allowed));
                assert_eq!(
                    stage_of(&owner).await?,
                    if allowed { 2 } else { before }
                );
            }
            Act::StepEast => {
                allowed = stage == Some(4) && callback.selector.matches_position(trigger);
                expected_message = Some(MINES_MESSAGE.to_owned());
                owner.clock_origin = owner
                    .clock_origin
                    .checked_sub(std::time::Duration::from_secs(2))
                    .ok_or("clock")?;
                let reply = wire(
                    &owner,
                    &mut admitted,
                    1,
                    1,
                    oteryn_protocol_oteryn::world_spatial::encode_step_intent(
                        StepDirection::East,
                    ),
                )
                .await?;
                assert_eq!(
                    oteryn_protocol_oteryn::world_spatial::decode_step_result(&reply)
                        .map_err(debug)?,
                    StepDisposition::Moved
                );
                assert_eq!(
                    owner
                        .runtime
                        .lock()
                        .await
                        .read_actor_position(deps.actor)
                        .map_err(debug)?
                        .position(),
                    local(trigger)
                );
                assert_eq!(writs(&h).await?, writs_before);
                assert_eq!(
                    stage_of(&owner).await?,
                    if allowed { 5 } else { before },
                    "step {trigger:?} stage={stage:?}"
                );
            }
        }
        assert_eq!(receipts(&h).await?, receipts_before + i64::from(allowed));
        let messages = owner.states.chat.drain(admitted.game_session_id);
        if let Some(text) = expected_message {
            assert_eq!(
                messages
                    .iter()
                    .filter(|line| matches!(line,
                        oteryn_protocol_oteryn::chat::ChatLine::Private { text: line, .. }
                            if *line == text))
                    .count(),
                usize::from(allowed),
                "{} feedback stage={stage:?}",
                callback.key
            );
        }
        eprintln!(
            "STAGPOS_POSTGRES_CASE_COMPLETED {} at={trigger:?} stage={stage:?} allowed={allowed} duration={:?}",
            callback.key,
            started.elapsed()
        );
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}
