use super::*;

#[test]
fn fury_gate_requires_all_three_owner_facts_at_the_exact_level_boundary() {
    assert!(!fury_admitted(false, true, 60));
    assert!(!fury_admitted(true, false, 60));
    assert!(!fury_admitted(true, true, 59));
    assert!(fury_admitted(true, true, 60));
    assert!(fury_admitted(true, true, 61));
}
