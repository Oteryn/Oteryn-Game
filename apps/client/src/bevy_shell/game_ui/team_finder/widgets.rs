use super::*;
use bevy::{
    input_focus::tab_navigation::{TabGroup, TabIndex},
    text::TextCursorStyle,
    ui::InteractionDisabled,
    ui_widgets::{ScrollArea, TextInput},
};
use oteryn_client_ui::team_finder::{MemberStatus, TeamFinderResult};
use oteryn_client_ui_bevy as source;
pub(super) fn build(commands: &mut Commands, skins: &Skins, key: Key, panel: &Panel) -> Entity {
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
    let dialog = commands
        .spawn((
            crate::bevy_shell::window_drag::DraggableDialog,
            source::frame(skins.frame.clone(), [4., 17., 4., 4.]),
            Node {
                width: px(720.),
                max_width: percent(95.),
                max_height: percent(95.),
                padding: UiRect::all(px(12.)),
                flex_direction: FlexDirection::Column,
                row_gap: px(6.),
                ..default()
            },
        ))
        .id();
    commands.entity(root).add_child(dialog);
    source::dialog_caption(commands, dialog, "Social — Join Team", skins.font.clone());
    label(
        commands,
        skins,
        dialog,
        "Search this page by leader name and level (blank = any level). Filters apply to each page.",
    );
    for (field, value) in [
        (Field::Name, panel.filter.name.clone()),
        (
            Field::Level,
            panel
                .filter
                .level
                .map(|level| level.to_string())
                .unwrap_or_default(),
        ),
    ] {
        let mut text = EditableText::new(value);
        text.max_characters = Some(64);
        let input = commands
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
                    padding: UiRect::all(px(4.)),
                    ..default()
                },
            ))
            .id();
        commands.entity(dialog).add_child(input);
    }
    let enabled = panel.selected && panel.pending.is_none() && !panel.applicants;
    let filters = row(commands, dialog);
    button(
        commands,
        skins,
        filters,
        key,
        Kind::Filter(panel.filter.clone()),
        "Search",
        enabled,
    );
    let mut open = panel.filter.clone();
    open.only_open = !open.only_open;
    button(
        commands,
        skins,
        filters,
        key,
        Kind::Filter(open),
        if panel.filter.only_open {
            "Open slots only"
        } else {
            "All capacities"
        },
        enabled,
    );
    let mut target = panel.filter.clone();
    target.target = (target.target.min(3) + 1) % 4;
    button(
        commands,
        skins,
        filters,
        key,
        Kind::Filter(target),
        ["All activities", "Boss", "Hunt", "Quest"][usize::from(panel.filter.target.min(3))],
        enabled,
    );
    let list = commands
        .spawn((
            ScrollArea,
            Node {
                height: px(300.),
                min_height: px(0.),
                overflow: Overflow::scroll_y(),
                flex_direction: FlexDirection::Column,
                row_gap: px(4.),
                ..default()
            },
        ))
        .id();
    commands.entity(dialog).add_child(list);
    if !panel.selected {
        label(
            commands,
            skins,
            list,
            "Team discovery is unavailable in this session",
        );
    } else if panel.result.is_none() && panel.pending.is_none() {
        label(commands, skins, list, "Refresh to search for teams.");
    } else if panel.result.is_some() && panel.visible_indices().is_empty() {
        label(
            commands,
            skins,
            list,
            "No teams match on this page. Adjust filters or browse the next page.",
        );
    }
    for (index, name) in panel.visible_names().iter().enumerate() {
        let caption = format!(
            "{}{}",
            if index == panel.selected_row {
                "> "
            } else {
                ""
            },
            name
        );
        button(
            commands,
            skins,
            list,
            key,
            Kind::Select(index),
            &caption,
            enabled,
        );
    }
    label(commands, skins, dialog, &panel.feedback);
    let footer = row(commands, dialog);
    button(
        commands,
        skins,
        footer,
        key,
        Kind::Browse(false),
        "Refresh",
        enabled,
    );
    button(
        commands,
        skins,
        footer,
        key,
        Kind::Browse(true),
        "Next page",
        enabled && panel.next().is_some(),
    );
    let selected = match &panel.result {
        Some(TeamFinderResult::Browse { listings, .. }) => panel
            .visible_indices()
            .get(panel.selected_row)
            .and_then(|index| listings.get(*index)),
        _ => None,
    };
    let apply = selected.is_some_and(|row| {
        matches!(row.status, MemberStatus::None | MemberStatus::Denied)
            && row.occupied_slots < row.listing.slots
    });
    let cancel = selected.is_some_and(|row| {
        matches!(
            row.status,
            MemberStatus::Pending | MemberStatus::Invited | MemberStatus::Accepted
        )
    });
    button(
        commands,
        skins,
        footer,
        key,
        Kind::Apply(false),
        "Apply to team",
        enabled && apply,
    );
    button(
        commands,
        skins,
        footer,
        key,
        Kind::Apply(true),
        "Cancel application",
        enabled && cancel,
    );
    button(commands, skins, footer, key, Kind::Close, "Close", true);
    root
}
fn label(commands: &mut Commands, skins: &Skins, parent: Entity, value: &str) {
    let child = commands
        .spawn(source::text(value, source::font(skins.font.clone(), false)))
        .id();
    commands.entity(parent).add_child(child);
}
fn row(commands: &mut Commands, parent: Entity) -> Entity {
    let child = commands
        .spawn(Node {
            column_gap: px(5.),
            flex_wrap: FlexWrap::Wrap,
            row_gap: px(4.),
            ..default()
        })
        .id();
    commands.entity(parent).add_child(child);
    child
}
fn button(
    commands: &mut Commands,
    skins: &Skins,
    parent: Entity,
    key: Key,
    kind: Kind,
    caption: &str,
    enabled: bool,
) {
    let listing_row = matches!(kind, Kind::Select(_));
    let mut entity = commands.spawn((
        source::button(skins.buttons.clone(), 150.),
        Control { key, kind },
    ));
    if listing_row {
        entity.insert(Node {
            width: percent(100.),
            min_height: px(24.),
            padding: UiRect::all(px(4.)),
            flex_shrink: 0.,
            ..default()
        });
    }
    if !enabled {
        entity.insert(InteractionDisabled);
    }
    let child = entity.observe(activate).id();
    commands.entity(parent).add_child(child);
    let text = commands
        .spawn(source::button_caption(caption, skins.font.clone()))
        .id();
    if listing_row {
        commands.entity(text).insert((
            TextLayout::default(),
            Node {
                max_width: percent(100.),
                flex_shrink: 1.,
                ..default()
            },
        ));
    }
    commands.entity(child).add_child(text);
}
