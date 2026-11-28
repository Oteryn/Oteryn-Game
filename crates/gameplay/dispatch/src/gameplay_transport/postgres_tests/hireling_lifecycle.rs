//! E12 native lamp custody, exact House authority and immutable replay.
//! Item creation uses the production native Inbox mint owner. Only the independent
//! Store entitlement is seeded; Store fulfilment has its own integration suite.
use super::hireling_lifecycle_fixtures::{Case, run_case};

use super::TestResult;

#[test]
fn srx3_rotten_essence_named_house_native_blue_effect_postgres() -> TestResult {
    run_case(Case::EssenceCue)
}

#[test]
fn srx3_rotten_essence_named_house_real_handoff_postgres() -> TestResult {
    run_case(Case::EssenceUse)
}

#[test]
fn hireling_postgres_native_inbox_lamp_fences_replay_and_return() -> TestResult {
    run_case(Case::Lifecycle)
}
#[test]
fn hireling_postgres_full_inbox_refusal_then_offline_recall_and_replay() -> TestResult {
    run_case(Case::InboxFull)
}
#[test]
fn hireling_postgres_0405_upgrade_preserves_owned_banker_and_real_provenance() -> TestResult {
    run_case(Case::LegacyConversion)
}
#[test]
fn hireling_postgres_scoped_bank_deposit_withdraw_and_replay() -> TestResult {
    run_case(Case::Bank)
}
#[test]
fn hireling_postgres_guest_service_rechecks_acl_and_house_content_fence() -> TestResult {
    run_case(Case::GuestAcl)
}
#[test]
fn hireling_postgres_client_use_management_durable_replay_and_lost_ingress_refusal() -> TestResult {
    run_case(Case::Wire)
}
#[test]
fn hireling_postgres_command220_durable_replay_and_lost_ingress_refusal() -> TestResult {
    run_case(Case::LegacyWire)
}
#[test]
fn hireling_postgres_trader_maximum_price_no_room_and_replay() -> TestResult {
    run_case(Case::Trader)
}
#[test]
fn hireling_postgres_gourmet_use_audit_cooldown_and_persistent_conditions() -> TestResult {
    run_case(Case::Gourmet)
}
#[test]
fn hireling_postgres_cook_carrot_purchase_package_and_replay() -> TestResult {
    run_case(Case::Cook)
}
#[test]
fn hireling_postgres_cook_bank_only_payment_without_backpack() -> TestResult {
    run_case(Case::CookBank)
}
#[test]
fn hireling_postgres_steward_native_stash_replay_and_revoked_acl() -> TestResult {
    run_case(Case::Steward)
}
#[test]
fn hireling_postgres_trader_wrapped_furniture_native_house_roundtrip() -> TestResult {
    run_case(Case::Furniture)
}
#[test]
fn hireling_postgres_lamp_pending_native_commit_and_refusal_recovery() -> TestResult {
    run_case(Case::LampPending)
}
#[test]
fn hireling_postgres_store_named_house_lamp_entitlement_and_replay() -> TestResult {
    run_case(Case::Store)
}
#[test]
fn hireling_postgres_store_lamp_claim_uses_the_placement_recovery_generation() -> TestResult {
    run_case(Case::StoreClaimAfterRecovery)
}
#[test]
fn hireling_postgres_house_position_ordered_capture_and_same_actor_resume() -> TestResult {
    run_case(Case::PositionCapture)
}
#[test]
fn hireling_postgres_house_logout_final_capture_and_authenticated_fresh_relog() -> TestResult {
    run_case(Case::PositionRelog)
}
#[test]
fn hireling_postgres_house_process_loss_refuses_started_namespace_then_fresh_restores() -> TestResult
{
    run_case(Case::PositionProcessLoss)
}
#[test]
fn hireling_postgres_house_acl_loss_gourmet_logout_and_fresh_refusal_cleanup() -> TestResult {
    run_case(Case::PositionAcl)
}
#[test]
fn hireling_postgres_house_fresh_restore_refuses_current_hireling_collision() -> TestResult {
    run_case(Case::PositionOccupied)
}
