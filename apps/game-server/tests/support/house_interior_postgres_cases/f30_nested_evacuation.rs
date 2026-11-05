//! Canary ae423e0e: the native inbox refuses nonempty containers atomically.
//! This proves exact-N retained custody on abort, not successful nested delivery.
use super::*;

#[test]
fn f30_nested_evacuation_abort_restores_exact_identity_set() -> TestResult {
    super::f29_reclaim_subject::case("house_interior_f30_nested", true, false)
}

#[test]
fn f30_flat_evacuation_moves_exact_identity_set() -> TestResult {
    super::f29_reclaim_subject::case("house_interior_f30_flat", false, false)
}
