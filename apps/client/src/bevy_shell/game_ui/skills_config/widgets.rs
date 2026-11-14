use super::*;
use oteryn_client_ui_bevy as source;
pub(super) struct Widgets {
    pub overlay: Entity,
    pub panel: Entity,
    pub content: Option<Entity>,
}
pub(super) fn build(commands: &mut Commands, skins: &Skins) -> Widgets {
    let overlay = commands
        .spawn((
            Overlay,
            GameModal,
            GlobalZIndex(165),
            bevy::input_focus::tab_navigation::TabGroup {
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
            Node {
                width: px(740.),
                max_width: percent(95.),
                max_height: percent(95.),
                padding: UiRect::axes(px(12.), px(24.)),
                flex_direction: FlexDirection::Column,
                overflow: Overflow::scroll_y(),
                ..default()
            },
            bevy::ui_widgets::ScrollArea,
        ))
        .id();
    commands.entity(overlay).add_child(panel);
    source::dialog_caption(
        commands,
        panel,
        "Skills / loyalty / progress",
        skins.font.clone(),
    );
    Widgets {
        overlay,
        panel,
        content: None,
    }
}
fn text(commands: &mut Commands, skins: &Skins, parent: Entity, caption: impl Into<String>) {
    let child = commands
        .spawn(source::text(
            caption.into(),
            source::font(skins.font.clone(), false),
        ))
        .id();
    commands.entity(parent).add_child(child);
}
fn button(
    commands: &mut Commands,
    skins: &Skins,
    parent: Entity,
    key: Key,
    kind: Kind,
    caption: &str,
) {
    let child = commands
        .spawn((
            source::button(skins.buttons.clone(), 160.),
            Control { key, kind },
        ))
        .observe(activate)
        .id();
    let label = commands
        .spawn(source::button_caption(caption, skins.font.clone()))
        .id();
    commands.entity(child).add_child(label);
    commands.entity(parent).add_child(child);
}
fn bar(commands: &mut Commands, parent: Entity, value: u16) {
    let track = commands
        .spawn((
            Node {
                width: percent(100.),
                height: px(5.),
                ..default()
            },
            BackgroundColor(Color::BLACK),
        ))
        .id();
    let fill = commands
        .spawn((
            Node {
                width: percent(f32::from(value) / 100.),
                height: percent(100.),
                ..default()
            },
            BackgroundColor(Color::WHITE),
        ))
        .id();
    commands.entity(track).add_child(fill);
    commands.entity(parent).add_child(track);
}
pub(super) fn render(
    commands: &mut Commands,
    skins: &Skins,
    widgets: &mut Widgets,
    key: Key,
    panel: &Panel,
) {
    if let Some(content) = widgets.content.take() {
        commands.entity(content).despawn();
    }
    let content = commands
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            row_gap: px(5.),
            ..default()
        })
        .id();
    commands.entity(widgets.panel).add_child(content);
    widgets.content = Some(content);
    text(
        commands,
        skins,
        content,
        panel.widget.header.replace("\r\n", "\n"),
    );
    let layout = &panel.layout.preferences;
    if layout.progress
        && let Some(value) = panel.widget.level_progress
    {
        bar(commands, content, value);
    }
    for row in &panel.widget.rows {
        text(commands, skins, content, row.text(layout));
        if layout.progress
            && let Some(value) = row.progress
        {
            bar(commands, content, value);
        }
    }
    text(
        commands,
        skins,
        content,
        "Skill / visibility / order / level values",
    );
    for (family, label) in oteryn_client_ui::skill_progress::SKILL_LABELS
        .iter()
        .enumerate()
    {
        button(
            commands,
            skins,
            content,
            key,
            Kind::Family(family as u8),
            &format!(
                "{} {label}",
                if usize::from(panel.layout.active) == family {
                    ">"
                } else {
                    ""
                }
            ),
        );
    }
    button(
        commands,
        skins,
        content,
        key,
        Kind::Visible(!layout.order.contains(&panel.layout.active)),
        if layout.order.contains(&panel.layout.active) {
            "Hide skill"
        } else {
            "Show skill"
        },
    );
    button(commands, skins, content, key, Kind::Up, "Move up");
    button(commands, skins, content, key, Kind::Down, "Move down");
    for (numbers, label) in [
        (SkillsNumbers::All, "All levels"),
        (SkillsNumbers::Trained, "Trained levels"),
        (SkillsNumbers::Loyalty, "Loyalty levels"),
        (SkillsNumbers::Current, "Current levels"),
    ] {
        button(
            commands,
            skins,
            content,
            key,
            Kind::Numbers(numbers),
            &format!(
                "{} {label}",
                if layout.numbers == numbers { ">" } else { "" }
            ),
        );
    }
    button(
        commands,
        skins,
        content,
        key,
        Kind::Progress(!layout.progress),
        if layout.progress {
            "Hide progress"
        } else {
            "Show progress"
        },
    );
    text(
        commands,
        skins,
        content,
        "Layout follows character settings. Current levels remain unavailable when the server cannot qualify them.",
    );
    button(commands, skins, content, key, Kind::Refresh, "Refresh");
    button(commands, skins, content, key, Kind::Close, "Close");
}
