use crate::coin_shop::launcher::{BrowserGate, Launcher};
use crate::combat_input::PlayCommand;
use crate::play::{PlayLink, mailbox::lock};
use oteryn_foundation::CancellationToken;
use std::sync::Arc;

#[cfg(test)]
thread_local! {
    static BEFORE_STORE_BROWSER: std::cell::Cell<Option<Box<dyn FnOnce() + Send>>> = const { std::cell::Cell::new(None) };
}

impl PlayLink {
    #[cfg(bevy_shell)]
    pub(crate) fn coin_checkout_available(&self) -> bool {
        lock(&self.mailbox)
            .connection
            .coin_shop
            .as_ref()
            .is_some_and(|shop| shop.can_start())
    }

    #[cfg(any(test, bevy_shell))]
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
        #[cfg(test)]
        let before_browser = std::sync::Mutex::new(BEFORE_STORE_BROWSER.take());
        let browser_gate: BrowserGate = Arc::new(move |cancel| {
            let Some(shared) = browser_mailbox.upgrade() else {
                return false;
            };
            let deliver = {
                let mailbox = lock(&shared);
                !mailbox.scope_pending && !mailbox.player_typing_shutdown
            };
            #[cfg(test)]
            if let Some(pause) = before_browser
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .take()
            {
                pause();
            }
            deliver && !cancel.is_cancelled()
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

#[cfg(test)]
#[test]
fn store_browser_callback_can_lock_the_play_mailbox() -> Result<(), Box<dyn std::error::Error>> {
    use crate::account_services::test_server::{CHARACTER, MockPlatform, test_thread};
    use std::{sync::mpsc, time::Duration};

    test_thread(|| {
        let mock = MockPlatform::start()?;
        let (link, _commands, outbox) = crate::play::play_channel();
        let mut config = crate::NativeLoginConfig::from_env(|name| match name {
            "OTERYN_PLATFORM_URL" | "OTERYN_GATEWAY_URL" => Some(mock.base.clone()),
            "OTERYN_OAUTH_CLIENT_ID" => Some("native-client".into()),
            _ => None,
        })?
        .ok_or("checkout config")?;
        let mailbox = Arc::downgrade(&link.mailbox);
        let (opened, result) = mpsc::channel();
        let (release, released) = mpsc::channel();
        let released = std::sync::Mutex::new(released);
        config.open_browser = Arc::new(move |_| {
            let accessible = mailbox
                .upgrade()
                .is_some_and(|shared| shared.try_lock().is_ok());
            let _ = opened.send(accessible);
            released
                .lock()
                .is_ok_and(|rx| rx.recv_timeout(Duration::from_secs(5)).is_ok())
        });
        link.configure_coin_shop(Some(config), CHARACTER);
        let launcher = lock(&link.mailbox)
            .connection
            .coin_shop
            .clone()
            .ok_or("launcher")?;
        assert!(launcher.start());
        assert!(
            result.recv_timeout(Duration::from_secs(5))?,
            "the Store called the browser opener while holding the play mailbox"
        );
        let (ended, done) = mpsc::channel();
        let closer = std::thread::spawn(move || {
            outbox.end(oteryn_platform_client::native_login::PublicClass::SessionUnavailable);
            let _ = ended.send(());
        });
        let ended = done.recv_timeout(Duration::from_secs(2));
        release.send(())?;
        closer.join().map_err(|_| "store end thread")?;
        ended?;
        Ok(())
    })
}

#[cfg(test)]
#[test]
fn session_end_wins_before_store_browser_delivery() -> Result<(), Box<dyn std::error::Error>> {
    use crate::account_services::test_server::{CHARACTER, MockPlatform, test_thread};
    use crate::play::play_channel;
    use std::{
        sync::{
            atomic::{AtomicUsize, Ordering},
            mpsc,
        },
        time::Duration,
    };

    test_thread(|| {
        let mock = MockPlatform::start()?;
        let mut config = crate::NativeLoginConfig::from_env(|name| match name {
            "OTERYN_PLATFORM_URL" | "OTERYN_GATEWAY_URL" => Some(mock.base.clone()),
            "OTERYN_OAUTH_CLIENT_ID" => Some("native-client".into()),
            _ => None,
        })?
        .ok_or("checkout config")?;
        let calls = Arc::new(AtomicUsize::new(0));
        let opened = calls.clone();
        config.open_browser = Arc::new(move |_| {
            opened.fetch_add(1, Ordering::SeqCst);
            false
        });
        let (paused, ready) = mpsc::channel();
        let (release, resume) = mpsc::channel();
        BEFORE_STORE_BROWSER.set(Some(Box::new(move || {
            assert!(paused.send(()).is_ok());
            assert!(resume.recv_timeout(Duration::from_secs(5)).is_ok());
        })));
        let (link, _commands, outbox) = play_channel();
        link.configure_coin_shop(Some(config), CHARACTER);
        let launcher = lock(&link.mailbox)
            .connection
            .coin_shop
            .clone()
            .ok_or("launcher")?;
        assert!(link.open_coin_checkout());
        ready.recv_timeout(Duration::from_secs(5))?;
        outbox.end(oteryn_platform_client::native_login::PublicClass::SessionUnavailable);
        release.send(())?;
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        while Arc::strong_count(&launcher) > 2 && std::time::Instant::now() < deadline {
            std::thread::yield_now();
        }
        assert_eq!(Arc::strong_count(&launcher), 2);
        assert_eq!(
            calls.load(Ordering::SeqCst),
            0,
            "cancelled Store checkout opened a browser"
        );
        Ok(())
    })
}

#[cfg(all(test, target_os = "linux"))]
#[test]
fn session_end_waits_for_the_configured_browser_helper() -> Result<(), Box<dyn std::error::Error>> {
    use crate::account_services::test_server::{CHARACTER, MockPlatform, test_thread};
    use std::{
        os::unix::fs::PermissionsExt,
        time::{Duration, Instant},
    };

    const ROOT: &str = "OTERYN_STORE_HELPER_TEST_ROOT";
    const LONG_HELPER: &str = "Starting long-running Store helper";
    let Some(root) = std::env::var_os(ROOT) else {
        let dir = tempfile::tempdir()?;
        let script = dir.path().join("xdg-open");
        std::fs::write(
            &script,
            "#!/bin/sh\necho $$ > \"$OTERYN_STORE_HELPER_TEST_ROOT/ready\"\nif test -f \"$OTERYN_STORE_HELPER_TEST_ROOT/long\"; then exec sleep 60; fi\nsleep 0.5\ntouch \"$OTERYN_STORE_HELPER_TEST_ROOT/done\"\n",
        )?;
        std::fs::set_permissions(script, std::fs::Permissions::from_mode(0o755))?;
        let name = std::thread::current().name().ok_or("test name")?.to_owned();
        let output = std::process::Command::new(std::env::current_exe()?)
            .args(["--exact", &name, "--nocapture"])
            .env(ROOT, dir.path())
            .env("PATH", format!("{}:/usr/bin:/bin", dir.path().display()))
            .output()?;
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(dir.path().join("done").exists());
        assert!(
            String::from_utf8_lossy(&output.stderr)
                .split_once(LONG_HELPER)
                .is_some_and(|(_, long)| long.contains("URL may already have been delivered"))
        );
        return Ok(());
    };
    test_thread(|| {
        let root = std::path::PathBuf::from(root);
        let mock = MockPlatform::start()?;
        let config = crate::NativeLoginConfig::from_env(|name| match name {
            "OTERYN_PLATFORM_URL" | "OTERYN_GATEWAY_URL" => Some(mock.base.clone()),
            "OTERYN_OAUTH_CLIENT_ID" => Some("native-client".into()),
            _ => None,
        })?
        .ok_or("checkout config")?;
        let (link, _commands, outbox) = crate::play::play_channel();
        link.configure_coin_shop(Some(config.clone()), CHARACTER);
        assert!(link.open_coin_checkout());
        let deadline = Instant::now() + Duration::from_secs(5);
        while !root.join("ready").exists() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(root.join("ready").exists());
        outbox.end(oteryn_platform_client::native_login::PublicClass::SessionUnavailable);
        assert!(
            root.join("done").exists(),
            "session end passed a live helper"
        );
        eprintln!("{LONG_HELPER}");
        std::fs::write(root.join("long"), b"long-running delivery")?;
        std::fs::remove_file(root.join("ready"))?;
        let (link, _commands, outbox) = crate::play::play_channel();
        link.configure_coin_shop(Some(config), CHARACTER);
        assert!(link.open_coin_checkout());
        let deadline = Instant::now() + Duration::from_secs(5);
        while !root.join("ready").exists() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(root.join("ready").exists());
        std::thread::sleep(Duration::from_millis(3200));
        let pid = std::fs::read_to_string(root.join("ready"))?;
        let process = std::path::Path::new("/proc").join(pid.trim());
        assert!(process.exists());
        outbox.end(oteryn_platform_client::native_login::PublicClass::SessionUnavailable);
        assert!(
            !process.exists(),
            "session end left the acknowledged opener alive"
        );
        Ok(())
    })
}
