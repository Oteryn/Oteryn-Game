use super::*;
use bevy::{
    input_focus::tab_navigation::{TabGroup, TabIndex},
    text::TextCursorStyle,
    ui_widgets::{ScrollArea, TextInput},
};
pub(super) fn build(
    commands: &mut Commands,
    skin: &Skins,
    state: &State,
    panel: &CoinPanel,
) -> Entity {
    let root = commands
        .spawn((
            GameModal,
            GlobalZIndex(160),
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
    let frame = commands
        .spawn((
            source::frame(skin.frame.clone(), [4., 17., 4., 4.]),
            TabGroup {
                order: 6,
                modal: true,
            },
            Node {
                width: px(850.),
                max_width: percent(96.),
                max_height: percent(94.),
                padding: px(24.).all(),
                flex_direction: FlexDirection::Column,
                row_gap: px(8.),
                ..default()
            },
        ))
        .id();
    commands.entity(root).add_child(frame);
    source::dialog_caption(commands, frame, "Coin market", skin.font.clone());
    let body = commands
        .spawn((
            ScrollArea,
            Node {
                overflow: Overflow::scroll_y(),
                flex_direction: FlexDirection::Column,
                row_gap: px(6.),
                ..default()
            },
        ))
        .id();
    commands.entity(frame).add_child(body);
    let text = commands
        .spawn((
            source::text(
                &oteryn_client_ui::market::coin_text(panel),
                source::font(skin.font.clone(), false),
            ),
            Name::new("coin_market.server"),
        ))
        .id();
    commands.entity(body).add_child(text);
    for (index, caption) in ["Coin amount", "Gold per coin", "Offer or transaction row"]
        .into_iter()
        .enumerate()
    {
        let label = commands
            .spawn(source::text(
                caption,
                source::font(skin.font.clone(), false),
            ))
            .id();
        commands.entity(body).add_child(label);
        let field = commands
            .spawn((
                Field(index),
                Name::new(format!("coin_market.field.{index}")),
                TextInput,
                EditableText::new(&panel.fields[index]),
                source::font(skin.font.clone(), false),
                TextColor(source::TEXT_COLOR),
                TextLayout::no_wrap(),
                TextCursorStyle::default(),
                TabIndex(index as i32),
                source::frame(skin.inset.clone(), [1.; 4]),
                Node {
                    width: px(200.),
                    height: px(26.),
                    padding: px(4.).all(),
                    ..default()
                },
            ))
            .id();
        commands.entity(body).add_child(field);
    }
    let buttons = commands
        .spawn(Node {
            flex_wrap: FlexWrap::Wrap,
            column_gap: px(5.),
            row_gap: px(5.),
            ..default()
        })
        .id();
    commands.entity(body).add_child(buttons);
    for (action, caption) in [
        "Book",
        "My offers",
        "Review sell",
        "Review buy",
        "Review take",
        "Review cancel",
        "Confirm reviewed terms",
        "Retry confirmation",
        "Coin history",
        "Next history page",
        "Transaction details",
        "Coin balances",
        "Refresh last operation",
        "Dismiss review",
        "Next offers",
    ]
    .into_iter()
    .enumerate()
    {
        button(commands, skin, buttons, state, Some(action), caption);
    }
    button(commands, skin, body, state, None, "Close");
    root
}
fn button(
    commands: &mut Commands,
    skin: &Skins,
    parent: Entity,
    state: &State,
    action: Option<usize>,
    caption: &str,
) {
    let entity = commands
        .spawn((
            source::button(skin.buttons.clone(), 180.),
            Control {
                generation: state.generation,
                binding: state.binding,
                revision: state.revision,
                action,
            },
            Name::new(action.map_or_else(
                || "coin_market.close".into(),
                |action| format!("coin_market.action.{action}"),
            )),
        ))
        .observe(activate)
        .id();
    let text = commands
        .spawn(source::button_caption(caption, skin.font.clone()))
        .id();
    commands.entity(entity).add_child(text);
    commands.entity(parent).add_child(entity);
}
