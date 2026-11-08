use crate::play::{Outbox, Stepper, mailbox::lock};
use oteryn_client_runtime::raw_feature::{CommandStatus, FeatureCommand, GatedCommandResult};
use oteryn_client_ui::mail::{Intent, Panel};
use oteryn_session::property_services::mail::{self, MailIntent};

#[derive(Debug, Default)]
pub(in crate::play) struct Panels {
    pub appearance: oteryn_client_ui::appearance::Panel,
    pub charm_progression: oteryn_client_ui::charm_progression::Panel,
    pub character_info: oteryn_client_ui::character_info::Panel,
    pub raid_schedule: oteryn_client_ui::raid_schedule::Panel,
    pub container_sort: oteryn_client_ui::container_sort::Panel,
    pub cyclopedia_map: oteryn_client_ui::cyclopedia_map::Panel,
    pub guild_war: oteryn_client_ui::guild_war::Panel,
    pub party_skull: oteryn_client_ui::party_skull::Panel,
    pub guildhall: oteryn_client_ui::guildhall::Panel,
    pub house: super::super::feature_0049_house::state::Panel,
    pub house_move_out: oteryn_client_ui::house_move_out::Panel,
    pub house_transfer: oteryn_client_ui::house_transfer::Panel,
    pub staff: super::super::feature_0051_staff::state::Panel,
    pub player_report: oteryn_client_ui::player_report::Panel,
    pub pvp_expert: oteryn_client_ui::pvp_expert::Panel,
    pub mail: Panel,
    pub house_furniture_rotation: oteryn_client_ui::house_furniture_rotation::Panel,
    pub house_furniture_wrap: oteryn_client_ui::house_furniture_wrap::Panel,
    pub house_podium: oteryn_client_ui::house_podium::Panel,
    pub timed_item: oteryn_client_ui::timed_item::Panel,
    pub training: oteryn_client_ui::training::Panel,
    pub guild_bank: oteryn_client_ui::guild_bank::Panel,
}
impl Panels {
    pub fn end(&mut self) {
        self.house_furniture_rotation = oteryn_client_ui::house_furniture_rotation::Panel {
            feedback: "The session has ended".into(),
            ..Default::default()
        };
        self.house = Default::default();
        self.house_furniture_wrap = oteryn_client_ui::house_furniture_wrap::Panel {
            feedback: "The session has ended".into(),
            ..Default::default()
        };
        self.staff = super::super::feature_0051_staff::state::Panel {
            feedback: "The session has ended".into(),
            ..Default::default()
        };
        self.party_skull = oteryn_client_ui::party_skull::Panel {
            feedback: "The session has ended".into(),
            ..Default::default()
        };
        self.cyclopedia_map = oteryn_client_ui::cyclopedia_map::Panel {
            feedback: "The session has ended".into(),
            ..Default::default()
        };
        self.timed_item = oteryn_client_ui::timed_item::Panel {
            feedback: "The session has ended".into(),
            ..Default::default()
        };
        self.raid_schedule = oteryn_client_ui::raid_schedule::Panel {
            feedback: "The session has ended".into(),
            ..Default::default()
        };
        self.character_info = oteryn_client_ui::character_info::Panel {
            feedback: "The session has ended".into(),
            ..Default::default()
        };
        self.guildhall = oteryn_client_ui::guildhall::Panel {
            feedback: "The session has ended".into(),
            ..Default::default()
        };
        self.house_transfer = oteryn_client_ui::house_transfer::Panel {
            feedback: "The session has ended".into(),
            ..Default::default()
        };
        self.house_move_out = oteryn_client_ui::house_move_out::Panel {
            feedback: "The session has ended".into(),
            ..Default::default()
        };
        self.container_sort = oteryn_client_ui::container_sort::Panel {
            feedback: "The session has ended".into(),
            ..Default::default()
        };
        self.guild_war = oteryn_client_ui::guild_war::Panel {
            feedback: "The session has ended".into(),
            ..Default::default()
        };
        self.player_report = oteryn_client_ui::player_report::Panel {
            feedback: "The session has ended".into(),
            ..Default::default()
        };
        for (open, pending, feedback) in [
            (
                &mut self.mail.open,
                &mut self.mail.pending,
                &mut self.mail.feedback,
            ),
            (
                &mut self.house_podium.open,
                &mut self.house_podium.pending,
                &mut self.house_podium.feedback,
            ),
            (
                &mut self.training.open,
                &mut self.training.pending,
                &mut self.training.feedback,
            ),
            (
                &mut self.guild_bank.open,
                &mut self.guild_bank.pending,
                &mut self.guild_bank.feedback,
            ),
        ] {
            *open = false;
            *pending = false;
            *feedback = "The session has ended".into();
        }
        self.appearance = oteryn_client_ui::appearance::Panel {
            feedback: "The session has ended".into(),
            ..Default::default()
        };
        self.charm_progression = oteryn_client_ui::charm_progression::Panel {
            feedback: "The session has ended".into(),
            ..Default::default()
        };
        self.pvp_expert = oteryn_client_ui::pvp_expert::Panel {
            feedback: "The session has ended".into(),
            ..Default::default()
        };
        self.mail.selected = false;
        self.house_podium.selected = false;
        self.training.selected = false;
        self.guild_bank.selected = false;
    }
}
include!("protocol/methods.rs");
pub(in crate::play) fn accepted(result: &GatedCommandResult) -> bool {
    matches!(
        result.status,
        CommandStatus::Accepted | CommandStatus::DuplicateReplay
    )
}
pub(in crate::play) fn handle(text: &str) -> Result<oteryn_session::ItemHandle, String> {
    text.trim()
        .parse::<u64>()
        .ok()
        .and_then(oteryn_session::ItemHandle::new)
        .ok_or_else(|| "Select a valid item handle".into())
}
pub(in crate::play) fn uuid(text: &str) -> Result<[u8; 16], String> {
    let text: String = text.chars().filter(|c| *c != '-').collect();
    if text.len() != 32 || !text.is_ascii() {
        return Err("Enter a valid item or guild UUID".into());
    }
    let mut value = [0; 16];
    for (i, byte) in value.iter_mut().enumerate() {
        *byte =
            u8::from_str_radix(&text[i * 2..i * 2 + 2], 16).map_err(|_| "Enter a valid UUID")?;
    }
    Ok(value)
}
pub(in crate::play) fn command(panel: &Panel, intent: Intent) -> Result<FeatureCommand, String> {
    let item_handle = handle(&panel.fields[0])?.get();
    let intent = match intent {
        Intent::Write => MailIntent::WriteText {
            item_handle,
            text: panel.fields[2].clone(),
        },
        Intent::Post => MailIntent::Post {
            item_handle,
            mailbox_handle: handle(&panel.fields[1])?.get(),
        },
    };
    let payload =
        mail::encode_intent(&intent).map_err(|_| "Letter text exceeds 7,996 UTF-8 bytes")?;
    Ok(FeatureCommand {
        capability: 115,
        kind: 124,
        payload,
    })
}
fn apply(panel: &mut Panel, command: &FeatureCommand, result: &GatedCommandResult) {
    if !panel.pending || panel.pending_kind != command.kind {
        return;
    }
    panel.pending = false;
    panel.feedback = if !accepted(result) {
        "The server refused the mail request"
    } else {
        match result.payload.as_slice() {
            [1] if command.payload.first() == Some(&1) => "Mail posted",
            [2] if command.payload.first() == Some(&2) => "Item text saved",
            [3] => "Add a recipient address to the letter or parcel label",
            [4] => "Unknown recipient",
            [5] => "The recipient cannot receive parcels",
            [6] => "This item cannot be mailed",
            [7] => "The recipient inbox is full",
            [8] if command.payload.first() == Some(&2) => "Wait before writing again",
            [8] => "Wait before posting again",
            [9] => "This item cannot be written on",
            [10] => "The letter text is too long",
            [11] => "The server refused the mail request",
            _ => "The server returned an invalid mail reply",
        }
    }
    .into();
}
