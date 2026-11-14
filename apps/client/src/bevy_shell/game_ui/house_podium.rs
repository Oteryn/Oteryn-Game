//! Presentation of the retained podium owner, including server-resolved artwork.
use super::{
    super::{owner::HostOwner, ui::Skins},
    GameModal,
    modal::ModalGuard,
};
use crate::play::feature_0150_house_podium::view::HousePodiumControl as Kind;
use crate::play::feature_0150_house_podium::view::house_podium_artwork;
use bevy::{
    prelude::*,
    text::{EditableText, TextCursorStyle},
    ui_widgets::{Activate, TextInput},
};
use oteryn_client_ui::house_podium::{Intent, Panel};
use oteryn_client_ui_bevy as source;
#[derive(Resource, Default)]
struct State {
    snapshot: Option<(u64, Panel)>,
    root: Option<Entity>,
    confirm: Option<[String; 4]>,
}
#[derive(Component, Clone)]
struct Control {
    generation: u64,
    expected: Panel,
    kind: Kind,
}
#[derive(Component)]
struct Field(usize);
pub(super) fn install(app: &mut App) {
    app.init_resource::<State>().add_systems(Update, project);
}
pub(in crate::bevy_shell) fn open(owner: &mut HostOwner) {
    if !owner.modal_active()
        && let Some(play) = &owner.play
    {
        play.link.house_podium_open();
    }
}
fn label(commands: &mut Commands, skins: &Skins, root: Entity, text: &str) {
    let child = commands
        .spawn(source::text(text, source::font(skins.font.clone(), false)))
        .id();
    commands.entity(root).add_child(child);
}
fn button(
    commands: &mut Commands,
    skins: &Skins,
    root: Entity,
    generation: u64,
    panel: &Panel,
    kind: Kind,
    text: &str,
) {
    let child = commands
        .spawn((
            Control {
                generation,
                expected: panel.clone(),
                kind,
            },
            source::button(skins.buttons.clone(), 160.),
        ))
        .observe(activate)
        .id();
    label(commands, skins, child, text);
    commands.entity(root).add_child(child);
}
fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skins: Res<Skins>,
    mut state: ResMut<State>,
    mut images: ResMut<Assets<Image>>,
) {
    let current = owner
        .play
        .as_ref()
        .map(|play| (owner.generation(), play.link.house_podium_panel()))
        .filter(|(_, p)| p.open);
    if state.snapshot == current {
        return;
    }
    if let Some(root) = state.root.take() {
        commands.entity(root).despawn();
    }
    state.confirm = None;
    state.snapshot = current.clone();
    let Some((generation, panel)) = current else {
        return;
    };
    let root = commands
        .spawn((
            GameModal,
            bevy::ui_widgets::ScrollArea,
            bevy::input_focus::tab_navigation::TabGroup::new(15),
            Name::new("house_podium.window"),
            GlobalZIndex(120),
            source::frame(skins.frame.clone(), [2., 17., 2., 2.]),
            Node {
                position_type: PositionType::Absolute,
                left: px(200.),
                top: px(60.),
                width: px(440.),
                max_height: percent(90.),
                overflow: Overflow::scroll_y(),
                padding: px(20.).all(),
                flex_direction: FlexDirection::Column,
                row_gap: px(4.),
                ..default()
            },
        ))
        .id();
    state.root = Some(root);
    source::dialog_caption(&mut commands, root, "House podium", skins.font.clone());
    label(
        &mut commands,
        &skins,
        root,
        &oteryn_client_ui::house_podium::panel_text(&panel),
    );
    for (index, caption) in [
        "Podium item UUID",
        "Display: 1 outfit, 2 mount, 3 creature",
        "Direction: 0 north, 1 east, 2 south, 3 west",
        "Display content key",
    ]
    .into_iter()
    .enumerate()
    {
        label(&mut commands, &skins, root, caption);
        let mut editable = EditableText::new(panel.fields[index].clone());
        editable.max_characters = Some(7996);
        editable.allow_newlines = false;
        let field = commands
            .spawn((
                Field(index),
                TextInput,
                editable,
                source::font(skins.font.clone(), false),
                TextColor(source::TEXT_COLOR),
                TextLayout::default(),
                TextCursorStyle::default(),
                bevy::input_focus::tab_navigation::TabIndex(0),
                Node {
                    min_height: px(24.),
                    ..default()
                },
            ))
            .id();
        commands.entity(root).add_child(field);
    }
    for (kind, caption) in [
        (Kind::Request(Intent::Refresh), "Refresh"),
        (Kind::Request(Intent::Projection), "Projection"),
        (Kind::Request(Intent::Set), "Set"),
        (Kind::Display(1), "Outfit"),
        (Kind::Display(2), "Mount"),
        (Kind::Display(3), "Creature"),
        (Kind::Turn, "Turn clockwise"),
        (Kind::Request(Intent::Preview), "Preview artwork"),
        (Kind::Close, "Close"),
    ] {
        button(
            &mut commands,
            &skins,
            root,
            generation,
            &panel,
            kind,
            caption,
        );
    }
    if panel.flagged {
        use oteryn_client_ui::house_podium_flags::Part;
        for (part, caption) in [
            (Part::Platform, "Toggle platform"),
            (Part::Outfit, "Toggle outfit"),
            (Part::Mount, "Toggle mount"),
        ] {
            button(
                &mut commands,
                &skins,
                root,
                generation,
                &panel,
                Kind::Toggle(part),
                caption,
            );
        }
    }
    if let Some(artwork) = panel.artwork {
        let rendered = house_podium_artwork(artwork);
        match rendered {
            Ok(rendered) => {
                for rgba in rendered {
                    let image = Image::new(
                        bevy::render::render_resource::Extent3d {
                            width: 128,
                            height: 128,
                            depth_or_array_layers: 1,
                        },
                        bevy::render::render_resource::TextureDimension::D2,
                        rgba,
                        bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
                        bevy::asset::RenderAssetUsages::MAIN_WORLD
                            | bevy::asset::RenderAssetUsages::RENDER_WORLD,
                    );
                    let child = commands
                        .spawn((
                            ImageNode::new(images.add(image)),
                            Node {
                                width: px(160.),
                                height: px(160.),
                                ..default()
                            },
                        ))
                        .id();
                    commands.entity(root).add_child(child);
                }
            }
            Err(error) => label(&mut commands, &skins, root, &error),
        }
    }
}
fn activate(
    event: On<Activate>,
    queries: (Query<&Control>, Query<(&Field, &EditableText)>),
    guard: ModalGuard,
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
    mut commands: Commands,
    skins: Res<Skins>,
) {
    let (controls, fields) = queries;
    let Ok(control) = controls.get(event.entity) else {
        return;
    };
    if !guard.allows(event.entity)
        || owner.modal_active()
        || owner.generation() != control.generation
    {
        return;
    }
    let Some(play) = &owner.play else {
        return;
    };
    if play.view.gameplay_input_blocked(&play.link)
        || play.link.house_podium_panel() != control.expected
    {
        return;
    }
    let mut values = control.expected.fields.clone();
    for (field, value) in &fields {
        values[field.0] = value.editor.text().to_string();
    }
    if control.kind == Kind::Request(Intent::Set) && state.confirm.as_ref() != Some(&values) {
        state.confirm = Some(values);
        if let Some(root) = state.root {
            label(
                &mut commands,
                &skins,
                root,
                "Apply this podium display? Press Set again to confirm.",
            );
        }
        return;
    }
    play.link
        .house_podium_draft_bound(&control.expected, values, control.kind.clone());
    state.confirm = None;
}

#[cfg(test)]
mod tests;
