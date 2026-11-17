use crate::play::{Outbox, Pushed, mailbox::lock};
use oteryn_session::{AppliedDelta, world_weather::WorldWeather};
pub(in crate::play) fn weather(pushed: &mut Pushed, delta: AppliedDelta<WorldWeather>) {
    oteryn_client_ui::world_weather::protocol::apply(&mut pushed.world_weather, delta.value);
}
impl Outbox {
    pub(in crate::play) fn world_weather_initialize(&self, payload: Option<Vec<u8>>) {
        lock(&self.0).pushed.world_weather = payload.and_then(|bytes| {
            oteryn_client_runtime::world_weather::decode_world_weather(&bytes).ok()
        });
    }
}
