//! Named local copies of the appearance draft. Applying a preset still requires OK.
use super::*;
use bevy::{text::EditableText, ui_widgets::TextInput};

#[derive(Component)]
pub(super) struct NameField(pub String);

#[derive(Clone)]
pub(super) enum Control {
    Apply(String),
    Create,
    Delete,
    Rename,
    Update,
}

pub(super) fn build(
    commands: &mut Commands,
    skin: &Skins,
    root: Entity,
    generation: u64,
    owner: &HostOwner,
    selected: Option<&str>,
    retained: Option<Entity>,
) {
    let presets = &owner.settings.preferences.outfit_presets;
    for name in presets.keys() {
        button(
            commands,
            skin,
            root,
            generation,
            Kind::Preset(Control::Apply(name.clone())),
            name,
        );
    }
    button(
        commands,
        skin,
        root,
        generation,
        Kind::Preset(Control::Create),
        "Create preset",
    );
    if let Some(name) = selected.filter(|name| presets.contains_key(*name)) {
        let editor = retained.unwrap_or_else(|| {
            let mut text = EditableText::new(name);
            text.max_characters = Some(128);
            commands
                .spawn((
                    NameField(name.into()),
                    Name::new("outfit.preset.name"),
                    TextInput,
                    text,
                    source::font(skin.font.clone(), false),
                    TextColor(source::TEXT_COLOR),
                    bevy::input_focus::tab_navigation::TabIndex(0),
                    bevy::text::TextCursorStyle::default(),
                    Node {
                        width: px(250.),
                        min_height: px(25.),
                        ..default()
                    },
                ))
                .id()
        });
        commands.entity(root).add_child(editor);
        for (label, action) in [
            ("Update preset", Control::Update),
            ("Rename preset", Control::Rename),
            ("Delete preset", Control::Delete),
        ] {
            button(
                commands,
                skin,
                root,
                generation,
                Kind::Preset(action),
                label,
            );
        }
    }
}

pub(super) fn activate(owner: &mut HostOwner, state: &mut State, action: &Control, input: String) {
    let Some(play) = &owner.play else { return };
    let mut panel = play.link.appearance_panel();
    if !panel.open || panel.pending || !panel.selected {
        return;
    }
    let mut prefs = owner.settings.preferences.clone();
    let presets = &mut prefs.outfit_presets;
    let draft = (
        std::array::from_fn(|index| panel.fields[index].clone()),
        state.preset_riding.or_else(|| play.link.riding()),
    );
    match action {
        Control::Apply(name) => {
            let Some((fields, riding)) = presets.get(name) else {
                return;
            };
            let expected = panel.clone();
            panel.fields[..4].clone_from_slice(fields);
            if panel.preview().is_none() || !edit(&mut panel, &Kind::Mount(fields[2].clone())) {
                return;
            }
            if !play.link.appearance_preset(&expected, fields) {
                return;
            }
            state.preset_riding = riding.map(|value| value && !fields[2].is_empty());
            state.preset = Some(name.clone());
        }
        Control::Create => {
            if presets.len() >= 64 || panel.preview().is_none() {
                return;
            }
            let mut name = "New Preset".to_string();
            for index in 2..=65 {
                if !presets.contains_key(&name) {
                    break;
                }
                name = format!("New Preset {index}");
            }
            presets.insert(name.clone(), draft);
            state.preset = Some(name);
        }
        Control::Delete | Control::Update | Control::Rename => {
            let Some(name) = state
                .preset
                .as_ref()
                .filter(|name| presets.contains_key(*name))
            else {
                return;
            };
            match action {
                Control::Delete => {
                    presets.remove(name);
                    state.preset = None;
                    state.preset_riding = None;
                }
                Control::Update if panel.preview().is_some() => {
                    presets.insert(name.clone(), draft);
                }
                Control::Rename => {
                    let next = input.trim();
                    if next.is_empty()
                        || next.len() > 128
                        || next.chars().any(char::is_control)
                        || presets.contains_key(next)
                    {
                        return;
                    }
                    if let Some(saved) = presets.remove(name) {
                        presets.insert(next.into(), saved);
                        state.preset = Some(next.into());
                    }
                }
                _ => return,
            }
        }
    }
    if prefs != owner.settings.preferences {
        owner.settings.apply(prefs);
    }
    state.dirty = true;
}

pub(super) fn send_selection(link: &crate::play::PlayLink, kind: &Kind, state: &mut State) {
    if matches!(kind, Kind::Riding)
        && let Some(riding) = &mut state.preset_riding
    {
        *riding = !*riding;
        state.dirty = true;
        return;
    }
    if matches!(kind, Kind::Ok)
        && let Some(riding) = state.preset_riding
    {
        if let Some(riding) = link.appearance_accept_preset(riding) {
            state.preset_riding = None;
            link.mount_request_to(Some(riding));
        }
        return;
    }
    send(link, kind);
    if matches!(kind, Kind::Mount(key) if key.is_empty()) && state.preset_riding.is_some() {
        state.preset_riding = Some(false);
    }
}
