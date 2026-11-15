use super::*;
use crate::play::GemControl;
use bevy::{
    text::{EditableText, TextCursorStyle},
    ui_widgets::TextInput,
};
use oteryn_client_ui::wheel_allocation::{self as model, gems};
use oteryn_client_ui_bevy as source;
#[derive(Clone, Component)]
struct Control {
    key: Key,
    action: Action,
}
#[derive(Clone)]
enum Action {
    Preset(usize),
    Gem(GemControl),
    OpenGems,
}
#[derive(Component)]
struct Field(usize);
#[derive(Resource, Default)]
struct Pending(Option<(Entity, Control)>);
pub(super) fn install(app: &mut App) {
    app.init_resource::<Pending>()
        .add_systems(PostUpdate, finish.after(bevy::text::EditableTextSystems));
}
fn activate(
    event: On<Activate>,
    controls: Query<&Control>,
    guard: ModalGuard,
    mut pending: ResMut<Pending>,
) {
    if guard.allows(event.entity)
        && let Ok(control) = controls.get(event.entity)
    {
        pending.0 = Some((event.entity, control.clone()));
    }
}
fn finish(
    owner: NonSend<HostOwner>,
    state: Res<State>,
    guard: ModalGuard,
    fields: Query<(&Field, &EditableText)>,
    mut pending: ResMut<Pending>,
) {
    let Some((entity, control)) = pending.0.as_ref() else {
        return;
    };
    let Some((key, expected)) = &state.snapshot else {
        pending.0 = None;
        return;
    };
    if control.key != *key
        || key.generation != owner.generation()
        || owner.modal_active()
        || !guard.allows(*entity)
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
    let Some((_, control)) = pending.0.take() else {
        return;
    };
    let Some(play) = &owner.play else {
        return;
    };
    if play.view.gameplay_input_blocked(&play.link) {
        return;
    }
    match control.action {
        Action::Preset(action) => {
            let mut values = expected.preset_fields.clone();
            for (field, text) in &fields {
                values[field.0] = text.value().to_string();
            }
            play.link
                .wheel_preset_control_bound(expected, action, values);
        }
        Action::Gem(action) => {
            play.link.wheel_gem_control_bound(expected, Some(action));
        }
        Action::OpenGems => {
            play.link.wheel_gem_control_bound(expected, None);
        }
    }
}
pub(super) fn render(
    commands: &mut Commands,
    skins: &Skins,
    parent: Entity,
    key: Key,
    p: &WheelPanel,
) {
    for (index, caption) in ["Preset name", "Build code or URL"].iter().enumerate() {
        label(commands, skins, parent, *caption);
        let mut text = EditableText::new(&p.preset_fields[index]);
        text.max_characters = Some(if index == 0 { 64 } else { 4096 });
        let field = commands
            .spawn((
                Field(index),
                TextInput,
                bevy::input_focus::tab_navigation::TabIndex(index as i32),
                TextLayout::no_wrap(),
                text,
                TextCursorStyle::default(),
                source::font(skins.font.clone(), false),
                TextColor(source::TEXT_COLOR),
                source::frame(skins.inset.clone(), [1.; 4]),
                Node {
                    height: px(26.),
                    width: percent(100.),
                    padding: px(4.).all(),
                    flex_shrink: 0.,
                    ..default()
                },
            ))
            .id();
        commands.entity(parent).add_child(field);
    }
    for (action, caption) in [
        (0, "Create preset"),
        (5, "Import code"),
        (6, "Rename preset"),
        (3, "Export code"),
        (4, "Export URL"),
        (2, "Delete preset"),
    ] {
        button(
            commands,
            skins,
            parent,
            key,
            Action::Preset(action),
            caption,
            p.ready(),
        );
    }
    if !p.gems.selected {
        return;
    }
    button(
        commands,
        skins,
        parent,
        key,
        Action::OpenGems,
        "Gem Atelier",
        true,
    );
    label(commands, skins, parent, gems::view::details(p));
    if let Some(atelier) = &p.confirmed_atelier {
        for gem in &atelier.gems {
            button(
                commands,
                skins,
                parent,
                key,
                Action::Gem(GemControl::Gem {
                    revision: atelier.revision,
                    id: gem.id,
                }),
                &model::gem_label(gem),
                true,
            );
        }
    }
    for (quality, name) in ["Lesser gems", "Regular gems", "Greater gems"]
        .iter()
        .enumerate()
    {
        button(
            commands,
            skins,
            parent,
            key,
            Action::Gem(GemControl::Quality(quality as u8)),
            name,
            p.ready(),
        );
    }
    for vessel in 0..p.vessel_slot_count() {
        button(
            commands,
            skins,
            parent,
            key,
            Action::Gem(GemControl::Vessel(vessel)),
            &format!("Vessel {}", vessel + 1),
            p.ready(),
        );
    }
    for (action, caption) in [
        (GemControl::Stage(model::Intent::Reveal), "Reveal gem"),
        (GemControl::Socket, "Assign gem to vessel"),
        (GemControl::Clear, "Clear vessel"),
        (GemControl::Save, "Save vessels"),
        (GemControl::Lock, "Lock / unlock gem"),
        (GemControl::Stage(model::Intent::Dismantle), "Dismantle gem"),
        (GemControl::Stage(model::Intent::Reshape), "Refine gem"),
        (
            GemControl::Stage(model::Intent::GradeUp),
            "Upgrade modifier grade",
        ),
        (GemControl::Refresh, "Refresh Gem Atelier"),
        (GemControl::Retry, "Retry gem operation"),
    ] {
        button(
            commands,
            skins,
            parent,
            key,
            Action::Gem(action),
            caption,
            p.ready(),
        );
    }
    label(commands, skins, parent, "Fragment Workshop");
    if let Some(reply) = gems::current(p) {
        for modifier in &reply.modifiers {
            button(
                commands,
                skins,
                parent,
                key,
                Action::Gem(GemControl::Modifier {
                    nonce: reply.request.nonce,
                    kind: modifier.kind,
                    id: modifier.id,
                }),
                &model::modifier_label(p, modifier.kind, modifier.id),
                p.ready(),
            );
        }
    }
    if let Some(review) = &p.gems.review {
        label(commands, skins, parent, &review.text);
        button(
            commands,
            skins,
            parent,
            key,
            Action::Gem(GemControl::Confirm),
            "Confirm gem operation",
            true,
        );
        button(
            commands,
            skins,
            parent,
            key,
            Action::Gem(GemControl::Cancel),
            "Cancel gem operation",
            true,
        );
    }
}
fn label(commands: &mut Commands, skins: &Skins, parent: Entity, text: impl Into<String>) {
    let label = commands
        .spawn(source::text(text, source::font(skins.font.clone(), false)))
        .id();
    commands.entity(parent).add_child(label);
}
fn button(
    commands: &mut Commands,
    skins: &Skins,
    parent: Entity,
    key: Key,
    action: Action,
    caption: &str,
    enabled: bool,
) {
    let mut button = commands.spawn((
        source::button(skins.buttons.clone(), 240.),
        Control { key, action },
    ));
    if !enabled {
        button.insert(bevy::ui::InteractionDisabled);
    }
    let button = button.observe(activate).id();
    let caption = commands
        .spawn(source::button_caption(caption, skins.font.clone()))
        .id();
    commands.entity(button).add_child(caption);
    commands.entity(parent).add_child(button);
}
