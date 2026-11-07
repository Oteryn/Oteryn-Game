use crate::play::{Outbox, Pushed, mailbox::lock};
use oteryn_session::{AppliedDelta, world_clock::WorldClock};
pub(in crate::play) fn clock(pushed: &mut Pushed, delta: AppliedDelta<WorldClock>) {
    oteryn_client_ui::world_light::protocol::apply(&mut pushed.world_clock, delta.value);
}
impl Outbox {
    pub(in crate::play) fn world_light_initialize(&self, payload: Option<Vec<u8>>) {
        lock(&self.0).pushed.world_clock = payload.and_then(|bytes| oteryn_client_runtime::world_light::decode_world_clock(&bytes).ok());
    }
}
