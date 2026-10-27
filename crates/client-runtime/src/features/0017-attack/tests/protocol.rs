use crate::combat::CombatReport;
use oteryn_session::SpellCastDisposition;

#[test]
fn a_cast_report_has_its_refusal_line() {
    assert_eq!(
        CombatReport::Cast(SpellCastDisposition::NotEnoughMana).text(),
        Some("Not enough mana")
    );
}
