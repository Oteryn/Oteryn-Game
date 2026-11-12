use super::*;

fn spell() -> Result<NonZeroU32, &'static str> {
    NonZeroU32::new(71).ok_or("spell")
}

fn cursor() -> CursorAim {
    CursorAim {
        position: SpellTargetPosition {
            x: 101,
            y: 100,
            floor: 7,
        },
        direction: Some(StepDirection::East),
    }
}

#[test]
fn each_target_option_selects_its_aimed_cast_command() -> Result<(), &'static str> {
    let spell = spell()?;
    assert_eq!(
        cast_command(TargetOption::TowardsTarget, spell, None),
        Some(CombatCommand::Cast(spell))
    );
    assert_eq!(
        cast_command(TargetOption::FacingDirection, spell, None),
        Some(CombatCommand::CastFacing {
            spell,
            direction: None
        })
    );
    assert_eq!(
        cast_command(TargetOption::TowardsCursor, spell, Some(cursor())),
        Some(CombatCommand::CastCursor {
            spell,
            position: cursor().position,
            direction: Some(StepDirection::East),
        })
    );
    Ok(())
}

#[test]
fn a_cursor_cast_without_a_cursor_tile_sends_nothing() -> Result<(), &'static str> {
    assert_eq!(
        cast_command(TargetOption::TowardsCursor, spell()?, None),
        None
    );
    Ok(())
}
