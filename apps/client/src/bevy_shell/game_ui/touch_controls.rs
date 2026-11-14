//! Native touch uses the retained overlay and gameplay controls, without a new protocol.
use super::{
    super::{owner::HostOwner, ui::Skins},
    viewport::WorldViewport,
};
use crate::play::TouchPhase;
use bevy::{prelude::*, ui_widgets::Activate};
use oteryn_client_ui_bevy as source;
use std::collections::HashMap;

#[derive(Resource, Default)]
struct State {
    snapshot: Option<u64>,
    root: Option<Entity>,
}
#[derive(Component)]
struct Close(u64);

pub(super) fn install(app: &mut App) {
    app.init_resource::<State>()
        .init_resource::<Gestures>()
        .add_systems(
            Update,
            super::super::input::route_touch.after(super::super::input::route),
        )
        .add_systems(Update, (keyboard, project).chain())
        .add_systems(Last, cancel_blocked);
}
pub(in crate::bevy_shell) fn open(owner: &mut HostOwner) {
    let generation = owner.generation();
    visibility(owner, generation, false, true);
}
fn visibility(owner: &mut HostOwner, generation: u64, expected: bool, visible: bool) -> bool {
    if generation != owner.generation() || (visible && owner.modal_active()) {
        return false;
    }
    owner.play.as_mut().is_some_and(|play| {
        play.view
            .touch_controls_visibility_bound(&play.link, expected, visible)
    })
}
fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skins: Res<Skins>,
    mut state: ResMut<State>,
) {
    let current = owner
        .play
        .as_ref()
        .filter(|p| p.view.touch_controls_visible())
        .map(|_| owner.generation());
    if current == state.snapshot {
        return;
    }
    if let Some(root) = state.root.take() {
        commands.entity(root).despawn();
    }
    state.snapshot = current;
    let Some(generation) = current else {
        return;
    };
    let root = commands
        .spawn((
            Name::new("touch_controls.window"),
            GlobalZIndex(100),
            source::frame(skins.frame.clone(), [2., 17., 2., 2.]),
            Node {
                position_type: PositionType::Absolute,
                right: px(20.),
                top: px(50.),
                padding: px(8.).all(),
                flex_direction: FlexDirection::Column,
                ..default()
            },
        ))
        .id();
    state.root = Some(root);
    source::dialog_caption(&mut commands, root, "Touch controls", skins.font.clone());
    let button = commands
        .spawn((
            Close(generation),
            source::button(skins.buttons.clone(), 100.),
        ))
        .observe(close)
        .id();
    let label = commands
        .spawn(source::text(
            "Close",
            source::font(skins.font.clone(), false),
        ))
        .id();
    commands.entity(button).add_child(label);
    commands.entity(root).add_child(button);
}
fn close(event: On<Activate>, controls: Query<&Close>, mut owner: NonSendMut<HostOwner>) {
    if let Ok(Close(generation)) = controls.get(event.entity) {
        visibility(&mut owner, *generation, true, false);
    }
}
fn keyboard(
    mut events: MessageReader<bevy::input::keyboard::KeyboardInput>,
    state: Res<State>,
    mut owner: NonSendMut<HostOwner>,
) {
    if events.read().any(|e| {
        e.key_code == KeyCode::Escape && e.state == bevy::input::ButtonState::Pressed && !e.repeat
    }) && let Some(generation) = state.snapshot
    {
        visibility(&mut owner, generation, true, false);
    }
}

// Owner polling runs in PreUpdate. Cancel a gesture after modal/editor widgets
// settle too, so a modal opened late in Update cannot leave a held step for it.
fn cancel_blocked(
    mut owner: NonSendMut<HostOwner>,
    mut gestures: ResMut<Gestures>,
    guard: super::modal::ModalGuard,
    focus: Res<bevy::input_focus::InputFocus>,
    editors: Query<(), With<bevy::text::EditableText>>,
) {
    if owner.modal_active()
        || !guard.allows_focus(None)
        || focus.get().is_some_and(|entity| editors.contains(entity))
    {
        gestures.cancel(&mut owner);
    }
}

/// A contact belongs to the world only after an admitted start. Geometry changes
/// invalidate it, so a release cannot silently select a different scene target.
#[derive(Resource, Default)]
pub(in crate::bevy_shell) struct Gestures {
    binding: Option<(u64, UVec2, UVec2, UVec3)>,
    contacts: HashMap<u64, Contact>,
}
struct Contact {
    view: oteryn_renderer::TileView,
    tile: oteryn_renderer::TileCoord,
    target: Option<crate::input::Targetable>,
    mode: Option<usize>,
    tap: bool,
}
impl Gestures {
    pub fn cancel(&mut self, owner: &mut HostOwner) {
        self.contacts.clear();
        self.binding = None;
        if let Some(play) = &mut owner.play {
            play.view.cancel_touch_controls();
        }
    }
    pub fn synchronize(&mut self, owner: &mut HostOwner, viewport: WorldViewport) -> bool {
        let Some(play) = &owner.play else {
            self.cancel(owner);
            return false;
        };
        let binding = (
            owner.generation(),
            viewport.origin,
            viewport.size,
            UVec3::new(
                play.view.scene().view().viewport_px().0,
                play.view.scene().view().viewport_px().1,
                play.view.scene().view().tile_px(),
            ),
        );
        let same = self.binding == Some(binding);
        if !same {
            self.cancel(owner);
            self.binding = Some(binding);
        }
        same
    }
    pub fn route(
        &mut self,
        owner: &mut HostOwner,
        viewport: WorldViewport,
        id: u64,
        phase: TouchPhase,
        point: Option<[f64; 2]>,
        allowed: bool,
    ) -> bool {
        self.synchronize(owner, viewport);
        if !allowed
            || owner.modal_active()
            || point.is_none()
            || owner.play.as_ref().is_none_or(|play| {
                !play.view.touch_controls_available(&play.link)
                    || (phase != TouchPhase::Started && !play.view.touch_controls_visible())
            })
        {
            self.cancel(owner);
            return false;
        }
        let Some(point) = point else {
            return false;
        };
        let Some(play) = &mut owner.play else {
            return false;
        };
        let Some(tile) = crate::input::click_tile(play.view.scene().view(), point[0], point[1])
        else {
            self.cancel(owner);
            return false;
        };
        if phase == TouchPhase::Started {
            if self.contacts.contains_key(&id) {
                return false;
            }
            self.contacts.insert(
                id,
                Contact {
                    view: *play.view.scene().view(),
                    tile,
                    target: play.view.scene().target_at(tile),
                    mode: play.view.touch_controls_mode(),
                    tap: false,
                },
            );
        } else {
            let Some(contact) = self.contacts.get(&id) else {
                return false;
            };
            if phase == TouchPhase::Ended
                && contact.tap
                && (play.view.scene().target_at(contact.tile) != contact.target
                    || *play.view.scene().view() != contact.view
                    || play.view.touch_controls_mode() != contact.mode)
            {
                self.cancel(owner);
                return false;
            }
        }
        if matches!(phase, TouchPhase::Ended | TouchPhase::Cancelled) {
            self.contacts.remove(&id);
        }
        if play.view.touch_input(&play.link, id, phase, point).is_err() {
            self.cancel(owner);
            return false;
        }
        if phase == TouchPhase::Started
            && let Some(contact) = self.contacts.get_mut(&id)
        {
            contact.mode = play.view.touch_controls_mode();
            contact.tap = play.view.touch_controls_tap_contact(id);
        }
        true
    }
}

/// Upstream picking establishes touch ownership just as it does for mouse starts.
#[derive(bevy::ecs::system::SystemParam)]
pub(in crate::bevy_shell) struct TouchHits<'w, 's> {
    hover: Res<'w, bevy::picking::hover::HoverMap>,
    parents: Query<'w, 's, &'static ChildOf>,
    pointers: Query<
        'w,
        's,
        (
            &'static bevy::picking::pointer::PointerId,
            &'static bevy::picking::pointer::PointerLocation,
        ),
    >,
    primary: Query<'w, 's, (Entity, &'static Window), With<bevy::window::PrimaryWindow>>,
}
impl TouchHits<'_, '_> {
    pub fn physical(&self, entity: Entity, point: Vec2) -> Option<Vec2> {
        let (primary, window) = self.primary.single().ok()?;
        (entity == primary && window.focused).then_some(point * window.scale_factor())
    }
    pub fn allows(&self, world: Entity, id: u64, position: Vec2) -> bool {
        use bevy::picking::pointer::PointerId;
        let Ok((entity, _)) = self.primary.single() else {
            return false;
        };
        let Some(target) = bevy::window::WindowRef::Entity(entity).normalize(None) else {
            return false;
        };
        let mut pointers = self
            .pointers
            .iter()
            .filter(|(pointer, _)| **pointer == PointerId::Touch(id));
        let Some((_, pointer)) = pointers.next() else {
            return false;
        };
        if pointers.next().is_some()
            || pointer.location().is_none_or(|location| {
                location.target != bevy::camera::NormalizedRenderTarget::Window(target)
                    || !location.position.is_finite()
                    || location.position.round() != position.round()
            })
        {
            return false;
        }

        self.hover
            .get(&bevy::picking::pointer::PointerId::Touch(id))
            .is_some_and(|hits| {
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
            })
    }
}
#[cfg(test)]
mod tests;
