use super::TaskBoardPanel;
use std::collections::BTreeSet;

/// Use server-provided race identities behind native creature choices.
#[must_use]
pub fn creature_choices(panel: &TaskBoardPanel) -> Vec<String> {
    let mut keys = BTreeSet::new();
    keys.insert("");
    if let Some(reply) = &panel.weekly {
        for task in &reply.weekly.kills {
            keys.insert(task.race.as_str());
        }
        for offer in &reply.bounty.offers {
            keys.insert(offer.race.as_str());
        }
        if let Some(active) = &reply.bounty.active {
            keys.insert(active.race.as_str());
        }
        for slot in &reply.bounty.lists {
            keys.insert(slot.preferred.as_str());
            keys.insert(slot.unwanted.as_str());
        }
    }
    keys.into_iter().map(str::to_owned).collect()
}

#[must_use]
pub fn creature_name(key: &str) -> String {
    key.strip_prefix("oteryn:creature.")
        .unwrap_or(key)
        .replace('_', " ")
}

#[must_use]
pub fn open_key(events: &[oteryn_input_actions::NormalizedInputEvent]) -> bool {
    use oteryn_input_actions::{ButtonState, Modifier, NormalizedInputEvent};
    events.iter().any(|event| {
        matches!(event,
        NormalizedInputEvent::Key { code, state: ButtonState::Pressed, modifiers, repeat: false }
        if ((code.get() == 18 && !modifiers.contains(Modifier::Shift))
            || (code.get() == 23 && modifiers.contains(Modifier::Shift)))
            && modifiers.contains(Modifier::Control) && !modifiers.contains(Modifier::Alt)
            && !modifiers.contains(Modifier::Super))
    })
}
