//! The composed quest transition and admission award must work with runtime grants.

use super::{Case, TestResult, scenario};

#[test]
fn quest_admission_runtime_role_commits_and_consumes_xp_once() -> TestResult {
    scenario(Case::RuntimeRole)
}
