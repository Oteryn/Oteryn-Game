use super::*;
use crate::play::mailbox::lock;

#[test]
fn destination_guild_initialization_keeps_rendered_roster_channel()
-> Result<(), Box<dyn std::error::Error>> {
    const CHANNEL: &str = "01010101-0101-7101-8101-010101010101";
    let (link, _commands, outbox) = play_channel();
    link.world_channels(
        "character",
        CHANNEL,
        &[crate::character_selection::ChannelChoice {
            id: CHANNEL.into(),
            name: "Harbor".into(),
            region: "Test".into(),
            pvp_type: oteryn_platform_contracts::DirectoryPvpType::OpenPvp,
        }],
    );
    publish_activity_handoff(&outbox);
    view()?.tick(&link).map_err(|error| format!("{error:?}"))?;
    assert!(
        outbox.scope_needs_initialize(),
        "view must acknowledge destination"
    );
    outbox.guild_initialize(true);
    let channel =
        oteryn_platform_client::native_login::parse_uuid_v7(CHANNEL).ok_or("fixture channel")?;
    lock(&link.mailbox)
        .guild
        .roster
        .push(oteryn_session::guild::GuildMember {
            channel: Some(channel),
            character: channel,
            name: "Aurelia".into(),
            level: 1,
            title: "Leader".into(),
        });
    let rendered = oteryn_client_ui::guild::panel_text(&link.guild_panel());
    assert!(
        rendered.contains("Aurelia") && rendered.contains("Harbor"),
        "{rendered}"
    );
    Ok(())
}
