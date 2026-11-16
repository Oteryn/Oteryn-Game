//! Per-frame projection of the active owner dialog into the Bevy overlay.
use super::*;

pub(super) fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skin: Res<Skin>,
    mut overlays: Query<(Entity, &mut Node), With<Overlay>>,
    mut previous: Local<Option<u64>>,
    mut panel: Local<Option<Entity>>,
    mut focus: ResMut<InputFocus>,
) {
    let pending = owner.dialogs.pending.as_ref();
    let id = pending.map(|request| request.id);
    let Ok((overlay, mut node)) = overlays.single_mut() else {
        return;
    };
    node.display = if pending.is_some() {
        Display::Flex
    } else {
        Display::None
    };
    if id == *previous {
        return;
    }
    *previous = id;
    if let Some(entity) = panel.take() {
        commands.entity(entity).despawn();
    }
    let Some(request) = pending else {
        return;
    };
    let window = commands
        .spawn((
            bevy::ui_widgets::ScrollArea,
            source::frame(skin.frame.clone(), [4., 17., 4., 4.]),
            TabGroup {
                order: 3,
                modal: true,
            },
            Node {
                width: px(300.),
                max_width: percent(95.),
                max_height: percent(95.),
                padding: UiRect::axes(px(16.), px(29.)),
                flex_direction: FlexDirection::Column,
                row_gap: px(5.),
                overflow: Overflow::scroll_y(),
                ..default()
            },
        ))
        .id();
    commands.entity(overlay).add_child(window);
    *panel = Some(window);
    let title = match request.content {
        Content::Logout => tr!("Logout"),
        Content::Context { .. } => tr!("Actions"),
    };
    source::dialog_caption(&mut commands, window, title, skin.font.clone());
    match &request.content {
        Content::Logout => {
            let text = commands
                .spawn(source::text(
                    "Log out of the game?",
                    source::font(skin.font.clone(), false),
                ))
                .id();
            commands.entity(window).add_child(text);
            button(
                &mut commands,
                &skin,
                window,
                request.id,
                tr!("Ok"),
                Choice::Logout,
            );
        }
        Content::Context {
            menu,
            party,
            inspection,
            report,
            trade,
            browse,
            npc,
        } => {
            if npc.is_some() {
                for (caption, choice) in [
                    (tr!("Talk"), Choice::Talk),
                    (tr!("Trade"), Choice::NpcTrade),
                ] {
                    button(&mut commands, &skin, window, request.id, caption, choice);
                }
            }
            if browse.is_some() {
                button(
                    &mut commands,
                    &skin,
                    window,
                    request.id,
                    tr!("Browse Field"),
                    Choice::Browse,
                );
            }
            if trade.is_some() {
                button(
                    &mut commands,
                    &skin,
                    window,
                    request.id,
                    "Trade with",
                    Choice::Trade,
                );
            }
            if report.is_some() {
                button(
                    &mut commands,
                    &skin,
                    window,
                    request.id,
                    "Report",
                    Choice::Report,
                );
            }
            for (index, (label, _)) in menu.iter().flat_map(|menu| menu.entries.iter()).enumerate()
            {
                button(
                    &mut commands,
                    &skin,
                    window,
                    request.id,
                    label,
                    Choice::Context(index),
                );
            }
            let (rotate, wrap) = owner.furniture(menu.as_deref());
            for (shown, caption, choice) in [
                (rotate, "Rotate", Choice::Rotate),
                (wrap, "Wrap / Unwrap", Choice::Wrap),
            ] {
                if shown {
                    button(&mut commands, &skin, window, request.id, caption, choice);
                }
            }
            if inspection.is_some() {
                button(
                    &mut commands,
                    &skin,
                    window,
                    request.id,
                    "Inspect",
                    Choice::Inspect,
                );
            }
            if let Some(target) = party {
                if let Some(name) = target.name() {
                    let label = commands
                        .spawn(source::text(name, source::font(skin.font.clone(), false)))
                        .insert((
                            TextLayout::default(),
                            Node {
                                width: percent(100.),
                                ..default()
                            },
                        ))
                        .id();
                    commands.entity(window).add_child(label);
                }
                let invite = button(
                    &mut commands,
                    &skin,
                    window,
                    request.id,
                    tr!("Invite to Party"),
                    Choice::Invite,
                );
                if !target.invite_enabled() {
                    commands
                        .entity(invite)
                        .insert(bevy::ui::InteractionDisabled);
                }
                button(
                    &mut commands,
                    &skin,
                    window,
                    request.id,
                    "Party",
                    Choice::Party,
                );
            }
        }
    }
    let cancel = button(
        &mut commands,
        &skin,
        window,
        request.id,
        tr!("Cancel"),
        Choice::Cancel,
    );
    focus.set(cancel, FocusCause::Navigated);
}
fn button(
    commands: &mut Commands,
    skin: &Skin,
    parent: Entity,
    id: u64,
    caption: &str,
    choice: Choice,
) -> Entity {
    let button = commands
        .spawn((
            source::button(skin.buttons.clone(), 260.),
            Reply { id, choice },
        ))
        .observe(activate)
        .id();
    let text = commands
        .spawn(source::button_caption(caption, skin.font.clone()))
        .id();
    commands.entity(button).add_child(text);
    commands.entity(parent).add_child(button);
    button
}
