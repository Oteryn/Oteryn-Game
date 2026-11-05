use super::{Intent, Panel, wire};
use oteryn_client_runtime::{
    CommandStatus,
    raw_feature::{FeatureCommand, GatedCommandResult},
};

/// Crystal201189f data/libs/systems/hireling.lua:30-41,205-227.
/// The accepted character-wide entitlement projection is already encoded by command470.
pub fn outfits(entry: &wire::Entry) -> Vec<(&'static str, u16)> {
    let female = entry.sex == 2;
    let mut result = vec![("Citizen", if female { 1107 } else { 1108 })];
    for (i, (name, f, m)) in [
        ("Banker", 1109, 1110),
        ("Cook", 1113, 1114),
        ("Steward", 1115, 1116),
        ("Trader", 1111, 1112),
        ("Servant", 1117, 1118),
        ("Hydra", 1129, 1130),
        ("Ferumbras", 1131, 1132),
        ("Bonelord", 1123, 1124),
        ("Dragon", 1125, 1126),
    ]
    .into_iter()
    .enumerate()
    {
        if entry.outfits & (1 << i) != 0 {
            result.push((name, if female { f } else { m }));
        }
    }
    result
}

pub fn request(p: &mut Panel, intent: Intent) -> Option<FeatureCommand> {
    if !p.selected {
        p.feedback = Some("Hireling management unavailable in this session");
        return None;
    }
    if p.pending.is_some() {
        p.feedback = Some("Waiting for the server");
        return None;
    }
    let action = match intent {
        Intent::List => wire::Request::List,
        Intent::Return | Intent::Outfit { .. } => {
            let Some(entry) = p.entries.iter().find(|e| Some(e.hireling) == p.identity) else {
                p.feedback = Some("Select an owned hireling");
                return None;
            };
            match intent {
                Intent::Return => wire::Request::ReturnToLamp {
                    hireling: entry.hireling,
                },
                Intent::Outfit { look_type, colors } => {
                    if !outfits(entry).iter().any(|(_, look)| *look == look_type) {
                        p.feedback = Some("This outfit is not owned");
                        return None;
                    }
                    wire::Request::Outfit {
                        hireling: entry.hireling,
                        look_type,
                        colors,
                    }
                }
                Intent::List => return None,
            }
        }
    };
    p.pending = Some(action);
    p.feedback = None;
    Some(FeatureCommand {
        capability: wire::CAPABILITY_HIRELING_MANAGEMENT_V1,
        kind: wire::COMMAND_TYPE_HIRELING_MANAGEMENT,
        payload: action.encode(),
    })
}

pub fn apply_result(p: &mut Panel, r: &GatedCommandResult) -> bool {
    let Some(action) = p.pending.take() else {
        return false;
    };
    if !matches!(
        r.status,
        CommandStatus::Accepted | CommandStatus::DuplicateReplay
    ) {
        p.feedback = Some("The server refused hireling management");
        return false;
    }
    match (action, wire::Reply::decode(&r.payload)) {
        (wire::Request::List, Some(wire::Reply::State(entries))) => {
            p.identity = p
                .identity
                .filter(|id| entries.iter().any(|e| e.hireling == *id));
            p.entries = entries;
            p.feedback = None;
            true
        }
        (
            wire::Request::Outfit {
                hireling,
                look_type,
                colors,
            },
            Some(wire::Reply::Committed),
        ) => {
            if let Some(entry) = p.entries.iter_mut().find(|e| e.hireling == hireling) {
                entry.look_type = look_type;
                entry.colors = colors;
            }
            p.feedback = Some("Outfit saved; refresh to read current server state");
            true
        }
        (wire::Request::ReturnToLamp { hireling }, Some(wire::Reply::Committed)) => {
            if let Some(entry) = p.entries.iter_mut().find(|e| e.hireling == hireling) {
                entry.position = None;
            }
            p.feedback = Some("Returned to lamp; refresh to read current server state");
            true
        }
        (_, Some(wire::Reply::Refused(reason))) => {
            p.feedback = Some(match reason {
                wire::Refusal::Stale => "Session authority is stale",
                wire::Refusal::NotOwner => "You do not own this hireling",
                wire::Refusal::NotActive => "Hireling is not placed",
                wire::Refusal::OutfitNotOwned => "This outfit is not owned",
                wire::Refusal::InventoryFull => "Inbox is full",
                wire::Refusal::Capacity => "Hireling capacity reached",
                wire::Refusal::NotInHouse => "Enter the hireling's house",
                wire::Refusal::Conflict => "The command conflicts with a prior request",
            });
            false
        }
        _ => {
            p.feedback = Some("The server returned invalid hireling management data");
            false
        }
    }
}
