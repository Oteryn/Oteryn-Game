//! Task Board presentation over the retained session owner.
mod rows;
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
use rows::{button, completed, entries, entry, label, unopened};
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
    Talisman(model::talisman::Intent),
    Full(FullIntent),
    ConfirmAction,
}
struct Entry {
    text: String,
    action: Option<Kind>,
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
        // These intents are created only from the displayed, snapshot-bound choices above.
        Kind::Full(intent) => Some(intent.clone()),
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
    confirmation: Option<Kind>,
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
    frames: Query<(&ChildOf, &UiTransform), With<crate::bevy_shell::window_drag::DraggableDialog>>,
) {
    let panel = owner
        .play
        .as_ref()
        .map(|p| p.link.task_board_panel())
        .filter(|p| p.open);
    if state.seen == panel && state.generation == owner.generation() && !state.dirty {
        return;
    }
    let position = (state.generation == owner.generation())
        .then(|| {
            frames
                .iter()
                .find(|(parent, _)| roots.contains(parent.parent()))
                .map(|(_, transform)| *transform)
        })
        .flatten();
    for root in &roots {
        commands.entity(root).despawn();
    }
    if state.generation != owner.generation() || panel.is_none() {
        *state = State::default();
    }
    if state.seen != panel {
        state.confirmation = None;
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
            position.unwrap_or_default(),
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
        ))
        .id();
    commands.entity(root).add_child(frame);
    source::dialog_caption(&mut commands, frame, "Task Board", skins.font.clone());
    let mut rows = Vec::new();
    if let Some(kind) = &state.confirmation
        && let Some(text) = confirmation_text(panel, kind)
    {
        label(&mut rows, text);
        button(&mut rows, "Confirm", Kind::ConfirmAction);
        button(&mut rows, "Cancel", Kind::Cancel);
    }
    for (name, tab) in [
        ("Bounty Tasks", Tab::Bounty),
        ("Weekly", Tab::Weekly),
        ("Shop", Tab::Shop),
    ] {
        button(&mut rows, name, Kind::Tab(tab));
    }
    rows.extend(
        entries(panel, state.tab, state.confirm)
            .into_iter()
            .map(|mut row| {
                row.action = row
                    .action
                    .filter(|_| panel.selected && !panel.pending && state.confirmation.is_none());
                row
            }),
    );
    if let Some(feedback) = panel.feedback {
        label(&mut rows, feedback);
    }
    if panel.pending {
        label(&mut rows, "Waiting for the server");
    }
    entry(
        &mut rows,
        "Refresh",
        (!panel.pending && panel.selected).then_some(Kind::Refresh),
    );
    button(&mut rows, "Close", Kind::Close);
    for row in rows {
        widget(&mut commands, &skins, frame, &state, &row.text, row.action);
    }
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
    if play.view.gameplay_input_blocked(&play.link)
        || (state.confirmation.is_some()
            && !matches!(
                action.kind,
                Kind::ConfirmAction | Kind::Cancel | Kind::Close | Kind::Tab(_)
            ))
    {
        return;
    }
    match action.kind {
        Kind::Talisman(_) if panel.selected && !panel.pending => {
            state.confirmation = Some(action.kind.clone());
            state.dirty = true;
        }
        Kind::Buy(_) | Kind::Deliver(_) | Kind::Claim => {
            if intent(&panel, &mut state.confirm, &action.kind).is_some() {
                state.confirmation = Some(action.kind.clone());
                state.dirty = true;
            }
        }
        Kind::ConfirmAction => {
            if let Some(kind) = state.confirmation.take() {
                if let Kind::Talisman(intent) = kind {
                    play.link.task_board_talisman_request(intent);
                } else if let Some(intent) = intent(&panel, &mut state.confirm, &kind) {
                    play.link.task_board_full_request(intent);
                }
            }
            state.dirty = true;
        }
        Kind::Cancel => {
            state.confirmation = None;
            state.confirm = false;
            state.dirty = true;
        }
        Kind::Close => {
            play.link.task_board_set_open(false);
            state.confirm = false;
        }
        Kind::Tab(tab) => {
            state.confirmation = None;
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

fn confirmation_text(panel: &TaskBoardPanel, kind: &Kind) -> Option<String> {
    let reply = panel.weekly.as_ref()?;
    Some(match kind {
        Kind::Talisman(model::talisman::Intent::Buy) => {
            "Purchase Bounty Talisman for 5000 gold?".into()
        }
        Kind::Talisman(model::talisman::Intent::Upgrade(path)) => {
            let level = reply.talisman.get(usize::from(*path))?;
            let name = ["Damage", "Life leech", "Loot", "Double Bestiary chance"]
                .get(usize::from(*path))?;
            format!(
                "Upgrade {name} to level {} for {} bounty points?",
                level.level + 1,
                level.next_cost
            )
        }
        Kind::Buy(index) => format!(
            "Confirm purchase: {}",
            model::shop_offer_text(reply.shop.iter().find(|o| o.index == *index)?, reply.points)
        ),
        Kind::Deliver(slot) => {
            let task = reply.weekly.deliveries.get(usize::from(*slot))?;
            format!("Deliver {} of item {}?", task.required, task.item)
        }
        Kind::Claim => {
            let offer = reply.bounty.active.as_ref().or_else(|| {
                reply
                    .bounty
                    .offers
                    .get(usize::from(reply.bounty.selected.saturating_sub(1)))
            })?;
            format!(
                "Claim {} XP and {} bounty points?",
                offer.experience, offer.points
            )
        }
        _ => return None,
    })
}
