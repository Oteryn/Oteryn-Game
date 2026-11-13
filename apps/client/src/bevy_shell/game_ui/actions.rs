//! Source-sized action rows; assignment and activation stay with the existing owner.
use super::super::{owner::HostOwner, ui::Skins};
use crate::play::BarGesture;
use bevy::{prelude::*, ui_widgets::Activate};
use oteryn_client_ui_bevy as source;

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
        .add_systems(Update, (projection::project, editor::project).chain());
}

fn setup(mut commands: Commands, skins: Res<Skins>, regions: Res<super::GameRegions>) {
    let tools = commands
        .spawn(Node {
            width: percent(100.),
            height: px(20.),
            min_height: px(20.),
            column_gap: px(2.),
            ..default()
        })
        .id();
    for bar in 0..3 {
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
    for bar in 0..3 {
        let row = commands
            .spawn((
                BarRow(bar),
                Node {
                    width: percent(100.),
                    min_height: px(SLOT_SIZE),
                    height: px(SLOT_SIZE),
                    flex_shrink: 0.,
                    column_gap: px(SLOT_GAP),
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
    for marker in [false, true] {
        let entity = commands
            .spawn(source::text("", source::font(skins.font.clone(), true)))
            .insert(Node {
                max_height: px(13.),
                overflow: Overflow::clip(),
                ..default()
            })
            .id();
        if marker {
            commands.entity(entity).insert(ActionHint);
        } else {
            commands.entity(entity).insert(CooldownLine);
        }
        commands.entity(regions.bars).add_child(entity);
    }
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
    controls: Query<(&ActionControl, &Generation)>,
    owner: NonSend<HostOwner>,
    mut editor: ResMut<editor::EditorState>,
) {
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
            if row.locked {
                if play.view.gameplay_input_blocked(&play.link) {
                    return;
                }
                play.link.bar_gesture(BarGesture::Cast { slot });
            } else {
                editor.open(slot, owner.generation());
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
