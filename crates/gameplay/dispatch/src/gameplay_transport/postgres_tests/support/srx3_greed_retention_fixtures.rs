use super::srx3_native_source_cast_fixtures::{
    NATIVE_TURN_TIMEOUT, drain_ready_monster_melee_at, next_native_melee_deadline,
    settle_monster_melee_at as settle,
};
use super::{TestResult, fixture::debug};

use crate::foundation::{CommandId, CommandRef, ExactActorRef};

use crate::gameplay_transport::{ComposedFreshAdmission, connection::AdmittedSession};

use oteryn_gameplay_encounters::bosses::goshnars_greed::{self as model, GreedEffect};

use oteryn_simulation_determinism::SemanticTimeMicros;

use std::sync::Arc;

pub(super) async fn exercise(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
) -> TestResult {
    let spawn_due = owner
        .states
        .spell_states
        .lock()
        .await
        .soul_war_callbacks
        .greed_spawn_due_for_test(model::WEAK_SOUL)
        .ok_or("actual retained lever spawn")?;
    settle(
        owner,
        spawn_due
            .checked_sub(1)
            .ok_or("actual positive source deadline")?,
    )
    .await?;
    assert!(actors(owner, model::WEAK_SOUL).await?.is_empty());
    let signal = owner
        .states
        .spell_states
        .lock()
        .await
        .soul_war_callbacks
        .pause_greed_after_spawn_for_test(model::WEAK_SOUL)
        .map_err(debug)?;
    reach_boundary(owner, &signal, spawn_due).await?;
    let weak = only_actor(owner, model::WEAK_SOUL).await?;
    {
        let runtime = owner.runtime.lock().await;
        let states = owner.states.spell_states.lock().await;
        let pending = states
            .soul_war_callbacks
            .greed_pending_for_test()
            .ok_or("cancelled actual timer remains owned")?;
        assert_eq!(pending.original_due, spawn_due);
        assert_eq!(pending.sampled_at, spawn_due);
        assert_eq!(pending.occurrence, None);
        assert_eq!(pending.committed, 1);
        assert!(
            pending.remaining.is_empty(),
            "physical spawn ACK precedes cancellation"
        );
        assert_eq!(
            crate::gameplay_transport::soul_war_callbacks::greed::next_source_due_time(
                &runtime, &states
            )
            .map_err(debug)?,
            Some(spawn_due)
        );
        assert!(
            states
                .soul_war_callbacks
                .greed_observed_for_test(weak)
                .is_some()
        );
    }
    let resumed = spawn_due + 1_000_000;
    settle(owner, resumed).await?;
    assert_eq!(
        only_actor(owner, model::WEAK_SOUL).await?,
        weak,
        "same committed spawn survives retry"
    );
    only_actor(owner, model::SOULSNATCHER).await?;
    let greedbeast = only_actor(owner, model::GREEDBEAST).await?;

    // Exercise the existing HP/kill producer while no older room callback is
    // pending. Its observation slot must not compete with its canonical death.
    let lethal_at = resumed + 1_000_000;
    heal(owner, admitted).await?;
    settle(owner, lethal_at).await?;
    let native = owner
        .active_generation
        .and_then(|active| active.native_gameplay())
        .ok_or("actual current source content")?;
    let player = admitted.runtime_actor.ok_or("actual transferred Player")?;
    {
        let mut runtime = owner.runtime.lock().await;
        let states = owner.states.spell_states.lock().await;
        let attack = owner.states.attack.lock().await;
        assert_eq!(states.soul_war_callbacks.greed_timer_due_for_test(), None);
        let health = runtime
            .companion_snapshot(greedbeast)
            .map_err(debug)?
            .health;
        let hit = runtime
            .borrow_exact_actor_commit_at_us(lethal_at)
            .commit_damage_for_bound_attacker(
                greedbeast,
                player,
                CommandRef::new(
                    admitted.game_session_id,
                    CommandId::new(32611).map_err(debug)?,
                ),
                0,
                crate::foundation::OwnerDamageCommand {
                    target: model::GREEDBEAST.as_bytes(),
                    occurrence: b"srx3:greed:copied-death",
                    binding: b"srx3:greed:actual-entry-owner",
                    damage: health,
                },
            )
            .map_err(debug)?;
        assert!(hit.applied && hit.health_before > 0 && hit.health_after == 0);
        let lease = runtime
            .bound_attacker_lease(player, admitted.game_session_id)
            .map_err(debug)?;
        let activity = attack.shared_experience_cells();
        crate::gameplay_transport::kill_reward::record_creature_kill(
            &mut runtime,
            crate::gameplay_transport::kill_reward::KillSink {
                prey: Some(&states),
                activity: &activity,
                queue: attack.kills(),
                rewards: native.reward_table(),
                charms: Some(owner.imported_charms),
            },
            greedbeast,
            crate::combat::CapturedRewardPrincipal {
                character: lease.character_id(),
                lease_generation: lease.generation(),
                session: admitted.game_session_id,
                actor: player,
            },
            lethal_at / 1_000,
            Some(SemanticTimeMicros::from_micros(lethal_at)),
        );
        let notice = runtime
            .next_source_creature_death()
            .map_err(debug)?
            .ok_or("actual copied death notice")?;
        assert_eq!(notice.actor(), greedbeast);
        assert_eq!(notice.enqueued_at_us(), Some(lethal_at));
        assert!(
            !states
                .soul_war_callbacks
                .greed_observed_for_test(greedbeast)
                .ok_or("actual observed spawn")?
                .1,
            "no HP-zero scan may consume the copied handler"
        );
        assert_eq!(
            crate::gameplay_transport::soul_war_callbacks::greed::next_source_due_time(
                &runtime, &states
            )
            .map_err(debug)?,
            None,
            "the duplicate observation is not a timer before its own BEGIN"
        );
    }
    super::srx3_native_source_cast_fixtures::drain_source_death(
        owner,
        greedbeast,
        SemanticTimeMicros::from_micros(lethal_at),
    )
    .await?;
    let first_warning = (lethal_at / 1_000 + 7_000) * 1_000;
    assert_eq!(
        owner
            .states
            .spell_states
            .lock()
            .await
            .soul_war_callbacks
            .greed_timer_due_for_test(),
        Some(first_warning)
    );
    settle(owner, lethal_at).await?;
    assert!(actors(owner, model::GREEDBEAST).await?.is_empty());

    // Source weak_soul.lua transforms on its eighth genuine native Think. The
    // pause observes the actual Strong Soul spawn, before the Weak Soul remove.
    let signal = owner
        .states
        .spell_states
        .lock()
        .await
        .soul_war_callbacks
        .pause_greed_after_spawn_for_test(model::STRONG_SOUL)
        .map_err(debug)?;
    let mut transformed_at = None;
    for _ in 0..12 {
        let now = next_think_due(owner, weak).await?;
        heal(owner, admitted).await?;
        if drive_to_boundary(owner, &signal, now).await? {
            transformed_at = Some(now);
            break;
        }
    }
    let transformed_at =
        transformed_at.ok_or("actual eighth Weak Soul Think must spawn Strong Soul")?;
    let strong = only_actor(owner, model::STRONG_SOUL).await?;
    let (occurrence, pass) = {
        let runtime = owner.runtime.lock().await;
        let states = owner.states.spell_states.lock().await;
        let pending = states
            .soul_war_callbacks
            .greed_pending_for_test()
            .ok_or("actual transform suffix after cancelled future")?;
        let occurrence = pending
            .occurrence
            .ok_or("actual retained Think occurrence")?;
        assert_eq!(occurrence.actor, weak);
        assert_eq!(pending.original_due, transformed_at);
        assert_eq!(pending.sampled_at, transformed_at);
        assert_eq!(pending.committed, 1);
        let observed = states
            .soul_war_callbacks
            .greed_observed_for_test(weak)
            .ok_or("original Weak actor association")?;
        assert_eq!(observed.2, Some(occurrence));
        assert_eq!(
            pending.remaining,
            [GreedEffect::RemoveMonster {
                instance: observed.0
            }]
        );
        assert!(
            runtime.contains_live_creature(weak),
            "source removal is the still-retained next statement"
        );
        assert!(runtime.contains_live_creature(strong));
        assert_eq!(
            crate::gameplay_transport::soul_war_callbacks::greed::next_source_due_time(
                &runtime, &states
            )
            .map_err(debug)?,
            Some(transformed_at)
        );
        (
            occurrence,
            states
                .native_profile_pass
                .pending()
                .ok_or("the SAME reserved native batch")?,
        )
    };
    // A later execution clock cannot replace the original occurrence/cutoff.
    let retry_at = transformed_at + 5_000_000;
    {
        let states = owner.states.spell_states.lock().await;
        assert!(
            states
                .native_profile_pass
                .pending()
                .is_some_and(|current| Arc::ptr_eq(&pass, &current))
        );
        assert_eq!(
            states
                .soul_war_callbacks
                .greed_pending_for_test()
                .ok_or("retained suffix")?
                .occurrence,
            Some(occurrence)
        );
    }
    heal(owner, admitted).await?;
    settle(owner, retry_at).await?;
    assert!(!owner.runtime.lock().await.contains_live_creature(weak));
    assert_eq!(
        only_actor(owner, model::STRONG_SOUL).await?,
        strong,
        "retry removes the old actor without a second replacement spawn"
    );
    let respawn_due = first_warning + 3_000_000;
    let final_at = retry_at.max(respawn_due);
    heal(owner, admitted).await?;
    settle(owner, final_at).await?;
    let replacement = only_actor(owner, model::GREEDBEAST).await?;
    assert_ne!(
        replacement, greedbeast,
        "the one copied death schedules one actual replacement"
    );
    settle(owner, final_at).await?;
    assert_eq!(only_actor(owner, model::GREEDBEAST).await?, replacement);
    assert!(
        owner
            .states
            .spell_states
            .lock()
            .await
            .soul_war_callbacks
            .greed_pending_for_test()
            .is_none()
    );
    late_soulsnatcher_health(owner, admitted, final_at).await?;
    Ok(())
}

async fn late_soulsnatcher_health(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    after: u64,
) -> TestResult {
    // The preceding actual Soulsnatcher callback removed itself and scheduled
    // its real replacement. Observe that queue entry instead of inventing one.
    let spawn = owner
        .states
        .spell_states
        .lock()
        .await
        .soul_war_callbacks
        .greed_spawn_due_for_test(model::SOULSNATCHER)
        .ok_or("actual Soulsnatcher return timer")?;
    let start = after.max(spawn);
    heal(owner, admitted).await?;
    settle(owner, start).await?;
    let soul = only_actor(owner, model::SOULSNATCHER).await?;
    // Lua counts source onThink callbacks, not the AI occurrence sequence:
    // movement-only passes also consume that sequence without calling onThink.
    let mut last = owner
        .states
        .spell_states
        .lock()
        .await
        .soul_war_callbacks
        .greed_observed_for_test(soul)
        .and_then(|(_, _, last)| last);
    let mut thinks = usize::from(last.is_some());
    while thinks < 7 {
        heal(owner, admitted).await?;
        last = tokio::time::timeout(NATIVE_TURN_TIMEOUT, async {
            let mut now = next_think_due(owner, soul).await?;
            loop {
                settle(owner, now).await?;
                let actual = owner
                    .states
                    .spell_states
                    .lock()
                    .await
                    .soul_war_callbacks
                    .greed_observed_for_test(soul)
                    .and_then(|(_, _, last)| last);
                if actual.is_some() && actual != last {
                    return Ok::<_, Box<dyn std::error::Error>>(actual);
                }
                // Movement-only AI passes do not count as a source Think.
                next_think_due(owner, soul).await?;
                now = next_native_melee_deadline(owner).await?;
            }
        })
        .await
        .map_err(|_| "seven genuine native Soulsnatcher Thinks")??;
        thinks += 1;
    }
    let original = next_think_due(owner, soul).await?;
    let before = owner
        .states
        .spell_states
        .lock()
        .await
        .soul_war_callbacks
        .pause_greed_before_think_for_test(soul)
        .map_err(debug)?;
    let hit = owner
        .states
        .spell_states
        .lock()
        .await
        .soul_war_callbacks
        .pause_greed_after_health_for_test()
        .map_err(debug)?;
    reach_boundary(owner, &before, original).await?;
    let actual = original + 5_000_000;
    let player = admitted
        .runtime_actor
        .ok_or("actual clock regression Player")?;
    let room = owner
        .effective_rooms()
        .gate_monster_ai()
        .ok_or("actual current Greed map")?;
    let (health_before, pass, expected_occurrence) = {
        let mut runtime = owner.runtime.lock().await;
        let mut states = owner.states.spell_states.lock().await;
        let pass = states
            .native_profile_pass
            .pending()
            .ok_or("original reserved eighth Think batch")?;
        let mut expected_occurrence = states
            .monster_melee
            .creature_ai
            .native_melee_selection(soul, 0)
            .ok_or("current native Soulsnatcher sequence")?
            .occurrence;
        expected_occurrence.sequence = expected_occurrence
            .sequence
            .checked_add(1)
            .ok_or("reserved source occurrence sequence")?;
        assert_eq!(
            states
                .native_profile_pass
                .next_due(&runtime)
                .map_err(debug)?,
            Some(original)
        );
        assert!(
            states.soul_war_callbacks.greed_pending_for_test().is_none(),
            "the original Think has not started its Greed statements yet"
        );
        // Advance the genuine current Player condition owner while its source
        // batch is retained. This is neither a condition fixture nor an HP setter.
        states.tick_in_environment(
            &mut runtime,
            room.movement_cells(),
            player,
            admitted.game_session_id,
            SemanticTimeMicros::from_micros(actual),
        );
        let state = states
            .get(&runtime, player, admitted.game_session_id)
            .ok_or("actual current Player condition clock")?;
        assert!(!state.owned_conditions().accepts_time(original));
        assert!(state.owned_conditions().accepts_time(actual));
        (state.vitals().health, pass, expected_occurrence)
    };
    reach_boundary(owner, &hit, actual).await?;
    {
        let runtime = owner.runtime.lock().await;
        let states = owner.states.spell_states.lock().await;
        assert!(
            states
                .native_profile_pass
                .pending()
                .is_some_and(|current| Arc::ptr_eq(&pass, &current))
        );
        let pending = states
            .soul_war_callbacks
            .greed_pending_for_test()
            .ok_or("actual Lua HP callback retained before removal")?;
        assert_eq!(pending.original_due, original);
        assert_eq!(pending.sampled_at, actual);
        let occurrence = pending.occurrence.ok_or("real eighth source occurrence")?;
        assert_eq!(occurrence, expected_occurrence);
        let health_after = states
            .get(&runtime, player, admitted.game_session_id)
            .ok_or("actual committed Player health")?
            .vitals()
            .health;
        let loss = health_before
            .checked_sub(health_after)
            .ok_or("actual source health loss")?;
        assert!(
            (model::SOULSNATCHER_DAMAGE_MIN..=model::SOULSNATCHER_DAMAGE_MAX).contains(&loss),
            "a late original Think must use the real current physical clock, never silently swallow stale-time addHealth: {loss}"
        );
        let id = states
            .soul_war_callbacks
            .greed_observed_for_test(soul)
            .ok_or("actual retained Soulsnatcher association")?
            .0;
        assert_eq!(
            pending.remaining,
            [GreedEffect::RemoveMonster { instance: id }]
        );
        assert!(runtime.contains_live_creature(soul));
    }
    settle(owner, actual).await?;
    assert!(!owner.runtime.lock().await.contains_live_creature(soul));
    assert_eq!(
        owner
            .states
            .spell_states
            .lock()
            .await
            .soul_war_callbacks
            .greed_spawn_due_for_test(model::SOULSNATCHER),
        Some((actual / 1_000 + 10_000) * 1_000),
        "the delayed respawn also starts at this actual callback execution time"
    );
    Ok(())
}

async fn actors(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    key: &str,
) -> TestResult<Vec<ExactActorRef>> {
    let runtime = owner.runtime.lock().await;
    Ok(runtime
        .current_live_creature_census()
        .map_err(debug)?
        .into_iter()
        .filter(|actor| runtime.matches_live_creature_identity(*actor, key.as_bytes()))
        .collect())
}

async fn only_actor(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    key: &str,
) -> TestResult<ExactActorRef> {
    let actual = actors(owner, key).await?;
    let [actor] = actual.as_slice() else {
        return Err(format!("expected one actual {key}, found {}", actual.len()).into());
    };
    Ok(*actor)
}

async fn heal(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
) -> TestResult {
    let runtime = owner.runtime.lock().await;
    let mut states = owner.states.spell_states.lock().await;
    let player = admitted.runtime_actor.ok_or("fixture current player")?;
    let maximum = states
        .get(&runtime, player, admitted.game_session_id)
        .ok_or("actual player health owner")?
        .character_facts()
        .max_health;
    states
        .apply_health_gain(
            &runtime,
            player,
            admitted.game_session_id,
            u64::from(maximum),
        )
        .ok_or("actual fixture health credit")?;
    Ok(())
}

async fn reach_boundary(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    signal: &Arc<tokio::sync::Notify>,
    now: u64,
) -> TestResult {
    drive_boundary(owner, signal, now, false).await?;
    Ok(())
}

async fn drive_to_boundary(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    signal: &Arc<tokio::sync::Notify>,
    now: u64,
) -> TestResult<bool> {
    drive_boundary(owner, signal, now, true).await
}

async fn next_think_due(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
) -> TestResult<u64> {
    owner
        .states
        .spell_states
        .lock()
        .await
        .monster_melee
        .creature_ai
        .due(actor)
        .ok_or_else(|| "actual next Greed creature Think".into())
}

async fn drive_boundary(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    signal: &tokio::sync::Notify,
    now: u64,
    allow_settled: bool,
) -> TestResult<bool> {
    let drive = async {
        loop {
            drain_ready_monster_melee_at(owner, now).await?;
            if allow_settled && next_native_melee_deadline(owner).await? > now {
                return Ok(false);
            }
            tokio::task::yield_now().await;
        }
    };
    tokio::time::timeout(NATIVE_TURN_TIMEOUT, async {
        // Keep the signal armed while driving the same semantic instant. Winning
        // the signal branch drops the paused SQL transaction and runtime guards.
        tokio::select! {
            biased;
            () = signal.notified() => Ok(true),
            result = drive => result,
        }
    })
    .await
    .map_err(|_| "actual source callback did not reach its observed boundary")?
}
