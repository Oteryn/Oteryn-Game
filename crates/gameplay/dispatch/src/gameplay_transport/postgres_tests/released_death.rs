//! NBREL-1: offline death must not trap an expired session behind its release fence.
use super::*;
use crate::ability::creature_bite::CreatureBiteVitals;
use crate::foundation::{CharacterId, CharacterLease, GameSessionState};
use crate::gameplay_transport::actor_spell::{FamiliarActorEnd, FamiliarLogoutSave};
use crate::gameplay_transport::{FenceHold, FenceStep, TransitionFence};

#[test]
fn grace_expiry_after_released_death_reaches_terminal_release() -> TestResult {
    scenario(Case::Release)
}

#[test]
fn settle_released_death_true_for_already_durable_death() -> TestResult {
    scenario(Case::AlreadyDurable)
}

#[test]
fn familiar_logout_save_returns_fenced_out_on_respawn_pending() -> TestResult {
    scenario(Case::Familiar)
}

#[derive(Clone, Copy)]
enum Case {
    Release,
    AlreadyDurable,
    Familiar,
}

fn scenario(case: Case) -> TestResult {
    kill_reward_e2e::scenario_thread::run_async(async move |admin| {
        let started = std::time::Instant::now();
        let (generation, pin) = death_ml_loss_composed::production_generation_with_pin(
            fence(1)?.runtime_scope.world_id(),
            "content-1",
        )?;
        let h = Harness::create_with_admission_seed(admin, "released_death", false, false).await?;
        eprintln!(
            "NBREL_1_POSTGRES_STARTED server_version={}",
            sqlx::query_scalar::<_, String>("SHOW server_version")
                .fetch_one(&h.pool)
                .await?
        );
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let active = generation.active().ok_or("active generation")?;
        let native = active.native_gameplay().ok_or("native gameplay")?;
        let room = pin.qualified_room().clone();
        let (channel_pin, _, _, _) = pin.into_channel_parts();
        let deps = Dependencies::with_content(channel_pin, room, native.spell_book().clone())?;
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(active);
        deps.initialize(&owner).await;
        let actor = deps.actor;
        let session = deps.admitted.game_session_id;
        {
            let mut runtime = owner.runtime.lock().await;
            runtime
                .initialize_first_entry_position(actor)
                .map_err(debug)?;
            runtime
                .bind_attacker_lease(
                    actor,
                    session,
                    CharacterLease::new(CharacterId::decode(&id(41)).map_err(debug)?, 2)
                        .map_err(debug)?,
                )
                .map_err(debug)?;
        }
        track(&owner, &deps)?;
        let mut admitted = deps.admitted;
        owner.admit_progression_session(&mut admitted).await;
        admitted.item_fence = Some(
            death_ml_loss_composed::current_fence(&owner, session)
                .await?
                .item_fence(),
        );
        let progression = owner
            .progression_binding(session)
            .ok_or("production progression binding")?;
        // Retain a familiar lifecycle owned by this session, as a prior summon would.
        {
            use crate::durability::character_familiar::{
                DurableFamiliarState, FamiliarLifecycleV2, FamiliarLifecycleWriter,
                FamiliarStateOccurrence, FamiliarStateRequest,
            };
            let fence = death_ml_loss_composed::current_fence(&owner, session).await?;
            let before = DurableFamiliarState::default();
            let after = DurableFamiliarState {
                lifecycle: Some(FamiliarLifecycleV2 {
                    version: 2,
                    familiar_remaining_ms: 60_000,
                    open: true,
                    writer: FamiliarLifecycleWriter::from_fence(&fence).map_err(debug)?,
                    clean_end_game_session_id: None,
                }),
                lifecycle_epoch: 1,
                familiar_definition: Some("familiar".into()),
                familiar_revision: Some("1".into()),
                ..before.clone()
            };
            h.root
                .commit_character_familiar_state(
                    &authority,
                    &h.node,
                    fence,
                    FamiliarStateRequest {
                        occurrence: FamiliarStateOccurrence::from_bytes(id(119)).map_err(debug)?,
                        before,
                        after,
                        content_revision: progression.context.content.clone(),
                        policy_revision: progression.policy_revision.clone(),
                        policy_digest: native.source_digest(),
                    },
                )
                .await
                .map_err(debug)?;
        }
        // Fixture seeding above used the root directly, outside the owner sequencer.
        owner.revision_sequencer =
            crate::durability::character_revision_sequencer::CharacterRevisionSequencer::new()
                .into();
        // Record the same expired loss in the durable and runtime owners, then kill offline.
        deps.loss(&h, -1).await?;
        let death = {
            let mut runtime = owner.runtime.lock().await;
            let mut states = owner.states.spell_states.lock().await;
            let hit = states
                .apply_creature_damage(
                    &mut runtime,
                    actor,
                    session,
                    100_000,
                    crate::foundation::owner_timer::SemanticTimeMicros::from_micros(
                        owner.owner_now().get(),
                    ),
                )
                .ok_or("lethal hit")?;
            assert_eq!(hit.damage.health_after, 0);
            states
                .player_death(&runtime, actor, session)
                .ok_or("offline death")?
        };
        let store = FreshAdmissionStore::from_root(h.root.clone());
        assert_eq!(
            store.current_session(session).await?.session_state(),
            GameSessionState::Reconnectable
        );
        match case {
            Case::Release => {
                let released = tokio::time::timeout(
                    Duration::from_secs(10),
                    owner.release_after_grace(admitted),
                )
                .await;
                assert_eq!(
                    released,
                    Ok(GraceExpiryResult::Released),
                    "offline death must reach terminal release"
                );
                assert_eq!(
                    store.current_session(session).await?.session_state(),
                    GameSessionState::Terminal
                );
                assert_eq!(pending(&h).await?, 1);
                // A genuine successor admission owns placement and consumption under its new lease.
                admit_and_consume(&h, &deps, &owner, &store).await?;
                assert_eq!(pending(&h).await?, 0);
            }
            Case::AlreadyDurable | Case::Familiar => {
                assert!(owner.settle_released_death(actor, session, &mut None).await);
                assert!(
                    h.root
                        .reconcile_character_death(&authority, death.occurrence)
                        .await
                        .map_err(debug)?
                        .is_some()
                );
                assert_eq!(pending(&h).await?, 1);
                let mut hold = FenceHold::new(session, TransitionFence::GraceExpiry(1));
                assert!(matches!(
                    owner.fence_transition(actor, &mut hold, false).await,
                    FenceStep::Fenced
                ));
                match case {
                    Case::AlreadyDurable => {
                        // Retained pre-death checkpoint work must not run again under
                        // the release fence once this death already has a durable receipt.
                        {
                            use oteryn_gameplay_domain::domain::hireling::food::{
                                GourmetBuff, GourmetConditions,
                            };
                            crate::gameplay_transport::item_use::gourmet::remember(
                                &mut *owner.states.spell_states.lock().await,
                                actor,
                                session,
                                GourmetConditions {
                                    attributes: Some(GourmetBuff {
                                        item_id: 29408,
                                        remaining_ms: 1000,
                                    }),
                                    haste: None,
                                },
                                0,
                            );
                        }
                        // Another actor may own the lane while a new release call
                        // arrives. A durable death must not wait on death-flush work.
                        let _lane = owner
                            .spell_lane
                            .acquire()
                            .await
                            .map_err(|_| "spell lane unresolved")?;
                        let mut settled = None;
                        assert_eq!(
                            tokio::time::timeout(
                                Duration::from_secs(5),
                                owner.settle_released_death(actor, session, &mut settled)
                            )
                            .await,
                            Ok(true),
                            "durable death must bypass fenced prechecks",
                        );
                        assert_eq!(settled, Some(death));
                    }
                    Case::Familiar => {
                        use crate::durability::character_familiar::{
                            FamiliarStateOccurrence, FamiliarStateRequest,
                        };
                        use crate::durability::character_progression::CharacterProgressionError;
                        let fence = death_ml_loss_composed::current_fence(&owner, session).await?;
                        let before = h
                            .root
                            .read_character_familiar_state(&authority, fence.character_id)
                            .await
                            .map_err(debug)?
                            .state()
                            .clone();
                        let mut after = before.clone();
                        after.lifecycle.as_mut().ok_or("familiar lifecycle")?.open = false;
                        assert!(
                            matches!(
                                h.root
                                    .commit_character_familiar_state(
                                        &authority,
                                        &h.node,
                                        fence,
                                        FamiliarStateRequest {
                                            occurrence: FamiliarStateOccurrence::from_bytes(id(
                                                121
                                            ))
                                            .map_err(debug)?,
                                            before,
                                            after,
                                            content_revision: progression.context.content.clone(),
                                            policy_revision: progression.policy_revision.clone(),
                                            policy_digest: native.source_digest(),
                                        },
                                    )
                                    .await,
                                Err(CharacterProgressionError::RespawnPending)
                            ),
                            "the real writer must refuse specifically because respawn is pending"
                        );
                        for _ in 0..2 {
                            let (save, writes) = owner
                                .test_familiar_clean_end_probe(
                                    owner.save_familiar_logout(
                                        actor,
                                        session,
                                        FamiliarActorEnd::GraceExpiry(1),
                                    ),
                                    None,
                                )
                                .await;
                            assert_eq!(writes, 1, "real PostgreSQL familiar clean-end attempt");
                            assert_eq!(save, FamiliarLogoutSave::FencedOut);
                            assert!(
                                owner
                                    .states
                                    .spell_states
                                    .lock()
                                    .await
                                    .pending_familiar_logouts
                                    .is_empty()
                            );
                        }
                    }
                    Case::Release => return Err("unexpected release case".into()),
                }
                owner.states.fence_holders.release(&mut hold);
            }
        }
        eprintln!(
            "NBREL_1_POSTGRES_EXECUTED elapsed_ms={}",
            started.elapsed().as_millis()
        );
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

async fn pending(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT count(*) FROM game_character_pending_respawns WHERE character_id=encode($1,'hex')::uuid")
        .bind(id(41).as_slice()).fetch_one(&h.pool).await?)
}

async fn admit_and_consume(
    h: &Harness,
    deps: &Dependencies,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    store: &FreshAdmissionStore,
) -> TestResult {
    use crate::durability::admission_authority_guards::AdmissionGuardStore;
    use crate::foundation::AuthenticatedTransportRefV1;
    use crate::foundation::fresh_admission_durability::FreshAdmissionDurableOutcomeV1;
    use crate::gameplay_transport::{AdmissionRetryBudget, canonical_uuid};
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
            .fetch_one(&h.pool)
            .await?;
    let mut source = fresh_fixture::fresh::Source::appearance_observer(
        now,
        canonical_uuid(&id(40)),
        CharacterId::decode(&id(41)).map_err(debug)?,
    )?;
    source.current.character_lease_generation = 2;
    let keys: Vec<_> = source.rows.iter().map(|row| row.key.clone()).collect();
    source.rows = AdmissionGuardStore::from_root(h.root.clone())
        .load(&keys)
        .await?
        .into_iter()
        .collect::<Option<Vec<_>>>()
        .ok_or("released authority")?;
    let session = GameSessionId::decode(&id(120)).map_err(debug)?;
    let request = source.request_with_nonce(
        session,
        AuthenticatedTransportRefV1::decode(&[120; 16]).map_err(debug)?,
        Some((&canonical_uuid(&id(40)), &canonical_uuid(&id(41)))),
        now,
        Some([120; 32]),
    )?;
    assert!(matches!(
        store.commit(&request).await?,
        FreshAdmissionDurableOutcomeV1::Committed(_)
    ));
    let lease = store
        .current_session(session)
        .await?
        .current_character_lease();
    let actor = {
        let mut runtime = owner.runtime.lock().await;
        let reservation = runtime.reserve_fresh_session(session).map_err(debug)?;
        let actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .initialize_first_entry_position(actor)
            .map_err(debug)?;
        runtime
            .bind_attacker_lease(actor, session, lease)
            .map_err(debug)?;
        actor
    };
    deps.initialize_player(owner, actor, session).await;
    let budget = AdmissionRetryBudget::for_root(&session, &h.root);
    assert!(
        owner
            .consume_admitted_respawn(&budget, session, actor, None)
            .await
    );
    Ok(())
}
