use super::super::*;
use crate::combat_input::{CombatCommand, PlayCommand};
use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
use oteryn_session::SpellTargetPosition;

#[test]
fn cursor_spell_click_reaches_native_combat_queue_without_walking_or_attacking()
-> Result<(), Box<dyn std::error::Error>> {
    let mut view = PlayView::new(TileCoord::new(100, 200), 7, &[])?;
    let key = NormalizedInputEvent::Key {
        code: KeyCode::new(30)?,
        state: ButtonState::Pressed,
        modifiers: Modifiers::one(Modifier::Shift),
        repeat: false,
    };
    view.cursor_spell_input(&[key]);
    assert!(view.combat_status().contains("Click a tile"));
    view.click(TileCoord::new(102, 199))?;
    assert!(
        !view
            .inputs
            .iter()
            .any(|input| matches!(input, Input::TargetWalk(_)))
    );
    assert_eq!(view.inputs.len(), 1);
    let (link, mut commands, _events) = play_channel();
    view.tick(&link).map_err(|_| "tick")?;
    assert_eq!(
        commands.try_recv()?,
        PlayCommand::Combat(CombatCommand::CastAt {
            spell: NonZeroU32::MIN,
            position: SpellTargetPosition {
                x: 102,
                y: 199,
                floor: 7
            },
        })
    );
    assert!(!view.combat_status().contains("Click a tile"));
    Ok(())
}

#[test]
fn cursor_spell_escape_and_focus_loss_cancel_and_repeats_do_not_arm()
-> Result<(), Box<dyn std::error::Error>> {
    let mut view = PlayView::new(TileCoord::new(100, 200), 0, &[])?;
    let key = |code, repeat| NormalizedInputEvent::Key {
        code,
        state: ButtonState::Pressed,
        modifiers: Modifiers::one(Modifier::Shift),
        repeat,
    };
    view.cursor_spell_input(&[key(KeyCode::new(30)?, true)]);
    assert!(!view.combat_status().contains("Click a tile"));
    for cancel in [
        key(KeyCode::ESCAPE, false),
        NormalizedInputEvent::FocusChanged { focused: false },
    ] {
        view.cursor_spell_input(&[key(KeyCode::new(30)?, false)]);
        view.cursor_spell_input(&[cancel]);
        view.click(TileCoord::new(102, 199))?;
        assert_eq!(
            view.inputs.back(),
            Some(&Input::TargetWalk(oteryn_session::ActorPosition {
                x: 102,
                y: 199,
                floor: 0,
            }))
        );
        assert!(
            !view
                .inputs
                .iter()
                .any(|input| matches!(input, Input::CastAt(..)))
        );
    }
    Ok(())
}

#[test]
fn cursor_spell_selects_entries_beyond_the_four_slot_bar() -> Result<(), Box<dyn std::error::Error>>
{
    let mut view = PlayView::new(TileCoord::new(100, 200), 0, &[])?;
    for code in [33, 31] {
        view.cursor_spell_input(&[NormalizedInputEvent::Key {
            code: KeyCode::new(code)?,
            state: ButtonState::Pressed,
            modifiers: Modifiers::one(Modifier::Shift),
            repeat: false,
        }]);
    }
    assert!(view.combat_status().starts_with("Spell 42:"));
    view.click(TileCoord::new(102, 199))?;
    assert_eq!(
        view.inputs.front(),
        Some(&Input::CastAt(
            NonZeroU32::new(42).ok_or("spell")?,
            SpellTargetPosition {
                x: 102,
                y: 199,
                floor: 0
            },
        ))
    );
    Ok(())
}
