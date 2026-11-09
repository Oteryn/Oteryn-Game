//! T201: native combat, existing fenced XP/dust owner, and literal Task Board commands.
use super::*;
use oteryn_gameplay_quests::quest::task_board::full::{self as rules, TaskBoardAction};

async fn elite(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    enabled: bool,
) -> TestResult {
    let first = admitted.continuity.next_command_id;
    let (_, results) = serve_attempt_kind(
        owner,
        admitted,
        first,
        &[full_request(19, u16::from(enabled), "")],
        wire::full::COMMAND_ELITE_BOUNTY,
    )
    .await?;
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].1, CommandStatus::Accepted);
    assert_eq!(
        wire::full::decode_reply(&results[0].2)
            .map_err(debug)?
            .bounty
            .elite,
        enabled
    );
    Ok(())
}

async fn save(harness: &Harness, data: &rules::TaskBoardData) -> TestResult {
    let mut tx = harness.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "UPDATE game_character_task_boards SET body=$2 WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(id(41).as_slice())
    .bind(serde_json::to_value(data)?)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(())
}

pub(super) async fn run_case(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    harness: &Harness,
    admitted: &mut AdmittedSession,
    actor: crate::foundation::ExactActorRef,
    rat: crate::foundation::ExactActorRef,
) -> TestResult {
    let session = admitted.game_session_id;
    elite(owner, admitted, true).await?;
    let initial = full_board(owner, admitted, 7, 0, "").await?;
    assert!(
        initial
            .bounty
            .offers
            .iter()
            .all(|task| task.elite && task.dust > 0)
    );
    full_board(owner, admitted, 11, 0, "").await?;
    let reloaded = fixture_board_data(harness).await?;
    for task in &initial.bounty.offers {
        assert_eq!(reloaded.bounty.rarity.get(&task.race), Some(&1));
    }
    println!("<T201> POSTGRES_CASE_COMPLETED: elite_preference_and_per_character_rarity_reload");

    // Stage a legal rare draw from the real content/rules generator, with the held native rat first.
    let mut creatures = owner.task_board_catalogue().ok_or("catalogue")?.creatures;
    creatures.sort_by_key(|creature| creature.race != "oteryn:creature.rat");
    let mut data = reloaded;
    data.bounty.offers.clear();
    data.bounty.active = None;
    data.bounty.state = rules::BountyStatus::None;
    data.bounty.rarity.insert("oteryn:creature.rat".into(), 20);
    rules::apply_action(
        &mut data,
        &mut rules::TaskBoardBalances::default(),
        &TaskBoardAction::OpenBounty,
        &rules::TaskBoardContext {
            level: 8,
            has_vocation: true,
            now_unix_seconds: 1_800_000_000,
            period: 0,
            creatures: &creatures,
            available_gold: 0,
            reroll_tokens: 0,
        },
        &mut |_| 0,
    )
    .map_err(debug)?;
    let platinum = data.bounty.offers.first().ok_or("rat offer")?.clone();
    assert_eq!(
        (
            &*platinum.race,
            platinum.grade,
            platinum.dust,
            platinum.points
        ),
        ("oteryn:creature.rat", 3, 8, 24)
    );
    save(harness, &data).await?;
    let selected = full_board(owner, admitted, 8, 0, "").await?;
    assert!(
        selected
            .bounty
            .active
            .as_ref()
            .is_some_and(|task| task.elite && task.grade == 3)
    );
    assert!(
        !fixture_board_data(harness)
            .await?
            .bounty
            .rarity
            .contains_key(&platinum.race)
    );
    elite(owner, admitted, false).await?;
    assert!(
        full_board(owner, admitted, 4, 0, "")
            .await?
            .bounty
            .active
            .as_ref()
            .is_some_and(|task| task.elite)
    );
    owner.refresh_quest_session(session, false).await;

    // A real incoming creature hit commits twenty HP for an ordinary ten-HP magnitude.
    {
        use crate::ability::creature_bite::CreatureBiteVitals;
        use crate::gameplay_transport::attack::player_block::{IncomingCharm, PlayerBiteVitals};
        use oteryn_simulation_determinism::{DecisionOccurrenceId, GameplayDecisionRoot};
        let mut runtime = owner.runtime.lock().await;
        let root = GameplayDecisionRoot::from_bytes(runtime.content_pin().server_artifact_digest());
        let mut states = owner.states.spell_states.lock().await;
        assert!(states.elite_bounty(&runtime, actor, session, &platinum.race));
        let mut attacks = owner.states.attack.lock().await;
        let mut trained = Vec::new();
        let mut vitals = PlayerBiteVitals {
            quest_sessions: None,
            conditions: None,
            charm: Some(IncomingCharm {
                source: rat,
                catalogue: owner.imported_charms,
                root,
                occurrence: DecisionOccurrenceId::from_bytes(death_occurrence(64000)),
                sequence: 1,
                race: &platinum.race,
                base_damage: 10,
            }),
            states: &mut states,
            attack: &mut attacks,
            formula: None,
            trained: &mut trained,
        };
        let hit = vitals
            .apply_creature_damage(
                &mut runtime,
                actor,
                session,
                10,
                crate::foundation::owner_timer::SemanticTimeMicros::from_micros(
                    owner.owner_now().get(),
                ),
            )
            .ok_or("Elite incoming hit")?;
        assert_eq!(hit.damage.applied, 20);
    }
    println!("<T201> POSTGRES_CASE_COMPLETED: elite_incoming_real_vitals_owner");

    // Premium is an input authenticated by the existing kill owner. Exercise both frozen decisions
    // through that owner's actual transaction and forge ledger, with independent death receipts.
    for (premium, serial, expected_dust) in [(false, 64001_u16, 0_u32), (true, 64002, 8)] {
        let mut current = fixture_board_data(harness).await?;
        current.bounty.selection_serial += 1;
        current.bounty.state = rules::BountyStatus::Active;
        let mut active = platinum.clone();
        active.current = active.required - 1;
        current.bounty.active = Some(active);
        current.bounty.offers.clear();
        save(harness, &current).await?;
        let before_dust = dust(harness).await?;
        settle_kill(owner, harness, admitted, premium, serial).await?;
        assert_eq!(dust(harness).await?, before_dust + expected_dust);
        settle_kill(owner, harness, admitted, premium, serial).await?;
        assert_eq!(dust(harness).await?, before_dust + expected_dust);
        // A distinct death before task credit also cannot award completion dust twice.
        settle_kill(owner, harness, admitted, premium, serial + 10).await?;
        assert_eq!(dust(harness).await?, before_dust + expected_dust);
        let fence = owner
            .current_quest_fence(session)
            .await
            .map_err(|()| "fence")?
            .ok_or("fence")?;
        assert!(
            owner
                .credit_task_board_kill(fence, &platinum.race, death_occurrence(serial))
                .await
        );
        let before_xp = experience(harness).await?;
        let claim = full_board(owner, admitted, 9, 0, "").await?;
        assert_eq!(claim.refusal, 0);
        assert_eq!(
            experience(harness).await?,
            before_xp + i64::try_from(platinum.experience)?
        );
        assert_eq!(
            claim.bounty_points,
            u64::from(platinum.points) * if premium { 2 } else { 1 }
        );
    }
    println!("<T201> POSTGRES_CASE_COMPLETED: platinum_claim_real_xp_and_premium_only_dust_replay");
    Ok(())
}

async fn dust(harness: &Harness) -> TestResult<u32> {
    let amount: Option<i64> = sqlx::query_scalar(
        "SELECT balance FROM game_character_forge_dust WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(id(41).as_slice())
    .fetch_optional(&harness.pool)
    .await?;
    Ok(u32::try_from(amount.unwrap_or(0))?)
}

async fn settle_kill(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    harness: &Harness,
    admitted: &AdmittedSession,
    premium: bool,
    serial: u16,
) -> TestResult {
    use crate::durability::{
        character_progression::{ExperienceAwardRequest, ExperienceRewardOccurrence},
        party_kill_xp::{self, PartyKillXpMember, PartyKillXpRequest},
    };
    let mut slot = owner
        .revision_sequencer
        .acquire(crate::domain::CharacterId::from_bytes(id(41)).map_err(debug)?)
        .await;
    slot.observe_endgame_progression();
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let crate::foundation::RuntimeScopeRefV1::Channel {
        world_id,
        channel_id,
    } = fence.runtime_scope
    else {
        return Err("channel".into());
    };
    let mut death = [0_u8; 52];
    death[..16].copy_from_slice(world_id.as_bytes());
    death[16..32].copy_from_slice(channel_id.as_bytes());
    death[32..40].copy_from_slice(&fence.scope_ownership_generation.get().to_be_bytes());
    death[40..44].copy_from_slice(&7_u32.to_be_bytes());
    death[44..52].copy_from_slice(&u64::from(serial).to_be_bytes());
    let binding = owner
        .progression_binding(admitted.game_session_id)
        .ok_or("progression")?;
    let permit = owner.spell_lane_permit().await.ok_or("lane")?;
    let mut tx = harness.pool.begin().await?;
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
    party_kill_xp::commit_in_transaction(
        &mut tx,
        &scope,
        owner.holder,
        PartyKillXpRequest {
            death_key: death.to_vec(),
            principal: fence.character_id,
            creature: "oteryn:creature.rat".into(),
            base_xp: 5,
            death_at_ms: u64::from(serial),
            death_at_us: u64::from(serial) * 1000,
            shared: false,
            party_id: None,
            party_revision: None,
            members: vec![PartyKillXpMember {
                fence,
                premium,
                forge_dust: None,
                live_stamina: None,
                shared: false,
                party_id: None,
                party_revision: None,
                request: ExperienceAwardRequest {
                    occurrence: ExperienceRewardOccurrence::from_bytes(death_occurrence(serial))
                        .map_err(debug)?,
                    amount: oteryn_simulation_determinism::ExactI64::new(5),
                    context: binding.context.clone(),
                    policy_revision: binding.policy_revision.clone(),
                    reward_revision: binding.reward_revision.clone(),
                    policy: binding.policy.clone(),
                },
            }],
        },
    )
    .await
    .map_err(debug)?;
    tx.commit().await?;
    slot.observe_endgame_progression();
    Ok(())
}
