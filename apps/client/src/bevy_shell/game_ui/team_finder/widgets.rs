use super::*;
use bevy::{
    input_focus::tab_navigation::{TabGroup, TabIndex},
    text::TextCursorStyle,
    ui::InteractionDisabled,
    ui_widgets::{ScrollArea, TextInput},
};
use oteryn_client_ui::team_finder::{MemberStatus, TeamFinderResult};
use oteryn_client_ui::team_finder_leader::{LeaderListing, TeamTarget};
use oteryn_client_ui_bevy as source;
/// The leader listing editors, in the field order the session parses.
pub(super) const LISTING_FIELDS: [&str; 9] = [
    "Minimum level",
    "Maximum level",
    "Vocation mask (0 = any)",
    "Team size",
    "Free slots",
    "Party members join (0 or 1)",
    "Activity (1 boss, 2 hunt, 3 quest)",
    "Boss, hunt or quest id",
    "Hunting area (hunts only)",
];
pub(super) fn build(
    commands: &mut Commands,
    skins: &Skins,
    key: Key,
    panel: &Panel,
    leader: &Leader,
) -> Entity {
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
    let enabled = panel.selected && panel.pending.is_none();
    if panel.applicants {
        source::dialog_caption(commands, dialog, "Social — Manage Team", skins.font.clone());
        manage(commands, skins, key, dialog, panel, leader, enabled);
        return root;
    }
    source::dialog_caption(commands, dialog, "Social — Join Team", skins.font.clone());
    let tabs = row(commands, dialog);
    button(
        commands,
        skins,
        tabs,
        key,
        Kind::Applicants(false),
        "Manage Team",
        enabled,
    );
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
        input(commands, skins, dialog, field, value);
    }
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
/// The leader's applicants page and advertisement editor.
fn manage(
    commands: &mut Commands,
    skins: &Skins,
    key: Key,
    dialog: Entity,
    panel: &Panel,
    leader: &Leader,
    ready: bool,
) {
    let tabs = row(commands, dialog);
    button(
        commands,
        skins,
        tabs,
        key,
        Kind::Browse(false),
        "Join Team",
        ready,
    );
    label(
        commands,
        skins,
        dialog,
        "Players who applied to or were invited to your team. Select one to decide.",
    );
    let list = commands
        .spawn((
            ScrollArea,
            Node {
                height: px(180.),
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
        label(commands, skins, list, "Refresh to see your applicants.");
    } else if panel.result.is_some() && panel.visible_indices().is_empty() {
        label(commands, skins, list, "No applicants on this page.");
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
            ready,
        );
    }
    label(commands, skins, dialog, &panel.feedback);
    let decisions = row(commands, dialog);
    button(
        commands,
        skins,
        decisions,
        key,
        Kind::Applicants(false),
        "Refresh",
        ready,
    );
    button(
        commands,
        skins,
        decisions,
        key,
        Kind::Applicants(true),
        "Next page",
        ready && panel.next().is_some(),
    );
    let chosen = ready && panel.handle().is_some();
    for (action, caption) in [(2, "Invite"), (3, "Accept"), (4, "Decline")] {
        button(
            commands,
            skins,
            decisions,
            key,
            Kind::Decide(action),
            caption,
            chosen,
        );
    }
    let current = match &panel.result {
        Some(TeamFinderResult::Applicants { listing, .. }) => (*listing).or(leader.confirmed),
        _ => leader.confirmed,
    };
    label(commands, skins, dialog, "Your team listing");
    let values = listing_values(current);
    for (index, (caption, value)) in LISTING_FIELDS.iter().zip(values).enumerate() {
        let line = row(commands, dialog);
        label(commands, skins, line, caption);
        input(commands, skins, line, Field::Listing(index), value);
    }
    label(commands, skins, dialog, &leader.feedback);
    let editable = leader.selected && !leader.pending;
    let footer = row(commands, dialog);
    button(
        commands,
        skins,
        footer,
        key,
        Kind::Publish(Vec::new()),
        "Publish listing",
        editable,
    );
    button(
        commands,
        skins,
        footer,
        key,
        Kind::Withdraw,
        "Remove listing",
        editable && current.is_some(),
    );
    button(commands, skins, footer, key, Kind::Close, "Close", true);
}
/// The editor values for a listing, empty without one.
fn listing_values(listing: Option<LeaderListing>) -> [String; 9] {
    let Some(listing) = listing else {
        return Default::default();
    };
    let (activity, id, area) = match listing.target {
        TeamTarget::Boss(id) => (1, id, String::new()),
        TeamTarget::Hunt { kind, area } => (2, kind, area.to_string()),
        TeamTarget::Quest(id) => (3, id, String::new()),
    };
    [
        listing.min_level.to_string(),
        listing.max_level.to_string(),
        listing.vocation_mask.to_string(),
        listing.slots.to_string(),
        listing.free_slots.to_string(),
        u8::from(listing.party_members_join).to_string(),
        activity.to_string(),
        id.to_string(),
        area,
    ]
}
fn input(commands: &mut Commands, skins: &Skins, parent: Entity, field: Field, value: String) {
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
    commands.entity(parent).add_child(input);
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
        Name::new(format!("Team Finder: {caption}")),
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
