use crate::bestiary_postgres_harness::{Harness, TestResult, debug};
use crate::foundation::{
    CommandId, CommandRef, ExactActorRef, GameSessionId, MovementLocalPosition,
};
use crate::gameplay_transport::{
    ComposedFreshAdmission,
    connection::{AdmittedSession, FirstEntryOutcome, SessionContinuity},
    kill_reward::{DrainOutcome, KillSink, record_creature_kill},
};
use std::sync::Arc;

const CREATURE: &str = "oteryn:creature.goblin_leader";
const TRACK: &str = "oteryn:quest-progress/quest/u8_1/tower_defence_quest/questline";

#[test]
fn qtowerdef_goblin_leader_credits_only_the_higher_experience_recipient() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::party_xp::run(super::party_xp::Case::TowerDefence)
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    players: &[(ExactActorRef, GameSessionId)],
) -> TestResult {
    let [principal, winner] = players else {
        return Err("two live quest participants required".into());
    };
    let loaded = oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
        .map_err(debug)?;
    owner.quest_catalogue = Some(Arc::new(loaded.catalogue().clone()));
    let mut characters = Vec::new();
    for (index, &(actor, session)) in players.iter().enumerate() {
        let admitted = AdmittedSession {
            game_session_id: session,
            world_id: owner.world_id,
            channel_id: owner.channel_id,
            runtime_actor: Some(actor),
            first_entry: FirstEntryOutcome::NotApplicable,
            controller: None,
            continuity: SessionContinuity::FRESH,
            item_fence: None,
        };
        let fence = owner
            .current_quest_fence(session)
            .await
            .map_err(|_| "fence")?
            .ok_or("quest fence")?;
        characters.push(fence.character_id);
        super::qengine_outfit_stories::seed_quest_prerequisites_in_pool(
            &h.pool,
            owner,
            &admitted,
            &[(TRACK.into(), 2)],
            0xac + u8::try_from(index)?,
        )
        .await?;
        owner.admit_quest_session(&admitted).await;
        assert_eq!(stage(h, fence.character_id).await?, 2);
    }
    let [principal_id, winner_id] = characters.as_slice() else {
        return Err("two quest characters required".into());
    };
    // The damage leader's exhausted stamina zeros their XP. The other player
    // must win even though the lethal blow and most damage both came from the leader.
    sqlx::query("UPDATE game_character_build_state SET stamina_minutes=0 WHERE character_id=encode($1,'hex')::uuid")
        .bind(principal_id.as_bytes().as_slice()).execute(&h.pool).await?;
    let queue = owner.states.attack.lock().await.kills().clone();
    let observed = owner.owner_now();
    let now = observed.get() / 1000;
    let rewards = owner
        .active_generation
        .and_then(|generation| generation.native_gameplay())
        .ok_or("native rewards")?
        .reward_table();
    {
        let mut runtime = owner.runtime.lock().await;
        owner
            .states
            .spell_states
            .lock()
            .await
            .load_stamina(&runtime, principal.0, principal.1, 0);
        let health = runtime
            .companion_policy(CREATURE)
            .map_err(debug)?
            .maximum_health;
        let at = runtime
            .read_actor_position(principal.0)
            .map_err(debug)?
            .position();
        let target = runtime
            .admit_encounter_actor(
                CREATURE,
                health,
                MovementLocalPosition {
                    x: at.x,
                    y: at.y + 1,
                    floor: at.floor,
                },
            )
            .map_err(debug)?;
        runtime
            .install_creature_policy(target, CREATURE)
            .map_err(debug)?;
        for &((actor, session), damage) in
            &[(*winner, health / 4), (*principal, health - health / 4)]
        {
            runtime
                .borrow_exact_actor_commit_at(now)
                .commit_damage_for_bound_attacker(
                    target,
                    actor,
                    CommandRef::new(session, CommandId::new(110).map_err(debug)?),
                    0,
                    crate::foundation::OwnerDamageCommand {
                        target: CREATURE.as_bytes(),
                        occurrence: session.as_bytes(),
                        binding: b"tower-defence-goblin-leader",
                        damage,
                    },
                )
                .map_err(debug)?;
        }
        let lease = runtime
            .bound_attacker_lease(principal.0, principal.1)
            .map_err(debug)?;
        record_creature_kill(
            &mut runtime,
            KillSink {
                prey: None,
                activity: &[],
                queue: &queue,
                rewards,
                charms: None,
            },
            target,
            crate::combat::CapturedRewardPrincipal {
                character: lease.character_id(),
                lease_generation: lease.generation(),
                session: principal.1,
                actor: principal.0,
            },
            now,
            Some(observed),
        );
    }
    assert_eq!(
        owner.drain_kill_rewards(principal.1).await,
        DrainOutcome::Drained
    );
    assert_eq!(
        owner.drain_kill_rewards(winner.1).await,
        DrainOutcome::Drained
    );
    let awards: Vec<(Vec<u8>, i64)> = sqlx::query_as(
        "SELECT uuid_send(character_id), xp_awarded FROM game_party_kill_xp_members ORDER BY xp_awarded DESC",
    ).fetch_all(&h.pool).await?;
    let damage_principal: Vec<u8> =
        sqlx::query_scalar("SELECT uuid_send(principal_id) FROM game_party_kill_xp_receipts")
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(damage_principal, principal_id.as_bytes().to_vec());
    assert_eq!(awards.len(), 2);
    assert_eq!(awards[0].0.as_slice(), winner_id.as_bytes());
    assert!(awards[0].1 > 0);
    assert_eq!(awards[1], (principal_id.as_bytes().to_vec(), 0));
    assert_eq!(stage(h, *principal_id).await?, 2);
    assert_eq!(stage(h, *winner_id).await?, 3);
    Ok(())
}

async fn stage(h: &Harness, character: crate::domain::CharacterId) -> TestResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2",
    ).bind(character.as_bytes().as_slice()).bind(TRACK).fetch_one(&h.pool).await?)
}
