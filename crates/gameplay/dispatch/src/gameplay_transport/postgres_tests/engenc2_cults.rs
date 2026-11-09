//! Real callback admission, all Cults native phases, and the existing durable quest writer.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::foundation::{
    ChannelRuntimeV1, CommandId, CommandRef, ExactActorRef, MovementLocalPosition,
    OwnerDamageCommand,
};
use crate::gameplay_transport::connection::{
    FirstEntryOutcome, FreshAdmissionAuthority, UseCommand,
};
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Position};

use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};
use std::sync::Arc;

const COUNT: &str = "oteryn:quest-progress/quest/u11_40/cults_of_tibia/humans/mission";

fn callback() -> TestResult<Callback> {
    let mut callback = oteryn_gameplay_quests::quest::world_callbacks::load()?
        .into_iter()
        .find(|c| c.key == "oteryn:quest-callback/engenc2/cults-essence")
        .ok_or("production Cults callback")?;
    // Translate the actual source roster into the fixture's qualified map frame.
    callback.selector.position = Some(Position {
        x: 100,
        y: 100,
        floor: -7,
    });
    callback.selector.appearance = Some(2772);
    callback.lever = Some([2772, 2773]);
    let encounter = callback
        .effects
        .encounter
        .as_mut()
        .ok_or("Cults encounter")?;
    for (i, player) in encounter.players.iter_mut().enumerate() {
        player.from = Position {
            x: 100,
            y: 100 + u16::try_from(i)?,
            floor: -7,
        };
        player.to = Position {
            x: 116,
            y: 117,
            floor: -7,
        };
    }
    for monster in &mut encounter.monsters {
        monster.position = Position {
            x: monster.position.x + 116 - 33098,
            y: monster.position.y + 116 - 31920,
            floor: -7,
        };
    }
    encounter.room_min = Position {
        x: 106,
        y: 106,
        floor: -7,
    };
    encounter.room_max = Position {
        x: 126,
        y: 126,
        floor: -7,
    };
    Ok(callback)
}

#[test]
fn engenc2_cults_callback_native_phases_and_settlement_postgres() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let began = std::time::Instant::now();
    super::kill_reward_e2e::scenario_thread::run("engenc2-cults", || {
        run(async |admin| {
            reset();
            let h =
                Harness::create_with_admission_seed(admin, "engenc2_cults", true, false).await?;
            prepare_current_session(&h).await?;
            let callback = callback()?;
            let trigger = Position {
                x: 100,
                y: 100,
                floor: -7,
            };
            let world = super::quest_world_callbacks::world(&callback, trigger)?;
            let mut deps = Dependencies::new()?;
            let (controller, activated) =
                super::death_ml_loss_composed::production_generation_with_pin(
                    deps.admitted.world_id,
                    "content-1",
                )?;
            let generation = controller.active().ok_or("active generation")?;
            let native = generation.native_gameplay().ok_or("native profiles")?;
            let cells = world
                .movement_cells(activated.qualified_room().movement_cells())
                .map_err(debug)?;
            let frame = world.frame_binding_digest(deps.admitted.world_id);
            let room = activated
                .qualified_room()
                .bundle_view(
                    cells.clone(),
                    frame,
                    world.map_revision_digest(),
                    crate::content::NativeEntryStart {
                        x: 100,
                        y: 100,
                        floor: 7,
                    },
                )
                .ok_or("bundle room")?;
            let pin = crate::content::channel_content_pin::ChannelContentPin::from_activation(
                deps.admitted.world_id,
                1,
                generation.identity().server_artifact_digest(),
                generation.identity().client_artifact_digest(),
                frame,
                world.map_revision_digest(),
                (100, 100, 7),
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
            let reservation = runtime
                .reserve_fresh_session(deps.admitted.game_session_id)
                .map_err(debug)?;
            deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
            runtime
                .initialize_pinned_test_position(
                    deps.actor,
                    MovementLocalPosition {
                        x: 100,
                        y: 100,
                        floor: 7,
                    },
                )
                .map_err(debug)?;
            native
                .install_companion_policies(&mut runtime)
                .map_err(debug)?;
            deps.runtime = tokio::sync::Mutex::new(runtime);
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let mut owner = deps.owner(&h, &authority);
            owner.active_generation = Some(generation);
            owner.movement_cells = (&cells).into();
            owner.rooms = crate::gameplay_transport::SeamRooms::new(Some(&room), Some(&room));
            owner.quest_catalogue = Some(Arc::new(
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
            admitted.runtime_actor = Some(deps.actor);
            admitted.first_entry = FirstEntryOutcome::Positioned;
            let fence = owner
                .current_quest_fence(admitted.game_session_id)
                .await
                .map_err(|()| "fence")?
                .ok_or("fence")?;
            admitted.item_fence = Some(fence.item_fence());
            sqlx::query("INSERT INTO game_control_scope_grants VALUES (session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
            sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).bind(generation.identity().server_artifact_digest().as_slice())
            .bind(generation.identity().client_artifact_digest().as_slice()).bind(frame.as_slice()).execute(&h.pool).await?;
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
            // Replacing the fixture runtime does not transfer the admitted Character lease.
            owner.runtime.lock().await.bind_attacker_lease(
                deps.actor, admitted.game_session_id,
                crate::foundation::CharacterLease::new(
                    crate::foundation::CharacterId::decode(fence.character_id.as_bytes()).map_err(debug)?,
                    fence.character_lease_generation,
                ).map_err(debug)?,
            ).map_err(debug)?;
            owner.admit_progression_session(&mut admitted).await;
            deps.initialize(&owner).await;
            owner.admit_quest_session(&admitted).await;
            assert!(
                owner
                    .refresh_source_party_presence(deps.actor, admitted.game_session_id)
                    .await
            );
            {
                let runtime = owner.runtime.lock().await;
                runtime.bound_attacker_lease(deps.actor, admitted.game_session_id).map_err(debug)?;
                for monster in &callback.effects.encounter.as_ref().ok_or("encounter")?.monsters {
                    runtime.companion_policy(&monster.creature)
                        .map_err(|e| format!("production creature {}: {e:?}", monster.creature))?;
                }
                for role in oteryn_gameplay_encounters::bosses::phase_controller::cults::plan().roles {
                    runtime.companion_policy(role.creature)
                        .map_err(|e| format!("production phase creature {}: {e:?}", role.creature))?;
                }
            }
            assert!(owner.states.quest_sessions.lock().map_err(|_| "quest sessions")?
                .get(&admitted.game_session_id).and_then(|s| s.copy.as_ref())
                .and_then(|copy| copy.character_revision()).is_some(), "admitted quest snapshot");
            let key =
                oteryn_world_bundle::bundle::placement_key(-7, 100, 100, 1).ok_or("lever key")?;
            let mut placement = world.base().digest().to_vec();
            placement.extend_from_slice(&key.to_be_bytes());
            let outcome = owner
                .use_object(
                    deps.actor,
                    UseCommand {
                        game_session_id: admitted.game_session_id,
                        command_id: 1,
                        item_fence: admitted.item_fence,
                    },
                    WorldObjectTarget {
                        placement,
                        expected_revision: 0,
                    },
                )
                .await;
            assert_eq!(
                outcome.disposition,
                UseDisposition::Committed,
                "phase admission"
            );
            assert_eq!(
                owner
                    .states
                    .spell_states
                    .lock()
                    .await
                    .encounter_phases
                    .active
                    .lock()
                    .map_err(|_| "phases")?
                    .len(),
                1
            );
            let plan = oteryn_gameplay_encounters::bosses::phase_controller::cults::plan();
            let mut command = 100;
            for i in (0..5).flat_map(|i| [i, i + 5]).chain([10]) {
                assert_eq!(
                    value(&h).await?,
                    0,
                    "no credit before the actual Essence death"
                );
                command += 1;
                let target = {
                    let runtime = owner.runtime.lock().await;
                    runtime
                        .positioned_melee_census()
                        .map_err(debug)?
                        .into_iter()
                        .find(|(actor, _, _)| {
                            runtime.matches_live_creature_identity(
                                *actor,
                                plan.roles[i].creature.as_bytes(),
                            )
                        })
                        .map(|(actor, _, _)| actor)
                        .ok_or("expected source phase actor")?
                };
                kill(&owner, &admitted, target, plan.roles[i].creature, command).await?;
                if i < 10 {
                    let runtime = owner.runtime.lock().await;
                    let essence = runtime
                        .positioned_melee_census()
                        .map_err(debug)?
                        .iter()
                        .filter(|(actor, _, _)| {
                            runtime.matches_live_creature_identity(
                                *actor,
                                plan.roles[10].creature.as_bytes(),
                            )
                        })
                        .count();
                    assert_eq!(
                        essence,
                        usize::from(i == 9),
                        "Essence joins only after all five guardians"
                    );
                }
            }
            assert_eq!(
                owner.drain_kill_rewards(admitted.game_session_id).await,
                crate::gameplay_transport::kill_reward::DrainOutcome::Drained
            );
            assert_eq!(value(&h).await?, 2, "source Humans.Mission reaches two");
            assert_eq!(
                owner.drain_kill_rewards(admitted.game_session_id).await,
                crate::gameplay_transport::kill_reward::DrainOutcome::Drained
            );
            assert_eq!(
                value(&h).await?,
                2,
                "replayed deaths never duplicate settlement"
            );
            let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts WHERE transition_key='oteryn:quest-transition/native-kills/cults-essence'")
            .fetch_one(&h.pool).await?;
            assert_eq!(receipts, 1);
            drop(owner);
            drop(authority);
            drop(seal);
            h.cleanup().await
        })
    })?;
    let elapsed = began.elapsed().as_secs_f64();
    assert!(elapsed > 0.05);
    eprintln!("ENGENC2_CULTS_PG_EXECUTED elapsed_s={elapsed:.6}");
    Ok(())
}

async fn value(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT coalesce((SELECT value FROM game_character_quest_tracks WHERE track_key=$1),0)",
    )
    .bind(COUNT)
    .fetch_one(&h.pool)
    .await?)
}

async fn kill(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &crate::gameplay_transport::connection::AdmittedSession,
    target: ExactActorRef,
    creature: &str,
    command: u64,
) -> TestResult {
    let queue = owner.states.attack.lock().await.kills().clone();
    let mut runtime = owner.runtime.lock().await;
    let states = owner.states.spell_states.lock().await;
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let lease = runtime
        .bound_attacker_lease(actor, admitted.game_session_id)
        .map_err(debug)?;
    let health = runtime.companion_snapshot(target).map_err(debug)?.health;
    let observed = owner.owner_now();
    let now = observed.get() / 1000;
    runtime
        .borrow_exact_actor_commit_at(now)
        .commit_damage_for_bound_attacker(
            target,
            actor,
            CommandRef::new(
                admitted.game_session_id,
                CommandId::new(command).map_err(debug)?,
            ),
            0,
            OwnerDamageCommand {
                target: creature.as_bytes(),
                occurrence: b"engenc2",
                binding: b"engenc2-phase-death",
                damage: health,
            },
        )
        .map_err(debug)?;
    for _ in 0..2 {
        crate::gameplay_transport::kill_reward::record_creature_kill(
            &mut runtime,
            crate::gameplay_transport::kill_reward::KillSink {
                prey: Some(&states),
                activity: &[],
                queue: &queue,
                rewards: owner.reward_table(),
                charms: None,
            },
            target,
            crate::combat::CapturedRewardPrincipal {
                character: lease.character_id(),
                lease_generation: lease.generation(),
                session: admitted.game_session_id,
                actor,
            },
            now,
            Some(observed),
        );
    }
    Ok(())
}
