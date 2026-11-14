use super::*;
use bevy::{
    input_focus::tab_navigation::TabGroup, ui::InteractionDisabled, ui_widgets::ScrollArea,
};
use oteryn_client_ui_bevy as source;
pub(super) fn build(commands: &mut Commands, skins: &Skins, snapshot: &Snapshot) -> Entity {
    let overlay = commands
        .spawn((
            GameModal,
            GlobalZIndex(166),
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
                width: px(650.),
                max_width: percent(95.),
                max_height: percent(90.),
                padding: UiRect {
                    left: px(12.),
                    right: px(12.),
                    top: px(25.),
                    bottom: px(10.),
                },
                flex_direction: FlexDirection::Column,
                row_gap: px(8.),
                ..default()
            },
        ))
        .id();
    commands.entity(overlay).add_child(panel);
    let title = match snapshot.window {
        Window::Daily => "Daily rewards",
        Window::Returner => "Welcome-back reward",
        Window::Boost => "Daily boost",
    };
    source::dialog_caption(commands, panel, title, skins.font.clone());
    let body = commands
        .spawn((
            ScrollArea,
            Node {
                width: percent(100.),
                max_height: px(480.),
                min_height: px(0.),
                overflow: Overflow::scroll_y(),
                flex_direction: FlexDirection::Column,
                ..default()
            },
        ))
        .id();
    commands.entity(panel).add_child(body);
    let text = match snapshot.window {
        Window::Daily => oteryn_client_ui::daily_reward::panel_text(&snapshot.daily.panel),
        Window::Returner => snapshot.daily.panel.returner.notification(),
        Window::Boost => format!(
            "{}\n{}",
            oteryn_client_ui::daily_boost::panel_text(&snapshot.boost.panel),
            snapshot.boost.xp
        ),
    };
    let text = commands
        .spawn((
            source::text(text, source::font(skins.font.clone(), false)),
            Node {
                max_width: percent(100.),
                flex_shrink: 0.,
                ..default()
            },
        ))
        .id();
    commands.entity(body).add_child(text);
    let footer = commands
        .spawn(Node {
            flex_wrap: FlexWrap::Wrap,
            column_gap: px(6.),
            row_gap: px(4.),
            ..default()
        })
        .id();
    commands.entity(panel).add_child(footer);
    let daily = &snapshot.daily.panel;
    let ready = daily.selected && !daily.pending;
    let buttons: Vec<(Kind, &str, bool)> = match snapshot.window {
        Window::Daily => vec![
            (Kind::Refresh, "Refresh", ready),
            (
                Kind::Claim,
                "Claim",
                ready
                    && daily
                        .wall
                        .is_some_and(|wall| wall.claim_enabled && !wall.claimed),
            ),
            (Kind::Details, "History", ready),
            (Kind::Returner, "Welcome back", true),
        ],
        Window::Returner => vec![
            (
                Kind::ReturnerRefresh,
                "Refresh",
                daily.selected && daily.returner_pending.is_none(),
            ),
            (
                Kind::ReturnerClaim,
                "Claim",
                daily.selected
                    && daily.returner_pending.is_none()
                    && !daily.returner.pending
                    && daily.returner.offer.is_some()
                    && !daily.returner.is_claimed(),
            ),
        ],
        Window::Boost => Vec::new(),
    };
    for (kind, label, enabled) in buttons.into_iter().chain([(Kind::Close, "Close", true)]) {
        let mut button = commands.spawn((
            source::button(skins.buttons.clone(), 108.),
            Control {
                key: snapshot.key,
                kind,
            },
        ));
        if !enabled {
            button.insert(InteractionDisabled);
        }
        let button = button.observe(activate).id();
        let caption = commands
            .spawn(source::button_caption(label, skins.font.clone()))
            .id();
        commands.entity(button).add_child(caption);
        commands.entity(footer).add_child(button);
    }
    overlay
}
