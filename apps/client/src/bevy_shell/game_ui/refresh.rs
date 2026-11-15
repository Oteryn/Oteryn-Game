//! Restore scroll offsets and equivalent focused controls after a projection rebuild.
use bevy::{
    input_focus::{FocusCause, InputFocus},
    prelude::*,
};
use std::sync::{Arc, Mutex, PoisonError};
#[derive(Default)]
struct Snapshot {
    scroll: Vec<ScrollPosition>,
    focus: Option<String>,
}
pub(super) struct Restore(Arc<Mutex<Snapshot>>);
fn signature(world: &World, entity: Entity) -> String {
    if let Some(name) = world.get::<Name>(entity) {
        return name.as_str().to_owned();
    }
    let mut value = String::new();
    if let Some(text) = world.get::<Text>(entity) {
        value.push_str(&text.0);
    }
    if let Some(children) = world.get::<Children>(entity) {
        for child in children.iter() {
            value.push_str(&signature(world, child));
        }
    }
    value
}
fn capture(world: &World, entity: Entity, focused: Option<Entity>, snapshot: &mut Snapshot) {
    if world.get::<bevy::ui_widgets::ScrollArea>(entity).is_some()
        && let Some(scroll) = world.get::<ScrollPosition>(entity)
    {
        snapshot.scroll.push(scroll.clone());
    }
    if focused == Some(entity) {
        snapshot.focus = Some(signature(world, entity));
    }
    if let Some(children) = world.get::<Children>(entity) {
        for child in children.iter() {
            capture(world, child, focused, snapshot);
        }
    }
}
fn descendants(world: &World, entity: Entity, result: &mut Vec<Entity>) {
    result.push(entity);
    if let Some(children) = world.get::<Children>(entity) {
        for child in children.iter() {
            descendants(world, child, result);
        }
    }
}
pub(super) fn preserve(commands: &mut Commands, root: Entity, focused: Option<Entity>) -> Restore {
    let snapshot = Arc::new(Mutex::new(Snapshot::default()));
    let saved = Arc::clone(&snapshot);
    commands.queue(move |world: &mut World| {
        let focused =
            focused.or_else(|| world.get_resource::<InputFocus>().and_then(InputFocus::get));
        capture(
            world,
            root,
            focused,
            &mut saved.lock().unwrap_or_else(PoisonError::into_inner),
        );
    });
    Restore(snapshot)
}
impl Restore {
    pub(super) fn restore(self, commands: &mut Commands, root: Entity) {
        commands.queue(move |world: &mut World| {
            let snapshot = self.0.lock().unwrap_or_else(PoisonError::into_inner);
            let mut entities = Vec::new();
            descendants(world, root, &mut entities);
            let scrollers: Vec<_> = entities
                .iter()
                .copied()
                .filter(|entity| world.get::<bevy::ui_widgets::ScrollArea>(*entity).is_some())
                .collect();
            for (entity, scroll) in scrollers.into_iter().zip(&snapshot.scroll) {
                world.entity_mut(entity).insert(scroll.clone());
            }
            if let Some(expected) = &snapshot.focus
                && let Some(entity) = entities.into_iter().find(|entity| {
                    (world
                        .get::<bevy::input_focus::tab_navigation::TabIndex>(*entity)
                        .is_some()
                        || world.get::<bevy::ui_widgets::Button>(*entity).is_some()
                        || world.get::<bevy::text::EditableText>(*entity).is_some())
                        && signature(world, *entity) == *expected
                })
                && let Some(mut focus) = world.get_resource_mut::<InputFocus>()
            {
                focus.set(entity, FocusCause::Navigated);
            }
        });
    }
}
