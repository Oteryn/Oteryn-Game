use super::*;
use oteryn_client_runtime::{
    CommandStatus,
    channel_switch::{SwitchLock, SwitchStatus, encode_status},
    raw_feature::GatedCommandResult,
};

const A: [u8; 16] = [1; 16];
const B: [u8; 16] = [2; 16];
const C: [u8; 16] = [3; 16];
const D: [u8; 16] = [4; 16];

fn result(status: CommandStatus, switch: SwitchStatus) -> Result<GatedCommandResult, &'static str> {
    Ok(GatedCommandResult {
        command_id: 1,
        server_sequence: 1,
        status,
        payload: encode_status(switch).ok_or("a wire status")?,
    })
}

fn row(channel: [u8; 16], name: &str, pvp: PvpType, ping: Option<u32>, full: bool) -> ChannelRow {
    ChannelRow {
        channel,
        name: name.into(),
        pvp: Some(pvp),
        ping_ms: ping,
        online: Some(40),
        full,
    }
}

fn panel() -> ChannelSwitchPanel {
    ChannelSwitchPanel {
        selected: true,
        open: true,
        current: Some(A),
        rows: vec![
            row(A, "Aurora", PvpType::OptionalPvp, Some(20), false),
            row(B, "Borealis", PvpType::OpenPvp, Some(10), false),
            row(C, "Celeste", PvpType::OptionalPvp, Some(60), false),
            row(D, "Dusk", PvpType::OptionalPvp, Some(5), true),
        ],
        ..Default::default()
    }
}

#[test]
fn c0512_window_sends_the_status_query_and_shows_the_columns() -> Result<(), &'static str> {
    let mut p = panel();
    let command = request_status(&mut p).ok_or("a status query")?;
    assert_eq!((command.capability, command.kind), (512, 6247));
    assert!(command.payload.is_empty());
    assert!(request_status(&mut p).is_none(), "one query at a time");
    apply_status(
        &mut p,
        &result(CommandStatus::Accepted, SwitchStatus::READY)?,
        0,
    );
    let rows = panel_rows(&p, 0);
    assert_eq!(
        rows[0].0,
        ["Channel", "Type", "Ping", "Online", "Status"].map(String::from)
    );
    assert_eq!(rows[1].0[4], "Current");
    assert!(rows[1].1, "the current Channel is greyed");
    assert_eq!(rows[2].0[1], "Open PvP");
    assert_eq!(rows[2].0[2], "10 ms");
    // The recommended Channel keeps the current PvP type before a lower ping elsewhere.
    assert_eq!(rows[3].0[4], "Recommended");
    assert_eq!(recommended(&p), Some(C));
    assert_eq!(rows[4].0[4], "Full");
    assert!(rows[4].1);
    assert!(!rows[2].1 && !rows[3].1);
    Ok(())
}

#[test]
fn c0512_cooldown_greys_every_row_with_a_running_timer() -> Result<(), &'static str> {
    let mut p = panel();
    let cooldown = SwitchStatus {
        lock: SwitchLock::Cooldown,
        cooldown_left_ms: 125_000,
    };
    apply_status(&mut p, &result(CommandStatus::Accepted, cooldown)?, 1_000);
    let rows = panel_rows(&p, 6_000);
    assert!(rows[1..].iter().all(|(_, grey)| *grey));
    assert_eq!(rows[3].0[4], "Wait 2:00");
    select(&mut p, C, 6_000);
    assert_eq!(p.confirming, None);
    assert_eq!(
        p.error.as_deref(),
        Some("You can change the channel again in 2:00.")
    );
    // After the cooldown ran out the rows are free again.
    let rows = panel_rows(&p, 126_000);
    assert_eq!(rows[3].0[4], "Recommended");
    assert!(!rows[3].1);
    Ok(())
}

#[test]
fn c0512_confirmed_switch_to_open_pvp_warns_and_returns_the_target() -> Result<(), &'static str> {
    let mut p = panel();
    select(&mut p, A, 0);
    assert_eq!(p.error.as_deref(), Some("You are already on this channel."));
    select(&mut p, D, 0);
    assert_eq!(p.error.as_deref(), Some("This channel is full."));
    select(&mut p, B, 0);
    assert_eq!(p.error, None);
    let text = confirm_text(&p).ok_or("a confirmation")?;
    assert!(text.starts_with("Do you want to change to channel Borealis?"));
    assert!(text.contains("Open PvP channel"));
    let command = confirm(&mut p).ok_or("a request")?;
    assert_eq!((command.capability, command.kind), (512, 6248));
    assert_eq!(command.payload, B.to_vec());
    assert_eq!(wait_text(), "Changing Channel… Please wait");
    let entered = apply_request(
        &mut p,
        &result(CommandStatus::Accepted, SwitchStatus::READY)?,
        0,
    );
    assert_eq!(entered, Some(B));
    assert_eq!(
        arrival_text(&p.rows[1]),
        "You are now on channel Borealis (Open PvP)."
    );
    Ok(())
}

#[test]
fn c0512_every_refusal_opens_the_error_of_its_lock() -> Result<(), &'static str> {
    for (lock, text) in [
        (
            SwitchLock::Combat,
            "You cannot change the channel while you are in a fight.",
        ),
        (
            SwitchLock::Skull,
            "With a red or black skull you can only change to an Open PvP channel.",
        ),
        (
            SwitchLock::Trade,
            "Close the trade before you change the channel.",
        ),
        (
            SwitchLock::Event,
            "You cannot change the channel during a boss fight, raid or event.",
        ),
        (SwitchLock::NoLogout, "You cannot change the channel here."),
    ] {
        let mut p = panel();
        select(&mut p, C, 0);
        let _ = confirm(&mut p).ok_or("a request")?;
        let status = SwitchStatus {
            lock,
            cooldown_left_ms: 0,
        };
        assert_eq!(
            apply_request(&mut p, &result(CommandStatus::Rejected, status)?, 0),
            None
        );
        assert_eq!(p.error.as_deref(), Some(text));
        assert_eq!(p.switching, None);
    }
    let mut p = ChannelSwitchPanel::default();
    assert!(request_status(&mut p).is_none());
    assert_eq!(
        p.error.as_deref(),
        Some("Channel switching is unavailable in this session")
    );
    Ok(())
}

#[test]
fn c0512_load_fills_the_online_column_and_unknown_channels_stay_unknown() -> Result<(), &'static str>
{
    let mut p = panel();
    let command = request_load(&p).ok_or("a load query")?;
    assert_eq!((command.capability, command.kind), (512, 6251));
    assert!(command.payload.is_empty());
    let payload = oteryn_client_runtime::channel_switch::encode_load(&[(A, 7), (C, 0)])
        .ok_or("a load payload")?;
    let load = |status, payload: Vec<u8>| GatedCommandResult {
        command_id: 2,
        server_sequence: 2,
        status,
        payload,
    };
    apply_load(&mut p, &load(CommandStatus::Accepted, payload));
    let rows = panel_rows(&p, 0);
    assert_eq!(rows[1].0[3], "7");
    assert_eq!(rows[3].0[3], "0");
    assert_eq!(
        rows[2].0[3], "—",
        "a Channel the server did not count stays unknown"
    );
    // A refused or malformed answer shows no count rather than a stale one.
    apply_load(&mut p, &load(CommandStatus::Rejected, Vec::new()));
    assert!(p.rows.iter().all(|row| row.online.is_none()));
    p.selected = false;
    assert!(
        request_load(&p).is_none(),
        "no query without the capability"
    );
    Ok(())
}
