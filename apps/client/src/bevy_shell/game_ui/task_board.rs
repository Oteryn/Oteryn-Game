//! Task Board presentation over the retained session owner.
mod extras;
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
    Talisman,
    Preferences,
    Soulseals,
}
#[derive(Clone)]
enum Kind {
    Close,
    Extra(extras::Action),
    ConfirmExtra(extras::Action),
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
    BuyTalisman,
    UpgradeTalisman(u8),
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
include!("task_board/entries.rs");
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
        Kind::BuyTalisman if r.is_some() => Some(FullIntent::BuyTalisman),
        Kind::UpgradeTalisman(path)
            if r.is_some_and(|r| {
                r.talisman
                    .get(usize::from(*path))
                    .is_some_and(|t| t.level < model::talisman::MAX_LEVEL)
            }) =>
        {
            Some(FullIntent::UpgradeTalisman(*path))
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
    extra: Option<extras::Action>,
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
    if state.seen != panel {
        state.extra = None;
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
        ("Bounty Talisman", Tab::Talisman),
        ("Preferred / unwanted", Tab::Preferences),
        ("Soulseals", Tab::Soulseals),
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
    if let Some(extra) = &state.extra {
        widget(
            &mut commands,
            &skins,
            frame,
            &state,
            &extras::summary(panel, extra),
            None,
        );
        widget(
            &mut commands,
            &skins,
            frame,
            &state,
            "Confirm",
            Some(Kind::ConfirmExtra(extra.clone())),
        );
        widget(
            &mut commands,
            &skins,
            frame,
            &state,
            "Cancel",
            Some(Kind::Cancel),
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
                Action {
                    generation: state.generation,
                    revision: state.revision,
                    kind,
                },
            ))
            .insert(Node {
                width: percent(100.),
                min_height: px(24.),
                height: Val::Auto,
                flex_shrink: 0.,
                padding: px(3.).all(),
                ..default()
            })
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
            if tab == Tab::Soulseals {
                play.link.soulseal_catalogue_request(0);
            }
            state.extra = None;
            state.tab = tab;
            state.confirm = false;
            state.dirty = true;
        }
        Kind::Extra(ref extra) => {
            state.extra = Some(extra.clone());
            state.dirty = true;
        }
        Kind::ConfirmExtra(ref extra) if state.extra.as_ref() == Some(extra) => {
            extras::send(&play.link, &panel, extra);
            state.extra = None;
            state.dirty = true;
        }
        Kind::Refresh if state.tab == Tab::Soulseals => play.link.soulseal_catalogue_request(0),
        _ => {
            state.extra = None;
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
