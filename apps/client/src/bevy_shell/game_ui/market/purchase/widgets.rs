use super::*;
use bevy::{
    input_focus::tab_navigation::{TabGroup, TabIndex},
    text::TextCursorStyle,
    ui_widgets::TextInput,
};
pub(super) fn build(
    commands: &mut Commands,
    skin: &Skins,
    quote: &Quote,
    state: &PurchaseState,
) -> Entity {
    let overlay = commands
        .spawn((
            GameModal,
            GlobalZIndex(158),
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
            source::frame(skin.frame.clone(), [4., 17., 4., 4.]),
            TabGroup {
                order: 4,
                modal: true,
            },
            Node {
                width: px(480.),
                max_width: percent(94.),
                padding: UiRect {
                    top: px(24.),
                    left: px(10.),
                    right: px(10.),
                    bottom: px(10.),
                },
                flex_direction: FlexDirection::Column,
                row_gap: px(8.),
                ..default()
            },
        ))
        .id();
    commands.entity(overlay).add_child(panel);
    source::dialog_caption(commands, panel, "Buy item", skin.font.clone());
    if let Some((amount, total)) = state.confirmation {
        text(
            commands,
            skin,
            panel,
            &format!(
                "Buy {amount} × {} at {} gold each (total {total} gold)?",
                quote.ware.name, quote.offer.price
            ),
            "market.purchase.confirmation",
        );
        button(
            commands,
            skin,
            panel,
            state.revision,
            ControlKind::Confirm,
            "Confirm purchase",
            "market.purchase.confirm",
        );
    } else {
        text(
            commands,
            skin,
            panel,
            &format!(
                "{} — {} gold each. Available: {}.",
                quote.ware.name, quote.offer.price, quote.offer.amount
            ),
            "market.purchase.item",
        );
        let mut editable = EditableText::new(&state.quantity);
        editable.max_characters = Some(5);
        let field = commands
            .spawn((
                Quantity,
                Name::new("market.purchase.quantity"),
                TextInput,
                editable,
                source::font(skin.font.clone(), false),
                TextColor(source::TEXT_COLOR),
                TextLayout::no_wrap(),
                TextCursorStyle::default(),
                TabIndex(0),
                source::frame(skin.inset.clone(), [1.; 4]),
                Node {
                    width: px(120.),
                    height: px(25.),
                    padding: px(4.).all(),
                    ..default()
                },
            ))
            .id();
        commands.entity(panel).add_child(field);
        if !state.feedback.is_empty() {
            text(
                commands,
                skin,
                panel,
                &state.feedback,
                "market.purchase.feedback",
            );
        }
        button(
            commands,
            skin,
            panel,
            state.revision,
            ControlKind::Review,
            "Review purchase",
            "market.purchase.review",
        );
    }
    button(
        commands,
        skin,
        panel,
        state.revision,
        ControlKind::Cancel,
        "Cancel",
        "market.purchase.cancel",
    );
    overlay
}
fn text(commands: &mut Commands, skin: &Skins, parent: Entity, value: &str, name: &'static str) {
    let entity = commands
        .spawn((
            source::text(value, source::font(skin.font.clone(), false)),
            Name::new(name),
        ))
        .insert((
            TextLayout::default(),
            Node {
                width: percent(100.),
                ..default()
            },
        ))
        .id();
    commands.entity(parent).add_child(entity);
}
fn button(
    commands: &mut Commands,
    skin: &Skins,
    parent: Entity,
    revision: u64,
    kind: ControlKind,
    value: &str,
    name: &'static str,
) {
    let entity = commands
        .spawn((
            source::button(skin.buttons.clone(), 150.),
            Control { revision, kind },
            Name::new(name),
        ))
        .observe(activate)
        .id();
    let caption = commands
        .spawn(source::button_caption(value, skin.font.clone()))
        .id();
    commands.entity(entity).add_child(caption);
    commands.entity(parent).add_child(entity);
}
