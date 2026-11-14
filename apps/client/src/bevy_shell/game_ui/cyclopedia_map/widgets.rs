use super::*;
use bevy::{
    input_focus::tab_navigation::{TabGroup, TabIndex},
    text::{EditableText, TextCursorStyle},
    ui_widgets::{ScrollArea, TextInput},
};
use oteryn_client_ui::cyclopedia_map::{self as map, Intent};
use oteryn_client_ui_bevy as source;
#[derive(Component)]
pub(super) struct Donation;
pub(super) fn build(
    commands: &mut Commands,
    skins: &Skins,
    generation: u64,
    snapshot: &MapSnapshot,
    confirmation: Option<&MapControl>,
    editor: Option<(EditableText, bool)>,
) -> Entity {
    let snapshot = std::sync::Arc::new(snapshot.clone());
    let root = commands
        .spawn((
            GlobalZIndex(90),
            bevy::picking::Pickable::IGNORE,
            TabGroup {
                order: 2,
                modal: confirmation.is_some(),
            },
            Node {
                position_type: PositionType::Absolute,
                width: percent(100.),
                height: percent(100.),
                justify_content: JustifyContent::FlexStart,
                padding: UiRect::left(px(8.)),
                align_items: AlignItems::Center,
                ..default()
            },
        ))
        .id();
    if confirmation.is_some() {
        commands.entity(root).insert((
            super::super::GameModal,
            bevy::picking::Pickable::default(),
            BackgroundColor(Color::srgba(0., 0., 0., 0.45)),
        ));
    }
    let body = commands
        .spawn((
            source::frame(skins.frame.clone(), [4., 17., 4., 4.]),
            ScrollArea,
            Node {
                width: px(940.),
                max_width: percent(75.),
                max_height: percent(95.),
                overflow: Overflow {
                    x: OverflowAxis::Scroll,
                    y: OverflowAxis::Scroll,
                },
                padding: UiRect::all(px(16.)),
                flex_direction: FlexDirection::Column,
                row_gap: px(6.),
                ..default()
            },
        ))
        .id();
    commands.entity(root).add_child(body);
    source::dialog_caption(
        commands,
        body,
        "Cyclopedia — Ctrl+Alt+Shift+F8",
        skins.font.clone(),
    );
    let close = row(commands, body);
    button(
        commands,
        skins,
        close,
        generation,
        &snapshot,
        MapControl::Close,
        "Close (Esc)",
        false,
    );
    if let Some(action) = confirmation {
        let question = match action {
            MapControl::Request(Intent::Donate) => format!(
                "Donate {} gold? The server must confirm the payment.",
                snapshot.panel.fields[0]
            ),
            MapControl::Request(Intent::Shuffle) => "Reset the current exploration POIs?".into(),
            _ => "Start discovery in the current exploration area?".into(),
        };
        label(commands, skins, body, &question);
        button(
            commands,
            skins,
            body,
            generation,
            &snapshot,
            action.clone(),
            "Confirm",
            true,
        );
        return root;
    }
    label(commands, skins, body, &snapshot.text);
    label(commands, skins, body, &map::map_text(&snapshot.panel));
    let actions = row(commands, body);
    for (intent, caption) in [
        (Intent::Query, "Query / exploration status"),
        (Intent::Start, "Start discovery"),
        (Intent::Shuffle, "Reset POIs"),
        (Intent::Find, "Find"),
        (Intent::Donate, "Donate"),
        (Intent::RegionAtlas, "Load region atlas"),
        (Intent::AtlasNext, "Next atlas page"),
    ] {
        button(
            commands,
            skins,
            actions,
            generation,
            &snapshot,
            MapControl::Request(intent),
            caption,
            false,
        );
    }
    let restore_focus = editor.as_ref().is_some_and(|(_, focused)| *focused);
    let mut edit = editor.map_or_else(
        || EditableText::new(&snapshot.panel.fields[0]),
        |(text, _)| text,
    );
    edit.max_characters = Some(64);
    let field = commands
        .spawn((
            Donation,
            TextInput,
            edit,
            source::font(skins.font.clone(), false),
            TextColor(source::TEXT_COLOR),
            TextLayout::no_wrap(),
            TextCursorStyle::default(),
            TabIndex(0),
            Node {
                width: px(180.),
                height: px(25.),
                ..default()
            },
            source::frame(skins.inset.clone(), [1.; 4]),
        ))
        .id();
    commands.entity(body).add_child(field);
    if restore_focus {
        commands.queue(move |world: &mut World| {
            world
                .resource_mut::<bevy::input_focus::InputFocus>()
                .set(field, bevy::input_focus::FocusCause::Navigated);
        });
    }
    button(
        commands,
        skins,
        body,
        generation,
        &snapshot,
        MapControl::Donation(String::new()),
        "Set donation gold (1–10000000)",
        false,
    );
    let floors = row(commands, body);
    for (action, caption) in [
        (MapControl::Floor(true), "Previous map floor"),
        (MapControl::Floor(false), "Next map floor"),
        (MapControl::Pois, "Toggle POI layer"),
    ] {
        button(
            commands, skins, floors, generation, &snapshot, action, caption, false,
        );
    }
    label(commands, skins, body, &map::atlas_detail(&snapshot.panel));
    let atlas = row(commands, body);
    for (action, caption) in [
        "Refresh atlas",
        "Center",
        "Floor up",
        "Floor down",
        "Zoom in",
        "Zoom out",
        "North",
        "South",
        "West",
        "East",
        "NPC layer",
        "Passages",
        "POI layer",
        "Regions",
        "Previous markers",
        "Next markers",
    ]
    .into_iter()
    .enumerate()
    {
        button(
            commands,
            skins,
            atlas,
            generation,
            &snapshot,
            MapControl::Atlas(action),
            caption,
            false,
        );
    }
    if let Some(a) = &snapshot.panel.atlas {
        let canvas = commands
            .spawn(Node {
                width: px(768.),
                height: px(480.),
                min_height: px(480.),
                position_type: PositionType::Relative,
                flex_shrink: 0.,
                ..default()
            })
            .id();
        commands.entity(body).add_child(canvas);
        for (cell, id) in a.cells.iter().enumerate() {
            let color = map::atlas_color(&snapshot.panel, *id);
            let entity = commands
                .spawn((
                    Control {
                        generation,
                        snapshot: snapshot.clone(),
                        action: MapControl::Cell(cell),
                        confirmed: false,
                    },
                    bevy::ui_widgets::Button,
                    Node {
                        position_type: PositionType::Absolute,
                        left: px((cell % 64) as f32 * 12.),
                        top: px((cell / 64) as f32 * 12.),
                        width: px(12.),
                        height: px(12.),
                        ..default()
                    },
                    BackgroundColor(rgb(color)),
                ))
                .observe(activate)
                .id();
            commands.entity(canvas).add_child(entity);
        }
        let step = 1u32 << a.query.zoom;
        for marker in &a.markers {
            if snapshot.panel.atlas_layers & (1 << (marker.kind - 1)) == 0 {
                continue;
            }
            let col = u32::from(marker.x)
                .checked_sub(u32::from(a.query.x))
                .map(|v| v / step);
            let row = u32::from(marker.y)
                .checked_sub(u32::from(a.query.y))
                .map(|v| v / step);
            if let (Some(col), Some(row)) = (col.filter(|v| *v < 64), row.filter(|v| *v < 40)) {
                let point = commands
                    .spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            left: px(col as f32 * 12. + 2.),
                            top: px(row as f32 * 12. + 2.),
                            width: px(8.),
                            height: px(8.),
                            ..default()
                        },
                        BackgroundColor(rgb(match marker.kind {
                            1 => 0xffaa44,
                            2 => 0xee66bb,
                            _ => 0x44eeee,
                        })),
                        bevy::picking::Pickable::IGNORE,
                    ))
                    .id();
                commands.entity(canvas).add_child(point);
            }
        }
        for area in &a.areas {
            label(
                commands,
                skins,
                body,
                &format!(
                    "{}{} — {}",
                    if area.parent == 0 {
                        "Region "
                    } else {
                        "Subarea "
                    },
                    area.id,
                    area.name
                ),
            );
        }
        for marker in &a.markers {
            if snapshot.panel.atlas_layers & (1 << (marker.kind - 1)) != 0 {
                label(
                    commands,
                    skins,
                    body,
                    &format!("{} ({}, {})", marker.label, marker.x, marker.y),
                );
            }
        }
    }
    label(
        commands,
        skins,
        body,
        &map::atlas_view::text(&snapshot.panel),
    );
    let geometry = row(commands, body);
    for (action, caption) in [
        "Regions",
        "Previous area",
        "Next area",
        "Subareas",
        "Top-level regions",
        "Load map",
        "Floor up",
        "Floor down",
        "Region layer",
        "POI layer",
        "Refresh visible POIs",
        "Show graphical map",
        "Next area page",
        "Previous area page",
    ]
    .into_iter()
    .enumerate()
    .filter(|(i, _)| *i != 11)
    {
        button(
            commands,
            skins,
            geometry,
            generation,
            &snapshot,
            MapControl::Geometry(action),
            caption,
            false,
        );
    }
    let drawing = map::atlas_view::project(&snapshot.panel);
    label(commands, skins, body, &drawing.caption);
    if snapshot.panel.geometry.bounds.is_some() {
        let canvas = commands
            .spawn(Node {
                width: px(640.),
                height: px(480.),
                min_height: px(480.),
                position_type: PositionType::Relative,
                flex_shrink: 0.,
                ..default()
            })
            .id();
        commands.entity(body).add_child(canvas);
        let colors = [0x78634c, 0x706c45, 0x687748, 0x83754f, 0x6d6253, 0x7b6b4b];
        for rect in drawing.rects {
            let [x, y, w, h] = rect.bounds;
            rectangle(
                commands,
                canvas,
                [x, y, w, h],
                rgb(colors[usize::from(rect.color)]),
            );
        }
        for [x, y] in drawing.points {
            rectangle(commands, canvas, [x - 3, y - 3, 7, 7], rgb(0x5eddf3));
        }
    }
    label(commands, skins, body, &snapshot.analytics);
    let tools = row(commands, body);
    for (index, tool) in oteryn_client_ui::analyser::Tool::CYCLOPEDIA
        .iter()
        .enumerate()
    {
        button(
            commands,
            skins,
            tools,
            generation,
            &snapshot,
            MapControl::Analytics(index),
            tool.title(),
            false,
        );
    }
    root
}
fn rgb(value: u32) -> Color {
    Color::srgb(
        (value & 255) as f32 / 255.,
        ((value >> 8) & 255) as f32 / 255.,
        ((value >> 16) & 255) as f32 / 255.,
    )
}
fn rectangle(commands: &mut Commands, parent: Entity, bounds: [i32; 4], color: Color) {
    let [x, y, w, h] = bounds;
    let e = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(x as f32),
                top: px(y as f32),
                width: px(w as f32),
                height: px(h as f32),
                ..default()
            },
            BackgroundColor(color),
        ))
        .id();
    commands.entity(parent).add_child(e);
}
fn row(commands: &mut Commands, parent: Entity) -> Entity {
    let e = commands
        .spawn(Node {
            width: percent(100.),
            flex_wrap: FlexWrap::Wrap,
            column_gap: px(4.),
            row_gap: px(4.),
            ..default()
        })
        .id();
    commands.entity(parent).add_child(e);
    e
}
fn label(commands: &mut Commands, skins: &Skins, parent: Entity, text: &str) {
    let e = commands
        .spawn(source::text(text, source::font(skins.font.clone(), false)))
        .id();
    commands.entity(parent).add_child(e);
}
// The shared button constructor keeps the binding beside its caption and confirmation flag.
#[allow(clippy::too_many_arguments)]
fn button(
    commands: &mut Commands,
    skins: &Skins,
    parent: Entity,
    generation: u64,
    snapshot: &std::sync::Arc<MapSnapshot>,
    action: MapControl,
    caption: &str,
    confirmed: bool,
) {
    let e = commands
        .spawn((
            source::button(skins.buttons.clone(), 150.),
            Control {
                generation,
                snapshot: snapshot.clone(),
                action,
                confirmed,
            },
        ))
        .observe(activate)
        .id();
    let t = commands
        .spawn(source::button_caption(caption, skins.font.clone()))
        .id();
    commands.entity(e).add_child(t);
    commands.entity(parent).add_child(e);
}
