//! Canonical PostgreSQL 17.6: genuine fresh sessions, real swings, queue and kill drain.
#![allow(clippy::expect_used)]
use super::party_xp_fixtures::{Case, run};

use crate::bestiary_postgres_harness::TestResult;

#[test]
fn party_xp_real_dispatch_equal_split() -> TestResult {
    run(Case::Shared)
}
#[test]
fn party_xp_real_dispatch_prey_then_stamina() -> TestResult {
    run(Case::Modifiers)
}
#[test]
fn party_xp_real_dispatch_inactive_member_disables_sharing() -> TestResult {
    run(Case::Inactive)
}
#[test]
fn party_xp_concurrent_replay_and_recovered_sequencers_do_not_award_twice() -> TestResult {
    run(Case::Replay)
}

#[test]
fn party_xp_real_dispatch_leader_pz_disables_sharing() -> TestResult {
    run(Case::Pz)
}
#[test]
fn party_xp_real_dispatch_last_member_failure_rolls_back_all_awards() -> TestResult {
    run(Case::Rollback)
}

#[test]
fn party_xp_real_dispatch_concurrent_kills_serialize_character_awards() -> TestResult {
    run(Case::Concurrent)
}
#[test]
fn party_xp_real_dispatch_exhausted_share_is_not_redistributed() -> TestResult {
    run(Case::Exhausted)
}

#[test]
fn party_xp_real_dispatch_preserves_foreign_damage_share() -> TestResult {
    run(Case::Foreign)
}

#[test]
fn party_xp_real_dispatch_stale_member_lease_cannot_award() -> TestResult {
    run(Case::Stale)
}

#[test]
fn party_xp_real_dispatch_lethal_first_hit_qualifies_activity() -> TestResult {
    run(Case::OneHit)
}

#[test]
fn party_xp_real_dispatch_contended_absent_pz_row_defers_without_awards() -> TestResult {
    run(Case::PzContention)
}

#[test]
fn task_board_real_dispatch_shared_member_without_damage_and_replay() -> TestResult {
    run(Case::TaskBoardParty)
}

#[test]
fn task_board_real_dispatch_all_damage_contributors_and_replay() -> TestResult {
    run(Case::TaskBoardForeign)
}

#[test]
fn bounty_talisman_real_dispatch_damage_life_and_equipped_active_bounty_gates() -> TestResult {
    run(Case::TaskBoardTalisman)
}

#[test]
fn task_board_real_dispatch_party_credit_without_progression_binding() -> TestResult {
    run(Case::TaskBoardUnbound)
}

#[test]
fn task_board_real_dispatch_shared_member_selects_after_prior_leader_death_settles() -> TestResult {
    run(Case::TaskBoardPrechange)
}

#[test]
fn task_board_real_dispatch_interleaved_principals_keep_death_order_and_final_bonus() -> TestResult
{
    run(Case::TaskBoardOrdered)
}

#[test]
fn t94_guild_season_real_kill_dispatch_membership_cut() -> TestResult {
    let started = std::time::Instant::now();
    run(Case::T94)?;
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_some() {
        eprintln!(
            "T94_GUILD_DISPATCH_PG_EXECUTED membership/dedup/ranking elapsed_ms={}",
            started.elapsed().as_millis()
        );
    }
    Ok(())
}
