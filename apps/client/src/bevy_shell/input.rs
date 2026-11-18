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
#[derive(Component)]
pub(in crate::bevy_shell) struct ChatComposer;

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
    editors: Query<Option<&ChatComposer>, With<bevy::text::EditableText>>,
    ui: WorldInputUi,
    mut capture: super::game_ui::ItemPointerCapture,
) {
    if input.generation != Some(owner.generation()) {
        // Discard old gestures, but retain focus gained in the admission frame.
        let reset = input
            .adapter
            .reset_after_window_events(raw.read().map(|event| &event.event));
        capture.before(
            &owner,
            *ui.viewport,
            None,
            &NormalizedInputEvent::DeviceLost,
            false,
        );
        owner.route_input(&reset, true);
        input.generation = Some(owner.generation());
        input.physical_pointer = None;
        return;
    }
    let modal = !ui.guard.allows_focus(None);
    let mut editing = focus.get().is_some_and(|e| editors.contains(e));
    let composer = focus
        .get()
        .is_some_and(|entity| editors.get(entity).is_ok_and(|marker| marker.is_some()));
    for event in raw.read() {
        let events = match input.adapter.process_window_event(&event.event) {
            Ok(events) => events,
            Err(_) => input.adapter.notify_device_lost(),
        };
        for event in events {
            if let NormalizedInputEvent::PointerMoved { position, .. } = &event {
                input.physical_pointer = Some(*position);
            }
            let physical = input
                .physical_pointer
                .map(|position| Vec2::new(position.x().get() as f32, position.y().get() as f32));
            // Pointer ownership decides whether a mouse gesture may leave an editor.
            // Active item drops keep this broad permission even outside the world.
            let allowed = input.adapter.focused() && !modal && !owner.modal_active();
            let consumed = capture.before(&owner, *ui.viewport, physical, &event, allowed);
            let world_owned =
                physical.is_some_and(|point| ui.hits.allows_world_start(ui.regions.world, point));
            let blocked = !input.adapter.focused() || modal || consumed;
            match event {
                NormalizedInputEvent::PointerMoved { position, motion } => {
                    if consumed || !world_owned {
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
                    if consumed || !world_owned {
                        owner.route_input(&[NormalizedInputEvent::DeviceLost], true);
                    } else if let Some(position) = raw_position
                        .and_then(|position| map_pointer(&owner, *ui.viewport, position))
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
                                event.clone(),
                            ],
                            !input.adapter.focused() || !ui.guard.allows(ui.regions.world),
                        );
                    } else {
                        owner.route_input(&[NormalizedInputEvent::DeviceLost], true);
                    }
                    capture.after_press(
                        &owner,
                        *ui.viewport,
                        physical,
                        &event,
                        allowed,
                        world_owned && !consumed,
                    );
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
                    route_keyboard(&mut owner, event, blocked, editing.then_some(composer));
                }
            }
        }
    }
}
pub(super) fn route_keyboard(
    owner: &mut HostOwner,
    event: NormalizedInputEvent,
    blocked: bool,
    editor: Option<bool>,
) {
    use oteryn_input_actions::Modifier;
    let shortcut = matches!(&event, NormalizedInputEvent::Key { code, modifiers, .. }
        if !matches!(code.get(), 79..=82)
            && ((modifiers.contains(Modifier::Control)
                    && !matches!(code.get(), 4 | 6 | 25 | 27..=29))
                || modifiers.contains(Modifier::Alt)
                || modifiers.contains(Modifier::Super)
                || matches!(code.get(), 58..=69 | 104..=115)));
    owner.route_input(
        &[event],
        blocked || editor.is_some_and(|composer| !composer || !shortcut),
    );
}
// Keep scene pixels for fixed controls; PlayView::click_tile reverses world scrolling.
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

/// Ctrl+Alt+Z opens the explored-world overview. Exact modifiers: F10 belongs to
/// audio mute and Ctrl+Alt+Shift+F10 to notification banners.
pub(super) fn satellite_key(events: &[NormalizedInputEvent]) -> bool {
    use oteryn_input_actions::{ButtonState, Modifier, Modifiers};
    let chord = Modifiers::one(Modifier::Control).with(Modifier::Alt);
    events.iter().any(|event| {
        matches!(event, NormalizedInputEvent::Key { code, state: ButtonState::Pressed, modifiers, repeat: false }
            if code.get() == 29 && *modifiers == chord)
    })
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
    #[test]
    fn focused_composer_preserves_help_channel_shortcut() -> Result<(), Box<dyn std::error::Error>>
    {
        use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers};
        let mut owner = HostOwner::new(None)?;
        let (link, mut commands, outbox) = crate::play::play_channel();
        crate::play::feature_0007_chat::protocol::bevy_chat_fixture(
            &outbox,
            Default::default(),
            vec![],
        );
        owner.play = Some(super::super::owner::ActivePlay {
            view: crate::play::PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?,
            link,
        });
        owner.phase = super::super::owner::Phase::Playing;
        route_keyboard(
            &mut owner,
            NormalizedInputEvent::Key {
                code: KeyCode::new(23)?,
                state: ButtonState::Pressed,
                modifiers: Modifiers::one(Modifier::Control),
                repeat: false,
            },
            false,
            Some(true),
        );
        assert!(
            std::iter::from_fn(|| commands.try_recv().ok()).any(|command| matches!(
                command,
                crate::combat_input::PlayCommand::ChatPublic(oteryn_session::ChatIntent::OpenRoom(
                    oteryn_session::ChatRoom::Help
                ))
            ))
        );
        Ok(())
    }
}
