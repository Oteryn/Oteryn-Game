use super::TestResult;
use super::quest_catalogue_chests_fixtures::{cases, execute, phase_a_cohort};

#[test]
fn quest_catalogue_chests_qchk1_ring_quest_use_replay_and_refusal() -> TestResult {
    phase_a_cohort(&["ring_quest"])
}

#[test]
fn quest_catalogue_chests_qchk1_the_secret_library_quest_use_replay_and_refusal() -> TestResult {
    // The six Falcon cooldown chests have separate rotating-reward scenarios.
    let claims = [
        "oteryn:reward-claim.quest.key.id0909",
        "oteryn:reward-claim.quest.u11_80.the_secret_library.asuras.black_skull",
        "oteryn:reward-claim.quest.u11_80.the_secret_library.asuras.ebony_piece",
        "oteryn:reward-claim.quest.u11_80.the_secret_library.asuras.eye_key",
        "oteryn:reward-claim.quest.u11_80.the_secret_library.asuras.lotus_key",
        "oteryn:reward-claim.quest.u11_80.the_secret_library.asuras.peacock_ballad",
        "oteryn:reward-claim.quest.u11_80.the_secret_library.asuras.scribbled_notes",
        "oteryn:reward-claim.quest.u11_80.the_secret_library.asuras.silver_chimes",
        "oteryn:reward-claim.quest.u11_80.the_secret_library.asuras.skeleton_notes",
        "oteryn:reward-claim.quest.u11_80.the_secret_library.asuras.strand_hair",
        "oteryn:reward-claim.quest.u11_80.the_secret_library.mo_ta.bony_rod",
        "oteryn:reward-claim.quest.u11_80.the_secret_library.mo_ta.broken_compass",
        "oteryn:reward-claim.quest.u11_80.the_secret_library.mo_ta.sample_blood",
        "oteryn:reward-claim.quest.u11_80.the_secret_library.small_islands.fishing",
        "oteryn:reward-claim.quest.u11_80.the_secret_library.small_islands.hawser",
        "oteryn:reward-claim.quest.u11_80.the_secret_library.small_islands.parchment",
        "oteryn:reward-claim.quest.u11_80.the_secret_library.small_islands.sapphire",
        "oteryn:reward-claim.quest.u11_80.the_secret_library.small_islands.shovel",
    ];
    let selected: Vec<_> = cases()?
        .into_iter()
        .filter(|chest| claims.contains(&chest.claim.as_str()))
        .collect();
    for claim in claims {
        let mut placements = selected.iter().filter(|chest| chest.claim == claim);
        let placement = placements
            .next()
            .ok_or_else(|| format!("missing source placement for {claim}"))?;
        assert!(
            placements.next().is_none(),
            "unexpected extra source placement for {claim}"
        );
        assert!(
            placement.servable,
            "{claim} has no native selector at {:?}",
            placement.position
        );
    }
    execute(selected)
}
