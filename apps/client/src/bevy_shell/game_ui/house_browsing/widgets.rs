use super::*;
use bevy::{
    input_focus::tab_navigation::{TabGroup, TabIndex},
    text::TextCursorStyle,
    ui::InteractionDisabled,
    ui_widgets::{ScrollArea, TextInput},
};
use oteryn_client_ui_bevy as source;
pub(super) fn build(commands: &mut Commands, skins: &Skins, state: &State) -> Entity {
    let root = commands
        .spawn((
            GameModal,
            GlobalZIndex(165),
            TabGroup {
                order: 2,
                modal: true,
            },
            Node {
                position_type: PositionType::Absolute,
                width: percent(100.),
                height: percent(100.),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0., 0., 0., 0.45)),
        ))
        .id();
    let panel = commands
        .spawn((
            crate::bevy_shell::window_drag::DraggableDialog,
            source::frame(skins.frame.clone(), [4., 17., 4., 4.]),
            ScrollArea,
            Node {
                width: px(780.),
                max_width: percent(95.),
                max_height: percent(95.),
                overflow: Overflow::scroll_y(),
                padding: UiRect {
                    left: px(12.),
                    right: px(12.),
                    top: px(24.),
                    bottom: px(10.),
                },
                flex_direction: FlexDirection::Column,
                row_gap: px(6.),
                ..default()
            },
        ))
        .id();
    commands.entity(root).add_child(panel);
    source::dialog_caption(commands, panel, "Houses", skins.font.clone());
    let Some(snapshot) = &state.snapshot else {
        return root;
    };
    let values = state.draft.as_ref().unwrap_or(&snapshot.fields);
    let ownership_values = state
        .ownership_draft
        .as_ref()
        .unwrap_or(&snapshot.ownership_fields);
    let busy = snapshot.model.pending.is_some() || snapshot.ownership.pending();
    if !snapshot.model.selected {
        text(commands, skins, panel, "Unavailable in this session");
    }
    if snapshot.ownership_open {
        text(
            commands,
            skins,
            panel,
            oteryn_client_ui::house::ownership_text(&snapshot.ownership),
        );
        if let Some(review) = &state.review {
            text(commands, skins, panel, review.text());
            button(
                commands,
                skins,
                panel,
                state,
                Kind::Confirm,
                "Confirm house action",
                !busy,
            );
            button(
                commands,
                skins,
                panel,
                state,
                Kind::CancelReview,
                "Cancel",
                !busy,
            );
        } else {
            if let Some(data) = snapshot.ownership.data() {
                for (index, row) in data.rows.iter().enumerate() {
                    button(
                        commands,
                        skins,
                        panel,
                        state,
                        Kind::Select(index),
                        &format!(
                            "{}{}",
                            if snapshot.ownership.index() == index {
                                "Selected: "
                            } else {
                                ""
                            },
                            row.name
                        ),
                        !busy,
                    );
                }
            }
            for (index, label) in [
                "Recipient name",
                "Date (YYYY-MM-DD HH:MM:SS UTC)",
                "Gold price",
            ]
            .iter()
            .enumerate()
            {
                field(
                    commands,
                    skins,
                    panel,
                    index + 7,
                    label,
                    &ownership_values[index],
                    !busy,
                );
            }
            text(
                commands,
                skins,
                panel,
                "Choose a date 24 hours to 30 days ahead.",
            );
            let buttons = row(commands, panel);
            for (direction, label, enabled) in [
                (0, "Refresh", true),
                (-1, "Previous page", snapshot.previous),
                (
                    1,
                    "Next page",
                    snapshot.ownership.data().is_some_and(|d| d.has_more),
                ),
            ] {
                button(
                    commands,
                    skins,
                    buttons,
                    state,
                    Kind::Page(direction),
                    label,
                    !busy && enabled,
                );
            }
            let buttons = row(commands, panel);
            for (index, label) in [
                "Schedule move-out",
                "Cancel move-out",
                "Offer transfer",
                "Cancel transfer",
                "Accept transfer",
                "Reject transfer",
            ]
            .iter()
            .enumerate()
            {
                let action = (index + 1) as u8;
                let enabled = snapshot
                    .ownership
                    .row()
                    .is_some_and(|r| r.allowed & (1 << action) != 0);
                button(
                    commands,
                    skins,
                    buttons,
                    state,
                    Kind::Review(action),
                    label,
                    !busy && enabled,
                );
            }
        }
        button(
            commands,
            skins,
            panel,
            state,
            Kind::Back,
            "Back to listings",
            !busy,
        );
    } else {
        text(
            commands,
            skins,
            panel,
            oteryn_client_ui::house::panel_text(&snapshot.model),
        );
        for (index, label) in [
            "World UUID",
            "Town key",
            "House row (1 onwards)",
            "Bid amount (gold)",
            "Member UUIDs (space/newline separated)",
            "Door: x y floor (current ACL revision)",
            "Type CONFIRM for changes",
        ]
        .iter()
        .enumerate()
        {
            field(commands, skins, panel, index, label, &values[index], !busy);
        }
        let buttons = row(commands, panel);
        for (selection, label) in [
            (0, "Query"),
            (1, "Next"),
            (2, "Bid"),
            (3, "Guests"),
            (4, "Subowners"),
            (5, "Door"),
            (6, "Kick"),
            (7, "Leave"),
            (11, "Load guests"),
            (12, "Load subowners"),
            (13, "Load door"),
        ] {
            button(
                commands,
                skins,
                buttons,
                state,
                Kind::Request(selection),
                label,
                !busy && snapshot.model.selected,
            );
        }
        button(
            commands,
            skins,
            buttons,
            state,
            Kind::Map,
            "Show entrance map",
            !busy,
        );
        button(
            commands,
            skins,
            buttons,
            state,
            Kind::Ownership,
            "Ownership and move-out",
            !busy && snapshot.ownership.available(),
        );
        if state.map {
            map(commands, skins, panel, &snapshot.model);
        }
    }
    if busy {
        text(commands, skins, panel, "Waiting for the server");
    }
    if !state.feedback.is_empty() {
        text(commands, skins, panel, &state.feedback);
    }
    button(commands, skins, panel, state, Kind::Close, "Close", true);
    root
}
fn map(
    commands: &mut Commands,
    skins: &Skins,
    parent: Entity,
    model: &oteryn_client_ui::house::Panel,
) {
    let houses = oteryn_client_ui::house::map_houses(model);
    text(
        commands,
        skins,
        parent,
        "House entrances — north up; selected entrance ◆; other entrances ⌂",
    );
    if houses.is_empty() {
        text(
            commands,
            skins,
            parent,
            "Query houses and select a row to view its floor",
        );
        return;
    }
    let canvas = commands
        .spawn((
            source::frame(skins.inset.clone(), [1.; 4]),
            Node {
                width: px(670.),
                height: px(420.),
                position_type: PositionType::Relative,
                ..default()
            },
        ))
        .id();
    commands.entity(parent).add_child(canvas);
    for house in houses {
        let (x, y, floor) = house.position;
        let entity = commands
            .spawn(source::text(
                format!(
                    "{} {}\n{x},{y},{floor}",
                    if house.selected { "◆" } else { "⌂" },
                    house.label
                ),
                source::font(skins.font.clone(), false),
            ))
            .insert(Node {
                position_type: PositionType::Absolute,
                left: px(house.pixel.0 as f32),
                top: px(house.pixel.1 as f32),
                width: px(220.),
                ..default()
            })
            .id();
        commands.entity(canvas).add_child(entity);
    }
}
fn row(commands: &mut Commands, parent: Entity) -> Entity {
    let entity = commands
        .spawn(Node {
            flex_wrap: FlexWrap::Wrap,
            column_gap: px(4.),
            row_gap: px(4.),
            ..default()
        })
        .id();
    commands.entity(parent).add_child(entity);
    entity
}
fn text(commands: &mut Commands, skins: &Skins, parent: Entity, value: impl Into<String>) {
    let entity = commands
        .spawn(source::text(value, source::font(skins.font.clone(), false)))
        .insert((
            TextLayout::default(),
            Node {
                max_width: percent(100.),
                flex_shrink: 0.,
                ..default()
            },
        ))
        .id();
    commands.entity(parent).add_child(entity);
}
fn field(
    commands: &mut Commands,
    skins: &Skins,
    parent: Entity,
    index: usize,
    label: &str,
    value: &str,
    enabled: bool,
) {
    text(commands, skins, parent, label);
    let mut editable = EditableText::new(value);
    editable.max_characters = Some(match index {
        7 => 29,
        8 => 40,
        9 => 15,
        _ => 8192,
    });
    editable.allow_newlines = index == 4;
    let mut entity = commands.spawn((
        Field(index),
        TextInput,
        editable,
        source::font(skins.font.clone(), false),
        TextColor(source::TEXT_COLOR),
        TextLayout::default(),
        TextCursorStyle::default(),
        TabIndex(index as i32),
        source::frame(skins.inset.clone(), [1.; 4]),
        Node {
            width: percent(100.),
            min_height: px(25.),
            max_height: px(70.),
            padding: UiRect::all(px(4.)),
            ..default()
        },
    ));
    if !enabled {
        entity.insert(InteractionDisabled);
    }
    let entity = entity.id();
    commands.entity(parent).add_child(entity);
}
fn button(
    commands: &mut Commands,
    skins: &Skins,
    parent: Entity,
    state: &State,
    kind: Kind,
    caption: &str,
    enabled: bool,
) {
    let mut entity = commands.spawn((
        source::button(skins.buttons.clone(), 150.),
        Control {
            generation: state.generation,
            revision: state.revision,
            kind,
        },
    ));
    if !enabled {
        entity.insert(InteractionDisabled);
    }
    let entity = entity.observe(activate).id();
    let label = commands
        .spawn(source::button_caption(caption, skins.font.clone()))
        .id();
    commands.entity(entity).add_child(label);
    commands.entity(parent).add_child(entity);
}
