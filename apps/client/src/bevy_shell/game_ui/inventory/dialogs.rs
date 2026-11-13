use super::super::GameModal;
use super::view::{Projection, label};
use super::*;
use crate::bevy_shell::game_ui::modal::ModalGuard;
use oteryn_client_ui_bevy as source;
pub(super) fn button(
    commands: &mut Commands,
    skins: &Skins,
    parent: Entity,
    text: &str,
    control: Control,
    width: f32,
) {
    let entity = commands
        .spawn((control, source::button(skins.buttons.clone(), width)))
        .observe(activate)
        .id();
    let caption = commands
        .spawn(source::button_caption(text, skins.font.clone()))
        .id();
    commands.entity(entity).add_child(caption);
    commands.entity(parent).add_child(entity);
}
fn modal(commands: &mut Commands, skins: &Skins, width: f32) -> Entity {
    commands
        .spawn((
            Projection,
            GameModal,
            bevy::input_focus::tab_navigation::TabGroup {
                order: 4,
                modal: true,
            },
            source::frame(skins.frame.clone(), [4., 17., 4., 4.]),
            Node {
                position_type: PositionType::Absolute,
                width: px(width),
                max_width: percent(95.),
                padding: UiRect::axes(px(8.), px(22.)),
                flex_direction: FlexDirection::Column,
                row_gap: px(4.),
                ..default()
            },
            GlobalZIndex(80),
        ))
        .id()
}
pub(super) fn context(commands: &mut Commands, skins: &Skins, slot: Slot, position: Vec2) {
    use oteryn_client_ui::world_interaction::menu::Action;
    let overlay = commands
        .spawn((
            Projection,
            GameModal,
            MenuScrim,
            Node {
                position_type: PositionType::Absolute,
                width: percent(100.),
                height: percent(100.),
                ..default()
            },
            GlobalZIndex(80),
            BackgroundColor(Color::srgba(0., 0., 0., 0.01)),
        ))
        .observe(dismiss_menu)
        .id();
    let panel = modal(commands, skins, 140.);
    commands.entity(panel).remove::<Projection>();
    commands.entity(overlay).add_child(panel);
    // Keep the popover inside the primary viewport, including near the right sidebar.
    commands.entity(panel).insert(Node {
        position_type: PositionType::Absolute,
        left: px(position.x.max(0.)),
        top: px(position.y.max(0.)),
        width: px(140.),
        max_width: percent(95.),
        padding: UiRect::all(px(5.)),
        flex_direction: FlexDirection::Column,
        row_gap: px(2.),
        ..default()
    });
    button(commands, skins, panel, "Open", Control::Open(slot), 126.);
    for (text, action) in [
        ("Use", Action::Use),
        ("Look", Action::Look),
        ("Use with…", Action::UseWith),
        ("Use rune…", Action::UseRune),
    ] {
        button(
            commands,
            skins,
            panel,
            text,
            Control::Context(slot, action),
            126.,
        );
    }
    button(
        commands,
        skins,
        panel,
        "Cancel",
        Control::Cancel {
            generation: slot.generation,
            prompt: None,
        },
        126.,
    );
}
pub(super) fn split(commands: &mut Commands, skins: &Skins, prompt: Prompt, count: u32) {
    let overlay = commands
        .spawn((
            Projection,
            GameModal,
            Node {
                position_type: PositionType::Absolute,
                width: percent(100.),
                height: percent(100.),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            GlobalZIndex(90),
            BackgroundColor(Color::srgba(0., 0., 0., 0.3)),
        ))
        .id();
    let panel = modal(commands, skins, 220.);
    commands.entity(panel).remove::<Projection>();
    commands.entity(panel).insert(Node {
        width: px(220.),
        padding: UiRect::axes(px(12.), px(24.)),
        flex_direction: FlexDirection::Column,
        row_gap: px(6.),
        ..default()
    });
    commands.entity(overlay).add_child(panel);
    source::dialog_caption(commands, panel, "Move stack", skins.font.clone());
    label(
        commands,
        skins,
        panel,
        &format!("{count} of {}", prompt.item.count.min(100)),
    );
    let row = commands
        .spawn(Node {
            column_gap: px(5.),
            ..default()
        })
        .id();
    commands.entity(panel).add_child(row);
    button(commands, skins, row, "−", Control::Count(prompt, -1), 28.);
    button(commands, skins, row, "+", Control::Count(prompt, 1), 28.);
    button(commands, skins, row, "Move", Control::Confirm(prompt), 50.);
    button(
        commands,
        skins,
        row,
        "Cancel",
        Control::Cancel {
            generation: prompt.generation,
            prompt: Some(prompt),
        },
        50.,
    );
}

#[derive(Component)]
struct MenuScrim;
fn dismiss_menu(
    event: On<bevy::picking::events::PointerClick>,
    guard: ModalGuard,
    scrims: Query<(), With<MenuScrim>>,
    mut ui: ResMut<InventoryUi>,
) {
    if !guard.allows(event.entity) {
        return;
    }
    if scrims.contains(event.entity) {
        ui.menu = None;
    }
}
