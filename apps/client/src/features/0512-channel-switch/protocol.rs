//! chsw (capability 512): server answers for the Change Channel window.
use crate::play::{Outbox, mailbox::lock};
use oteryn_client_runtime::channel_switch::{
    COMMAND_TYPE_CHANNEL_LOAD, COMMAND_TYPE_CHANNEL_SWITCH_REQUEST,
};
use oteryn_client_runtime::raw_feature::{FeatureCommand, GatedCommandResult};

/// Client wall clock in ms; it only times the cooldown the server reported.
pub(in crate::play) fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .and_then(|elapsed| i64::try_from(elapsed.as_millis()).ok())
        .unwrap_or(0)
}

impl Outbox {
    pub(in crate::play) fn channel_switch_initialize(&self, selected: bool) {
        lock(&self.0).channel_switch.selected = selected;
    }

    /// An accepted request leaves the target for the shell, which enters it by a fresh
    /// admission once the server closes this session.
    pub(in crate::play) fn channel_switch_result(
        &self,
        command: &FeatureCommand,
        result: &GatedCommandResult,
    ) {
        let now = now_ms();
        let mut mailbox = lock(&self.0);
        if command.kind == COMMAND_TYPE_CHANNEL_SWITCH_REQUEST {
            let target = oteryn_client_ui::channel_switch::apply_request(
                &mut mailbox.channel_switch,
                result,
                now,
            );
            if target.is_some() {
                mailbox.channel_switch_target = target;
            }
        } else if command.kind == COMMAND_TYPE_CHANNEL_LOAD {
            oteryn_client_ui::channel_switch::apply_load(&mut mailbox.channel_switch, result);
        } else {
            oteryn_client_ui::channel_switch::apply_status(&mut mailbox.channel_switch, result, now);
        }
    }
}
