use super::{ChannelRow, ChannelSwitchPanel, PvpType};
use oteryn_client_runtime::channel_switch::SwitchLock;

/// The wait notice between the safe exit and the fresh admission.
#[must_use]
pub fn wait_text() -> &'static str {
    tr!("Changing Channel… Please wait")
}

fn pvp_text(pvp: PvpType) -> &'static str {
    match pvp {
        PvpType::OptionalPvp => tr!("Optional PvP"),
        PvpType::OpenPvp => tr!("Open PvP"),
    }
}

/// `m:ss` of a cooldown, rounded up to the next second.
#[must_use]
pub fn cooldown_text(left_ms: u32) -> String {
    let seconds = left_ms.div_ceil(1000);
    format!("{}:{:02}", seconds / 60, seconds % 60)
}

/// The cooldown still left now, counted down from the server's answer.
fn cooldown_left(panel: &ChannelSwitchPanel, now_ms: i64) -> u32 {
    let Some(status) = panel.status else {
        return 0;
    };
    let elapsed =
        u32::try_from(now_ms.saturating_sub(panel.status_at_ms).max(0)).unwrap_or(u32::MAX);
    status.cooldown_left_ms.saturating_sub(elapsed)
}

/// The error text of a lock; `cooldown_left_ms` fills the cooldown text.
#[must_use]
pub fn lock_text(lock: SwitchLock, cooldown_left_ms: u32) -> String {
    match lock {
        SwitchLock::None | SwitchLock::Cooldown => lfmt!(
            "You can change the channel again in {time}.",
            time = cooldown_text(cooldown_left_ms)
        ),
        SwitchLock::Combat => tr!("You cannot change the channel while you are in a fight.").to_owned(),
        SwitchLock::Skull => {
            tr!("With a red or black skull you can only change to an Open PvP channel.").to_owned()
        }
        SwitchLock::Trade => tr!("Close the trade before you change the channel.").to_owned(),
        SwitchLock::ValueTransaction => {
            tr!("Wait until your transaction is complete before you change the channel.").to_owned()
        }
        SwitchLock::Event => {
            tr!("You cannot change the channel during a boss fight, raid or event.").to_owned()
        }
        SwitchLock::Transition => {
            tr!("You cannot change the channel while you are moving to a house, instance or another place.")
                .to_owned()
        }
        SwitchLock::Checkpoint => {
            tr!("Your character is being saved. Try again in a moment.").to_owned()
        }
        SwitchLock::Destination => tr!("This channel is not available.").to_owned(),
        SwitchLock::SameChannel => tr!("You are already on this channel.").to_owned(),
        SwitchLock::NoLogout => tr!("You cannot change the channel here.").to_owned(),
    }
}

/// The lock or cooldown that blocks every switch now, as its error text.
pub(super) fn blocking_text(panel: &ChannelSwitchPanel, now_ms: i64) -> Option<String> {
    let status = panel.status?;
    let left = cooldown_left(panel, now_ms);
    match status.lock {
        SwitchLock::None | SwitchLock::Cooldown if left == 0 => None,
        lock => Some(lock_text(lock, left)),
    }
}

/// The recommended Channel: not the current one, not full, the current PvP type first, then the
/// lowest measured ping, then the lowest load; unknown values sort last.
#[must_use]
pub fn recommended(panel: &ChannelSwitchPanel) -> Option<[u8; 16]> {
    let current_pvp = panel
        .current
        .and_then(|current| panel.row(current))
        .and_then(|row| row.pvp);
    panel
        .rows
        .iter()
        .filter(|row| Some(row.channel) != panel.current && !row.full)
        .min_by_key(|row| {
            (
                current_pvp.is_none() || row.pvp != current_pvp,
                row.ping_ms.unwrap_or(u32::MAX),
                row.online.unwrap_or(u32::MAX),
            )
        })
        .map(|row| row.channel)
}

fn status_cell(
    panel: &ChannelSwitchPanel,
    row: &ChannelRow,
    best: Option<[u8; 16]>,
    left: u32,
) -> String {
    if Some(row.channel) == panel.current {
        tr!("Current").to_owned()
    } else if row.full {
        tr!("Full").to_owned()
    } else if left > 0 {
        lfmt!("Wait {time}", time = cooldown_text(left))
    } else if Some(row.channel) == best {
        tr!("Recommended").to_owned()
    } else {
        tr!("Available").to_owned()
    }
}

/// The table: a header, then per Channel `[Channel, Type, Ping, Load, Status]` and whether the
/// row is greyed (the current Channel, a full one, or any during a lock or the cooldown).
#[must_use]
pub fn panel_rows(panel: &ChannelSwitchPanel, now_ms: i64) -> Vec<([String; 5], bool)> {
    let left = cooldown_left(panel, now_ms);
    let blocked = blocking_text(panel, now_ms).is_some();
    let best = recommended(panel);
    let unknown = || "—".to_owned();
    let mut rows = vec![(
        [
            tr!("Channel").to_owned(),
            tr!("Type").to_owned(),
            tr!("Ping").to_owned(),
            tr!("Online").to_owned(),
            tr!("Status").to_owned(),
        ],
        false,
    )];
    for row in &panel.rows {
        let grey = blocked || row.full || Some(row.channel) == panel.current;
        rows.push((
            [
                row.name.clone(),
                row.pvp.map_or_else(unknown, |pvp| pvp_text(pvp).to_owned()),
                row.ping_ms.map_or_else(unknown, |ms| format!("{ms} ms")),
                row.online.map_or_else(unknown, |online| online.to_string()),
                status_cell(panel, row, best, left),
            ],
            grey,
        ));
    }
    rows
}

/// The confirmation text; a target of Open PvP carries the warning.
#[must_use]
pub fn confirm_text(panel: &ChannelSwitchPanel) -> Option<String> {
    let row = panel.row(panel.confirming?)?;
    let mut text = lfmt!("Do you want to change to channel {name}?", name = row.name);
    if row.pvp == Some(PvpType::OpenPvp) {
        text.push_str("\n\n");
        text.push_str(tr!(
            "Warning: this is an Open PvP channel. Other players can attack you there without restrictions."
        ));
    }
    Some(text)
}

/// The line shown after the fresh admission into `row`.
#[must_use]
pub fn arrival_text(row: &ChannelRow) -> String {
    match row.pvp {
        Some(pvp) => lfmt!(
            "You are now on channel {name} ({kind}).",
            name = row.name,
            kind = pvp_text(pvp)
        ),
        None => lfmt!("You are now on channel {name}.", name = row.name),
    }
}
