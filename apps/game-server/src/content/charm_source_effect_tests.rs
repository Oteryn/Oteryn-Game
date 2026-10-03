use super::*;
use crate::combat::charm_effects::{
    CharmCategory, CharmDefinition, CharmDefinitionError, CharmStageValue,
};
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashSet;

type TestResult = Result<(), Box<dyn std::error::Error>>;
#[derive(Deserialize)]
struct Shard {
    records: [Record; 25],
}
#[derive(Deserialize)]
struct Record {
    definition: Definition,
}
#[derive(Deserialize)]
struct Definition {
    effect: Box<RawValue>,
}
fn actual_effects() -> Result<[Record; 25], serde_json::Error> {
    let shard: Shard = serde_json::from_slice(include_bytes!(
        "../../../../content/charms/charms-00000-00024.json"
    ))?;
    Ok(shard.records)
}
fn decode(value: &Value) -> Result<CharmEffect, Box<dyn std::error::Error>> {
    Ok(lower_effect(&RawValue::from_string(
        serde_json::to_string(value)?,
    )?)?)
}
fn ordinary_damage(element: CharmElement) -> CharmDamageKind {
    CharmDamageKind {
        element,
        ignores_resistances: false,
        reduced_by_armor: false,
    }
}
fn attack(element: CharmElement) -> Result<CharmEffect, ContentError> {
    Ok(CharmEffect::AttackProcDamage {
        damage: ordinary_damage(element),
        percent_of_creature_max_health: percent(500)?,
        damage_cap_level_multiplier: 2,
    })
}
fn resource(resource: CharmResource, value: u32) -> Result<CharmEffect, ContentError> {
    Ok(CharmEffect::AttackProcResourceDamage {
        damage: CharmDamageKind {
            ignores_resistances: true,
            ..ordinary_damage(CharmElement::Physical)
        },
        resource,
        percent_of_own_maximum: percent(value)?,
        damage_cap_percent_of_creature_max_health: percent(800)?,
    })
}

#[test]
fn actual_25_source_effects_preserve_all_18_shapes_and_parameters() -> TestResult {
    let expected = [
        CharmEffect::HasteAfterHit {
            duration_ms: 10_000,
        },
        CharmEffect::DeathLossReduction,
        CharmEffect::KillAreaDamage {
            damage: CharmDamageKind {
                reduced_by_armor: true,
                ..ordinary_damage(CharmElement::Physical)
            },
            percent_of_creature_max_health: percent(1500)?,
            damage_cap_level_multiplier: 6,
        },
        CharmEffect::CleanseAfterHit,
        CharmEffect::ParalyseCreatureOnAttack {
            duration_ms: 10_000,
        },
        attack(CharmElement::Death)?,
        attack(CharmElement::Holy)?,
        CharmEffect::DodgeAttack,
        attack(CharmElement::Fire)?,
        CharmEffect::PreventCreatureFlee {
            duration_ms: 30_000,
        },
        attack(CharmElement::Ice)?,
        CharmEffect::CreatureProductBonus,
        CharmEffect::CriticalHitChance,
        CharmEffect::ParalyseCreatureAfterItsAttack {
            duration_ms: 10_000,
        },
        resource(CharmResource::Mana, 250)?,
        resource(CharmResource::Health, 500)?,
        CharmEffect::ReflectDamageTaken {
            damage: CharmDamageKind {
                element: CharmElement::Physical,
                ignores_resistances: true,
                reduced_by_armor: true,
            },
        },
        attack(CharmElement::Earth)?,
        CharmEffect::CriticalExtraDamage,
        CharmEffect::SkinningChanceBonus,
        CharmEffect::LifeLeech,
        CharmEffect::ManaDrainInversion,
        CharmEffect::ManaLeech,
        attack(CharmElement::Physical)?,
        attack(CharmElement::Energy)?,
    ];
    let mut shapes = HashSet::new();
    for (record, expected) in actual_effects()?.into_iter().zip(expected) {
        let actual = lower_effect(&record.definition.effect)?;
        assert_eq!(actual, expected);
        shapes.insert(std::mem::discriminant(&actual));
    }
    assert_eq!(shapes.len(), 18);
    Ok(())
}

#[test]
fn every_real_shape_rejects_missing_wrong_and_unknown_fields() -> TestResult {
    for record in actual_effects()? {
        let source: Value = serde_json::from_str(record.definition.effect.get())?;
        let members = source.as_object().ok_or("effect object")?;
        let mut unknown = source.clone();
        unknown["surprise"] = Value::Bool(true);
        assert!(decode(&unknown).is_err(), "unknown in {source}");
        for key in members.keys() {
            let mut missing = members.clone();
            missing.remove(key);
            assert!(
                decode(&Value::Object(missing)).is_err(),
                "missing {key} in {source}"
            );
            let mut wrong = source.clone();
            wrong[key] = Value::Null;
            assert!(decode(&wrong).is_err(), "wrong {key} in {source}");
        }
    }
    for json in [
        "null",
        "[]",
        "true",
        "1",
        "\"dodge_attack\"",
        "{}",
        "{\"type\":\"unknown\"}",
    ] {
        assert!(
            lower_effect(&RawValue::from_string(json.into())?).is_err(),
            "{json}"
        );
    }
    Ok(())
}

#[test]
fn duplicate_keys_exact_percentages_and_closed_discriminants_fail_without_coercion() -> TestResult {
    for json in [
        r#"{"type":"dodge_attack","type":"dodge_attack"}"#,
        r#"{"type":"dodge_attack","t\u0079pe":"dodge_attack"}"#,
        r#"{"type":"haste_after_hit","duration_ms":1,"duration_ms":1}"#,
        r#"{"type":"haste_after_hit","duration_ms":1,"duration_\u006ds":1}"#,
        r#"{"type":"haste_after_hit","duration_ms":"1"}"#,
        r#"{"type":"haste_after_hit","duration_ms":4294967296}"#,
    ] {
        assert!(
            lower_effect(&RawValue::from_string(json.into())?).is_err(),
            "{json}"
        );
    }
    let resource_json = actual_effects()?[14].definition.effect.get().to_owned();
    for token in [
        "2.50000000000000001",
        "2.501",
        "25e-1",
        "-2.5",
        "42949672.96",
        "0",
        "1000.01",
        "\"2.5\"",
        "{}",
    ] {
        let changed = resource_json.replace(
            "\"percent_of_own_maximum\":2.5",
            &format!("\"percent_of_own_maximum\":{token}"),
        );
        assert_ne!(changed, resource_json);
        assert!(
            lower_effect(&RawValue::from_string(changed)?).is_err(),
            "{token}"
        );
    }
    let duplicate_percent = resource_json.replace(
        "\"percent_of_own_maximum\":2.5",
        "\"percent_of_own_maximum\":2.5,\"percent_of_own_maxim\\u0075m\":2.5",
    );
    assert_ne!(duplicate_percent, resource_json);
    assert!(lower_effect(&RawValue::from_string(duplicate_percent)?).is_err());
    let resource_source: Value = serde_json::from_str(&resource_json)?;
    for (key, value) in [
        ("element", "unknown"),
        ("resource", "stamina"),
        ("ignores_resistances", "true"),
        ("reduced_by_armor", "false"),
    ] {
        let mut changed = resource_source.clone();
        changed[key] = Value::String(value.into());
        assert!(decode(&changed).is_err(), "{key}");
    }
    let escaped = resource_json.replace(
        "\"percent_of_own_maximum\"",
        "\"percent_of_own_maxim\\u0075m\"",
    );
    assert_eq!(
        lower_effect(&RawValue::from_string(escaped)?)?,
        resource(CharmResource::Mana, 250)?
    );
    Ok(())
}

#[test]
fn semantic_duration_and_cap_validation_remains_with_combat_definition_owner() -> TestResult {
    let stages = [percent(100)?, percent(200)?, percent(300)?];
    for json in [
        r#"{"type":"haste_after_hit","duration_ms":0}"#,
        r#"{"type":"attack_proc_damage","element":"physical","ignores_resistances":false,"reduced_by_armor":false,"percent_of_creature_max_health":5,"damage_cap_level_multiplier":0}"#,
    ] {
        let effect = lower_effect(&RawValue::from_string(json.into())?)?;
        assert_eq!(
            CharmDefinition::new(
                "oteryn:charm.test",
                CharmCategory::Major,
                CharmStageValue::TriggerChancePercent,
                stages,
                effect
            ),
            Err(CharmDefinitionError::InvalidParameter)
        );
    }
    // Flags are faithfully decoded. The source adapter adds no stricter gameplay policy.
    let flags = r#"{"type":"reflect_damage_taken","element":"physical","ignores_resistances":false,"reduced_by_armor":false}"#;
    assert_eq!(
        lower_effect(&RawValue::from_string(flags.into())?)?,
        CharmEffect::ReflectDamageTaken {
            damage: ordinary_damage(CharmElement::Physical)
        }
    );
    Ok(())
}
