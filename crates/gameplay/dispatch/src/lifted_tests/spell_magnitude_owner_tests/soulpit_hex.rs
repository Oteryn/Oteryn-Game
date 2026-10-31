use super::*;
use crate::foundation::{
    ActorConditionTransition, ApplicationFacts, ConditionDefinition, ConditionSource,
    ConditionSourceKind, ConditionValues,
};
use oteryn_simulation_determinism::{
    DecisionOccurrenceId, GameplayDecisionRoot, SemanticTimeMicros,
};

fn debug(value: impl std::fmt::Debug) -> String {
    format!("{value:?}")
}

#[test]
fn soulpit_canonical_hex_composes_once_reduces_healing_and_stays_frozen_for_a_planted_strike()
-> Result<(), String> {
    let mut f = fixture();
    f.state.conditions = Default::default();
    f.attributes.base_critical_chance_permyriad = Some(0);
    f.attributes.equipment_critical_chance_permyriad = Some(0);
    f.attributes.fatal_chance_permyriad = Some(ExactRatio {
        numerator: 0,
        denominator: 1,
    });
    f.attributes.wheel_flat_damage = Some(0);
    f.attributes.wheel_flat_healing = Some(0);
    let root = GameplayDecisionRoot::from_bytes([0x72; 32]);
    let facts = ApplicationFacts {
        now: 100_000,
        base_speed: 220,
        mana_shield_capacity: 0,
        target_reentry_protected: false,
        source_reentry_protected: false,
        target_is_player: true,
        decision_root: &root,
        occurrence: DecisionOccurrenceId::from_bytes([0x73; 16]),
    };
    let attributes = ConditionDefinition::new(
        "fixture:71-percent",
        1,
        ConditionValues::Attributes {
            duration_ms: 10_000,
            critical_chance_percent: 0,
            critical_extra_percentage_points: 0,
            damage_dealt_percent: 71,
            incoming_reduction_percent: 0,
        },
    )
    .ok_or("attributes")?;
    let hex = ConditionDefinition::new(
        "oteryn:condition.soulpit.intense_hex",
        201189,
        ConditionValues::IntenseHex {
            duration_ms: 3000,
            damage_dealt_percent: 50,
            healing_received_percent: 50,
        },
    )
    .ok_or("hex")?;
    let condition = f
        .runtime
        .prepare_actor_condition(
            f.caster,
            Some(f.session),
            ActorConditionTransition::Apply {
                definition: &hex,
                source: ConditionSource {
                    actor: f.bindings[1].actor,
                    session: None,
                    kind: ConditionSourceKind::Creature,
                },
                immunities: &[],
                facts,
            },
            SemanticTimeMicros::from_micros(100_000),
        )
        .map_err(debug)?;
    assert!(
        f.runtime
            .commit_actor_condition(&condition, SemanticTimeMicros::from_micros(100_000))
            .map_err(debug)?
    );
    assert_eq!(
        f.runtime
            .soulpit_hex_percent(f.caster, 100_000)
            .map_err(debug)?,
        (50, 50)
    );
    f.state
        .conditions
        .apply_with_external_damage_buff(
            &attributes,
            None,
            ConditionSourceKind::Creature,
            &[],
            &facts,
            f.runtime
                .soulpit_hex_delta(f.caster, 100_000)
                .map_err(debug)?
                .damage_dealt,
        )
        .map_err(debug)?;
    assert_eq!(
        f.state.conditions.intense_hex_damage_percent_at(100_000),
        100,
        "canonical owner need not be mirrored to affect combat"
    );
    let mut p = plan();
    let NativeCombatPlan::Combat(combat) = &mut p else {
        return Err("combat".into());
    };
    combat.hits.truncate(1);
    combat.hits[0].magnitude = 1000;
    combat.hits[0].side_percent = None;
    combat.hits[0].bonus_percent = 0;
    combat.block_armor = false;
    let hit = combat.hits[0].clone();
    let mut current = qualify(&f, &p).map_err(debug)?;
    // Hex first saves-50; Attributes71 then saves trunc(50*-0.29)=-14, retaining36.
    // Existing source order then gives1000*36%=360,5%mitigation=342,90%sensitivity=308.
    assert_eq!(
        current
            .finish(&p, &hit, &f.bindings[1], &mut |_, high| high)
            .map_err(debug)?,
        308
    );
    let frozen = StrikeCastModifiers::capture(
        &f.attributes,
        &f.state,
        "fixture:delayed",
        100,
        f.runtime
            .soulpit_hex_delta(f.caster, 100_000)
            .map_err(debug)?,
    )
    .map_err(debug)?;
    let mut healing = p.clone();
    let NativeCombatPlan::Combat(heal) = &mut healing else {
        return Err("healing".into());
    };
    heal.element = Element::Healing;
    heal.hits[0].target = 1;
    heal.hits[0].magnitude = 200;
    let heal_hit = heal.hits[0].clone();
    assert_eq!(
        qualify(&f, &healing)
            .map_err(debug)?
            .finish(&healing, &heal_hit, &f.bindings[0], &mut |_, high| high)
            .map_err(debug)?,
        100
    );
    let expiry = f
        .runtime
        .prepare_actor_condition_expiry(
            f.caster,
            Some(f.session),
            DecisionOccurrenceId::from_bytes([0x74; 16]),
            SemanticTimeMicros::from_micros(3_100_000),
        )
        .map_err(debug)?;
    f.runtime
        .commit_actor_condition(&expiry, SemanticTimeMicros::from_micros(3_100_000))
        .map_err(debug)?;
    assert!(
        current
            .validate_current(&f.runtime, &f.state, &f.attributes, &[])
            .is_err(),
        "a prepared owner cannot ignore changed canonical conditions"
    );
    assert_eq!(
        f.runtime
            .soulpit_hex_percent(f.caster, 3_100_000)
            .map_err(debug)?,
        (100, 100)
    );
    let qualify_later = || {
        PreparedMagnitudeOwner::qualify(
            &f.runtime,
            &f.state,
            f.caster,
            f.session,
            Some(&f.attributes),
            &p,
            &f.bindings,
            &[],
            3100,
        )
    };
    let mut fresh = qualify_later().map_err(debug)?;
    assert_eq!(
        fresh
            .finish(&p, &hit, &f.bindings[1], &mut |_, high| high)
            .map_err(debug)?,
        735,
        "Hex expiry leaves Attributes' saved-14, hence86% damage"
    );
    let mut planted = qualify_later().map_err(debug)?;
    planted.select_planted_strike(&frozen);
    assert_eq!(
        planted
            .finish(&p, &hit, &f.bindings[1], &mut |_, high| high)
            .map_err(debug)?,
        308,
        "already planted source strike retains its cast-time Hex buff"
    );
    Ok(())
}

#[test]
fn approved_single_hex_stage_against_soulpit_oppressor_reaches_actual_health_once()
-> Result<(), String> {
    let mut f = fixture();
    f.state.conditions = Default::default();
    f.attributes.base_critical_chance_permyriad = Some(0);
    f.attributes.equipment_critical_chance_permyriad = Some(0);
    f.attributes.fatal_chance_permyriad = Some(ExactRatio {
        numerator: 0,
        denominator: 1,
    });
    f.attributes.wheel_flat_damage = Some(0);
    let target = f.bindings[1].actor;
    f.runtime
        .project_soulpit_oppressor_for_test(target)
        .map_err(debug)?;
    assert!(f.runtime.soulpit_variant(target).is_some_and(|tag| {
        tag.stack == crate::domain::soulpit::SoulpitStack::Boss
            && tag
                .boss
                .is_some_and(|boss| boss.ability == crate::domain::soulpit::BossAbility::Oppressor)
    }));
    let before = f.runtime.companion_snapshot(target).map_err(debug)?.health;
    let root = GameplayDecisionRoot::from_bytes([0x78; 32]);
    let facts = ApplicationFacts {
        now: 100_000,
        base_speed: 220,
        mana_shield_capacity: 0,
        target_reentry_protected: false,
        source_reentry_protected: false,
        target_is_player: true,
        decision_root: &root,
        occurrence: DecisionOccurrenceId::from_bytes([0x79; 16]),
    };
    let hex = ConditionDefinition::new(
        "oteryn:condition.soulpit.intense_hex",
        201189,
        ConditionValues::IntenseHex {
            duration_ms: 3000,
            damage_dealt_percent: 50,
            healing_received_percent: 50,
        },
    )
    .ok_or("Hex definition")?;
    let condition = f
        .runtime
        .prepare_actor_condition(
            f.caster,
            Some(f.session),
            ActorConditionTransition::Apply {
                definition: &hex,
                source: ConditionSource {
                    actor: target,
                    session: None,
                    kind: ConditionSourceKind::Creature,
                },
                immunities: &[],
                facts,
            },
            SemanticTimeMicros::from_micros(100_000),
        )
        .map_err(debug)?;
    f.runtime
        .commit_actor_condition(&condition, SemanticTimeMicros::from_micros(100_000))
        .map_err(debug)?;
    let mut p = plan();
    let NativeCombatPlan::Combat(combat) = &mut p else {
        return Err("combat".into());
    };
    combat.hits.truncate(1);
    combat.hits[0].magnitude = 100;
    combat.hits[0].side_percent = None;
    combat.hits[0].bonus_percent = 0;
    combat.block_armor = false;
    let hit = combat.hits[0].clone();
    let mut magnitude = qualify(&f, &p).map_err(debug)?;
    let amount = magnitude
        .finish(&p, &hit, &f.bindings[1], &mut |_, high| high)
        .map_err(debug)?;
    // ANSWER6543595950: one composed50% stage. The actual owner's later5%
    // mitigation truncates50→47, then90% sensitivity rounds47→42. Crystal's
    // event-dependent second50% application is an explicitly approved deviation.
    assert_eq!(amount, 42);
    magnitude
        .validate_current(&f.runtime, &f.state, &f.attributes, &[])
        .map_err(debug)?;
    f.runtime
        .borrow_exact_actor_commit()
        .commit_damage(
            target,
            crate::foundation::OwnerDamageCommand {
                target: b"test:creature",
                occurrence: b"soulpit-single-hex",
                binding: b"soulpit-single-hex\0approved-stage",
                damage: amount,
            },
        )
        .map_err(debug)?;
    assert_eq!(
        before - f.runtime.companion_snapshot(target).map_err(debug)?.health,
        42
    );
    Ok(())
}
