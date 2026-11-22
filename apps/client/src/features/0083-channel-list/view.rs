use crate::play::{
    PlayLink, PlayView,
    mailbox::{Mailbox, lock},
};

impl PlayLink {
    /// chsw (capabilities 512 and 513): the World's Channels from the directory. Party, VIP and
    /// Guild rows name a Channel instead of showing its id; the Change Channel window lists them.
    pub fn world_channels(
        &self,
        character: &str,
        current: &str,
        channels: &[crate::character_selection::ChannelChoice],
    ) {
        use oteryn_client_ui::channel_switch::{ChannelRow, PvpType};
        use oteryn_platform_contracts::DirectoryPvpType;
        let mut mailbox = lock(&self.mailbox);
        character.clone_into(&mut mailbox.connection.channel_character);
        mailbox.connection.channel_current =
            oteryn_platform_client::native_login::parse_uuid_v7(current);
        // Load and ping are observations of the current scope, so start with directory rows.
        mailbox.connection.channel_rows = channels
            .iter()
            .filter_map(|choice| {
                let channel = oteryn_platform_client::native_login::parse_uuid_v7(&choice.id)?;
                Some(ChannelRow {
                    channel,
                    name: choice.name.clone(),
                    pvp: Some(match choice.pvp_type {
                        DirectoryPvpType::OptionalPvp => PvpType::OptionalPvp,
                        DirectoryPvpType::OpenPvp => PvpType::OpenPvp,
                    }),
                    ping_ms: None,
                    online: None,
                    full: false,
                })
            })
            .collect();
        mailbox.project_world_channels();
    }

    #[must_use]
    pub fn social_channels(
        &self,
    ) -> (
        Option<[u8; 16]>,
        oteryn_client_ui::social_channel::ChannelNames,
    ) {
        let mailbox = lock(&self.mailbox);
        (
            mailbox.party.own_channel,
            mailbox.party.channel_names.clone(),
        )
    }
}
impl Mailbox {
    pub(in crate::play) fn project_world_channels(&mut self) {
        self.channel_switch
            .rows
            .clone_from(&self.connection.channel_rows);
        self.channel_switch.current = self.connection.channel_current;
        self.party.own_channel = self.connection.channel_current;
        self.party.channel_names = self
            .connection
            .channel_rows
            .iter()
            .map(|row| (row.channel, row.name.clone()))
            .collect();
        self.guild
            .channel_names
            .clone_from(&self.party.channel_names);
    }
}

impl PlayView {
    pub fn served_channel_text(&self) -> Option<String> {
        self.served_channel
            .map(oteryn_client_ui::channel_list::channel_text)
    }
}
