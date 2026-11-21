//! SPELL-AUGMENT-1: framed ordinary cast with a real PostgreSQL equipment read.
use super::TestResult;

#[test]
fn item_spell_augments_postgres_equipped_healing_cooldown_unequip_and_replay() -> TestResult {
    super::parameter_spell_fixtures::scenario(4)
}

#[test]
fn item_spell_augments_postgres_damage_critical_leech_and_replay() -> TestResult {
    super::parameter_spell_fixtures::scenario(5)
}

#[test]
fn dmg2_postgres_equipment_spell_leech_without_augment_and_replay() -> TestResult {
    super::parameter_spell_fixtures::scenario(6)
}
