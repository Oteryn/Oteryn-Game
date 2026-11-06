use super::{DromeAction, Intent};
use oteryn_client_runtime::arena::ArenaAction;
/// Selectors, not prices or entitlements: the server alone authorizes purchases.
pub const DROME_REWARDS: [&str; 18] = [
    "Drome Cube",
    "Arena Badge Replica",
    "Brown Pit Demon",
    "Green Pit Demon",
    "Blue Pit Demon",
    "Ogre Doll",
    "Domestikion Plushie",
    "Hoodinion Plushie",
    "Mearidion Plushie",
    "Murmillion Plushie",
    "Scissorion Plushie",
    "Retching Horror Doll",
    "Black Pit Demon",
    "Sublime Tournament Accolade",
    "Tournament Accolade",
    "Red Pit Demon",
    "Phant",
    "Lion of War Outfit",
];

use oteryn_input_actions::{ButtonState, Modifier, NormalizedInputEvent};

/// Native button IDs and zero-based reward selection. A closed/empty combo
/// reports -1 and cannot submit a purchase.
pub fn drome_intent(control: usize, selected_reward: isize) -> Option<Intent> {
    let action = match control {
        200 => DromeAction::Status,
        201 => DromeAction::Enter,
        202 => DromeAction::Leave,
        203 => DromeAction::LowerStart,
        204 => DromeAction::ClaimPotion,
        205 => DromeAction::Buy(
            u8::try_from(selected_reward.checked_add(1)?)
                .ok()
                .filter(|selector| (1..=18).contains(selector))?,
        ),
        _ => return None,
    };
    Some(Intent::Action(ArenaAction::Drome(action)))
}

/// Ctrl+Shift+Z opens the arena; Ctrl+Shift+A remains account achievements.
pub fn open_key(events: &[NormalizedInputEvent]) -> bool {
    events.iter().any(|event| {
        matches!(event,
        NormalizedInputEvent::Key { code, state: ButtonState::Pressed, modifiers, repeat: false }
        if code.get() == 29 && modifiers.contains(Modifier::Control)
        && modifiers.contains(Modifier::Shift) && !modifiers.contains(Modifier::Alt)
        && !modifiers.contains(Modifier::Super))
    })
}
