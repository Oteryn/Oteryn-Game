//! Same genuine allocated Source room, unchanged25500HP actor, native lethal owner.
//! This is settlement authority proof, not a claim to test player weapon balance.
use super::super::fixture::Harness;
use super::*;
use crate::foundation::{CommandId, CommandRef};
use crate::gameplay_transport::connection::AdmittedSession;
use crate::gameplay_transport::kill_reward::{DrainOutcome, KillSink};
const CREATURE: &str = "oteryn:creature.rakesh_moonfang";
pub(super) async fn death_and_replay(
    source: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    admitted: AdmittedSession,
    peer: AdmittedSession,
) -> TestResult {
    let owner = source
        .resolve_runtime_view(admitted)
        .await
        .map_err(|_| "resolve genuine Instance")?
        .ok_or("actual Instance owner")?;
    let actor = admitted.runtime_actor.ok_or("actual successor actor")?;
    let session = admitted.game_session_id;
    owner.refresh_quest_session(session, false).await;
    assert!(
        owner.refresh_source_party_presence(actor, session).await,
        "actual current WorldParty presence, no membership grant"
    );
    let native = owner
        .active_generation
        .and_then(|generation| generation.native_gameplay())
        .ok_or("actual active native")?;
    let row = native.reward_table().row(CREATURE).map_err(debug)?;
    assert_eq!(row.corpse_item.production_key, "oteryn:item.tibia.i43758");
    assert_eq!(
        row.xp_amount.get(),
        25_500,
        "source HP/XP are unchanged, not a fixture alias"
    );
    let (target, principal) = {
        let runtime = owner.runtime.lock().await;
        let target = runtime
            .positioned_actor_census()
            .map_err(debug)?
            .iter()
            .find_map(|(target, _, session)| {
                (session.is_none()
                    && runtime
                        .companion_snapshot(*target)
                        .is_ok_and(|view| view.state.policy.definition_key == CREATURE))
                .then_some(*target)
            })
            .ok_or("actual25500HP Rakesh actor")?;
        let view = runtime.companion_snapshot(target).map_err(debug)?;
        assert_eq!((view.health, view.maximum_health), (25_500, 25_500));
        assert_eq!(view.state.policy.reward_boss, Some(false));
        let lease = runtime
            .bound_attacker_lease(actor, session)
            .map_err(debug)?;
        (
            target,
            crate::combat::CapturedRewardPrincipal {
                character: lease.character_id(),
                lease_generation: lease.generation(),
                session,
                actor,
            },
        )
    };
    let queue = owner.states.attack.lock().await.kills().clone();
    let observed = owner.owner_now();
    let now = observed.get() / 1000;
    {
        let mut runtime = owner.runtime.lock().await;
        let result = runtime
            .borrow_exact_actor_commit_at(now)
            .commit_damage_for_bound_attacker(
                target,
                actor,
                CommandRef::new(session, CommandId::new(500).map_err(debug)?),
                0,
                crate::foundation::OwnerDamageCommand {
                    target: CREATURE.as_bytes(),
                    occurrence: session.as_bytes(),
                    binding: b"srv7-real-rakesh-native-lethal",
                    damage: 25_500,
                },
            )
            .map_err(debug)?;
        assert_eq!(
            (result.applied, result.health_before, result.health_after),
            (true, 25_500, 0)
        );
        for _ in 0..2 {
            crate::gameplay_transport::kill_reward::record_creature_kill(
                &mut runtime,
                KillSink {
                    prey: None,
                    activity: &[],
                    queue: &queue,
                    rewards: native.reward_table(),
                    charms: None,
                },
                target,
                principal,
                now,
                Some(observed),
            );
        }
    }
    assert_eq!(
        owner.drain_kill_rewards(session).await,
        DrainOutcome::Drained
    );
    // Separate guard component proof over clones of the real retained MINT
    // reservation. The actual successful native death/MINT above is its source;
    // neither the private MINT context nor the real reservation is modified.
    let mut guard_case = h.pool.begin().await?;
    sqlx::raw_sql(sqlx::AssertSqlSafe(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../tools/content-schema/native-gameplay/tests/mint-reservation-generated-key-cases.sql"
    ))))
    .execute(&mut *guard_case)
    .await?;
    guard_case.rollback().await?;
    let body:(Vec<u8>,Vec<u8>,i64)=sqlx::query_as("SELECT uuid_send(i.item_instance_id),uuid_send(r.death_instance_id),i.quantity FROM game_item_instances i JOIN game_item_mint_receipts r USING(item_instance_id) JOIN game_item_ground g USING(item_instance_id) WHERE i.definition_production_key='oteryn:item.tibia.i43758' AND r.loot_purpose_key='CORPSE_MATERIALIZATION'")
        .fetch_one(&h.pool).await?;
    let crate::foundation::RuntimeScopeRefV1::Instance { instance_id, .. } = actor.runtime_scope()
    else {
        return Err("actual Instance".into());
    };
    assert_eq!(body.1, instance_id.as_slice());
    assert_eq!(body.2, 1);
    {
        let runtime = owner.runtime.lock().await;
        let bodies = runtime.visible_corpses();
        let visible = bodies
            .iter()
            .find(|body| body.identity == target.corpse_identity())
            .ok_or("native corpse projection after committedMINT")?;
        assert_eq!(visible.item.item_instance_id.as_slice(), body.0.as_slice());
        assert_eq!(visible.item.production_key, "oteryn:item.tibia.i43758");
        assert_eq!(
            (
                visible.position.x,
                visible.position.y,
                visible.position.floor
            ),
            (33042, 32993, -8)
        );
    }
    let boss_key: String =
        sqlx::query_scalar("SELECT boss_key FROM game_character_bosstiary_kill_receipts")
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(
        &boss_key,
        &row.boss
            .as_ref()
            .ok_or("real source Bosstiary identity")?
            .key
    );
    let counts = settlement_counts(h).await?;
    assert!(counts.0 >= 1, "actual body/loot MINT receipts");
    assert_eq!(counts.1, 1, "real ordinary XP once");
    assert_eq!(counts.2, 1, "real Bosstiary once");
    {
        let mut runtime = owner.runtime.lock().await;
        crate::gameplay_transport::kill_reward::record_creature_kill(
            &mut runtime,
            KillSink {
                prey: None,
                activity: &[],
                queue: &queue,
                rewards: native.reward_table(),
                charms: None,
            },
            target,
            principal,
            now,
            Some(observed),
        );
    }
    assert_eq!(
        owner.drain_kill_rewards(session).await,
        DrainOutcome::Drained
    );
    assert_eq!(
        settlement_counts(h).await?,
        counts,
        "original UUIDs and all durable ordinary rewards survive exact death replay"
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_boss_reward_deaths")
            .fetch_one(&h.pool)
            .await?,
        0,
        "sourcefalse Rakesh must not produce a private reward-boss chest"
    );
    let body_id: [u8; 16] = body.0.try_into().map_err(|_| "actual body UUID")?;
    let picked = super::pickup::pickup(&owner, h, admitted, body_id).await?;
    super::retirement::return_and_retire(source, &owner, h, admitted, peer, body_id, picked)
        .await?;
    Ok(())
}
async fn settlement_counts(h: &Harness) -> Result<(i64, i64, i64), Box<dyn std::error::Error>> {
    Ok(sqlx::query_as("SELECT(SELECT count(*) FROM game_item_mint_receipts),(SELECT count(*) FROM game_character_xp_receipts),(SELECT count(*) FROM game_character_bosstiary_kill_receipts)").fetch_one(&h.pool).await?)
}
