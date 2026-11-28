//! Ordinary world starts consume Bevy's resolved Mouse ownership, never a second UI hit test.
use bevy::{
    ecs::system::SystemParam,
    picking::{
        hover::HoverMap,
        pointer::{PointerId, PointerLocation},
    },
    prelude::*,
    ui_widgets::{Button, Checkbox, TextInput},
    window::PrimaryWindow,
};
type UiControls = Or<(With<Button>, With<Checkbox>, With<TextInput>)>;

type TouchGeometry = (
    &'static ComputedNode,
    &'static UiGlobalTransform,
    &'static InheritedVisibility,
    Option<&'static bevy::ui::CalculatedClip>,
    Option<&'static Pickable>,
);

#[derive(SystemParam)]
pub(in crate::bevy_shell) struct WorldPointerHits<'w, 's> {
    stack: Option<Res<'w, bevy::ui::UiStack>>,
    geometry: Query<'w, 's, TouchGeometry>,
    hover: Res<'w, HoverMap>,
    pointers: Query<'w, 's, (&'static PointerId, &'static PointerLocation)>,
    primary: Query<'w, 's, (Entity, &'static Window), With<PrimaryWindow>>,
    controls: Query<'w, 's, (), UiControls>,
    parents: Query<'w, 's, &'static ChildOf>,
}
impl WorldPointerHits<'_, '_> {
    /// Compare at the native normalizer's physical-pixel precision. A frame-final
    /// pointer cannot establish ownership for an earlier, different press position.
    pub fn matches_primary_pointer(&self, physical: Vec2) -> bool {
        self.pointer_matches(PointerId::Mouse, physical) == Some(true)
    }
    fn pointer_matches(&self, pointer_id: PointerId, physical: Vec2) -> Option<bool> {
        let Ok((entity, window)) = self.primary.single() else {
            return None;
        };
        let scale = window.scale_factor();
        if !physical.is_finite()
            || !scale.is_finite()
            || scale <= 0.
            || !physical.cmpge(Vec2::ZERO).all()
            || !physical
                .cmplt(Vec2::new(
                    window.physical_width() as f32,
                    window.physical_height() as f32,
                ))
                .all()
        {
            return None;
        }
        let mut pointers = self.pointers.iter().filter(|(id, _)| **id == pointer_id);
        let (_, pointer) = pointers.next()?;
        if pointers.next().is_some() {
            return None;
        }
        let location = pointer.location()?;
        let target = bevy::window::WindowRef::Entity(entity).normalize(None)?;
        let current = location.position * scale;
        (location.target == bevy::camera::NormalizedRenderTarget::Window(target)
            && current.is_finite())
        .then(|| current.round() == physical.round())
    }
    /// Only a resolved world hit may start ordinary world input or a ground-item
    /// candidate. Active item releases use their captured upstream Slot/World token
    /// instead, so they remain valid when the pointer leaves the world rectangle.
    pub fn allows_world_start(&self, world: Entity, physical: Vec2) -> bool {
        self.allows_pointer_start(PointerId::Mouse, world, physical)
    }
    pub fn allows_pointer_start(&self, id: PointerId, world: Entity, physical: Vec2) -> bool {
        let Some(matches) = self.pointer_matches(id, physical) else {
            return false;
        };
        if !matches {
            // Touch input may include several positions before picking publishes the last.
            // Resolve the initial position against the actual rendered UI stack.
            if matches!(id, PointerId::Touch(_))
                && let Some(stack) = &self.stack
            {
                let Some(nodes) = stack
                    .partition
                    .iter()
                    .map(|range| &stack.uinodes[range.clone()])
                    .find(|nodes| nodes.contains(&world))
                else {
                    return false;
                };
                for entity in nodes.iter().rev() {
                    let Ok((node, transform, visibility, clip, pickable)) =
                        self.geometry.get(*entity)
                    else {
                        continue;
                    };
                    if !visibility.get()
                        || node.size() == Vec2::ZERO
                        || !node.contains_point(*transform, physical)
                        || clip.is_some_and(|clip| !clip.contains_point(physical))
                        || pickable.is_some_and(|p| !p.is_hoverable && !p.should_block_lower)
                    {
                        continue;
                    }
                    return self.owned_by_world(*entity, world)
                        && pickable.is_none_or(|p| p.is_hoverable);
                }
            }
            return false;
        }
        let Some(hits) = self.hover.get(&id) else {
            return false;
        };
        !hits.is_empty()
            && hits
                .keys()
                .all(|entity| self.owned_by_world(*entity, world))
    }
    fn owned_by_world(&self, mut current: Entity, world: Entity) -> bool {
        loop {
            if current == world {
                return true;
            }
            if self.controls.contains(current) {
                return false;
            }
            let Ok(parent) = self.parents.get(current) else {
                return false;
            };
            current = parent.parent();
        }
    }
}
#[cfg(test)]
mod tests;

#[cfg(any(test, feature = "qualification-i09"))]
mod qualification;
#[cfg(feature = "qualification-i09")]
pub(in crate::bevy_shell) use qualification::{qualification_input, qualification_right_click};
