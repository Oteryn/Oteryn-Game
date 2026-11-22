use crate::content::native_gameplay::NativeGameplayInput;
use crate::content::{
    native_gameplay_test_world_id, qualified_native_gameplay_test_state,
    qualify_selected_native_gameplay_room,
};
use crate::foundation::{
    ApplicationFacts, ConditionDefinition, ConditionRefusal, ConditionSourceKind, ConditionStore,
    ConditionValues, StatusKind,
};
use oteryn_simulation_determinism::{DecisionOccurrenceId, GameplayDecisionRoot};

#[test]
fn native_boss_profiles_refuse_drunk_at_condition_admission()
-> Result<(), Box<dyn std::error::Error>> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| exercise().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "native boss policy worker panicked")??;
    Ok(())
}

fn exercise() -> Result<(), Box<dyn std::error::Error>> {
    let manifest =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content/spells.manifest.json");
    let input = NativeGameplayInput::from_manifest(&manifest)?;
    let room = qualify_selected_native_gameplay_room(native_gameplay_test_world_id()?, &input)?;
    let native = qualified_native_gameplay_test_state(room.compiled())?;
    let drunk = ConditionDefinition::new(
        "test.creature.drunk",
        1,
        ConditionValues::TimedStatus {
            kind: StatusKind::Drunk,
            duration_ms: 1000,
        },
    )
    .ok_or("valid Drunk condition")?;
    let root = GameplayDecisionRoot::from_bytes([7; 32]);
    for key in [
        "oteryn:creature.morshabaal",
        "oteryn:creature.the_primal_menace",
    ] {
        let policy = native
            .policies()
            .policies()
            .iter()
            .find(|policy| policy.definition_key == key)
            .ok_or_else(|| format!("missing native policy for {key}"))?;
        let facts = ApplicationFacts {
            now: 0,
            base_speed: u16::try_from(policy.base_speed)?,
            mana_shield_capacity: 0,
            target_reentry_protected: false,
            source_reentry_protected: false,
            target_is_player: false,
            decision_root: &root,
            occurrence: DecisionOccurrenceId::from_bytes([1; 16]),
        };
        let mut conditions = ConditionStore::<u32>::new();
        assert_eq!(
            conditions.apply(
                &drunk,
                Some(1),
                ConditionSourceKind::Creature,
                &policy.condition_immunities,
                &facts,
            ),
            Err(ConditionRefusal::Immune),
            "{key}",
        );
        assert!(!conditions.has_status(StatusKind::Drunk, 0));
        assert!(
            conditions
                .apply(&drunk, Some(1), ConditionSourceKind::Creature, &[], &facts)
                .is_ok()
        );
    }
    Ok(())
}
