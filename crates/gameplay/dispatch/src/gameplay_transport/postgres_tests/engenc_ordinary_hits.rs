//! M30a: the framed ordinary-attack caller must deliver its own native hit credit.
use super::party_xp_fixtures::{self as party_xp, Case};
use crate::bestiary_postgres_harness::TestResult;

#[test]
fn engenc_postgres_framed_ordinary_spear_hit_credits_child_training() -> TestResult {
    party_xp::run(Case::EngencHits)
}

#[test]
fn engenc_postgres_framed_magic_missile_hit_credits_child_training() -> TestResult {
    super::item_look_fixtures::large_stack(|| {
        super::fixture::run(async |admin| {
            super::rune_use_bridge_fixtures::run_case(admin, super::rune_use_bridge_fixtures::Case::EngencTraining)
                .await
        })
    })
}
