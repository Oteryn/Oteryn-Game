//! Native endgame death descendants after the real command-178 Instance handoff.
use super::*;
use crate::foundation::runtime_actor_spell_types::{
    OwnerCombatBatch, OwnerCombatChange, OwnerCombatEffect, SpellOccurrenceBinding,
};
use crate::foundation::{CommandId, CommandRef};
use crate::gameplay_transport::kill_reward::{DrainOutcome, KillSink, record_batch_kills};

pub(super) async fn settle(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    policy: &crate::foundation::CompiledCreaturePolicy,
    case: Case,
) -> TestResult {
    let fence = admitted.item_fence.ok_or("actual Instance fence")?;
    let RuntimeScopeRefV1::Instance { instance_id, .. } = fence.runtime_scope else {
        return Err("real Instance admission required".into());
    };
    let native = owner.active_generation.and_then(|g| g.native_gameplay()).ok_or("native gameplay")?;
    let actor = admitted.runtime_actor.ok_or("transferred actor")?;
    assert!(owner.progression_binding(admitted.game_session_id).is_some(), "progression follows real handoff");
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

    let keys: Vec<Vec<u8>> = sqlx::query_scalar("SELECT death_key FROM game_party_kill_xp_receipts")
        .fetch_all(&h.pool).await?;
    assert_eq!(keys.len(), 1, "native Instance kill has one atomic XP occurrence");
    assert_eq!(keys[0].len(), 53);
    assert_eq!(keys[0][0], 2);
    assert_eq!(&keys[0][17..33], instance_id.as_slice());
    if case == Case::RottenBloodDefeat {
        let completed: bool = sqlx::query_scalar("SELECT completed FROM game_character_rotten_blood")
            .fetch_one(&h.pool).await?;
        assert!(completed, "real Bakragore death completes the admitted chain");
    } else {
        let (flags, completed): (i16, bool) = sqlx::query_as("SELECT defeated_bosses,megalomania_completed FROM game_character_soul_war")
            .fetch_one(&h.pool).await?;
        if case == Case::SoulWarDefeat { assert_eq!(flags, 4); }
        else { assert!(completed); }
    }
    assert_eq!(owner.drain_kill_rewards(admitted.game_session_id).await, DrainOutcome::Drained);
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_party_kill_xp_receipts")
        .fetch_one(&h.pool).await?;
    assert_eq!(count, 1, "second drain cannot mint another death");
    eprintln!("L3_INSTANCE_ENDGAME POSTGRES_CASE_COMPLETED {}", policy.definition_key);
    Ok(())
}
