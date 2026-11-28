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
use crate::durability::loop_training::TrainingContext;
use crate::qualification_spell_character::{QualificationBuildSpec, prepare_character_with_policy};
use oteryn_gameplay_dispatch::gameplay_transport::composed_test_owner as binding;
use oteryn_gameplay_progression::loop_training::TrainingRates;
use oteryn_simulation_determinism::{ExactI64, RoundingMode};
use std::sync::Arc;

#[derive(Clone, Copy)]
enum Case {
    Admitted,
    BusyHolder,
    Advanced,
    FixtureXpCommitted,
    Vocation,
    Skill,
    EarnedThenDied,
    Died,
    MismatchedContext,
    MismatchedPolicy,
    MismatchedReward,
    BootstrapOnly,
    BootstrapVocation,
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
                    let h = if matches!(case, Case::BootstrapVocation) {
                        Harness::create_with_bootstrap_vocation(admin, tag, "Druid").await?
                    } else {
                        Harness::create(admin, tag, false).await?
                    };
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

#[test]
fn exact_spell_profiles_build_and_refuse_level_drift() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            runtime()
                .map_err(debug)?
                .block_on(async {
                    for (profile, expected_level, words) in [
                        ("levitate", 12, "exani hur up"),
                        ("magic-rope", 9, "exani tera"),
                    ] {
                        let h = Harness::create(
                            admin.clone(),
                            profile.replace('-', "_").as_str(),
                            false,
                        )
                        .await?;
                        let result = exercise_exact_spell(&h, profile, expected_level, words).await;
                        h.cleanup().await?;
                        result?;
                    }
                    Ok::<(), Box<dyn std::error::Error>>(())
                })
                .map_err(debug)
        })?
        .join()
        .map_err(|_| "exact spell profile thread failed")?
        .map_err(Into::into)
}

async fn exercise_exact_spell(
    h: &Harness,
    profile: &str,
    expected_level: u32,
    words: &str,
) -> TestResult {
    use crate::node::serve::fixture_build_spec;
    use crate::qualification_spell_character::{advance_source_level, fixture_policy};

    let initial = fence(1)?;
    let current = fixture_session()?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let policy = fixture_policy(context())?;
    let prepare = async || {
        let spec = fixture_build_spec(profile, initial.character_id.as_bytes(), context())
            .ok_or("exact spell profile missing")?;
        prepare_character_with_policy(&h.root, &authority, &h.node, &current, spec, policy.clone())
            .await
    };
    prepare().await?;
    prepare().await?;

    let state = h
        .root
        .read_character_progression(&authority, initial.character_id)
        .await
        .map_err(debug)?
        .ok_or("spell progression absent")?;
    let build = h
        .root
        .read_character_build_state(&authority, initial.character_id)
        .await
        .map_err(debug)?;
    assert_eq!(state.level, expected_level);
    let manifest =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content/spells.manifest.json");
    let room = crate::content::qualify_native_entry_room_from_gameplay_manifest(
        initial.runtime_scope.world_id(),
        &manifest,
    )?;
    let native = crate::content::qualified_native_gameplay_test_state(room.compiled())?;
    let vocation = crate::spell::Vocation::from_key(build.vocation()).ok_or("spell vocation")?;
    let spell = native
        .spell_book()
        .spoken(words)
        .ok_or("compiled spell absent")?
        .spell;
    assert_eq!(spell.level, state.level);
    assert!(spell.vocations.contains(&vocation));
    assert!(!spell.learning_required && !spell.premium);

    advance_source_level(
        &h.root,
        &authority,
        &h.node,
        &current,
        expected_level + 1,
        id(86),
    )
    .await?;
    let error = prepare()
        .await
        .err()
        .ok_or("fixture accepted the wrong level")?;
    assert_eq!(
        error.to_string(),
        "qualification fixture XP checkpoint has changed"
    );
    let state = h
        .root
        .read_character_progression(&authority, initial.character_id)
        .await
        .map_err(debug)?
        .ok_or("advanced progression absent")?;
    assert_eq!(state.level, expected_level + 1);
    Ok(())
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
    let training_formula = || {
        crate::spell::mana_training::CompiledTrainingFormula::from_profile(
            include_bytes!("../../../../../tools/content-schema/native-gameplay/build-training.json"),
            &context().content,
        )
        .map_err(debug)
    };
    let sequencer = CharacterRevisionSequencer::new();
    let mut slot = sequencer.acquire(initial.character_id).await;
    if matches!(case, Case::Advanced | Case::EarnedThenDied | Case::FixtureXpCommitted) {
        let checkpoint = matches!(case, Case::FixtureXpCommitted);
        slot.commit_experience(
            &h.root,
            &authority,
            &h.node,
            initial,
            ExperienceAwardRequest {
                occurrence: ExperienceRewardOccurrence::from_bytes(id(if checkpoint { 83 } else { 81 })).map_err(debug)?,
                amount: ExactI64::new(if checkpoint { 100 } else { 1 }),
                context: context(),
                policy_revision: "policy-1".into(),
                reward_revision: "reward-1".into(),
                policy: policy(),
            },
            None,
        )
        .await
        .map_err(debug)?;
    } else if matches!(case, Case::Vocation | Case::Skill) {
        let formula = training_formula()?;
        let before = h
            .root
            .read_character_build_state(&authority, initial.character_id)
            .await
            .map_err(debug)?;
        let (after, cause) = if matches!(case, Case::Vocation) {
            (
                convert_vocation(&formula, &before, "druid").map_err(debug)?,
                BuildCause::VocationChoice,
            )
        } else {
            let mut skills = before.skills();
            skills[0].1 = 1;
            (
                crate::durability::character_build::DurableBuildState::new(
                    before.vocation(),
                    before.magic(),
                    skills,
                )
                .map_err(debug)?,
                BuildCause::Training,
            )
        };
        slot.commit_build(
            &h.root,
            &authority,
            &h.node,
            initial,
            BuildChangeRequest {
                occurrence: BuildOccurrence::from_bytes(id(82)).map_err(debug)?,
                cause,
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
    if matches!(case, Case::BootstrapVocation) {
        let before = h
            .root
            .read_character_build_state(&authority, initial.character_id)
            .await
            .map_err(debug)?;
        assert_eq!(before.vocation(), "druid");
        let rates = TrainingRates::from_json(include_bytes!(
            "../../../../../rulesets/training/crystal-201189f-v1.json"
        ))
        .map_err(debug)?;
        // First admission has no predecessor. The native writer still commits its marker and build.
        let checkpoint = h
            .root
            .settle_offline_training(
                &authority,
                &h.node,
                initial,
                TrainingContext {
                    rates: Arc::new(rates),
                    formula: training_formula()?,
                    progression: None,
                },
            )
            .await
            .map_err(|error| format!("v4 no-gain admission checkpoint: {error:?}"))?;
        assert_eq!(checkpoint.revision.get(), 2);
        assert_eq!(
            h.root
                .read_character_build_state(&authority, initial.character_id)
                .await
                .map_err(debug)?,
            before
        );
    }
    if matches!(case, Case::Died | Case::EarnedThenDied) {
        use crate::durability::character_death::{
            CharacterDeathRequest, DeathCell, PlayerDeathOccurrence,
        };
        let mut death_fence = initial;
        death_fence.expected_character_revision = h
            .root
            .read_current_character(&authority, initial.character_id)
            .await
            .map_err(debug)?
            .revision;
        h.root
            .settle_player_death(
                &sequencer,
                &authority,
                &h.node,
                death_fence,
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
        assert_eq!(
            state.character_revision.get(),
            if matches!(case, Case::Died) { 2 } else { 3 }
        );
    }
    assert_eq!(
        h.root
            .has_character_experience_or_death_receipts(&authority, initial.character_id)
            .await
            .map_err(debug)?,
        matches!(case, Case::Advanced | Case::Died | Case::EarnedThenDied | Case::FixtureXpCommitted)
    );
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
    let prepare = async || {
        prepare_character_with_policy(
        &h.root,
        &authority,
        &h.node,
        &current,
        QualificationBuildSpec {
            vocation: "knight",
            level: 2,
            exact_level: false,
            context: served.context.clone(),
            xp_occurrence: id(83),
            build_occurrence: id(84),
        },
        served.clone(),
    )
    .await
    };
    let result = if matches!(case, Case::BusyHolder) {
        use crate::durability::{DurabilityError, DurabilityRoot};
        use std::future::{Future, poll_fn};
        use std::task::Poll;

        let holder = h.root.try_issue_semantic_pass()?;
        let preparation = DurabilityRoot::with_holder_owner(prepare());
        tokio::pin!(preparation);
        let first = poll_fn(|cx| Poll::Ready(preparation.as_mut().poll(cx))).await;
        assert!(first.is_pending(), "fixture must wait for the holder");
        holder.run(|_, _| Box::pin(async { Ok(()) })).await?;
        // Reproduce the scheduler racing the fixture after the holder returns.
        assert!(matches!(
            h.root.try_issue_background_semantic_pass(),
            Err(DurabilityError::RootUnavailable)
        ));
        assert!(matches!(
            DurabilityRoot::with_holder_owner(async { h.root.try_acquire_ready() }).await,
            Err(DurabilityError::RootUnavailable)
        ));
        preparation.await
    } else {
        prepare().await
    };
    if matches!(case, Case::FixtureXpCommitted | Case::BootstrapVocation) && result.is_ok() {
        // The build commit itself can also succeed before its caller loses the reply.
        prepare().await?;
    }
    if matches!(
        case,
        Case::Admitted | Case::BusyHolder | Case::BootstrapOnly | Case::FixtureXpCommitted | Case::BootstrapVocation
    ) {
        result?;
        let expected_revision = if matches!(case, Case::BootstrapVocation) {
            4
        } else {
            3
        };
        let after = h
            .root
            .read_character_progression(&authority, initial.character_id)
            .await
            .map_err(debug)?
            .ok_or("provisioned progression absent")?;
        assert_eq!(after.level, 2);
        assert_eq!(after.total_experience, ExactI64::new(100));
        assert_eq!(after.character_revision.get(), expected_revision);
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
            expected_revision
        );
    } else {
        let error = result
            .err()
            .ok_or("fixture overwrote a non-fresh Character")?;
        let expected = match case {
            Case::Advanced => "spell fixture progression level or XP is not initial",
            Case::Vocation => "spell fixture vocation is set",
            Case::Skill => "spell fixture build is not initial",
            Case::Died | Case::EarnedThenDied => {
                "spell fixture noninitial progression revision has XP or death history"
            }
            Case::MismatchedContext => "spell fixture progression context mismatch",
            Case::MismatchedPolicy => "spell fixture progression policy revision mismatch",
            Case::MismatchedReward => "spell fixture progression reward revision mismatch",
            Case::Admitted
            | Case::BusyHolder
            | Case::BootstrapOnly
            | Case::FixtureXpCommitted
            | Case::BootstrapVocation => {
                return Err("positive fixture case reached refusal assertion".into());
            }
        };
        assert_eq!(error.to_string(), expected);
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
fn qualification_fixture_accepts_v4_bootstrap_vocation_after_no_gain_checkpoint() -> TestResult {
    run(Case::BootstrapVocation, "qfixture_birth_vocation")
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

#[test]
fn qualification_fixture_refuses_earned_skills_without_xp() -> TestResult {
    run(Case::Skill, "qfixture_skill")
}

#[test]
fn qualification_fixture_refuses_earned_xp_after_death_returns_it_to_zero() -> TestResult {
    run(Case::EarnedThenDied, "qfixture_earned_death")
}

#[test]
fn qualification_fixture_resumes_its_exact_committed_experience() -> TestResult {
    run(Case::FixtureXpCommitted, "qfixture_resume_xp")
}

#[test]
fn qualification_build_gets_returned_holder_ahead_of_competing_work() -> TestResult {
    run(Case::BusyHolder, "fixture_busy_holder")
}
