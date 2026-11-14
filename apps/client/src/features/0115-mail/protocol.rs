use crate::play::{Outbox, Stepper, mailbox::lock};
use oteryn_client_runtime::raw_feature::{CommandStatus, FeatureCommand, GatedCommandResult};
use oteryn_client_ui::mail::{Intent, Panel};
use oteryn_session::property_services::mail::{self, MailIntent};

/// UTF-8 byte limit enforced by the MAIL_V1 text codec.
pub const MAX_TEXT_BYTES: usize = 7_996;

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

impl Outbox {
    pub(in crate::play) fn properties_initialize<T: Stepper>(&self, session: &T) {
        let mut shared = lock(&self.0);
        shared.item_forge.preview.selected = session.feature_selected(152);
        shared.imbue.details_selected = session.feature_selected(447);
        shared.imbue.success_selected = session.feature_selected(
            oteryn_session::imbuement::CAPABILITY_IMBUEMENT_SUCCESS_CHANCE_V1,
        );
        shared
            .properties
            .house
            .initialize(session.feature_selected(49));
        shared.properties.staff = super::super::feature_0051_staff::state::Panel {
            selected: session.feature_selected(51),
            tools: session.feature_selected(330),
            ..Default::default()
        };
        shared.properties.party_skull = oteryn_client_ui::party_skull::Panel {
            selected: session.feature_selected(134),
            ..Default::default()
        };
        shared.properties.cyclopedia_map = oteryn_client_ui::cyclopedia_map::Panel {
            selected: session.feature_selected(57),
            ..Default::default()
        };
        shared.properties.timed_item = oteryn_client_ui::timed_item::Panel {
            selected: session.feature_selected(39),
            ..Default::default()
        };
        shared.properties.character_info = oteryn_client_ui::character_info::Panel {
            selected: session.feature_selected(97),
            ..Default::default()
        };
        shared.properties.raid_schedule = oteryn_client_ui::raid_schedule::Panel {
            selected: session.feature_selected(149),
            ..Default::default()
        };
        shared.properties.container_sort = oteryn_client_ui::container_sort::Panel {
            selected: session.feature_selected(146),
            ..Default::default()
        };
        shared.properties.guild_war = oteryn_client_ui::guild_war::Panel {
            selected: session.feature_selected(103),
            ..Default::default()
        };
        shared.properties.house_move_out = oteryn_client_ui::house_move_out::Panel {
            selected: session.feature_selected(59),
            ..Default::default()
        };
        shared.properties.house_transfer = oteryn_client_ui::house_transfer::Panel {
            selected: session.feature_selected(86),
            ..Default::default()
        };
        shared.properties.guildhall = oteryn_client_ui::guildhall::Panel {
            selected: session.feature_selected(99),
            ..Default::default()
        };
        shared.properties.player_report = oteryn_client_ui::player_report::Panel {
            selected: session.feature_selected(76),
            ..Default::default()
        };
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
        // Publish selection and its confirmed snapshot together, including on re-admission.
        super::super::feature_0087_pvp_expert::protocol::initialize(
            &mut shared.properties.pvp_expert,
            session,
        );
        shared.properties.mail.selected = session.feature_selected(115);
        shared.properties.house_furniture_rotation =
            oteryn_client_ui::house_furniture_rotation::Panel {
                selected: session.feature_selected(445),
                ..Default::default()
            };
        shared.properties.house_furniture_wrap = oteryn_client_ui::house_furniture_wrap::Panel {
            selected: session.feature_selected(207),
            ..Default::default()
        };
        shared.properties.house_podium.selected = session.feature_selected(150);
        shared.properties.house_podium.flagged = session.feature_selected(
            oteryn_session::property_services::house_podium::CAPABILITY_HOUSE_PODIUM_FLAGS_V1,
        );
        shared.properties.training.selected = session.feature_selected(54);
        shared.properties.guild_bank.selected = session.feature_selected(55);
    }
    pub(in crate::play) fn properties_result(
        &self,
        command: &FeatureCommand,
        result: &GatedCommandResult,
    ) {
        let mut shared = lock(&self.0);
        if command.capability == 152 {
            oteryn_client_ui::item_forge::preview::apply(&mut shared.item_forge, command, result);
            return;
        }
        if command.capability == 447 {
            oteryn_client_ui::imbue::details::apply(&mut shared.imbue, command, result);
            return;
        }
        match command.capability {
            49 => shared.properties.house.apply(command, result),
            51 | 330 => super::super::feature_0051_staff::protocol::apply(
                &mut shared.properties.staff,
                command,
                result,
            ),
            25 => super::super::feature_0025_appearance::protocol::apply(
                &mut shared.properties.appearance,
                command,
                result,
            ),
            39 => super::super::feature_0039_timed_item::protocol::apply(
                &mut shared.properties.timed_item,
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
            57 => super::super::feature_0057_cyclopedia_map::protocol::apply(
                &mut shared.properties.cyclopedia_map,
                command,
                result,
            ),
            59 => super::super::feature_0059_house_move_out::protocol::apply(
                &mut shared.properties.house_move_out,
                command,
                result,
            ),
            67 => super::super::feature_0067_bestiary_charm_progression::protocol::apply(
                &mut shared.properties.charm_progression,
                command,
                result,
            ),
            76 => super::super::feature_0076_player_report::protocol::apply(
                &mut shared.properties.player_report,
                command,
                result,
            ),
            86 => super::super::feature_0086_house_transfer::protocol::apply(
                &mut shared.properties.house_transfer,
                command,
                result,
            ),
            87 => super::super::feature_0087_pvp_expert::protocol::apply(
                &mut shared.properties.pvp_expert,
                command,
                result,
            ),
            97 => super::super::feature_0097_character_info::protocol::apply(
                &mut shared.properties.character_info,
                command,
                result,
            ),
            99 => super::super::feature_0099_guildhall::protocol::apply(
                &mut shared.properties.guildhall,
                command,
                result,
            ),
            103 => super::super::feature_0103_guild_war::protocol::apply(
                &mut shared.properties.guild_war,
                command,
                result,
            ),
            445 => super::super::feature_0445_house_furniture_rotation::protocol::apply(
                &mut shared.properties.house_furniture_rotation,
                command,
                result,
            ),
            207 => super::super::feature_0207_house_furniture_wrap::protocol::apply(
                &mut shared.properties.house_furniture_wrap,
                command,
                result,
            ),
            115 => apply(&mut shared.properties.mail, command, result),
            134 => super::super::feature_0134_party_skull::protocol::apply(
                &mut shared.properties.party_skull,
                command,
                result,
            ),
            146 => super::super::feature_0146_container_sort::protocol::apply(
                &mut shared.properties.container_sort,
                command,
                result,
            ),
            149 => super::super::feature_0149_raid_schedule::protocol::apply(
                &mut shared.properties.raid_schedule,
                command,
                result,
            ),
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
        .ok_or_else(|| tr!("Select a valid item handle").into())
}
pub(in crate::play) fn uuid(text: &str) -> Result<[u8; 16], String> {
    let text: String = text.chars().filter(|c| *c != '-').collect();
    if text.len() != 32 || !text.is_ascii() {
        return Err(tr!("Enter a valid item or guild UUID").into());
    }
    let mut value = [0; 16];
    for (i, byte) in value.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&text[i * 2..i * 2 + 2], 16)
            .map_err(|_| tr!("Enter a valid UUID"))?;
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
        mail::encode_intent(&intent).map_err(|_| tr!("Letter text exceeds 7,996 UTF-8 bytes"))?;
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
        tr!("The server refused the mail request")
    } else {
        match result.payload.as_slice() {
            [1] if command.payload.first() == Some(&1) => tr!("Mail posted"),
            [2] if command.payload.first() == Some(&2) => tr!("Item text saved"),
            [3] => tr!("Add a recipient address to the letter or parcel label"),
            [4] => tr!("Unknown recipient"),
            [5] => tr!("The recipient cannot receive parcels"),
            [6] => tr!("This item cannot be mailed"),
            [7] => tr!("The recipient inbox is full"),
            [8] if command.payload.first() == Some(&2) => tr!("Wait before writing again"),
            [8] => tr!("Wait before posting again"),
            [9] => tr!("This item cannot be written on"),
            [10] => tr!("The letter text is too long"),
            [11] => tr!("The server refused the mail request"),
            _ => tr!("The server returned an invalid mail reply"),
        }
    }
    .into();
}

#[path = "panel_lifecycle.rs"]
mod panel_lifecycle;

#[cfg(test)]
#[path = "tests/e2e_i04.rs"]
mod e2e_i04;
