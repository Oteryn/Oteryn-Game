//! Real native STEP, credited deaths and survey USE complete personal journals once.
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use super::engworldchg_journals_fixtures::{kill_actor, live_creatures, scene, step, use_marker, value};

use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::content::channel_content_pin::ChannelContentPin;
use crate::foundation::ChannelRuntimeV1;
use crate::gameplay_transport::connection::{FirstEntryOutcome, FreshAdmissionAuthority};

use oteryn_gameplay_quests::quest::world_callbacks::Position;
use oteryn_protocol_oteryn::world_spatial::StepDirection;

#[test]
fn engworldchg_fury_native_admission_completion_egress_postgres() -> TestResult {
    scenario("fury_gates", "entry", "seal", "fury", 10)
}
#[test]
fn engworldchg_fire_goal_achievement_atomic_postgres() -> TestResult {
    scenario("fire_from_the_earth", "volcano", "volcano", "dragon", 50)
}
#[test]
fn engworldchg_grimvale_personal_native_journal_postgres() -> TestResult {
    scenario("grimvale", "island", "survey", "werewolf", 10)
}
#[test]
fn engworldchg_kingsday_native_npc_journal_postgres() -> TestResult {
    scenario("kingsday", "royal-visit", "royal-visit", "", 0)
}
#[test]
fn engworldchg_insectoid_counter_completion_postgres() -> TestResult {
    scenario(
        "insectoid_invasion",
        "incursion",
        "incursion",
        "insectoid_scout",
        10,
    )
}
#[test]
fn engworldchg_outpost_personal_native_journal_postgres() -> TestResult {
    scenario("hive_outpost", "outpost", "survey", "insectoid_worker", 20)
}

fn scenario(
    program: &'static str,
    entry_name: &'static str,
    final_name: &'static str,
    creature: &'static str,
    goal: i64,
) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("engworldchg-journal", move || {
        let callbacks = oteryn_gameplay_quests::quest::world_callbacks::load()?;
        let find = |name| {
            callbacks
                .iter()
                .find(|c| c.key == format!("oteryn:quest-callback/worldchg-m31/{program}/{name}"))
                .cloned()
                .ok_or("personal callback")
        };
        let entry_callback = find(entry_name)?;
        let final_callback = find(final_name)?;
        let entry = entry_callback.selector.position.ok_or("entry")?;
        let world = scene(&entry_callback, &final_callback)?;
        run(async move |admin| {
            let started = std::time::Instant::now();
            let h = Harness::create_with_admission_seed(admin, "engworldchg_journal", false, false)
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
            let origin = super::quest_world_callbacks_fixtures::local(
                entry_callback
                    .effects
                    .encounter
                    .as_ref()
                    .filter(|_| {
                        entry_callback.trigger
                            == oteryn_gameplay_quests::quest::world_callbacks::Trigger::Use
                    })
                    .and_then(|encounter| encounter.players.first())
                    .map_or(
                        Position {
                            x: entry.x + 1,
                            ..entry
                        },
                        |member| member.from,
                    ),
            );
            let room = super::rust_lottery_fixtures::bundle_action_room(
                activated.qualified_room(),
                &world,
                deps.admitted.world_id,
                &cells,
            )?;
            let bundle_frame = room.frame_binding().digest();
            let pin = ChannelContentPin::from_activation(
                deps.admitted.world_id,
                1,
                generation.identity().server_artifact_digest(),
                generation.identity().client_artifact_digest(),
                bundle_frame,
                room.map_revision_digest(),
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
                64,
                pin,
            )
            .map_err(debug)?;
            // This generic bundle view has no qualified native onDeath source room.
            runtime.use_classic_creature_deaths().map_err(debug)?;
            let reservation = runtime
                .reserve_fresh_session(deps.admitted.game_session_id)
                .map_err(debug)?;
            deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
            runtime
                .initialize_pinned_test_position(deps.actor, origin)
                .map_err(debug)?;
            deps.runtime = tokio::sync::Mutex::new(runtime);
            deps.combat_room = Some(room);
            let (consumer, clock, activation, _) =
                crate::gameplay_transport::spell_premium_coordinator::purchase_fixture::current(
                    id(40),
                );
            h.root
                .latch_premium_activation(
                    &activation.activation_id,
                    activation.switch_over_us,
                    activation.switch_over_us + 60_000_000,
                )
                .await
                .map_err(debug)?;
            let premium =
                crate::gameplay_transport::spell_premium_coordinator::SpellPremiumCoordinator::new(
                    consumer,
                    Some(clock),
                    Some(activation),
                    Some(h.root.clone()),
                );
            let mut owner = deps.owner(&h, &authority);
            owner.premium_coordinator = Some(&premium);
            owner.admit_premium(id(40), deps.admitted.game_session_id);
            owner.active_generation = Some(generation);
            owner.movement_cells = (&cells).into();
            let lowered =
                oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
                    .map_err(debug)?;
            owner.quest_catalogue = Some(std::sync::Arc::new(lowered.catalogue().clone()));
            owner.quest_log_content = Some(
                crate::gameplay_transport::quest_log::QuestLogLines::new(
                    oteryn_gameplay_quests::quest::log::loader::load_embedded_quest_log(&lowered)?
                        .content,
                )
                .ok_or("journal bounds")?,
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
            .bind(bundle_frame.as_slice()).execute(&h.pool).await?;
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
            native
                .install_companion_policies(&mut *owner.runtime.lock().await)
                .map_err(debug)?;
            let phase = format!("oteryn:quest-progress/world-change-journal/{program}/phase");
            let kills = format!("oteryn:quest-progress/world-change-journal/{program}/kills");
            let mut journal_command_override = None;
            if program == "fury_gates" {
                journal_command_override = Some(
                    super::engworldchg_fury::exercise(
                        &mut owner,
                        &mut admitted,
                        &h,
                        &authority,
                        &entry_callback,
                        &final_callback,
                    )
                    .await?,
                );
            } else if program == "fire_from_the_earth" {
                journal_command_override = Some(
                    super::engworldchg_fire::exercise(
                        &mut owner,
                        &mut admitted,
                        &h,
                        &authority,
                        &entry_callback,
                    )
                    .await?,
                );
            } else if program == "insectoid_invasion" {
                exercise_incursion(&mut owner, &mut admitted, &h, &authority).await?;
            } else if program == "kingsday" {
                super::engworldchg_kingsday::exercise(&mut owner, &mut admitted, &h, &authority)
                    .await?;
            } else {
                // Before entry, actual source-area deaths have no active personal predecessor.
                super::qengine_kill_stories_fixtures::death(
                    &owner,
                    &admitted,
                    &format!("oteryn:creature.{creature}"),
                    21000,
                )
                .await?;
                assert_eq!(value(&h, &kills).await?, 0);
                owner.premium_coordinator = None;
                step(&owner, &mut admitted, 1, StepDirection::West).await?;
                assert_eq!(
                    value(&h, &phase).await?,
                    0,
                    "an absent native Premium authority cannot open the personal hunt"
                );
                assert!(
                    live_creatures(&owner, &format!("oteryn:creature.{creature}"))
                        .await?
                        .is_empty(),
                    "preflight cannot publish actors before a fenced journal receipt"
                );
                step(&owner, &mut admitted, 2, StepDirection::East).await?;
                owner.premium_coordinator = Some(&premium);
                step(&owner, &mut admitted, 3, StepDirection::West).await?;
                assert_eq!(value(&h, &phase).await?, 1);
                assert_eq!(
                    use_marker(&owner, &mut admitted, &final_callback, 4).await?,
                    oteryn_protocol_oteryn::world_object::UseDisposition::Rejected
                );
                if program == "grimvale" {
                    for wave in 0..2_u64 {
                        let cohort = live_creatures(&owner, "oteryn:creature.werewolf").await?;
                        assert_eq!(
                            cohort.len(),
                            5,
                            "the real admitted STEP must supply the selected island encounter"
                        );
                        let survivor = if wave == 0 {
                            let target = cohort[0];
                            let mut runtime = owner.runtime.lock().await;
                            let from = runtime
                                .read_actor_position(target)
                                .map_err(debug)?
                                .position();
                            let moved = runtime
                                .commit_quest_platform_displacement(
                                    from,
                                    origin,
                                    |_, _, _| true,
                                    || true,
                                )
                                .map_err(debug)?;
                            assert_eq!(moved.len(), 1);
                            Some(target)
                        } else {
                            None
                        };
                        for (n, target) in cohort.into_iter().enumerate() {
                            if Some(target) == survivor {
                                continue;
                            }
                            let ordinal = if wave == 0 {
                                n.checked_sub(1).ok_or("cohort order")?
                            } else {
                                n
                            };
                            kill_actor(
                                &owner,
                                &admitted,
                                target,
                                22000 + wave * 5 + u64::try_from(ordinal)?,
                            )
                            .await?;
                        }
                        if let Some(survivor) = survivor {
                            assert_eq!(value(&h, &kills).await?, 4);
                            step(&owner, &mut admitted, 5, StepDirection::North).await?;
                            assert_eq!(
                                value(
                                    &h,
                                    "oteryn:quest-progress/world-change-journal/grimvale/waves"
                                )
                                .await?,
                                1,
                                "a live admitted actor outside the room still owns its wave"
                            );
                            assert_eq!(
                                live_creatures(&owner, "oteryn:creature.werewolf").await?,
                                vec![survivor]
                            );
                            kill_actor(&owner, &admitted, survivor, 22004).await?;
                            assert_eq!(value(&h, &kills).await?, 5);
                            step(&owner, &mut admitted, 6, StepDirection::East).await?;
                            step(&owner, &mut admitted, 7, StepDirection::West).await?;
                        }
                    }
                } else {
                    for n in 0..goal {
                        super::qengine_kill_stories_fixtures::death(
                            &owner,
                            &admitted,
                            &format!("oteryn:creature.{creature}"),
                            22000 + u64::try_from(n)?,
                        )
                        .await?;
                    }
                }
                assert_eq!(value(&h, &kills).await?, goal);
                if program == "grimvale" {
                    step(&owner, &mut admitted, 8, StepDirection::North).await?;
                    assert!(
                        live_creatures(&owner, "oteryn:creature.werewolf")
                            .await?
                            .is_empty(),
                        "a finished hunt cannot emit another group"
                    );
                    assert_eq!(
                        value(
                            &h,
                            "oteryn:quest-progress/world-change-journal/grimvale/waves"
                        )
                        .await?,
                        2
                    );
                }
                let completion_command = if program == "grimvale" { 9 } else { 5 };
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                assert_eq!(
                    use_marker(&owner, &mut admitted, &final_callback, completion_command).await?,
                    oteryn_protocol_oteryn::world_object::UseDisposition::Committed
                );
                assert_eq!(value(&h, &phase).await?, 4);
                let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts WHERE quest_key=$1 AND completes")
                .bind(&final_callback.quest).fetch_one(&h.pool).await?;
                assert_eq!(receipts, 1);
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                assert_eq!(
                    use_marker(
                        &owner,
                        &mut admitted,
                        &final_callback,
                        completion_command + 1
                    )
                    .await?,
                    oteryn_protocol_oteryn::world_object::UseDisposition::Rejected
                );
                let loaded = h
                    .root
                    .read_character_quest_state(&authority, fence.character_id)
                    .await
                    .map_err(debug)?;
                assert_eq!(loaded.tracks().get(&phase), Some(&4));
                assert_eq!(loaded.tracks().get(&kills), Some(&goal));
            }
            let journal_command = journal_command_override.unwrap_or(match program {
                "grimvale" => 11,
                "hive_outpost" => 7,
                "insectoid_invasion" => 10,
                _ => 3,
            });
            prove_journal_wire(
                &owner,
                admitted,
                &entry_callback.quest,
                (goal > 0).then_some(goal),
                journal_command,
                program != "kingsday",
            )
            .await?;
            eprintln!(
                "ENGWORLDCHG_JOURNAL_PG_EXECUTED program={program} duration={:?}",
                started.elapsed()
            );
            assert!(started.elapsed() > std::time::Duration::from_millis(10));
            drop(owner);
            drop(authority);
            drop(seal);
            h.cleanup().await
        })
    })
}

async fn exercise_incursion(
    owner: &mut crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut super::AdmittedSession,
    h: &Harness,
    authority: &crate::durability::character_authority::ReconciledCharacterAuthority<'_, '_>,
) -> TestResult {
    const QUEST: &str = "oteryn:quest.authored.insectoid_invasion_world_change";
    const KILLS: &str = "oteryn:quest-progress/world-change-journal/insectoid_invasion/kills";
    const SCOUT: &str = "oteryn:creature.insectoid_scout";
    owner.premium_coordinator = None;
    super::qengine_kill_stories_fixtures::death(owner, admitted, SCOUT, 23000).await?;
    assert_eq!(
        value(h, KILLS).await?,
        0,
        "a death before entry cannot count"
    );
    let blocker_cell = super::quest_world_callbacks_fixtures::local(Position {
        x: 32208,
        y: 32058,
        floor: -7,
    });
    {
        let mut runtime = owner.runtime.lock().await;
        let session = crate::foundation::GameSessionId::decode(&id(76)).map_err(debug)?;
        let reserved = runtime.reserve_fresh_session(session).map_err(debug)?;
        let blocker = runtime.commit_fresh_session(reserved).map_err(debug)?;
        runtime
            .initialize_pinned_test_position(blocker, blocker_cell)
            .map_err(debug)?;
    }
    step(owner, admitted, 1, StepDirection::West).await?;
    assert!(
        live_creatures(owner, SCOUT).await?.is_empty(),
        "another player must block admission"
    );
    assert_eq!(
        value(
            h,
            "oteryn:quest-progress/world-change-journal/insectoid_invasion/phase"
        )
        .await?,
        0
    );
    {
        let outside = super::quest_world_callbacks_fixtures::local(Position {
            x: 32211,
            y: 32061,
            floor: -7,
        });
        let mut runtime = owner.runtime.lock().await;
        let moved = runtime
            .commit_quest_platform_displacement(blocker_cell, outside, |_, _, _| true, || true)
            .map_err(debug)?;
        assert_eq!(moved.len(), 1);
    }
    step(owner, admitted, 2, StepDirection::East).await?;
    let mut credited = 0_i64;
    for wave in 0..6_u64 {
        step(owner, admitted, wave + 3, StepDirection::West).await?;
        let cohort = live_creatures(owner, SCOUT).await?;
        assert_eq!(
            cohort.len(),
            2,
            "the native step must supply the source scout pair"
        );
        for (ordinal, target) in cohort.into_iter().enumerate() {
            let outside_source = wave == 0 && ordinal == 1;
            if outside_source {
                let mut runtime = owner.runtime.lock().await;
                let from = runtime
                    .read_actor_position(target)
                    .map_err(debug)?
                    .position();
                let outside = super::quest_world_callbacks_fixtures::local(Position {
                    x: 32221,
                    y: 32059,
                    floor: -7,
                });
                let moved = runtime
                    .commit_quest_platform_displacement(from, outside, |_, _, _| true, || true)
                    .map_err(debug)?;
                assert_eq!(moved.len(), 1);
            }
            kill_actor(
                owner,
                admitted,
                target,
                24000 + wave * 2 + u64::try_from(ordinal)?,
            )
            .await?;
            credited += i64::from(!outside_source);
            assert_eq!(
                value(h, KILLS).await?,
                credited,
                "out-of-area native deaths cannot advance the goal"
            );
            let completions: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts WHERE quest_key=$1 AND completes")
                .bind(QUEST).fetch_one(&h.pool).await?;
            assert_eq!(
                completions,
                i64::from(credited == 10),
                "only the tenth real death completes"
            );
            if credited == 10 {
                assert!(
                    live_creatures(owner, SCOUT).await?.is_empty(),
                    "completion must retire the surviving exact cohort actor"
                );
                break;
            }
        }
    }
    assert_eq!(credited, 10);
    step(owner, admitted, 9, StepDirection::West).await?;
    assert!(
        live_creatures(owner, SCOUT).await?.is_empty(),
        "the completed encounter cannot restart"
    );
    super::qengine_kill_stories_fixtures::death(owner, admitted, SCOUT, 25000).await?;
    assert_eq!(value(h, KILLS).await?, 10);
    let completions: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_character_quest_receipts WHERE quest_key=$1 AND completes",
    )
    .bind(QUEST)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(
        completions, 1,
        "later deaths cannot mint another completion"
    );
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let state = h
        .root
        .read_character_quest_state(authority, fence.character_id)
        .await
        .map_err(debug)?;
    assert_eq!(state.tracks().get(KILLS), Some(&10));
    Ok(())
}

async fn prove_journal_wire(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    mut admitted: super::AdmittedSession,
    quest: &str,
    goal: Option<i64>,
    command_id: u64,
    completed: bool,
) -> TestResult {
    use oteryn_protocol_oteryn::quest_log::{
        QuestLogQuery, QuestLogView, decode_quest_log, encode_quest_log_query,
    };
    use oteryn_protocol_oteryn::{MessageType, decode_wire_envelope};
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    let state = oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
        .map_err(debug)?;
    let logs = oteryn_gameplay_quests::quest::log::loader::load_embedded_quest_log(&state)?;
    let index = u32::try_from(
        logs.content
            .active()
            .quests()
            .iter()
            .position(|q| q.key == quest)
            .ok_or("native journal index")?,
    )?;
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(
            owner.offered_capabilities(),
            &[16],
        )
        .ok_or("journal capability")?;
    let payload = encode_quest_log_query(&if completed {
        QuestLogQuery::Quest { quest_index: index }
    } else {
        QuestLogQuery::List
    })
    .map_err(debug)?;
    let command = oteryn_protocol_oteryn::encode_client_command(
        1,
        &oteryn_protocol_oteryn::ClientCommandValue {
            command_id,
            command_type: 22,
            payload: &payload,
        },
    )
    .map_err(debug)?;
    let mut framed = u32::try_from(command.len())?.to_be_bytes().to_vec();
    framed.extend(command);
    let (_, output) =
        super::weather_wire_fixtures::fixture_access::serve_input(owner, admitted, &framed).await?;
    let mut reader = output.as_slice();
    let mut found = false;
    while !reader.is_empty() {
        let frame = crate::gameplay_transport::tcp_tls::read_frame(&mut reader).await?;
        let envelope = decode_wire_envelope(&frame).map_err(debug)?;
        if envelope.message_type() == MessageType::StateDelta {
            let delta =
                oteryn_protocol_oteryn::decode_state_delta(envelope.payload()).map_err(debug)?;
            if delta.domain_id == 16 {
                let log = decode_quest_log(delta.payload).map_err(debug)?;
                if !completed && let Some(QuestLogView::List(quests)) = &log.view {
                    assert!(
                        !quests.iter().any(|quest| quest.quest_index == index),
                        "an unavailable donor mission must not be listed"
                    );
                    found = true;
                }
                if completed && let Some(QuestLogView::Line(line)) = log.view {
                    found |= line.completed
                        && line.quest_index == index
                        && line.missions.iter().any(|m| {
                            m.done
                                && goal
                                    .is_none_or(|goal| m.text.contains(&format!("{goal}/{goal}")))
                        });
                }
            }
        }
    }
    assert!(found, "the native journal result must reach domain 16");
    Ok(())
}
