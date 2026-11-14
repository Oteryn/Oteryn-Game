//! Physical-pointer bridge over the existing item movement owner, shared by native input tests.
use super::super::{GameRegions, WorldRegion, viewport::WorldViewport};
use super::*;
use crate::play::GroundItem;
mod picking;
use bevy::{diagnostic::FrameCount, ecs::system::SystemParam};
use oteryn_input_actions::{ButtonState, Modifiers, MouseButton, NormalizedInputEvent};
use oteryn_renderer::TileCoord;
pub(super) use picking::DropCapture;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct GroundSource {
    generation: u64,
    tile: TileCoord,
    binding: GroundItem,
    item: DragItem,
}
impl GroundSource {
    fn current(self, owner: &HostOwner) -> Option<DragItem> {
        if self.generation != owner.generation() || owner.modal_active() {
            return None;
        }
        if !world_input_allowed(owner) {
            return None;
        }
        let play = owner.play.as_ref()?;
        let current = play.view.item_drag_binding(self.tile)?;
        if !same_binding(self.binding, current) {
            return None;
        }
        play.view
            .item_drag_source(self.tile)
            .filter(|item| item.handle == self.item.handle)
    }
}
/// Use the existing world gameplay policy without restricting ordinary inventory capabilities.
pub(super) fn world_input_allowed(owner: &HostOwner) -> bool {
    owner
        .play
        .as_ref()
        .is_some_and(|play| play.view.item_world_gesture_eligible(&play.link))
}
fn same_binding(a: GroundItem, b: GroundItem) -> bool {
    match (a, b) {
        (
            GroundItem::Map {
                header: a,
                position: ap,
                item: ai,
            },
            GroundItem::Map {
                header: b,
                position: bp,
                item: bi,
            },
        ) => {
            a == b
                && ap == bp
                && ai.definition == bi.definition
                && ai.sub_type == bi.sub_type
                && ai.origin == bi.origin
                && ai.appearance_id == bi.appearance_id
        }
        (
            GroundItem::Entity {
                reference: a,
                position: ap,
                definition: ad,
                handle: ah,
                ..
            },
            GroundItem::Entity {
                reference: b,
                position: bp,
                definition: bd,
                handle: bh,
                ..
            },
        ) => a == b && ap == bp && ad == bd && ah == bh,
        _ => false,
    }
}
#[derive(Clone, Copy)]
struct Candidate {
    source: GroundSource,
    start: Vec2,
    origin: UVec2,
    size: UVec2,
    token: u64,
}
#[derive(Resource, Default)]
pub(super) struct Gesture {
    candidate: Option<Candidate>,
    active: bool,
    completed: Option<u32>,
    picking: picking::Ledger,
    carried_token: Option<(u64, Slot)>,
}
/// Root input owns ordering: before every event, after_press after ordinary press routing.
#[derive(SystemParam)]
pub(in crate::bevy_shell) struct ItemPointerCapture<'w> {
    ui: ResMut<'w, InventoryUi>,
    gesture: ResMut<'w, Gesture>,
    frame: Res<'w, FrameCount>,
}
impl ItemPointerCapture<'_> {
    /// `allowed` is focus/modal/editor permission, independent of cursor location.
    pub fn before(
        &mut self,
        owner: &HostOwner,
        viewport: WorldViewport,
        point: Option<Vec2>,
        event: &NormalizedInputEvent,
        allowed: bool,
    ) -> bool {
        before(
            &mut self.ui,
            &mut self.gesture,
            self.frame.0,
            owner,
            viewport,
            point,
            event,
            allowed,
        )
    }
    pub fn after_press(
        &mut self,
        owner: &HostOwner,
        viewport: WorldViewport,
        point: Option<Vec2>,
        event: &NormalizedInputEvent,
        allowed: bool,
        allowed_world_start: bool,
    ) {
        after_press(
            &mut self.ui,
            &mut self.gesture,
            self.frame.0,
            owner,
            viewport,
            point,
            event,
            allowed,
            allowed_world_start,
        );
    }
}
pub(super) fn install(app: &mut App) {
    app.init_resource::<Gesture>()
        .init_resource::<FrameCount>()
        .add_systems(Startup, setup.after(super::super::setup))
        .add_observer(picking::press);
}
fn setup(mut commands: Commands, regions: Res<GameRegions>) {
    commands.entity(regions.world).observe(picking::world_drop);
}
fn tile(owner: &HostOwner, viewport: WorldViewport, point: Option<Vec2>) -> Option<TileCoord> {
    let view = &owner.play.as_ref()?.view;
    let (w, h) = view.scene().view().viewport_px();
    let local = viewport.scene_point(point?, UVec2::new(w, h))?;
    crate::input::click_tile(view.scene().view(), f64::from(local.x), f64::from(local.y))
}
fn reset(ui: &mut InventoryUi, gesture: &mut Gesture, owner: &HostOwner) {
    if (gesture.active || ui.dragged.is_some())
        && let Some(play) = &owner.play
    {
        play.link.item_drag_abort();
    }
    ui.dragged = None;
    gesture.active = false;
    gesture.candidate = None;
    gesture.carried_token = None;
}
// The same event, retained owner and separate broad/world permissions must share one snapshot.
#[allow(clippy::too_many_arguments)]
fn after_press(
    ui: &mut InventoryUi,
    gesture: &mut Gesture,
    frame: u32,
    owner: &HostOwner,
    viewport: WorldViewport,
    point: Option<Vec2>,
    event: &NormalizedInputEvent,
    allowed: bool,
    allowed_world_start: bool,
) {
    if !matches!(
        event,
        NormalizedInputEvent::MouseButton {
            button: MouseButton::PRIMARY,
            state: ButtonState::Pressed,
            modifiers: Modifiers::NONE
        }
    ) {
        return;
    }
    if !allowed
        || gesture.completed == Some(frame)
        || gesture.active
        || owner.modal_active()
        || ui.prompt.is_some()
        || ui.menu.is_some()
    {
        return;
    }
    let Some(point) = point.filter(|point| point.is_finite()) else {
        return;
    };
    let Some(press) = gesture.picking.claim(frame, owner.generation(), point) else {
        return;
    };
    if let picking::Source::Carried(slot) = press.source {
        gesture.carried_token = Some((press.token, slot));
        return;
    }
    if !allowed_world_start || !world_input_allowed(owner) || ui.dragged.is_some() {
        return;
    }
    let Some(play) = &owner.play else {
        return;
    };
    if play.link.item_movement().pending || play.link.item_movement().prompt.is_some() {
        return;
    }
    let Some(tile) = tile(owner, viewport, Some(point)) else {
        return;
    };
    let (Some(binding), Some(item)) = (
        play.view.item_drag_binding(tile),
        play.view.item_drag_source(tile),
    ) else {
        return;
    };
    let start = point;
    let source = GroundSource {
        generation: owner.generation(),
        tile,
        binding,
        item,
    };
    gesture.candidate = Some(Candidate {
        source,
        start,
        origin: viewport.origin,
        size: viewport.size,
        token: press.token,
    });
}
// Matches the retained native bridge: four physical pixels on either axis, not UI scale units.
fn moved(start: Vec2, end: Vec2) -> bool {
    (end - start).abs().max_element() >= 4.
}
// Frame identity and both Bevy resources are checked together before routing this native event.
#[allow(clippy::too_many_arguments)]
fn before(
    ui: &mut InventoryUi,
    gesture: &mut Gesture,
    frame: u32,
    owner: &HostOwner,
    viewport: WorldViewport,
    point: Option<Vec2>,
    event: &NormalizedInputEvent,
    allowed: bool,
) -> bool {
    let captured = gesture.active || ui.dragged.is_some() || gesture.completed == Some(frame);
    // The ordinary press may consume a cursor spell before after_press runs.
    // Revoke its world token while the existing mode still owns this exact click.
    if matches!(
        event,
        NormalizedInputEvent::MouseButton {
            button: MouseButton::PRIMARY,
            state: ButtonState::Pressed,
            ..
        }
    ) && !world_input_allowed(owner)
        && let Some(point) = point.filter(|point| point.is_finite())
    {
        gesture
            .picking
            .revoke_world_press(frame, owner.generation(), point);
    }

    let reset_event = matches!(
        event,
        NormalizedInputEvent::DeviceLost
            | NormalizedInputEvent::FocusChanged { focused: false }
            | NormalizedInputEvent::CaptureChanged { captured: false }
            | NormalizedInputEvent::Key {
                code: oteryn_input_actions::KeyCode::ESCAPE,
                state: ButtonState::Pressed,
                ..
            }
    );
    let valid = point.is_none_or(Vec2::is_finite)
        && gesture.candidate.is_none_or(|candidate| {
            candidate.source.current(owner).is_some()
                && candidate.origin == viewport.origin
                && candidate.size == viewport.size
        });
    if reset_event || !allowed || owner.modal_active() || !valid {
        reset(ui, gesture, owner);
        return captured;
    }
    if matches!(event, NormalizedInputEvent::PointerMoved { .. })
        && !gesture.active
        && let (Some(candidate), Some(point)) = (gesture.candidate, point)
        && moved(candidate.start, point)
        && let Some(item) = candidate.source.current(owner)
        && let Some(play) = &owner.play
        && play.link.item_drag_begin(item)
    {
        gesture.active = true;
        ui.ground_source = Some(candidate.source);
    }
    if matches!(
        event,
        NormalizedInputEvent::MouseButton {
            button: MouseButton::PRIMARY,
            state: ButtonState::Released,
            ..
        }
    ) {
        let active = gesture.active || ui.dragged.is_some();
        if active {
            let token = if gesture.active {
                gesture.candidate.map(|candidate| candidate.token)
            } else {
                gesture
                    .carried_token
                    .filter(|(_, slot)| Some(*slot) == ui.dragged)
                    .map(|(token, _)| token)
            };
            let destination = token
                .and_then(|token| gesture.picking.take_drop(frame, token))
                .and_then(|destination| match destination {
                    picking::Destination::Slot(slot) => slot.current(owner).and(slot.destination),
                    picking::Destination::World => tile(owner, viewport, point).and_then(|tile| {
                        owner.play.as_ref().map(|play| {
                            ItemMoveDestination::Ground(
                                oteryn_session::item_movement::WorldTilePosition {
                                    x: tile.x,
                                    y: tile.y,
                                    floor: play.view.item_floor(),
                                },
                            )
                        })
                    }),
                });
            let destination = destination.filter(|destination| {
                !matches!(destination, ItemMoveDestination::Ground(_)) || world_input_allowed(owner)
            });
            let item = if gesture.active {
                gesture
                    .candidate
                    .and_then(|candidate| candidate.source.current(owner))
            } else {
                ui.dragged.and_then(|slot| slot.drag(owner))
            };
            if let Some(play) = &owner.play {
                if let Some(item) = item
                    && destination.is_some()
                    && play.link.item_drag_begin(item)
                {
                    play.link.item_drag_drop(destination);
                } else {
                    play.link.item_drag_abort();
                }
            }
            gesture.active = false;
            gesture.candidate = None;
            gesture.carried_token = None;
            ui.dragged = None;
            gesture.completed = Some(frame);
            return true;
        }
        gesture.candidate = None;
        gesture.carried_token = None;
    }
    gesture.active || ui.dragged.is_some() || gesture.completed == Some(frame)
}
/// A ground prompt keeps its original metadata, rather than manufacturing a carried entry.
pub(super) fn prompt_maximum(owner: &HostOwner, ui: &InventoryUi, prompt: Prompt) -> Option<u32> {
    let source = ui.ground_source?;
    if source.item.handle != prompt.item.handle || source.generation != prompt.generation {
        return None;
    }
    Some(source.current(owner)?.count.min(prompt.item.count).min(100))
}
#[cfg(test)]
mod tests;
