//! Live analyser reports over the retained session measurements.
#[cfg(test)]
mod tests;
mod tracker;
mod widgets;
use super::modal::ModalGuard;
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

#[derive(Component)]
struct Refresh {
    generation: u64,
    tool: Tool,
    boss: Option<oteryn_client_ui::boss_encounter::Intent>,
}
fn refresh(
    event: On<Activate>,
    controls: Query<&Refresh>,
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
    if control.generation == owner.generation() && panel.open && panel.tool == control.tool {
        if let Some(intent) = control.boss {
            play.link.boss_encounter_request(intent);
        } else {
            play.link.analyser_party_refresh();
        }
    }
}
#[derive(PartialEq, Eq)]
struct IdentityProgress {
    generation: u64,
    items: Vec<u32>,
    resolved: usize,
}
fn resolve_items(owner: NonSend<HostOwner>, mut previous: Local<Option<IdentityProgress>>) {
    let Some(play) = &owner.play else {
        *previous = None;
        return;
    };
    let panel = play.link.analyser_panel();
    if !panel.open || !panel.selected {
        return;
    }
    let mut items = panel
        .loot
        .keys()
        .chain(panel.supply.keys())
        .chain(panel.loot_tracked.iter())
        .copied()
        .collect::<Vec<_>>();
    items.sort_unstable();
    items.dedup();
    let progress = IdentityProgress {
        generation: owner.generation(),
        resolved: panel.item_info.len(),
        items,
    };
    if previous.as_ref() != Some(&progress) {
        if panel.catalogue_pending.is_empty()
            && progress
                .items
                .iter()
                .any(|id| !panel.item_info.contains_key(id))
        {
            play.link.analyser_resolve_items();
        }
        *previous = Some(progress);
    }
}
#[derive(Component)]
struct ConfigField(usize);
#[derive(Clone, Copy)]
enum ConfigKind {
    Goal,
    Loot,
    Supply,
}
#[derive(Component)]
struct ConfigControl {
    generation: u64,
    tool: Tool,
    kind: ConfigKind,
}
#[derive(Resource, Default)]
struct ConfigPending(Option<(Entity, u64, Tool, ConfigKind)>);
fn configure(
    event: On<Activate>,
    controls: Query<&ConfigControl>,
    guard: ModalGuard,
    owner: NonSend<HostOwner>,
    mut pending: ResMut<ConfigPending>,
) {
    if !guard.allows(event.entity) || owner.modal_active() {
        return;
    }
    let Ok(control) = controls.get(event.entity) else {
        return;
    };
    if control.generation == owner.generation() && pending.0.is_none() {
        pending.0 = Some((event.entity, control.generation, control.tool, control.kind));
    }
}
fn finish_configuration(
    owner: NonSend<HostOwner>,
    mut pending: ResMut<ConfigPending>,
    guard: ModalGuard,
    fields: Query<(&ConfigField, &bevy::text::EditableText)>,
) {
    let Some((entity, generation, tool, kind)) = pending.0 else {
        return;
    };
    let Some(play) = &owner.play else {
        pending.0 = None;
        return;
    };
    let panel = play.link.analyser_panel();
    if generation != owner.generation()
        || owner.modal_active()
        || !guard.allows(entity)
        || !panel.open
        || panel.tool != tool
    {
        pending.0 = None;
        return;
    }
    if fields.iter().any(|(_, text)| {
        text.pending_paste.is_some()
            || text
                .pending_edits
                .iter()
                .any(bevy::text::TextEdit::is_destructive)
    }) {
        return;
    }
    pending.0 = None;
    let mut values: [String; 3] = default();
    for (field, text) in &fields {
        if let Some(value) = values.get_mut(field.0) {
            *value = text.value().to_string();
        }
    }
    match kind {
        ConfigKind::Goal => {
            play.link.analyser_xp_goal(&values[0]);
        }
        ConfigKind::Loot | ConfigKind::Supply => {
            play.link
                .analyser_select(if matches!(kind, ConfigKind::Supply) {
                    Tool::Supply.key()
                } else {
                    Tool::Loot.key()
                });
            let saved = play.link.analyser_item_price(&values[1], &values[2]);
            play.link.analyser_select(tool.key());
            if saved && tool == Tool::Party {
                play.link.analyser_party_refresh();
            }
        }
    }
}
pub(super) fn install(app: &mut App) {
    app.init_resource::<InputFocus>()
        .init_resource::<ConfigPending>()
        .add_systems(
            PostUpdate,
            finish_configuration.after(bevy::text::EditableTextSystems),
        )
        .add_observer(configure)
        .add_message::<bevy::input::keyboard::KeyboardInput>()
        .add_systems(Update, (keyboard, resolve_items, project).chain())
        .add_observer(activate)
        .add_observer(refresh);
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
    focus: Res<InputFocus>,
    parents: Query<&ChildOf>,
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
        && overlays.iter().any(|overlay| {
            let mut entity = focus.get();
            while let Some(current) = entity {
                if current == overlay {
                    return guard.allows(overlay);
                }
                entity = parents.get(current).ok().map(ChildOf::parent);
            }
            false
        })
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
    if let Some(feedback) = panel.feedback {
        text.push_str("\r\n");
        text.push_str(feedback);
    }
    let text = text.replace("\r\n", "\n");
    let window = window.get_or_insert_with(|| {
        if panel.tool == Tool::Party {
            play.link.analyser_party_refresh();
        } else if panel.tool == Tool::Bosses {
            play.link
                .boss_encounter_request(oteryn_client_ui::boss_encounter::Intent::Refresh);
        }
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
