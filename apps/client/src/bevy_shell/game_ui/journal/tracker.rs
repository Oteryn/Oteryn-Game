use super::*;
use oteryn_client_ui::quest_log::QuestLine;

#[derive(Component)]
pub(super) struct Window;
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
    count: usize,
    painted: usize,
    lines: Vec<QuestLine>,
}
pub(super) fn install(app: &mut App) {
    app.init_resource::<Windows>().add_systems(Update, project);
}
fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skins: Res<Skins>,
    mut windows: ResMut<Windows>,
    roots: Query<Entity, With<Window>>,
) {
    let active = owner
        .play
        .as_ref()
        .filter(|p| p.link.quest_tracker_visible());
    let generation = active.map(|_| owner.generation());
    if generation != windows.generation {
        windows.count = usize::from(active.is_some());
        windows.generation = generation;
    }
    let lines = active
        .map(|p| p.link.quest_panel().state.log.tracked)
        .unwrap_or_default();
    if windows.painted == windows.count && lines == windows.lines {
        return;
    }
    for root in &roots {
        commands.entity(root).despawn();
    }
    windows.lines = lines;
    windows.painted = windows.count;
    let Some(generation) = generation else {
        return;
    };
    for index in 0..windows.count {
        let root = commands
            .spawn((
                Window,
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
                    padding: px(4.).all(),
                    ..default()
                },
                bevy::ui_widgets::ScrollArea,
            ))
            .id();
        source::dialog_caption(&mut commands, root, "Quest tracker", skins.font.clone());
        let mut button = |kind, label: &str| {
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
        button(Control::Journal, "Open quest journal");
        if windows.count < 8 {
            button(Control::Add, "New tracker window");
        }
        button(Control::Close, "Close tracker");
        for line in &windows.lines {
            for mission in &line.missions {
                button(
                    Control::Mission(line.quest_index, mission.mission_index),
                    &mission.name,
                );
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
        || action.index >= windows.count
    {
        return;
    }
    match action.kind {
        Control::Add => windows.count = (windows.count + 1).min(8),
        Control::Close => {
            windows.count -= 1;
            if windows.count == 0 {
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
