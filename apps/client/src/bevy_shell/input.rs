//! Reuse the production winit normalizer; world pointers use the current physical viewport.
pub(super) mod hits;
use super::{
    game_ui::{GameRegions, modal::ModalGuard, viewport::WorldViewport},
    owner::HostOwner,
};
use bevy::{prelude::*, winit::RawWinitWindowEvent};
use oteryn_input_actions::{
    NormalizedInputEvent, PointerCoordinate, PointerDelta, PointerMotion, PointerPosition,
};
use oteryn_input_platform::InputPlatformAdapter;
#[derive(bevy::ecs::system::SystemParam)]
pub(super) struct WorldInputUi<'w, 's> {
    viewport: Res<'w, WorldViewport>,
    regions: Res<'w, GameRegions>,
    guard: ModalGuard<'w, 's>,
    hits: hits::WorldPointerHits<'w, 's>,
}
pub(super) struct NativeInput {
    adapter: InputPlatformAdapter,
    generation: Option<u64>,
    physical_pointer: Option<PointerPosition>,
}
pub(super) fn route(
    mut raw: MessageReader<RawWinitWindowEvent>,
    mut input: NonSendMut<NativeInput>,
    mut owner: NonSendMut<HostOwner>,
    mut focus: ResMut<bevy::input_focus::InputFocus>,
    editors: Query<(), With<bevy::text::EditableText>>,
    ui: WorldInputUi,
) {
    if input.generation != Some(owner.generation()) {
        // Discard old gestures, but retain focus gained in the admission frame.
        let reset = input
            .adapter
            .reset_after_window_events(raw.read().map(|event| &event.event));
        owner.route_input(&reset, true);
        input.generation = Some(owner.generation());
        input.physical_pointer = None;
        return;
    }
    let modal = !ui.guard.allows_focus(None);
    let mut editing = focus.get().is_some_and(|e| editors.contains(e));
    for event in raw.read() {
        let events = match input.adapter.process_window_event(&event.event) {
            Ok(events) => events,
            Err(_) => input.adapter.notify_device_lost(),
        };
        for event in events {
            let blocked = !input.adapter.focused() || editing || modal;
            match event {
                NormalizedInputEvent::PointerMoved { position, motion } => {
                    input.physical_pointer = Some(position);
                    if !ui.hits.allows_world_start(
                        ui.regions.world,
                        Vec2::new(position.x().get() as f32, position.y().get() as f32),
                    ) {
                        owner.route_input(&[NormalizedInputEvent::DeviceLost], true);
                        continue;
                    }
                    if let Some(position) = map_pointer(&owner, *ui.viewport, position) {
                        owner.route_input(
                            &[NormalizedInputEvent::PointerMoved { position, motion }],
                            !input.adapter.focused() || !ui.guard.allows(ui.regions.world),
                        );
                    } else {
                        owner.route_input(&[NormalizedInputEvent::DeviceLost], true);
                    }
                }
                event @ (NormalizedInputEvent::MouseButton { .. }
                | NormalizedInputEvent::Wheel { .. }) => {
                    // Resize, DPI, UI scale and world zoom may change without CursorMoved.
                    // Reproject the retained physical cursor before the button reaches picking.
                    let raw_position = input.physical_pointer;
                    if !raw_position.is_some_and(|position| {
                        ui.hits.allows_world_start(
                            ui.regions.world,
                            Vec2::new(position.x().get() as f32, position.y().get() as f32),
                        )
                    }) {
                        owner.route_input(&[NormalizedInputEvent::DeviceLost], true);
                        continue;
                    }
                    let position = raw_position
                        .and_then(|position| map_pointer(&owner, *ui.viewport, position));
                    if let Some(position) = position
                        && let Ok(zero) = PointerDelta::new(0)
                    {
                        if matches!(
                            event,
                            NormalizedInputEvent::MouseButton {
                                state: oteryn_input_actions::ButtonState::Pressed,
                                ..
                            }
                        ) && !modal
                        {
                            focus.clear();
                            editing = false;
                        }
                        owner.route_input(
                            &[
                                NormalizedInputEvent::PointerMoved {
                                    position,
                                    motion: PointerMotion::new(zero, zero),
                                },
                                event,
                            ],
                            !input.adapter.focused() || !ui.guard.allows(ui.regions.world),
                        );
                    } else {
                        owner.route_input(&[NormalizedInputEvent::DeviceLost], true);
                    }
                }
                event => {
                    if matches!(
                        event,
                        NormalizedInputEvent::DeviceLost
                            | NormalizedInputEvent::FocusChanged { focused: false }
                            | NormalizedInputEvent::CaptureChanged { captured: false }
                    ) {
                        input.physical_pointer = None;
                    }
                    owner.route_input(&[event], blocked);
                }
            }
        }
    }
}
fn map_pointer(
    owner: &HostOwner,
    viewport: WorldViewport,
    position: PointerPosition,
) -> Option<PointerPosition> {
    let (w, h) = owner.play.as_ref()?.view.scene().view().viewport_px();
    let point = viewport.scene_point(
        Vec2::new(position.x().get() as f32, position.y().get() as f32),
        UVec2::new(w, h),
    )?;
    Some(PointerPosition::new(
        PointerCoordinate::new(point.x as i32).ok()?,
        PointerCoordinate::new(point.y as i32).ok()?,
    ))
}
impl Default for NativeInput {
    fn default() -> Self {
        Self {
            adapter: InputPlatformAdapter::new(),
            generation: None,
            physical_pointer: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stationary_cursor_is_remapped_after_viewport_changes()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut owner = HostOwner::new(None)?;
        let view = crate::play::PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?;
        let (w, h) = view.scene().view().viewport_px();
        let (link, _commands, _events) = crate::play::play_channel();
        owner.play = Some(super::super::owner::ActivePlay { view, link });
        let mut viewport = WorldViewport {
            origin: UVec2::new(4, 28),
            size: UVec2::new(w + 80, h + 32),
        };
        let position =
            PointerPosition::new(PointerCoordinate::new(64)?, PointerCoordinate::new(64)?);
        let first = map_pointer(&owner, viewport, position).ok_or("first cursor projection")?;
        assert_eq!((first.x().get(), first.y().get()), (20, 20));
        viewport.origin.x += 10;
        let shifted = map_pointer(&owner, viewport, position).ok_or("stationary cursor remap")?;
        assert_eq!((shifted.x().get(), shifted.y().get()), (10, 20));
        viewport.origin.x += 100;
        assert!(map_pointer(&owner, viewport, position).is_none());
        Ok(())
    }
}
