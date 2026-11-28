//! chsw: the real client on one World with two Channels. It reads the picker, switches through
//! the real re-entry path onto the other Channel, and is refused a second switch inside the
//! cooldown. Platform, Gateway and node are the scripted `Stack`.
use super::test_support::{CHANNEL, Stack, channel_node};
use crate::play::feature_0512_channel_switch::view::now_ms;
use crate::play::{PlayLink, PlayView};
use crate::reentry::Reentry;
use oteryn_client_ui::channel_switch::{
    ChannelSwitchPanel, PvpType, cooldown_text, panel_rows, wait_text,
};
use oteryn_protocol_oteryn::channel_switch::{COOLDOWN_MS, SwitchLock};
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn deadline() -> Instant {
    Instant::now() + Duration::from_secs(10)
}

fn admitted(entry: &mut Reentry) -> TestResult<(PlayView, PlayLink)> {
    let until = deadline();
    loop {
        if let Some(result) = entry.poll() {
            return Ok(result?);
        }
        if Instant::now() >= until {
            return Err("entry timed out".into());
        }
        std::thread::yield_now();
    }
}

/// Ticks the session until the picker shows what `done` waits for.
fn panel_until(
    view: &mut PlayView,
    link: &PlayLink,
    done: impl Fn(&ChannelSwitchPanel) -> bool,
) -> TestResult<ChannelSwitchPanel> {
    let until = deadline();
    loop {
        view.tick(link)
            .map_err(|end| format!("session ended: {end:?}"))?;
        let panel = link.channel_switch_panel();
        if done(&panel) {
            return Ok(panel);
        }
        if Instant::now() >= until {
            return Err(format!("picker timed out: {panel:?}").into());
        }
        std::thread::sleep(Duration::from_millis(2));
    }
}

#[test]
fn native_admission_offer_contains_cross_town_depot() -> TestResult {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| -> Result<(), String> {
            let run = || -> TestResult {
                let stack = Stack::start_two_channels()?;
                let client = crate::ClientBootstrap::new()?.with_native_login(stack.config());
                let mut entry = Reentry::new(client);
                assert!(entry.request());
                // The TLS peer asserts 627 on ClientBootstrap before it accepts admission.
                let (_view, link) = admitted(&mut entry)?;
                drop(link);
                entry.shutdown();
                stack.finish();
                Ok(())
            };
            run().map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "native admission offer thread failed")?
        .map_err(Into::into)
}

#[test]
fn the_client_switches_to_the_other_channel_and_a_second_switch_in_the_cooldown_is_refused()
-> TestResult {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| -> Result<(), String> {
            let run = || -> TestResult {
                let stack = Stack::start_two_channels()?;
                let first = channel_node::uuid(CHANNEL).ok_or("Channel A")?;
                let second = channel_node::uuid(channel_node::CHANNEL_B).ok_or("Channel B")?;
                let client = crate::ClientBootstrap::new()?.with_native_login(stack.config());
                let mut entry = Reentry::new(client);
                assert!(entry.request());
                let (mut view, link) = admitted(&mut entry)?;
                let [x, y, floor] = channel_node::POSITION;
                let position = view.map_center();
                assert_eq!(position, (i32::from(x), i32::from(y), i16::from(floor)));

                // The picker: both Channels with their PvP type, ping and online count.
                panel_until(&mut view, &link, |panel| panel.selected)?;
                link.channel_switch_open();
                let panel = panel_until(&mut view, &link, |panel| {
                    !panel.pending && panel.rows.iter().all(|row| row.online.is_some())
                })?;
                assert_eq!(panel.current, Some(first));
                assert_eq!(
                    panel.status.map(|status| status.lock),
                    Some(SwitchLock::None)
                );
                let rows: Vec<_> = panel
                    .rows
                    .iter()
                    .map(|row| (row.channel, row.name.as_str(), row.pvp, row.online))
                    .collect();
                let [others_a, others_b] = channel_node::OTHERS;
                assert_eq!(
                    rows,
                    [
                        (
                            first,
                            "First",
                            Some(PvpType::OptionalPvp),
                            Some(others_a + 1)
                        ),
                        (second, "Second", Some(PvpType::OpenPvp), Some(others_b)),
                    ]
                );
                let cells = panel_rows(&panel, now_ms());
                assert_eq!(cells[0].0[2], "Ping");
                // No round trip is measured here (no live rtt, no https edge on port 443), so
                // both cells show "—"; the probe list itself was asked of the Platform.
                assert_eq!([&cells[1].0[2], &cells[2].0[2]], ["—", "—"]);
                assert_eq!(
                    [&cells[1].0[3], &cells[2].0[3]],
                    [&(others_a + 1).to_string(), &others_b.to_string()]
                );
                let until = deadline();
                while super::test_support::count(&stack.requests(), "GET /v1/client/channel-probes")
                    == 0
                {
                    if Instant::now() >= until {
                        return Err("probe list never requested".into());
                    }
                    std::thread::sleep(Duration::from_millis(2));
                }

                stack.node.full.store(true, Ordering::SeqCst);
                link.channel_switch_select(second);
                assert!(link.channel_switch_confirm());
                let full = panel_until(&mut view, &link, |p| !p.pending && p.error.is_some())?;
                assert_eq!(full.error.as_deref(), Some("This channel is full."));
                assert_eq!(stack.node.switches.load(Ordering::SeqCst), 0);
                link.channel_switch_back();
                assert!(
                    link.channel_switch_panel()
                        .rows
                        .iter()
                        .any(|row| row.channel == second && row.full)
                );
                link.channel_switch_close();
                stack.node.full.store(false, Ordering::SeqCst);
                link.channel_switch_open();
                panel_until(&mut view, &link, |panel| !panel.pending)?;

                // The switch to B: the server ends the session after the accepted result.
                link.channel_switch_select(second);
                assert_eq!(link.channel_switch_panel().confirming, Some(second));
                assert!(link.channel_switch_confirm());
                let until = deadline();
                let end = loop {
                    if let Err(end) = view.tick(&link) {
                        break end;
                    }
                    if Instant::now() >= until {
                        return Err("the switch did not end the session".into());
                    }
                    std::thread::sleep(Duration::from_millis(2));
                };
                let target = link
                    .channel_switch_target()
                    .ok_or_else(|| format!("no switch target after {end:?}"))?;
                assert_eq!(target.channel_id, channel_node::CHANNEL_B);
                assert_eq!(
                    target.row.as_ref().map(|row| row.name.as_str()),
                    Some("Second")
                );
                assert_eq!(stack.node.switches.load(Ordering::SeqCst), 1);
                assert!(!wait_text().is_empty());

                // As the shell does: leave play and enter B by the real re-entry path.
                drop((view, link));
                entry.leave_play();
                assert!(entry.request_switch(target.character_id, target.channel_id));
                let (mut view, link) = admitted(&mut entry)?;
                assert_eq!(stack.admissions.load(Ordering::SeqCst), 2);
                let logins: Vec<_> = stack
                    .requests()
                    .into_iter()
                    .filter(|request| request.starts_with("POST /v1/login"))
                    .collect();
                assert_eq!(logins.len(), 2);
                assert!(logins[1].contains(channel_node::CHANNEL_B), "{}", logins[1]);
                assert_eq!(view.map_center(), position, "the same position on B");

                // A second switch inside the cooldown: the server reports it and the client
                // refuses the request with the cooldown line.
                panel_until(&mut view, &link, |panel| panel.selected)?;
                link.channel_switch_open();
                let panel = panel_until(&mut view, &link, |panel| {
                    !panel.pending && panel.status.is_some()
                })?;
                assert_eq!(panel.current, Some(second));
                let status = panel.status.ok_or("status")?;
                assert_eq!(status.lock, SwitchLock::Cooldown);
                assert_eq!(status.cooldown_left_ms, COOLDOWN_MS);
                link.channel_switch_select(first);
                let refused = link.channel_switch_panel();
                assert_eq!(refused.confirming, None);
                let error = refused.error.ok_or("no refusal shown")?;
                let left = cooldown_text(COOLDOWN_MS - 1_000);
                assert!(
                    error.contains(&left) || error.contains(&cooldown_text(COOLDOWN_MS)),
                    "{error}"
                );
                assert!(
                    !link.channel_switch_confirm(),
                    "no request leaves the client"
                );
                assert_eq!(stack.node.switches.load(Ordering::SeqCst), 1);

                drop((view, link));
                entry.leave_play();
                entry.shutdown();
                stack.finish();
                Ok(())
            };
            run().map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "channel switch test thread panicked")?
        .map_err(Into::into)
}

#[test]
fn initial_channel_full_refusal_is_visible_without_retry() -> TestResult {
    let stack = Stack::start_with_frames(
        Vec::new(),
        vec![oteryn_protocol_oteryn::admission_queue::encode_channel_full()],
    )?;
    let error = stack.entry(stack.config())?;
    assert_eq!(
        crate::native_login::error_text(error),
        "This channel is full."
    );
    assert_eq!(stack.admissions.load(Ordering::SeqCst), 1);
    stack.finish();
    Ok(())
}
