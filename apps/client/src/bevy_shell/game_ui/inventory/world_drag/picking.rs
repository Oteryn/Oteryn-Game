//! Picking supplies the hit and per-press identity; the normalized bridge submits commands.
use super::*;
use crate::bevy_shell::input::hits::WorldPointerHits;
use bevy::{
    picking::{
        events::{PointerDragDrop, PointerPress},
        pointer::{PointerButton, PointerId},
    },
    window::PrimaryWindow,
};
#[derive(Clone, Copy)]
pub(super) enum Source {
    World,
    Carried(Slot),
}
#[derive(Clone, Copy)]
pub(super) struct Press {
    pub token: u64,
    pub source: Source,
    pub point: Vec2,
    entity: Entity,
    frame: u32,
    generation: u64,
}
#[derive(Clone, Copy)]
pub(super) enum Destination {
    World,
    Slot(Slot),
}
#[derive(Default)]
pub(super) struct Ledger {
    next: u64,
    current: Option<Press>,
    presses: Vec<Press>,
    drops: Vec<(u64, u32, Destination)>,
}
#[allow(clippy::type_complexity)]
type Tree<'w, 's> = Query<
    'w,
    's,
    (
        Option<&'static ChildOf>,
        Has<WorldRegion>,
        Option<&'static Slot>,
        Has<bevy::ui_widgets::Button>,
        Has<bevy::ui_widgets::TextInput>,
    ),
>;
fn source(mut entity: Entity, tree: &Tree) -> Option<(Source, Entity)> {
    loop {
        let (parent, world, slot, button, editor) = tree.get(entity).ok()?;
        if let Some(slot) = slot {
            return Some((Source::Carried(*slot), entity));
        }
        if button || editor {
            return None;
        }
        if world {
            return Some((Source::World, entity));
        }
        entity = parent?.parent();
    }
}
impl Ledger {
    fn reject_press(&mut self) {
        self.current = None;
        self.presses.clear();
    }
    #[allow(clippy::too_many_arguments)]
    pub(super) fn press(
        &mut self,
        frame: u32,
        generation: u64,
        entity: Entity,
        source: Source,
        point: Vec2,
    ) {
        self.presses.retain(|press| press.frame == frame);
        self.drops.retain(|(_, at, _)| *at == frame);
        let Some(token) = self.next.checked_add(1) else {
            self.current = None;
            return;
        };
        self.next = token;
        let press = Press {
            token,
            source,
            point,
            entity,
            frame,
            generation,
        };
        self.current = Some(press);
        self.presses.push(press);
    }
    pub(super) fn claim(&mut self, frame: u32, generation: u64, point: Vec2) -> Option<Press> {
        let index = self.presses.iter().position(|press| {
            press.frame == frame
                && press.generation == generation
                && press.point.round() == point.round()
        })?;
        Some(self.presses.remove(index))
    }
    pub(super) fn drop_on(
        &mut self,
        frame: u32,
        generation: u64,
        dropped: Entity,
        destination: Destination,
    ) {
        let Some(press) = self
            .current
            .filter(|press| press.generation == generation && press.entity == dropped)
        else {
            return;
        };
        self.drops.retain(|(_, at, _)| *at == frame);
        if !self.drops.iter().any(|(token, _, _)| *token == press.token) {
            self.drops.push((press.token, frame, destination));
        }
    }
    pub(super) fn take_drop(&mut self, frame: u32, token: u64) -> Option<Destination> {
        let index = self
            .drops
            .iter()
            .position(|(id, at, _)| *id == token && *at == frame)?;
        Some(self.drops.remove(index).2)
    }
}
pub(super) fn press(
    event: On<PointerPress>,
    owner: NonSend<HostOwner>,
    tree: Tree,
    geometry: Query<(
        &ComputedNode,
        &UiGlobalTransform,
        Option<&bevy::ui::CalculatedClip>,
    )>,
    windows: Query<(Entity, &Window), With<PrimaryWindow>>,
    mut gesture: ResMut<Gesture>,
    frame: Res<FrameCount>,
    hits: WorldPointerHits,
) {
    if event.entity != event.original_event_target()
        || event.pointer.id != PointerId::Mouse
        || event.button != PointerButton::Primary
    {
        return;
    }
    let Some((source, node)) = source(event.original_event_target(), &tree) else {
        gesture.picking.reject_press();
        return;
    };
    let Ok((entity, window)) = windows.single() else {
        gesture.picking.reject_press();
        return;
    };
    if !window.focused
        || bevy::window::WindowRef::Entity(entity)
            .normalize(None)
            .map(bevy::camera::NormalizedRenderTarget::Window)
            .as_ref()
            != Some(&event.pointer.target)
    {
        gesture.picking.reject_press();
        return;
    }
    let point = event.pointer.position * window.scale_factor();
    let inside = geometry.get(node).is_ok_and(|(node, transform, clip)| {
        node.contains_point(*transform, point) && clip.is_none_or(|clip| clip.contains_point(point))
    });
    if point.is_finite() && inside && hits.matches_primary_pointer(point) {
        gesture
            .picking
            .press(frame.0, owner.generation(), event.entity, source, point);
    } else {
        gesture.picking.reject_press();
    }
}
pub(super) fn world_drop(
    event: On<PointerDragDrop>,
    guard: ModalGuard,
    owner: NonSend<HostOwner>,
    tree: Tree,
    mut gesture: ResMut<Gesture>,
    frame: Res<FrameCount>,
) {
    if guard.allows(event.entity)
        && matches!(
            source(event.original_event_target(), &tree),
            Some((Source::World, _))
        )
        && event.pointer.id == PointerId::Mouse
        && event.button == PointerButton::Primary
    {
        gesture.picking.drop_on(
            frame.0,
            owner.generation(),
            event.dropped,
            Destination::World,
        );
    }
}
#[derive(SystemParam)]
pub(in crate::bevy_shell::game_ui::inventory) struct DropCapture<'w> {
    gesture: Option<ResMut<'w, Gesture>>,
    frame: Option<Res<'w, FrameCount>>,
}
impl DropCapture<'_> {
    pub(in crate::bevy_shell::game_ui::inventory) fn admits_source(
        &self,
        slot: Slot,
        pointer: PointerId,
        original: Entity,
    ) -> bool {
        self.gesture.as_ref().is_none_or(|gesture| {
            gesture.picking.current.is_some_and(|press| {
                pointer == PointerId::Mouse
                    && press.entity == original
                    && matches!(press.source,Source::Carried(current)if current==slot)
            })
        })
    }
    pub(in crate::bevy_shell::game_ui::inventory) fn matches_pointer(
        &self,
        pointer: PointerId,
    ) -> bool {
        self.gesture.is_none() || pointer == PointerId::Mouse
    }
    pub(in crate::bevy_shell::game_ui::inventory) fn slot(
        &mut self,
        event: &PointerDragDrop,
        owner: &HostOwner,
        target: Slot,
    ) {
        if event.pointer.id == PointerId::Mouse && event.button == PointerButton::Primary {
            let (Some(gesture), Some(frame)) = (&mut self.gesture, &self.frame) else {
                return;
            };
            gesture.picking.drop_on(
                frame.0,
                owner.generation(),
                event.dropped,
                Destination::Slot(target),
            );
        }
    }
    pub(in crate::bevy_shell::game_ui::inventory) fn completed(&mut self) {
        if let (Some(gesture), Some(frame)) = (&mut self.gesture, &self.frame) {
            gesture.completed = Some(frame.0);
        }
    }
}
