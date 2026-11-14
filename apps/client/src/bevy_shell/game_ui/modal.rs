//! One hierarchy-derived UI fence shared by normalized input and native widget observers.
use super::GameModal;
use bevy::{
    ecs::system::SystemParam,
    input_focus::InputFocus,
    prelude::*,
    ui::{ComputedStackIndex, UiStack, UiSystems},
};
use std::{
    cmp::Ordering,
    sync::{Mutex, PoisonError},
};

// Optional ancestry fields keep the guard independent of computed layout and render plugins.
#[allow(clippy::type_complexity)]
type Tree<'w, 's> = Query<
    'w,
    's,
    (
        Option<&'static Node>,
        Option<&'static Visibility>,
        Option<&'static ChildOf>,
        Option<&'static GlobalZIndex>,
        Option<&'static ZIndex>,
        Option<&'static Children>,
        Option<&'static ComputedStackIndex>,
    ),
>;

/// Reads the currently visible modal hierarchy; it never owns gameplay or modal state.
#[derive(SystemParam)]
pub(in crate::bevy_shell) struct ModalGuard<'w, 's> {
    dom: ModalTree<'w, 's>,
    frame: Option<Res<'w, FrameModal>>,
}
#[derive(SystemParam)]
struct ModalTree<'w, 's> {
    modals: Query<'w, 's, Entity, With<GameModal>>,
    tree: Tree<'w, 's>,
    stack: Option<Res<'w, UiStack>>,
}
#[derive(Resource, Default)]
struct FrameModal(Mutex<Option<Snapshot>>);
#[derive(Clone)]
struct Snapshot {
    entity: Entity,
    layer: Layer,
}
#[derive(Clone)]
struct Layer {
    global: i32,
    paint: Option<u32>,
    path: Vec<(i32, u64)>,
    depth: usize,
}
impl ModalGuard<'_, '_> {
    /// A widget may act only inside the frontmost visible modal, when one exists.
    pub fn allows(&self, entity: Entity) -> bool {
        if !self.dom.visible(entity) {
            return false;
        }
        let Some(active) = self.active() else {
            return true;
        };
        // A closed/despawned cached top remains a fence until the frame ends.
        self.dom.layer(active.entity).is_some() && self.dom.allows(entity, &active)
    }
    /// Unfocused global handlers cannot run through an active modal.
    pub fn allows_focus(&self, entity: Option<Entity>) -> bool {
        match entity {
            Some(entity) => self.allows(entity),
            None => self.active().is_none(),
        }
    }
    fn active(&self) -> Option<Snapshot> {
        let current = self.dom.frontmost();
        let Some(frame) = &self.frame else {
            return current;
        };
        let mut cached = frame.0.lock().unwrap_or_else(PoisonError::into_inner);
        if current.as_ref().is_some_and(|current| {
            cached.as_ref().is_none_or(|previous| {
                if current.layer.global <= previous.layer.global
                    && !self.dom.visible(previous.entity)
                {
                    return false;
                }
                // A not-yet-painted peer cannot displace an already painted top based
                // on fallback entity order. Wait for Bevy's stack at frame end; a new
                // higher global layer or a nested child can still tighten immediately.
                if current.layer.global == previous.layer.global
                    && previous.layer.paint.is_some()
                    && current.layer.paint.is_none()
                    && !self.dom.descends_from(current.entity, previous.entity)
                {
                    return false;
                }
                self.dom.compare(current, previous) == Ordering::Greater
            })
        }) {
            cached.clone_from(&current);
        }
        cached.clone()
    }
}
impl ModalTree<'_, '_> {
    fn allows(&self, entity: Entity, active: &Snapshot) -> bool {
        if self.descends_from(entity, active.entity) {
            return true;
        }
        // Inventory context menus mark both the scrim and panel. Their own ancestor
        // scrim must still dismiss them; unrelated background controls stay fenced.
        self.modals.contains(entity)
            && self.descends_from(active.entity, entity)
            && self
                .layer(entity)
                .is_some_and(|candidate| candidate.global == active.layer.global)
    }
    fn frontmost(&self) -> Option<Snapshot> {
        let mut candidates = self
            .modals
            .iter()
            .filter_map(|entity| self.layer(entity).map(|layer| Snapshot { entity, layer }))
            .collect::<Vec<_>>();
        // Use one ordering for the entire candidate set. New nodes are not in UiStack
        // until PostUpdate; mixing painted and unpainted comparisons is not transitive.
        if candidates
            .iter()
            .any(|candidate| candidate.layer.paint.is_none())
        {
            for candidate in &mut candidates {
                candidate.layer.paint = None;
            }
        }
        candidates
            .into_iter()
            .max_by(|left, right| self.compare(left, right))
    }
    fn compare(&self, left: &Snapshot, right: &Snapshot) -> Ordering {
        left.layer
            .global
            .cmp(&right.layer.global)
            .then_with(|| match (left.layer.paint, right.layer.paint) {
                (Some(left), Some(right)) => left.cmp(&right),
                _ => Ordering::Equal,
            })
            .then(left.layer.path.cmp(&right.layer.path))
            .then(left.layer.depth.cmp(&right.layer.depth))
            .then(left.entity.index_u32().cmp(&right.entity.index_u32()))
            .then(left.entity.to_bits().cmp(&right.entity.to_bits()))
    }
    fn layer(&self, entity: Entity) -> Option<Layer> {
        if !self.visible(entity) || self.tree.get(entity).ok()?.0.is_none() {
            return None;
        }
        let mut chain = Vec::new();
        let mut next = Some(entity);
        while let Some(current) = next {
            let (_, _, parent, global, z, _, _) = self.tree.get(current).ok()?;
            chain.push((
                current,
                parent.map(ChildOf::parent),
                global.map(|z| z.0),
                z.map_or(0, |z| z.0),
            ));
            next = parent.map(ChildOf::parent);
        }
        // GlobalZIndex escapes its ancestors' draw ordering; descendants inherit the
        // nearest explicit global layer. Local siblings retain their hierarchy order.
        let global = chain
            .iter()
            .find_map(|(_, _, global, _)| *global)
            .unwrap_or(0);
        // Repeated GlobalZIndex on a nested panel must retain the same ancestry prefix
        // as its scrim. Resetting at every explicit component would make comparisons
        // between a nested pair and an unrelated equal-layer modal non-transitive.
        let context = chain
            .iter()
            .rposition(|(_, _, candidate, _)| *candidate == Some(global))
            .unwrap_or(chain.len().saturating_sub(1));
        let path = chain[..=context]
            .iter()
            .rev()
            .map(|(entity, parent, _, z)| {
                let sibling = parent
                    .and_then(|parent| self.tree.get(parent).ok())
                    .and_then(|(_, _, _, _, _, children, _)| children)
                    .and_then(|children| children.iter().position(|child| child == *entity))
                    .map_or_else(|| u64::from(entity.index_u32()), |position| position as u64);
                (*z, sibling)
            })
            .collect();
        let paint = self.tree.get(entity).ok()?.6.and_then(|paint| {
            self.stack
                .as_ref()
                .and_then(|stack| stack.uinodes.get(paint.0 as usize))
                .filter(|painted| **painted == entity)
                .map(|_| paint.0)
        });
        Some(Layer {
            global,
            paint,
            path,
            depth: chain.len(),
        })
    }
    fn visible(&self, entity: Entity) -> bool {
        let mut next = Some(entity);
        while let Some(current) = next {
            let Ok((node, visibility, parent, _, _, _, _)) = self.tree.get(current) else {
                return false;
            };
            if node.is_some_and(|node| node.display == Display::None)
                || visibility.is_some_and(|value| *value == Visibility::Hidden)
            {
                return false;
            }
            next = parent.map(ChildOf::parent);
        }
        true
    }
    fn descends_from(&self, entity: Entity, ancestor: Entity) -> bool {
        let mut next = Some(entity);
        while let Some(current) = next {
            if current == ancestor {
                return true;
            }
            next = self
                .tree
                .get(current)
                .ok()
                .and_then(|(_, _, parent, _, _, _, _)| parent.map(ChildOf::parent));
        }
        false
    }
}

pub(in crate::bevy_shell) fn install(app: &mut App) {
    app.init_resource::<InputFocus>()
        .init_resource::<FrameModal>()
        .add_systems(PostUpdate, finish_frame.after(UiSystems::Stack));
}
fn finish_frame(dom: ModalTree, frame: Res<FrameModal>, mut focus: ResMut<InputFocus>) {
    let actual = dom.frontmost();
    if let Some(entity) = focus.get()
        && (!dom.visible(entity)
            || actual
                .as_ref()
                .is_some_and(|active| !dom.allows(entity, active)))
    {
        focus.clear();
    }
    *frame.0.lock().unwrap_or_else(PoisonError::into_inner) = actual;
}
#[cfg(test)]
mod tests;
