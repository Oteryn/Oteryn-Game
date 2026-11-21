//! Bestiary catalogue and discovery details over the capability-67 owner.
#[cfg(test)]
mod tests;
mod widgets;
use super::{
    super::{
        owner::{HostOwner, Phase},
        ui::Skins,
    },
    GameModal,
    modal::ModalGuard,
};
use bevy::{input_focus::InputFocus, prelude::*, ui_widgets::Activate};
use oteryn_client_ui::charm_progression::{Intent, Panel};

#[derive(Component, Clone, Copy, PartialEq, Eq)]
struct Control {
    generation: u64,
    serial: u64,
    kind: Kind,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Catalogue(usize),
    Select(u16),
    Progress,
    /// Choose the charm (by its 1-based index) the charm actions apply to.
    Charm(u32),
    /// Unlock, assign to the selected creature, remove, or reset all charms.
    CharmAction(Intent),
    /// Pay the shown gold fee and send the pending Remove or ResetAll.
    Confirm(Intent),
    /// Drop the pending Remove or ResetAll without sending it.
    Cancel,
    Close,
}
/// A Remove or ResetAll awaiting the player's confirmation, with the fee shown for it
/// (`None` while the character level is not yet reported).
type Pending = (Intent, Option<u64>);
#[derive(Resource, Default)]
struct State {
    generation: u64,
    serial: u64,
    snapshot: Option<Panel>,
    root: Option<Entity>,
    load_catalogue: bool,
    /// Every charm catalogue page is still to be loaded for this opening.
    load_charms: bool,
    /// The `after` cursor of the last charm catalogue page requested for this opening.
    charm_cursor: Option<u16>,
    confirm: Option<Pending>,
    shown: Option<Pending>,
}
/// The gold the server charges for a Remove or ResetAll: removal is the level times 100 gold
/// (75 with the Charm Expansion, `charm_state::removal_fee_gold`); a reset is 100,000 gold plus
/// 11,000 per level above 100, a quarter less with the Expansion (`charm::reset_all_fee`).
/// The reply reports the Expansion as no assignment slot limit.
fn fee(intent: Intent, level: u32, expansion: bool) -> Option<u64> {
    let level = u64::from(level);
    match intent {
        Intent::Remove => level.checked_mul(if expansion { 75 } else { 100 }),
        Intent::ResetAll => {
            let fee = level
                .saturating_sub(100)
                .checked_mul(11_000)?
                .checked_add(100_000)?;
            if expansion {
                fee.checked_mul(75).map(|n| n / 100)
            } else {
                Some(fee)
            }
        }
        Intent::Query | Intent::Unlock | Intent::Assign => None,
    }
}
fn current_fee(
    play: &super::super::owner::ActivePlay,
    panel: &Panel,
    intent: Intent,
) -> Option<u64> {
    let expansion = panel.reply.as_ref()?.charms.assignment_slot_limit.is_none();
    let level = play.link.skill_progress_panel().complete?.level;
    fee(intent, level, expansion)
}
pub(super) fn install(app: &mut App) {
    app.init_resource::<State>()
        .add_message::<bevy::input::keyboard::KeyboardInput>()
        .add_systems(Update, (keyboard, project).chain());
}
fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skins: Res<Skins>,
    mut state: ResMut<State>,
    mut focus: ResMut<InputFocus>,
    parents: Query<&ChildOf>,
) {
    let panel = owner
        .play
        .as_ref()
        .filter(|_| owner.phase == Phase::Playing)
        .map(|play| play.link.charm_progression_panel())
        .filter(|panel| panel.open);
    let changed_session = state.generation != owner.generation();
    if changed_session || panel.is_none() {
        if let Some(root) = state.root.take() {
            clear_focus(&mut focus, root, &parents);
            commands.entity(root).despawn();
        }
        state.snapshot = None;
        state.load_catalogue = false;
        state.load_charms = false;
        state.charm_cursor = None;
        state.confirm = None;
        state.shown = None;
        state.generation = owner.generation();
    }
    let Some(mut panel) = panel else {
        return;
    };
    let Some(play) = &owner.play else {
        return;
    };
    if state.root.is_none() && state.snapshot.is_none() {
        state.load_catalogue = true;
        state.load_charms = true;
        state.charm_cursor = None;
        if panel.selected && !panel.pending {
            play.link.charm_progression_request(Intent::Query);
        }
        panel = play.link.charm_progression_panel();
    }
    if state.load_catalogue && panel.selected && !panel.pending {
        play.link.bestiary_catalogue_request(0);
        state.load_catalogue = false;
        panel = play.link.charm_progression_panel();
    }
    // Charm names and effect descriptions: every catalogue page, one at a time, after the
    // creature catalogue. A page the server refuses leaves the cursor unchanged and stops.
    if state.load_charms && !state.load_catalogue && panel.selected && !panel.pending {
        let browser = &panel.charm_browser;
        let request = match state.charm_cursor {
            None => Some((false, 0)),
            Some(last) => browser
                .next
                .filter(|next| browser.loaded && *next != last)
                .map(|next| (true, next)),
        };
        if let Some((next, after)) = request {
            play.link.charm_catalogue_request(next);
            state.charm_cursor = Some(after);
            panel = play.link.charm_progression_panel();
        } else {
            state.load_charms = false;
        }
    }
    if state.snapshot.as_ref() != Some(&panel) {
        state.confirm = None;
    } else if let Some((intent, shown)) = state.confirm {
        let fee = current_fee(play, &panel, intent);
        if fee != shown {
            state.confirm = Some((intent, fee));
        }
    }
    if state.snapshot.as_ref() == Some(&panel) && state.shown == state.confirm {
        return;
    }
    if let Some(root) = state.root.take() {
        clear_focus(&mut focus, root, &parents);
        commands.entity(root).despawn();
    }
    state.serial = state.serial.saturating_add(1);
    let control = Control {
        generation: state.generation,
        serial: state.serial,
        kind: Kind::Close,
    };
    state.root = Some(widgets::build(
        &mut commands,
        &skins,
        control,
        &panel,
        state.confirm,
    ));
    state.snapshot = Some(panel);
    state.shown = state.confirm;
}
fn perform(owner: &HostOwner, state: &mut State, control: Control) {
    if owner.phase != Phase::Playing
        || owner.modal_active()
        || owner.generation() != control.generation
        || state.serial != control.serial
    {
        return;
    }
    let Some(play) = &owner.play else {
        return;
    };
    let panel = play.link.charm_progression_panel();
    if !panel.open || state.snapshot.as_ref() != Some(&panel) {
        return;
    }
    if control.kind == Kind::Close {
        play.link.bestiary_close();
        return;
    }
    if panel.pending || !panel.selected || play.view.gameplay_input_blocked(&play.link) {
        return;
    }
    match control.kind {
        Kind::Catalogue(action) => play.link.bestiary_catalogue_request(action),
        Kind::Progress => play.link.charm_progression_request(Intent::Query),
        Kind::Select(race)
            if panel
                .catalogue
                .page
                .cards
                .iter()
                .any(|card| card.race == race) =>
        {
            play.link.charm_progression_edit(1, race.to_string());
            play.link.bestiary_catalogue_request(2);
        }
        Kind::Charm(charm)
            if panel.reply.as_ref().is_some_and(|reply| {
                reply.charms.charms.iter().any(|c| c.charm.get() == charm)
            }) =>
        {
            play.link.charm_progression_edit(0, charm.to_string());
        }
        Kind::CharmAction(intent @ (Intent::Remove | Intent::ResetAll)) => {
            let fee = current_fee(play, &panel, intent);
            if fee.is_none() {
                // The fee follows the character level; ask for it and show it once reported.
                play.link.skill_progress_request();
            }
            state.confirm = Some((intent, fee));
        }
        Kind::CharmAction(intent @ (Intent::Unlock | Intent::Assign)) => {
            play.link.charm_progression_request(intent);
        }
        Kind::Confirm(intent)
            if state.shown == state.confirm
                && state.confirm.is_some_and(|(pending, shown)| {
                    pending == intent
                        && shown.is_some()
                        && shown == current_fee(play, &panel, intent)
                }) =>
        {
            state.confirm = None;
            play.link.charm_progression_request(intent);
        }
        Kind::Cancel => state.confirm = None,
        Kind::Select(_)
        | Kind::Charm(_)
        | Kind::CharmAction(Intent::Query)
        | Kind::Confirm(_)
        | Kind::Close => {}
    }
}
fn activate(
    event: On<Activate>,
    controls: Query<&Control>,
    guard: ModalGuard,
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
) {
    if guard.allows(event.entity)
        && let Ok(control) = controls.get(event.entity)
    {
        perform(&owner, &mut state, *control);
    }
}
fn keyboard(
    mut events: MessageReader<bevy::input::keyboard::KeyboardInput>,
    guard: ModalGuard,
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
) {
    for event in events.read() {
        if event.key_code == KeyCode::Escape
            && event.state == bevy::input::ButtonState::Pressed
            && !event.repeat
            && state.root.is_some_and(|root| guard.allows(root))
        {
            let control = Control {
                generation: state.generation,
                serial: state.serial,
                kind: Kind::Close,
            };
            perform(&owner, &mut state, control);
        }
    }
}

fn clear_focus(focus: &mut InputFocus, root: Entity, parents: &Query<&ChildOf>) {
    let mut current = focus.get();
    while let Some(entity) = current {
        if entity == root {
            focus.clear();
            return;
        }
        current = parents.get(entity).ok().map(ChildOf::parent);
    }
}
