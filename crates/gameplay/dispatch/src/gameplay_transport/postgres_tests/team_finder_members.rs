//! S14b/S14c qualification through persistent admitted client connections and committed rows.
#[path = "team_finder_members/cases.rs"]
mod cases;
#[path = "team_finder_members/fences.rs"]
mod fences;
#[path = "team_finder_members/fresh_request.rs"]
mod fresh_request;
#[path = "team_finder_members/relog.rs"]
mod relog;
#[path = "team_finder_members/support.rs"]
mod support;

use crate::bestiary_postgres_harness::{self as fixture, Harness, TestResult, debug};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Case {
    Request,
    Cancel,
    Invite,
    Accept,
    Deny,
    Remove,
    Logout,
    Refusals,
}

impl Case {
    fn tag(self) -> &'static str {
        match self {
            Self::Request => "tf_request",
            Self::Cancel => "tf_cancel",
            Self::Invite => "tf_invite",
            Self::Accept => "tf_accept",
            Self::Deny => "tf_deny",
            Self::Remove => "tf_remove",
            Self::Logout => "tf_logout",
            Self::Refusals => "tf_refusals",
        }
    }
}

fn run_case(case: Case) -> TestResult {
    let Some(admin) = fixture::configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .name(case.tag().into())
        .stack_size(64 << 20)
        .spawn(move || {
            fixture::runtime().map_err(debug)?.block_on(async move {
                super::reset();
                let h = Harness::create(admin, case.tag(), false)
                    .await
                    .map_err(debug)?;
                let result = support::scenario(&h, case).await.map_err(debug);
                h.cleanup().await.map_err(debug)?;
                result
            })
        })?
        .join()
        .map_err(|_| "team finder PostgreSQL scenario thread panicked")??;
    eprintln!("S14BC {} POSTGRES_CASE_COMPLETED", case.tag());
    Ok(())
}

#[test]
fn team_finder_members_request_single_pending_and_durable_replay() -> TestResult {
    run_case(Case::Request)
}

#[test]
fn team_finder_members_cancel_deletes_only_the_applicant_entry() -> TestResult {
    run_case(Case::Cancel)
}

#[test]
fn team_finder_members_invite_records_status_two_without_party_membership() -> TestResult {
    run_case(Case::Invite)
}

#[test]
fn team_finder_members_accept_uses_party_invite_then_normal_accept_and_capacity() -> TestResult {
    run_case(Case::Accept)
}

#[test]
fn team_finder_members_deny_preserves_status_until_cancellation() -> TestResult {
    run_case(Case::Deny)
}

#[test]
fn team_finder_members_remove_cascades_requests_and_rejects_retained_handle() -> TestResult {
    run_case(Case::Remove)
}

#[test]
fn team_finder_members_real_logout_closes_advertisement_but_retains_template() -> TestResult {
    run_case(Case::Logout)
}

#[test]
fn team_finder_members_refuses_malformed_wrong_owner_unknown_handle_and_stale_fence() -> TestResult
{
    run_case(Case::Refusals)
}
