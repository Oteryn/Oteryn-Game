//! Native endgame death descendants after the real command-178 Instance handoff.
use super::*;
use crate::foundation::runtime_actor_spell_types::{
    OwnerCombatBatch, OwnerCombatChange, OwnerCombatEffect, SpellOccurrenceBinding,
};
use crate::foundation::{CommandId, CommandRef};
use crate::gameplay_transport::kill_reward::{DrainOutcome, KillSink, record_batch_kills};

/// Qualify later writes against a real receipt chain, rather than a raw revision import.
pub(super) async fn prepare_level(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
) -> TestResult {
    use crate::durability::character_progression::{
        ExperienceAwardRequest, ExperienceCommitOutcome, ExperienceRewardOccurrence,
    };
    let mut fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|_| "source level fence")?
        .ok_or("source session")?;
    let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
    fence.expected_character_revision = slot
        .cursor(owner.root, owner.character)
        .await
        .map_err(debug)?;
    let before = owner
        .root
        .read_character_progression(owner.character, fence.character_id)
        .await?
        .ok_or("initialized source progression")?;
    let binding = owner
        .progression_binding(admitted.game_session_id)
        .ok_or("source progression policy")?;
    let total = binding
        .policy
        .thresholds
        .iter()
        .find(|threshold| threshold.level == 250)
        .ok_or("source level 250 threshold")?
        .minimum_experience
        .get();
    let result = owner
        .root
        .commit_character_experience(
            owner.character,
            owner.holder,
            fence,
            ExperienceAwardRequest {
                occurrence: ExperienceRewardOccurrence::from_bytes(fixture::id(228))
                    .map_err(debug)?,
                amount: oteryn_simulation_determinism::ExactI64::new(
                    total - before.total_experience.get(),
                ),
                context: binding.context.clone(),
                policy_revision: binding.policy_revision.clone(),
                reward_revision: binding.reward_revision.clone(),
                policy: binding.policy.clone(),
            },
        )
        .await;
    slot.observe_party_experience(&result);
    assert!(matches!(
        result.map_err(debug)?,
        ExperienceCommitOutcome::Committed(_)
    ));
    Ok(())
}

pub(super) async fn settle(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    policy: &crate::foundation::CompiledCreaturePolicy,
    case: Case,
) -> TestResult {
    let started = std::time::Instant::now();
    let fence = admitted.item_fence.ok_or("actual Instance fence")?;
    let RuntimeScopeRefV1::Instance { instance_id, .. } = fence.runtime_scope else {
        return Err("real Instance admission required".into());
    };
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native gameplay")?;
    let actor = admitted.runtime_actor.ok_or("transferred actor")?;
    assert!(
        owner
            .progression_binding(admitted.game_session_id)
            .is_some(),
        "progression follows real handoff"
    );
    // The migration preserves the source truth tables when applied twice.
    sqlx::raw_sql(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../apps/game-server/migrations/1023_native_encounter_progression_scope.sql"
    )))
    .execute(&h.pool)
    .await?;
    if case == Case::SoulWarLateDefeat {
        let (deadline, now): (i64, i64) = sqlx::query_as(
            "SELECT deadline,floor(extract(epoch FROM clock_timestamp())*1000)::bigint FROM game_boss_room_activities",
        ).fetch_one(&h.pool).await?;
        tokio::time::sleep(std::time::Duration::from_millis(
            u64::try_from(deadline.saturating_sub(now).max(0))? + 5,
        ))
        .await;
    }
    let now = owner.owner_now();
    {
        let mut runtime = owner.runtime.lock().await;
        assert_eq!(runtime.binding().runtime_scope(), fence.runtime_scope);
        let target = runtime
            .visible_entities()
            .creatures
            .first()
            .ok_or("actual room boss")?
            .actor;
        let batch = OwnerCombatBatch {
            caster: actor,
            attacker: crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                .map_err(debug)?,
            current_lease_generation: fence.character_lease_generation,
            command: CommandRef::new(admitted.game_session_id, CommandId::new(91).map_err(debug)?),
            occurrence: SpellOccurrenceBinding {
                id: "boss-activity:reward-lethal".into(),
                revisions: ["r1", "r1", "r1", "r1", "r1"].map(str::to_owned),
            },
            binding: b"boss-activity-reward-lethal".to_vec(),
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
        assert!(receipt.effects.iter().any(|effect| {
            effect.target == target && effect.health.is_some_and(|health| health.health_after == 0)
        }));
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
        // Projection is the death owner's next step after the lethal vitals
        // commit. Retaining this native proof does not authorize the separately
        // refused ordinary Instance corpse/loot MINT descendant.
        assert!(runtime.retains_corpse_projection(target));
        assert!(attack.kills().has_session(admitted.game_session_id));
    }
    assert_eq!(
        owner.drain_kill_rewards(admitted.game_session_id).await,
        DrainOutcome::Drained
    );

    let keys: Vec<Vec<u8>> =
        sqlx::query_scalar("SELECT death_key FROM game_party_kill_xp_receipts")
            .fetch_all(&h.pool)
            .await?;
    assert_eq!(
        keys.len(),
        1,
        "native Instance kill has one atomic XP occurrence"
    );
    assert_eq!(keys[0].len(), 53);
    assert_eq!(keys[0][0], 2);
    assert_eq!(&keys[0][17..33], instance_id.as_slice());
    if case == Case::PrimalDefeat {
        let unlocked: i16 = sqlx::query_scalar("SELECT unlocked FROM game_character_hazard")
            .fetch_one(&h.pool)
            .await?;
        assert_eq!(
            unlocked, 2,
            "native Primal victory advances the selected maximum"
        );
    } else if case == Case::RottenBloodDefeat {
        let completed: bool =
            sqlx::query_scalar("SELECT completed FROM game_character_rotten_blood")
                .fetch_one(&h.pool)
                .await?;
        assert!(
            completed,
            "real Bakragore death completes the admitted chain"
        );
    } else {
        let (flags, completed): (i16, bool) = sqlx::query_as(
            "SELECT defeated_bosses,megalomania_completed FROM game_character_soul_war",
        )
        .fetch_one(&h.pool)
        .await?;
        if case == Case::SoulWarLateDefeat {
            assert_eq!(flags, 0, "late native death cannot earn a room victory");
            assert!(!completed);
        } else if case == Case::SoulWarDefeat {
            assert_eq!(flags, 4);
        } else {
            assert!(completed);
        }
    }
    assert_eq!(
        owner.drain_kill_rewards(admitted.game_session_id).await,
        DrainOutcome::Drained
    );
    let receipt_sql = if case == Case::PrimalDefeat {
        "SELECT count(*),COALESCE(bool_and(r.created_xact_id=d.created_xact_id),true) FROM game_character_primal_hazard_receipts r JOIN game_party_kill_xp_receipts d USING(death_key) WHERE r.death_key=$1"
    } else if case == Case::RottenBloodDefeat {
        "SELECT count(*),COALESCE(bool_and(r.created_xact_id=d.created_xact_id),true) FROM game_character_rotten_blood_receipts r JOIN game_party_kill_xp_receipts d ON d.death_key=r.source_death_key WHERE r.source_death_key=$1"
    } else {
        "SELECT count(*),COALESCE(bool_and(r.created_xact_id=d.created_xact_id),true) FROM game_character_soul_war_receipts r JOIN game_party_kill_xp_receipts d ON d.death_key=r.source_death_key WHERE r.source_death_key=$1"
    };
    let (receipt_count, atomic): (i64, bool) = sqlx::query_as(sqlx::AssertSqlSafe(receipt_sql))
        .bind(&keys[0])
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(
        receipt_count,
        if case == Case::SoulWarLateDefeat {
            0
        } else {
            1
        }
    );
    assert!(
        atomic,
        "progression and native XP commit in the same transaction"
    );
    if case != Case::SoulWarLateDefeat {
        let current = owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(|_| "replay fence")?
            .ok_or("replay session")?;
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
            .execute(&mut *tx)
            .await?;
        let participants = [(current, true)];
        crate::durability::quest_state::soul_war::apply_soul_war_kill_in_transaction(
            &mut tx,
            owner.holder,
            &keys[0],
            &policy.definition_key,
            &participants,
        )
        .await
        .map_err(debug)?;
        crate::durability::quest_state::rotten_blood::apply_rotten_blood_kill_in_transaction(
            &mut tx,
            owner.holder,
            &keys[0],
            &policy.definition_key,
            &participants,
        )
        .await
        .map_err(debug)?;
        crate::durability::character_hazard::apply_hazard_kill_in_transaction(
            &mut tx,
            owner.holder,
            &keys[0],
            &policy.definition_key,
            &participants,
        )
        .await
        .map_err(debug)?;
        tx.commit().await?;
        let retained: (i64, bool) = sqlx::query_as(sqlx::AssertSqlSafe(receipt_sql))
            .bind(&keys[0])
            .fetch_one(&h.pool)
            .await?;
        assert_eq!(
            retained,
            (receipt_count, atomic),
            "persisted 53-byte source replay is idempotent"
        );
    }
    let (leveled, base): (bool, i64) = sqlx::query_as("SELECT EXISTS(SELECT 1 FROM game_character_xp_receipts x JOIN game_party_kill_xp_members m ON m.reward_occurrence_id=x.reward_occurrence_id WHERE x.level_after>x.level_before),(SELECT base_xp FROM game_party_kill_xp_receipts)")
        .fetch_one(&h.pool).await?;
    assert_eq!(
        u64::try_from(base)?,
        native
            .reward_table()
            .encounter_facts(&policy.definition_key)
            .and_then(|facts| facts.experience)
            .ok_or("source experience")?
    );
    if leveled {
        assert!(
            owner.states.level_refresh.is_marked(fence.character_id),
            "native encounter level effects stay pending for the current actor"
        );
    }
    let loot: (i64, i64) = sqlx::query_as("SELECT (SELECT count(*) FROM game_item_corpse_container_entries),(SELECT count(*) FROM game_character_boss_reward_chests)")
        .fetch_one(&h.pool).await?;
    assert_eq!(
        loot,
        (0, 0),
        "quest XP cannot fabricate corpse or personal loot admission"
    );
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_party_kill_xp_receipts")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(count, 1, "second drain cannot mint another death");
    eprintln!(
        "L3_INSTANCE_ENDGAME POSTGRES_CASE_COMPLETED {} elapsed_ms={}",
        policy.definition_key,
        started.elapsed().as_millis()
    );
    Ok(())
}
