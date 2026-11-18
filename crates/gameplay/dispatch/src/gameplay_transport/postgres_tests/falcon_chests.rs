//! Six repeat chests must use the dedicated durable cycle owner, never once claims.
use super::falcon_chests_fixtures::{run_falcon_chests};

use super::TestResult;

#[test]
fn falcon_chests_exact_six_sources_and_seven_rewards() -> TestResult {
    use oteryn_gameplay_quests::quest::falcon::{CHESTS, REWARDS, claim_key, draw_index};
    assert_eq!(CHESTS.len(), 6);
    assert_eq!(
        REWARDS,
        [
            (9081, 1),
            (28821, 1),
            (28823, 1),
            (9058, 1),
            (6299, 1),
            (3052, 1),
            (3035, 10)
        ]
    );
    for chest in CHESTS {
        assert!(claim_key(&chest.key()).is_some());
    }
    let mut seen = std::collections::BTreeSet::new();
    for value in 0..=255 {
        let entropy = [value; 32];
        let index = draw_index(&entropy);
        assert!(index < 7);
        assert_eq!(index, draw_index(&entropy));
        seen.insert(index);
    }
    assert_eq!(seen.len(), 7);
    Ok(())
}


#[test]
fn falcon_chests_postgres_real_use_frozen_draw_refusal_restart_and_fences() -> TestResult {
    run_falcon_chests()
}
