use super::*;
use crate::gameplay_transport::kill_reward::{DrainOutcome, KillSink};

/// Native owner damage is the accepted lethal boundary used by the existing
/// proficiency/party kill integration suites. No quest kill writer is called here.
pub(super) async fn kill(owner: &ComposedFreshAdmission<'_, '_, '_>, h: &Harness,
    actor: ExactActorRef, session: GameSessionId, target: ExactActorRef,
    boss: &quest::GrizzlyBoss) -> TestResult {
    let before = ordinary_values(h).await?;
    capture(owner, actor, session, target, &boss.creature_key, 500).await?;
    assert_eq!(owner.drain_kill_rewards(session).await, DrainOutcome::Drained);
    assert_eq!(value(owner, boss.kill_storage).await?, 1);
    assert_eq!(owner.drain_kill_rewards(session).await, DrainOutcome::Drained);
    assert_eq!(value(owner, boss.kill_storage).await?, 1);
    assert_eq!(receipts(h, &Action::BossKill { boss_id: boss.id, observed_revision: 1 }.transition_key()).await?, 1,
        "lethal callback and drain replay credit only once");
    assert_eq!(ordinary_values(h).await?, before, "Q11 fallback never mints loot or ordinary XP");
    assert!(!owner.runtime.lock().await.corpse_projection_pinned(target));
    Ok(())
}

async fn capture(owner: &ComposedFreshAdmission<'_, '_, '_>, actor: ExactActorRef,
    session: GameSessionId, target: ExactActorRef, creature: &str, command: u64) -> TestResult {
    owner.refresh_quest_session(session, false).await;
    assert!(owner.refresh_source_party_presence(actor, session).await);
    let native = owner.active_generation.and_then(|generation| generation.native_gameplay())
        .ok_or("native rewards")?;
    let queue = owner.states.attack.lock().await.kills().clone();
    let observed = owner.owner_now();
    let now = observed.get() / 1000;
    {
        let mut runtime = owner.runtime.lock().await;
        let lease = runtime.bound_attacker_lease(actor, session).map_err(debug)?;
        let health = runtime.companion_snapshot(target).map_err(debug)?.health;
        runtime.borrow_exact_actor_commit_at(now).commit_damage_for_bound_attacker(target, actor,
            CommandRef::new(session, CommandId::new(command).map_err(debug)?), 0,
            crate::foundation::OwnerDamageCommand {
                target: creature.as_bytes(), occurrence: session.as_bytes(),
                binding: b"grizzly-native-creature-kill", damage: health,
            }).map_err(debug)?;
        for _ in 0..2 {
            crate::gameplay_transport::kill_reward::record_creature_kill(&mut runtime,
                KillSink { prey: None, activity: &[], queue: &queue,
                    rewards: native.reward_table(), charms: None }, target,
                crate::combat::CapturedRewardPrincipal {
                    character: lease.character_id(), lease_generation: lease.generation(), session, actor,
                }, now, Some(observed));
        }
    }
    assert!(queue.has_session(session), "the real native death must be queued");
    assert_eq!(native.reward_table().row(creature).err(),
        Some(crate::content::creature_reward::NoSettlementReason::CorpseItemInadmissible));
    let lease = owner.runtime.lock().await.bound_attacker_lease(actor, session).map_err(debug)?;
    let crate::gameplay_transport::kill_reward::TakeOutcome::Taken(taken) = queue.take(session, lease.generation())
        else { return Err("native fallback entry".into()); };
    assert!(taken.entry().ok_or("captured entry")?.row.is_none(), "ordinary rewards stay inadmissible");
    assert_eq!(queue.counts(), (0, 1, 0), "quest-only work reserves zero loot MINTs");
    drop(taken); // Cancellation returns the same frozen entry to its existing queue.
    assert!(owner.runtime.lock().await.corpse_projection_pinned(target));
    Ok(())
}

pub(super) async fn ordinary(owner: &ComposedFreshAdmission<'_, '_, '_>, h: &Harness, deps: &Dependencies,
    actor: ExactActorRef, session: GameSessionId) -> TestResult {
    // Earlier portal/kill receipts already pin this Character's progression chain.
    // Establish task eligibility through the real XP owner before measuring the
    // quest-only deaths, which must themselves award no XP.
    task_level(owner, session, 241).await?;

    let before = ordinary_values(h).await?;
    let key = "oteryn:creature.crocodile";
    let first = spawn(owner, actor, key).await?;
    capture(owner, actor, session, first, key, 501).await?;
    task(owner, session, 600, Action::StartTask(1)).await?;
    owner.refresh_quest_session(session, false).await;
    assert_eq!(owner.drain_kill_rewards(session).await, DrainOutcome::Drained);
    assert_eq!(value(owner, quest::KILL_COUNTER_BASE + 1).await?, 0,
        "a task started after the actual death cannot receive its credit");

    let second = spawn(owner, actor, key).await?;
    capture(owner, actor, session, second, key, 502).await?;
    task(owner, session, 601, Action::CancelTask(1)).await?;
    task(owner, session, 602, Action::StartTask(1)).await?;
    owner.refresh_quest_session(session, false).await;
    assert_eq!(owner.drain_kill_rewards(session).await, DrainOutcome::Drained);
    assert_eq!(value(owner, quest::KILL_COUNTER_BASE + 1).await?, 0,
        "a delayed death belongs only to the original task run");

    let third = spawn(owner, actor, key).await?;
    capture(owner, actor, session, third, key, 503).await?;
    assert_eq!(owner.drain_kill_rewards(session).await, DrainOutcome::Drained);
    assert_eq!(value(owner, quest::KILL_COUNTER_BASE + 1).await?, 1);
    assert_eq!(owner.drain_kill_rewards(session).await, DrainOutcome::Drained);
    assert_eq!(value(owner, quest::KILL_COUNTER_BASE + 1).await?, 1);
    let transition = Action::KillCreature { task_id: 1, creature: key.into(), observed_revision: 1 }.transition_key();
    assert_eq!(receipts(h, &transition).await?, 1);
    let refused: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_grizzly_refusals WHERE character_id=encode($1,'hex')::uuid AND transition_key=$2")
        .bind(id(41).as_slice()).bind(&transition).fetch_one(&h.pool).await?;
    assert_eq!(refused, 1);
    assert_eq!(ordinary_values(h).await?, before);
    assert!(!owner.runtime.lock().await.corpse_projection_pinned(third));
    late_party(owner, h, deps, actor, session).await?;
    Ok(())
}

async fn spawn(owner: &ComposedFreshAdmission<'_, '_, '_>, actor: ExactActorRef,
    creature: &str) -> TestResult<ExactActorRef> {
    let mut runtime = owner.runtime.lock().await;
    let position = runtime.read_actor_position(actor).map_err(debug)?.position();
    let policy = runtime.companion_policy(creature).map_err(debug)?;
    let target = runtime.admit_encounter_actor(creature, policy.maximum_health,
        MovementLocalPosition { x: position.x + 1, ..position }).map_err(debug)?;
    runtime.install_creature_policy(target, creature).map_err(debug)?;
    Ok(target)
}

struct PresentPremium;
impl crate::durability::quest_state::grizzly::GrizzlyPremiumGate for PresentPremium {
    async fn task_permitted(&self, tx: &mut sqlx::Transaction<'_, sqlx::Postgres>, account: [u8; 16])
        -> Result<bool, crate::durability::DurabilityError> {
        assert!([id(40), id(100)].contains(&account));
        sqlx::query("SELECT 1").execute(&mut **tx).await?;
        Ok(true)
    }
}

async fn task(owner: &ComposedFreshAdmission<'_, '_, '_>, session: GameSessionId,
    n: u64, action: Action) -> TestResult {
    let fence = owner.current_quest_fence(session).await.map_err(|()| "fence")?.ok_or("fence")?;
    let transition = action.transition_key();
    let outcome = owner.revision_sequencer.acquire(fence.character_id).await.commit_grizzly_transition(
        owner.root, owner.character, owner.holder, fence,
        QuestCause::Command(CommandRef::new(session, CommandId::new(n).map_err(debug)?)), action,
        Arc::clone(owner.quest_catalogue.as_ref().ok_or("catalogue")?), &PresentPremium).await
        .map_err(|error| format!("Grizzly combat task {n} {transition}: {error:?}"))?;
    assert!(matches!(outcome, QuestTransitionOutcome::Committed(_)), "{outcome:?}");
    Ok(())
}

async fn task_level(owner: &ComposedFreshAdmission<'_, '_, '_>, session: GameSessionId,
    occurrence: u8) -> TestResult {
    use crate::durability::character_progression::{
        ExperienceAwardRequest, ExperienceCommitOutcome, ExperienceRewardOccurrence,
    };
    let binding = owner.progression_binding(session).ok_or("task-level native policy")?;
    let target = binding.policy.thresholds.iter().find(|row| row.level == 8)
        .ok_or("task-level native threshold")?.minimum_experience.get();
    let fence = owner.current_quest_fence(session).await
        .map_err(|()| "task-level fence")?.ok_or("task-level fence")?;
    let before = owner.root.read_character_progression(owner.character, fence.character_id)
        .await.map_err(debug)?.ok_or("task-level progression")?;
    let amount = target.checked_sub(before.total_experience.get()).ok_or("task-level XP delta")?;
    if amount > 0 {
        let outcome = owner.revision_sequencer.acquire(fence.character_id).await.commit_experience(
            owner.root, owner.character, owner.holder, fence, ExperienceAwardRequest {
                occurrence: ExperienceRewardOccurrence::from_bytes(id(occurrence)).map_err(debug)?,
                amount: oteryn_simulation_determinism::ExactI64::new(amount),
                context: binding.context.clone(), policy_revision: binding.policy_revision.clone(),
                reward_revision: binding.reward_revision.clone(), policy: binding.policy.clone(),
            }, None).await.map_err(|error| format!("Grizzly task-level XP {occurrence}: {error:?}"))?;
        assert!(matches!(outcome, ExperienceCommitOutcome::Committed(_)), "{outcome:?}");
    }
    let after = owner.root.read_character_progression(owner.character, fence.character_id)
        .await.map_err(debug)?.ok_or("task-level updated progression")?;
    assert_eq!(after.level, 8, "the native XP owner establishes task eligibility");
    Ok(())
}

async fn receipts(h: &Harness, transition: &str) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts WHERE character_id=encode($1,'hex')::uuid AND transition_key=$2")
        .bind(id(41).as_slice()).bind(transition).fetch_one(&h.pool).await?)
}

/// Item count and XP receipts cover every ordinary reward effect. Initial equipment
/// and progression are predecessor facts; a row-less death must not change any.
async fn ordinary_values(h: &Harness) -> TestResult<(i64, i64, i64)> {
    Ok(sqlx::query_as("SELECT (SELECT coalesce(sum(total_experience),0)::bigint FROM game_character_progression_state),(SELECT count(*) FROM game_character_xp_receipts),(SELECT count(*) FROM game_item_instances)")
        .fetch_one(&h.pool).await?)
}

async fn late_party(owner: &ComposedFreshAdmission<'_, '_, '_>, h: &Harness, deps: &Dependencies,
    actor: ExactActorRef, session: GameSessionId) -> TestResult {
    let at = owner.runtime.lock().await.read_actor_position(actor).map_err(debug)?.position();
    let (other, second) = super::vacancy::second_player(owner, deps, h,
        MovementLocalPosition { x: at.x - 1, ..at }).await?;
    task_level(owner, second.game_session_id, 242).await?;
    task(owner, second.game_session_id, 700, Action::Join).await?;
    task(owner, second.game_session_id, 701, Action::StartTask(1)).await?;
    owner.refresh_quest_session(second.game_session_id, false).await;
    let before = ordinary_values(h).await?;
    let target = spawn(owner, actor, "oteryn:creature.crocodile").await?;
    capture(owner, actor, session, target, "oteryn:creature.crocodile", 504).await?;
    // The death captured a solo principal. Native invite/accept/enable changes the
    // party only afterward; no delayed settle may discover this new recipient.
    join_party(owner, h, actor, session, other, second.game_session_id).await?;
    assert_eq!(owner.drain_kill_rewards(session).await, DrainOutcome::Drained);
    assert_eq!(value(owner, quest::KILL_COUNTER_BASE + 1).await?, 1,
        "Q8 current membership proof refuses a changed original source");
    let copy = h.root.read_character_quest_state(owner.character,
        crate::domain::CharacterId::from_bytes(id(101)).map_err(debug)?).await.map_err(debug)?;
    assert_eq!(quest::storage_value(copy.tracks(), quest::KILL_COUNTER_BASE + 1), 0,
        "late party join cannot receive an earlier native death");
    assert_eq!(owner.drain_kill_rewards(session).await, DrainOutcome::Drained);
    assert_eq!(ordinary_values(h).await?, before);
    assert!(!owner.runtime.lock().await.corpse_projection_pinned(target));
    Ok(())
}

async fn join_party(owner: &ComposedFreshAdmission<'_, '_, '_>, h: &Harness,
    actor: ExactActorRef, session: GameSessionId, other: ExactActorRef, other_session: GameSessionId) -> TestResult {
    use crate::foundation::PartyAction;
    let permit = owner.spell_lane_permit().await.ok_or("party spell lane")?;
    let mut party = None;
    for (member, command) in [(session, 800), (other_session, 800), (session, 801)] {
        let fence = owner.current_quest_fence(member).await.map_err(|()| "party fence")?.ok_or("party fence")?;
        let runtime = owner.runtime.lock().await;
        let mut tx = h.pool.begin().await?;
        let authority = crate::durability::spell_item_transaction::assert_spell_item_authority_in_transaction(
            &mut tx, &permit, owner.root, owner.character, owner.holder, &fence.item_fence(),
            CommandRef::new(member, CommandId::new(command).map_err(debug)?), [19; 32]).await.map_err(debug)?;
        let action = if member == other_session { PartyAction::Accept { party: party.ok_or("invited party")? } }
            else if command == 800 { PartyAction::Invite { target: id(101) } }
            else { PartyAction::SetSharedExperience { enabled: true } };
        let visible = if matches!(action, PartyAction::Invite { .. }) {
            crate::durability::admission_journal::party_target_binding::prove_visible_party_target_in_transaction(
                &mut tx, owner.root, &authority, &runtime, actor, other, other_session).await.map_err(debug)?
        } else { None };
        let receipt = crate::durability::world_party::apply_world_party_command_in_transaction(
            &mut tx, &authority, action, visible.as_ref()).await.map_err(debug)?;
        assert_eq!(receipt.disposition, "APPLIED");
        party = receipt.party_id;
        tx.commit().await?;
    }
    Ok(())
}
