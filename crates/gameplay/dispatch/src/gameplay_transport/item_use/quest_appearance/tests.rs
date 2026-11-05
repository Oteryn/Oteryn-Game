use super::*;
use crate::domain::condition::{ConditionValues, ConflictKey};

fn appearances() -> Result<CompiledSpellAppearances, String> {
    crate::content::native_spell_appearances::compile(
        include_bytes!(
            "../../../../../../../content/presentations/bindings/spell-appearances.json"
        ),
        [7; 32],
    )
    .map_err(|error| format!("{error:?}"))
}

fn player() -> Result<PlayerSpellState, String> {
    PlayerSpellState::new(
        CharacterCastFacts {
            vocation: Vocation::Knight,
            level: 100,
            magic_level: 10,
            max_health: 1000,
            max_mana: 500,
            max_soul: 100,
        },
        0,
        0,
    )
    .ok_or_else(|| "valid character cast facts".to_owned())
}

#[test]
fn production_artifact_already_qualifies_both_source_quest_looks() -> Result<(), String> {
    let appearances = appearances()?;
    for (key, look) in [(MUMMY_CREATURE, 65), ("oteryn:creature.mutated_bat", 307)] {
        let member = appearances
            .for_creature(key, CREATURE_REVISION)
            .ok_or("qualified source creature")?;
        assert_eq!(member.look_type(), look);
        assert_eq!(member.selection().colours, [0; 4]);
        assert_eq!(member.selection().addons, 0);
        assert_eq!(member.selection().mount_key, None);
        assert!(appearances.for_look_type(look).is_none());
    }
    Ok(())
}

#[test]
fn mummy_staging_is_temporary_and_expires_at_ten_seconds() -> Result<(), String> {
    let before = player()?;
    let next = stage_mummy(&before, &appearances()?, 100).map_err(|error| format!("{error:?}"))?;
    assert_eq!(next.revision(), before.revision() + 1);
    assert!(
        before
            .owned_conditions()
            .active_at(ConflictKey::Outfit, 100_000)
            .is_none()
    );
    let condition = next
        .owned_conditions()
        .active_at(ConflictKey::Outfit, 10_099_999)
        .ok_or("condition until deadline")?;
    assert_eq!(
        condition.definition().values(),
        ConditionValues::Outfit {
            duration_ms: 10_000,
            look_type: 65
        },
    );
    assert!(
        condition
            .definition()
            .source_appearance_selection()
            .is_some()
    );
    assert!(
        next.owned_conditions()
            .active_at(ConflictKey::Outfit, 10_100_000)
            .is_none()
    );
    Ok(())
}

#[test]
fn repeated_command_keeps_the_original_deadline_while_a_new_use_can_refresh() -> Result<(), String>
{
    let appearances = appearances()?;
    let mut gate = ItemUseGate::default();
    gate.admit(1, 100).map_err(|error| format!("{error:?}"))?;
    let next = stage_mummy(&player()?, &appearances, 100).map_err(|error| format!("{error:?}"))?;
    gate.settle(1, UseDisposition::Committed, &[]);
    assert_eq!(gate.admit(1, 1_100), Err(UseDisposition::Committed));
    assert!(
        next.owned_conditions()
            .active_at(ConflictKey::Outfit, 10_100_000)
            .is_none()
    );
    gate.admit(2, 1_100).map_err(|error| format!("{error:?}"))?;
    let refreshed =
        stage_mummy(&next, &appearances, 1_100).map_err(|error| format!("{error:?}"))?;
    assert!(
        refreshed
            .owned_conditions()
            .active_at(ConflictKey::Outfit, 10_100_000)
            .is_some()
    );
    assert!(
        refreshed
            .owned_conditions()
            .active_at(ConflictKey::Outfit, 11_100_000)
            .is_none()
    );
    Ok(())
}
