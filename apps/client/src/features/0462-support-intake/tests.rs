use crate::support_delivery::{Evidence, EvidenceOwner};
#[test]
fn support_evidence_window_shows_server_data_only() {
    let mut owner = EvidenceOwner::default();
    owner.observe(Evidence::new(
        "approved-crash.txt".into(),
        6144,
        "exact owner evidence".into(),
    ));
    assert_eq!(owner.rows()[0].text, "exact owner evidence");
    assert!(!owner.rows()[0].pending);
}
#[test]
fn support_evidence_actions_are_bound_and_confirmed() {
    let mut owner = EvidenceOwner::default();
    owner.observe(Evidence::new(
        "approved-crash.txt".into(),
        6144,
        "exact evidence".into(),
    ));
    let expected = owner.rows()[0].clone();
    assert!(owner.submit(&expected, false, false, |_| true).is_none());
    assert!(owner.submit(&expected, true, false, |_| true).is_some());
    assert!(owner.rows()[0].pending);
    assert!(owner.rows()[0].result.is_none());
    assert!(owner.submit(&expected, true, false, |_| true).is_none());
}
#[test]
fn support_evidence_rejected_when_session_replaced_or_target_changed() {
    let mut owner = EvidenceOwner::default();
    let evidence = Evidence::new("approved-crash.txt".into(), 6144, "exact evidence".into());
    owner.observe(evidence.clone());
    assert!(owner.submit(&evidence, true, true, |_| true).is_none());
    let mut changed = evidence.clone();
    changed.text = "changed".into();
    assert!(owner.submit(&changed, true, false, |_| true).is_none());
    assert_eq!(owner.rows(), &[evidence]);
}
