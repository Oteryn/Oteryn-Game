use super::*;
use oteryn_gameplay_progression::condition_potions::DURATION_MS;

#[test]
fn condition_potions_apply_exact_source_bonuses_and_expire_at_ten_minutes() -> Result<(), String> {
    let at = 17_000;
    let expiry = at + u64::from(DURATION_MS) * 1_000;
    for (potion, expected) in [
        (ConditionPotion::Berserk, (0, 0, 5, 0, -10)),
        (ConditionPotion::Mastermind, (3, 0, 0, 0, 0)),
        (ConditionPotion::Bullseye, (0, 0, 0, 5, -10)),
        (ConditionPotion::Transcendence, (0, 4, 0, 0, 0)),
    ] {
        let before = ConditionStore::new();
        let next = prepared_conditions(&before, potion, at, [1; 32], [2; 16])
            .map_err(|error| format!("source condition applies: {error:?}"))?;
        assert!(
            before.instances().is_empty(),
            "PREPARE must not mutate the actual owner"
        );
        for now in [at, expiry - 1] {
            let value = next.skill_adjustments(now).ok_or("live adjustment")?;
            assert_eq!(
                (
                    value.magic_level,
                    value.fist,
                    value.melee,
                    value.distance,
                    value.shielding
                ),
                expected
            );
        }
        assert_eq!(next.skill_adjustments(expiry), Some(Default::default()));
        assert_eq!(next.skill_adjustments(expiry + 1), Some(Default::default()));
    }
    Ok(())
}

#[test]
fn condition_potions_preserve_donor_collision_instead_of_stacking() -> Result<(), String> {
    let berserk = prepared_conditions(
        &ConditionStore::new(),
        ConditionPotion::Berserk,
        1_000,
        [1; 32],
        [2; 16],
    )
    .map_err(|error| format!("berserk: {error:?}"))?;
    let mastermind = prepared_conditions(
        &berserk,
        ConditionPotion::Mastermind,
        2_000,
        [1; 32],
        [3; 16],
    )
    .map_err(|error| format!("mastermind replaces: {error:?}"))?;
    assert_eq!(mastermind.instances().len(), 1);
    let value = mastermind.skill_adjustments(2_000).ok_or("skills")?;
    assert_eq!((value.magic_level, value.melee, value.shielding), (3, 0, 0));
    Ok(())
}

#[test]
fn condition_potions_refuse_kept_current_and_overflow_without_changing_owner() -> Result<(), String>
{
    let mut store = ConditionStore::<String>::new();
    let mut values = ConditionPotion::Berserk.values();
    if let crate::foundation::ConditionValues::SpellSkills { duration_ms, .. } = &mut values {
        *duration_ms = 900_000;
    }
    let longer = ConditionDefinition::new("oteryn:test.longer-skill-condition", 1, values)
        .ok_or("longer condition")?;
    let root = GameplayDecisionRoot::from_bytes([1; 32]);
    store
        .apply(
            &longer,
            None,
            ConditionSourceKind::SelfUse,
            &[],
            &ApplicationFacts {
                now: 1_000,
                base_speed: 0,
                mana_shield_capacity: 0,
                target_reentry_protected: false,
                source_reentry_protected: false,
                target_is_player: true,
                decision_root: &root,
                occurrence: DecisionOccurrenceId::from_bytes([1; 16]),
            },
        )
        .map_err(|error| format!("longer buff: {error:?}"))?;
    let held = store.clone();
    assert_eq!(
        prepared_conditions(&store, ConditionPotion::Berserk, 2_000, [1; 32], [2; 16]),
        Err(UseDisposition::Rejected)
    );
    assert_eq!(store, held);
    assert_eq!(
        prepared_conditions(
            &ConditionStore::new(),
            ConditionPotion::Berserk,
            u64::MAX,
            [1; 32],
            [2; 16]
        ),
        Err(UseDisposition::Rejected)
    );
    Ok(())
}
