use oteryn_gameplay_content::content::item_abilities::{
    AbilitySkill, ItemAbility, item_ability_profile, item_offence_modifiers,
};
use oteryn_gameplay_content::content::native_gameplay::{
    NativeGameplayInput, NativeItemDestination, NativeItemStackClass,
};
use oteryn_gameplay_content::content::{
    ContentActivationController, NativeEntryActivationIssuance, NodeBootQuiescence,
    ProjectV2RangedElement, ProjectV2RangedWeapon, ReferenceAmmoType, ReferenceItemField,
    ReferenceModifierParameter, ReferenceRationalPercent, ReferenceResistanceKind as Resistance,
    ReferenceSignedPoints, ReferenceSkillModifierKind, ReferenceWeaponType,
    activate_native_entry_room_with_gameplay, qualify_selected_native_gameplay_room,
};
use oteryn_gameplay_domain::domain::equipment_policy_abi::{
    EquipmentBaseVocation, EquipmentPolicyLookup,
};
use std::{collections::BTreeMap, path::Path};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[test]
fn crystal_equipment_third_quarter_qualifies_for_move_and_combat() -> TestResult {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(check_crystal_equipment)?
        .join()
        .map_err(|_| "equipment loader test panicked")?
}

fn check_crystal_equipment() -> TestResult {
    #[derive(serde::Deserialize)]
    struct Capture {
        records: Vec<SourceItem>,
    }
    #[derive(serde::Deserialize)]
    struct SourceItem {
        id: u32,
        slot: u8,
        xml: BTreeMap<String, String>,
        script: BTreeMap<String, String>,
        held: Option<String>,
        stackable: bool,
        top_order: u8,
        vocations: Vec<String>,
    }

    let capture: Capture = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../imports/crystalserver/facts/equipment-gap-quarter-3.json"
    )))?;
    let mut input = NativeGameplayInput::from_manifest(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../content/spells.manifest.json"),
    )?;
    input.source_world = None;
    let world = oteryn_protocol_oteryn::WorldId::decode(&[
        1, 144, 0, 0, 0, 1, 112, 0, 128, 0, 0, 0, 0, 0, 0, 1,
    ])?;
    let room = qualify_selected_native_gameplay_room(world, &input)?;
    let issuance = NativeEntryActivationIssuance {
        world_id: world,
        activation_sequence: 1,
        server_artifact_digest: room.compiled().server_digest(),
        client_artifact_digest: room.compiled().client_digest(),
        frame_binding_digest: room.frame_binding().digest(),
    };
    let mut controller = ContentActivationController::new();
    activate_native_entry_room_with_gameplay(
        &mut controller,
        &NodeBootQuiescence::before_channel_runtime(),
        world,
        &issuance,
        &input,
    )
    .map_err(|error| format!("{error:?}"))?;
    let content = controller
        .active()
        .and_then(|active| active.native_gameplay())
        .ok_or("native generation")?;
    let mut checked = 0;
    for source in capture.records {
        let key = format!("oteryn:item.tibia.i{}", source.id);
        let policy = content.equipment_policy(&key, "definition-r1");
        if source.held.is_some() {
            assert!(
                policy.is_none_or(|policy| policy.claims().iter().all(|(_, claim)| claim.is_err())),
                "{key}"
            );
            continue;
        }
        checked += 1;
        let value = |name: &str| -> TestResult<i32> {
            Ok(source.xml.get(name).map_or(Ok(0), |raw| raw.parse())?)
        };
        let policy = policy.ok_or_else(|| format!("{key} missing through the equipment owner"))?;
        let weapon_type = source
            .script
            .get("weapontype")
            .or(source.xml.get("weapontype"));
        let launcher = weapon_type.is_some_and(|weapon| weapon == "distance")
            && source.xml.contains_key("ammotype");
        let two_handed = source
            .xml
            .get("slottype")
            .is_some_and(|slot| slot == "two-handed")
            || source
                .script
                .get("slot")
                .is_some_and(|slot| slot == "two-handed");
        for (slot, claims) in policy.claims() {
            if *slot != source.slot {
                assert!(claims.is_err(), "{key} unexpectedly equips in slot {slot}");
                continue;
            }
            let claims = claims
                .as_ref()
                .map_err(|reason| format!("{key}: {reason}"))?;
            let slots = if two_handed && !launcher {
                vec![5, 6]
            } else {
                vec![source.slot]
            };
            assert_eq!(claims.slots, slots, "{key}");
            assert_eq!(
                claims.groups,
                if launcher || source.slot == 6 {
                    vec!["oteryn:equipment-group.non_quiver_left_hand"]
                } else {
                    vec![]
                },
                "{key}"
            );
            let level = source
                .script
                .get("level")
                .map_or(Ok(0), |raw| raw.parse())?;
            assert_eq!(claims.level, level, "{key}");
            let vocations = source
                .vocations
                .iter()
                .map(|name| {
                    Ok(match name.as_str() {
                        "DRUID" => EquipmentBaseVocation::Druid,
                        "KNIGHT" => EquipmentBaseVocation::Knight,
                        "MONK" => EquipmentBaseVocation::Monk,
                        "PALADIN" => EquipmentBaseVocation::Paladin,
                        "SORCERER" => EquipmentBaseVocation::Sorcerer,
                        _ => return Err(format!("{key} unsupported source vocation {name}")),
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;
            assert_eq!(claims.vocations, vocations, "{key}");
        }
        let record = content
            .item_policy(&key, "definition-r1")
            .ok_or("item policy")?
            .record();
        assert!(record.admission.materializable, "{key}");
        assert_eq!(
            record.attributes.always_on_top,
            Some(source.top_order != 0),
            "{key}"
        );
        assert_eq!(
            record.attributes.always_on_top_order,
            Some(source.top_order),
            "{key}"
        );
        assert_eq!(
            record.admission.legal_destinations,
            [
                NativeItemDestination::CharacterInventory,
                NativeItemDestination::Ground
            ],
            "{key}"
        );
        assert_eq!(
            record.admission.stack_class,
            if source.stackable {
                NativeItemStackClass::StackCapable
            } else {
                NativeItemStackClass::NonStackable
            },
            "{key}"
        );
        assert_eq!(
            record.semantics.container,
            ReferenceItemField::NotApplicable,
            "{key}"
        );
        assert_eq!(
            record.semantics.temporal,
            ReferenceItemField::NotApplicable,
            "{key}"
        );
        let ReferenceItemField::Known(stack) = &record.semantics.stack else {
            return Err(format!("{key} missing stack policy").into());
        };
        assert_eq!(
            stack.stackable,
            ReferenceItemField::Known(source.stackable),
            "{key}"
        );
        assert_eq!(
            stack.stack_max,
            if source.stackable {
                ReferenceItemField::Known(100)
            } else {
                ReferenceItemField::NotApplicable
            },
            "{key}"
        );
        if source.xml.contains_key("imbuementslot") {
            let ReferenceItemField::Known(imbuement) = &record.semantics.imbuement else {
                return Err(format!("{key} missing imbuement slots").into());
            };
            assert_eq!(
                imbuement.slot_count,
                ReferenceItemField::Known(u8::try_from(value("imbuementslot")?)?),
                "{key}"
            );
        }
        let ReferenceItemField::Known(physical) = &record.semantics.physical else {
            return Err(format!("{key} missing physical facts").into());
        };
        assert_eq!(
            physical.weight,
            ReferenceItemField::Known(if source.id == 35520 {
                1200 // Verified Global Cyclopedia.
            } else {
                u32::try_from(value("weight")?)?
            }),
            "{key}"
        );
        assert_eq!(physical.movable, ReferenceItemField::Known(true), "{key}");
        assert_eq!(
            physical.pickupable,
            ReferenceItemField::Known(true),
            "{key}"
        );
        let ReferenceItemField::Known(protection_facts) = &record.semantics.protection else {
            return Err(format!("{key} missing armor").into());
        };
        assert_eq!(
            protection_facts.armor,
            ReferenceItemField::Known(ReferenceSignedPoints(value("armor")?)),
            "{key}"
        );
        let abilities = item_ability_profile(&record.semantics)?;
        let speed = abilities
            .abilities
            .iter()
            .find_map(|ability| {
                if let ItemAbility::Speed(points) = ability {
                    Some(*points)
                } else {
                    None
                }
            })
            .unwrap_or_default();
        assert_eq!(speed, value("speed")?, "{key}");
        assert_eq!(record.attributes.speed_bonus, Some(speed), "{key}");
        let mut protection = BTreeMap::new();
        for (field, element) in [
            ("absorbpercentdeath", Resistance::Death),
            ("absorbpercentearth", Resistance::Earth),
            ("absorbpercentpoison", Resistance::Earth),
            ("absorbpercentenergy", Resistance::Energy),
            ("absorbpercentfire", Resistance::Fire),
            ("absorbpercentholy", Resistance::Holy),
            ("absorbpercentice", Resistance::Ice),
            ("absorbpercentphysical", Resistance::Physical),
        ] {
            if source.xml.contains_key(field) {
                let resistance = ReferenceRationalPercent {
                    numerator: i64::from(value(field)?),
                    denominator: 1,
                };
                assert!(protection.insert(element, resistance).is_none(), "{key}");
            }
        }
        let actual_protection = abilities
            .abilities
            .iter()
            .filter_map(|ability| {
                if let ItemAbility::Protection {
                    element,
                    percent,
                    field_only: false,
                } = ability
                {
                    Some((*element, *percent))
                } else {
                    None
                }
            })
            .collect::<BTreeMap<_, _>>();
        assert_eq!(actual_protection, protection, "{key}");
        for (field, skill) in [
            ("skillaxe", AbilitySkill::Axe),
            ("skillclub", AbilitySkill::Club),
            ("skilldist", AbilitySkill::Distance),
            ("skillshield", AbilitySkill::Shield),
            ("skillsword", AbilitySkill::Sword),
            ("magiclevelpoints", AbilitySkill::MagicLevel),
        ] {
            let points = abilities
                .abilities
                .iter()
                .find_map(|ability| match ability {
                    ItemAbility::SkillBoost {
                        skill: actual,
                        points,
                    } if *actual == skill => Some(*points),
                    _ => None,
                })
                .unwrap_or_default();
            assert_eq!(points, value(field)?, "{key}/{field}");
        }
        let offence = item_offence_modifiers(&record.semantics);
        if source.xml.contains_key("lifeleechchance") {
            let ReferenceItemField::Known(skills) = &record.semantics.skill_modifiers else {
                return Err(format!("{key} missing leech chance").into());
            };
            let ReferenceItemField::Known(modifiers) = &skills.modifiers else {
                return Err(format!("{key} missing modifiers").into());
            };
            let chance = modifiers
                .iter()
                .find(|modifier| modifier.kind == ReferenceSkillModifierKind::LifeLeechChance)
                .ok_or("source leech chance")?;
            assert_eq!(
                chance.parameter,
                ReferenceItemField::Known(ReferenceModifierParameter::RationalPercent(
                    ReferenceRationalPercent {
                        numerator: i64::from(value("lifeleechchance")?),
                        denominator: 1,
                    }
                )),
                "{key}"
            );
        }
        assert_eq!(
            offence.critical_chance,
            i64::from(value("criticalhitchance")?),
            "{key}"
        );
        assert_eq!(
            offence.critical_damage,
            i64::from(value("criticalhitdamage")?),
            "{key}"
        );
        assert_eq!(
            offence.life_leech,
            i64::from(value("lifeleechamount")?),
            "{key}"
        );
        assert_eq!(
            offence.cleave,
            i64::from(value("cleavepercent")?) * 100,
            "{key}"
        );
        assert_eq!(
            offence.reflect_flat,
            i64::from(value("reflectdamage")?),
            "{key}"
        );
        let perfect_shot = if source.xml.contains_key("perfectshotrange") {
            BTreeMap::from([(
                u16::try_from(value("perfectshotrange")?)?,
                i64::from(value("perfectshotdamage")?),
            )])
        } else {
            BTreeMap::new()
        };
        assert_eq!(offence.perfect_shot, perfect_shot, "{key}");
        if let Some(weapon_type) = weapon_type {
            let ReferenceItemField::Known(weapon) = &record.semantics.weapon else {
                return Err(format!("{key} missing combat facts").into());
            };
            assert_eq!(
                weapon.defense,
                ReferenceItemField::Known(ReferenceSignedPoints(value("defense")?)),
                "{key}"
            );
            assert_eq!(
                weapon.extra_defense,
                ReferenceItemField::Known(ReferenceSignedPoints(value("extradef")?)),
                "{key}"
            );
            let kind: ReferenceWeaponType =
                serde_json::from_value(weapon_type.to_ascii_uppercase().into())?;
            assert_eq!(weapon.weapon_type, ReferenceItemField::Known(kind), "{key}");
            assert_eq!(
                weapon.attack,
                ReferenceItemField::Known(ReferenceSignedPoints(value("attack")?)),
                "{key}"
            );
            if launcher {
                // Global supplies the final attack and hit chance for these launchers.
                let (attack, hit_chance) = match source.id {
                    28718 => (6, 5),
                    29417 => (5, 5),
                    _ => (
                        u16::try_from(value("attack")?)?,
                        i8::try_from(value("hitchance")?)?,
                    ),
                };
                assert_eq!(
                    record.authoring.ranged,
                    Some(ProjectV2RangedWeapon::Distance {
                        ammunition: if source.xml["ammotype"] == "arrow" {
                            ReferenceAmmoType::Arrow
                        } else {
                            ReferenceAmmoType::Bolt
                        },
                        attack,
                        range_cells: u8::try_from(value("range")?)?,
                        hit_chance,
                        max_hit_chance: 90,
                    }),
                    "{key}"
                );
            } else if weapon_type == "wand" {
                let Some(ProjectV2RangedWeapon::Wand(shot) | ProjectV2RangedWeapon::Rod(shot)) =
                    &record.authoring.ranged
                else {
                    return Err(format!("{key} missing magic shot").into());
                };
                let element: ProjectV2RangedElement =
                    serde_json::from_value(source.script["wandtype"].to_ascii_uppercase().into())?;
                assert_eq!(shot.element, element, "{key}");
                assert_eq!(
                    shot.mana_per_shot,
                    source.script["mana"].parse::<u16>()?,
                    "{key}"
                );
                assert_eq!(
                    shot.damage_min,
                    source.script["fromdamage"].parse::<u16>()?,
                    "{key}"
                );
                assert_eq!(
                    shot.damage_max,
                    source.script["todamage"].parse::<u16>()?,
                    "{key}"
                );
                assert_eq!(
                    shot.range_cells,
                    source
                        .xml
                        .get("range")
                        .map_or(Ok(1), |raw| raw.parse::<u8>())?,
                    "{key}"
                );
            }
        }
    }
    assert_eq!(checked, 155);
    Ok(())
}
