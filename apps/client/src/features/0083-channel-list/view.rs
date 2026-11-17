use crate::play::{PlayLink, PlayView, mailbox::lock};

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
        let parsed: Vec<_> = channels
            .iter()
            .filter_map(|channel| {
                Some((
                    oteryn_platform_client::native_login::parse_uuid_v7(&channel.id)?,
                    channel,
                ))
            })
            .collect();
        let names: oteryn_client_ui::social_channel::ChannelNames = parsed
            .iter()
            .map(|(id, channel)| (*id, channel.name.clone()))
            .collect();
        let mut mailbox = lock(&self.mailbox);
        mailbox.guild.channel_names.clone_from(&names);
        mailbox.party.channel_names = names;
        character.clone_into(&mut mailbox.channel_switch_character);
        mailbox.channel_switch.current =
            oteryn_platform_client::native_login::parse_uuid_v7(current);
        mailbox.party.own_channel = mailbox.channel_switch.current;
        // Ping per Channel needs a Platform endpoint and full a capacity: both stay "—"/false;
        // the online count arrives with command 6251.
        mailbox.channel_switch.rows = parsed
            .into_iter()
            .map(|(channel, choice)| ChannelRow {
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
            .collect();
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
impl PlayView {
    pub fn served_channel_text(&self) -> Option<String> {
        self.served_channel
            .map(oteryn_client_ui::channel_list::channel_text)
    }
}
