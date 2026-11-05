use oteryn_client_ui::{
    boss_encounter, boss_rewards, browse_field, highscores, hotkey_equip, item_forge, store,
    tutorial_notices, weapon_proficiency, world_event,
};
use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};

#[test]
fn registered_shortcuts_reach_one_panel_each() -> Result<(), oteryn_input_actions::InputError> {
    type OpenKey = fn(&[NormalizedInputEvent]) -> bool;
    let panels: [(u16, bool, OpenKey); 10] = [
        (30, true, boss_rewards::open_key),
        (5, true, browse_field::open_key),
        (8, true, hotkey_equip::open_key),
        (9, true, item_forge::open_key),
        (22, true, store::open_key),
        (19, true, weapon_proficiency::open_key),
        (13, true, boss_encounter::open_key),
        (11, false, highscores::open_key),
        (17, false, world_event::open_key),
        (17, true, tutorial_notices::open_key),
    ];
    for (index, (code, shift, _)) in panels.iter().enumerate() {
        let mut modifiers = Modifiers::one(Modifier::Control);
        if *shift {
            modifiers = modifiers.with(Modifier::Shift);
        }
        let event = NormalizedInputEvent::Key {
            code: KeyCode::new(*code)?,
            state: ButtonState::Pressed,
            modifiers,
            repeat: false,
        };
        for (other, (_, _, open)) in panels.iter().enumerate() {
            assert_eq!(
                open(std::slice::from_ref(&event)),
                index == other,
                "shortcut {code} (shift={shift}) reached panel {other}"
            );
        }
    }
    Ok(())
}
