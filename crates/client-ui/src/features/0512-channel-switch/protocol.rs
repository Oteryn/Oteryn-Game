use super::{ChannelSwitchPanel, view::lock_text};
use oteryn_client_runtime::{
    CommandStatus,
    channel_switch::{
        CAPABILITY_CHANNEL_SWITCH_V1, COMMAND_TYPE_CHANNEL_LOAD,
        COMMAND_TYPE_CHANNEL_SWITCH_REQUEST, COMMAND_TYPE_CHANNEL_SWITCH_STATUS, SwitchLock,
        decode_load, decode_status, encode_request,
    },
    raw_feature::{FeatureCommand, GatedCommandResult},
};

/// The status query, sent when the window opens. A session without capability 512 sends nothing.
pub fn request_status(panel: &mut ChannelSwitchPanel) -> Option<FeatureCommand> {
    if !panel.selected {
        panel.error = Some(tr!("Channel switching is unavailable in this session").to_owned());
        return None;
    }
    if panel.pending {
        return None;
    }
    panel.pending = true;
    panel.error = None;
    Some(FeatureCommand {
        capability: CAPABILITY_CHANNEL_SWITCH_V1,
        kind: COMMAND_TYPE_CHANNEL_SWITCH_STATUS,
        payload: Vec::new(),
    })
}

/// The online count of each Channel, asked with the status when the window opens.
#[must_use]
pub fn request_load(panel: &ChannelSwitchPanel) -> Option<FeatureCommand> {
    panel.selected.then(|| FeatureCommand {
        capability: CAPABILITY_CHANNEL_SWITCH_V1,
        kind: COMMAND_TYPE_CHANNEL_LOAD,
        payload: Vec::new(),
    })
}

/// Takes the online counts; a Channel the server did not list shows "—".
pub fn apply_load(panel: &mut ChannelSwitchPanel, result: &GatedCommandResult) {
    let counts = (result.status == CommandStatus::Accepted)
        .then(|| decode_load(&result.payload))
        .flatten()
        .unwrap_or_default();
    for row in &mut panel.rows {
        row.online = counts
            .iter()
            .find(|(channel, _)| *channel == row.channel)
            .map(|(_, online)| *online);
    }
}

/// Takes the server's status; a refusal or an unreadable answer keeps the one shown before.
pub fn apply_status(panel: &mut ChannelSwitchPanel, result: &GatedCommandResult, now_ms: i64) {
    panel.pending = false;
    match decode_status(&result.payload) {
        Some(status) if result.status == CommandStatus::Accepted => {
            panel.status = Some(status);
            panel.status_at_ms = now_ms;
        }
        _ => panel.error = Some(tr!("The server refused the channel change").to_owned()),
    }
}

/// A click on a row: the current Channel, a full one or a running lock opens the error dialog,
/// any other Channel the confirmation.
pub fn select(panel: &mut ChannelSwitchPanel, channel: [u8; 16], now_ms: i64) {
    panel.confirming = None;
    let lock = if panel.current == Some(channel) {
        Some(SwitchLock::SameChannel)
    } else if panel.row(channel).is_none() {
        Some(SwitchLock::Destination)
    } else {
        None
    };
    panel.error = match (lock, panel.row(channel)) {
        (Some(lock), _) => Some(lock_text(lock, 0)),
        (None, Some(row)) if row.full => Some(tr!("This channel is full.").to_owned()),
        _ => super::view::blocking_text(panel, now_ms),
    };
    if panel.error.is_none() {
        panel.confirming = Some(channel);
    }
}

/// The confirmed request; `None` without a confirmation or with a request in flight.
pub fn confirm(panel: &mut ChannelSwitchPanel) -> Option<FeatureCommand> {
    if panel.pending || !panel.selected {
        return None;
    }
    let target = panel.confirming.take()?;
    panel.pending = true;
    panel.switching = Some(target);
    Some(FeatureCommand {
        capability: CAPABILITY_CHANNEL_SWITCH_V1,
        kind: COMMAND_TYPE_CHANNEL_SWITCH_REQUEST,
        payload: encode_request(target),
    })
}

/// The request's answer. `Some(target)` once the server released the session: the shell keeps
/// "Changing Channel… Please wait" up and enters `target` by a fresh admission. A refusal opens
/// the error dialog of its lock.
pub fn apply_request(
    panel: &mut ChannelSwitchPanel,
    result: &GatedCommandResult,
    now_ms: i64,
) -> Option<[u8; 16]> {
    panel.pending = false;
    let status = decode_status(&result.payload);
    if result.status == CommandStatus::Accepted
        && status.is_some_and(|s| s.lock == SwitchLock::None)
    {
        return panel.switching;
    }
    panel.switching = None;
    match status {
        Some(status) => {
            panel.status = Some(status);
            panel.status_at_ms = now_ms;
            panel.error = Some(lock_text(status.lock, status.cooldown_left_ms));
        }
        None => panel.error = Some(tr!("The server refused the channel change").to_owned()),
    }
    None
}
