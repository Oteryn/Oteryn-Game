use super::*;

#[test]
fn rejected_reads_spells_unavailable_and_others_are_distinct() {
    let mut feedback = SpellFeedback::default();
    assert_eq!(feedback.text(), None);
    feedback.record(SpellCastDisposition::Rejected);
    assert_eq!(feedback.text(), Some("Spells unavailable"));
    feedback.record(SpellCastDisposition::NotEnoughMana);
    assert_eq!(feedback.text(), Some("Not enough mana"));
}
