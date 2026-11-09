use super::*;

impl Panels {
    pub fn end(&mut self) {
        self.house_furniture_rotation = oteryn_client_ui::house_furniture_rotation::Panel {
            feedback: tr!("The session has ended").into(),
            ..Default::default()
        };
        self.house = Default::default();
        self.house_furniture_wrap = oteryn_client_ui::house_furniture_wrap::Panel {
            feedback: tr!("The session has ended").into(),
            ..Default::default()
        };
        self.staff = super::super::super::feature_0051_staff::state::Panel {
            feedback: tr!("The session has ended").into(),
            ..Default::default()
        };
        self.party_skull = oteryn_client_ui::party_skull::Panel {
            feedback: tr!("The session has ended").into(),
            ..Default::default()
        };
        self.cyclopedia_map = oteryn_client_ui::cyclopedia_map::Panel {
            feedback: tr!("The session has ended").into(),
            ..Default::default()
        };
        self.timed_item = oteryn_client_ui::timed_item::Panel {
            feedback: tr!("The session has ended").into(),
            ..Default::default()
        };
        self.raid_schedule = oteryn_client_ui::raid_schedule::Panel {
            feedback: tr!("The session has ended").into(),
            ..Default::default()
        };
        self.character_info = oteryn_client_ui::character_info::Panel {
            feedback: tr!("The session has ended").into(),
            ..Default::default()
        };
        self.guildhall = oteryn_client_ui::guildhall::Panel {
            feedback: tr!("The session has ended").into(),
            ..Default::default()
        };
        self.house_transfer = oteryn_client_ui::house_transfer::Panel {
            feedback: tr!("The session has ended").into(),
            ..Default::default()
        };
        self.house_move_out = oteryn_client_ui::house_move_out::Panel {
            feedback: tr!("The session has ended").into(),
            ..Default::default()
        };
        self.container_sort = oteryn_client_ui::container_sort::Panel {
            feedback: tr!("The session has ended").into(),
            ..Default::default()
        };
        self.guild_war = oteryn_client_ui::guild_war::Panel {
            feedback: tr!("The session has ended").into(),
            ..Default::default()
        };
        self.player_report = oteryn_client_ui::player_report::Panel {
            feedback: tr!("The session has ended").into(),
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
            *feedback = tr!("The session has ended").into();
        }
        self.appearance = oteryn_client_ui::appearance::Panel {
            feedback: tr!("The session has ended").into(),
            ..Default::default()
        };
        self.charm_progression = oteryn_client_ui::charm_progression::Panel {
            feedback: tr!("The session has ended").into(),
            ..Default::default()
        };
        self.pvp_expert = oteryn_client_ui::pvp_expert::Panel {
            feedback: tr!("The session has ended").into(),
            ..Default::default()
        };
        self.mail.selected = false;
        self.house_podium.selected = false;
        self.training.selected = false;
        self.guild_bank.selected = false;
    }
}
