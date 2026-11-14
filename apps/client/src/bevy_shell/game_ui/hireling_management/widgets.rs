use super::*;
use bevy::{
    input_focus::tab_navigation::{TabGroup, TabIndex},
    text::TextCursorStyle,
    ui::InteractionDisabled,
    ui_widgets::{ScrollArea, TextInput},
};
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
                width: px(700.),
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
    source::dialog_caption(commands, panel, "Hireling management", skins.font.clone());
    Widgets {
        overlay,
        panel,
        content: None,
    }
}
pub(super) fn render(
    commands: &mut Commands,
    skins: &Skins,
    widgets: &mut Widgets,
    key: Key,
    snapshot: &HirelingSnapshot,
    state: &State,
) {
    if let Some(content) = widgets.content.take() {
        commands.entity(content).despawn();
    }
    let content = commands
        .spawn((
            ScrollArea,
            Node {
                width: percent(100.),
                max_height: px(580.),
                min_height: px(0.),
                overflow: Overflow::scroll_y(),
                flex_direction: FlexDirection::Column,
                row_gap: px(6.),
                ..default()
            },
        ))
        .id();
    commands.entity(widgets.panel).add_child(content);
    widgets.content = Some(content);
    text(commands, skins, content, ui::panel_text(&snapshot.panel));
    text(
        commands,
        skins,
        content,
        oteryn_client_ui::store::panel_text(&snapshot.store),
    );
    if !state.feedback.is_empty() {
        text(commands, skins, content, &state.feedback);
    }
    if let Some(confirmation) = &state.confirmation {
        let message = match confirmation {
            Confirmation::Return(name) => format!("Return {name} to lamp?"),
            Confirmation::Hireling(review) => review.text(),
        };
        text(commands, skins, content, message);
        let row = row(commands, content);
        button(commands, skins, row, key, Kind::Confirm, "Confirm", true);
        button(commands, skins, row, key, Kind::Cancel, "Cancel", true);
    } else {
        let enabled =
            snapshot.panel.selected && snapshot.panel.pending.is_none() && !snapshot.store.pending;
        let identities = row(commands, content);
        for (index, entry) in snapshot.panel.entries.iter().enumerate() {
            button(
                commands,
                skins,
                identities,
                key,
                Kind::Select(index),
                &entry.name,
                enabled,
            );
        }
        if let Some(entry) = snapshot
            .panel
            .entries
            .iter()
            .find(|e| Some(e.hireling) == snapshot.panel.identity)
        {
            text(
                commands,
                skins,
                content,
                format!("Selected hireling: {}", entry.name),
            );
            text(
                commands,
                skins,
                content,
                "Colors: head / body / legs / feet (0–255)",
            );
            let colors = row(commands, content);
            for (index, value) in state.colors.iter().enumerate() {
                field(commands, skins, colors, Field::Color(index), value, 3, 70.);
            }
            let outfits = row(commands, content);
            for (label, look) in ui::outfits(entry) {
                button(
                    commands,
                    skins,
                    outfits,
                    key,
                    Kind::Save(look),
                    &format!("Save {label} outfit"),
                    enabled,
                );
            }
            text(
                commands,
                skins,
                content,
                "New hireling name (3–18 letters/spaces)",
            );
            field(commands, skins, content, Field::Name, &state.name, 18, 250.);
            let configuration = row(commands, content);
            for (action, label) in [
                (131, "Rename"),
                (132, "Change sex"),
                (133, "Unlock Banker"),
                (134, "Unlock Cook"),
                (135, "Unlock Steward"),
                (136, "Unlock Trader"),
            ] {
                button(
                    commands,
                    skins,
                    configuration,
                    key,
                    Kind::Configure(action),
                    label,
                    enabled,
                );
            }
        }
        let row = row(commands, content);
        button(
            commands,
            skins,
            row,
            key,
            Kind::Prices,
            "Load prices",
            enabled,
        );
        button(commands, skins, row, key, Kind::Refresh, "Refresh", enabled);
        button(
            commands,
            skins,
            row,
            key,
            Kind::Return,
            "Return to lamp",
            enabled && snapshot.panel.identity.is_some(),
        );
    }
    button(commands, skins, content, key, Kind::Close, "Close", true);
}
fn row(commands: &mut Commands, parent: Entity) -> Entity {
    let entity = commands
        .spawn(Node {
            column_gap: px(4.),
            row_gap: px(4.),
            flex_wrap: FlexWrap::Wrap,
            flex_shrink: 0.,
            ..default()
        })
        .id();
    commands.entity(parent).add_child(entity);
    entity
}
fn text(commands: &mut Commands, skins: &Skins, parent: Entity, value: impl Into<String>) {
    let entity = commands
        .spawn((
            source::text(value, source::font(skins.font.clone(), false)),
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
    kind: Field,
    value: &str,
    max: usize,
    width: f32,
) {
    let mut editable = EditableText::new(value);
    editable.max_characters = Some(max);
    editable.visible_lines = Some(1.);
    let entity = commands
        .spawn((
            kind,
            TextInput,
            editable,
            TextCursorStyle::default(),
            TabIndex(0),
            source::font(skins.font.clone(), false),
            TextColor(source::TEXT_COLOR),
            TextLayout::default(),
            source::frame(skins.inset.clone(), [1.; 4]),
            Node {
                width: px(width),
                min_height: px(26.),
                padding: UiRect::all(px(4.)),
                flex_shrink: 0.,
                ..default()
            },
        ))
        .id();
    commands.entity(parent).add_child(entity);
}
// Each button carries the exact projected session and revision.
#[allow(clippy::too_many_arguments)]
fn button(
    commands: &mut Commands,
    skins: &Skins,
    parent: Entity,
    key: Key,
    kind: Kind,
    label: &str,
    enabled: bool,
) {
    let mut entity = commands.spawn((
        source::button(skins.buttons.clone(), 150.),
        Control { key, kind },
    ));
    if !enabled {
        entity.insert(InteractionDisabled);
    }
    let entity = entity.observe(activate).id();
    let caption = commands
        .spawn(source::button_caption(label, skins.font.clone()))
        .id();
    commands.entity(entity).add_child(caption);
    commands.entity(parent).add_child(entity);
}
