use super::*;
use bevy::input_focus::FocusCause;
pub(super) fn setup(mut commands: Commands) {
    commands.spawn((
        Overlay,
        Name::new("exit.dialog"),
        GameModal,
        GlobalZIndex(220),
        Node {
            position_type: PositionType::Absolute,
            width: percent(100.),
            height: percent(100.),
            display: Display::None,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        BackgroundColor(Color::srgba(0., 0., 0., 0.45)),
    ));
}
pub(super) fn project(
    mut commands: Commands,
    mut state: ResMut<State>,
    skins: Res<Skins>,
    mut overlays: Query<(Entity, &mut Node), With<Overlay>>,
    replies: Query<(), With<Reply>>,
    mut focus: ResMut<InputFocus>,
) {
    let mut root = None;
    for (entity, mut node) in &mut overlays {
        node.display = if state.pending.is_some() {
            Display::Flex
        } else {
            Display::None
        };
        root = Some(entity);
    }
    if state.rendered == state.pending {
        return;
    }
    if let Some(panel) = state.panel.take() {
        commands.entity(panel).despawn();
    }
    if focus.get().is_some_and(|entity| replies.contains(entity)) {
        focus.clear();
    }
    state.rendered = state.pending;
    let (Some(id), Some(root)) = (state.pending, root) else {
        return;
    };
    let panel = commands
        .spawn((
            crate::bevy_shell::window_drag::DraggableDialog,
            source::frame(skins.frame.clone(), [4., 17., 4., 4.]),
            TabGroup {
                order: 220,
                modal: true,
            },
            Name::new("exit.panel"),
            Node {
                width: px(340.),
                height: px(140.),
                max_width: percent(98.),
                max_height: percent(98.),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::SpaceBetween,
                padding: UiRect {
                    left: px(8.),
                    right: px(8.),
                    top: px(25.),
                    bottom: px(8.),
                },
                row_gap: px(8.),
                ..default()
            },
        ))
        .id();
    source::dialog_caption(
        &mut commands,
        panel,
        "Oteryn confirmation",
        skins.font.clone(),
    );
    let body = commands
        .spawn((
            source::text("Close Oteryn?", source::font(skins.font.clone(), false)),
            Node {
                flex_shrink: 0.,
                ..default()
            },
        ))
        .id();
    let footer = commands
        .spawn(Node {
            justify_content: JustifyContent::End,
            column_gap: px(8.),
            ..default()
        })
        .id();
    let mut cancel = None;
    for (confirm, caption) in [(true, "Yes"), (false, "No")] {
        let button = commands
            .spawn((
                source::button(skins.buttons.clone(), 80.),
                Name::new(if confirm { "exit.yes" } else { "exit.no" }),
                Reply { id, confirm },
            ))
            .observe(activate)
            .id();
        let caption = commands
            .spawn(source::button_caption(caption, skins.font.clone()))
            .id();
        commands.entity(button).add_child(caption);
        commands.entity(footer).add_child(button);
        if !confirm {
            cancel = Some(button);
        }
    }
    commands.entity(panel).add_children(&[body, footer]);
    commands.entity(root).add_child(panel);
    state.panel = Some(panel);
    if let Some(cancel) = cancel {
        focus.set(cancel, FocusCause::Navigated);
    }
}
