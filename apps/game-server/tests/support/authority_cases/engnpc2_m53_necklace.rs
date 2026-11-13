//! M53 uses the actual locked AMULET slot, not carried inventory or wire slot 2.
#[test]
fn engnpc2_m53_necklace_admission_is_fenced_atomic_and_replay_bound()
-> crate::bestiary_postgres_harness::TestResult {
    super::engnpc2_equipped::run_equipped_fixture("necklace", "oteryn:item.tibia.i3083", 7)
}
