use super::*;
use crate::spell::{Vocation, cast::CharacterCastFacts};
use oteryn_gameplay_domain::domain::condition::ConflictKey;

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
/// infinite one, food (restored by BED-REGEN) and a mana shield are not saved.
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
        .saved_conditions_snapshot(4 * SECOND)
        .ok_or("snapshot")?;
    let mut fresh = PlayerSpellState::new(FACTS, 0, 0).ok_or("actor")?;
    let login = 100 * SECOND;
    assert_eq!(fresh.restore_saved_conditions(&saved, login), Some(3));
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
    assert!(restored(ConflictKey::ManaShield).is_none());
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
    let saved = live.saved_conditions_snapshot(SECOND).ok_or("snapshot")?;
    assert_eq!(
        saved.as_str(),
        r#"[{"key":"muted","revision":2,"values":{"type":"gate","gate":"muted","duration_ms":4000}}]"#
    );
    assert_eq!(live.saved_conditions_snapshot(5 * SECOND), None);
    assert_eq!(
        PlayerSpellState::new(FACTS, 0, 0)
            .ok_or("actor")?
            .saved_conditions_snapshot(0),
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
    assert_eq!(fresh.restore_saved_conditions(&saved, 0), Some(1));
    let unknown =
        SavedConditions::new(r#"[{"key":"x","revision":1,"values":{"type":"z"}}]"#.to_owned())
            .ok_or("json array")?;
    assert_eq!(fresh.restore_saved_conditions(&unknown, 0), None);
    Ok(())
}
