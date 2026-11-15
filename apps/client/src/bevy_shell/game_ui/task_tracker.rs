//! Nonmodal task progress beside the map; the session remains the sole data owner.
use super::{
    super::{owner::HostOwner, ui::Skins},
    GameRegions,
    modal::ModalGuard,
};
use bevy::{
    prelude::*,
    ui_widgets::{Activate, Button},
};
use oteryn_client_ui::{prey, task_board};
use oteryn_client_ui_bevy as source;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
#[cfg(test)]
mod tests;
const REFRESH: Duration = Duration::from_secs(5);
#[derive(Component)]
struct Tracker(u64);
#[derive(Resource, Default)]
struct State {
    generation: Option<u64>,
    next_refresh: Duration,
    root: Option<Entity>,
    text: String,
}
pub(super) fn install(app: &mut App) {
    app.init_resource::<State>().add_systems(Update, project);
}
fn duration(seconds: u64) -> String {
    format!(
        "{:02}:{:02}:{:02}",
        seconds / 3600,
        seconds / 60 % 60,
        seconds % 60
    )
}
fn progress(name: &str, current: u32, required: u32) -> String {
    format!(
        "{name}: {current}/{required}{}",
        if current >= required {
            " (complete)"
        } else {
            ""
        }
    )
}
fn content(board: &task_board::TaskBoardPanel, prey: &prey::PreyPanel, now: u64) -> String {
    let mut rows = vec!["Task tracker".to_owned()];
    if !prey.selected {
        rows.push("Prey unavailable in this session".into());
    } else if let Some(reply) = &prey.confirmed {
        let mut active = false;
        for slot in &reply.slots {
            if slot.state != 2 || slot.selected.is_empty() {
                continue;
            }
            active = true;
            rows.push(format!("Prey · {}", prey::slot_text(slot)));
            rows.push(format!(
                "Hunting time: {}",
                duration(u64::from(slot.hunting_seconds))
            ));
            if slot.free_reroll_at > now {
                rows.push(format!(
                    "Free reroll in {}",
                    duration(slot.free_reroll_at - now)
                ));
            }
        }
        if !active {
            rows.push("No active Prey".into());
        }
    } else {
        rows.push("Loading Prey…".into());
    }
    if !board.selected {
        rows.push("Task Board unavailable in this session".into());
    } else if let Some(reply) = &board.weekly {
        let bounty = &reply.bounty;
        if bounty.selected != 0 {
            if let Some(offer) = bounty
                .active
                .as_ref()
                .or_else(|| bounty.offers.get(usize::from(bounty.selected - 1)))
            {
                rows.push(format!(
                    "Bounty · {}",
                    progress(
                        &task_board::creature_name(&offer.race),
                        bounty.current,
                        offer.required
                    )
                ));
                rows.push(format!(
                    "{} · {} XP · {} points · {} dust",
                    task_board::rarity_name(offer.grade),
                    offer.experience,
                    offer.points,
                    offer.dust
                ));
            }
        } else {
            rows.push("No active Bounty".into());
        }
        let weekly = &reply.weekly;
        if weekly.started {
            rows.push(format!(
                "Weekly · week {} · x{}",
                reply.period, weekly.multiplier
            ));
            for task in &weekly.kills {
                let name = if task.race.is_empty() {
                    "Any creature".into()
                } else {
                    task_board::creature_name(&task.race)
                };
                rows.push(progress(&name, task.current, task.required));
            }
            for task in &weekly.deliveries {
                rows.push(format!(
                    "{}{}",
                    progress(&format!("Item {}", task.item), task.current, task.required),
                    if task.delivered { " (delivered)" } else { "" }
                ));
            }
            rows.push(format!(
                "{} points · {} soulseals",
                weekly.pending_points, weekly.pending_soulseals
            ));
            rows.push(format!(
                "Kill XP {} · Delivery XP {}",
                weekly.kill_experience, weekly.delivery_experience
            ));
        } else {
            rows.push("No active Weekly Tasks".into());
        }
    } else {
        rows.push("Loading tasks…".into());
    }
    if let Some(feedback) = board.feedback {
        rows.push(feedback.into());
    }
    if let Some(feedback) = prey.feedback {
        rows.push(feedback.into());
    }
    rows.push("Click to open Task Board".into());
    rows.join("\n")
}
fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    regions: Res<GameRegions>,
    skins: Res<Skins>,
    time: Res<Time>,
    mut state: ResMut<State>,
) {
    let generation = owner.play.as_ref().map(|_| owner.generation());
    if generation != state.generation {
        if let Some(root) = state.root.take() {
            commands.entity(root).despawn();
        }
        *state = State {
            generation,
            ..default()
        };
    }
    let Some(play) = &owner.play else {
        return;
    };
    let board = play.link.task_board_panel();
    let prey = play.link.prey_panel();
    if time.elapsed() >= state.next_refresh {
        state.next_refresh = time.elapsed().saturating_add(REFRESH);
        // Read-only queries use the existing capability and pending guards. Do not
        // interrupt selections or confirmation flows in either full editor.
        if !board.open {
            play.link
                .task_board_full_request(task_board::FullIntent::Refresh);
        }
        if !prey.open {
            play.link.prey_request(prey::Intent::Refresh);
        }
    }
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let text = content(&board, &prey, now);
    if state.root.is_some() && state.text == text {
        return;
    }
    if let Some(root) = state.root.take() {
        commands.entity(root).despawn();
    }
    let root = commands
        .spawn((
            Tracker(owner.generation()),
            Button,
            source::frame(skins.frame.clone(), [4., 17., 4., 4.]),
            Node {
                width: percent(100.),
                flex_shrink: 0.,
                padding: UiRect::all(px(4.)),
                flex_direction: FlexDirection::Column,
                ..default()
            },
        ))
        .observe(open_board)
        .id();
    let label = commands
        .spawn(source::text(&text, source::font(skins.font.clone(), false)))
        .insert((
            TextLayout::default(),
            Node {
                width: percent(100.),
                min_width: px(0.),
                ..default()
            },
        ))
        .id();
    commands.entity(root).add_child(label);
    commands.entity(regions.minimap).add_child(root);
    state.root = Some(root);
    state.text = text;
}
fn open_board(
    event: On<Activate>,
    nodes: Query<&Tracker>,
    owner: NonSend<HostOwner>,
    guard: ModalGuard,
) {
    if owner.modal_active() || !guard.allows(event.entity) {
        return;
    }
    let Ok(tracker) = nodes.get(event.entity) else {
        return;
    };
    if tracker.0 != owner.generation() {
        return;
    }
    if let Some(play) = &owner.play {
        play.link.open_task_board();
    }
}
