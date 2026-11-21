use super::{publish_activity_handoff, play_channel};
use crate::play::mailbox::lock;
use oteryn_foundation::CancellationToken;
use std::sync::{Arc, mpsc};
use std::time::Duration;

#[test]
fn committed_handoff_cancels_checkout_and_preserves_directory()
-> Result<(), Box<dyn std::error::Error>> {
    const CHANNEL: &str = "01010101-0101-7101-8101-010101010101";
    const PLATFORM: &str = "invalid-platform-url";
    let (link, _commands, outbox) = play_channel();
    let mut config = crate::NativeLoginConfig::from_env(|name| match name {
        "OTERYN_PLATFORM_URL" | "OTERYN_GATEWAY_URL" => Some(PLATFORM.into()),
        "OTERYN_OAUTH_CLIENT_ID" => Some("test".into()),
        _ => None,
    })?
    .ok_or("native login fixture")?;
    let (ready_tx, ready_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let release_rx = std::sync::Mutex::new(release_rx);
    config.coin_shop = Arc::new(move |_, cancel| {
        let _ = ready_tx.send(cancel);
        if let Ok(release) = release_rx.lock() {
            let _ = release.recv_timeout(Duration::from_secs(5));
        }
        crate::coin_shop::Action::Back
    });
    link.configure_coin_shop(Some(config), CHANNEL);
    link.world_channels(
        CHANNEL,
        CHANNEL,
        &[crate::character_selection::ChannelChoice {
            id: CHANNEL.into(),
            name: "Harbor".into(),
            pvp_type: oteryn_platform_contracts::DirectoryPvpType::OpenPvp,
            region: "Test".into(),
        }],
    );
    link.channel_probe_platform(Some(PLATFORM.into()));
    let probe = CancellationToken::new();
    let launcher = {
        let mut mailbox = lock(&link.mailbox);
        mailbox.channel_probe_cancel = Some(probe.clone());
        mailbox.channel_switch.rows[0].online = Some(42);
        mailbox
            .connection
            .coin_shop
            .clone()
            .ok_or("configured checkout")?
    };
    assert!(launcher.start());
    let checkout = ready_rx.recv_timeout(Duration::from_secs(5))?;

    publish_activity_handoff(&outbox);

    {
        let mailbox = lock(&link.mailbox);
        assert!(
            mailbox
                .connection
                .coin_shop
                .as_ref()
                .is_some_and(|shop| Arc::ptr_eq(shop, &launcher)),
            "committed handoff dropped the checkout launcher"
        );
        let channel = oteryn_platform_client::native_login::parse_uuid_v7(CHANNEL)
            .ok_or("invalid fixture channel")?;
        assert_eq!(mailbox.channel_switch.current, Some(channel));
        assert_eq!(mailbox.party.own_channel, Some(channel));
        assert_eq!(mailbox.channel_switch.rows.len(), 1);
        assert_eq!(mailbox.channel_switch.rows[0].name, "Harbor");
        assert_eq!(mailbox.channel_switch.rows[0].online, None);
        assert_eq!(mailbox.connection.probe_platform.as_deref(), Some(PLATFORM));
        assert_eq!(
            mailbox
                .party
                .channel_names
                .get(&channel)
                .map(String::as_str),
            Some("Harbor")
        );
        assert_eq!(
            mailbox
                .guild
                .channel_names
                .get(&channel)
                .map(String::as_str),
            Some("Harbor")
        );
    }
    assert!(probe.is_cancelled());
    assert!(
        checkout.is_cancelled(),
        "source checkout survived the committed handoff"
    );
    assert!(launcher.can_start(), "destination checkout can start independently");
    outbox.end(oteryn_platform_client::native_login::PublicClass::SessionUnavailable);
    assert!(!launcher.can_start());
    release_tx.send(())?;
    Ok(())
}

#[test]
fn handoff_after_owner_drop_keeps_shutdown_and_releases_sender()
-> Result<(), Box<dyn std::error::Error>> {
    let (link, mut commands, outbox) = play_channel();
    let config = crate::NativeLoginConfig::from_env(|name| match name {
        "OTERYN_PLATFORM_URL" | "OTERYN_GATEWAY_URL" | "OTERYN_OAUTH_CLIENT_ID" => {
            Some("test".into())
        }
        _ => None,
    })?
    .ok_or("checkout config")?;
    link.configure_coin_shop(Some(config), "character");
    drop(link);
    publish_activity_handoff(&outbox);
    assert!(outbox.session_cancelled(), "handoff cleared owner shutdown");
    assert!(matches!(
        commands.try_recv(),
        Err(tokio::sync::mpsc::error::TryRecvError::Disconnected)
    ));
    Ok(())
}
