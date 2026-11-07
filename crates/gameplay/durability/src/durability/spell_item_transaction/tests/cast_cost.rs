#![allow(clippy::unwrap_used)]
use super::*;
fn cost() -> CastCostBinding {
    CastCostBinding {
        vitals_revision_before: 9,
        vitals_revision_after: 10,
        mana_before: 100,
        mana_after: 60,
        soul_before: 5,
        soul_after: 3,
        cooldowns_before: vec![("spell:old".into(), 123)],
        cooldowns_after: vec![("group:attack".into(), 2000), ("spell:old".into(), 123)],
        caster_digest_before: [1; 32],
        caster_digest_after: [2; 32],
    }
}
#[test]
fn costs_bind_all_namespaced_integer_deadlines() {
    let before = encode_cast_cost(&cost()).unwrap();
    let mut changed = cost();
    changed.cooldowns_after[0].1 += 1;
    assert_ne!(
        Sha256::digest(&before),
        Sha256::digest(encode_cast_cost(&changed).unwrap())
    );
}
#[test]
fn invalid_cost_successors_fail_closed() {
    let mut c = cost();
    c.vitals_revision_after = 11;
    assert!(encode_cast_cost(&c).is_err());
    c = cost();
    c.mana_after = 101;
    assert!(encode_cast_cost(&c).is_err());
    c = cost();
    c.soul_after = 6;
    assert!(encode_cast_cost(&c).is_err());
    c = cost();
    c.cooldowns_after.reverse();
    assert!(encode_cast_cost(&c).is_err());
    c = cost();
    c.cooldowns_after
        .push(c.cooldowns_after.last().unwrap().clone());
    assert!(encode_cast_cost(&c).is_err());
    c = cost();
    c.cooldowns_after[0].0 = "attack".into();
    assert!(encode_cast_cost(&c).is_err());
    c = cost();
    c.caster_digest_after = c.caster_digest_before;
    assert!(encode_cast_cost(&c).is_err());
}
