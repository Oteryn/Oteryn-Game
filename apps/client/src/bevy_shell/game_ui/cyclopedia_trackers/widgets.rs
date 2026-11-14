use super::*;
use bevy::text::TextCursorStyle;
use bevy::{
    input_focus::tab_navigation::{TabGroup, TabIndex},
    ui::InteractionDisabled,
    ui_widgets::{ScrollArea, TextInput},
};
use oteryn_client_ui::cyclopedia_tracker::tracker_rows;
use oteryn_client_ui_bevy as source;
pub(super) struct Widgets {
    pub root: Entity,
    pub fields: [Entity; 3],
    body: Entity,
    searches: [Entity; 2],
    content: Option<Entity>,
}
pub(super) fn build(commands: &mut Commands, skins: &Skins, panel: &Panel) -> Widgets {
    let root = commands
        .spawn((
            Overlay,
            GlobalZIndex(90),
            TabGroup {
                order: 2,
                modal: false,
            },
            crate::bevy_shell::window_drag::DraggableDialog,
            source::frame(skins.frame.clone(), [4., 17., 4., 4.]),
            Node {
                position_type: PositionType::Absolute,
                left: percent(10.),
                top: percent(10.),
                width: percent(80.),
                max_height: percent(85.),
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
    source::dialog_caption(
        commands,
        root,
        "Cyclopedia trackers (F8)",
        skins.font.clone(),
    );
    label(commands, skins, root, "Creature key to add/remove");
    let key = field(commands, skins, root, Field::Key, &panel.creature_key);
    let searches = row(commands, root);
    let bestiary = column(commands, searches);
    label(
        commands,
        skins,
        bestiary,
        "Search tracked Bestiary creatures",
    );
    let search = field(
        commands,
        skins,
        bestiary,
        Field::Search(false),
        &panel.search,
    );
    let bosstiary = column(commands, searches);
    label(
        commands,
        skins,
        bosstiary,
        "Search tracked Bosstiary creatures",
    );
    let boss_search = field(
        commands,
        skins,
        bosstiary,
        Field::Search(true),
        &panel.boss_search,
    );
    let body = commands
        .spawn((
            ScrollArea,
            Node {
                max_height: px(470.),
                min_height: px(0.),
                overflow: Overflow::scroll_y(),
                flex_direction: FlexDirection::Column,
                row_gap: px(6.),
                ..default()
            },
        ))
        .id();
    commands.entity(root).add_child(body);
    Widgets {
        root,
        fields: [key, search, boss_search],
        body,
        searches: [bestiary, bosstiary],
        content: None,
    }
}
pub(super) fn render(
    commands: &mut Commands,
    skins: &Skins,
    widgets: &mut Widgets,
    generation: u64,
    revision: u64,
    panel: &Panel,
    confirmation: Option<&str>,
) {
    if let Some(content) = widgets.content.take() {
        commands.entity(content).despawn();
    }
    let content = column(commands, widgets.body);
    widgets.content = Some(content);
    let controls = row(commands, content);
    let enabled = panel.selected && !panel.pending && confirmation.is_none();
    let button = |commands: &mut Commands, parent, kind, label: &str, enabled| {
        button(
            commands,
            skins,
            parent,
            Control {
                generation,
                revision,
                kind,
            },
            label,
            enabled,
        )
    };
    button(
        commands,
        controls,
        Kind::Boss,
        if panel.boss {
            "Editing: Bosstiary"
        } else {
            "Editing: Bestiary"
        },
        enabled,
    );
    for (intent, caption) in [
        (Intent::Add, "Add"),
        (Intent::Remove, "Remove"),
        (Intent::Read, "Refresh"),
    ] {
        button(commands, controls, Kind::Request(intent), caption, enabled);
    }
    if let Some(key) = confirmation {
        label(
            commands,
            skins,
            content,
            format!("Remove {key} from this tracker?"),
        );
        let confirm = row(commands, content);
        button(commands, confirm, Kind::Confirm, "Confirm removal", true);
        button(commands, confirm, Kind::Cancel, "Cancel", true);
    }
    label(
        commands,
        skins,
        content,
        oteryn_client_ui::cyclopedia_tracker::panel_text(panel),
    );
    let displays = row(commands, content);
    button(
        commands,
        displays,
        Kind::Widget(false),
        if panel.bestiary_widget_open {
            "Close Bestiary widget"
        } else {
            "Bestiary widget"
        },
        enabled,
    );
    button(
        commands,
        displays,
        Kind::Widget(true),
        if panel.bosstiary_widget_open {
            "Close Bosstiary widget"
        } else {
            "Bosstiary widget"
        },
        enabled,
    );
    for (index, visible) in [panel.bestiary_widget_open, panel.bosstiary_widget_open]
        .into_iter()
        .enumerate()
    {
        commands.entity(widgets.searches[index]).insert(Node {
            display: if visible {
                Display::Flex
            } else {
                Display::None
            },
            flex_direction: FlexDirection::Column,
            flex_grow: 1.,
            min_width: px(0.),
            flex_shrink: 0.,
            row_gap: px(4.),
            ..default()
        });
    }
    for (boss, title) in [(false, "Bestiary tracker"), (true, "Bosstiary tracker")] {
        if if boss {
            !panel.bosstiary_widget_open
        } else {
            !panel.bestiary_widget_open
        } {
            continue;
        }
        label(commands, skins, content, title);
        let filters = row(commands, content);
        for (sort, caption) in [
            (TrackerSort::Name, "Name"),
            (TrackerSort::Kills, "Most kills"),
            (TrackerSort::Remaining, "Nearest completion"),
        ] {
            button(commands, filters, Kind::Sort(boss, sort), caption, enabled);
        }
        let hide = if boss {
            panel.boss_hide_completed
        } else {
            panel.hide_completed
        };
        button(
            commands,
            filters,
            Kind::Hide(boss),
            if hide {
                "Show completed"
            } else {
                "Hide completed"
            },
            enabled,
        );
        let sort = if boss { panel.boss_sort } else { panel.sort };
        label(
            commands,
            skins,
            content,
            format!(
                "Sort: {}",
                match sort {
                    TrackerSort::Name => "Name",
                    TrackerSort::Kills => "Most kills",
                    TrackerSort::Remaining => "Nearest completion",
                }
            ),
        );
        if if boss {
            panel.bosstiary_snapshot.is_none()
        } else {
            panel.bestiary_snapshot.is_none()
        } {
            label(commands, skins, content, "No server snapshot yet");
        }
        for entry in tracker_rows(panel, boss) {
            let goal = entry.thresholds[2];
            let percent = if goal == 0 {
                0
            } else {
                (u64::from(entry.kill_count) * 100 / u64::from(goal)).min(100)
            };
            let row = row(commands, content);
            label(
                commands,
                skins,
                row,
                format!(
                    "{}   {} / {} kills ({}%)   stages {} / {} / {}{}",
                    entry.creature_key,
                    entry.kill_count,
                    goal,
                    percent,
                    entry.thresholds[0],
                    entry.thresholds[1],
                    goal,
                    if entry.completed { " — Complete" } else { "" }
                ),
            );
            button(
                commands,
                row,
                Kind::Remove(boss, entry.creature_key.clone()),
                "Remove selected",
                enabled,
            );
        }
    }
    if !panel.selected {
        label(
            commands,
            skins,
            content,
            "Feature unavailable in this session",
        );
    }
    if panel.pending {
        label(commands, skins, content, "Waiting for the server");
    }
    if let Some(feedback) = panel.feedback {
        label(commands, skins, content, feedback);
    }
    button(commands, content, Kind::Close, "Close", true);
}
fn column(commands: &mut Commands, parent: Entity) -> Entity {
    let entity = commands
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            flex_grow: 1.,
            min_width: px(0.),
            flex_shrink: 0.,
            row_gap: px(4.),
            ..default()
        })
        .id();
    commands.entity(parent).add_child(entity);
    entity
}
fn row(commands: &mut Commands, parent: Entity) -> Entity {
    let entity = commands
        .spawn(Node {
            flex_wrap: FlexWrap::Wrap,
            column_gap: px(5.),
            row_gap: px(3.),
            flex_shrink: 0.,
            ..default()
        })
        .id();
    commands.entity(parent).add_child(entity);
    entity
}
fn label(commands: &mut Commands, skins: &Skins, parent: Entity, value: impl Into<String>) {
    let entity = commands
        .spawn(source::text(value, source::font(skins.font.clone(), false)))
        .insert(TextLayout::default())
        .id();
    commands.entity(parent).add_child(entity);
}
fn button(
    commands: &mut Commands,
    skins: &Skins,
    parent: Entity,
    control: Control,
    caption: &str,
    enabled: bool,
) {
    let mut entity = commands.spawn((source::button(skins.buttons.clone(), 145.), control));
    if !enabled {
        entity.insert(InteractionDisabled);
    }
    let entity = entity.observe(activate).id();
    let caption = commands
        .spawn(source::button_caption(caption, skins.font.clone()))
        .id();
    commands.entity(entity).add_child(caption);
    commands.entity(parent).add_child(entity);
}
fn field(
    commands: &mut Commands,
    skins: &Skins,
    parent: Entity,
    field: Field,
    value: &str,
) -> Entity {
    let mut text = EditableText::new(value);
    text.max_characters = Some(128);
    let entity = commands
        .spawn((
            field,
            TextInput,
            text,
            source::font(skins.font.clone(), false),
            TextColor(source::TEXT_COLOR),
            TextLayout::no_wrap(),
            TextCursorStyle::default(),
            TabIndex(0),
            source::frame(skins.inset.clone(), [1.; 4]),
            Node {
                width: percent(100.),
                height: px(25.),
                padding: px(4.).all(),
                flex_shrink: 0.,
                ..default()
            },
        ))
        .id();
    commands.entity(parent).add_child(entity);
    entity
}
