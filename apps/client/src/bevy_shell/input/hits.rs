//! Ordinary world starts consume Bevy's resolved Mouse ownership, never a second UI hit test.
use bevy::{
    ecs::system::SystemParam,
    picking::{
        hover::HoverMap,
        pointer::{PointerId, PointerLocation},
    },
    prelude::*,
    window::PrimaryWindow,
};
#[derive(SystemParam)]
pub(in crate::bevy_shell) struct WorldPointerHits<'w, 's> {
    hover: Res<'w, HoverMap>,
    pointers: Query<'w, 's, (&'static PointerId, &'static PointerLocation)>,
    primary: Query<'w, 's, (Entity, &'static Window), With<PrimaryWindow>>,
    parents: Query<'w, 's, &'static ChildOf>,
}
impl WorldPointerHits<'_, '_> {
    /// Compare at the native normalizer's physical-pixel precision. A frame-final
    /// pointer cannot establish ownership for an earlier, different press position.
    pub fn matches_primary_pointer(&self, physical: Vec2) -> bool {
        let Ok((entity, window)) = self.primary.single() else {
            return false;
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
            return false;
        }
        let mut mice = self
            .pointers
            .iter()
            .filter(|(id, _)| **id == PointerId::Mouse);
        let Some((_, pointer)) = mice.next() else {
            return false;
        };
        if mice.next().is_some() {
            return false;
        }
        let Some(location) = pointer.location() else {
            return false;
        };
        let Some(target) = bevy::window::WindowRef::Entity(entity).normalize(None) else {
            return false;
        };
        let current = location.position * scale;
        location.target == bevy::camera::NormalizedRenderTarget::Window(target)
            && current.is_finite()
            && current.round() == physical.round()
    }
    /// Only a resolved world hit may start ordinary world input or a ground-item
    /// candidate. Active item releases use their captured upstream Slot/World token
    /// instead, so they remain valid when the pointer leaves the world rectangle.
    pub fn allows_world_start(&self, world: Entity, physical: Vec2) -> bool {
        if !self.matches_primary_pointer(physical) {
            return false;
        }
        let Some(hits) = self.hover.get(&PointerId::Mouse) else {
            return false;
        };
        !hits.is_empty()
            && hits.keys().all(|entity| {
                let mut current = *entity;
                loop {
                    if current == world {
                        return true;
                    }
                    let Ok(parent) = self.parents.get(current) else {
                        return false;
                    };
                    current = parent.parent();
                }
            })
    }
}
#[cfg(test)]
mod tests;
