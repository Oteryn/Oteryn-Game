use crate::domain::progression::ProgressionRevisionContext;
use crate::qualification_spell_character::{
    PLAYABLE_VOCATIONS, TestResult, debug, fixture_policy, progression_busy, retry_holder,
    source_experience,
};

#[tokio::test]
async fn qualification_holder_busy_retries_then_succeeds() {
    use crate::durability::DurabilityError;
    let mut attempts = 0;
    let result = retry_holder(
        None,
        tokio::time::Instant::now() + std::time::Duration::from_secs(1),
        |error| matches!(error, DurabilityError::RootUnavailable),
        async || {
            attempts += 1;
            if attempts == 1 {
                Err(DurabilityError::RootUnavailable)
            } else {
                Ok(42)
            }
        },
    )
    .await;
    assert!(matches!(result, Ok(42)), "{result:?}");
    assert_eq!(attempts, 2);
}

#[tokio::test]
async fn qualification_holder_busy_stops_at_deadline() {
    use crate::durability::DurabilityError;
    let mut attempts = 0;
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_millis(20);
    let result: Result<(), _> = retry_holder(
        None,
        deadline,
        |error| matches!(error, DurabilityError::RootUnavailable),
        async || {
            attempts += 1;
            Err(DurabilityError::RootUnavailable)
        },
    )
    .await;
    assert!(matches!(result, Err(DurabilityError::RootUnavailable)));
    assert!(tokio::time::Instant::now() >= deadline);
    assert_eq!(attempts, 1);
}

#[tokio::test]
async fn qualification_holder_nonbusy_error_is_final() {
    use crate::durability::{DurabilityError, character_progression::CharacterProgressionError};
    for unknown_commit in [false, true] {
        let mut attempts = 0;
        let result: Result<(), _> = retry_holder(
            None,
            tokio::time::Instant::now() + std::time::Duration::from_secs(1),
            progression_busy,
            async || {
                attempts += 1;
                if unknown_commit {
                    Err(CharacterProgressionError::Unavailable(
                        DurabilityError::CommitOutcomeUnknown,
                    ))
                } else {
                    Err(CharacterProgressionError::AuthorityRejected)
                }
            },
        )
        .await;
        if unknown_commit {
            assert!(matches!(
                result,
                Err(CharacterProgressionError::Unavailable(
                    DurabilityError::CommitOutcomeUnknown
                ))
            ));
        } else {
            assert!(matches!(
                result,
                Err(CharacterProgressionError::AuthorityRejected)
            ));
        }
        assert_eq!(attempts, 1);
    }
}

#[test]
fn source_fixture_matches_known_level_thresholds() -> TestResult {
    for (level, xp) in [(1, 0), (2, 100), (8, 4200), (20, 98800), (100, 15694800)] {
        assert_eq!(source_experience(level)?, xp);
    }
    assert!(source_experience(0).is_err());
    assert!(source_experience(1002).is_err());
    Ok(())
}

#[test]
fn finite_policy_covers_every_base_vocation_level_without_extrapolation() -> TestResult {
    let context = ProgressionRevisionContext {
        profile: "fixture-profile".into(),
        ruleset: "fixture-ruleset".into(),
        content: "fixture-content".into(),
        simulation: "fixture-simulation".into(),
        evidence: "fixture-evidence".into(),
        declaration: "fixture-declaration".into(),
    };
    let policy = fixture_policy(context.clone())?;
    crate::domain::progression::validate_policy(&policy).map_err(debug)?;
    assert_eq!(policy.context, context);
    assert_eq!(policy.thresholds[999].level, 1000);
    for vocation in PLAYABLE_VOCATIONS {
        assert!(crate::spell::Vocation::from_key(vocation).is_some());
    }
    assert!(!PLAYABLE_VOCATIONS.contains(&"master_sorcerer"));
    Ok(())
}
