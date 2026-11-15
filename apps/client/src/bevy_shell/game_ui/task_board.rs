//! Task Board presentation over the retained session owner.
#[cfg(test)]
mod tests;
use super::{
    super::{owner::HostOwner, ui::Skins},
    GameModal,
    modal::ModalGuard,
};
use bevy::{prelude::*, ui_widgets::Activate};
use oteryn_client_ui::task_board::{self as model, FullIntent, TaskBoardPanel};
use oteryn_client_ui_bevy as source;
#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum Tab {
    #[default]
    Bounty,
    Weekly,
    Shop,
}
#[derive(Clone)]
enum Kind {
    Close,
    Tab(Tab),
    Refresh,
    OpenBounty,
    Select(u8),
    Claim,
    Difficulty(u8),
    Reroll,
    ConfirmReroll,
    Cancel,
    Deliver(u8),
    Start(u8),
    Buy(u16),
}
struct Entry {
    text: String,
    action: Option<Kind>,
}
fn entry(rows: &mut Vec<Entry>, text: impl Into<String>, action: Option<Kind>) {
    rows.push(Entry {
        text: text.into(),
        action,
    });
}
fn completed(panel: &TaskBoardPanel) -> bool {
    panel.weekly.as_ref().is_some_and(|r| {
        let b = &r.bounty;
        b.selected != 0
            && b.active
                .as_ref()
                .or_else(|| b.offers.get(usize::from(b.selected.saturating_sub(1))))
                .is_some_and(|a| b.current >= a.required)
    })
}
/// The server generates bounty offers only on OpenBounty (task_board_full.rs),
/// so a board with no offers and no active task needs it.
fn unopened(b: &model::task_board::full::Bounty) -> bool {
    b.selected == 0 && b.offers.is_empty() && b.active.is_none()
}
fn entries(panel: &TaskBoardPanel, tab: Tab, confirm: bool) -> Vec<Entry> {
    let mut rows = Vec::new();
    let Some(r) = &panel.weekly else {
        entry(&mut rows, "Refresh to load the Task Board", None);
        return rows;
    };
    match tab {
        Tab::Bounty => {
            let b = &r.bounty;
            entry(
                &mut rows,
                format!("Reroll Tokens: {} / {}", b.rerolls, model::REROLL_TOKEN_CAP),
                None,
            );
            entry(
                &mut rows,
                "Daily tokens: claim at the Daily Reward Wall",
                None,
            );
            if unopened(b) {
                entry(&mut rows, "Open bounty", Some(Kind::OpenBounty));
            }
            for (index, offer) in b.offers.iter().enumerate() {
                let active = usize::from(b.selected) == index + 1;
                let progress = if active { b.current } else { 0 };
                let state = if active && completed(panel) {
                    "Completed"
                } else if active {
                    "Active"
                } else {
                    "Available"
                };
                entry(
                    &mut rows,
                    format!(
                        "{}: {}/{} | {} | {}",
                        model::creature_name(&offer.race),
                        progress,
                        offer.required,
                        model::rarity_name(offer.grade),
                        state
                    ),
                    if b.selected == 0 {
                        u8::try_from(index).ok().map(Kind::Select)
                    } else {
                        None
                    },
                );
            }
            if let Some(active) = &b.active
                && b.offers.get(usize::from(b.selected.saturating_sub(1))) != Some(active)
            {
                entry(
                    &mut rows,
                    format!(
                        "Active: {}: {}/{} | {} | {}",
                        model::creature_name(&active.race),
                        b.current,
                        active.required,
                        model::rarity_name(active.grade),
                        if completed(panel) {
                            "Completed"
                        } else {
                            "Active"
                        }
                    ),
                    None,
                );
            }
            entry(
                &mut rows,
                "Claim reward",
                completed(panel).then_some(Kind::Claim),
            );
            entry(
                &mut rows,
                format!("Task Difficulty: {}", model::difficulty_name(b.difficulty)),
                None,
            );
            for (i, (name, _)) in model::DIFFICULTIES.iter().enumerate() {
                entry(&mut rows, *name, Some(Kind::Difficulty(i as u8)));
            }
            if confirm {
                entry(&mut rows, "Reroll these tasks for 1 token?", None);
                entry(
                    &mut rows,
                    "Confirm reroll (1 token)",
                    Some(Kind::ConfirmReroll),
                );
                entry(&mut rows, "Cancel", Some(Kind::Cancel));
            } else {
                entry(
                    &mut rows,
                    "Reroll (1 token)",
                    (b.rerolls > 0).then_some(Kind::Reroll),
                );
            }
        }
        Tab::Weekly => {
            let w = &r.weekly;
            entry(
                &mut rows,
                format!(
                    "Week {} | Difficulty {} | Multiplier {}",
                    r.period, w.difficulty, w.multiplier
                ),
                None,
            );
            entry(
                &mut rows,
                format!(
                    "Kill XP: {} | Delivery XP: {} | Pending points: {} | Soulseals: {}",
                    w.kill_experience, w.delivery_experience, w.pending_points, w.pending_soulseals
                ),
                None,
            );
            for task in &w.kills {
                entry(
                    &mut rows,
                    format!(
                        "{}: {}/{}",
                        if task.race.is_empty() {
                            "Any creature".into()
                        } else {
                            model::creature_name(&task.race)
                        },
                        task.current,
                        task.required
                    ),
                    None,
                );
            }
            for (i, task) in w.deliveries.iter().enumerate() {
                entry(
                    &mut rows,
                    format!(
                        "Item {}: {}/{}{}",
                        task.item,
                        task.current,
                        task.required,
                        if task.delivered { " (delivered)" } else { "" }
                    ),
                    (!task.delivered && task.current >= task.required)
                        .then_some(Kind::Deliver(i as u8)),
                );
            }
            if !w.started {
                for (i, (name, _)) in model::DIFFICULTIES.iter().enumerate() {
                    entry(
                        &mut rows,
                        format!("Start weekly: {name}"),
                        Some(Kind::Start(i as u8)),
                    );
                }
            }
            if let Some(results) = &r.weekly_results {
                entry(
                    &mut rows,
                    format!(
                        "Week {} rewards already paid: {} points, {} soulseals",
                        results.period, results.points, results.soulseals
                    ),
                    None,
                );
            }
        }
        Tab::Shop => {
            entry(
                &mut rows,
                format!(
                    "Points: {} | Bounty points: {} | Soulseals: {}",
                    r.points, r.bounty_points, r.soulseals
                ),
                None,
            );
            for offer in &r.shop {
                entry(
                    &mut rows,
                    model::shop_offer_text(offer, r.points),
                    (!offer.bought).then_some(Kind::Buy(offer.index)),
                );
            }
        }
    }
    rows
}
fn intent(panel: &TaskBoardPanel, confirm: &mut bool, kind: &Kind) -> Option<FullIntent> {
    if !panel.open || !panel.selected || panel.pending {
        return None;
    }
    let r = panel.weekly.as_ref();
    match kind {
        Kind::Refresh => Some(FullIntent::Refresh),
        Kind::OpenBounty if r.is_some_and(|r| unopened(&r.bounty)) => Some(FullIntent::OpenBounty),
        Kind::Select(slot)
            if r.is_some_and(|r| {
                r.bounty.selected == 0 && usize::from(*slot) < r.bounty.offers.len()
            }) =>
        {
            Some(FullIntent::SelectBounty(*slot))
        }
        Kind::Claim if completed(panel) => Some(FullIntent::ClaimBounty),
        Kind::Difficulty(value) if usize::from(*value) < model::DIFFICULTIES.len() => {
            Some(FullIntent::BountyDifficulty(*value))
        }
        Kind::Reroll if r.is_some_and(|r| r.bounty.rerolls > 0) => {
            *confirm = true;
            None
        }
        Kind::ConfirmReroll if *confirm && r.is_some_and(|r| r.bounty.rerolls > 0) => {
            *confirm = false;
            Some(FullIntent::Reroll)
        }
        Kind::Cancel => {
            *confirm = false;
            None
        }
        Kind::Deliver(slot)
            if r.is_some_and(|r| {
                r.weekly
                    .deliveries
                    .get(usize::from(*slot))
                    .is_some_and(|t| !t.delivered && t.current >= t.required)
            }) =>
        {
            Some(FullIntent::Deliver(*slot))
        }
        Kind::Start(value) if r.is_some_and(|r| !r.weekly.started) => {
            Some(FullIntent::StartWeekly(*value))
        }
        Kind::Buy(index)
            if r.is_some_and(|r| r.shop.iter().any(|o| o.index == *index && !o.bought)) =>
        {
            Some(FullIntent::BuyShop(*index))
        }
        _ => None,
    }
}
#[derive(Component)]
struct Root;
#[derive(Component)]
struct Action {
    generation: u64,
    revision: u64,
    kind: Kind,
}
#[derive(Resource, Default)]
struct State {
    tab: Tab,
    confirm: bool,
    seen: Option<TaskBoardPanel>,
    generation: u64,
    revision: u64,
    dirty: bool,
}
pub(super) fn install(app: &mut App) {
    app.init_resource::<State>()
        .add_systems(Update, (keyboard, project).chain());
}
fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skins: Res<Skins>,
    mut state: ResMut<State>,
    roots: Query<Entity, With<Root>>,
) {
    let panel = owner
        .play
        .as_ref()
        .map(|p| p.link.task_board_panel())
        .filter(|p| p.open);
    if state.seen == panel && state.generation == owner.generation() && !state.dirty {
        return;
    }
    for root in &roots {
        commands.entity(root).despawn();
    }
    if state.generation != owner.generation() || panel.is_none() {
        *state = State::default();
    }
    state.generation = owner.generation();
    state.revision = state.revision.wrapping_add(1);
    state.dirty = false;
    state.seen = panel;
    let Some(panel) = &state.seen else {
        return;
    };
    let root = commands
        .spawn((
            Root,
            GameModal,
            GlobalZIndex(159),
            Node {
                position_type: PositionType::Absolute,
                width: percent(100.),
                height: percent(100.),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0., 0., 0., 0.45)),
        ))
        .id();
    let frame = commands
        .spawn((
            crate::bevy_shell::window_drag::DraggableDialog,
            bevy::input_focus::tab_navigation::TabGroup {
                order: 3,
                modal: true,
            },
            source::frame(skins.frame.clone(), [4., 17., 4., 4.]),
            Node {
                width: px(650.),
                max_width: percent(96.),
                max_height: percent(92.),
                padding: UiRect {
                    top: px(25.),
                    bottom: px(10.),
                    left: px(10.),
                    right: px(10.),
                },
                flex_direction: FlexDirection::Column,
                row_gap: px(4.),
                overflow: Overflow::scroll_y(),
                ..default()
            },
            bevy::ui_widgets::ScrollArea,
            crate::bevy_shell::game_ui::install::RememberScroll("task_board", 0),
        ))
        .id();
    commands.entity(root).add_child(frame);
    source::dialog_caption(&mut commands, frame, "Task Board", skins.font.clone());
    for (name, tab) in [
        ("Bounty Tasks", Tab::Bounty),
        ("Weekly", Tab::Weekly),
        ("Shop", Tab::Shop),
    ] {
        widget(
            &mut commands,
            &skins,
            frame,
            &state,
            name,
            Some(Kind::Tab(tab)),
        );
    }
    for row in entries(panel, state.tab, state.confirm) {
        widget(
            &mut commands,
            &skins,
            frame,
            &state,
            &row.text,
            row.action.filter(|_| panel.selected && !panel.pending),
        );
    }
    if let Some(feedback) = panel.feedback {
        widget(&mut commands, &skins, frame, &state, feedback, None);
    }
    if panel.pending {
        widget(
            &mut commands,
            &skins,
            frame,
            &state,
            "Waiting for the server",
            None,
        );
    }
    widget(
        &mut commands,
        &skins,
        frame,
        &state,
        "Refresh",
        (!panel.pending && panel.selected).then_some(Kind::Refresh),
    );
    widget(
        &mut commands,
        &skins,
        frame,
        &state,
        "Close",
        Some(Kind::Close),
    );
}
fn widget(
    commands: &mut Commands,
    skins: &Skins,
    parent: Entity,
    state: &State,
    text: &str,
    kind: Option<Kind>,
) {
    let entity = if let Some(kind) = kind {
        let button = commands
            .spawn((
                source::button(skins.buttons.clone(), 580.),
                Node {
                    width: percent(100.),
                    min_height: px(24.),
                    height: Val::Auto,
                    flex_shrink: 0.,
                    padding: px(3.).all(),
                    ..default()
                },
                Action {
                    generation: state.generation,
                    revision: state.revision,
                    kind,
                },
            ))
            .observe(activate)
            .id();
        let caption = commands
            .spawn(source::button_caption(text, skins.font.clone()))
            .id();
        commands.entity(button).add_child(caption);
        button
    } else {
        commands
            .spawn(source::text(text, source::font(skins.font.clone(), false)))
            .id()
    };
    commands.entity(parent).add_child(entity);
}
fn activate(
    event: On<Activate>,
    actions: Query<&Action>,
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
    guard: ModalGuard,
) {
    if !guard.allows(event.entity) || owner.modal_active() {
        return;
    }
    let Ok(action) = actions.get(event.entity) else {
        return;
    };
    if action.generation != owner.generation() || action.revision != state.revision {
        return;
    }
    let Some(play) = &owner.play else {
        return;
    };
    let panel = play.link.task_board_panel();
    if !panel.open || state.seen.as_ref() != Some(&panel) {
        return;
    }
    match action.kind {
        Kind::Close => {
            play.link.task_board_set_open(false);
            state.confirm = false;
        }
        Kind::Tab(tab) => {
            state.tab = tab;
            state.confirm = false;
            state.dirty = true;
        }
        _ => {
            if let Some(intent) = intent(&panel, &mut state.confirm, &action.kind) {
                play.link.task_board_full_request(intent);
            }
            state.dirty = true;
        }
    }
}

fn keyboard(
    mut events: MessageReader<bevy::input::keyboard::KeyboardInput>,
    roots: Query<Entity, With<Root>>,
    owner: NonSend<HostOwner>,
    guard: ModalGuard,
) {
    for event in events.read() {
        if event.state == bevy::input::ButtonState::Pressed
            && !event.repeat
            && event.key_code == KeyCode::Escape
            && !owner.modal_active()
            && roots.iter().any(|root| guard.allows(root))
            && let Some(play) = &owner.play
        {
            play.link.task_board_set_open(false);
        }
    }
}
