use super::*;
use bevy::{
    input_focus::tab_navigation::TabGroup,
    ui::{Checked, InteractionDisabled},
    ui_widgets::ScrollArea,
};
use oteryn_client_ui_bevy as source;
pub(super) struct Widgets {
    pub overlay: Entity,
    pub panel: Entity,
    pub content: Option<Entity>,
}
pub(super) fn build(commands: &mut Commands, skin: &Skins) -> Widgets {
    let overlay = commands
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
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0., 0., 0., 0.45)),
        ))
        .id();
    let panel = commands
        .spawn((
            crate::bevy_shell::window_drag::DraggableDialog,
            Name::new("cyclopedia.window"),
            source::frame(skin.frame.clone(), [4., 17., 4., 4.]),
            // Tibia Global Cyclopedia: 700 wide, 36px tab bar over a 420px tab page.
            crate::bevy_shell::ui::window_layout::window(
                "cyclopedia",
                Node {
                    max_width: percent(96.),
                    max_height: percent(94.),
                    padding: UiRect {
                        top: px(24.),
                        bottom: px(10.),
                        left: px(10.),
                        right: px(10.),
                    },
                    flex_direction: FlexDirection::Column,
                    ..default()
                },
            ),
        ))
        .id();
    commands.entity(overlay).add_child(panel);
    source::dialog_caption(commands, panel, "Cyclopedia", skin.font.clone());
    Widgets {
        overlay,
        panel,
        content: None,
    }
}
pub(super) fn render(
    commands: &mut Commands,
    skin: &Skins,
    parent: Entity,
    control: Control,
    tab: Tab,
    panel: &CyclopediaCharacterPanel,
) -> Entity {
    let content = commands
        .spawn(Node {
            width: percent(100.),
            flex_grow: 1.,
            min_height: px(0.),
            flex_direction: FlexDirection::Column,
            row_gap: px(6.),
            ..default()
        })
        .id();
    commands.entity(parent).add_child(content);
    let tabs = commands
        .spawn((
            Name::new("cyclopedia.tabs"),
            ScrollArea,
            crate::bevy_shell::ui::window_layout::Scaled {
                height: Some(36.),
                ..default()
            },
            Node {
                width: percent(100.),
                height: px(36.),
                flex_shrink: 0.,
                column_gap: px(4.),
                align_items: AlignItems::Center,
                overflow: Overflow::scroll_x(),
                ..default()
            },
        ))
        .id();
    commands.entity(content).add_child(tabs);
    let page = commands
        .spawn((
            Name::new("cyclopedia.page"),
            Node {
                width: percent(100.),
                height: px(420.),
                // The page takes what the user gives the window and scrolls below.
                flex_grow: 1.,
                flex_shrink: 1.,
                min_height: px(0.),
                flex_direction: FlexDirection::Column,
                row_gap: px(6.),
                overflow: Overflow::clip(),
                ..default()
            },
        ))
        .id();
    commands.entity(content).add_child(page);
    let (body, content) = (content, page);
    for choice in Tab::ALL {
        let button = sized_button(
            commands,
            skin,
            tabs,
            Control {
                kind: Kind::Tab(choice),
                ..control
            },
            choice.label(),
            panel.pending || !panel.selected,
            if choice == tab { 200. } else { 140. },
        );
        if choice == tab {
            commands.entity(button).insert(Checked);
        }
    }
    text(
        commands,
        skin,
        content,
        if panel.pending {
            "Waiting for the server"
        } else if !panel.selected {
            "Feature unavailable in this session"
        } else {
            tab.label()
        },
    );
    if let Some(feedback) = panel.feedback {
        text(commands, skin, content, feedback);
    }
    if let Some(error) = panel
        .achievements
        .error
        .filter(|_| tab == Tab::Character(CharacterTab::Achievements))
    {
        text(commands, skin, content, error);
    }
    if tab == Tab::Items {
        use oteryn_client_runtime::cyclopedia_character::items::Location;
        let filters = commands
            .spawn(Node {
                width: percent(100.),
                column_gap: px(4.),
                flex_wrap: FlexWrap::Wrap,
                ..default()
            })
            .id();
        commands.entity(content).add_child(filters);
        let search = commands
            .spawn((
                HoldingsSearch,
                bevy::ui_widgets::TextInput,
                bevy::text::EditableText::new(&panel.items.search),
                source::font(skin.font.clone(), false),
                TextColor(source::TEXT_COLOR),
                Node {
                    width: px(240.),
                    min_height: px(24.),
                    ..default()
                },
            ))
            .id();
        if panel.pending || !panel.holdings_selected {
            commands.entity(search).insert(InteractionDisabled);
        }
        commands.entity(filters).add_child(search);
        button(
            commands,
            skin,
            filters,
            Control {
                kind: Kind::HoldingsFilter,
                ..control
            },
            "Search holdings",
            panel.pending || !panel.holdings_selected,
        );
        for (location, label) in [
            (None, "All locations"),
            (Some(Location::Inventory), "Inventory"),
            (Some(Location::Depot), "Depot"),
            (Some(Location::Stash), "Stash"),
            (Some(Location::Inbox), "Inbox"),
        ] {
            let choice = button(
                commands,
                skin,
                filters,
                Control {
                    kind: Kind::HoldingsLocation(location),
                    ..control
                },
                label,
                panel.pending || !panel.holdings_selected,
            );
            if panel.items.location == location {
                commands.entity(choice).insert(Checked);
            }
        }
        if let Some(page) = &panel.items.page {
            text(
                commands,
                skin,
                content,
                format!(
                    "Page {} of {} — {} matching holdings",
                    usize::from(page.offset) / 24 + 1,
                    usize::from(page.total).div_ceil(24).max(1),
                    page.total
                ),
            );
        }
    }
    let rows = commands
        .spawn((
            ScrollArea,
            crate::bevy_shell::game_ui::install::RememberScroll("cyclopedia", 0),
            source::frame(skin.inset.clone(), [1.; 4]),
            Node {
                width: percent(100.),
                min_height: px(0.),
                flex_grow: 1.,
                padding: UiRect::all(px(6.)),
                flex_direction: FlexDirection::Column,
                row_gap: px(5.),
                overflow: Overflow::scroll(),
                ..default()
            },
        ))
        .id();
    commands.entity(content).add_child(rows);
    if matches!(
        tab,
        Tab::Character(CharacterTab::RecentDeaths | CharacterTab::RecentPvpKills)
    ) {
        text(
            commands,
            skin,
            rows,
            oteryn_client_ui::cyclopedia_death_details::history_columns(
                panel.extensions.death_details && tab == Tab::Character(CharacterTab::RecentDeaths),
            ),
        );
    }
    if matches!(
        tab,
        Tab::Familiars | Tab::Character(CharacterTab::Appearances)
    ) {
        text(
            commands,
            skin,
            rows,
            "Familiars are content catalogue entries, not account unlocks.",
        );
        if let Some(page) = &panel.gallery.page {
            text(
                commands,
                skin,
                rows,
                format!(
                    "Appearance catalogue: {}–{} of {}. Familiar filtering applies to this page.",
                    if page.total == 0 {
                        0
                    } else {
                        usize::from(page.offset) + 1
                    },
                    usize::from(page.offset) + page.entries.len(),
                    page.total,
                ),
            );
        }
    }
    let gallery = rows::gallery_entries(tab, panel);
    let projected = rows::project(tab, panel);
    if projected.is_empty() {
        text(
            commands,
            skin,
            rows,
            if tab == Tab::Familiars && tab.can_page(true, panel) {
                "No confirmed familiars on this appearance page. Use Next to browse further pages."
            } else if tab == Tab::Familiars {
                "No confirmed familiars on this appearance page."
            } else {
                "No confirmed rows"
            },
        );
    }
    for (index, row) in projected.into_iter().enumerate() {
        let parent = if let Some(entry) = gallery.get(index) {
            let card = commands
                .spawn(Node {
                    column_gap: px(8.),
                    align_items: AlignItems::Center,
                    flex_shrink: 0.,
                    ..default()
                })
                .id();
            let portrait = commands
                .spawn((
                    artwork::Portrait(entry.look_type),
                    Node {
                        width: px(96.),
                        height: px(96.),
                        flex_shrink: 0.,
                        ..default()
                    },
                ))
                .id();
            commands.entity(card).add_child(portrait);
            commands.entity(rows).add_child(card);
            card
        } else {
            rows
        };
        text(commands, skin, parent, row);
    }
    if tab == Tab::Badges && panel.target.is_none() && panel.badge_visibility_known {
        let toggles = commands
            .spawn(Node {
                flex_wrap: FlexWrap::Wrap,
                column_gap: px(4.),
                row_gap: px(4.),
                flex_shrink: 0.,
                ..default()
            })
            .id();
        commands.entity(rows).add_child(toggles);
        for row in model_badges(panel) {
            let label = if panel.server_hidden_badges.contains(&row.id) {
                format!("Show {} publicly", row.name)
            } else {
                format!("Hide {} publicly", row.name)
            };
            button(
                commands,
                skin,
                toggles,
                Control {
                    kind: Kind::BadgeVisibility(row.id),
                    ..control
                },
                &label,
                panel.pending || !panel.selected,
            );
        }
    }
    let footer = commands
        .spawn(Node {
            column_gap: px(6.),
            ..default()
        })
        .id();
    commands.entity(body).add_child(footer);
    for (kind, label) in [
        (Kind::Encyclopedia, "Item encyclopedia"),
        (Kind::Refresh, "Refresh"),
        (Kind::Previous, "Previous"),
        (Kind::Next, "Next"),
        (Kind::Close, "Close"),
    ] {
        let disabled = !matches!(kind, Kind::Close | Kind::Encyclopedia)
            && (panel.pending
                || !panel.selected
                || (matches!(kind, Kind::Previous | Kind::Next)
                    && !tab.can_page(kind == Kind::Next, panel)));
        button(
            commands,
            skin,
            footer,
            Control { kind, ..control },
            label,
            disabled,
        );
    }
    body
}
fn text(commands: &mut Commands, skin: &Skins, parent: Entity, value: impl Into<String>) {
    let row = commands
        .spawn((
            source::text(value, source::font(skin.font.clone(), false)),
            Node {
                flex_shrink: 0.,
                ..default()
            },
        ))
        .id();
    commands.entity(parent).add_child(row);
}
fn button(
    commands: &mut Commands,
    skin: &Skins,
    parent: Entity,
    control: Control,
    label: &str,
    disabled: bool,
) -> Entity {
    sized_button(commands, skin, parent, control, label, disabled, 140.)
}
fn sized_button(
    commands: &mut Commands,
    skin: &Skins,
    parent: Entity,
    control: Control,
    label: &str,
    disabled: bool,
    width: f32,
) -> Entity {
    let button = commands
        .spawn((source::button(skin.buttons.clone(), width), control))
        .observe(activate)
        .id();
    if disabled {
        commands.entity(button).insert(InteractionDisabled);
    }
    let caption = commands
        .spawn(source::button_caption(label, skin.font.clone()))
        .id();
    commands.entity(button).add_child(caption);
    commands.entity(parent).add_child(button);
    button
}
/// The player's own earned badges, the ones whose public visibility can change.
fn model_badges(
    panel: &CyclopediaCharacterPanel,
) -> Vec<oteryn_client_ui::cyclopedia_character::BadgeRow> {
    oteryn_client_ui::cyclopedia_character::badge_rows(panel)
        .into_iter()
        .filter(|row| row.earned)
        .collect()
}
