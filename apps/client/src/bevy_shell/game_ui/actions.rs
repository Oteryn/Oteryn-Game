//! Source-sized action rows; assignment and activation stay with the existing owner.
use super::super::{owner::HostOwner, ui::Skins};
use crate::bevy_shell::game_ui::modal::ModalGuard;
use crate::bevy_shell::gameplay_dialogs::{
    art::{self, ArtSlot},
    popup,
    scale::{Module, ModuleScale, Scaled, ScaledFont},
};
/// The action-bar module's scale tag for a node of `size` at 100%.
fn bar_size(size: Vec2) -> Scaled {
    Scaled {
        module: Module::ActionBars,
        size,
    }
}
/// The action-bar module's scale tag for a text of `size` pixels at 100%.
fn bar_font(size: f32) -> ScaledFont {
    ScaledFont {
        module: Module::ActionBars,
        size,
    }
}
use crate::play::BarGesture;
use bevy::{prelude::*, ui_widgets::Activate};
use oteryn_client_ui_bevy as source;

mod cooldown;
#[cfg(test)]
mod cooldown_tests;
mod dragdrop;
mod editor;
#[cfg(test)]
mod item_drag_tests;
mod projection;
#[cfg(test)]
mod tests;

// TibiaStyle.actionBarSlotSize / actionBarSlotMargin.
const SLOT_SIZE: f32 = 34.;
const SLOT_GAP: f32 = 2.;
const ROW_SLOTS: usize = 50;
// Tibia Global action bar QML: a 12 x 16 lock button on each bar, 17 x 17 scroll buttons.
const LOCK_SIZE: Vec2 = Vec2::new(12., 16.);
const SCROLL_SIZE: f32 = 17.;

#[derive(Component, Clone, Copy)]
enum ActionControl {
    Slot(usize),
    Lock(usize),
    Scroll(usize, i8),
}
/// A row of the slot popup menu.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
enum MenuChoice {
    AssignSpell,
    AssignObject,
    SetText,
    EditHotkey,
    Clear,
    Lock,
    Hide,
    ResetZoom,
}
/// The hotkey of a slot, drawn small in its top-right corner.
#[derive(Component)]
struct SlotHotkey(usize);
#[derive(Component)]
struct MenuEntry {
    slot: usize,
    generation: u64,
    choice: MenuChoice,
}
/// The open slot popup menu.
#[derive(Resource, Default)]
struct SlotMenu(Option<Entity>);
#[derive(Component)]
struct Generation(u64);
#[derive(Component)]
struct BarRow(usize);
#[derive(Component)]
struct DockAnchor(crate::play::BarDock);
#[derive(Component)]
struct SlotCaption(usize);
#[derive(Component)]
struct ActionHint;
#[derive(Component)]
struct CooldownLine;

pub(super) fn install(app: &mut App) {
    editor::install(app);
    app.init_resource::<editor::EditorState>()
        .init_resource::<SlotMenu>()
        .add_systems(Startup, (setup, editor::setup).after(super::setup))
        .add_systems(
            Update,
            (
                close_menu,
                projection::project,
                editor::project,
                cooldown::project,
            )
                .chain(),
        );
}

fn setup(mut commands: Commands, skins: Res<Skins>, regions: Res<super::GameRegions>) {
    for dock in [
        crate::play::BarDock::Bottom,
        crate::play::BarDock::Left,
        crate::play::BarDock::Right,
    ] {
        let side = dock != crate::play::BarDock::Bottom;
        let anchor = commands
            .spawn((
                DockAnchor(dock),
                Node {
                    position_type: if side {
                        PositionType::Absolute
                    } else {
                        PositionType::Relative
                    },
                    left: if dock == crate::play::BarDock::Left {
                        px(0.)
                    } else {
                        Val::Auto
                    },
                    right: if dock == crate::play::BarDock::Right {
                        px(0.)
                    } else {
                        Val::Auto
                    },
                    top: if side { px(0.) } else { Val::Auto },
                    width: if side { Val::Auto } else { percent(100.) },
                    height: if side { percent(100.) } else { Val::Auto },
                    flex_direction: if side {
                        FlexDirection::Row
                    } else {
                        FlexDirection::Column
                    },
                    ..default()
                },
            ))
            .id();
        commands
            .entity(if side { regions.world } else { regions.bars })
            .add_child(anchor);
    }
    for bar in 0..9 {
        let row = commands
            .spawn((
                BarRow(bar),
                crate::bevy_shell::gameplay_dialogs::scale::ModuleRoot(Module::ActionBars),
                Node {
                    width: percent(100.),
                    min_height: px(SLOT_SIZE),
                    height: px(SLOT_SIZE),
                    flex_shrink: 0.,
                    column_gap: px(SLOT_GAP),
                    row_gap: px(SLOT_GAP),
                    overflow: Overflow::clip(),
                    ..default()
                },
            ))
            .id();
        let previous = button(
            &mut commands,
            &skins,
            ActionControl::Scroll(bar, -1),
            SCROLL_SIZE,
            "<",
        );
        commands.entity(row).add_child(previous);
        for local in 0..ROW_SLOTS {
            let slot = bar * ROW_SLOTS + local;
            let entity = commands
                .spawn((
                    source::button(skins.buttons.clone(), SLOT_SIZE),
                    ActionControl::Slot(slot),
                    Generation(u64::MAX),
                    bar_size(Vec2::splat(SLOT_SIZE)),
                ))
                .insert(Node {
                    width: px(SLOT_SIZE),
                    min_width: px(SLOT_SIZE),
                    height: px(SLOT_SIZE),
                    min_height: px(SLOT_SIZE),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    flex_shrink: 0.,
                    overflow: Overflow::clip(),
                    ..default()
                })
                .observe(activate)
                .observe(edit_click)
                .observe(dragdrop::drop_slot)
                .id();
            let caption = commands
                .spawn((
                    SlotCaption(slot),
                    source::text("", source::font(skins.font.clone(), true)),
                    bar_font(8.),
                ))
                .insert((
                    TextLayout::default(),
                    Node {
                        max_width: px(32.),
                        max_height: px(32.),
                        ..default()
                    },
                ))
                .id();
            let hotkey = commands
                .spawn((
                    SlotHotkey(slot),
                    source::text("", source::font(skins.font.clone(), true)),
                    bar_font(8.),
                    Node {
                        position_type: PositionType::Absolute,
                        top: px(1.),
                        right: px(2.),
                        ..default()
                    },
                ))
                .id();
            commands.entity(entity).add_children(&[caption, hotkey]);
            commands.entity(row).add_child(entity);
        }
        let next = button(
            &mut commands,
            &skins,
            ActionControl::Scroll(bar, 1),
            SCROLL_SIZE,
            ">",
        );
        // The bar's lock replaces the old "1 on / Locked" configuration row.
        let lock = commands
            .spawn((
                source::button(skins.buttons.clone(), LOCK_SIZE.x),
                ActionControl::Lock(bar),
                Generation(u64::MAX),
                bar_size(LOCK_SIZE),
            ))
            .insert(Node {
                width: px(LOCK_SIZE.x),
                min_width: px(LOCK_SIZE.x),
                height: px(LOCK_SIZE.y),
                min_height: px(LOCK_SIZE.y),
                flex_shrink: 0.,
                align_self: AlignSelf::Center,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            })
            .observe(activate)
            .id();
        let icon = art::icon(
            &mut commands,
            ArtSlot::ActionBarLocked,
            skins.font.clone(),
            LOCK_SIZE - Vec2::splat(2.),
        );
        commands
            .entity(icon)
            .insert(bar_size(LOCK_SIZE - Vec2::splat(2.)));
        commands.entity(lock).add_child(icon);
        commands.entity(row).add_children(&[next, lock]);
        commands.entity(regions.bars).add_child(row);
    }
    let cooldown = commands
        .spawn((
            CooldownLine,
            Node {
                width: percent(100.),
                min_height: px(24.),
                column_gap: px(3.),
                overflow: Overflow::clip(),
                ..default()
            },
        ))
        .id();
    let hint = commands
        .spawn((
            ActionHint,
            source::text("", source::font(skins.font.clone(), true)),
            Node {
                max_height: px(13.),
                overflow: Overflow::clip(),
                ..default()
            },
        ))
        .id();
    commands
        .entity(regions.bars)
        .add_children(&[cooldown, hint]);
}

fn button(
    commands: &mut Commands,
    skins: &Skins,
    control: ActionControl,
    width: f32,
    caption: &str,
) -> Entity {
    let button = commands
        .spawn((
            source::button(skins.buttons.clone(), width),
            control,
            Generation(u64::MAX),
        ))
        .observe(activate)
        .id();
    let label = commands
        .spawn(source::button_caption(caption, skins.font.clone()))
        .id();
    commands.entity(button).add_child(label);
    button
}

fn activate(
    event: On<Activate>,
    guard: ModalGuard,
    controls: Query<(&ActionControl, &Generation)>,
    owner: NonSend<HostOwner>,
    editor: Res<editor::EditorState>,
) {
    if !guard.allows(event.entity) {
        return;
    }
    let Ok((control, generation)) = controls.get(event.entity) else {
        return;
    };
    if owner.modal_active() || editor.active() || generation.0 != owner.generation() {
        return;
    }
    let Some(play) = &owner.play else {
        return;
    };
    let layout = play.link.bar_layout();
    match *control {
        ActionControl::Slot(slot) => {
            let Some(row) = layout.rows.get(slot / ROW_SLOTS) else {
                return;
            };
            if !row.enabled || slot % ROW_SLOTS >= usize::from(row.count) {
                return;
            }
            if play.view.gameplay_input_blocked(&play.link) {
                return;
            }
            if let Ok(slot) = u32::try_from(slot + 1)
                && let Some(slot) = std::num::NonZeroU32::new(slot)
            {
                play.link.cast_spell_slot(slot);
            }
        }
        ActionControl::Lock(bar) => configure(&play.link, bar, false, true),
        ActionControl::Scroll(bar, by) => {
            play.link.bar_gesture(BarGesture::Scroll { bar, by });
        }
    }
}

/// Flips a bar's visibility (`hide`) or lock (`lock`) through the existing layout owner.
fn configure(link: &crate::play::PlayLink, bar: usize, hide: bool, lock: bool) {
    let layout = link.bar_layout();
    let Some(row) = layout.rows.get(bar) else {
        return;
    };
    link.bar_gesture(BarGesture::Configure {
        bar,
        dock: row.dock,
        enabled: row.enabled != hide,
        locked: row.locked != lock,
    });
}

/// The slot popup: the slot's assignments, then the bar's lock and visibility, which moved
/// here from the removed configuration row. Assignments are refused on a locked bar.
fn menu_items(locked: bool) -> Vec<(popup::Item, Option<MenuChoice>)> {
    vec![
        (
            popup::Item::entry("Assign Spell"),
            Some(MenuChoice::AssignSpell),
        ),
        (
            popup::Item::entry("Assign Object"),
            Some(MenuChoice::AssignObject),
        ),
        (popup::Item::entry("Set Text"), Some(MenuChoice::SetText)),
        (
            popup::Item::entry("Edit Hotkey"),
            Some(MenuChoice::EditHotkey),
        ),
        (popup::Item::entry("Clear"), Some(MenuChoice::Clear)),
        (popup::Item::Separator, None),
        (
            popup::Item::check("Lock Action Bar", locked),
            Some(MenuChoice::Lock),
        ),
        (
            popup::Item::entry("Hide Action Bar"),
            Some(MenuChoice::Hide),
        ),
        (popup::Item::Separator, None),
        (
            popup::Item::entry("Reset Zoom"),
            Some(MenuChoice::ResetZoom),
        ),
    ]
}

// The menu owns its own widgets and the shared editor; the queries do not alias.
#[allow(clippy::too_many_arguments)]
fn edit_click(
    event: On<PointerClick>,
    guard: ModalGuard,
    controls: Query<(&ActionControl, &Generation)>,
    owner: NonSend<HostOwner>,
    editor: Res<editor::EditorState>,
    skins: Res<Skins>,
    mut menu: ResMut<SlotMenu>,
    mut commands: Commands,
) {
    if event.button != bevy::picking::pointer::PointerButton::Secondary
        || !guard.allows(event.entity)
        || owner.modal_active()
        || editor.active()
    {
        return;
    }
    let Ok((ActionControl::Slot(slot), generation)) = controls.get(event.entity) else {
        return;
    };
    let Some(play) = &owner.play else {
        return;
    };
    if generation.0 != owner.generation() || play.view.gameplay_input_blocked(&play.link) {
        return;
    }
    let Some(row) = play.link.bar_layout().rows.get(*slot / ROW_SLOTS).cloned() else {
        return;
    };
    if !row.enabled {
        return;
    }
    if let Some(old) = menu.0.take() {
        commands.entity(old).despawn();
    }
    let rows = menu_items(row.locked);
    let items: Vec<_> = rows.iter().map(|(item, _)| item.clone()).collect();
    let look = popup::Look {
        font: skins.font.clone(),
        frame: skins.frame.clone(),
        checked: Some(skins.checked.clone()),
        unchecked: Some(skins.unchecked.clone()),
    };
    let opened = popup::spawn(
        &mut commands,
        &look,
        event.pointer.location().position,
        &items,
    );
    let layer = commands
        .spawn((
            super::GameModal,
            GlobalZIndex(200),
            Node {
                position_type: PositionType::Absolute,
                width: percent(100.),
                height: percent(100.),
                ..default()
            },
        ))
        .add_children(&[opened.scrim, opened.root])
        .id();
    commands.entity(opened.scrim).observe(dismiss_menu);
    for (entity, choice) in opened
        .entries
        .iter()
        .zip(rows.iter().filter_map(|(_, choice)| *choice))
    {
        commands
            .entity(*entity)
            .insert(MenuEntry {
                slot: *slot,
                generation: owner.generation(),
                choice,
            })
            .observe(menu_action);
        if row.locked
            && !matches!(
                choice,
                MenuChoice::Lock | MenuChoice::Hide | MenuChoice::ResetZoom
            )
        {
            commands
                .entity(*entity)
                .insert(bevy::ui::InteractionDisabled);
        }
    }
    menu.0 = Some(layer);
}

fn dismiss_menu(_event: On<PointerClick>, mut menu: ResMut<SlotMenu>, mut commands: Commands) {
    if let Some(layer) = menu.0.take() {
        commands.entity(layer).despawn();
    }
}

/// Escape closes the slot menu; a new session generation drops it.
fn close_menu(
    keys: Option<Res<ButtonInput<KeyCode>>>,
    owner: NonSend<HostOwner>,
    entries: Query<&MenuEntry>,
    mut menu: ResMut<SlotMenu>,
    mut commands: Commands,
) {
    let Some(layer) = menu.0 else {
        return;
    };
    let escape = keys.is_some_and(|keys| keys.just_pressed(KeyCode::Escape));
    let stale = owner.play.is_none()
        || entries
            .iter()
            .any(|entry| entry.generation != owner.generation());
    if escape || stale {
        commands.entity(layer).despawn();
        menu.0 = None;
    }
}

fn menu_action(
    event: On<Activate>,
    guard: ModalGuard,
    entries: Query<&MenuEntry>,
    owner: NonSend<HostOwner>,
    mut editor: ResMut<editor::EditorState>,
    (mut menu, mut scale): (ResMut<SlotMenu>, Option<ResMut<ModuleScale>>),
    mut commands: Commands,
) {
    if !guard.allows(event.entity) {
        return;
    }
    let Ok(entry) = entries.get(event.entity) else {
        return;
    };
    if let Some(layer) = menu.0.take() {
        commands.entity(layer).despawn();
    }
    let Some(play) = &owner.play else {
        return;
    };
    if entry.generation != owner.generation() || play.view.gameplay_input_blocked(&play.link) {
        return;
    }
    let bar = entry.slot / ROW_SLOTS;
    let Some(row) = play.link.bar_layout().rows.get(bar).cloned() else {
        return;
    };
    match entry.choice {
        MenuChoice::Lock => configure(&play.link, bar, false, true),
        MenuChoice::Hide => configure(&play.link, bar, true, false),
        MenuChoice::ResetZoom => {
            if let Some(scale) = scale.as_mut() {
                scale.set(Module::ActionBars, 1.);
            }
        }
        _ if !row.enabled || row.locked => {}
        // The shared editor holds the spell catalogue, object, text and hotkey fields.
        MenuChoice::AssignSpell
        | MenuChoice::AssignObject
        | MenuChoice::SetText
        | MenuChoice::EditHotkey => editor.open(entry.slot, owner.generation()),
        MenuChoice::Clear => {
            if entry.slot < ROW_SLOTS * 3 {
                play.link.clear_bar_action(entry.slot);
                play.link.assign_spell_slot(entry.slot, None);
            }
        }
    }
}
