use crate::play::{Outbox, Pushed, mailbox::lock};
use oteryn_session::{AppliedDelta, VipPresenceBatch};
impl Outbox {
    pub(in crate::play) fn vip_presence_initialize(&self, selected: bool) {
        lock(&self.0).vip_presence.selected = selected;
    }
}
pub(in crate::play) fn vip_presence(pushed: &mut Pushed, delta: AppliedDelta<VipPresenceBatch>) {
    pushed.vip_presence = Some(delta.value);
}
