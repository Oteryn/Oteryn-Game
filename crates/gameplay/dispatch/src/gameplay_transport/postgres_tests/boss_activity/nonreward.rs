//! A source-qualified ordinary creature used as the isolated room's primary
//! boss. Its committed death drives W02 independently of personal reward groups
//! and the explicitly unavailable ordinary Instance corpse-MINT descendant.
use super::*;
use crate::durability::activity_handoff::ActivityReturnReason;
use crate::foundation::runtime_actor_spell_types::{
    OwnerCombatBatch, OwnerCombatChange, OwnerCombatEffect, SpellOccurrenceBinding,
};
use crate::foundation::{CommandId, CommandRef};
use crate::gameplay_transport::kill_reward::{DrainOutcome, KillSink, record_batch_kills};
use crate::gameplay_transport::native_activity::NativeActivityOutcome;

pub(super) fn source_creature(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
) -> TestResult<(String, crate::foundation::CompiledCreaturePolicy)> {
    let native = owner
        .active_generation
        .and_then(|generation| generation.native_gameplay())
        .ok_or("active native gameplay")?;
    let policy = native
        .policies()
        .policies()
        .iter()
        .find(|policy| {
            policy.definition_key == "oteryn:creature.rat" && policy.reward_boss == Some(false)
        })
        .ok_or("unchanged ordinary source rat")?;
    native
        .reward_table()
        .row(&policy.definition_key)
        .map_err(debug)?;
    Ok((BOSS.to_owned(), policy.as_ref().clone()))
}

fn unix_ms() -> TestResult<i64> {
    Ok(i64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_millis(),
    )?)
}

pub(super) async fn assert_defeat_and_return(
    h: &Harness,
    origin: &ComposedFreshAdmission<'_, '_, '_>,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    policy: &crate::foundation::CompiledCreaturePolicy,
) -> TestResult {
    let fence = admitted.item_fence.ok_or("real Instance fence")?;
    assert!(matches!(
        fence.runtime_scope,
        RuntimeScopeRefV1::Instance { .. }
    ));
    assert_eq!(policy.reward_boss, Some(false));
    let actor = admitted.runtime_actor.ok_or("transferred native actor")?;
    let native = owner
        .active_generation
        .and_then(|generation| generation.native_gameplay())
        .ok_or("current native gameplay")?;
    let deadline: i64 = sqlx::query_scalar("SELECT deadline FROM game_boss_room_activities")
        .fetch_one(&h.pool)
        .await?;
    let earliest_death = unix_ms()?;
    let now = owner.owner_now();
    let batch = {
        let mut runtime = owner.runtime.lock().await;
        assert_eq!(runtime.binding().runtime_scope(), fence.runtime_scope);
        let target = runtime
            .visible_entities()
            .creatures
            .first()
            .ok_or("primary native boss")?
            .actor;
        let batch = OwnerCombatBatch {
            caster: actor,
            attacker: crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                .map_err(debug)?,
            current_lease_generation: fence.character_lease_generation,
            command: CommandRef::new(admitted.game_session_id, CommandId::new(91).map_err(debug)?),
            occurrence: SpellOccurrenceBinding {
                id: "boss-activity:ordinary-lethal".into(),
                revisions: ["r1", "r1", "r1", "r1", "r1"].map(str::to_owned),
            },
            binding: b"boss-activity-ordinary-lethal".to_vec(),
            anchor: None,
            now_us: Some(now),
            now_ms: now.get() / 1_000,
            effects: vec![OwnerCombatEffect {
                target,
                sub_ordinal: 0,
                change: OwnerCombatChange::Damage {
                    target_atom: runtime.creature_spell_target_atom(target).map_err(debug)?,
                    magnitude: policy.maximum_health,
                },
            }],
            deferred: None,
        };
        let staged = runtime.stage_spell_batch(&batch).map_err(debug)?;
        let receipt = runtime.commit_spell_batch(staged).map_err(debug)?;
        assert!(receipt.applied);
        assert!(
            receipt.effects.iter().any(|effect| effect.target == target
                && effect
                    .health
                    .is_some_and(|health| health.health_before > 0 && health.health_after == 0)),
            "native commit retains the exact lethal target before corpse projection"
        );
        let attack = owner.states.attack.lock().await;
        record_batch_kills(
            &mut runtime,
            KillSink {
                prey: None,
                activity: &[],
                queue: attack.kills(),
                rewards: native.reward_table(),
                charms: None,
            },
            &receipt,
            now.get() / 1_000,
        );
        assert!(runtime.retains_corpse_projection(target));
        assert!(attack.kills().has_session(admitted.game_session_id));
        batch
    };
    let latest_death = unix_ms()?;
    assert!(
        latest_death <= deadline,
        "native lethal preceded the fight deadline"
    );
    // Delay SQL beyond the fight deadline. The actual lethal clock, retained in
    // the existing queue, must still determine victory and the after-kill timer.
    tokio::time::sleep(std::time::Duration::from_millis(
        u64::try_from(deadline.saturating_sub(unix_ms()?).max(0))? + 5,
    ))
    .await;
    assert_eq!(
        owner.drain_kill_rewards(admitted.game_session_id).await,
        DrainOutcome::Drained
    );
    type DefeatRow = (i16, Vec<u8>, i64, i64);
    let recorded: DefeatRow = sqlx::query_as(
        "SELECT state,uuid_send(defeat_id),terminal_at,exit_deadline FROM game_boss_room_activities",
    ).fetch_one(&h.pool).await?;
    assert_eq!(recorded.0, 2);
    assert_eq!(recorded.1.len(), 16);
    assert!(recorded.1.iter().any(|byte| *byte != 0));
    assert!((earliest_death..=latest_death).contains(&recorded.2));
    assert_eq!(recorded.3, recorded.2 + 1_000);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_boss_reward_deaths")
            .fetch_one(&h.pool)
            .await?,
        0,
        "ordinary primary boss creates no W03 reward group"
    );

    // A replayed native receipt after the deadline must reuse the original
    // clock, so the recorded victory is neither replaced nor stuck as Conflict.
    {
        let mut runtime = owner.runtime.lock().await;
        let staged = runtime.stage_spell_batch(&batch).map_err(debug)?;
        let replay = runtime.commit_spell_batch(staged).map_err(debug)?;
        assert!(!replay.applied);
        let attack = owner.states.attack.lock().await;
        record_batch_kills(
            &mut runtime,
            KillSink {
                prey: None,
                activity: &[],
                queue: attack.kills(),
                rewards: native.reward_table(),
                charms: None,
            },
            &replay,
            now.get() / 1_000,
        );
        assert!(attack.kills().has_session(admitted.game_session_id));
    }
    assert_eq!(
        owner.drain_kill_rewards(admitted.game_session_id).await,
        DrainOutcome::Drained
    );
    let replayed: DefeatRow = sqlx::query_as(
        "SELECT state,uuid_send(defeat_id),terminal_at,exit_deadline FROM game_boss_room_activities",
    ).fetch_one(&h.pool).await?;
    assert_eq!(replayed, recorded);
    let result = owner
        .return_due_activity(admitted, ActivityReturnReason::Defeated)
        .await
        .map_err(|_| "actual after-kill return")?;
    let NativeActivityOutcome::Committed(commit) = result else {
        return Err("after-kill return refused".into());
    };
    let next = commit
        .connections
        .first()
        .ok_or("actual origin successor")?
        .1;
    assert_ne!(next.game_session_id, admitted.game_session_id);
    assert!(matches!(
        next.item_fence.ok_or("returned fence")?.runtime_scope,
        RuntimeScopeRefV1::Channel { .. }
    ));
    origin
        .publish_activity_commit(&commit)
        .map_err(|_| "publish actual return")?;
    assert!(
        owner
            .runtime
            .lock()
            .await
            .player_control_facts(actor, admitted.game_session_id)
            .is_err()
    );
    assert!(
        origin
            .runtime
            .lock()
            .await
            .player_control_facts(
                next.runtime_actor.ok_or("returned actor")?,
                next.game_session_id
            )
            .is_ok()
    );
    assert_eq!(sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM game_activity_runtime_returns WHERE reason=3 AND command_id IS NULL",
    ).fetch_one(&h.pool).await?, 1, "server after-kill return has no synthetic client command");
    assert_eq!(
        sqlx::query_scalar::<_, i16>("SELECT state FROM game_boss_room_activities")
            .fetch_one(&h.pool)
            .await?,
        4
    );
    eprintln!("BOSS_ACTIVITY_NONREWARD_DEATH_AFTER_KILL_RETURN POSTGRES_CASE_COMPLETED");
    Ok(())
}
