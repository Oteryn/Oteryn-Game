//! chsw (capability 512): the Change Channel window's actions.
use super::protocol::now_ms;
use crate::combat_input::PlayCommand;
use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::channel_switch;
use oteryn_foundation::CancellationToken;
use oteryn_platform_client::channel_probe::{PROBE_REFRESH_INTERVAL, measure_all, probe_client};
use oteryn_platform_client::{PlatformClient, PlatformClientConfig};

impl PlayLink {
    /// The Platform the picker asks for client-channel-probe-v1 targets; `None` measures only
    /// the current Channel.
    pub fn channel_probe_platform(&self, platform: Option<String>) {
        lock(&self.mailbox).channel_probe_platform = platform;
    }

    /// Opens the window and asks the server for the locks and the cooldown.
    pub fn channel_switch_open(&self) {
        // Read before the mailbox lock: the live round trip locks it too.
        let live = self.performance_latency();
        let mut mailbox = lock(&self.mailbox);
        let mailbox = &mut *mailbox;
        let panel = &mut mailbox.channel_switch;
        panel.open = true;
        let due = mailbox
            .channel_probe_refreshed
            .is_none_or(|at| at.elapsed() >= PROBE_REFRESH_INTERVAL);
        let platform = mailbox.channel_probe_platform.clone().filter(|_| due);
        if let Some(platform) = platform {
            mailbox.channel_probe_refreshed = Some(std::time::Instant::now());
            mailbox.channel_probe_pings.clear();
            let cancel = CancellationToken::new();
            if let Some(previous) = mailbox.channel_probe_cancel.replace(cancel.clone()) {
                previous.cancel();
            }
            refresh_probes(std::sync::Arc::downgrade(&self.mailbox), platform, cancel);
        }
        let panel = &mut mailbox.channel_switch;
        let current = panel.current;
        for row in &mut panel.rows {
            row.ping_ms = channel_ping(row.channel, current, live, &mailbox.channel_probe_pings);
        }
        panel.confirming = None;
        if let Some(command) = channel_switch::request_status(panel)
            && self.commands.send(PlayCommand::Feature(command)).is_err()
        {
            panel.pending = false;
            panel.error = Some(tr!("The session has ended").to_owned());
        }
        // The online column; a lost query leaves it at "—".
        if let Some(command) = channel_switch::request_load(panel) {
            let _ = self.commands.send(PlayCommand::Feature(command));
        }
    }

    /// A click on a Channel row: the confirmation, or the error of the lock in the way.
    pub fn channel_switch_select(&self, channel: [u8; 16]) {
        channel_switch::select(&mut lock(&self.mailbox).channel_switch, channel, now_ms());
    }

    /// The confirmed switch request.
    pub fn channel_switch_confirm(&self) -> bool {
        let mut mailbox = lock(&self.mailbox);
        let panel = &mut mailbox.channel_switch;
        let Some(command) = channel_switch::confirm(panel) else {
            return false;
        };
        if self.commands.send(PlayCommand::Feature(command)).is_err() {
            panel.pending = false;
            panel.switching = None;
            panel.error = Some(tr!("The session has ended").to_owned());
            return false;
        }
        true
    }

    /// Back from a confirmation or an error dialog to the Channel list.
    pub fn channel_switch_back(&self) {
        let mut mailbox = lock(&self.mailbox);
        mailbox.channel_switch.confirming = None;
        mailbox.channel_switch.error = None;
    }

    pub fn channel_switch_close(&self) {
        let mut mailbox = lock(&self.mailbox);
        let panel = &mut mailbox.channel_switch;
        panel.open = false;
        panel.confirming = None;
        panel.error = None;
        if let Some(cancel) = mailbox.channel_probe_cancel.take() {
            cancel.cancel();
        }
    }
}

/// Ping of one Channel: the current Channel is measured on the live session; another Channel
/// takes its last client-channel-probe-v1 measurement, or "—" without one.
fn channel_ping(
    channel: [u8; 16],
    current: Option<[u8; 16]>,
    live: Option<u32>,
    probes: &[([u8; 16], Option<u32>)],
) -> Option<u32> {
    if current == Some(channel) {
        live
    } else {
        probes
            .iter()
            .find(|(probed, _)| *probed == channel)
            .and_then(|(_, ping)| *ping)
    }
}

/// Fetches the probe list and measures every edge off the frame loop; the pings land only
/// while the picker is still open. A failed list or edge leaves "—".
fn refresh_probes(
    mailbox: std::sync::Weak<std::sync::Mutex<crate::play::mailbox::Mailbox>>,
    platform: String,
    cancel: CancellationToken,
) {
    let open = move || {
        mailbox
            .upgrade()
            .filter(|shared| lock(shared).channel_switch.open)
    };
    let work = move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .ok()?;
        let pings = runtime.block_on(async {
            let config = PlatformClientConfig::new(&platform).ok()?;
            let list = PlatformClient::new(config)
                .ok()?
                .fetch_channel_probes(cancel.clone())
                .await
                .ok()?;
            open()?;
            let client = probe_client().ok()?;
            tokio::select! {
                biased;
                () = cancel.cancelled() => None,
                pings = measure_all(&client, &list.probes) => Some(pings),
            }
        })?;
        let shared = open()?;
        let mut mailbox = lock(&shared);
        let mailbox = &mut *mailbox;
        if cancel.is_cancelled() {
            return None;
        }
        let current = mailbox.channel_switch.current;
        for row in &mut mailbox.channel_switch.rows {
            if current != Some(row.channel) {
                row.ping_ms = channel_ping(row.channel, current, None, &pings);
            }
        }
        mailbox.channel_probe_pings = pings;
        Some(())
    };
    let _ = std::thread::Builder::new()
        .name("channel-probe".into())
        .spawn(move || {
            let _ = work();
        });
}

#[cfg(test)]
mod ping_tests {
    #[test]
    fn failed_probe_list_clears_the_previous_measurements() {
        let (link, _commands, _) = crate::play::play_channel();
        super::lock(&link.mailbox).channel_probe_pings.push(([2; 16], Some(17)));
        link.channel_probe_platform(Some("invalid".into()));
        link.channel_switch_open();
        assert!(super::lock(&link.mailbox).channel_probe_pings.is_empty());
        let cancel = super::lock(&link.mailbox).channel_probe_cancel.clone();
        link.channel_switch_close();
        assert!(cancel.is_some_and(|token| token.is_cancelled()));
    }

    #[test]
    fn the_current_channel_is_live_and_another_takes_its_probe() {
        let probes = [([2; 16], Some(17)), ([3; 16], None)];
        let ping = |channel| super::channel_ping(channel, Some([1; 16]), Some(42), &probes);
        assert_eq!(ping([1; 16]), Some(42));
        assert_eq!(ping([2; 16]), Some(17));
        assert_eq!(ping([3; 16]), None, "a failed edge shows no ping");
        assert_eq!(
            ping([4; 16]),
            None,
            "a Channel without a probe shows no ping"
        );
    }
}
