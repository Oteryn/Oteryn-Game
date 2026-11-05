use oteryn_client_ui::{
    achievement_notices, boss_encounter, boss_rewards, browse_field, container_tree, depot, guild,
    highscores, hotkey_equip, item_forge, market, npc_dialogue, party, prey, reference_preferences,
    stash, store, tutorial_notices, weapon_proficiency, world_event,
};
use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};

#[test]
fn registered_shortcuts_reach_one_panel_each() -> Result<(), oteryn_input_actions::InputError> {
    type OpenKey = fn(&[NormalizedInputEvent]) -> bool;
    let panels: [(u16, bool, OpenKey); 21] = [
        (15, false, oteryn_client_ui::logout::logout_key),
        (10, false, guild::open_key),
        (10, true, party::open_key),
        (19, false, prey::open_key),
        (27, true, npc_dialogue::open_key),
        (18, true, reference_preferences::open_key),
        (14, true, container_tree::input::open_key),
        (30, true, boss_rewards::open_key),
        (16, false, market::open_key),
        (16, true, achievement_notices::open_key),
        (5, true, browse_field::open_key),
        (8, true, hotkey_equip::open_key),
        (9, true, item_forge::open_key),
        (22, true, store::open_key),
        (22, false, stash::open_key),
        (15, true, depot::open_key),
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

#[test]
fn right_click_actions_coexist_in_stable_order() {
    use oteryn_client_ui::world_interaction::menu::{
        Action, ContextAction, Menu, Target, context_entries,
    };
    let world = Menu::new(
        Target::Creature(party::EntityRef {
            identity: [1; 16],
            generation: 1,
        }),
        true,
        false,
    );
    for npc in [false, true] {
        for trade in [false, true] {
            for party in [false, true] {
                let mut expected = vec![("Look", ContextAction::World(Action::Look))];
                if npc {
                    expected.extend([
                        ("Talk", ContextAction::Talk),
                        ("Trade", ContextAction::NpcTrade),
                    ]);
                }
                if trade {
                    expected.push(("Trade with this player", ContextAction::PlayerTrade));
                }
                if party {
                    expected.extend([
                        ("Invite to party", ContextAction::Invite),
                        ("Party (Ctrl+Shift+G)", ContextAction::Party),
                    ]);
                }
                assert_eq!(context_entries(Some(&world), npc, trade, party), expected);
                assert_eq!(context_entries(None, npc, trade, party), expected[1..]);
            }
        }
    }
}
