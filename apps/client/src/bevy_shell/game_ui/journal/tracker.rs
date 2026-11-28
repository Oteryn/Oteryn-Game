use super::*;
use oteryn_client_ui::quest_log::QuestLine;

#[derive(Component)]
pub(super) struct Window {
    generation: u64,
    index: usize,
}
#[derive(Clone, Copy, PartialEq)]
pub(super) enum Control {
    Add,
    Close,
    Journal,
    Mission(u32, u32),
}
#[derive(Component)]
pub(super) struct ButtonAction {
    pub(super) kind: Control,
    generation: u64,
    index: usize,
}
#[derive(Resource, Default)]
struct Windows {
    generation: Option<u64>,
    slots: [bool; 8],
    lines: Vec<(String, QuestLine)>,
    mission_count: Option<String>,
    full: bool,
}
pub(super) fn install(app: &mut App) {
    app.init_resource::<Windows>().add_systems(Update, project);
}
fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skins: Res<Skins>,
    mut windows: ResMut<Windows>,
    roots: Query<(Entity, &Window)>,
) {
    let active = owner
        .play
        .as_ref()
        .filter(|p| p.link.quest_tracker_visible());
    let generation = active.map(|_| owner.generation());
    if generation != windows.generation {
        windows.slots = [false; 8];
        windows.slots[0] = active.is_some();
        windows.generation = generation;
    }
    let panel = active.map(|p| p.link.quest_panel());
    let mission_count = panel
        .as_ref()
        .filter(|p| p.state.mission_tracking)
        .map(oteryn_client_ui::quest_log::mission_tracking_summary);
    let lines = panel
        .as_ref()
        .map(|panel| {
            oteryn_client_ui::quest_log::tracker_lines(panel)
                .into_iter()
                .map(|line| {
                    let mut line = line.clone();
                    if panel.tracker.hide_completed_missions {
                        line.missions.retain(|m| !m.done);
                    }
                    (panel.title(line.quest_index), line)
                })
                .collect()
        })
        .unwrap_or_default();
    let full = !windows.slots.contains(&false);
    let changed =
        windows.lines != lines || windows.full != full || windows.mission_count != mission_count;
    windows.full = full;
    let mut mounted = [None; 8];
    for (entity, window) in &roots {
        if Some(window.generation) == generation && windows.slots[window.index] {
            mounted[window.index] = Some(entity);
        } else {
            commands.entity(entity).despawn();
        }
    }
    windows.lines = lines;
    windows.mission_count = mission_count;
    let Some(generation) = generation else {
        return;
    };
    for (index, slot) in windows.slots.iter().enumerate() {
        if !slot {
            continue;
        }
        let root = if let Some(entity) = mounted[index] {
            if !changed {
                continue;
            }
            commands.entity(entity).despawn_related::<Children>();
            entity
        } else {
            commands
                .spawn((
                    Window { index, generation },
                    crate::bevy_shell::window_drag::DraggableDialog,
                    GlobalZIndex(110),
                    source::frame(skins.frame.clone(), [4., 17., 4., 4.]),
                    Node {
                        position_type: PositionType::Absolute,
                        right: px(10. + (index % 4) as f32 * 32.),
                        top: px(68. + index as f32 * 32.),
                        width: px(245.),
                        max_height: percent(75.),
                        overflow: Overflow::scroll_y(),
                        flex_direction: FlexDirection::Column,
                        padding: UiRect {
                            top: px(19.),
                            ..px(4.).all()
                        },
                        ..default()
                    },
                    bevy::ui_widgets::ScrollArea,
                ))
                .id()
        };
        source::dialog_caption(&mut commands, root, "Quest tracker", skins.font.clone());
        let button = |commands: &mut Commands, kind, label: &str| {
            let entity = commands
                .spawn((
                    ButtonAction {
                        kind,
                        generation,
                        index,
                    },
                    source::button(skins.buttons.clone(), 225.),
                ))
                .observe(activate)
                .id();
            let text = commands
                .spawn(source::button_caption(label, skins.font.clone()))
                .id();
            commands.entity(entity).add_child(text);
            commands.entity(root).add_child(entity);
        };
        button(&mut commands, Control::Journal, "Open quest journal");
        if !full {
            button(&mut commands, Control::Add, "New tracker window");
        }
        button(&mut commands, Control::Close, "Close tracker");
        if let Some(count) = &windows.mission_count {
            let text = commands
                .spawn(source::text(count, source::font(skins.font.clone(), false)))
                .id();
            commands.entity(root).add_child(text);
        }
        for (title, line) in &windows.lines {
            let heading = commands
                .spawn(source::text(title, source::font(skins.font.clone(), true)))
                .id();
            commands.entity(root).add_child(heading);
            for mission in &line.missions {
                button(
                    &mut commands,
                    Control::Mission(line.quest_index, mission.mission_index),
                    &mission.name,
                );
                if !mission.text.is_empty() {
                    let description = commands
                        .spawn(source::text(
                            &mission.text,
                            source::font(skins.font.clone(), false),
                        ))
                        .id();
                    commands.entity(root).add_child(description);
                }
            }
        }
    }
}
fn activate(
    event: On<Activate>,
    actions: Query<&ButtonAction>,
    owner: NonSend<HostOwner>,
    guard: ModalGuard,
    mut windows: ResMut<Windows>,
) {
    let Ok(action) = actions.get(event.entity) else {
        return;
    };
    let Some(play) = &owner.play else { return };
    if !guard.allows(event.entity)
        || owner.modal_active()
        || action.generation != owner.generation()
        || !play.link.quest_tracker_visible()
        || !windows.slots[action.index]
    {
        return;
    }
    match action.kind {
        Control::Add => {
            if let Some(slot) = windows.slots.iter_mut().find(|slot| !**slot) {
                *slot = true;
            }
        }
        Control::Close => {
            windows.slots[action.index] = false;
            if !windows.slots.contains(&true) {
                play.link.quest_close_tracker();
            }
        }
        Control::Journal => {
            play.link.quest_set_open(true);
            play.link
                .quest_request(&oteryn_client_ui::quest_log::QuestLogQuery::ListOnly);
        }
        Control::Mission(q, m) => {
            play.link.quest_open_tracked_mission(q, m);
        }
    }
}
