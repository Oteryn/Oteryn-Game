use super::*;
use bevy::{
    input_focus::tab_navigation::TabGroup, ui::InteractionDisabled, ui_widgets::ScrollArea,
};
use oteryn_client_ui::item_forge::{self as forge, resource_dialog::Content};
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
                width: px(760.),
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
    source::dialog_caption(commands, panel, "Exaltation Forge", skins.font.clone());
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
    panel: &Panel,
    reviewing: bool,
) {
    if let Some(content) = widgets.content.take() {
        commands.entity(content).despawn();
    }
    let content = commands
        .spawn((
            ScrollArea,
            Node {
                width: percent(100.),
                max_height: px(650.),
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
    let editable = panel.editing() && panel.preview.pending.is_none() && !reviewing;
    let options = row(commands, content);
    for (op, label) in [
        (Operation::Fusion, "Fusion"),
        (Operation::ConvergenceFusion, "Convergence fusion"),
        (Operation::Transfer, "Transfer"),
        (Operation::ConvergenceTransfer, "Convergence transfer"),
    ] {
        button(
            commands,
            skins,
            options,
            key,
            (Kind::Owner(ForgeControl::Operation(op)), label, editable),
        );
    }
    if !panel.preview.selected {
        text(
            commands,
            skins,
            content,
            "Server Forge previews are unavailable in this session.",
        );
    }
    let browse = row(commands, content);
    for (action, label, enabled) in [
        (
            ForgeControl::Refresh,
            "Refresh items",
            editable && panel.preview.selected,
        ),
        (
            ForgeControl::Next,
            "Next items",
            editable
                && panel
                    .preview
                    .page
                    .as_ref()
                    .is_some_and(|p| p.next.is_some()),
        ),
    ] {
        button(
            commands,
            skins,
            browse,
            key,
            (Kind::Owner(action), label, enabled),
        );
    }
    if let Some(page) = &panel.preview.page {
        let r = page.resources;
        text(
            commands,
            skins,
            content,
            format!(
                "Available: {} carried gold; {} bank gold{}; {} / {} dust; {} slivers; {} cores",
                r.carried_gold,
                r.bank_gold,
                if r.bank_spendable {
                    ""
                } else {
                    " (not spendable)"
                },
                r.dust,
                r.dust_limit,
                r.slivers,
                r.cores
            ),
        );
    }
    for item in panel.preview.items.iter().filter(|item| !item.container) {
        let line = row(commands, content);
        text(
            commands,
            skins,
            line,
            format!(
                "{} — class {}, tier {} / {}",
                item.name, item.classification, item.tier, item.maximum_tier
            ),
        );
        button(
            commands,
            skins,
            line,
            key,
            (
                Kind::Owner(ForgeControl::First(item.handle.get())),
                "Receiver",
                editable,
            ),
        );
        button(
            commands,
            skins,
            line,
            key,
            (
                Kind::Owner(ForgeControl::Second(item.handle.get())),
                "Donor",
                editable,
            ),
        );
    }
    let cores = row(commands, content);
    button(
        commands,
        skins,
        cores,
        key,
        (
            Kind::Owner(ForgeControl::Cores {
                boost: !panel.boost,
                protect: panel.protect,
            }),
            if panel.boost {
                "Boost: on"
            } else {
                "Boost: off"
            },
            editable && panel.operation == Operation::Fusion,
        ),
    );
    button(
        commands,
        skins,
        cores,
        key,
        (
            Kind::Owner(ForgeControl::Cores {
                boost: panel.boost,
                protect: !panel.protect,
            }),
            if panel.protect {
                "Protect: on"
            } else {
                "Protect: off"
            },
            editable && panel.operation == Operation::Fusion,
        ),
    );
    text(commands, skins, content, forge::selected_pair_text(panel));
    let pair = row(commands, content);
    button(
        commands,
        skins,
        pair,
        key,
        (
            Kind::Owner(ForgeControl::Quote),
            "Request server quote",
            editable && panel.preview.selected && panel.selection().is_some(),
        ),
    );
    if reviewing {
        text(
            commands,
            skins,
            content,
            "Confirm this item pair and the displayed server quote?",
        );
        button(
            commands,
            skins,
            pair,
            key,
            (Kind::Confirm, "Confirm operation", true),
        );
        button(commands, skins, pair, key, (Kind::Cancel, "Cancel", true));
    } else {
        button(
            commands,
            skins,
            pair,
            key,
            (
                Kind::Review,
                "Review operation",
                editable && panel.verified_quote(),
            ),
        );
    }
    let conversions = row(commands, content);
    for operation in [
        Operation::DustToSlivers,
        Operation::SliversToCores,
        Operation::DustLimit,
    ] {
        button(
            commands,
            skins,
            conversions,
            key,
            (
                Kind::Owner(ForgeControl::Resource(operation)),
                forge::resource_dialog::operation_name(operation),
                editable && panel.preview.selected,
            ),
        );
    }
    if let Some(dialog) = &panel.resource_dialog.content {
        match dialog {
            Content::Confirm { .. }
            | Content::Message(_)
            | Content::Effects(_)
            | Content::Exaltation { .. } => {
                text(
                    commands,
                    skins,
                    content,
                    forge::resource_dialog::dialog_text(dialog, 0),
                );
                let controls = row(commands, content);
                if matches!(dialog, Content::Confirm { quote: Some(_), .. }) {
                    button(
                        commands,
                        skins,
                        controls,
                        key,
                        (
                            Kind::Owner(ForgeControl::ConfirmResource(
                                panel.resource_dialog.serial,
                            )),
                            "Confirm conversion",
                            !panel.pending,
                        ),
                    );
                }
                button(
                    commands,
                    skins,
                    controls,
                    key,
                    (
                        Kind::Owner(ForgeControl::DismissResource(panel.resource_dialog.serial)),
                        "Dismiss",
                        !panel.pending,
                    ),
                );
            }
            Content::Result(_) => text(commands, skins, content, "Server result received."),
        }
    }
    text(commands, skins, content, forge::panel_text(panel));
    let footer = row(commands, content);
    button(
        commands,
        skins,
        footer,
        key,
        (
            Kind::Owner(ForgeControl::History),
            "Forge history",
            panel.selected
                && !panel.pending
                && panel.preview.pending.is_none()
                && !panel.resource_dialog.confirming()
                && !reviewing,
        ),
    );
    button(
        commands,
        skins,
        footer,
        key,
        (Kind::Owner(ForgeControl::Close), "Close", true),
    );
}
fn row(commands: &mut Commands, parent: Entity) -> Entity {
    let entity = commands
        .spawn(Node {
            flex_wrap: FlexWrap::Wrap,
            column_gap: px(6.),
            row_gap: px(4.),
            flex_shrink: 0.,
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
fn button(
    commands: &mut Commands,
    skins: &Skins,
    parent: Entity,
    key: Key,
    (kind, label, enabled): (Kind, &str, bool),
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
