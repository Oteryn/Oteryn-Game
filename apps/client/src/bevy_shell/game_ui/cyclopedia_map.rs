//! Capability-57 exploration: confirmed area/subarea footprints and visible POIs.
use super::{GameModal, modal::ModalGuard};
use crate::bevy_shell::{
    owner::{HostOwner, Phase},
    ui::Skins,
};
use bevy::{
    prelude::*,
    text::{EditableText, TextCursorStyle},
    ui_widgets::{Activate, TextInput},
};
use oteryn_client_ui::cyclopedia_map::{Intent, Panel, atlas_view, region_atlas_text};
use oteryn_client_ui_bevy as source;

#[derive(Resource, Default)]
struct State {
    generation: u64,
    root: Option<Entity>,
    snapshot: Option<Panel>,
    serial: u64,
    /// A pressed Donate button (its window serial), sent once the gold editor settled.
    donate: Option<u64>,
}
#[derive(Component, Clone, Copy)]
struct Action {
    generation: u64,
    serial: u64,
    kind: Kind,
}
/// The donation gold the Donate button sends; the session keeps the last submitted amount.
#[derive(Component)]
struct Donation;
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Atlas(usize),
    Explore(Intent),
    /// Cycles the floors present in the confirmed exploration result and region atlas.
    Floor {
        previous: bool,
    },
    Close,
}

pub(super) fn install(app: &mut App) {
    app.init_resource::<State>()
        .add_systems(Update, project)
        .add_systems(PostUpdate, donate.after(bevy::text::EditableTextSystems));
}
fn activate(
    event: On<Activate>,
    actions: Query<&Action>,
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
    guard: ModalGuard,
) {
    if !guard.allows(event.entity) || owner.modal_active() {
        return;
    }
    let Ok(action) = actions.get(event.entity) else {
        return;
    };
    let Some(play) = &owner.play else {
        return;
    };
    let panel = play.link.cyclopedia_map_panel();
    if action.generation != owner.generation()
        || action.serial != state.serial
        || !panel.geometry.open
        || state.snapshot.as_ref() != Some(&panel)
    {
        return;
    }
    match action.kind {
        Kind::Close => play.link.cyclopedia_atlas_close(),
        Kind::Atlas(index) => play.link.cyclopedia_atlas_action(index),
        Kind::Floor { previous } => play.link.cyclopedia_map_floor(previous),
        Kind::Explore(Intent::Donate) => state.donate = Some(state.serial),
        Kind::Explore(intent) => play.link.cyclopedia_map_request(intent),
    }
}
/// Sends a pressed Donate once pending edits and a pending paste reached the gold editor.
fn donate(
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
    guard: ModalGuard,
    donation: Query<(Entity, &EditableText), With<Donation>>,
) {
    let Some(serial) = state.donate else {
        return;
    };
    let Ok((field, gold)) = donation.single() else {
        state.donate = None;
        return;
    };
    if gold.pending_paste.is_some()
        || gold
            .pending_edits
            .iter()
            .any(bevy::text::TextEdit::is_destructive)
    {
        return;
    }
    let Some(play) = &owner.play else {
        state.donate = None;
        return;
    };
    let panel = play.link.cyclopedia_map_panel();
    if serial != state.serial
        || state.generation != owner.generation()
        || owner.modal_active()
        || !guard.allows(field)
        || !panel.geometry.open
    {
        state.donate = None;
        return;
    }
    // A map update since the press redraws the window first; the press waits for that redraw.
    if state.snapshot.as_ref() != Some(&panel) {
        return;
    }
    state.donate = None;
    play.link
        .cyclopedia_map_edit(0, gold.value().to_string().trim().to_owned());
    play.link.cyclopedia_map_request(Intent::Donate);
}
fn label(commands: &mut Commands, skins: &Skins, parent: Entity, text: impl Into<String>) {
    let child = commands
        .spawn(source::text(
            text.into(),
            source::font(skins.font.clone(), false),
        ))
        .id();
    commands.entity(parent).add_child(child);
}
fn remove_root(
    state: &mut State,
    commands: &mut Commands,
    focus: &mut bevy::input_focus::InputFocus,
    parents: &Query<&ChildOf>,
) {
    if let Some(root) = state.root.take() {
        let mut current = focus.get();
        while let Some(entity) = current {
            if entity == root {
                focus.clear();
                break;
            }
            current = parents.get(entity).ok().map(ChildOf::parent);
        }
        commands.entity(root).despawn();
    }
}
fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skins: Res<Skins>,
    mut state: ResMut<State>,
    mut focus: ResMut<bevy::input_focus::InputFocus>,
    parents: Query<&ChildOf>,
    donation: Query<&EditableText, With<Donation>>,
) {
    let panel = owner
        .play
        .as_ref()
        .filter(|_| owner.phase == Phase::Playing)
        .and_then(|play| {
            play.link.cyclopedia_atlas_pump();
            play.link.cyclopedia_atlas_panel()
        });
    if state.generation != owner.generation() || panel.is_none() {
        remove_root(&mut state, &mut commands, &mut focus, &parents);
        state.snapshot = None;
        state.donate = None;
        state.generation = owner.generation();
    }
    let Some(panel) = panel else {
        return;
    };
    if state.snapshot.as_ref() == Some(&panel) {
        return;
    }
    // The typed gold survives a redraw of the same window; only Donate submits it.
    let draft = state
        .root
        .and_then(|_| donation.single().ok())
        .map(|gold| gold.value().to_string());
    remove_root(&mut state, &mut commands, &mut focus, &parents);
    let pressed = state.donate == Some(state.serial);
    state.serial = state.serial.wrapping_add(1);
    // A Donate pressed in the replaced window stays armed for the redrawn one.
    state.donate = pressed.then_some(state.serial);
    let root = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(140.),
                top: px(35.),
                width: px(700.),
                max_height: percent(90.),
                padding: UiRect::all(px(10.)),
                flex_direction: FlexDirection::Column,
                overflow: Overflow::scroll_y(),
                ..default()
            },
            BackgroundColor(Color::srgb(0.08, 0.08, 0.08)),
            GlobalZIndex(20),
            GameModal,
            bevy::input_focus::tab_navigation::TabGroup {
                order: 2,
                modal: true,
            },
            bevy::ui_widgets::ScrollArea,
        ))
        .id();
    label(&mut commands, &skins, root, "Cyclopedia Map");
    let bar = commands
        .spawn(Node {
            flex_wrap: FlexWrap::Wrap,
            ..default()
        })
        .id();
    commands.entity(root).add_child(bar);
    for (caption, kind) in [
        ("Regions", Kind::Atlas(0)),
        ("Previous area", Kind::Atlas(1)),
        ("Next area", Kind::Atlas(2)),
        ("Subareas", Kind::Atlas(3)),
        ("Load footprint", Kind::Atlas(5)),
        ("Previous floor", Kind::Floor { previous: true }),
        ("Next floor", Kind::Floor { previous: false }),
        ("Previous page", Kind::Atlas(13)),
        ("Next page", Kind::Atlas(12)),
        ("Exploration status", Kind::Explore(Intent::Query)),
        ("Start exploration", Kind::Explore(Intent::Start)),
        ("Find POI", Kind::Explore(Intent::Find)),
        ("Reset POIs", Kind::Explore(Intent::Shuffle)),
        ("Donate", Kind::Explore(Intent::Donate)),
        ("Region atlas", Kind::Explore(Intent::RegionAtlas)),
        ("Next atlas page", Kind::Explore(Intent::AtlasNext)),
        ("Close", Kind::Close),
    ] {
        let button = commands
            .spawn((
                source::button(skins.buttons.clone(), 130.),
                Action {
                    generation: owner.generation(),
                    serial: state.serial,
                    kind,
                },
            ))
            .observe(activate)
            .id();
        if kind != Kind::Close && (!panel.selected || panel.pending || panel.speed_pending) {
            commands
                .entity(button)
                .insert(bevy::ui::InteractionDisabled);
        }
        label(&mut commands, &skins, button, caption);
        commands.entity(bar).add_child(button);
    }
    label(&mut commands, &skins, root, "Donation gold (1-10000000)");
    let mut gold = EditableText::new(draft.as_deref().unwrap_or(&panel.fields[0]));
    gold.max_characters = Some(64);
    gold.visible_lines = Some(1.);
    let field = commands
        .spawn((
            Donation,
            TextInput,
            gold,
            source::font(skins.font.clone(), false),
            TextColor(source::TEXT_COLOR),
            TextLayout::default(),
            TextCursorStyle::default(),
            Node {
                width: px(200.),
                min_height: px(26.),
                padding: px(4.).all(),
                ..default()
            },
            BackgroundColor(Color::srgb(0.1, 0.1, 0.1)),
        ))
        .id();
    commands.entity(root).add_child(field);
    label(
        &mut commands,
        &skins,
        root,
        if !panel.selected {
            "Unavailable in this session".into()
        } else {
            format!(
                "{}\n{}\n{}",
                atlas_view::text(&panel),
                region_atlas_text(&panel),
                panel.feedback
            )
        },
    );
    let drawing = atlas_view::project(&panel);
    label(&mut commands, &skins, root, drawing.caption);
    let canvas = commands
        .spawn((
            Node {
                width: px(640.),
                height: px(480.),
                min_height: px(480.),
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(Color::BLACK),
        ))
        .id();
    commands.entity(root).add_child(canvas);
    let colors = [
        Color::srgb(0.25, 0.45, 0.3),
        Color::srgb(0.4, 0.4, 0.6),
        Color::srgb(0.6, 0.4, 0.3),
        Color::srgb(0.3, 0.5, 0.6),
        Color::srgb(0.6, 0.5, 0.3),
        Color::srgb(0.5, 0.3, 0.5),
    ];
    for rect in drawing.rects {
        let [x, y, w, h] = rect.bounds;
        let cell = commands
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: px(x as f32),
                    top: px(y as f32),
                    width: px(w as f32),
                    height: px(h as f32),
                    ..default()
                },
                BackgroundColor(colors[usize::from(rect.color)]),
            ))
            .id();
        commands.entity(canvas).add_child(cell);
    }
    for [x, y] in drawing.points {
        let point = commands
            .spawn((
                source::text("◆", source::font(skins.font.clone(), false)),
                Node {
                    position_type: PositionType::Absolute,
                    left: px(x as f32),
                    top: px(y as f32),
                    ..default()
                },
            ))
            .id();
        commands
            .entity(point)
            .insert(TextColor(Color::srgb(1., 0.85, 0.1)));
        commands.entity(canvas).add_child(point);
    }
    label(
        &mut commands,
        &skins,
        root,
        "Yellow diamonds: server-visible POIs. Exploration status refers to your current location, independently of the selected area.",
    );
    state.root = Some(root);
    state.snapshot = Some(panel);
}
#[cfg(test)]
mod tests;
