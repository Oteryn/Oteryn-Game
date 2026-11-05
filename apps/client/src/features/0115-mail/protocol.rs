use crate::play::{Outbox, Stepper, mailbox::lock};
use oteryn_client_runtime::raw_feature::{CommandStatus, FeatureCommand, GatedCommandResult};
use oteryn_client_ui::mail::{Intent, Panel};
use oteryn_session::property_services::mail::{self, MailIntent};

#[derive(Debug, Default)]
pub(in crate::play) struct Panels {
    pub appearance: oteryn_client_ui::appearance::Panel,
    pub charm_progression: oteryn_client_ui::charm_progression::Panel,
    pub pvp_expert: oteryn_client_ui::pvp_expert::Panel,
    pub mail: Panel,
    pub house_podium: oteryn_client_ui::house_podium::Panel,
    pub training: oteryn_client_ui::training::Panel,
    pub guild_bank: oteryn_client_ui::guild_bank::Panel,
}
impl Panels {
    pub fn end(&mut self) {
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
impl Outbox {
    pub(in crate::play) fn properties_initialize<T: Stepper>(&self, session: &T) {
        let mut shared = lock(&self.0);
        shared.properties.appearance = oteryn_client_ui::appearance::Panel {
            selected: session.feature_selected(25),
            ..Default::default()
        };
        shared.properties.charm_progression = oteryn_client_ui::charm_progression::Panel {
            selected: session.feature_selected(67),
            ..Default::default()
        };
        shared.properties.pvp_expert = oteryn_client_ui::pvp_expert::Panel {
            selected: session.feature_selected(87),
            ..Default::default()
        };
        shared.properties.mail.selected = session.feature_selected(115);
        shared.properties.house_podium.selected = session.feature_selected(150);
        shared.properties.training.selected = session.feature_selected(54);
        shared.properties.guild_bank.selected = session.feature_selected(55);
    }
    pub(in crate::play) fn properties_result(
        &self,
        command: &FeatureCommand,
        result: &GatedCommandResult,
    ) {
        let mut shared = lock(&self.0);
        match command.capability {
            25 => super::super::feature_0025_appearance::protocol::apply(
                &mut shared.properties.appearance,
                command,
                result,
            ),
            54 => super::super::feature_0054_training::protocol::apply(
                &mut shared.properties.training,
                command,
                result,
            ),
            55 => super::super::feature_0055_guild_bank::protocol::apply(
                &mut shared.properties.guild_bank,
                command,
                result,
            ),
            67 => super::super::feature_0067_bestiary_charm_progression::protocol::apply(
                &mut shared.properties.charm_progression,
                command,
                result,
            ),
            87 => super::super::feature_0087_pvp_expert::protocol::apply(
                &mut shared.properties.pvp_expert,
                command,
                result,
            ),
            115 => apply(&mut shared.properties.mail, command, result),
            150 => super::super::feature_0150_house_podium::protocol::apply(
                &mut shared.properties.house_podium,
                command,
                result,
            ),
            _ => {}
        }
    }
}
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
            [2] if command.payload.first() == Some(&2) => "Letter text saved",
            [3] => "Add a recipient address to the letter or parcel label",
            [4] => "Unknown recipient",
            [5] => "The recipient cannot receive parcels",
            [6] => "This item cannot be mailed",
            [7] => "The recipient inbox is full",
            [8] => "Wait before posting again",
            [9] => "This item cannot be written on",
            [10] => "The letter text is too long",
            [11] => "The server refused the mail request",
            _ => "The server returned an invalid mail reply",
        }
    }
    .into();
}
