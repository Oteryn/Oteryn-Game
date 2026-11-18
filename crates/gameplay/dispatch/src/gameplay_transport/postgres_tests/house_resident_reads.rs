use super::house_balloon_fixtures::{Case, run_case};
use super::house_resident_reads_fixtures::ClockCase;
use super::TestResult;

#[test]
fn house_resident_tick_postgres() -> TestResult {
    run_case(Case::ResidentTick)
}

#[test]
fn house_resident_tick_with_due_furniture_postgres() -> TestResult {
    run_case(Case::ResidentExpiryTick)
}

#[test]
fn house_resident_letter_6246_postgres() -> TestResult {
    run_case(Case::ResidentLetter)
}

#[test]
fn house_resident_mail_without_inscription_postgres() -> TestResult {
    run_case(Case::ResidentMail)
}

#[test]
fn house_resident_equip_moves_helmet_postgres() -> TestResult {
    run_case(Case::ResidentEquip)
}

#[test]
fn house_resident_energy_ring_expires_postgres() -> TestResult {
    run_case(Case::ResidentTimedEquip(ClockCase::Expiry))
}

#[test]
fn house_resident_ring_handoff_settles_elapsed_postgres() -> TestResult {
    run_case(Case::ResidentTimedEquip(ClockCase::Handoff))
}

#[test]
fn house_resident_ring_logout_after_acl_revocation_postgres() -> TestResult {
    run_case(Case::ResidentTimedEquip(ClockCase::Logout))
}

#[test]
fn house_resident_ring_expires_in_reconnect_grace_postgres() -> TestResult {
    run_case(Case::ResidentTimedEquip(ClockCase::Grace))
}

#[test]
fn house_resident_ring_tick_yields_to_registration_writer_postgres() -> TestResult {
    run_case(Case::ResidentTimedEquip(ClockCase::Writer))
}

#[test]
fn house_resident_ring_unequip_settles_elapsed_postgres() -> TestResult {
    run_case(Case::ResidentTimedEquip(ClockCase::Unequip))
}

#[test]
fn house_resident_revoked_appearance_postgres() -> TestResult {
    run_case(Case::ResidentAppearanceRevoked)
}

#[test]
fn house_resident_mount_and_dismount_visible_postgres() -> TestResult {
    run_case(Case::ResidentMount)
}

#[test]
fn house_resident_quest_titles_postgres() -> TestResult {
    run_case(Case::ResidentQuest)
}
