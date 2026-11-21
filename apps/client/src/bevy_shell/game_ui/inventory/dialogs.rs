use super::super::GameModal;
use super::view::Projection;
use super::*;
use crate::bevy_shell::game_ui::modal::ModalGuard;
use crate::bevy_shell::gameplay_dialogs::popup;
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
/// Logical height of the item context menu: five fixed entries plus the admitted `extra`
/// entries, each 16 pixels high with 3-pixel spacing inside the 4-pixel popup padding.
pub(super) fn menu_height(extra: (bool, bool)) -> f32 {
    let rows = 5. + f32::from(u8::from(extra.0) + u8::from(extra.1));
    rows * popup::ENTRY_HEIGHT + (rows - 1.) * popup::ENTRY_SPACING + 8.
}
/// The item menu entries in Tibia order. `extra` admits the writing and timed item windows
/// (capabilities 115 and 39).
pub(super) fn menu_entries(slot: Slot, extra: (bool, bool)) -> Vec<(popup::Item, Control)> {
    use oteryn_client_ui::world_interaction::menu::Action;
    let mut entries = vec![
        (
            popup::Item::shortcut("Look", "Shift"),
            Control::Context(slot, Action::Look),
        ),
        (
            popup::Item::shortcut("Use", "Ctrl"),
            Control::Context(slot, Action::Use),
        ),
        (
            popup::Item::shortcut("Use with ...", "Ctrl"),
            Control::Context(slot, Action::UseWith),
        ),
        (
            popup::Item::entry("Use rune ..."),
            Control::Context(slot, Action::UseRune),
        ),
        (popup::Item::entry("Open"), Control::Open(slot)),
    ];
    if extra.0 {
        entries.push((popup::Item::entry("Write ..."), Control::Write(slot)));
    }
    if extra.1 {
        entries.push((popup::Item::entry("Timed item ..."), Control::Timed(slot)));
    }
    entries
}
/// The item popup menu at `position`; a click outside or Escape closes it.
pub(super) fn context(
    commands: &mut Commands,
    skins: &Skins,
    slot: Slot,
    position: Vec2,
    extra: (bool, bool),
) {
    let entries = menu_entries(slot, extra);
    let items: Vec<_> = entries.iter().map(|(item, _)| item.clone()).collect();
    let look = popup::Look {
        font: skins.font.clone(),
        frame: skins.frame.clone(),
        checked: Some(skins.checked.clone()),
        unchecked: Some(skins.unchecked.clone()),
    };
    let opened = popup::spawn(commands, &look, position, &items);
    commands
        .spawn((
            Projection,
            GameModal,
            Node {
                position_type: PositionType::Absolute,
                width: percent(100.),
                height: percent(100.),
                ..default()
            },
            GlobalZIndex(80),
        ))
        .add_children(&[opened.scrim, opened.root]);
    commands
        .entity(opened.scrim)
        .insert(MenuScrim)
        .observe(dismiss_menu);
    for (entity, (_, control)) in opened.entries.iter().zip(entries) {
        commands.entity(*entity).insert(control).observe(activate);
    }
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
