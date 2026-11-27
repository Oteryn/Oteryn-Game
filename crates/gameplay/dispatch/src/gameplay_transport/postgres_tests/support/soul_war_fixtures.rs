use super::fixture::{debug, id};

use super::TestResult;

use crate::durability::character_progression::CurrentCharacterGameplayFence;

use crate::durability::quest_state::soul_war::{
    SoulWarCommand, apply_soul_war_kill_in_transaction, soul_war_experience_in_transaction,
};

use crate::foundation::{CommandId, CommandRef};

use crate::gameplay_transport::ComposedFreshAdmission;

use oteryn_gameplay_quests::quest::soul_war::{
    SoulWarBoss, TAINT_DURATION_SECONDS,
};

pub(super) async fn current(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    session: crate::foundation::GameSessionId,
) -> TestResult<CurrentCharacterGameplayFence> {
    use crate::durability::DurabilityError::{CommitOutcomeUnknown, RootUnavailable};
    use crate::gameplay_transport::character_gameplay_fence::current_once;

    let read = match current_once(owner, session, false).await {
        Err(CommitOutcomeUnknown | RootUnavailable) => {
            owner.root.await_ready().await?;
            current_once(owner, session, false).await
        }
        result => result,
    };
    read.map_err(|error| format!("Soul War current fence: {error:?}"))?
        .ok_or_else(|| "Soul War current fence".into())
}

pub(super) async fn command(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    session: crate::foundation::GameSessionId,
    number: u64,
    action: SoulWarCommand,
) -> TestResult<crate::durability::quest_state::soul_war::SoulWarCommandOutcome> {
    let fence = current(owner, session).await?;
    let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
    let result = owner
        .root
        .commit_soul_war_command(
            owner.character,
            owner.holder,
            fence,
            CommandRef::new(session, CommandId::new(number).map_err(debug)?),
            action,
        )
        .await;
    slot.observe_endgame_progression();
    result.map_err(debug)
}

/// A test-side projection of a native captured death. The physical capture owner
/// supplies these values; no client command carries or selects them.
#[derive(Clone)]
pub(super) struct CapturedEndgameDeath<K = [u8; 52]> {
    pub death_key: K,
    pub occurrence: [u8; 16],
    pub creature: String,
    pub raw_xp: i64,
    pub death_at_ms: u64,
    pub death_at_us: u64,
    pub damaged: bool,
}

/// Existing native XP/death seam for source identities captured by another mounted
/// fixture. This does not bypass or make a claim about physical corpse admission.
pub(super) async fn settle_captured_endgame_death<K: AsRef<[u8]>>(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    pool: &sqlx::PgPool,
    session: crate::foundation::GameSessionId,
    source: &CapturedEndgameDeath<K>,
) -> TestResult {
    use crate::durability::character_progression::{
        ExperienceAwardRequest, ExperienceRewardOccurrence,
    };
    use crate::durability::party_kill_xp::{PartyKillXpMember, PartyKillXpRequest};
    let initial = current(owner, session).await?;
    let lane = crate::durability::spell_owner_commit::SpellLane::for_scope(initial.runtime_scope);
    let permit = lane.acquire().await.map_err(|_| "spell lane")?;
    let mut slot = owner.revision_sequencer.acquire(initial.character_id).await;
    slot.observe_endgame_progression();
    let fence = current(owner, session).await?;
    if fence.character_id != initial.character_id || fence.runtime_scope != initial.runtime_scope {
        return Err("captured death changed Character/scope before settlement".into());
    }
    let mut tx = pool.begin().await?;
    sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
        .execute(&mut *tx)
        .await?;
    let scope = crate::durability::spell_item_transaction::assert_spell_item_scope_in_transaction(
        &mut tx,
        &permit,
        owner.root,
        owner.character,
        owner.holder,
        fence.runtime_scope,
        fence.scope_ownership_generation.get(),
    )
    .await
    .map_err(debug)?;
    let replay = crate::durability::party_kill_xp::reconcile_in_transaction(
        &mut tx,
        &scope,
        source.death_key.as_ref(),
        fence.character_id,
        &source.creature,
        source.raw_xp,
        (source.death_at_ms, source.death_at_us),
    )
    .await
    .map_err(debug)?;
    let awards = if let Some(awards) = replay {
        awards
    } else {
        let before = soul_war_experience_in_transaction(
            &mut tx,
            fence.character_id,
            &source.creature,
            source.raw_xp,
        )
        .await
        .map_err(debug)?;
        let binding = owner
            .progression_binding(session)
            .ok_or("progression policy")?;
        let awards = crate::durability::party_kill_xp::commit_in_transaction(
            &mut tx,
            &scope,
            owner.holder,
            PartyKillXpRequest {
                death_key: source.death_key.as_ref().to_vec(),
                principal: fence.character_id,
                creature: source.creature.clone(),
                base_xp: source.raw_xp,
                death_at_ms: source.death_at_ms,
                death_at_us: source.death_at_us,
                died_at_unix_ms: 0,
                shared: false,
                party_id: None,
                party_revision: None,
                members: vec![PartyKillXpMember {
                    task_board_eligible: true,
                    fence,
                    request: ExperienceAwardRequest {
                        occurrence: ExperienceRewardOccurrence::from_bytes(source.occurrence)
                            .map_err(debug)?,
                        amount: oteryn_simulation_determinism::ExactI64::new(before),
                        context: binding.context.clone(),
                        policy_revision: binding.policy_revision.clone(),
                        reward_revision: binding.reward_revision.clone(),
                        policy: binding.policy.clone(),
                    },
                    premium: false,
                    forge_dust: None,
                    live_stamina: None,
                    shared: false,
                    party_id: None,
                    party_revision: None,
                }],
            },
        )
        .await
        .map_err(debug)?;
        // Match the production composition, including zero-XP members. The
        // repeated same-transaction calls must observe their accepted receipts.
        for _ in 0..2 {
            let participants = [(fence, source.damaged)];
            apply_soul_war_kill_in_transaction(
                &mut tx,
                owner.holder,
                source.death_key.as_ref(),
                &source.creature,
                &participants,
            )
            .await
            .map_err(debug)?;
            crate::durability::quest_state::rotten_blood::apply_rotten_blood_kill_in_transaction(
                &mut tx,
                owner.holder,
                source.death_key.as_ref(),
                &source.creature,
                &participants,
            )
            .await
            .map_err(debug)?;
            crate::durability::character_hazard::apply_hazard_kill_in_transaction(
                &mut tx,
                owner.holder,
                source.death_key.as_ref(),
                &source.creature,
                &participants,
            )
            .await
            .map_err(debug)?;
        }
        awards
    };
    assert_eq!(awards.len(), 1);
    tx.commit().await?;
    slot.observe_endgame_progression();
    Ok(())
}

/// Source-side fixture enters the real atomic XP owner and native Quest consumer;
/// raw boss/participant values are never exposed as client command payloads.
pub(super) async fn kill(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    pool: &sqlx::PgPool,
    session: crate::foundation::GameSessionId,
    number: u8,
    creature: &str,
    damaged: bool,
) -> TestResult<[u8; 52]> {
    let fence = current(owner, session).await?;
    let crate::foundation::RuntimeScopeRefV1::Channel {
        world_id,
        channel_id,
    } = fence.runtime_scope
    else {
        return Err("channel fixture".into());
    };
    let mut death_key = [0_u8; 52];
    death_key[..16].copy_from_slice(world_id.as_bytes());
    death_key[16..32].copy_from_slice(channel_id.as_bytes());
    death_key[32..40].copy_from_slice(&fence.scope_ownership_generation.get().to_be_bytes());
    death_key[40..44].copy_from_slice(&u32::from(number).to_be_bytes());
    death_key[44..52].copy_from_slice(&1_u64.to_be_bytes());
    let now = owner.owner_now().get();
    settle_captured_endgame_death(
        owner,
        pool,
        session,
        &CapturedEndgameDeath {
            death_key,
            occurrence: id(number),
            creature: creature.into(),
            raw_xp: 10_000,
            death_at_ms: now / 1000,
            death_at_us: now,
            damaged,
        },
    )
    .await?;
    Ok(death_key)
}

/// Uses the existing two-player fresh-admission fixture. Every completion flag
/// is earned through the native XP owner; only the historical first-taint clock
/// is seeded administratively to exercise expiry during an admitted encounter.
pub(super) async fn terminal_expiry_case(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    pool: &sqlx::PgPool,
    players: &[(
        crate::foundation::ExactActorRef,
        crate::foundation::GameSessionId,
    )],
) -> TestResult {
    use crate::durability::endgame_terminal::{EndgameFailureRequest, FinalEncounter};
    assert_eq!(players.len(), 2);
    let bosses = ["malice", "hatred", "spite", "cruelty", "greed"];
    for (member, (_, session)) in players.iter().copied().enumerate() {
        command(owner, session, 901, SoulWarCommand::Start).await?;
        for (index, boss) in bosses.iter().enumerate() {
            kill(
                owner,
                pool,
                session,
                230 + (member * 5 + index) as u8,
                &format!("oteryn:creature.goshnar_s_{boss}"),
                true,
            )
            .await?;
        }
        assert!(read_state(owner, session).await?.chain_ready());
    }
    let first = current(owner, players[0].1).await?;
    let second = current(owner, players[1].1).await?;
    let second_clock = read_state(owner, players[1].1).await?.first_taint_at;
    expire_clock(pool, first.character_id).await?;
    let mut members = vec![first, second];
    members.sort_by_key(|member| member.character_id);
    let roster = members
        .iter()
        .flat_map(|member| member.character_id.as_bytes().iter().copied())
        .collect::<Vec<_>>();
    let mixed = EndgameFailureRequest {
        attempt: [75; 32],
        encounter: FinalEncounter::Megalomania,
        participants: members,
    };
    let receipt = terminal_failure(owner, mixed.clone()).await?;
    let pair = receipt.removed_flags.ok_or("mixed-expiry retained pair")?;
    assert!(!receipt.replayed);
    assert_eq!(pair.count_ones(), 2);
    let expired = read_state(owner, players[0].1).await?;
    let reduced = read_state(owner, players[1].1).await?;
    assert_eq!(expired.defeated_bosses, 0);
    assert_eq!(expired.first_taint_at, None);
    assert_eq!(reduced.defeated_bosses, 31 & !pair);
    assert_eq!(reduced.defeated_bosses.count_ones(), 3);
    assert_eq!(reduced.first_taint_at, second_clock);
    assert_eq!(
        current(owner, players[0].1)
            .await?
            .expected_character_revision,
        first.expected_character_revision,
        "expiry no-op must not advance Character state"
    );
    let decision = terminal_decision(pool, &mixed.attempt).await?;
    assert_eq!(
        decision,
        (
            roster.clone(),
            second.character_id.as_bytes().to_vec(),
            first.character_id.as_bytes().to_vec(),
            Some(i16::from(pair)),
            1
        )
    );
    let replay = terminal_failure(owner, mixed.clone()).await?;
    assert!(replay.replayed);
    assert_eq!(replay.removed_flags, Some(pair));
    assert_eq!(read_state(owner, players[1].1).await?, reduced);
    assert_eq!(terminal_decision(pool, &mixed.attempt).await?, decision);

    // Earn a complete new eligible roster before the second admitted attempt.
    for (member, (_, session)) in players.iter().copied().enumerate() {
        for (index, boss) in bosses.iter().enumerate() {
            if member == 0 || pair & (1 << index) != 0 {
                kill(
                    owner,
                    pool,
                    session,
                    240 + (member * 5 + index) as u8,
                    &format!("oteryn:creature.goshnar_s_{boss}"),
                    true,
                )
                .await?;
            }
        }
        assert!(read_state(owner, session).await?.chain_ready());
    }
    expire_clock(pool, first.character_id).await?;
    expire_clock(pool, second.character_id).await?;
    let mut members = vec![
        current(owner, players[0].1).await?,
        current(owner, players[1].1).await?,
    ];
    members.sort_by_key(|member| member.character_id);
    let all_expired = EndgameFailureRequest {
        attempt: [76; 32],
        encounter: FinalEncounter::Megalomania,
        participants: members,
    };
    let receipt = terminal_failure(owner, all_expired.clone()).await?;
    assert!(!receipt.replayed);
    assert_eq!(receipt.removed_flags, None);
    let decision = terminal_decision(pool, &all_expired.attempt).await?;
    assert_eq!(decision, (roster.clone(), vec![], roster, None, 0));
    for (_, session) in players {
        assert_eq!(read_state(owner, *session).await?.defeated_bosses, 0);
    }
    let replay = terminal_failure(owner, all_expired.clone()).await?;
    assert!(replay.replayed);
    assert_eq!(replay.removed_flags, None);
    assert_eq!(
        terminal_decision(pool, &all_expired.attempt).await?,
        decision
    );

    // Old attempts remain historical even when a participant starts a new cycle.
    kill(
        owner,
        pool,
        players[0].1,
        250,
        "oteryn:creature.goshnar_s_malice",
        true,
    )
    .await?;
    let new_cycle = read_state(owner, players[0].1).await?;
    assert_eq!(new_cycle.defeated_bosses, SoulWarBoss::Malice.bit());
    let revision = current(owner, players[0].1)
        .await?
        .expected_character_revision;
    let replay = terminal_failure(owner, all_expired.clone()).await?;
    assert!(replay.replayed);
    assert_eq!(replay.removed_flags, None);
    let replay = terminal_failure(owner, mixed).await?;
    assert!(replay.replayed);
    assert_eq!(replay.removed_flags, Some(pair));
    assert_eq!(read_state(owner, players[0].1).await?, new_cycle);
    assert_eq!(
        current(owner, players[0].1)
            .await?
            .expected_character_revision,
        revision
    );
    assert_eq!(
        terminal_decision(pool, &all_expired.attempt).await?,
        decision
    );
    // A later eligible attempt starts with the full chain, then its clock expires
    // before the final death. XP-sharing bystanders still receive no completion.
    for (index, boss) in bosses.iter().enumerate() {
        kill(
            owner,
            pool,
            players[1].1,
            214 + index as u8,
            &format!("oteryn:creature.goshnar_s_{boss}"),
            true,
        )
        .await?;
    }
    assert!(read_state(owner, players[1].1).await?.chain_ready());
    expire_clock(pool, second.character_id).await?;
    assert!(!read_state(owner, players[1].1).await?.chain_ready());
    assert!(!read_state(owner, players[1].1).await?.completed());
    kill(
        owner,
        pool,
        players[1].1,
        219,
        "oteryn:creature.goshnar_s_megalomania_blue",
        false,
    )
    .await?;
    assert!(!read_state(owner, players[1].1).await?.completed());
    assert!(
        owner
            .root
            .pending_soul_war_reward(owner.character, second.character_id)
            .await
            .map_err(debug)?
            .is_none()
    );
    let victory_death = kill(
        owner,
        pool,
        players[1].1,
        220,
        "oteryn:creature.goshnar_s_megalomania_green",
        true,
    )
    .await?;
    let victory = read_state(owner, players[1].1).await?;
    assert!(victory.completed());
    assert_eq!(victory.defeated_bosses, 0);
    assert_eq!(victory.first_taint_at, None);
    let reward = owner
        .root
        .pending_soul_war_reward(owner.character, second.character_id)
        .await
        .map_err(debug)?
        .ok_or("verified final after expiry earns its first item")?;
    assert!((34082..=34099).contains(&reward.item_source_id));
    let fence = current(owner, players[1].1).await?;
    let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
    let mut tx = pool.begin().await?;
    sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
        .execute(&mut *tx)
        .await?;
    apply_soul_war_kill_in_transaction(
        &mut tx,
        owner.holder,
        &victory_death,
        "oteryn:creature.goshnar_s_megalomania_green",
        &[(fence, true)],
    )
    .await
    .map_err(debug)?;
    tx.commit().await?;
    slot.observe_endgame_progression();
    drop(slot);
    assert_eq!(
        current(owner, players[1].1)
            .await?
            .expected_character_revision,
        fence.expected_character_revision
    );
    let entitlements: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_soul_war_reward_entitlements WHERE character_id=encode($1,'hex')::uuid")
        .bind(second.character_id.as_bytes().as_slice()).fetch_one(pool).await?;
    assert_eq!(
        entitlements, 1,
        "replaying the successful native death cannot earn another reward"
    );
    assert!(terminal_failure(owner, all_expired).await?.replayed);
    assert_eq!(read_state(owner, players[1].1).await?, victory);
    assert_eq!(
        owner
            .root
            .pending_soul_war_reward(owner.character, second.character_id)
            .await
            .map_err(debug)?,
        Some(reward)
    );
    eprintln!(
        "SOUL_WAR_VICTORY POSTGRES_CASE_COMPLETED expired_victory_reward_once_bystander_replay"
    );
    eprintln!("SOUL_WAR_TERMINAL POSTGRES_CASE_COMPLETED mixed_all_expired_replay");
    Ok(())
}

async fn read_state(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    session: crate::foundation::GameSessionId,
) -> TestResult<oteryn_gameplay_quests::quest::soul_war::SoulWarState> {
    owner
        .root
        .read_soul_war_state(
            owner.character,
            owner.holder,
            current(owner, session).await?,
        )
        .await
        .map_err(debug)
}

async fn expire_clock(pool: &sqlx::PgPool, character: crate::domain::CharacterId) -> TestResult {
    let mut tx = pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    let updated = sqlx::query("UPDATE game_character_soul_war SET first_taint_at=floor(extract(epoch FROM transaction_timestamp()))::bigint-$2 WHERE character_id=encode($1,'hex')::uuid")
        .bind(character.as_bytes().as_slice()).bind(TAINT_DURATION_SECONDS).execute(&mut *tx).await?;
    assert_eq!(updated.rows_affected(), 1);
    tx.commit().await?;
    Ok(())
}

async fn terminal_failure(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    request: crate::durability::endgame_terminal::EndgameFailureRequest,
) -> TestResult<crate::durability::endgame_terminal::EndgameFailureReceipt> {
    use crate::durability::endgame_terminal::{TestOwnerState, TestSharedTerminalOwner};
    let terminal =
        TestSharedTerminalOwner::new(request.clone(), TestOwnerState::TerminalUnsuccessful);
    let mut slots = Vec::new();
    for member in &request.participants {
        slots.push(owner.revision_sequencer.acquire(member.character_id).await);
    }
    let result = owner
        .root
        .apply_endgame_failure(owner.character, owner.holder, &terminal, request)
        .await;
    for mut slot in slots {
        slot.observe_endgame_progression();
    }
    result.map_err(debug)
}

async fn terminal_decision(
    pool: &sqlx::PgPool,
    attempt: &[u8; 32],
) -> TestResult<(Vec<u8>, Vec<u8>, Vec<u8>, Option<i16>, i64)> {
    Ok(sqlx::query_as("SELECT participants,affected_participants,expired_participants,removed_flags,(SELECT count(*) FROM game_character_soul_war_receipts r WHERE r.source_attempt=o.attempt) FROM game_endgame_terminal_outcomes o WHERE attempt=$1")
        .bind(attempt.as_slice()).fetch_one(pool).await?)
}
