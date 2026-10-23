use super::*;

fn input(slot: EquipmentSlot, abilities: Vec<ItemAbility>) -> EquipmentEffectInput<'static> {
    EquipmentEffectInput {
        slot,
        definition_key: ("item", "r1"),
        in_own_slot: true,
        profile: ItemAbilityProfile {
            abilities,
            ..Default::default()
        },
    }
}

fn percent(numerator: i64, denominator: u64) -> ReferenceRationalPercent {
    ReferenceRationalPercent {
        numerator,
        denominator,
    }
}

#[test]
fn equipment_effects_speed_contribution_requires_untimed_own_slot_outside_ammo() {
    for slot in [EquipmentSlot::Head, EquipmentSlot::Ammo] {
        for in_own_slot in [false, true] {
            for timed in [false, true] {
                let mut item = input(slot, vec![ItemAbility::Speed(30)]);
                item.in_own_slot = in_own_slot;
                item.profile.timed = timed;
                let contributes = slot == EquipmentSlot::Head && in_own_slot && !timed;
                assert_eq!(item.contributes_speed(), contributes);
                assert_eq!(
                    derive_equipment_effects(vec![item]).map(|effects| effects.speed),
                    Some(if contributes { 30 } else { 0 })
                );
            }
        }
    }
}

#[test]
fn equipment_effects_single_truncation_and_checked_arithmetic() {
    assert_eq!(apply_stat(101, -4, percent(1, 2)), Some(97));
    assert_eq!(apply_stat(101, 4, percent(-3, 2)), Some(104));
    assert_eq!(apply_protection(101, percent(3, 2)), Some(100));
    assert_eq!(apply_protection(101, percent(-3, 2)), Some(102));
    assert_eq!(apply_protection(101, percent(101, 1)), Some(0));
    assert_eq!(apply_protection(101, percent(-101, 1)), Some(202));
    assert_eq!(apply_stat(i64::MAX, 1, percent(0, 1)), None);
    assert_eq!(apply_stat(i64::MAX, 0, percent(2, 1)), None);
    assert_eq!(apply_protection(i64::MAX, percent(-100, 1)), None);
    assert_eq!(apply_protection(1, percent(1, 0)), None);
    assert_eq!(apply_protection(-1, percent(0, 1)), None);
    assert_eq!(apply_stat(100, 0, percent(2, 2)), None);
    assert_eq!(apply_stat(100, 0, percent(1, u64::MAX)), None);
    assert_eq!(apply_stat(i64::MIN, -1, percent(0, 1)), None);
}

#[test]
fn equipment_effects_rational_sums_and_overflow_are_checked()
-> Result<(), Box<dyn std::error::Error>> {
    let derive = |percents: &[ReferenceRationalPercent]| {
        derive_equipment_effects(vec![input(
            EquipmentSlot::Head,
            percents
                .iter()
                .map(|&amount| ItemAbility::StatBoost {
                    stat: AbilityStat::MaxHealth,
                    amount: AbilityAmount::Percent(amount),
                })
                .collect(),
        )])
    };
    let effects = derive(&[percent(1, 2), percent(1, 3)]).ok_or("rational sum")?;
    assert_eq!(
        effects.stat_percents[&AbilityStat::MaxHealth],
        percent(5, 6)
    );
    assert_eq!(apply_stat(120, 0, percent(5, 6)), Some(121));
    assert!(derive(&[percent(i64::MAX, 1), percent(1, 1)]).is_none());
    assert!(derive(&[percent(1, 0)]).is_none());
    assert!(derive(&[percent(1, i64::MAX as u64), percent(1, 2)]).is_none());
    let effects = derive(&[percent(-1, 2), percent(1, 2)]).ok_or("zero sum")?;
    assert_eq!(
        effects.stat_percents[&AbilityStat::MaxHealth],
        percent(0, 1)
    );
    Ok(())
}

#[test]
fn equipment_effects_sum_before_clamp_and_application() -> Result<(), Box<dyn std::error::Error>> {
    let effects = derive_equipment_effects(vec![input(
        EquipmentSlot::Head,
        vec![
            ItemAbility::Protection {
                element: ReferenceResistanceKind::Fire,
                percent: percent(80, 1),
                field_only: false,
            },
            ItemAbility::Protection {
                element: ReferenceResistanceKind::Fire,
                percent: percent(80, 1),
                field_only: false,
            },
            ItemAbility::Protection {
                element: ReferenceResistanceKind::Fire,
                percent: percent(-70, 1),
                field_only: false,
            },
            ItemAbility::Protection {
                element: ReferenceResistanceKind::Fire,
                percent: percent(-100, 1),
                field_only: true,
            },
            ItemAbility::StatBoost {
                stat: AbilityStat::MaxMana,
                amount: AbilityAmount::Percent(percent(1, 2)),
            },
            ItemAbility::StatBoost {
                stat: AbilityStat::MaxMana,
                amount: AbilityAmount::Percent(percent(1, 2)),
            },
            ItemAbility::StatBoost {
                stat: AbilityStat::MaxMana,
                amount: AbilityAmount::Flat(7),
            },
            ItemAbility::SkillBoost {
                skill: AbilitySkill::Sword,
                points: 3,
            },
            ItemAbility::SkillBoost {
                skill: AbilitySkill::Sword,
                points: -1,
            },
        ],
    )]);
    let effects = effects.ok_or("valid sums")?;
    assert_eq!(
        effects.protections[&ReferenceResistanceKind::Fire],
        percent(90, 1)
    );
    assert_eq!(
        effects.field_protections[&ReferenceResistanceKind::Fire],
        percent(-100, 1)
    );
    assert_eq!(effects.skills[&AbilitySkill::Sword], 2);
    assert_eq!(
        apply_stat(
            101,
            effects.stat_flats[&AbilityStat::MaxMana],
            effects.stat_percents[&AbilityStat::MaxMana]
        ),
        Some(109)
    );
    Ok(())
}

#[test]
fn equipment_effects_timed_wrong_slot_and_extra_are_inactive_except_extra_light()
-> Result<(), Box<dyn std::error::Error>> {
    let abilities = vec![
        ItemAbility::Speed(20),
        ItemAbility::Suppress(AbilitySuppression::Drown),
        ItemAbility::SkillBoost {
            skill: AbilitySkill::Axe,
            points: 3,
        },
        ItemAbility::StatBoost {
            stat: AbilityStat::MaxHealth,
            amount: AbilityAmount::Flat(10),
        },
        ItemAbility::Protection {
            element: ReferenceResistanceKind::Fire,
            percent: percent(5, 1),
            field_only: false,
        },
        ItemAbility::Light { level: 7, color: 3 },
    ];
    let mut timed = input(EquipmentSlot::Head, abilities.clone());
    timed.profile.timed = true;
    let mut wrong = input(EquipmentSlot::Feet, abilities.clone());
    wrong.in_own_slot = false;
    let extra = input(EquipmentSlot::Ammo, abilities);
    let effects = derive_equipment_effects(vec![timed, wrong, extra]).ok_or("valid slots")?;
    assert_eq!(effects.speed, 0);
    assert!(effects.suppressions.is_empty());
    assert!(effects.skills.is_empty());
    assert!(effects.stat_flats.is_empty());
    assert!(effects.protections.is_empty());
    assert_eq!(effects.light, Some((7, 3)));
    let mut timed_extra = input(
        EquipmentSlot::Ammo,
        vec![ItemAbility::Light { level: 9, color: 1 }],
    );
    timed_extra.profile.timed = true;
    assert_eq!(
        derive_equipment_effects(vec![timed_extra]),
        Some(EquipmentEffects::default())
    );
    assert!(
        derive_equipment_effects(vec![
            input(EquipmentSlot::Head, vec![]),
            input(EquipmentSlot::Head, vec![])
        ])
        .is_none()
    );
    Ok(())
}

#[test]
fn equipment_effects_every_equip_permutation_is_identical() {
    fn items() -> Vec<EquipmentEffectInput<'static>> {
        [
            EquipmentSlot::Head,
            EquipmentSlot::Feet,
            EquipmentSlot::Ammo,
        ]
        .into_iter()
        .enumerate()
        .map(|(i, slot)| {
            input(
                slot,
                vec![
                    ItemAbility::Speed(10),
                    ItemAbility::SkillBoost {
                        skill: AbilitySkill::Shield,
                        points: 2,
                    },
                    ItemAbility::StatBoost {
                        stat: AbilityStat::MaxMana,
                        amount: AbilityAmount::Percent(percent(1, 2)),
                    },
                    ItemAbility::StatBoost {
                        stat: AbilityStat::MaxMana,
                        amount: AbilityAmount::Flat(-3),
                    },
                    ItemAbility::Protection {
                        element: ReferenceResistanceKind::Fire,
                        percent: percent(3, 2),
                        field_only: false,
                    },
                    ItemAbility::Protection {
                        element: ReferenceResistanceKind::Fire,
                        percent: percent(-1, 2),
                        field_only: true,
                    },
                    ItemAbility::Suppress(AbilitySuppression::Drunk),
                    ItemAbility::Light {
                        level: 5,
                        color: i as u8 + 1,
                    },
                ],
            )
        })
        .collect()
    }
    let expected = derive_equipment_effects(items());
    for order in [
        [0, 1, 2],
        [0, 2, 1],
        [1, 0, 2],
        [1, 2, 0],
        [2, 0, 1],
        [2, 1, 0],
    ] {
        let mut ordered = items();
        ordered.sort_by_key(|item| order.iter().position(|&i| items()[i].slot == item.slot));
        for item in &mut ordered {
            item.profile.abilities.reverse();
        }
        assert_eq!(derive_equipment_effects(ordered), expected);
    }
    assert_eq!(expected.map(|effects| effects.light), Some(Some((5, 1))));
}
