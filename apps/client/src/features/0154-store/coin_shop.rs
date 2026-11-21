use crate::coin_shop::launcher::{BrowserGate, Launcher};
use crate::combat_input::PlayCommand;
use crate::play::{PlayLink, mailbox::lock};
use oteryn_foundation::CancellationToken;
use std::sync::Arc;

impl PlayLink {
    #[cfg(bevy_shell)]
    pub(crate) fn coin_checkout_available(&self) -> bool {
        lock(&self.mailbox)
            .connection
            .coin_shop
            .as_ref()
            .is_some_and(|shop| shop.can_start())
    }

    #[cfg(bevy_shell)]
    pub(crate) fn open_coin_checkout(&self) -> bool {
        let mailbox = lock(&self.mailbox);
        !mailbox.scope_pending
            && mailbox
                .connection
                .coin_shop
                .as_ref()
                .is_some_and(|shop| shop.start())
    }

    pub(crate) fn configure_coin_shop(
        &self,
        config: Option<crate::NativeLoginConfig>,
        character_id: &str,
    ) {
        let Some(config) = config else {
            return;
        };
        let weak = Arc::downgrade(&self.mailbox);
        let commands = self.commands.downgrade();
        let browser_mailbox = weak.clone();
        let browser_gate: BrowserGate = Arc::new(move |browser, url, cancel| {
            let Some(shared) = browser_mailbox.upgrade() else {
                return false;
            };
            let mailbox = lock(&shared);
            !mailbox.scope_pending
                && !mailbox.player_typing_shutdown
                && !cancel.is_cancelled()
                && browser(url)
        });
        let after = Arc::new(move |cancel: &CancellationToken| {
            #[cfg(test)]
            crate::coin_shop::delivery_tests::pause(crate::coin_shop::delivery_tests::Point::Store);
            let Some(shared) = weak.upgrade() else {
                return;
            };
            let mut mailbox = lock(&shared);
            if cancel.is_cancelled() || mailbox.scope_pending || mailbox.player_typing_shutdown {
                return;
            }
            let Some(commands) = commands.upgrade().filter(|commands| !commands.is_closed()) else {
                return;
            };
            let panel = &mut mailbox.store;
            if !panel.selected || panel.pending {
                return;
            }
            panel.catalogue = None;
            let b = panel.browser.clone();
            if let Some(command) =
                oteryn_client_ui::store::browse(panel, &b.category, &b.search, b.sort, 0)
            {
                let _ = commands.send(PlayCommand::ServerMechanic(command));
            }
        });
        lock(&self.mailbox).connection.coin_shop = Some(Launcher::new(
            config,
            character_id.to_owned(),
            after,
            Some(browser_gate),
        ));
    }
}

#[cfg(test)]
#[test]
fn session_end_wins_before_checkout_refresh_delivery() -> Result<(), Box<dyn std::error::Error>> {
    use std::time::{Duration, Instant};
    let (link, mut commands, outbox) = crate::play::play_channel();
    let mut config = crate::NativeLoginConfig::from_env(|name| match name {
        "OTERYN_PLATFORM_URL" | "OTERYN_GATEWAY_URL" | "OTERYN_OAUTH_CLIENT_ID" => {
            Some("invalid".into())
        }
        _ => None,
    })?
    .ok_or("checkout config")?;
    let (presenter, ready, release) = crate::coin_shop::delivery_tests::store_worker_gate();
    config.coin_shop = presenter;
    link.configure_coin_shop(Some(config), "character");
    let launcher = {
        let mut mailbox = lock(&link.mailbox);
        mailbox.store.selected = true;
        mailbox.connection.coin_shop.clone().ok_or("launcher")?
    };
    assert!(launcher.start());
    ready.recv_timeout(Duration::from_secs(5))?;
    outbox.end(oteryn_platform_client::native_login::PublicClass::SessionUnavailable);
    release.send(())?;

    let deadline = Instant::now() + Duration::from_secs(5);
    while Arc::strong_count(&launcher) > 2 && Instant::now() < deadline {
        std::thread::yield_now();
    }
    assert_eq!(Arc::strong_count(&launcher), 2);
    assert!(
        commands.try_recv().is_err(),
        "cancelled worker queued a Store refresh"
    );
    Ok(())
}
