use oteryn_input_actions::{ButtonState, Modifier, NormalizedInputEvent};
pub fn open_key(events: &[NormalizedInputEvent]) -> bool {
    events.iter().any(|event| matches!(event, NormalizedInputEvent::Key { code, state: ButtonState::Pressed, modifiers, repeat: false } if code.get() == 22 && modifiers.contains(Modifier::Control) && modifiers.contains(Modifier::Shift)))
}

use super::{StorePanel, catalogue};
use oteryn_client_runtime::server_mechanic::ServerMechanicCommand;

pub fn categories() -> Vec<String> {
    let mut keys = std::collections::BTreeSet::new();
    for source in [
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../rulesets/economy/store/catalogue.json"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../rulesets/economy/store/hirelings.json"
        )),
    ] {
        let Ok(document) = serde_json::from_str::<serde_json::Value>(source) else {
            return Vec::new();
        };
        let Some(offers) = document.get("offers").and_then(serde_json::Value::as_array) else {
            return Vec::new();
        };
        for offer in offers {
            if offer.get("enabled").and_then(serde_json::Value::as_bool) != Some(false)
                && let Some(key) = offer.get("category").and_then(serde_json::Value::as_str)
            {
                keys.insert(key.to_owned());
            }
        }
    }
    keys.into_iter().collect()
}

pub fn select_category(panel: &mut StorePanel, key: &str) -> Option<ServerMechanicCommand> {
    if !panel.selected || panel.pending || !categories().iter().any(|category| category == key) {
        return None;
    }
    panel.category = key.to_owned();
    panel.catalogue = None;
    panel.selected_offer = usize::MAX;
    catalogue(panel)
}
