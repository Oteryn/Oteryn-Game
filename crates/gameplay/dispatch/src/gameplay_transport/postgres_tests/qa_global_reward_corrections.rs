//! Exact reward identity of the first Power Ring Quest box through dispatch USE.
use super::TestResult;
use sha2::{Digest, Sha256};

#[test]
fn qa_written_reward_claims_bind_corrected_text_hashes() -> TestResult {
    for (claim, position, uid, appearance, digest) in [
        (
            "oteryn:reward-claim.kv.powerbolt2",
            (32814, 32281, -8),
            6112,
            4240,
            "82c070d062d19713a8dcb51b748174e179a9570800e07a316e209d641708119f",
        ),
        (
            "oteryn:reward-claim.quest.u7_9.dreamers_challenge.chest_book",
            (32850, 32285, -14),
            6196,
            2472,
            "feb0cc940641e7a9e502fe1bfe7437958b826ee3938270d5f6217c010e751658",
        ),
        (
            "oteryn:reward-claim.quest.u7_9.the_pits_of_inferno.chest_tible",
            (32828, 32340, -7),
            6202,
            2472,
            "c80460595bd34757c3d5065a1af387e1a5af38b8d8c54813d153add07bad182b",
        ),
    ] {
        let world = super::quest_callbacks_fixtures::source_world(position, uid, appearance, 0)?;
        let report = crate::content::world_reward_claims::bind_embedded_reward_claims(
            world.base().as_ref(),
            &|_| true,
        )?;
        let served = report
            .served
            .iter()
            .find(|entry| entry.key.as_str() == claim)
            .ok_or_else(|| format!("UID {uid}: claim did not bind"))?;
        assert_eq!(served.placements.len(), 1, "UID {uid}");
        let body = served.placements[0]
            .root_text
            .as_deref()
            .ok_or_else(|| format!("UID {uid}: missing bound text"))?;
        let actual = Sha256::digest(body.as_bytes())
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        assert_eq!(actual, digest, "UID {uid}");
    }
    Ok(())
}

#[test]
fn qa_corrected_books_persist_from_shipped_chests() -> TestResult {
    use super::quest_catalogue_chests_fixtures::{cases_including, execute_committed};
    const CLAIMS: [&str; 2] = [
        "oteryn:reward-claim.quest.u7_9.dreamers_challenge.chest_book",
        "oteryn:reward-claim.quest.u7_9.the_pits_of_inferno.chest_tible",
    ];
    let cases: Vec<_> = cases_including(|key| CLAIMS.contains(&key))?
        .into_iter()
        .filter(|case| CLAIMS.contains(&case.claim.as_str()))
        .collect();
    assert_eq!(cases.len(), CLAIMS.len());
    for (claim, unique, root) in [
        (CLAIMS[0], 6196, "oteryn:item.tibia.i2821"),
        (CLAIMS[1], 6202, "oteryn:item.tibia.i2836"),
    ] {
        let case = cases.iter().find(|case| case.claim == claim).ok_or(claim)?;
        assert_eq!(
            (
                case.unique,
                case.root.as_str(),
                case.quantity,
                case.texts.len()
            ),
            (unique, root, 1, 1)
        );
    }
    execute_committed(cases)
}

#[test]
fn qa_ghostsilver_lantern_claim_mints_once_and_use_toggles_both_ways() -> TestResult {
    use super::quest_catalogue_chests_fixtures::{cases_including, execute_committed};
    const CLAIM: &str = "oteryn:reward-claim.kv.ghostsilver_lantern_quest";
    let cases: Vec<_> = cases_including(|key| key == CLAIM)?
        .into_iter()
        .filter(|case| case.claim == CLAIM)
        .collect();
    assert_eq!(cases.len(), 1);
    assert_eq!(
        (cases[0].unique, cases[0].root.as_str(), cases[0].quantity),
        (6310, "oteryn:item.tibia.i23738", 1)
    );
    execute_committed(cases)
}

// Crystal 201189f chest.lua:1755–1761 grants one gold coin at UID 6151.
#[test]
fn qa_power_ring_quest_first_box_grants_crystal_gold_once() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result: TestResult = std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| -> Result<(), String> {
            super::qready1_reward_claims::claim_case(
                "oteryn:reward-claim.kv.powerring1",
                (32599, 31776, -9),
                &[],
                2473,
                "oteryn:item.tibia.i3031",
                1,
            )
            .map_err(|e| e.to_string())
        })?
        .join()
        .map_err(|_| "power ring dispatch panicked")?
        .map_err(Into::into);
    eprintln!(
        "POSTGRES_CASE_COMPLETED qa_power_ring_quest duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}
