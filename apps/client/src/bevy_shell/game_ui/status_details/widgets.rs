use super::*;
use bevy::{
    input_focus::tab_navigation::TabGroup, ui::InteractionDisabled, ui_widgets::ScrollArea,
};
use oteryn_client_ui_bevy as source;
pub(super) struct Widgets {
    pub root: Entity,
    pub body: Option<Entity>,
}
pub(super) fn build(commands: &mut Commands, skins: &Skins) -> Widgets {
    let root = commands
        .spawn((
            Name::new("status_details.window"),
            crate::bevy_shell::window_drag::DraggableDialog,
            TabGroup::new(3),
            GlobalZIndex(120),
            source::frame(skins.frame.clone(), [4., 17., 4., 4.]),
            Node {
                position_type: PositionType::Absolute,
                top: percent(15.),
                left: percent(20.),
                width: px(650.),
                max_width: percent(90.),
                max_height: percent(80.),
                padding: UiRect {
                    left: px(12.),
                    right: px(12.),
                    top: px(24.),
                    bottom: px(10.),
                },
                flex_direction: FlexDirection::Column,
                row_gap: px(8.),
                ..default()
            },
        ))
        .id();
    Widgets { root, body: None }
}
pub(super) fn render(
    commands: &mut Commands,
    skins: &Skins,
    widgets: &mut Widgets,
    key: Key,
    panel: &Panel,
    tab: Tab,
    scroll: ScrollPosition,
) {
    let root = widgets.root;
    commands.entity(root).despawn_children();
    source::dialog_caption(
        commands,
        root,
        "Status details — Ctrl+Alt+Shift+F11",
        skins.font.clone(),
    );
    let tabs = commands
        .spawn(Node {
            flex_wrap: FlexWrap::Wrap,
            column_gap: px(5.),
            ..default()
        })
        .id();
    commands.entity(root).add_child(tabs);
    for (target, caption) in [
        (Tab::Party, "Party effects"),
        (Tab::Potions, "Potion durations"),
        (Tab::Mechanics, "Character effects"),
    ] {
        button(
            commands,
            skins,
            tabs,
            key,
            Action::Open(target),
            caption,
            target == tab,
        );
    }
    let body = commands
        .spawn((
            source::frame(skins.inset.clone(), [1.; 4]),
            ScrollArea,
            scroll,
            Node {
                min_height: px(0.),
                height: px(360.),
                overflow: Overflow::scroll_y(),
                padding: UiRect::all(px(6.)),
                ..default()
            },
        ))
        .id();
    widgets.body = Some(body);
    let text = commands
        .spawn((
            Body,
            Name::new("status_details.text"),
            source::text(
                &panel.text_at(tab, Instant::now()).replace("\r\n", "\n"),
                source::font(skins.font.clone(), false),
            ),
        ))
        .id();
    commands.entity(body).add_child(text);
    commands.entity(root).add_child(body);
    let footer = commands
        .spawn(Node {
            column_gap: px(5.),
            justify_content: JustifyContent::End,
            ..default()
        })
        .id();
    commands.entity(root).add_child(footer);
    button(
        commands,
        skins,
        footer,
        key,
        Action::Refresh(tab),
        "Refresh",
        !panel.ready(tab),
    );
    button(
        commands,
        skins,
        footer,
        key,
        Action::Close(tab),
        "Close",
        false,
    );
}
// The button's parent, binding and caption are independent presentation values.
#[allow(clippy::too_many_arguments)]
fn button(
    commands: &mut Commands,
    skins: &Skins,
    parent: Entity,
    key: Key,
    action: Action,
    caption: &str,
    disabled: bool,
) {
    let button = commands
        .spawn((
            source::button(skins.buttons.clone(), 145.),
            Control { key, action },
        ))
        .observe(activate)
        .id();
    if disabled {
        commands.entity(button).insert(InteractionDisabled);
    }
    let caption = commands
        .spawn(source::button_caption(caption, skins.font.clone()))
        .id();
    commands.entity(button).add_child(caption);
    commands.entity(parent).add_child(button);
}
