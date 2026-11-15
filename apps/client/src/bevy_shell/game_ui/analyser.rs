//! Live analyser reports over the retained session measurements.
#[cfg(test)]
mod tests;
mod tracker;
mod widgets;
use super::{GameModal, modal::ModalGuard};
use crate::bevy_shell::{owner::HostOwner, ui::Skins};
use bevy::{input_focus::InputFocus, prelude::*, ui_widgets::Activate};
use oteryn_client_ui::analyser::Tool;

#[derive(Component)]
struct Overlay;
#[derive(Component)]
struct Report;
#[derive(Component)]
struct Control {
    generation: u64,
    tool: Tool,
    reset: bool,
}

pub(super) fn install(app: &mut App) {
    app.init_resource::<InputFocus>()
        .add_message::<bevy::input::keyboard::KeyboardInput>()
        .add_systems(Update, (keyboard, project).chain())
        .add_observer(activate);
    tracker::install(app);
}
pub(super) fn open(owner: &HostOwner, tool: Tool) {
    if owner.modal_active() {
        return;
    }
    if let Some(play) = &owner.play
        && !play.view.gameplay_input_blocked(&play.link)
        && play.link.analyser_panel().selected
    {
        play.link.analyser_select(tool.key());
        play.link.analyser_open();
    }
}
fn activate(
    event: On<Activate>,
    controls: Query<&Control>,
    guard: ModalGuard,
    owner: NonSend<HostOwner>,
) {
    if !guard.allows(event.entity) || owner.modal_active() {
        return;
    }
    let Ok(control) = controls.get(event.entity) else {
        return;
    };
    let Some(play) = &owner.play else { return };
    let panel = play.link.analyser_panel();
    if control.generation != owner.generation() || control.tool != panel.tool || !panel.open {
        return;
    }
    if control.reset {
        if !play.view.gameplay_input_blocked(&play.link) {
            play.link.analyser_reset();
        }
    } else {
        play.link.analyser_close();
    }
}
fn keyboard(
    mut events: MessageReader<bevy::input::keyboard::KeyboardInput>,
    overlays: Query<Entity, With<Overlay>>,
    guard: ModalGuard,
    owner: NonSend<HostOwner>,
) {
    let escape = events.read().any(|event| {
        event.key_code == KeyCode::Escape
            && event.state == bevy::input::ButtonState::Pressed
            && !event.repeat
    });
    if escape
        && !owner.modal_active()
        && overlays.iter().any(|entity| guard.allows(entity))
        && let Some(play) = &owner.play
    {
        play.link.analyser_close();
    }
}
fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skins: Res<Skins>,
    mut focus: ResMut<InputFocus>,
    parents: Query<&ChildOf>,
    mut window: Local<Option<widgets::Widgets>>,
) {
    let panel = owner.play.as_ref().map(|play| play.link.analyser_panel());
    let panel = panel.filter(|panel| panel.selected && panel.open);
    let key = panel.as_ref().map(|panel| (owner.generation(), panel.tool));
    if window
        .as_ref()
        .is_some_and(|window| Some(window.key) != key)
        && let Some(old) = window.take()
    {
        let mut entity = focus.get();
        while let Some(current) = entity {
            if current == old.overlay {
                focus.clear();
                break;
            }
            entity = parents.get(current).ok().map(ChildOf::parent);
        }
        commands.entity(old.overlay).despawn();
    }
    let Some(panel) = panel else { return };
    let Some(play) = &owner.play else { return };
    let mut text = play.link.analyser_text();
    if panel.tool == Tool::Hunting {
        text.push_str("\r\n\r\n");
        text.push_str(&oteryn_client_ui::analyser::session_text(&panel));
    }
    let text = text.replace("\r\n", "\n");
    let window = window.get_or_insert_with(|| {
        widgets::build(&mut commands, &skins, owner.generation(), panel.tool)
    });
    if let Some(caption) = window.tracker_filter {
        commands
            .entity(caption)
            .insert(Text::new(if panel.loot_only_tracked {
                "Show all drops"
            } else {
                "Show tracked only"
            }));
    }
    if window.text != text {
        commands.entity(window.report).insert(Text::new(&text));
        window.text = text;
    }
}
