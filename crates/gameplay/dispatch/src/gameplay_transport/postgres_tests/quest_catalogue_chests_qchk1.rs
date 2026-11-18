use super::TestResult;
use super::quest_catalogue_chests::{cases, execute, phase_a_cohort};

#[test]
fn quest_catalogue_chests_qchk1_ring_quest_use_replay_and_refusal() -> TestResult {
    phase_a_cohort(&["ring_quest"])
}

#[test]
fn quest_catalogue_chests_qchk1_the_secret_library_quest_use_replay_and_refusal() -> TestResult {
    // The six Falcon cooldown chests have separate rotating-reward scenarios.
    let selected: Vec<_> = cases()?
        .into_iter()
        .filter(|chest| {
            chest.claim == "oteryn:reward-claim.quest.key.id0909"
                || chest
                    .claim
                    .strip_prefix("oteryn:reward-claim.quest.u11_80.the_secret_library.")
                    .is_some_and(|suffix| {
                        ["asuras.", "mo_ta.", "small_islands."]
                            .iter()
                            .any(|prefix| suffix.starts_with(prefix))
                    })
        })
        .collect();
    assert_eq!(selected.len(), 18);
    assert!(selected.iter().all(|chest| chest.servable));
    execute(selected)
}
