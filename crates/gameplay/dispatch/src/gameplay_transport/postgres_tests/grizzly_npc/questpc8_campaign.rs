//! Native Grizzly progression for the three selected task/boss pairs.
//! The chosen campaign adds its own once-only 10,000 XP and 5,000 gold completion.
//! No task counters, boss victories, points or quest receipts are seeded.
use super::*;
use crate::foundation::{CommandId, CommandRef, ExactActorRef, MovementLocalPosition};
use crate::gameplay_transport::kill_reward::{DrainOutcome, KillSink};
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_protocol_oteryn::world_spatial::{StepDirection, encode_step_intent};

#[test]
fn questpc8_hunting_campaign_native_deaths_portals_reports_and_rewards() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::super::kill_reward_e2e::scenario_thread::run("questpc8-hunting-campaign", || {
        scenario(5)
    })?;
    println!(
        "POSTGRES_CASE_COMPLETED questpc8_hunting_campaign_native_deaths_portals_reports_and_rewards duration={:?}",
        started.elapsed()
    );
    Ok(())
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    npc: EntityRef,
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    journal: &QuestLogContent,
) -> TestResult {
    {
        let native = owner
            .active_generation
            .and_then(|generation| generation.native_gameplay())
            .ok_or("native campaign content")?;
        native
            .install_companion_policies(&mut *owner.runtime.lock().await)
            .map_err(debug)?;
    }
    let actor = admitted.runtime_actor.ok_or("hunter")?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "campaign fence")?
        .ok_or("campaign fence")?;
    assert!(owner.track_restored(
        admitted.game_session_id,
        actor,
        &crate::gameplay_transport::last_position::AdmissionPosition::FirstLogin,
        fence
    ));
    assert!(
        owner
            .final_last_position(admitted.game_session_id, actor)
            .await
            .is_some(),
        "boss visit cleanup needs the normal last-position owner"
    );
    let origin = owner
        .runtime
        .lock()
        .await
        .read_actor_position(admitted.runtime_actor.ok_or("hunter")?)
        .map_err(debug)?
        .position();
    let mut command = 2;
    let initial_experience = experience(h).await?;
    say(owner, admitted, npc, &mut command, &["hi", "campaign"]).await?;
    assert_eq!(experience(h).await?, initial_experience);
    assert_eq!(campaign_receipts(h).await?, 0);

    // The chosen next-rank milestone cannot bypass the source ten-point gate.
    // Complete all three allowed low-level repetitions to earn twelve points.
    for repetition in 0..3 {
        for (task, name, boss) in [(1, "crocodiles", 1), (4, "carniphilas", 3)] {
            say(owner, admitted, npc, &mut command, &["hi", name, "yes"]).await?;
            assert_eq!(
                value(owner, authority, quest::QUEST_STORAGE_BASE + task).await?,
                1
            );
            let source = quest::task(u8::try_from(task)?).ok_or("native task")?;
            assert_eq!(source.required, if task == 1 { 300 } else { 150 });
            hunt(
                owner,
                admitted,
                source.creatures[0].key.as_str(),
                u32::try_from(source.required)?,
                command,
            )
            .await?;
            assert_eq!(
                value(owner, authority, quest::KILL_COUNTER_BASE + task).await?,
                source.required
            );
            journal_count(owner, admitted, journal, &source.name, source.required).await?;
            let before_xp = experience(h).await?;
            say(owner, admitted, npc, &mut command, &["hi", "tasks"]).await?;
            assert_eq!(
                experience(h).await? - before_xp,
                source.experience.ok_or("task XP")?
            );
            assert_eq!(
                value(owner, authority, quest::REPEAT_STORAGE_BASE + task).await?,
                i64::from(repetition + 1)
            );
            eprintln!(
                "HUNTING_CAMPAIGN task={task} repetition={} kills={}",
                repetition + 1,
                source.required
            );
            if repetition == 0 {
                defeat_boss(owner, admitted, h, authority, boss, &mut command).await?;
                teleport(owner, admitted, origin).await?;
                say(owner, admitted, npc, &mut command, &["hi", "report"]).await?;
                assert_eq!(
                    value(owner, authority, quest::boss(boss).ok_or("boss")?.storage).await?,
                    3
                );
            }
        }
    }
    assert_eq!(value(owner, authority, quest::POINTS_STORAGE).await?, 12);
    say(owner, admitted, npc, &mut command, &["hi", "promotion"]).await?;
    assert_eq!(value(owner, authority, quest::PROMOTION_STORAGE).await?, 0);
    // Rank alone cannot replace the missing immutable victory/report evidence.
    // An incomplete ordered campaign answers with its objectives, without a grant.
    let early_xp = experience(h).await?;
    say(owner, admitted, npc, &mut command, &["hi", "campaign"]).await?;
    assert_eq!(experience(h).await?, early_xp);
    assert_eq!(campaign_receipts(h).await?, 0);
    let early_phase: i64 = sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2")
        .bind(id(41).as_slice()).bind(quest::campaign::PHASE).fetch_one(&h.pool).await?;
    assert_eq!(
        early_phase, 7,
        "a premature claim cannot advance chosen progress"
    );

    // Level 130 is an external eligibility condition. Establish it with the
    // actual XP owner, never by changing a stored level or bypassing the guard.
    level_130(owner, admitted).await?;
    say(owner, admitted, npc, &mut command, &["hi", "hydras", "yes"]).await?;
    assert_eq!(
        value(owner, authority, quest::QUEST_STORAGE_BASE + 32).await?,
        1
    );
    hunt(owner, admitted, "oteryn:creature.hydra", 650, command).await?;
    assert_eq!(
        value(owner, authority, quest::KILL_COUNTER_BASE + 32).await?,
        650
    );
    journal_count(owner, admitted, journal, "Hydras", 650).await?;
    eprintln!("HUNTING_CAMPAIGN hydras=650 journal_verified=true");
    let before_xp = experience(h).await?;
    say(owner, admitted, npc, &mut command, &["hi", "tasks"]).await?;
    assert_eq!(experience(h).await? - before_xp, 30_000);
    defeat_boss(owner, admitted, h, authority, 19, &mut command).await?;
    teleport(owner, admitted, origin).await?;
    say(owner, admitted, npc, &mut command, &["hi", "report"]).await?;
    assert_eq!(
        value(owner, authority, quest::boss(19).ok_or("Many")?.storage).await?,
        3
    );
    assert_eq!(value(owner, authority, quest::POINTS_STORAGE).await?, 18);
    let before: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    let before_xp = experience(h).await?;
    // An extra report does not earn another achievement or another boss reward.
    say(owner, admitted, npc, &mut command, &["hi", "report"]).await?;
    let after: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(after, before);
    assert_eq!(experience(h).await?, before_xp);
    let achievements: Vec<String> = sqlx::query_scalar(
        "SELECT achievement_key FROM game_account_achievements ORDER BY achievement_key LIMIT 4",
    )
    .fetch_all(&h.pool)
    .await?;
    assert_eq!(
        achievements,
        [
            "oteryn:achievement/blood_red_snapper",
            "oteryn:achievement/one_less",
            "oteryn:achievement/rootless_behaviour"
        ]
    );
    let before_campaign_xp = experience(h).await?;
    eprintln!("QUESTPC8_CAMPAIGN POSTGRES_FINAL_PAYOUT_CHECK");
    command += 1;
    let greeting = command;
    command += 1;
    wire::talk_refused(
        owner,
        admitted,
        npc,
        &[(greeting, "hi"), (command, "campaign")],
    )
    .await?;
    assert_eq!(
        experience(h).await?,
        before_campaign_xp,
        "missing backpack refuses the complete reward transaction"
    );
    let phase: i64 = sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2")
        .bind(id(41).as_slice()).bind(quest::campaign::PHASE).fetch_one(&h.pool).await?;
    assert_eq!(phase, 10, "failed delivery cannot complete the campaign");
    super::super::item_look::seed_main_backpack(h, &[]).await?;
    say(owner, admitted, npc, &mut command, &["hi", "campaign"]).await?;
    assert_eq!(
        experience(h).await? - before_campaign_xp,
        10_000,
        "the complete chosen campaign owes its authored XP exactly once"
    );
    let minted: (String, i64, bool) = sqlx::query_as("SELECT i.definition_production_key,m.quantity,m.created_xact_id=q.created_xact_id FROM game_reward_claim_mint_receipts m JOIN game_item_instances i USING(item_instance_id) JOIN game_character_quest_receipts q ON q.character_id=m.character_id AND q.cause_id=m.game_session_id AND q.cause_ordinal=m.command_id AND q.transition_key=m.claim_production_key WHERE q.character_id=encode($1,'hex')::uuid AND q.transition_key=$2")
        .bind(id(41).as_slice()).bind("oteryn:quest-reward/chosen/hunting-campaign").fetch_one(&h.pool).await?;
    assert_eq!(
        minted,
        ("oteryn:item.tibia.i3035".into(), 50, true),
        "exact 5,000 gold value (50 platinum) shares the completion transaction"
    );
    let completed: bool = sqlx::query_scalar("SELECT completed_character_revision IS NOT NULL FROM game_character_quest_states WHERE character_id=encode($1,'hex')::uuid AND quest_key=$2")
        .bind(id(41).as_slice()).bind(quest::QUEST).fetch_one(&h.pool).await?;
    assert!(completed);
    say(owner, admitted, npc, &mut command, &["hi", "campaign"]).await?;
    assert_eq!(
        experience(h).await? - before_campaign_xp,
        10_000,
        "fresh-command replay grants no XP twice"
    );
    owner
        .states
        .quest_sessions
        .lock()
        .map_err(debug)?
        .remove(&admitted.game_session_id);
    owner.admit_quest_session(admitted).await;
    say(owner, admitted, npc, &mut command, &["hi", "campaign"]).await?;
    assert_eq!(experience(h).await? - before_campaign_xp, 10_000);
    let receipts: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_reward_claim_mint_receipts WHERE claim_production_key=$1",
    )
    .bind("oteryn:quest-reward/chosen/hunting-campaign")
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(
        receipts, 1,
        "fresh-command replay cannot mint currency twice"
    );
    Ok(())
}

async fn experience(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT total_experience FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid")
        .bind(id(41).as_slice()).fetch_one(&h.pool).await?)
}

async fn journal_count(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    journal: &QuestLogContent,
    name: &str,
    count: i64,
) -> TestResult {
    let QuestLogObservation::Changed(source) = owner
        .observe_quest_log(
            admitted.runtime_actor.ok_or("hunter")?,
            admitted.game_session_id,
            None,
        )
        .await
    else {
        return Err("campaign journal unavailable".into());
    };
    let index = journal
        .active()
        .quests()
        .iter()
        .position(|entry| entry.key == quest::QUEST)
        .ok_or("hunting journal")?;
    let line = journal
        .line(source.copy.as_ref(), u32::try_from(index)?)
        .ok_or("listed hunting campaign")?;
    assert!(
        line.missions
            .iter()
            .any(|mission| mission.name == format!("Paw and Fur: {name}")
                && mission.text.contains(&format!("{count}/{count}"))),
        "{name} journal must observe every actual death"
    );
    Ok(())
}

async fn say(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    npc: EntityRef,
    command: &mut u64,
    words: &[&str],
) -> TestResult {
    let mut commands: Vec<_> = words
        .iter()
        .map(|word| {
            *command += 1;
            (*command, *word)
        })
        .collect();
    if let Some(last) = commands.last().copied() {
        commands.push(last);
    }
    wire::talk(owner, admitted, npc, &commands).await
}

pub(in super::super) async fn hunt(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    creature: &str,
    count: u32,
    command: u64,
) -> TestResult {
    let actor = admitted.runtime_actor.ok_or("hunter")?;
    for index in 0..count {
        let target = {
            let mut runtime = owner.runtime.lock().await;
            let position = runtime
                .read_actor_position(actor)
                .map_err(debug)?
                .position();
            let health = runtime
                .companion_policy(creature)
                .map_err(|error| format!("campaign creature policy {creature}: {error:?}"))?
                .maximum_health;
            let target = runtime
                .admit_encounter_actor(
                    creature,
                    health,
                    MovementLocalPosition {
                        x: position.x + 1,
                        ..position
                    },
                )
                .map_err(debug)?;
            runtime
                .install_creature_policy(target, creature)
                .map_err(debug)?;
            target
        };
        lethal(
            owner,
            admitted,
            target,
            creature,
            command,
            u16::try_from(index + 1)?,
        )
        .await?;
        let mut runtime = owner.runtime.lock().await;
        assert!(!runtime.corpse_projection_pinned(target));
        runtime.remove_encounter_actor(target).map_err(debug)?;
    }
    Ok(())
}

pub(in super::super) async fn lethal(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    target: ExactActorRef,
    creature: &str,
    command: u64,
    swing: u16,
) -> TestResult {
    let actor = admitted.runtime_actor.ok_or("hunter")?;
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    assert!(
        owner
            .refresh_source_party_presence(actor, admitted.game_session_id)
            .await
    );
    let native = owner
        .active_generation
        .and_then(|active| active.native_gameplay())
        .ok_or("native")?;
    let queue = owner.states.attack.lock().await.kills().clone();
    let observed = owner.owner_now();
    let now = observed.get() / 1000;
    {
        let mut runtime = owner.runtime.lock().await;
        let lease = runtime
            .bound_attacker_lease(actor, admitted.game_session_id)
            .map_err(debug)?;
        let health = runtime.companion_snapshot(target).map_err(debug)?.health;
        runtime
            .borrow_exact_actor_commit_at(now)
            .commit_swing_damage_for_bound_attacker(
                target,
                actor,
                CommandRef::new(
                    admitted.game_session_id,
                    CommandId::new(command).map_err(debug)?,
                ),
                swing,
                crate::foundation::OwnerDamageCommand {
                    target: creature.as_bytes(),
                    occurrence: admitted.game_session_id.as_bytes(),
                    binding: b"questpc8-hunting-campaign",
                    damage: health,
                },
            )
            .map_err(debug)?;
        // Duplicate lethal notification must retain just one quest credit.
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
                crate::combat::CapturedRewardPrincipal {
                    character: lease.character_id(),
                    lease_generation: lease.generation(),
                    session: admitted.game_session_id,
                    actor,
                },
                now,
                Some(observed),
            );
        }
    }
    assert!(queue.has_session(admitted.game_session_id));
    assert_eq!(
        owner.drain_kill_rewards(admitted.game_session_id).await,
        DrainOutcome::Drained
    );
    assert_eq!(
        owner.drain_kill_rewards(admitted.game_session_id).await,
        DrainOutcome::Drained
    );
    Ok(())
}

async fn defeat_boss(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    id: u8,
    command: &mut u64,
) -> TestResult {
    let boss = quest::boss(id).ok_or("boss")?;
    assert_eq!(value(owner, authority, boss.storage).await?, 1);
    teleport(
        owner,
        admitted,
        MovementLocalPosition {
            x: i32::from(boss.portal.x) - 1,
            y: i32::from(boss.portal.y),
            floor: i16::from(boss.portal.z),
        },
    )
    .await?;
    *command += 1;
    super::super::quest_world_callbacks::wire(
        owner,
        admitted,
        *command,
        1,
        encode_step_intent(StepDirection::East),
    )
    .await?;
    assert_eq!(value(owner, authority, boss.storage).await?, 2);
    let target = {
        let runtime = owner.runtime.lock().await;
        runtime
            .visible_entities()
            .creatures
            .into_iter()
            .find(|entry| {
                runtime
                    .companion_snapshot(entry.actor)
                    .is_ok_and(|snapshot| snapshot.state.policy.definition_key == boss.creature_key)
            })
            .ok_or("spawned native boss")?
            .actor
    };
    lethal(owner, admitted, target, &boss.creature_key, *command, 1).await?;
    assert_eq!(value(owner, authority, boss.kill_storage).await?, 1);
    let visits: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_grizzly_boss_entries WHERE character_id=encode($1,'hex')::uuid").bind(super::id(41).as_slice()).fetch_one(&h.pool).await?;
    assert!(visits > 0);
    Ok(())
}

async fn teleport(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    destination: MovementLocalPosition,
) -> TestResult {
    let _permit = owner.spell_lane_permit().await.ok_or("spell lane")?;
    let map = owner.bundle_map().ok_or("map")?;
    let mut runtime = owner.runtime.lock().await;
    let actor = admitted.runtime_actor.ok_or("hunter")?;
    let before = runtime.read_actor_position(actor).map_err(debug)?;
    let doors = crate::map::floor::BundleDoors(map.facts.doors());
    let proof = crate::map::floor::prepare_quest_callback_teleport(
        &runtime,
        &owner.movement_cells,
        actor,
        admitted.game_session_id,
        before,
        destination,
        false,
        &doors,
    )
    .map_err(debug)?;
    runtime.commit_source_step(proof).map_err(debug)?;
    Ok(())
}

async fn level_130(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
) -> TestResult {
    use crate::durability::character_progression::{
        ExperienceAwardRequest, ExperienceCommitOutcome, ExperienceRewardOccurrence,
    };
    let binding = owner
        .progression_binding(admitted.game_session_id)
        .ok_or("progression")?;
    let target = binding
        .policy
        .thresholds
        .iter()
        .find(|row| row.level == 130)
        .ok_or("level 130")?
        .minimum_experience
        .get();
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let before = owner
        .root
        .read_character_progression(owner.character, fence.character_id)
        .await
        .map_err(debug)?
        .ok_or("progression")?;
    let amount = target
        .checked_sub(before.total_experience.get())
        .filter(|amount| *amount > 0)
        .ok_or("level eligibility XP")?;
    let outcome = owner
        .revision_sequencer
        .acquire(fence.character_id)
        .await
        .commit_experience(
            owner.root,
            owner.character,
            owner.holder,
            fence,
            ExperienceAwardRequest {
                occurrence: ExperienceRewardOccurrence::from_bytes(id(240)).map_err(debug)?,
                amount: oteryn_simulation_determinism::ExactI64::new(amount),
                context: binding.context.clone(),
                policy_revision: binding.policy_revision.clone(),
                reward_revision: binding.reward_revision.clone(),
                policy: binding.policy.clone(),
            },
            None,
        )
        .await
        .map_err(debug)?;
    assert!(
        matches!(outcome, ExperienceCommitOutcome::Committed(_)),
        "{outcome:?}"
    );
    // The helper called the durable XP owner directly. Notify the normal live
    // refresh owner, which reloads level/vocation facts before NPC admission.
    owner.states.level_refresh.mark([fence.character_id]);
    Ok(())
}

async fn campaign_receipts(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT count(*) FROM game_quest_reward_sources WHERE character_id=encode($1,'hex')::uuid AND transition_key='oteryn:quest-reward/chosen/hunting-campaign'").bind(id(41).as_slice()).fetch_one(&h.pool).await?)
}
