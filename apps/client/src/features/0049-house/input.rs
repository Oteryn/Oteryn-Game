#[path = "input/ownership.rs"]
pub(in crate::play) mod ownership;
use crate::combat_input::PlayCommand;
use crate::play::{
    PlayLink,
    mailbox::{Mailbox, lock},
};
use oteryn_client_runtime::house::Action;
use tokio::sync::mpsc::UnboundedSender;
#[path = "controller.rs"]
pub(super) mod controller;
use super::state::Panel;
pub(in crate::play) const BUTTONS: &[&str] = &[
    controller::BUTTONS[0],
    controller::BUTTONS[1],
    controller::BUTTONS[2],
    controller::BUTTONS[3],
    controller::BUTTONS[4],
    controller::BUTTONS[5],
    controller::BUTTONS[6],
    controller::BUTTONS[7],
    controller::BUTTONS[8],
    controller::BUTTONS[9],
    "Show entrance map",
    "Load guests",
    "Load subowners",
    "Load door",
    "Ownership and move-out",
];

pub(in crate::play) fn prepare(
    panel: &mut Panel,
    selection: usize,
) -> Result<oteryn_client_runtime::raw_feature::FeatureCommand, String> {
    if selection >= BUTTONS.len() {
        return Err(tr!("Invalid action").into());
    }
    if !panel.model.selected {
        return Err(tr!("Unavailable in this session").into());
    }
    if panel.model.pending.is_some() {
        return Err(tr!("Waiting for the server").into());
    }
    let uuid = crate::play::feature_0115_mail::protocol::uuid;
    let world = uuid(panel.fields[0].trim())?;
    let town = panel.fields[1].trim();
    if panel.model.world != world || panel.model.town != town {
        panel.model.page = Default::default();
        panel.model.index = 0;
        panel.model.world = world;
        panel.model.town = town.into();
    }
    if selection >= 2 {
        panel.model.index = panel.fields[2]
            .trim()
            .parse::<usize>()
            .ok()
            .and_then(|n| n.checked_sub(1))
            .ok_or(tr!("Select a house row (starting at 1)"))?;
    }
    if selection >= 11 {
        use oteryn_client_runtime::house::HouseAclSelector;
        let selector = match selection {
            11 => HouseAclSelector::Guest,
            12 => HouseAclSelector::Subowner,
            13 => {
                let fields = panel.fields[5].split_whitespace().collect::<Vec<_>>();
                let [x, y, floor] = fields.as_slice() else {
                    return Err(tr!("Enter door x y floor").into());
                };
                HouseAclSelector::Door {
                    x: x.parse().map_err(|_| tr!("Invalid door x"))?,
                    y: y.parse().map_err(|_| tr!("Invalid door y"))?,
                    floor: floor.parse().map_err(|_| tr!("Invalid door floor"))?,
                }
            }
            _ => return Err("Invalid access list".into()),
        };
        return oteryn_client_ui::house::read_acl(&mut panel.model, selector)
            .map_err(str::to_owned);
    }
    let members = || {
        panel.fields[4]
            .split_whitespace()
            .map(uuid)
            .collect::<Result<Vec<_>, _>>()
    };
    let amount = || {
        panel.fields[3]
            .trim()
            .parse::<u64>()
            .map_err(|_| tr!("Enter an amount or Unix time").to_owned())
    };
    let action = match selection {
        0 | 1 | 7 | 9 => None,
        2 => Some(Action::Bid(amount()?)),
        3 => Some(Action::Guests(members()?)),
        4 => Some(Action::Subowners(members()?)),
        5 => {
            let coordinates = panel.fields[5].split_whitespace().collect::<Vec<_>>();
            let [x, y, floor] = coordinates.as_slice() else {
                return Err(tr!("Enter door x y floor").into());
            };
            Some(Action::Door {
                x: x.parse().map_err(|_| tr!("Invalid door x"))?,
                y: y.parse().map_err(|_| tr!("Invalid door y"))?,
                floor: floor.parse().map_err(|_| tr!("Invalid door floor"))?,
                revision: panel
                    .model
                    .page
                    .rows
                    .get(panel.model.index)
                    .ok_or(tr!("Select a house"))?
                    .acl_revision,
                members: members()?,
            })
        }
        6 => Some(Action::Kick(uuid(panel.fields[4].trim())?)),
        8 => Some(Action::Notice(amount()?)),
        _ => return Err(tr!("Invalid action").into()),
    };
    if selection >= 2 && panel.fields[6].trim() != "CONFIRM" {
        return Err(tr!("Review the selected house and enter CONFIRM to apply").into());
    }
    let result = controller::submit(&mut panel.model, selection, action).map_err(str::to_owned);
    if result.is_ok() {
        panel.fields[6].clear();
    }
    result
}
pub(in crate::play) fn submit(
    mailbox: &mut Mailbox,
    commands: &UnboundedSender<PlayCommand>,
    selection: usize,
) {
    let panel = &mut mailbox.properties.house;
    match prepare(panel, selection) {
        Ok(command) => {
            if commands.send(PlayCommand::Feature(command)).is_err() {
                panel.model.pending = None;
                panel.model.feedback = tr!("The session has ended").into();
            }
        }
        Err(message) => panel.model.feedback = message,
    }
}
impl PlayLink {
    pub(crate) fn house_world(&self, world: &str) {
        lock(&self.mailbox).properties.house.fields[0] = world.into();
    }
    pub fn house_request(&self, selection: usize) {
        if matches!(selection,8|9|14) {self.house_ownership_open();return;}
        submit(&mut lock(&self.mailbox), &self.commands, selection);
    }
    pub fn house_edit(&self, field: usize, value: String) {
        let mut mailbox = lock(&self.mailbox);
        let panel = &mut mailbox.properties.house;
        if panel.model.pending.is_none()
            && value.len() <= 8192
            && !value.contains('\0')
            && let Some(slot) = panel.fields.get_mut(field)
        {
            *slot = value;
        }
    }
    pub fn house_input(&self, events: &[oteryn_input_actions::NormalizedInputEvent]) -> bool {
        use oteryn_input_actions::{ButtonState, Modifier, Modifiers, NormalizedInputEvent};
        if !events.iter().any(|event| matches!(event, NormalizedInputEvent::Key {
            code, state: ButtonState::Pressed, modifiers, repeat: false
        } if code.get() == 11 && *modifiers == Modifiers::one(Modifier::Control).with(Modifier::Alt).with(Modifier::Shift))) { return false; }
        self.house_open();
        true
    }
    pub fn house_open(&self) {
        #[cfg(windows)]
        if !self.uses_shared_frontend() {
            super::view::open_window(self);
        }
        if cfg!(not(windows)) || self.uses_shared_frontend() {
            lock(&self.mailbox).properties.house.model.open = true;
        }
    }
}
