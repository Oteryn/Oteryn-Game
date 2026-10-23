//! Shared existing exact-actor speed fixture, moved from dispatch without a second implementation.
use super::*;
pub struct Native {
    pub actor: ExactActorRef,
    pub base: u16,
    pub pin: [u8; 32],
}
impl CurrentCreatureSpeedReader for Native {
    fn current_native_base_speed(&self, r: &ChannelRuntimeV1, a: ExactActorRef) -> Option<u16> {
        (a == self.actor
            && r.contains_live_creature(a)
            && r.content_pin().server_artifact_digest() == self.pin)
            .then_some(self.base)
    }
}

impl CurrentCreatureSpeedReader for Vec<Native> {
    fn current_native_base_speed(
        &self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
    ) -> Option<u16> {
        self.iter()
            .find_map(|source| source.current_native_base_speed(runtime, actor))
    }
}
