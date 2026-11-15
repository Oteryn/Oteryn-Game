//! Source-sized action rows; assignment and activation stay with the existing owner.
use super::super::{owner::HostOwner, ui::Skins};
use crate::bevy_shell::game_ui::modal::ModalGuard;
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

#[derive(Component, Clone, Copy)]
enum ActionControl {
    Slot(usize),
    Toggle(usize),
    Lock(usize),
    Scroll(usize, i8),
}
#[derive(Component)]
struct Generation(u64);
#[derive(Component)]
struct BarRow(usize);
#[derive(Component)]
struct DockAnchor(crate::play::BarDock);
#[derive(Component)]
struct SlotCaption(usize);
#[derive(Component)]
struct ToolCaption(ActionControl);
#[derive(Component)]
struct ActionHint;
#[derive(Component)]
struct CooldownLine;

pub(super) fn install(app: &mut App) {
    editor::install(app);
    app.init_resource::<editor::EditorState>()
        .add_systems(Startup, (setup, editor::setup).after(super::setup))
        .add_systems(
            Update,
            (projection::project, editor::project, cooldown::project).chain(),
        );
}

fn setup(mut commands: Commands, skins: Res<Skins>, regions: Res<super::GameRegions>) {
    let tools = commands
        .spawn(Node {
            width: percent(100.),
            height: Val::Auto,
            min_height: px(20.),
            flex_wrap: FlexWrap::Wrap,
            column_gap: px(2.),
            ..default()
        })
        .id();
    for bar in 0..9 {
        let toggle = button(
            &mut commands,
            &skins,
            ActionControl::Toggle(bar),
            30.,
            &format!("{}", bar + 1),
        );
        let lock = button(
            &mut commands,
            &skins,
            ActionControl::Lock(bar),
            48.,
            "Locked",
        );
        commands.entity(tools).add_children(&[toggle, lock]);
    }
    commands.entity(regions.bars).add_child(tools);
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
            17.,
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
            commands.entity(entity).add_child(caption);
            commands.entity(row).add_child(entity);
        }
        let next = button(
            &mut commands,
            &skins,
            ActionControl::Scroll(bar, 1),
            17.,
            ">",
        );
        commands.entity(row).add_child(next);
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
        .spawn((
            source::button_caption(caption, skins.font.clone()),
            ToolCaption(control),
        ))
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
        ActionControl::Toggle(bar) | ActionControl::Lock(bar) => {
            let Some(row) = layout.rows.get(bar) else {
                return;
            };
            play.link.bar_gesture(BarGesture::Configure {
                bar,
                dock: row.dock,
                enabled: if matches!(control, ActionControl::Toggle(_)) {
                    !row.enabled
                } else {
                    row.enabled
                },
                locked: if matches!(control, ActionControl::Lock(_)) {
                    !row.locked
                } else {
                    row.locked
                },
            });
        }
        ActionControl::Scroll(bar, by) => {
            play.link.bar_gesture(BarGesture::Scroll { bar, by });
        }
    }
}

fn edit_click(
    event: On<PointerClick>,
    guard: ModalGuard,
    controls: Query<(&ActionControl, &Generation)>,
    owner: NonSend<HostOwner>,
    mut editor: ResMut<editor::EditorState>,
) {
    if event.button != bevy::picking::pointer::PointerButton::Secondary
        || !guard.allows(event.entity)
        || owner.modal_active()
    {
        return;
    }
    let Ok((ActionControl::Slot(slot), generation)) = controls.get(event.entity) else {
        return;
    };
    let Some(play) = &owner.play else {
        return;
    };
    if generation.0 == owner.generation()
        && !play.view.gameplay_input_blocked(&play.link)
        && play
            .link
            .bar_layout()
            .rows
            .get(*slot / ROW_SLOTS)
            .is_some_and(|row| row.enabled && !row.locked)
    {
        editor.open(*slot, owner.generation());
    }
}
