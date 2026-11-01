use super::*;
use crate::spell::{Vocation, cast::CharacterCastFacts};
use oteryn_gameplay_domain::domain::condition::ConflictKey;
use oteryn_gameplay_domain::domain::condition::attributes::{
    AttributeModifier, AttributeModifiers,
};
use oteryn_gameplay_foundation::foundation::condition::{ConditionStore, TickFacts};

/// No runtime-store instance.
const NO_RUNTIME: &[ConditionInstance<String>] = &[];
const DIGEST: [u8; 32] = [3; 32];

const FACTS: CharacterCastFacts = CharacterCastFacts {
    vocation: Vocation::Knight,
    level: 20,
    magic_level: 0,
    max_health: 365,
    max_mana: 150,
    max_soul: 100,
};
const SECOND: u64 = 1_000_000;

fn apply(state: &mut PlayerSpellState, definition: ConditionDefinition, now: u64) -> Option<()> {
    let root = GameplayDecisionRoot::from_bytes([7; 32]);
    let facts = ApplicationFacts {
        now,
        base_speed: u16::try_from(state.base_speed).ok()?,
        mana_shield_capacity: 300,
        target_reentry_protected: false,
        source_reentry_protected: false,
        target_is_player: true,
        decision_root: &root,
        occurrence: DecisionOccurrenceId::from_bytes([3; 16]),
    };
    state
        .conditions
        .apply(
            &definition,
            None,
            ConditionSourceKind::Creature,
            &[],
            &facts,
        )
        .ok()
        .map(|_| ())
}

fn definition(key: &str, values: ConditionValues) -> Option<ConditionDefinition> {
    ConditionDefinition::new(key, 2, values)
}

/// C08 (Crystal 201189f8 `iologindata_save_player.cpp:264`, `condition.cpp:356`): a finite
/// condition survives logout with its remaining time and a fresh login re-applies it; an
/// infinite one and food (restored by BED-REGEN) are not saved; a mana shield is saved with its
/// cast capacity and restored full (`condition.cpp:1496`, `startCondition`).
#[test]
fn c08_persistent_conditions_survive_logout_with_their_remaining_time() -> Result<(), &'static str>
{
    let mut live = PlayerSpellState::new(FACTS, 0, 0).ok_or("actor")?;
    let drunk = ConditionValues::TimedStatus {
        kind: StatusKind::Drunk,
        duration_ms: 10_000,
    };
    apply(&mut live, definition("drunk", drunk).ok_or("drunk")?, 0).ok_or("apply drunk")?;
    let poison = ConditionValues::DamageOverTime {
        element: DotElement::Poison,
        total_min: 100,
        total_max: 100,
        per_tick: 10,
        interval_ms: 2_000,
        delayed: true,
    };
    apply(&mut live, definition("poison", poison).ok_or("poison")?, 0).ok_or("apply poison")?;
    let haste = ConditionValues::Speed {
        paralysis: false,
        range: SpeedRange {
            a_min: 1000,
            b_min: 90,
            a_max: 1000,
            b_max: 190,
        },
        duration_ms: 30_000,
    };
    apply(&mut live, definition("haste", haste).ok_or("haste")?, 0).ok_or("apply haste")?;
    let shield = ConditionValues::ManaShield {
        duration_ms: 30_000,
    };
    apply(&mut live, definition("shield", shield).ok_or("shield")?, 0).ok_or("apply shield")?;
    let swim = ConditionDefinition::new_permanent_outfit("swim", 1, 267).ok_or("swim")?;
    apply(&mut live, swim, 0).ok_or("apply swim")?;
    let live = live.after_food(31, 1000).ok_or("eat")?;
    let delta = live
        .conditions
        .get(ConflictKey::Speed)
        .ok_or("haste live")?
        .speed_delta();
    assert!((50..=150).contains(&delta), "{delta}");

    let saved = live
        .saved_conditions_snapshot(NO_RUNTIME, 4 * SECOND)
        .ok_or("snapshot")?;
    let mut fresh = PlayerSpellState::new(FACTS, 0, 0).ok_or("actor")?;
    let login = 100 * SECOND;
    assert_eq!(
        fresh
            .restore_saved_conditions(&saved, login, DIGEST)
            .map(|r| r.applied),
        Some(4)
    );
    let restored = |key| fresh.conditions.get(key);
    let drunk = restored(ConflictKey::Status(StatusKind::Drunk)).ok_or("drunk restored")?;
    assert_eq!(drunk.remaining_duration_ms(login), Some(6_000));
    assert_eq!(drunk.definition().key(), "drunk");
    assert_eq!(drunk.definition().revision(), 2);
    let poison = restored(ConflictKey::Element(DotElement::Poison)).ok_or("poison")?;
    assert_eq!(poison.remaining_total(), 100);
    let haste = restored(ConflictKey::Speed).ok_or("haste restored")?;
    assert_eq!(haste.speed_delta(), delta);
    assert_eq!(haste.remaining_duration_ms(login), Some(26_000));
    let shield = restored(ConflictKey::ManaShield).ok_or("shield restored")?;
    assert_eq!(shield.mana_shield(), (300, 300));
    assert_eq!(shield.remaining_duration_ms(login), Some(26_000));
    assert!(restored(ConflictKey::Outfit).is_none());
    assert!(restored(ConflictKey::FoodRegeneration).is_none());
    Ok(())
}

/// The stored text is a durable format: pin it, and refuse an expired or empty snapshot.
#[test]
fn c08_saved_condition_text_is_pinned_and_expired_instances_are_not_saved()
-> Result<(), &'static str> {
    let mut live = PlayerSpellState::new(FACTS, 0, 0).ok_or("actor")?;
    let muted = ConditionValues::gate("muted", 5_000).ok_or("gate")?;
    apply(&mut live, definition("muted", muted).ok_or("muted")?, 0).ok_or("apply")?;
    let saved = live
        .saved_conditions_snapshot(NO_RUNTIME, SECOND)
        .ok_or("snapshot")?;
    assert_eq!(
        saved.as_str(),
        r#"[{"key":"muted","revision":2,"values":{"type":"gate","gate":"muted","duration_ms":4000}}]"#
    );
    assert_eq!(live.saved_conditions_snapshot(NO_RUNTIME, 5 * SECOND), None);
    assert_eq!(
        PlayerSpellState::new(FACTS, 0, 0)
            .ok_or("actor")?
            .saved_conditions_snapshot(NO_RUNTIME, 0),
        None
    );
    Ok(())
}

/// An entry no definition admits any more is dropped; the others still apply.
#[test]
fn c08_an_inadmissible_saved_entry_is_dropped() -> Result<(), &'static str> {
    let saved = SavedConditions::new(
        r#"[{"key":"BAD KEY","revision":1,"values":{"type":"invisible","duration_ms":10}},
            {"key":"invisible","revision":1,"values":{"type":"invisible","duration_ms":10}}]"#
            .to_owned(),
    )
    .ok_or("text")?;
    let mut fresh = PlayerSpellState::new(FACTS, 0, 0).ok_or("actor")?;
    assert_eq!(
        fresh
            .restore_saved_conditions(&saved, 0, DIGEST)
            .map(|r| r.applied),
        Some(1)
    );
    let unknown =
        SavedConditions::new(r#"[{"key":"x","revision":1,"values":{"type":"z"}}]"#.to_owned())
            .ok_or("json array")?;
    assert_eq!(fresh.restore_saved_conditions(&unknown, 0, DIGEST), None);
    Ok(())
}

const NO_FIELD: TickFacts = TickFacts {
    in_protection_zone: false,
    standing_on_field: None,
    standing_on_agony_field: false,
};

/// C08 (`ConditionDamage::serialize`, `condition.cpp:1680`): a damage sequence, an agony ramp
/// and a damage schedule resume with what they have still to deal, and a restored damage has no
/// attacker: it carries the saved creature provenance (Crystal's load skips the owner,
/// `condition.cpp:1652`), which the player periodic owners deal.
#[test]
fn c08_damage_families_resume_where_they_stopped() -> Result<(), &'static str> {
    let mut live = PlayerSpellState::new(FACTS, 0, 0).ok_or("actor")?;
    let mut steps = [DotSequenceStep::default(); MAX_DOT_SEQUENCE_STEPS];
    steps[0] = DotSequenceStep {
        amount: 10,
        interval_ms: 2_000,
        repetitions: 3,
    };
    steps[1] = DotSequenceStep {
        amount: 5,
        interval_ms: 2_000,
        repetitions: 2,
    };
    let sequence = ConditionValues::DamageSequence {
        element: DotElement::Energy,
        steps,
        len: 2,
        delayed: true,
    };
    apply(&mut live, definition("sequence", sequence).ok_or("seq")?, 0).ok_or("apply seq")?;
    let ramp = ConditionValues::AgonyRamp {
        start: 5,
        interval_ms: 1_000,
        delayed: false,
    };
    apply(&mut live, definition("ramp", ramp).ok_or("ramp")?, 0).ok_or("apply ramp")?;
    let schedule = ConditionDefinition::new_damage_schedule(
        "burn",
        2,
        DotElement::Fire,
        DamageSchedule::Fixed {
            delayed: true,
            segments: vec![
                DamageSegment {
                    count: 2,
                    amount: 7,
                    interval_ms: 1_000,
                },
                DamageSegment {
                    count: 1,
                    amount: 3,
                    interval_ms: 1_000,
                },
            ],
        },
    )
    .ok_or("schedule")?;
    apply(&mut live, schedule, 0).ok_or("apply schedule")?;
    let mut dealt = Vec::new();
    for second in [0, 1, 2, 3, 4] {
        for tick in live.conditions.take_due(second * SECOND, NO_FIELD) {
            dealt.push((second, tick.provenance.definition_key));
        }
    }
    // The ramp at 0 to 4 (5, 4, 3, 2, 1: it ends), the schedule at 1, 2 and 3 (it ends), the
    // sequence at 2 and 4.
    assert_eq!(dealt.len(), 10, "{dealt:?}");
    let saved = live
        .saved_conditions_snapshot(NO_RUNTIME, 4 * SECOND + 1)
        .ok_or("snapshot")?;
    let mut fresh = PlayerSpellState::new(FACTS, 0, 0).ok_or("actor")?;
    let login = 50 * SECOND;
    assert_eq!(
        fresh
            .restore_saved_conditions(&saved, login, DIGEST)
            .map(|r| r.applied),
        Some(1)
    );
    // 10 + 5 + 5 left of the sequence, with the saved creature provenance.
    let sequence = fresh
        .conditions
        .get(ConflictKey::Element(DotElement::Energy))
        .ok_or("sequence restored")?;
    assert_eq!(sequence.remaining_total(), 20);
    assert_eq!(
        sequence.provenance().source.as_deref(),
        Some(SAVED_DAMAGE_SOURCE)
    );
    assert_eq!(
        sequence.provenance().source_kind,
        ConditionSourceKind::Creature
    );
    let mut amounts = Vec::new();
    for second in [52, 54, 56] {
        for tick in fresh.conditions.take_due(second * SECOND, NO_FIELD) {
            amounts.push(tick.kind);
        }
    }
    assert_eq!(amounts.len(), 3, "{amounts:?}");
    assert!(fresh.conditions.instances().is_empty());

    // A ramp and a schedule saved part way: 2 + 1 of the ramp after 5, 4, 3; the last 3 of the
    // schedule after 7, 7.
    let mut live = PlayerSpellState::new(FACTS, 0, 0).ok_or("actor")?;
    apply(&mut live, definition("ramp", ramp).ok_or("ramp")?, 0).ok_or("apply ramp")?;
    let schedule = ConditionDefinition::new_damage_schedule(
        "burn",
        2,
        DotElement::Fire,
        DamageSchedule::Fixed {
            delayed: true,
            segments: vec![
                DamageSegment {
                    count: 2,
                    amount: 7,
                    interval_ms: 1_000,
                },
                DamageSegment {
                    count: 1,
                    amount: 3,
                    interval_ms: 1_000,
                },
            ],
        },
    )
    .ok_or("schedule")?;
    apply(&mut live, schedule, 0).ok_or("apply schedule")?;
    for second in [0, 1, 2] {
        let _ = live.conditions.take_due(second * SECOND, NO_FIELD);
    }
    let saved = live
        .saved_conditions_snapshot(NO_RUNTIME, 2 * SECOND + 1)
        .ok_or("snapshot")?;
    let mut fresh = PlayerSpellState::new(FACTS, 0, 0).ok_or("actor")?;
    assert_eq!(
        fresh
            .restore_saved_conditions(&saved, login, DIGEST)
            .map(|r| r.applied),
        Some(2)
    );
    let ramp = fresh.conditions.get(ConflictKey::Agony).ok_or("ramp")?;
    assert_eq!(ramp.remaining_total(), 3);
    let schedule = fresh
        .conditions
        .get(ConflictKey::Element(DotElement::Fire))
        .ok_or("schedule")?;
    assert_eq!(schedule.remaining_total(), 3);
    assert_eq!(
        schedule.remaining_damage_segments(),
        Some(vec![DamageSegment {
            count: 1,
            amount: 3,
            interval_ms: 1_000,
        }])
    );
    Ok(())
}

/// C08 (`ConditionAttributes::serialize`, `condition.cpp:645`; `ConditionOutfit::serialize`,
/// `condition.cpp:2553`): source attributes, an outfit with its appearance selection and an item
/// outfit survive; an item outfit of another server artifact does not.
#[test]
fn c08_source_attributes_and_appearance_outfits_survive() -> Result<(), &'static str> {
    let mut live = PlayerSpellState::new(FACTS, 0, 0).ok_or("actor")?;
    let modifiers = AttributeModifiers {
        magic_points: Some(AttributeModifier::PercentOfBase(90)),
        melee: Some(AttributeModifier::Add(-3)),
        distance: None,
    };
    let attributes = ConditionValues::SourceAttributes {
        modifiers,
        duration_ms: 10_000,
    };
    apply(
        &mut live,
        definition("weaken", attributes).ok_or("attr")?,
        0,
    )
    .ok_or("apply attr")?;
    let chameleon = definition(
        "chameleon",
        ConditionValues::ItemOutfit {
            duration_ms: 10_000,
        },
    )
    .ok_or("chameleon")?
    .with_item_appearance("oteryn:item.tibia.i3264", "source-r1", DIGEST)
    .ok_or("item appearance")?;
    apply(&mut live, chameleon, 0).ok_or("apply chameleon")?;
    let saved = live
        .saved_conditions_snapshot(NO_RUNTIME, SECOND)
        .ok_or("snapshot")?;
    let mut fresh = PlayerSpellState::new(FACTS, 0, 0).ok_or("actor")?;
    assert_eq!(
        fresh
            .restore_saved_conditions(&saved, 0, DIGEST)
            .map(|r| r.applied),
        Some(2)
    );
    let restored = fresh
        .conditions
        .get(ConflictKey::Attributes)
        .ok_or("attr")?;
    assert_eq!(
        restored.definition().values(),
        ConditionValues::SourceAttributes {
            modifiers,
            duration_ms: 9_000,
        }
    );
    let outfit = fresh.conditions.get(ConflictKey::Outfit).ok_or("outfit")?;
    assert_eq!(
        outfit
            .definition()
            .source_item_appearance()
            .map(|item| (item.definition_key(), item.artifact_digest())),
        Some(("oteryn:item.tibia.i3264", DIGEST))
    );
    let mut other = PlayerSpellState::new(FACTS, 0, 0).ok_or("actor")?;
    assert_eq!(
        other
            .restore_saved_conditions(&saved, 0, [4; 32])
            .map(|r| r.applied),
        Some(1)
    );
    assert!(other.conditions.get(ConflictKey::Outfit).is_none());

    let selection = AppearanceSelection {
        outfit_key: "oteryn:outfit.citizen".to_owned(),
        colours: [1, 2, 3, 4],
        addons: 1,
        mount_key: None,
    };
    let costume = definition(
        "costume",
        ConditionValues::Outfit {
            duration_ms: 10_000,
            look_type: 128,
        },
    )
    .ok_or("costume")?
    .with_appearance(selection.clone())
    .ok_or("appearance")?;
    let mut live = PlayerSpellState::new(FACTS, 0, 0).ok_or("actor")?;
    apply(&mut live, costume, 0).ok_or("apply costume")?;
    let saved = live
        .saved_conditions_snapshot(NO_RUNTIME, SECOND)
        .ok_or("snapshot")?;
    let mut fresh = PlayerSpellState::new(FACTS, 0, 0).ok_or("actor")?;
    assert_eq!(
        fresh
            .restore_saved_conditions(&saved, 0, DIGEST)
            .map(|r| r.applied),
        Some(1)
    );
    let outfit = fresh.conditions.get(ConflictKey::Outfit).ok_or("costume")?;
    assert_eq!(
        outfit.definition().source_appearance_selection(),
        Some(&selection)
    );
    Ok(())
}

/// C08 (`spells.cpp:789`, `800`: cooldowns are `CONDITIONID_DEFAULT` conditions with finite
/// ticks, so `isPersistent()` saves them): a running spell cooldown is armed again for its
/// remaining time; an expired one is not saved.
#[test]
fn c08_spell_cooldowns_survive_with_their_remaining_time() -> Result<(), &'static str> {
    let mut live = PlayerSpellState::new(FACTS, 0, 0).ok_or("actor")?;
    live.cooldowns.spells.insert(
        "exura".to_owned(),
        SemanticTimeMicros::from_micros(10 * SECOND),
    );
    live.cooldowns.groups.insert(
        "healing".to_owned(),
        SemanticTimeMicros::from_micros(3 * SECOND),
    );
    let saved = live
        .saved_conditions_snapshot(NO_RUNTIME, 4 * SECOND)
        .ok_or("snapshot")?;
    assert_eq!(
        saved.as_str(),
        r#"[{"key":"exura","revision":0,"values":{"type":"spell_cooldown","remaining_us":6000000}}]"#
    );
    let mut fresh = PlayerSpellState::new(FACTS, 0, 0).ok_or("actor")?;
    assert_eq!(
        fresh
            .restore_saved_conditions(&saved, 100 * SECOND, DIGEST)
            .map(|r| r.applied),
        Some(1)
    );
    assert_eq!(
        fresh.cooldowns.spells.get("exura"),
        Some(&SemanticTimeMicros::from_micros(106 * SECOND))
    );
    assert!(fresh.cooldowns.groups.is_empty());
    Ok(())
}

/// C08: the runtime actor's own store is saved too, and handed back to its owner at restore.
#[test]
fn c08_runtime_store_conditions_are_handed_back() -> Result<(), &'static str> {
    let root = GameplayDecisionRoot::from_bytes([7; 32]);
    let facts = ApplicationFacts {
        now: 0,
        base_speed: 220,
        mana_shield_capacity: 0,
        target_reentry_protected: false,
        source_reentry_protected: false,
        target_is_player: true,
        decision_root: &root,
        occurrence: DecisionOccurrenceId::from_bytes([3; 16]),
    };
    let mut runtime = ConditionStore::<u8>::new();
    let feared = ConditionValues::TimedStatus {
        kind: StatusKind::Feared,
        duration_ms: 8_000,
    };
    runtime
        .apply(
            &definition("feared", feared).ok_or("feared")?,
            Some(1),
            ConditionSourceKind::SelfUse,
            &[],
            &facts,
        )
        .map_err(|_| "apply feared")?;
    let live = PlayerSpellState::new(FACTS, 0, 0).ok_or("actor")?;
    let saved = live
        .saved_conditions_snapshot(runtime.instances(), 2 * SECOND)
        .ok_or("snapshot")?;
    assert_eq!(
        saved.as_str(),
        r#"[{"key":"feared","revision":2,"values":{"type":"timed_status","kind":"feared","duration_ms":6000},"runtime":true}]"#
    );
    let mut fresh = PlayerSpellState::new(FACTS, 0, 0).ok_or("actor")?;
    let restored = fresh
        .restore_saved_conditions(&saved, 0, DIGEST)
        .ok_or("restore")?;
    assert_eq!(restored.applied, 0);
    assert_eq!(restored.runtime.len(), 1);
    assert_eq!(restored.runtime[0].key(), "feared");
    assert!(fresh.conditions.instances().is_empty());
    Ok(())
}

/// The 0691 bounds hold on PostgreSQL's `jsonb` text (a space after each `:` and `,`): an entry
/// that would exceed them is left out, the rest kept.
#[test]
fn c08_snapshot_keeps_within_the_stored_bounds() -> Result<(), &'static str> {
    let sample = r#"{"a":1,"b":"x:,\"y"}"#;
    assert_eq!(stored_len(sample), sample.len() + 3);
    let mut live = PlayerSpellState::new(FACTS, 0, 0).ok_or("actor")?;
    for index in 0..80 {
        live.cooldowns.spells.insert(
            format!("{index:03}{}", "s".repeat(300)),
            SemanticTimeMicros::from_micros(10 * SECOND),
        );
    }
    let saved = live
        .saved_conditions_snapshot(NO_RUNTIME, 0)
        .ok_or("snapshot")?;
    let entries =
        serde_json::from_str::<Vec<serde_json::Value>>(saved.as_str()).map_err(|_| "json")?;
    assert!(stored_len(saved.as_str()) <= MAX_SAVED_CONDITIONS_BYTES);
    assert!(
        (1..MAX_SAVED_CONDITIONS).contains(&entries.len()),
        "{}",
        entries.len()
    );
    Ok(())
}
