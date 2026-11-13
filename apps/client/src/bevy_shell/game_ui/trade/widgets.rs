use super::*;
use bevy::{
    input_focus::tab_navigation::TabGroup,
    ui::InteractionDisabled,
    ui_widgets::{Button, ScrollArea},
};
use oteryn_client_ui::player_trade::{picker, player_trade::Offer};

pub(super) fn build(
    commands: &mut Commands,
    skins: &Skins,
    dock: Entity,
    generation: u64,
    revision: u64,
    panel: &PlayerTradePanel,
) -> Entity {
    let root = commands
        .spawn((
            Name::new("trade.sidebar"),
            TabGroup::new(3),
            source::frame(skins.frame.clone(), [2., 17., 2., 2.]),
            Node {
                width: px(176.),
                flex_shrink: 0.,
                padding: UiRect {
                    top: px(20.),
                    bottom: px(4.),
                    left: px(4.),
                    right: px(4.),
                },
                flex_direction: FlexDirection::Column,
                row_gap: px(3.),
                ..default()
            },
        ))
        .id();
    commands.entity(dock).add_child(root);
    source::dialog_caption(commands, root, "Trade", skins.font.clone());
    let labels = row(commands, root);
    for label in ["Your offer", "Their offer"] {
        let text = label_text(commands, skins, labels, label);
        commands.entity(text).insert(Node {
            width: px(80.),
            ..default()
        });
    }
    let grids = row(commands, root);
    let state = panel.state.as_ref();
    for (counter, offer) in [
        (false, state.and_then(|s| s.own)),
        (true, state.and_then(|s| s.counter)),
    ] {
        if counter {
            let separator = commands
                .spawn((
                    Node {
                        width: px(2.),
                        height: px(37.),
                        margin: UiRect::horizontal(px(2.)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.1, 0.1, 0.1)),
                ))
                .id();
            commands.entity(grids).add_child(separator);
        }
        let side = row(commands, grids);
        commands.entity(side).insert(Node {
            column_gap: px(0.),
            ..default()
        });
        for index in 0..2 {
            slot(
                commands,
                skins,
                side,
                Action {
                    generation,
                    revision,
                    kind: Kind::Look(counter),
                },
                (index == 0).then_some(offer).flatten(),
                panel,
            );
        }
    }
    // Server Look text is bounded by the protocol, but may still be longer than
    // the sidebar. Keep the canonical action row reachable below its scroll area.
    let details = commands
        .spawn((
            ScrollArea,
            Node {
                width: percent(100.),
                max_height: px(80.),
                overflow: Overflow::scroll_y(),
                flex_direction: FlexDirection::Column,
                flex_shrink: 0.,
                ..default()
            },
        ))
        .id();
    commands.entity(root).add_child(details);
    let status = status(panel);
    label_text(commands, skins, details, &status);
    let footer = row(commands, root);
    let action = |kind| Action {
        generation,
        revision,
        kind,
    };
    if trade_phase(panel) == TradePhase::Ready {
        button(
            commands,
            skins,
            footer,
            action(Kind::Accept),
            "Accept",
            76.,
            panel,
        );
    }
    if trade_phase(panel).active() {
        let caption = if state.is_some_and(|s| s.counter.is_some()) {
            "Reject"
        } else {
            "Cancel"
        };
        button(
            commands,
            skins,
            footer,
            action(Kind::Cancel),
            caption,
            76.,
            panel,
        );
    } else {
        button(
            commands,
            skins,
            footer,
            action(Kind::Close),
            "Close",
            76.,
            panel,
        );
    }
    if trade_picker_available(panel) {
        label_text(commands, skins, root, "Choose an inventory item:");
        let list = commands
            .spawn((
                ScrollArea,
                Node {
                    width: percent(100.),
                    max_height: px(120.),
                    overflow: Overflow::scroll_y(),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(2.),
                    ..default()
                },
            ))
            .id();
        commands.entity(root).add_child(list);
        for (source, item) in picker::rows(&panel.items) {
            let row = row(commands, list);
            let selected = panel
                .offer
                .is_some_and(|(_, handle)| handle == item.handle.get());
            let caption = format!(
                "{}{} x{}",
                if selected { "✓ " } else { "" },
                source,
                item.count
            );
            button(
                commands,
                skins,
                row,
                action(Kind::Select(item.handle)),
                &caption,
                110.,
                panel,
            );
            if panel.items.can_open {
                button(
                    commands,
                    skins,
                    row,
                    action(Kind::Open(item.handle)),
                    "Open",
                    42.,
                    panel,
                );
            }
        }
        button(
            commands,
            skins,
            root,
            action(Kind::Offer),
            "Offer selected",
            155.,
            panel,
        );
    }
    root
}
fn row(commands: &mut Commands, parent: Entity) -> Entity {
    let row = commands
        .spawn(Node {
            column_gap: px(2.),
            flex_shrink: 0.,
            ..default()
        })
        .id();
    commands.entity(parent).add_child(row);
    row
}
fn label_text(commands: &mut Commands, skins: &Skins, parent: Entity, value: &str) -> Entity {
    let label = commands
        .spawn((
            source::text(value, source::font(skins.font.clone(), true)),
            Node {
                max_width: percent(100.),
                flex_shrink: 0.,
                ..default()
            },
        ))
        .id();
    commands.entity(label).insert(TextLayout::default());
    commands.entity(parent).add_child(label);
    label
}
fn button(
    commands: &mut Commands,
    skins: &Skins,
    parent: Entity,
    action: Action,
    caption: &str,
    width: f32,
    panel: &PlayerTradePanel,
) {
    let mut button = commands.spawn((source::button(skins.buttons.clone(), width), action));
    if !trade_action_enabled(panel, action.kind.action()) {
        button.insert(InteractionDisabled);
    }
    let button = button.observe(activate).id();
    let label = commands
        .spawn(source::button_caption(caption, skins.font.clone()))
        .id();
    commands.entity(button).add_child(label);
    commands.entity(parent).add_child(button);
}
fn slot(
    commands: &mut Commands,
    skins: &Skins,
    parent: Entity,
    action: Action,
    offer: Option<Offer>,
    panel: &PlayerTradePanel,
) {
    let mut slot = commands.spawn((
        OfferSlot,
        source::frame(skins.inset.clone(), [1.; 4]),
        Node {
            width: px(37.),
            height: px(37.),
            flex_shrink: 0.,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
    ));
    if offer.is_some() {
        slot.insert((Button, action)).observe(activate);
        if !trade_action_enabled(panel, action.kind.action()) {
            slot.insert(InteractionDisabled);
        }
    }
    let slot = slot.id();
    commands.entity(parent).add_child(slot);
    if let Some(offer) = offer {
        // The trade projection has no custody handle or appearance ID. Never synthesize an ItemEntry.
        label_text(commands, skins, slot, "?");
        if offer.count > 1 {
            let count = label_text(commands, skins, slot, &offer.count.to_string());
            commands.entity(count).insert(Node {
                position_type: PositionType::Absolute,
                bottom: px(2.),
                right: px(2.),
                ..default()
            });
        }
    }
}
fn status(panel: &PlayerTradePanel) -> String {
    let mut lines = Vec::new();
    if panel.pending || panel.items.pending {
        lines.push("Please wait.".to_owned());
    }
    if let Some(state) = &panel.state {
        lines.push(
            match trade_phase(panel) {
                TradePhase::Offered => "Waiting for an offer.",
                TradePhase::Ready if state.accepted[0] => "Waiting for partner.",
                TradePhase::Ready if state.accepted[1] => "Your partner has accepted.",
                TradePhase::Ready => "Review both offers before accepting.",
                TradePhase::Transferring => "Transferring trade.",
                TradePhase::Inactive => "Trade closed.",
                TradePhase::Unknown => "Trade unavailable.",
            }
            .to_owned(),
        );
        for text in [&state.message, &state.look] {
            if !text.is_empty() {
                lines.push(text.clone());
            }
        }
    } else {
        lines.push("Choose Trade with from a player's world context menu.".to_owned());
    }
    if let Some(feedback) = panel.feedback {
        lines.push(feedback.to_owned());
    }
    lines.join("\n")
}
