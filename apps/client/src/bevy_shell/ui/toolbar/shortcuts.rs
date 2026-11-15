//! Toolbar entry points reuse the keyboard owners without routing through editable bindings.
use super::*;
use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};

pub(super) fn open(owner: &mut HostOwner, id: &str) -> bool {
    if owner.play.is_none() {
        return false;
    }
    match id {
        "map" => {
            owner.request_toolbar_map();
            return true;
        }
        "houses" => {
            owner.request_toolbar_house();
            return true;
        }
        "bars" => {
            let mut preferences = owner.settings.preferences.clone();
            preferences.action_bar.visible = !preferences.action_bar.visible;
            owner.settings.apply(preferences);
            return true;
        }
        _ => {}
    }
    let Some(play) = &owner.play else {
        return false;
    };
    let link = &play.link;
    let (code, shift, alt) = match id {
        "cyclopedia" => (24, false, false),
        "rewards" => (7, false, false),
        "forge" => (9, true, false),
        "outfit" => (4, false, true),
        "prey" => (19, false, false),
        "proficiency" => (19, true, false),
        "wheel" => (26, false, false),
        "npc" => (27, true, false),
        _ => (4, false, false),
    };
    let Ok(code) = KeyCode::new(code) else {
        return false;
    };
    let mut modifiers = Modifiers::one(Modifier::Control);
    if shift {
        modifiers = modifiers.with(Modifier::Shift);
    }
    if alt {
        modifiers = modifiers.with(Modifier::Alt);
    }
    let events = [NormalizedInputEvent::Key {
        code,
        modifiers,
        state: ButtonState::Pressed,
        repeat: false,
    }];
    match id {
        "cyclopedia" => link.cyclopedia_character_input(&events),
        "rewards" => link.daily_reward_input(&events),
        "forge" => link.item_forge_input(&events),
        "outfit" => link.appearance_input(&events),
        "prey" => link.prey_input(&events),
        "proficiency" => link.weapon_proficiency_input(&events),
        "wheel" => link.wheel_input(&events),
        "npc" => link.npc_dialogue_input(&events, &play.view),
        "highscores" => {
            owner.controls.highscores_generation = Some(owner.generation());
            link.highscores_request(oteryn_client_ui::highscores::Action::Refresh);
            true
        }
        "imbuing" => {
            link.open_imbue();
            true
        }
        "stash" => {
            link.open_stash();
            true
        }
        "store" => {
            link.store_set_open(true);
            true
        }
        "tasks" => {
            link.task_board_set_open(true);
            true
        }
        "bug" => {
            link.bug_report_open();
            true
        }
        "report" => {
            link.rule_violation_open("");
            true
        }
        _ => false,
    }
}
