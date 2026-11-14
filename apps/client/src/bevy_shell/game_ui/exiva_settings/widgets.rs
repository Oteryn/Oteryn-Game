use super::*;
use bevy::{
    input_focus::tab_navigation::{TabGroup, TabIndex},
    text::TextCursorStyle,
    ui::{Checked, InteractionDisabled},
    ui_widgets::{ScrollArea, TextInput},
};
use oteryn_client_ui::exiva_settings::{ExivaPanel, panel_text};
use oteryn_client_ui_bevy as source;
pub(super) struct Widgets {
    pub overlay: Entity,
    pub panel: Entity,
    pub content: Option<Entity>,
}
pub(super) fn build(commands: &mut Commands, skins: &Skins) -> Widgets {
    let overlay = commands
        .spawn((
            GameModal,
            GlobalZIndex(167),
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
            Node {
                width: px(600.),
                max_width: percent(95.),
                max_height: percent(95.),
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
    commands.entity(overlay).add_child(panel);
    source::dialog_caption(commands, panel, "Exiva settings", skins.font.clone());
    Widgets {
        overlay,
        panel,
        content: None,
    }
}
// One render receives the exact owner projection and the presenter's local selection/draft.
#[allow(clippy::too_many_arguments)]
pub(super) fn render(
    commands: &mut Commands,
    skins: &Skins,
    widgets: &mut Widgets,
    key: Key,
    panel: &ExivaPanel,
    selected: Option<[u8; 16]>,
    confirmation: Option<[u8; 16]>,
    draft: &str,
) {
    if let Some(content) = widgets.content.take() {
        commands.entity(content).despawn();
    }
    let content = column(commands, widgets.panel);
    widgets.content = Some(content);
    let status = column(commands, content);
    commands.entity(status).insert((
        ScrollArea,
        Node {
            max_height: px(80.),
            overflow: Overflow::scroll_y(),
            flex_direction: FlexDirection::Column,
            ..default()
        },
    ));
    label(commands, skins, status, &panel_text(panel));
    let ready = panel.selected && !panel.pending;
    if let Some(id) = confirmation {
        if let Some((_, name)) = panel
            .list
            .as_ref()
            .and_then(|list| list.players.iter().find(|(held, _)| *held == id))
        {
            label(
                commands,
                skins,
                content,
                &format!("Remove {name} from the Exiva allow list?"),
            );
            button(
                commands,
                skins,
                content,
                key,
                Kind::ConfirmRemove,
                "Confirm removal",
                ready,
            );
            button(
                commands,
                skins,
                content,
                key,
                Kind::CancelRemove,
                "Cancel",
                true,
            );
        }
    } else {
        let body = column(commands, content);
        commands.entity(body).insert((
            ScrollArea,
            Node {
                max_height: px(350.),
                overflow: Overflow::scroll_y(),
                flex_direction: FlexDirection::Column,
                row_gap: px(5.),
                ..default()
            },
        ));
        if let Some(list) = &panel.list {
            for (bit, caption) in [
                "Allow everyone",
                "Allow player list",
                "Allow own guild",
                "Allow own party",
                "Allow VIP list",
            ]
            .iter()
            .enumerate()
            {
                let entity = button(
                    commands,
                    skins,
                    body,
                    key,
                    Kind::Toggle(bit as u8),
                    caption,
                    ready,
                );
                // Checked follows only the server modes; the click never toggles it locally.
                if list.modes & (1 << bit) != 0 {
                    commands.entity(entity).insert(Checked);
                }
                let icon = commands
                    .spawn(ImageNode::new(if list.modes & (1 << bit) != 0 {
                        skins.checked.clone()
                    } else {
                        skins.unchecked.clone()
                    }))
                    .id();
                commands.entity(entity).add_child(icon);
            }
            for (id, name) in &list.players {
                let entity = button(commands, skins, body, key, Kind::Select(*id), name, ready);
                if selected == Some(*id) {
                    commands
                        .entity(entity)
                        .insert(BackgroundColor(Color::srgb(0.3, 0.3, 0.4)));
                }
            }
        }
        label(
            commands,
            skins,
            content,
            "Character name (2–29 letters/spaces)",
        );
        let mut editable = EditableText::new(draft);
        editable.max_characters = Some(29);
        let field = commands
            .spawn((
                NameField,
                TextInput,
                editable,
                source::font(skins.font.clone(), false),
                TextColor(source::TEXT_COLOR),
                TextLayout::no_wrap(),
                TextCursorStyle::default(),
                TabIndex(0),
                source::frame(skins.inset.clone(), [1.; 4]),
                Node {
                    width: percent(100.),
                    height: px(26.),
                    padding: px(4.).all(),
                    flex_shrink: 0.,
                    ..default()
                },
            ))
            .id();
        if !ready || panel.list.is_none() {
            commands.entity(field).insert(InteractionDisabled);
        }
        commands.entity(content).add_child(field);
        button(
            commands,
            skins,
            content,
            key,
            Kind::Add,
            "Add name",
            ready && panel.list.is_some(),
        );
        button(
            commands,
            skins,
            content,
            key,
            Kind::Remove,
            "Remove selected",
            ready && selected.is_some(),
        );
        button(
            commands,
            skins,
            content,
            key,
            Kind::Refresh,
            "Refresh",
            ready,
        );
    }
    button(commands, skins, content, key, Kind::Close, "Close", true);
}
fn column(commands: &mut Commands, parent: Entity) -> Entity {
    let entity = commands
        .spawn(Node {
            width: percent(100.),
            min_height: px(0.),
            flex_direction: FlexDirection::Column,
            row_gap: px(5.),
            ..default()
        })
        .id();
    commands.entity(parent).add_child(entity);
    entity
}
fn label(commands: &mut Commands, skins: &Skins, parent: Entity, text: &str) {
    let entity = commands
        .spawn(source::text(text, source::font(skins.font.clone(), false)))
        .insert(TextLayout::default())
        .id();
    commands.entity(parent).add_child(entity);
}
// Buttons share a snapshot key and use the retained skin/caption helpers.
#[allow(clippy::too_many_arguments)]
fn button(
    commands: &mut Commands,
    skins: &Skins,
    parent: Entity,
    key: Key,
    kind: Kind,
    caption: &str,
    enabled: bool,
) -> Entity {
    let mut entity = commands.spawn((
        source::button(skins.buttons.clone(), 220.),
        Control { key, kind },
    ));
    if !enabled {
        entity.insert(InteractionDisabled);
    }
    let entity = entity.observe(activate).id();
    let caption = commands
        .spawn(source::button_caption(caption, skins.font.clone()))
        .id();
    commands.entity(entity).add_child(caption);
    commands.entity(parent).add_child(entity);
    entity
}
