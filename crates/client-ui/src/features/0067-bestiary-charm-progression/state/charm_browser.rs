//! Confirmed names are presentation aliases for the existing authoritative wire indices.
use super::Panel;
use oteryn_client_runtime::charm_progression::catalogue::Card;
use std::collections::BTreeMap;
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CharmBrowser {
    pub cards: BTreeMap<u16, Card>,
    pub pending: Option<u16>,
    pub next: Option<u16>,
    pub loaded: bool,
}
pub fn resolve(panel: &Panel, field: usize, name: &str) -> Option<String> {
    let names: Vec<_> = match field {
        0 => panel
            .charm_browser
            .cards
            .iter()
            .filter(|(_, card)| card.name.eq_ignore_ascii_case(name.trim()))
            .map(|(&id, _)| id)
            .collect(),
        1 => panel
            .catalogue
            .names
            .iter()
            .filter(|(_, caption)| caption.eq_ignore_ascii_case(name.trim()))
            .map(|(&id, _)| id)
            .collect(),
        _ => return None,
    };
    let current = panel.fields.get(field)?.parse::<u16>().ok();
    if let Some(id) = current.filter(|id| names.contains(id)) {
        return Some(id.to_string());
    }
    match names.as_slice() {
        [id] => Some(id.to_string()),
        _ => None,
    }
}

pub fn named_fields(panel: &Panel) -> Vec<String> {
    panel
        .fields
        .iter()
        .enumerate()
        .map(|(field, value)| {
            let Some(id) = value.parse::<u16>().ok() else {
                return value.clone();
            };
            match field {
                0 => panel.charm_browser.cards.get(&id).map(|c| c.name.clone()),
                1 => panel.catalogue.names.get(&id).cloned(),
                _ => None,
            }
            .unwrap_or_else(|| value.clone())
        })
        .collect()
}
pub fn text(panel: &Panel) -> String {
    use std::fmt::Write;
    let mut text = String::from("Charms\r\n");
    if let Some(reply) = &panel.reply {
        let _ = write!(
            text,
            "{} Charm Points | {} Minor Charm Echoes\r\nAssignment limit: {}\r\n",
            reply.charms.charm_points_available,
            reply.charms.minor_charm_echoes_available,
            reply
                .charms
                .assignment_slot_limit
                .map_or_else(|| "No fixed limit".into(), |n| n.to_string())
        );
    } else {
        text.push_str("Query to load balances and unlock/assignment state\r\n");
    }
    for (&id, card) in &panel.charm_browser.cards {
        let selected = panel.fields[0] == id.to_string();
        let _ = write!(
            text,
            "{}{} — {}\r\n",
            if selected { "> " } else { "" },
            card.name,
            card.category
        );
        if let Some(state) = panel.reply.as_ref().and_then(|r| {
            r.charms
                .charms
                .iter()
                .find(|c| c.charm.get() == u32::from(id))
        }) {
            let race = state.assigned_race.map_or_else(
                || "none".into(),
                |r| {
                    u16::try_from(r.get())
                        .ok()
                        .and_then(|id| panel.catalogue.names.get(&id))
                        .cloned()
                        .unwrap_or_else(|| "Creature name not loaded".into())
                },
            );
            let currency = if card.category == "Major" {
                "Charm Points"
            } else {
                "Minor Charm Echoes"
            };
            let next = if state.unlocked_stage == 3 {
                "All stages unlocked".into()
            } else {
                format!("Next stage: {} {currency}", state.next_stage_cost)
            };
            let _ = write!(
                text,
                "Unlocked stage: {} | {next}\r\nAssigned creature: {race} | {}\r\n",
                state.unlocked_stage,
                if state.effect_active {
                    "Effect active"
                } else {
                    "Effect not yet active"
                }
            );
        }
        if selected {
            text.push_str(&card.detail);
            text.push_str("\r\n");
        }
    }
    if panel.charm_browser.next.is_some() {
        text.push_str("More charm names available\r\n");
    }
    if panel.pending {
        text.push_str("Waiting for the server\r\n");
    }
    text.push_str(&panel.feedback);
    text
}
