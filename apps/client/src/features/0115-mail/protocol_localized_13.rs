use super::*;
impl Outbox {
    pub(in crate::play) fn properties_initialize<T: Stepper>(&self, session: &T) {
        let mut shared = lock(&self.0);
        shared.imbue.details_selected = session.feature_selected(447);
        shared
            .properties
            .house
            .initialize(session.feature_selected(49));
        shared.properties.staff = super::super::super::feature_0051_staff::state::Panel {
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
        super::super::super::feature_0087_pvp_expert::protocol::initialize(
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
        shared.properties.training.selected = session.feature_selected(54);
        shared.properties.guild_bank.selected = session.feature_selected(55);
    }
    pub(in crate::play) fn properties_result(
        &self,
        command: &FeatureCommand,
        result: &GatedCommandResult,
    ) {
        let mut shared = lock(&self.0);
        if command.capability == 447 {
            oteryn_client_ui::imbue::details::apply(&mut shared.imbue, command, result);
            return;
        }
        match command.capability {
            49 => shared.properties.house.model.apply(command, result),
            51 | 330 => super::super::super::feature_0051_staff::protocol::apply(
                &mut shared.properties.staff,
                command,
                result,
            ),
            25 => super::super::super::feature_0025_appearance::protocol::apply(
                &mut shared.properties.appearance,
                command,
                result,
            ),
            39 => super::super::super::feature_0039_timed_item::protocol::apply(
                &mut shared.properties.timed_item,
                command,
                result,
            ),
            54 => super::super::super::feature_0054_training::protocol::apply(
                &mut shared.properties.training,
                command,
                result,
            ),
            55 => super::super::super::feature_0055_guild_bank::protocol::apply(
                &mut shared.properties.guild_bank,
                command,
                result,
            ),
            57 => super::super::super::feature_0057_cyclopedia_map::protocol::apply(
                &mut shared.properties.cyclopedia_map,
                command,
                result,
            ),
            59 => super::super::super::feature_0059_house_move_out::protocol::apply(
                &mut shared.properties.house_move_out,
                command,
                result,
            ),
            67 => super::super::super::feature_0067_bestiary_charm_progression::protocol::apply(
                &mut shared.properties.charm_progression,
                command,
                result,
            ),
            76 => super::super::super::feature_0076_player_report::protocol::apply(
                &mut shared.properties.player_report,
                command,
                result,
            ),
            86 => super::super::super::feature_0086_house_transfer::protocol::apply(
                &mut shared.properties.house_transfer,
                command,
                result,
            ),
            87 => super::super::super::feature_0087_pvp_expert::protocol::apply(
                &mut shared.properties.pvp_expert,
                command,
                result,
            ),
            97 => super::super::super::feature_0097_character_info::protocol::apply(
                &mut shared.properties.character_info,
                command,
                result,
            ),
            99 => super::super::super::feature_0099_guildhall::protocol::apply(
                &mut shared.properties.guildhall,
                command,
                result,
            ),
            103 => super::super::super::feature_0103_guild_war::protocol::apply(
                &mut shared.properties.guild_war,
                command,
                result,
            ),
            445 => super::super::super::feature_0445_house_furniture_rotation::protocol::apply(
                &mut shared.properties.house_furniture_rotation,
                command,
                result,
            ),
            207 => super::super::super::feature_0207_house_furniture_wrap::protocol::apply(
                &mut shared.properties.house_furniture_wrap,
                command,
                result,
            ),
            115 => apply(&mut shared.properties.mail, command, result),
            134 => super::super::super::feature_0134_party_skull::protocol::apply(
                &mut shared.properties.party_skull,
                command,
                result,
            ),
            146 => super::super::super::feature_0146_container_sort::protocol::apply(
                &mut shared.properties.container_sort,
                command,
                result,
            ),
            149 => super::super::super::feature_0149_raid_schedule::protocol::apply(
                &mut shared.properties.raid_schedule,
                command,
                result,
            ),
            150 => super::super::super::feature_0150_house_podium::protocol::apply(
                &mut shared.properties.house_podium,
                command,
                result,
            ),
            _ => {}
        }
    }
}
