use super::house_transfer_fixtures::{RecoverySource, prepare_owned_session};
use super::*;
use crate::durability::item_use::ItemUseError;
use crate::gameplay_transport::TerminalRelease;
use crate::gameplay_transport::connection::{
    ConnectionIdentifiers, FirstEntryOutcome, FreshAdmissionAuthority, GraceExpiryResult,
    LogoutReply,
};
use crate::gameplay_transport::paced_movement::underwater_drown::{
    DROWN_DEFINITION_KEY, DROWN_SOURCE, drown_definition,
};
use crate::gameplay_transport::spell_character_facts::{CastFactsLoad, load_character_cast_facts};
use oteryn_gameplay_domain::domain::condition::{ConditionValues, ConflictKey};
use oteryn_gameplay_entities::{ActorConditionTransition, ConditionSource};
use oteryn_gameplay_foundation::foundation::condition::{
    ApplicationFacts, ConditionDefinition, ConditionSourceKind, TickFacts, TickKind,
};
use oteryn_protocol_oteryn::logout::LogoutResult;
use oteryn_simulation_determinism::{DecisionOccurrenceId, GameplayDecisionRoot};
use std::time::Duration;

// Sub-ID 0 shares the Recovery slot.
const REGENERATION: ConflictKey = ConflictKey::Recovery;
const DURATION_US: u64 = 60_000_000;
const FIRST_TICK_US: u64 = 2_000_000;
const PVE_OFFLINE_MS: u32 = 15_000;

#[test]
fn native_finite_condition_postgres_relogin_preserves_remaining_ticks() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    on_thread(admin, Case::Condition)
}

#[test]
fn native_condition_postgres_terminal_save_excludes_environment_and_stale_lease() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    on_thread(admin, Case::Environment)
}

#[test]
fn pve_logout_block_survives_fresh_admission_until_original_deadline_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    on_thread(admin, Case::PveLogout)
}

#[test]
fn g05_single_cleanup_logout_retries_late_condition_holder_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    on_thread(admin, Case::G05Cleanup)
}

#[test]
fn c20c_first_logout_retries_late_monk_fence_checkout_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    on_thread(admin, Case::C20cFirstLogout)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Case {
    Condition,
    Environment,
    PveLogout,
    G05Cleanup,
    C20cFirstLogout,
}

fn on_thread(admin: String, case: Case) -> TestResult {
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || {
            let run = || -> TestResult {
                let rt = crate::bestiary_postgres_harness::runtime()?;
                let h = rt.block_on(Harness::create_with_admission_seed(
                    admin,
                    "native_condition_relogin",
                    true,
                    false,
                ))?;
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    rt.block_on(scenario(&h, case))
                }));
                let cleanup = rt.block_on(h.cleanup());
                match result {
                    Ok(result) => {
                        result?;
                        cleanup
                    }
                    Err(panic) => std::panic::resume_unwind(panic),
                }
            };
            run().map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "native condition PostgreSQL thread panicked")??;
    Ok(())
}

struct FreshIds;
impl ConnectionIdentifiers for FreshIds {
    fn game_session_id(&self) -> Option<GameSessionId> {
        GameSessionId::decode(&id(91)).ok()
    }

    fn transport_ref(&self) -> Option<crate::foundation::AuthenticatedTransportRefV1> {
        crate::foundation::AuthenticatedTransportRefV1::decode(&[91; 16]).ok()
    }
}

async fn scenario(h: &Harness, case: Case) -> TestResult {
    let with_environment = case == Case::Environment;
    let recovery = RecoverySource::start(h).await?;
    prepare_owned_session(h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let deps = Dependencies::new()?;
    let mut owner = deps.owner(h, &authority);
    owner.evidence = &recovery.evidence;
    let mut admitted = deps.admitted;
    let session = admitted.game_session_id;
    let (current, _) = FreshAdmissionStore::from_root(h.root.clone())
        .current_session_at(session)
        .await?;
    admitted.item_fence = crate::gameplay_transport::item_fence_of(current);
    admitted.first_entry = FirstEntryOutcome::Positioned;
    let old_fence = admitted.item_fence.ok_or("old lease fence")?;
    // Logout checkpoints Monk state, so the actor must use its durable vocation and values.
    let character = old_fence.character_id;
    let facts = match load_character_cast_facts(&h.root, &authority, character).await {
        CastFactsLoad::Ready { facts, .. } | CastFactsLoad::NoVocation { facts, .. } => facts,
        CastFactsLoad::Unavailable => return Err("fixture character facts".into()),
    };
    let monk = owner
        .load_monk_state(character)
        .await
        .ok_or("fixture monk state")?;
    {
        let mut runtime = deps.runtime.lock().await;
        runtime
            .initialize_first_entry_position(deps.actor)
            .map_err(debug)?;
        owner
            .states
            .spell_states
            .try_lock()
            .map_err(debug)?
            .initialize(
                &runtime,
                deps.actor,
                session,
                facts,
                monk,
                owner.owner_now(),
            )
            .ok_or("fixture spell state")?;
    }
    let applied_at = install_conditions(&owner, &deps, with_environment).await?;
    if case == Case::PveLogout {
        let runtime = owner.runtime.lock().await;
        owner.states.attack.lock().await.record_hit_taken(
            &runtime,
            deps.actor,
            session,
            owner.owner_now(),
        );
    }
    owner.clock_origin -= Duration::from_millis(750);
    let _clock = owner.freeze_owner_clock_for_test();
    let elapsed_at_save = owner.owner_now().get() - applied_at;
    let original_retry = if case == Case::PveLogout {
        let LogoutReply::Result(LogoutResult::InFight { retry_after_ms }) =
            owner.logout_session(admitted).await
        else {
            return Err("the PvE hit must refuse logout".into());
        };
        // A force-disconnect retires the old actor while its deadline is live.
        let transport = admitted.controller.ok_or("original transport")?.transport;
        assert_eq!(
            owner
                .release_terminal(admitted, TerminalRelease::StaffKick(transport))
                .await,
            GraceExpiryResult::Released
        );
        Some(retry_after_ms)
    } else if case == Case::C20cFirstLogout {
        logout_with_late_contention(h, &owner, admitted, case).await?;
        return Ok(());
    } else {
        assert_eq!(
            owner.logout_session(admitted).await,
            LogoutReply::Result(LogoutResult::Accepted)
        );
        None
    };
    let saved = h
        .root
        .read_character_conditions(&authority, old_fence.character_id)
        .await
        .map_err(debug)?
        .ok_or("terminal release snapshot")?;
    assert_eq!(saved.lease_generation, old_fence.character_lease_generation);
    let rows = saved.conditions["conditions"]
        .as_array()
        .ok_or("saved conditions")?;
    if with_environment {
        assert!(
            rows.iter()
                .all(|row| row["definition"]["key"] != DROWN_DEFINITION_KEY),
            "drowning is re-established by the map"
        );
    }
    assert_eq!(
        rows.len(),
        1,
        "terminal release saves the eligible native condition"
    );
    let ends_in = rows[0]["ends_at"].as_u64().ok_or("remaining duration")?;
    assert_eq!(ends_in, DURATION_US - elapsed_at_save);

    if original_retry.is_some() {
        // Age the durable checkpoint without sleeping or changing actor clocks.
        let aged = sqlx::query(
            "UPDATE game_character_condition_snapshots SET saved_at = saved_at - $2 * interval '1 millisecond' \
             WHERE character_id = encode($1,'hex')::uuid",
        )
        .bind(old_fence.character_id.as_bytes().as_slice())
        .bind(i64::from(PVE_OFFLINE_MS))
        .execute(&h.pool)
        .await?;
        assert_eq!(aged.rows_affected(), 1);
    }

    let fresh_deps = Dependencies::new()?;
    let mut fresh_owner = fresh_deps.owner(h, &authority);
    fresh_owner.evidence = &recovery.evidence;
    let token = RecoverySource::fresh_token(91)?;
    let frame = oteryn_protocol_oteryn::encode_client_bootstrap(
        &oteryn_protocol_oteryn::ClientBootstrapValue {
            schema_revision: 1,
            character_id: crate::foundation::CharacterId::decode(old_fence.character_id.as_bytes())
                .map_err(debug)?,
            admission_material: token.as_bytes(),
            client_build_id: "native-condition-relogin",
            supported_capabilities: &[],
        },
    )
    .map_err(debug)?;
    let (mut server, _client) = tokio::io::duplex(1 << 20);
    let fresh = crate::gameplay_transport::connection::admit_frame(
        &mut server,
        &frame,
        &fresh_owner,
        &FreshIds,
    )
    .await
    .map_err(debug)?;
    assert_ne!(fresh.game_session_id, session);
    assert!(
        matches!(
            fresh.first_entry,
            FirstEntryOutcome::Positioned | FirstEntryOutcome::Reconciled
        ),
        "fresh first entry: {:?}",
        fresh.first_entry
    );
    let fresh_fence = fresh.item_fence.ok_or("fresh lease fence")?;
    assert!(fresh_fence.character_lease_generation > saved.lease_generation);
    if case == Case::G05Cleanup {
        logout_with_late_contention(h, &fresh_owner, fresh, case).await?;
        return Ok(());
    }
    if let Some(original_retry) = original_retry {
        let actor = fresh.runtime_actor.ok_or("fresh actor")?;
        assert!(
            fresh_owner
                .observe_combat_state(actor, fresh.game_session_id)
                .await
                .ok_or("combat snapshot")?
                .in_fight,
            "the first combat snapshot on a fresh transport must show the retained PvE block"
        );
        let LogoutReply::Result(LogoutResult::InFight { retry_after_ms }) =
            fresh_owner.logout_session(fresh).await
        else {
            return Err(
                "the original PvE deadline must refuse logout after fresh admission".into(),
            );
        };
        assert!(
            retry_after_ms > 0 && retry_after_ms <= original_retry - PVE_OFFLINE_MS,
            "the restored block must subtract the time spent offline"
        );
        let attack = fresh_owner.states.attack.lock().await;
        let deadline = attack
            .in_fight_until(actor, fresh.game_session_id, fresh_owner.owner_now())
            .ok_or("restored PvE deadline")?;
        drop(attack);
        fresh_owner
            .states
            .test_owner_time
            .store(deadline.get(), std::sync::atomic::Ordering::Relaxed);
        assert_eq!(
            fresh_owner.logout_session(fresh).await,
            LogoutReply::Result(LogoutResult::InFight { retry_after_ms: 1 })
        );
        fresh_owner
            .states
            .test_owner_time
            .store(deadline.get() + 1, std::sync::atomic::Ordering::Relaxed);
        assert_eq!(
            fresh_owner.logout_session(fresh).await,
            LogoutReply::Result(LogoutResult::Accepted)
        );
        return Ok(());
    }
    let restored = {
        let runtime = fresh_deps.runtime.lock().await;
        let states = fresh_owner.states.spell_states.lock().await;
        states
            .get(
                &runtime,
                fresh.runtime_actor.ok_or("fresh actor")?,
                fresh.game_session_id,
            )
            .ok_or("fresh spell state")?
            .owned_conditions()
            .clone()
    };
    verify_restored_phase(restored, ends_in)?;

    if with_environment {
        let mut replacement = saved.clone();
        replacement.conditions = serde_json::json!({"schema": 1, "conditions": []});
        assert!(matches!(
            h.root
                .save_character_conditions(
                    &authority,
                    &h.node,
                    old_fence,
                    *fresh.channel_id.as_bytes(),
                    replacement,
                )
                .await,
            Err(ItemUseError::AuthorityRejected)
        ));
        let current = h
            .root
            .read_character_conditions(&authority, old_fence.character_id)
            .await
            .map_err(debug)?
            .ok_or("snapshot after stale save")?;
        assert_eq!(current.lease_generation, saved.lease_generation);
        assert_eq!(current.conditions, saved.conditions);
    }
    Ok(())
}

async fn logout_with_late_contention(
    h: &Harness,
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    case: Case,
) -> TestResult {
    use crate::durability::ROOT_RECOVERY_WINDOW;
    use crate::gameplay_transport::logout::LOGOUT_CHECKOUT_BARRIER;
    use crate::gameplay_transport::{RECONCILE_ATTEMPTS, RECONCILE_BACKOFF};
    use std::cell::Cell;
    use std::future::{Future, poll_fn};
    use std::sync::Arc;
    use std::task::Poll;
    use tokio::sync::Notify;

    let cleanup = case == Case::G05Cleanup;
    let stage = if cleanup {
        "condition_holder"
    } else {
        "wheel_monk"
    };
    let barrier = Arc::new((Notify::new(), Notify::new()));
    let reply = tokio::time::timeout(ROOT_RECOVERY_WINDOW * 3, async {
        let mut logout = std::pin::pin!(LOGOUT_CHECKOUT_BARRIER.scope(
            (Cell::new(Some(stage)), barrier.clone()),
            owner.logout_command(admitted, 1),
        ));
        tokio::select! {
            () = barrier.0.notified() => {}
            reply = &mut logout => return Err(format!("logout ended before {stage}: {reply:?}").into()),
        }
        h.root.await_ready().await?;
        // Age the release without ageing a running SQL pass. The stage itself
        // has not had a chance to attempt its checkout yet.
        tokio::time::pause();
        tokio::time::advance(ROOT_RECOVERY_WINDOW).await;
        let condition_holder = if cleanup { Some(owner.states.attack.lock().await) } else { None };
        let durable_holder = if cleanup { None } else { Some(h.root.try_issue_semantic_pass()?) };
        barrier.1.notify_one();
        let checks = if cleanup { 1 } else { RECONCILE_ATTEMPTS };
        for _ in 0..checks {
            assert!(poll_fn(|cx| Poll::Ready(logout.as_mut().poll(cx))).await.is_pending());
            tokio::time::advance(RECONCILE_BACKOFF + Duration::from_millis(1)).await;
        }
        tokio::time::resume();
        drop(condition_holder);
        if let Some(held) = durable_holder {
            held.run(|_, _| Box::pin(async { Ok(()) })).await?;
        }
        TestResult::Ok(logout.await)
    }).await??;
    assert_eq!(
        reply,
        LogoutReply::Result(LogoutResult::Accepted),
        "one logout must release after {stage} contention clears"
    );
    assert_eq!(
        FreshAdmissionStore::from_root(h.root.clone())
            .current_session(admitted.game_session_id)
            .await?
            .session_state(),
        crate::foundation::GameSessionState::Terminal
    );
    Ok(())
}

async fn install_conditions(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    deps: &Dependencies,
    with_environment: bool,
) -> TestResult<u64> {
    let session = deps.admitted.game_session_id;
    let mut runtime = deps.runtime.lock().await;
    let now = owner.owner_now();
    let root = GameplayDecisionRoot::from_bytes([8; 32]);
    let facts = ApplicationFacts {
        now: now.get(),
        base_speed: 220,
        mana_shield_capacity: 0,
        target_reentry_protected: false,
        source_reentry_protected: false,
        target_is_player: true,
        decision_root: &root,
        occurrence: DecisionOccurrenceId::from_bytes([8; 16]),
    };
    let definition = ConditionDefinition::new(
        "fixture:c08.native.regeneration",
        1,
        ConditionValues::SpellRegeneration {
            duration_ms: 60_000,
            sub_id: 0,
            health_gain: 3,
            health_interval_ms: 2_000,
            mana_gain: 2,
            mana_interval_ms: 3_000,
        },
    )
    .ok_or("native regeneration definition")?;
    let transition = ActorConditionTransition::Apply {
        definition: &definition,
        source: ConditionSource {
            actor: deps.actor,
            session: Some(session),
            kind: ConditionSourceKind::SelfUse,
        },
        immunities: &[],
        facts,
    };
    let plan = runtime
        .prepare_actor_condition(deps.actor, Some(session), transition, now)
        .map_err(debug)?;
    assert!(runtime.commit_actor_condition(&plan, now).map_err(debug)?);
    let native = runtime
        .actor_conditions(deps.actor, Some(session))
        .map_err(debug)?;
    assert_eq!(
        native
            .get(REGENERATION)
            .ok_or("native regeneration")?
            .provenance()
            .source,
        Some(deps.actor)
    );
    if with_environment {
        let mut states = owner.states.spell_states.try_lock().map_err(debug)?;
        let state = states
            .get_mut(&runtime, deps.actor, session)
            .ok_or("source spell state")?;
        let before = state.owned_conditions().clone();
        let mut next = before.clone();
        next.apply(
            &drown_definition().ok_or("drowning definition")?,
            Some(DROWN_SOURCE.to_owned()),
            ConditionSourceKind::Field,
            &[],
            &facts,
        )
        .map_err(debug)?;
        state
            .apply_batch_conditions(&before, &next)
            .map_err(debug)?;
    }
    Ok(now.get())
}

fn verify_restored_phase(
    mut conditions: oteryn_gameplay_foundation::foundation::condition::ConditionStore<String>,
    ends_in: u64,
) -> TestResult {
    let restored = conditions
        .get(REGENERATION)
        .ok_or("restored native regeneration")?;
    assert!(restored.provenance().is_unattributed());
    let elapsed_at_save = DURATION_US - ends_in;
    assert!((750_000..FIRST_TICK_US).contains(&elapsed_at_save));
    let first_tick = restored.started_at() + FIRST_TICK_US - elapsed_at_save;
    assert_eq!(
        restored.remaining_duration_ms(restored.started_at()),
        Some((ends_in / 1000).try_into()?)
    );
    assert!(
        conditions
            .take_due(first_tick - 1, TickFacts::default())
            .is_empty()
    );
    let ticks = conditions.take_due(first_tick, TickFacts::default());
    assert_eq!(ticks.len(), 1);
    assert_eq!(
        ticks[0].kind,
        TickKind::SpellRegeneration {
            health_gain: 3,
            mana_gain: 0,
            suppressed: false,
        }
    );
    let ticks = conditions.take_due(first_tick + 1_000_000, TickFacts::default());
    assert_eq!(ticks.len(), 1);
    assert_eq!(
        ticks[0].kind,
        TickKind::SpellRegeneration {
            health_gain: 0,
            mana_gain: 2,
            suppressed: false,
        }
    );
    Ok(())
}
