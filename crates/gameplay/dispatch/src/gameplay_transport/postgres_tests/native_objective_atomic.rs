//! Isolated native XP/objective owner proof. Prior route progress is a fixture
//! precondition; this does not qualify Elvenbane traversal, combat or rewards.
use super::{AdmittedSession, ComposedFreshAdmission, TestResult, fixture::debug};
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::foundation::{CommandId, CommandRef, MovementLocalPosition};
use oteryn_gameplay_quests::quest::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTransition,
    kill_objectives as rules,
};

#[test]
fn native_objective_and_xp_share_one_postgres_transaction() -> TestResult {
    super::annihilator_route::native_objective_atomicity()
}

pub(super) async fn scenario(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    member: &mut AdmittedSession,
    map: &crate::map::boot::BundleMap,
    pool: &sqlx::PgPool,
) -> TestResult {
    let started = std::time::Instant::now();
    let version: i32 = sqlx::query_scalar("SELECT current_setting('server_version_num')::int")
        .fetch_one(pool)
        .await?;
    eprintln!("NATIVE_OBJECTIVE_ATOMIC POSTGRES_CASE_OBSERVED server_version_num={version}");
    let catalogue = owner.quest_catalogue.as_ref().ok_or("catalogue")?;
    let mut tracks = vec![catalogue.track(rules::PHASE).ok_or("phase track")?.clone()];
    let mut transitions = Vec::new();
    for rule in rules::OBJECTIVES {
        transitions.push(
            catalogue
                .transition(rule.transition)
                .ok_or("native binding")?
                .clone(),
        );
        if let Some((counter, _)) = rule.counter {
            tracks.push(catalogue.track(counter).ok_or("counter")?.clone());
        }
    }
    const PRECONDITION: &str = "oteryn:quest-transition/elvenbane/fixture-prior-route";
    transitions.push(QuestTransition {
        key: PRECONDITION.into(),
        quest: rules::QUEST.into(),
        effects: vec![QuestEffect {
            track: rules::PHASE.into(),
            from: QuestComparison::Eq(0),
            effect: QuestEffectKind::Set(2),
        }],
        completes: false,
        experience: None,
    });
    owner.quest_catalogue = Some(std::sync::Arc::new(
        QuestStateCatalogue::new(catalogue.content_revision(), tracks, transitions)
            .map_err(debug)?,
    ));
    let fence = owner
        .current_quest_fence(member.game_session_id)
        .await
        .map_err(|_| "quest fence")?
        .ok_or("session")?;
    let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
    let result = slot
        .commit_quest_transition(
            owner.root,
            owner.character,
            owner.holder,
            fence,
            QuestTransitionRequest {
                transition_key: PRECONDITION.into(),
                cause: QuestCause::Command(CommandRef::new(
                    member.game_session_id,
                    CommandId::new(9000).map_err(debug)?,
                )),
            },
            owner.quest_catalogue.as_ref().ok_or("catalogue")?.clone(),
        )
        .await
        .map_err(debug)?;
    assert!(matches!(result, QuestTransitionOutcome::Committed(_)));
    drop(slot);
    owner
        .refresh_quest_session(member.game_session_id, true)
        .await;
    {
        let sessions = owner
            .states
            .quest_sessions
            .lock()
            .map_err(|_| "quest sessions")?;
        let copy = sessions
            .get(&member.game_session_id)
            .and_then(|session| session.copy.as_ref())
            .ok_or("admitted quest copy")?;
        assert_eq!(
            copy.tracks().get(rules::PHASE).copied(),
            Some(2),
            "the isolated prior-route precondition must be admitted before death"
        );
    }

    let native = owner
        .active_generation
        .ok_or("generation")?
        .native_gameplay()
        .ok_or("native")?;
    let target_at = MovementLocalPosition {
        x: 32590,
        y: 31645,
        floor: 5,
    };
    use crate::content::{CollisionClass, LogicalCell};
    assert_eq!(
        owner.movement_cells.index().lookup(
            owner.movement_cells.scope(),
            LogicalCell {
                x: target_at.x,
                y: target_at.y,
                z: i32::from(target_at.floor)
            }
        ),
        Ok(CollisionClass::Walkable)
    );
    assert!(!map.facts.quest_callback_blocked(
        map.overlay.base(),
        crate::map::overlay::TilePos {
            x: 32590,
            y: 31645,
            floor: -5
        },
        false
    ));
    let queue = owner.states.attack.lock().await.kills().clone();
    let now = owner.owner_now();
    let mut runtime = owner.runtime.lock().await;
    let pin = runtime.content_pin().clone();
    runtime
        .realize_native_qualification_spawn(pin, "oteryn:creature.dragon", target_at)
        .map_err(debug)?;
    let target = runtime
        .visible_entities()
        .creatures
        .into_iter()
        .find(|row| row.position == target_at)
        .ok_or("actual source dragon")?
        .actor;
    let snapshot = runtime.companion_snapshot(target).map_err(debug)?;
    assert_eq!(snapshot.health, snapshot.maximum_health);
    let actor = member.runtime_actor.ok_or("actor")?;
    let lease = runtime
        .bound_attacker_lease(actor, member.game_session_id)
        .map_err(debug)?;
    let principal = crate::combat::CapturedRewardPrincipal {
        character: lease.character_id(),
        lease_generation: lease.generation(),
        actor,
        session: member.game_session_id,
    };
    // This fixture exercises the native damage/XP/Quest transaction seam, not a
    // client combat route. The full route must independently earn its kills.
    runtime
        .borrow_exact_actor_commit_at(now.get() / 1000)
        .commit_damage_for_bound_attacker(
            target,
            actor,
            CommandRef::new(member.game_session_id, CommandId::new(9001).map_err(debug)?),
            0,
            crate::foundation::OwnerDamageCommand {
                target: b"oteryn:creature.dragon",
                occurrence: b"native-objective-atomic",
                binding: b"native-objective-atomic",
                damage: snapshot.maximum_health,
            },
        )
        .map_err(debug)?;
    crate::gameplay_transport::kill_reward::record_creature_kill(
        &mut runtime,
        crate::gameplay_transport::kill_reward::KillSink {
            prey: None,
            activity: &[],
            queue: &queue,
            rewards: native.reward_table(),
            charms: None,
        },
        target,
        principal,
        now.get() / 1000,
        Some(now),
    );
    drop(runtime);
    let entry = match queue.take(member.game_session_id, principal.lease_generation) {
        crate::gameplay_transport::kill_reward::TakeOutcome::Taken(held) => {
            held.entry().cloned().ok_or("native death")?
        }
        _ => return Err("native death must enter existing kill queue".into()),
    };
    assert_eq!(
        entry.objectives.as_ref().map(|v| v.len()),
        Some(1),
        "freeze one genuine native objective"
    );
    let lane =
        crate::durability::spell_owner_commit::SpellLane::new(member.world_id, member.channel_id);
    let permit = lane.acquire().await.map_err(|_| "spell lane")?;
    for _ in 0..2 {
        let settled = owner
            .settle_party_experience(&permit, &entry)
            .await
            .map_err(debug)?;
        assert!(
            settled.xp.is_ok(),
            "native XP owner must accept actual captured death"
        );
        drop(settled);
    }
    let phase: i64 = sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2")
        .bind(fence.character_id.as_bytes().as_slice()).bind(rules::PHASE).fetch_one(pool).await?;
    assert_eq!(
        phase, 3,
        "native XP owner must atomically commit its captured objective"
    );
    let death = crate::durability::party_kill_xp::encode_death_key(entry.facts.death);
    let same_transaction: bool = sqlx::query_scalar("SELECT d.xmin=q.xmin FROM game_party_kill_xp_receipts d JOIN game_character_quest_receipts q ON q.character_id=d.principal_id WHERE d.death_key=$1 AND q.transition_key=$2")
        .bind(death).bind(rules::OBJECTIVES[0].transition).fetch_one(pool).await?;
    assert!(
        same_transaction,
        "native objective and XP cannot have a crash gap"
    );
    eprintln!(
        "NATIVE_OBJECTIVE_ATOMIC POSTGRES_CASE_COMPLETED source_dragon=1 replay=stable same_xact=true elapsed_ms={}",
        started.elapsed().as_millis()
    );
    Ok(())
}
