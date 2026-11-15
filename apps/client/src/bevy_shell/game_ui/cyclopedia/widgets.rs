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
            source::frame(skin.frame.clone(), [4., 17., 4., 4.]),
            Node {
                width: px(1000.),
                height: px(650.),
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
        ))
        .id();
    commands.entity(overlay).add_child(panel);
    source::dialog_caption(commands, panel, "Cyclopedia character", skin.font.clone());
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
        .spawn(Node {
            width: percent(100.),
            flex_wrap: FlexWrap::Wrap,
            column_gap: px(4.),
            row_gap: px(4.),
            ..default()
        })
        .id();
    commands.entity(content).add_child(tabs);
    for choice in Tab::ALL {
        let button = button(
            commands,
            skin,
            tabs,
            Control {
                kind: Kind::Tab(choice),
                ..control
            },
            choice.label(),
            panel.pending || !panel.selected,
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
    let rows = commands
        .spawn((
            ScrollArea,
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
    let projected = rows::project(tab, panel);
    if projected.is_empty() {
        text(commands, skin, rows, "No confirmed rows");
    }
    for row in projected {
        text(commands, skin, rows, row);
    }
    let footer = commands
        .spawn(Node {
            column_gap: px(6.),
            ..default()
        })
        .id();
    commands.entity(content).add_child(footer);
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
                    && tab.page(kind == Kind::Next).is_none()));
        button(
            commands,
            skin,
            footer,
            Control { kind, ..control },
            label,
            disabled,
        );
    }
    content
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
    let button = commands
        .spawn((source::button(skin.buttons.clone(), 140.), control))
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
