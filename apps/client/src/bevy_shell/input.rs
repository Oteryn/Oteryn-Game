//! Reuse the production winit physical normalizer on every native desktop.
use super::owner::HostOwner;
use bevy::{prelude::*, winit::RawWinitWindowEvent};
use oteryn_input_platform::InputPlatformAdapter;
pub(super) struct NativeInput {
    adapter: InputPlatformAdapter,
    generation: Option<u64>,
}
pub(super) fn route(
    mut raw: MessageReader<RawWinitWindowEvent>,
    mut input: NonSendMut<NativeInput>,
    mut owner: NonSendMut<HostOwner>,
    focus: Res<bevy::input_focus::InputFocus>,
    nodes: Query<(), With<Node>>,
) {
    if input.generation != Some(owner.generation()) {
        let reset = input.adapter.notify_device_lost();
        owner.route_input(&reset, true);
        input.generation = Some(owner.generation());
        raw.clear();
        return;
    }
    for event in raw.read() {
        let events = match input.adapter.process_window_event(&event.event) {
            Ok(events) => events,
            Err(_) => input.adapter.notify_device_lost(),
        };
        let blocked = !input.adapter.focused() || focus.get().is_some_and(|e| nodes.contains(e));
        owner.route_input(&events, blocked);
    }
}

impl Default for NativeInput {
    fn default() -> Self {
        Self {
            adapter: InputPlatformAdapter::new(),
            generation: None,
        }
    }
}
