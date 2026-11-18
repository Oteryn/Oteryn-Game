//! Generic source chest -> literal USE -> existing atomic claim/MINT -> reconnect/replay.
use super::quest_callbacks_fixtures::{CHEST, CLAIM, scenario_with_definition_ready_claim, source_world};

use super::TestResult;

#[test]
fn quest_callbacks_binding_and_coverage_require_exact_source_witnesses() -> TestResult {
    use crate::content::world_reward_claims::{LeftOut, bind_embedded_reward_claims};
    use std::collections::BTreeSet;
    let manifest: serde_json::Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../docs/repository/QUEST_CALLBACK_COVERAGE.json"
    )))?;
    let supported: BTreeSet<_> = manifest["quests"]
        .as_array()
        .ok_or("quests")?
        .iter()
        .flat_map(|quest| quest["chests"].as_array().into_iter().flatten())
        .chain(
            manifest["native_authored_claims"]
                .as_array()
                .into_iter()
                .flatten(),
        )
        .filter(|chest| chest["status"] != "not_playable")
        .map(|chest| chest["claim"].as_str().ok_or("claim"))
        .collect::<Result<_, _>>()?;
    // Falcon's repeatable sources use the dedicated native handler, not the
    // one-shot binder. Keep its closed runtime catalogue in the coverage proof.
    let dedicated: BTreeSet<_> = oteryn_gameplay_quests::quest::falcon::CHESTS
        .into_iter()
        .map(|chest| chest.key())
        .collect();
    assert_eq!(dedicated.len(), 6);
    let dedicated_refs: BTreeSet<_> = dedicated.iter().map(String::as_str).collect();
    let declared_dedicated: BTreeSet<_> = supported
        .iter()
        .copied()
        .filter(|key| key.starts_with(oteryn_gameplay_quests::quest::falcon::CLAIM_PREFIX))
        .collect();
    assert_eq!(declared_dedicated, dedicated_refs);
    // Timed random chests have their own durable cycle owner; the generic
    // once-claim binder must never admit them as ordinary once-only rewards.
    let falcon_keys: Vec<_> = oteryn_gameplay_quests::quest::falcon::CHESTS
        .into_iter()
        .map(|chest| chest.key())
        .collect();
    let falcon: BTreeSet<_> = falcon_keys.iter().map(String::as_str).collect();
    assert_eq!(falcon.len(), 6);
    for (unique, appearance, offset, bound) in [
        (6176, 2434, 0, true),
        (6177, 2434, 0, false),
        (6176, 2435, 0, false),
        (6176, 2434, 1, false),
    ] {
        let world = source_world(CHEST, unique, appearance, offset)?;
        let report = bind_embedded_reward_claims(world.base().as_ref(), &|_| true)?;
        assert_eq!(
            report
                .served
                .iter()
                .any(|claim| claim.key.as_str() == CLAIM),
            bound
        );
        // Cover both production owners, rather than interpreting readiness
        // fields or allowing the once binder to bypass a timed cycle.
        let candidates: BTreeSet<_> = report
            .served
            .iter()
            .map(|claim| claim.key.as_str())
            .chain(
                report
                    .unbound
                    .iter()
                    .map(|placement| placement.claim.as_str()),
            )
            .collect();
        assert!(candidates.is_disjoint(&falcon));
        for &claim in &falcon {
            assert!(
                report
                    .left_out
                    .iter()
                    .any(|(key, reason)| { key == claim && *reason == LeftOut::Variant }),
                "timed claim must remain excluded from the once binder: {claim}"
            );
        }
        let all_candidates: BTreeSet<_> = candidates.union(&falcon).copied().collect();
        assert_eq!(supported, all_candidates);
    }
    Ok(())
}

#[test]
fn quest_callbacks_source_chest_use_once_replay_and_second_use_refused() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario(false).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "quest callbacks e2e panicked")?
        .map_err(Into::into)
}

#[test]
fn quest_callbacks_bundle_chest_advances_quest_once_after_catalogue_admission() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario(true).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "quest chest e2e panicked")?
        .map_err(Into::into)
}

fn scenario(quest: bool) -> TestResult {
    scenario_with_definition_ready_claim(quest, None)
}
