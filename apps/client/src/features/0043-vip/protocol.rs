use crate::play::{Outbox, mailbox::lock};

impl Outbox {
    pub(in crate::play) fn vip_initialize(&self, selected: bool) {
        lock(&self.0).vip.selected = selected;
    }
    pub(in crate::play) fn vip_result(&self, response: &oteryn_client_runtime::vip::VipResponse) {
        oteryn_client_ui::vip::apply_result(&mut lock(&self.0).vip, response);
    }
}

/// A successful management action is followed by a fresh first page.
pub(in crate::play) fn refresh_after(
    command: &oteryn_client_runtime::vip::VipCommand,
    response: &oteryn_client_runtime::vip::VipResponse,
) -> Option<oteryn_client_runtime::vip::VipCommand> {
    use oteryn_client_runtime::vip::{VipCommand, vip};
    if command.kind == vip::COMMAND_TYPE_VIP_QUERY || response.result.code != 1 {
        return None;
    }
    VipCommand::new(&vip::Request {
        operation: vip::Operation::Query,
        number: 0,
        text: String::new(),
        icon: 0,
        notify: false,
        groups: vec![],
    })
}
