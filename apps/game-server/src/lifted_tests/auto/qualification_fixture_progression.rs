//! Fixture freshness after the real admission progression binder has run.
use crate::bestiary_postgres_harness::{
    Harness, TestResult, configured_admin, context, debug, fence, id, runtime,
};
use crate::domain::progression::{FiniteProgressionPolicy, LevelThreshold};
use crate::durability::character_build::{
    BuildCause, BuildChangeRequest, BuildOccurrence, convert_vocation,
};
use crate::durability::character_progression::{
    ExperienceAwardRequest, ExperienceRewardOccurrence,
};
use crate::durability::character_revision_sequencer::CharacterRevisionSequencer;
use crate::qualification_spell_character::{QualificationBuildSpec, prepare_character_with_policy};
use oteryn_simulation_determinism::{ExactI64, RoundingMode};

// Use the same production binder as the existing progression PostgreSQL target.
#[allow(
    dead_code,
    reason = "this fixture uses only the admission binding entry point"
)]
#[path = "../../../../../crates/gameplay/dispatch/src/gameplay_transport/character_progression_binding.rs"]
mod binding;

#[derive(Clone, Copy)]
enum Case {
    Admitted,
    Advanced,
    Vocation,
    Died,
    MismatchedContext,
    MismatchedPolicy,
    MismatchedReward,
    BootstrapOnly,
}

fn policy() -> FiniteProgressionPolicy<String, 2> {
    FiniteProgressionPolicy {
        context: context(),
        policy_revision: "policy-1".into(),
        reward_revision: "reward-1".into(),
        death_policy_revision: "death-1".into(),
        declared_difference_revision: "declaration-1".into(),
        thresholds: [
            LevelThreshold {
                level: 1,
                minimum_experience: ExactI64::new(0),
            },
            LevelThreshold {
                level: 2,
                minimum_experience: ExactI64::new(100),
            },
        ],
        terminal_exclusive_experience: ExactI64::new(200),
        death_loss_numerator: 1,
        death_loss_denominator: 1,
        death_loss_rounding: RoundingMode::Floor,
    }
}

fn run(case: Case, tag: &'static str) -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            runtime()
                .map_err(debug)?
                .block_on(async {
                    let h = Harness::create(admin, tag, false).await?;
                    let result = exercise(&h, case).await;
                    h.cleanup().await?;
                    result
                })
                .map_err(debug)
        })?
        .join()
        .map_err(|_| "qualification fixture test thread failed")?
        .map_err(Into::into)
}

// The shared harness seeds the admitted session/fences, but not a FreshAdmissionStore
// receipt. This test-only snapshot describes that session; all fixture writes still
// independently verify its generations, lease and scope against PostgreSQL.
fn fixture_session() -> TestResult<
    crate::foundation::GameSessionAuthoritySnapshot<crate::foundation::AuthenticatedTransportRefV1>,
> {
    use crate::foundation::{
        AuthenticatedTransportRefV1, ChannelId, CharacterId, CharacterLease, FreshAdmissionCommit,
        FreshAdmissionFacts, GameSessionAuthoritySnapshot, GameSessionState, WorldId,
    };
    let initial = fence(1)?;
    let character = CharacterId::decode(&id(41)).map_err(debug)?;
    let facts = FreshAdmissionFacts::new(
        [1; 32],
        character,
        WorldId::decode(&id(42)).map_err(debug)?,
        ChannelId::decode(&id(43)).map_err(debug)?,
        1,
        1,
    )
    .map_err(debug)?;
    let transport = AuthenticatedTransportRefV1::decode(&[9; 16]).map_err(debug)?;
    let commit = FreshAdmissionCommit::from_facts(initial.game_session_id, facts, transport)
        .map_err(debug)?;
    Ok(GameSessionAuthoritySnapshot::new(
        commit,
        GameSessionState::Active,
        initial.connection_generation,
        Some(transport),
        CharacterLease::new(character, 1).map_err(debug)?,
        initial.scope_ownership_generation,
    ))
}

async fn exercise(h: &Harness, case: Case) -> TestResult {
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let initial = fence(1)?;
    if !matches!(case, Case::BootstrapOnly) {
        let bound = binding::bind_character_progression(
            binding::ProgressionDurable {
                root: &h.root,
                character: &authority,
                holder: &h.node,
            },
            |_, _, _| Some(policy()),
            || async { Ok(Some(initial)) },
            binding::ProgressionRounds {
                attempts: 1,
                backoff: std::time::Duration::ZERO,
            },
        )
        .await;
        assert_eq!(
            bound,
            binding::ProgressionBinding::Bound(Box::new(policy()))
        );
        let stored = h
            .root
            .read_character_progression(&authority, initial.character_id)
            .await
            .map_err(debug)?
            .ok_or("admission did not initialize progression")?;
        assert_eq!(stored.character_revision.get(), 1);
        assert_eq!(stored.level, 1);
        assert_eq!(stored.total_experience, ExactI64::new(0));
    }
    let sequencer = CharacterRevisionSequencer::new();
    let mut slot = sequencer.acquire(initial.character_id).await;
    if matches!(case, Case::Advanced) {
        // Still level one: even one earned XP must make the Character ineligible.
        slot.commit_experience(
            &h.root,
            &authority,
            &h.node,
            initial,
            ExperienceAwardRequest {
                occurrence: ExperienceRewardOccurrence::from_bytes(id(81)).map_err(debug)?,
                amount: ExactI64::new(1),
                context: context(),
                policy_revision: "policy-1".into(),
                reward_revision: "reward-1".into(),
                policy: policy(),
            },
            None,
        )
        .await
        .map_err(debug)?;
    } else if matches!(case, Case::Vocation) {
        let formula = crate::spell::mana_training::CompiledTrainingFormula::from_profile(
            include_bytes!(
                "../../../../../tools/content-schema/native-gameplay/build-training.json"
            ),
            &context().content,
        )
        .map_err(debug)?;
        let before = h
            .root
            .read_character_build_state(&authority, initial.character_id)
            .await
            .map_err(debug)?;
        let after = convert_vocation(&formula, &before, "druid").map_err(debug)?;
        slot.commit_build(
            &h.root,
            &authority,
            &h.node,
            initial,
            BuildChangeRequest {
                occurrence: BuildOccurrence::from_bytes(id(82)).map_err(debug)?,
                cause: BuildCause::VocationChoice,
                before,
                after,
                pruned_stance: None,
            },
            &formula,
            None,
        )
        .await
        .map_err(debug)?;
    }
    drop(slot);
    if matches!(case, Case::Died) {
        use crate::durability::character_death::{
            CharacterDeathRequest, DeathCell, PlayerDeathOccurrence,
        };
        h.root
            .settle_player_death(
                &sequencer,
                &authority,
                &h.node,
                initial,
                CharacterDeathRequest {
                    public_cause: None,
                    pvp_contributors: Vec::new(),
                    occurrence: PlayerDeathOccurrence::from_bytes(id(85)).map_err(debug)?,
                    context: context(),
                    policy_revision: "policy-1".into(),
                    reward_revision: "reward-1".into(),
                    policy: policy(),
                    held_blessings: Vec::new(),
                    death_cell: DeathCell {
                        runtime_scope: initial.runtime_scope,
                        spatial_position: vec![1, 2, 3, 7],
                        map_revision: "map-1".into(),
                    },
                    respawn_position: b"temple:thais".to_vec(),
                },
            )
            .await
            .map_err(debug)?;
        let state = h
            .root
            .read_character_progression(&authority, initial.character_id)
            .await
            .map_err(debug)?
            .ok_or("death progression absent")?;
        assert_eq!(state.level, 1);
        assert_eq!(state.total_experience, ExactI64::new(0));
        assert_eq!(state.character_revision.get(), 2);
    }
    let before_progression = h
        .root
        .read_character_progression(&authority, initial.character_id)
        .await
        .map_err(debug)?;
    let before_build = h
        .root
        .read_character_build_state(&authority, initial.character_id)
        .await
        .map_err(debug)?;
    let current = fixture_session()?;
    let mut served = policy();
    if matches!(case, Case::MismatchedContext) {
        served.context.evidence = "other-evidence".into();
    }
    if matches!(case, Case::MismatchedPolicy) {
        served.policy_revision = "other-policy".into();
    }
    if matches!(case, Case::MismatchedReward) {
        served.reward_revision = "other-reward".into();
    }
    let result = prepare_character_with_policy(
        &h.root,
        &authority,
        &h.node,
        &current,
        QualificationBuildSpec {
            vocation: "knight",
            level: 2,
            context: served.context.clone(),
            xp_occurrence: id(83),
            build_occurrence: id(84),
        },
        served,
    )
    .await;
    if matches!(case, Case::Admitted | Case::BootstrapOnly) {
        result?;
        let after = h
            .root
            .read_character_progression(&authority, initial.character_id)
            .await
            .map_err(debug)?
            .ok_or("provisioned progression absent")?;
        assert_eq!(after.level, 2);
        assert_eq!(after.total_experience, ExactI64::new(100));
        assert_eq!(after.character_revision.get(), 3);
        assert_eq!(
            h.root
                .read_character_build_state(&authority, initial.character_id)
                .await
                .map_err(debug)?
                .vocation(),
            "knight"
        );
        assert_eq!(
            h.root
                .read_current_character(&authority, initial.character_id)
                .await
                .map_err(debug)?
                .revision
                .get(),
            3
        );
    } else {
        let error = result
            .err()
            .ok_or("fixture overwrote a non-fresh Character")?;
        assert_eq!(
            error.to_string(),
            "spell fixture requires a fresh disposable Character"
        );
        assert_eq!(
            h.root
                .read_character_progression(&authority, initial.character_id)
                .await
                .map_err(debug)?,
            before_progression
        );
        assert_eq!(
            h.root
                .read_character_build_state(&authority, initial.character_id)
                .await
                .map_err(debug)?,
            before_build
        );
    }
    Ok(())
}

#[test]
fn qualification_fixture_accepts_admission_initialized_character() -> TestResult {
    run(Case::Admitted, "qfixture_admitted")
}

#[test]
fn qualification_fixture_refuses_earned_experience() -> TestResult {
    run(Case::Advanced, "qfixture_advanced")
}

#[test]
fn qualification_fixture_refuses_existing_vocation() -> TestResult {
    run(Case::Vocation, "qfixture_vocation")
}

#[test]
fn qualification_fixture_refuses_different_policy() -> TestResult {
    run(Case::MismatchedPolicy, "qfixture_policy")
}

#[test]
fn qualification_fixture_accepts_bootstrap_without_progression() -> TestResult {
    run(Case::BootstrapOnly, "qfixture_bootstrap")
}

#[test]
fn qualification_fixture_refuses_death_at_initial_level_and_experience() -> TestResult {
    run(Case::Died, "qfixture_died")
}

#[test]
fn qualification_fixture_refuses_different_context() -> TestResult {
    run(Case::MismatchedContext, "qfixture_context")
}

#[test]
fn qualification_fixture_refuses_different_reward() -> TestResult {
    run(Case::MismatchedReward, "qfixture_reward")
}
