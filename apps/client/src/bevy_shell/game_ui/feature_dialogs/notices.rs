//! Notifications retain the core expiry/preferences; death follows authoritative vitals.
use super::*;
use crate::bevy_shell::window_drag::DraggableDialog;
use bevy::{input_focus::tab_navigation::TabGroup, ui_widgets::ScrollArea};
use oteryn_client_ui_bevy as source;

#[derive(Component)]
pub(super) struct DeathOverlay;
#[derive(Component)]
pub(super) struct DeathText;
#[derive(Component)]
pub(super) struct Banner;
#[derive(Component)]
pub(super) struct NoticeOverlay;
#[derive(Component)]
pub(super) struct NoticeText;
#[derive(Component)]
pub(super) struct CloseNotice(u64);

pub(super) fn setup(mut commands: Commands, skins: Res<Skins>) {
    let death = overlay(&mut commands, 175, DeathOverlay);
    let panel = notice_panel(&mut commands, &skins, death, "You are dead");
    let text = commands
        .spawn((
            source::text("", source::font(skins.font.clone(), false)),
            DeathText,
        ))
        .insert(TextLayout::default())
        .id();
    commands.entity(panel).add_child(text);
    commands
        .spawn((
            Banner,
            source::text("", source::font(skins.font.clone(), false)),
            Node {
                position_type: PositionType::Absolute,
                top: px(72.),
                left: percent(20.),
                max_width: percent(55.),
                ..default()
            },
            GlobalZIndex(30),
        ))
        .insert(TextLayout::default());
    let notices = overlay(&mut commands, 170, NoticeOverlay);
    let panel = notice_panel(&mut commands, &skins, notices, "Compendium — Notices");
    let scroll = commands
        .spawn((
            ScrollArea,
            Node {
                flex_direction: FlexDirection::Column,
                overflow: Overflow::scroll_y(),
                min_height: px(0.),
                max_height: px(380.),
                width: percent(100.),
                ..default()
            },
        ))
        .id();
    let text = commands
        .spawn((
            source::text("", source::font(skins.font.clone(), false)),
            NoticeText,
        ))
        .insert(TextLayout::default())
        .id();
    commands.entity(scroll).add_child(text);
    commands.entity(panel).add_child(scroll);
    close_button(&mut commands, &skins, panel, 0);
}
fn overlay<C: Component>(commands: &mut Commands, z: i32, marker: C) -> Entity {
    commands
        .spawn((
            marker,
            GameModal,
            GlobalZIndex(z),
            Node {
                position_type: PositionType::Absolute,
                width: percent(100.),
                height: percent(100.),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                display: Display::None,
                ..default()
            },
            BackgroundColor(Color::srgba(0., 0., 0., 0.4)),
        ))
        .id()
}
fn notice_panel(commands: &mut Commands, skins: &Skins, overlay: Entity, title: &str) -> Entity {
    let panel = commands
        .spawn((
            DraggableDialog,
            TabGroup {
                order: 3,
                modal: true,
            },
            source::frame(skins.frame.clone(), [4., 17., 4., 4.]),
            Node {
                width: px(500.),
                max_width: percent(95.),
                max_height: percent(94.),
                padding: UiRect {
                    top: px(26.),
                    bottom: px(12.),
                    left: px(12.),
                    right: px(12.),
                },
                flex_direction: FlexDirection::Column,
                row_gap: px(8.),
                ..default()
            },
        ))
        .id();
    source::dialog_caption(commands, panel, title, skins.font.clone());
    commands.entity(overlay).add_child(panel);
    panel
}
fn close_button(commands: &mut Commands, skins: &Skins, parent: Entity, generation: u64) -> Entity {
    let button = commands
        .spawn((
            source::button(skins.buttons.clone(), 80.),
            CloseNotice(generation),
        ))
        .observe(close)
        .id();
    let caption = commands
        .spawn(source::button_caption("Close", skins.font.clone()))
        .id();
    commands.entity(button).add_child(caption);
    commands.entity(parent).add_child(button);
    button
}
fn close(
    event: On<Activate>,
    buttons: Query<&CloseNotice>,
    owner: NonSend<HostOwner>,
    guard: ModalGuard,
) {
    if !owner
        .play
        .as_ref()
        .is_some_and(|play| play.link.server_modal_active())
        && guard.allows(event.entity)
        && let Ok(button) = buttons.get(event.entity)
        && button.0 == owner.generation()
        && let Some(play) = &owner.play
    {
        play.link.notices_set_open(false);
    }
}
pub(super) fn keyboard(
    mut keys: MessageReader<bevy::input::keyboard::KeyboardInput>,
    owner: NonSend<HostOwner>,
    guard: ModalGuard,
    overlays: Query<Entity, With<NoticeOverlay>>,
) {
    if owner
        .play
        .as_ref()
        .is_some_and(|play| play.link.server_modal_active())
        || !overlays.single().is_ok_and(|overlay| guard.allows(overlay))
    {
        keys.clear();
        return;
    }
    if let Some(play) = &owner.play {
        if keys.read().any(|event| {
            event.state == bevy::input::ButtonState::Pressed
                && !event.repeat
                && event.key_code == KeyCode::Escape
        }) {
            play.link.notices_set_open(false);
        }
    } else {
        keys.clear();
    }
}
// Bevy injects independent visibility, text and focus projections.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub(super) fn project(
    mut commands: Commands,
    skins: Res<Skins>,
    owner: NonSend<HostOwner>,
    mut death: Query<(Entity, &mut Node), (With<DeathOverlay>, Without<NoticeOverlay>)>,
    mut notice: Query<&mut Node, (With<NoticeOverlay>, Without<DeathOverlay>)>,
    mut texts: Query<(&mut Text, Has<DeathText>, Has<Banner>, Has<NoticeText>)>,
    close: Query<(Entity, &CloseNotice, &ChildOf)>,
    mut focus: ResMut<InputFocus>,
    mut previous: Local<(bool, bool)>,
) {
    let dead = owner
        .play
        .as_ref()
        .and_then(|p| p.view.death_modal_notice());
    let notices = owner
        .play
        .as_ref()
        .map(|p| p.link.notices_panel())
        .filter(|p| p.open);
    let server_modal = owner
        .play
        .as_ref()
        .is_some_and(|p| p.link.server_modal_panel().is_some());
    for (entity, mut node) in &mut death {
        if dead.is_some() && !previous.0 && !server_modal {
            focus.set(entity, FocusCause::Navigated);
        }
        node.display = if dead.is_some() {
            Display::Flex
        } else {
            Display::None
        };
    }
    for mut node in &mut notice {
        node.display = if notices.is_some() {
            Display::Flex
        } else {
            Display::None
        };
    }
    for (old_entity, button, parent) in &close {
        let entity = if button.0 != owner.generation() {
            // Replace the action entity so delayed activation cannot target a new session.
            commands.entity(old_entity).despawn();
            close_button(&mut commands, &skins, parent.parent(), owner.generation())
        } else {
            old_entity
        };
        if notices.is_some() && !previous.1 && !server_modal && dead.is_none() {
            focus.set(entity, FocusCause::Navigated);
        }
    }
    if ((previous.0 && dead.is_none()) || (previous.1 && notices.is_none())) && !server_modal {
        focus.clear();
    }
    *previous = (dead.is_some(), notices.is_some());
    for (mut text, is_death, is_banner, is_notice) in &mut texts {
        let value = if is_death {
            Some(dead.clone().unwrap_or_default())
        } else if is_banner {
            Some(
                owner
                    .play
                    .as_ref()
                    .map(|p| {
                        [
                            p.view.notification_text(false),
                            p.view.notification_text(true),
                        ]
                        .into_iter()
                        .filter(|text| !text.is_empty())
                        .collect::<Vec<_>>()
                        .join("\n")
                    })
                    .unwrap_or_default(),
            )
        } else if is_notice {
            Some(
                notices
                    .as_ref()
                    .map(oteryn_client_ui::tutorial_notices::panel_text)
                    .unwrap_or_default(),
            )
        } else {
            None
        };
        if let Some(value) = value
            && text.0 != value
        {
            text.0 = value;
        }
    }
}
