//! Imbuing shrine presentation over the retained model and existing commands.
#[cfg(test)]
mod tests;
mod widgets;
use super::{
    super::{
        owner::{HostOwner, Phase},
        ui::Skins,
    },
    modal::ModalGuard,
};
use crate::play::feature_0068_imbue::input::ImbueViewAction;
use bevy::{prelude::*, ui_widgets::Activate};
use oteryn_client_runtime::depot::ItemEntry;
use oteryn_client_ui::imbue::{ImbuePanel, Intent, ItemHandle, family_name, slot_label};

#[derive(Clone, Copy, Component)]
struct Control {
    generation: u64,
    serial: u64,
    kind: Kind,
}
#[derive(Clone, Copy)]
enum Kind {
    Close,
    Refresh,
    Item(ItemHandle),
    Slot(u8),
    BaseType(u8),
    Offer(usize),
    Apply,
    Clear,
    Confirm,
    Cancel,
}
#[derive(Default)]
struct Selection {
    slot: Option<u8>,
    offer: Option<usize>,
    base_type: Option<u8>,
    confirmation: Option<ImbuePanel>,
}
impl Selection {
    fn can_apply(&self, panel: &ImbuePanel) -> bool {
        self.slot == Some(panel.slot) && self.offer == Some(panel.offer) && panel.can_apply()
    }
    fn intent(&mut self, panel: &ImbuePanel, kind: Kind) -> Option<Intent> {
        match kind {
            Kind::Apply if self.can_apply(panel) => Some(Intent::Apply),
            Kind::Clear if panel.can_clear() => {
                self.confirmation = Some(panel.clone());
                None
            }
            Kind::Confirm => self
                .confirmation
                .take()
                .filter(|expected| expected == panel && panel.can_clear())
                .map(|_| Intent::Clear),
            _ => {
                self.confirmation = None;
                None
            }
        }
    }
}
#[derive(Resource, Default)]
struct State {
    generation: u64,
    serial: u64,
    panel: Option<ImbuePanel>,
    items: Vec<ItemEntry>,
    selection: Selection,
    root: Option<Entity>,
    dirty: bool,
}
pub(super) fn install(app: &mut App) {
    app.init_resource::<State>()
        .add_systems(Update, (keyboard, project).chain());
}
fn keyboard(
    keys: Option<Res<ButtonInput<KeyCode>>>,
    guard: ModalGuard,
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
) {
    if !keys.is_some_and(|keys| keys.just_pressed(KeyCode::Escape))
        || owner.modal_active()
        || state.generation != owner.generation()
        || !state.root.is_some_and(|root| guard.allows(root))
    {
        return;
    }
    if let Some(play) = &owner.play {
        state.selection.confirmation = None;
        state.dirty = true;
        play.link
            .imbue_dispatch_bound(&play.link.imbue_panel(), ImbueViewAction::Close);
    }
}
fn slot_rows(panel: &ImbuePanel) -> Vec<String> {
    panel.window.as_ref().map_or_else(Vec::new, |window| {
        (0..window.slot_count)
            .map(|slot| slot_label(panel, slot))
            .collect()
    })
}
fn offer_rows(panel: &ImbuePanel, base_type: Option<u8>) -> Vec<(usize, String)> {
    let Some(window) = &panel.window else {
        return Vec::new();
    };
    window
        .offers
        .iter()
        .enumerate()
        .filter(|(_, offer)| base_type.is_none_or(|tier| offer.tier == tier))
        .map(|(index, offer)| {
            let mut label = format!(
                "{} — tier {} — {} gold",
                family_name(offer.family),
                offer.tier,
                offer.gold
            );
            let chance = oteryn_client_ui::imbuement_success_chance::success_percent(panel, index)
                .or_else(|| {
                    panel
                        .details
                        .as_ref()
                        .filter(|details| Some(details.item) == panel.item)
                        .and_then(|details| {
                            details.quotes.iter().find(|quote| {
                                quote.family == offer.family && quote.tier == offer.tier
                            })
                        })
                        .map(|quote| quote.success_percent)
                });
            if let Some(chance) = chance {
                label.push_str(&format!(" — {chance}% success"));
            }
            if offer.requires_premium {
                label.push_str(" (premium required)");
            }
            (index, label)
        })
        .collect()
}
fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skins: Res<Skins>,
    mut state: ResMut<State>,
) {
    let play = owner
        .play
        .as_ref()
        .filter(|_| owner.phase == Phase::Playing);
    let panel = play
        .map(|play| play.link.imbue_panel())
        .filter(|panel| panel.open);
    let items = play
        .filter(|_| panel.is_some())
        .map_or_else(Vec::new, |play| play.link.imbue_carried_items());
    if let (Some(play), Some(panel)) = (play, &panel)
        && panel.window.is_some()
        && panel.details_selected
        && !panel.details_attempted
    {
        play.link.imbue_details_request();
    }
    if state.generation == owner.generation()
        && state.panel == panel
        && state.items == items
        && !state.dirty
    {
        return;
    }
    if state.generation != owner.generation()
        || state.panel.as_ref().map(|p| (&p.item, &p.window))
            != panel.as_ref().map(|p| (&p.item, &p.window))
    {
        state.selection = Selection::default();
    }
    if state
        .selection
        .confirmation
        .as_ref()
        .is_some_and(|expected| Some(expected) != panel.as_ref())
    {
        state.selection.confirmation = None;
    }
    if let Some(root) = state.root.take() {
        commands.entity(root).despawn();
    }
    state.generation = owner.generation();
    state.serial = state.serial.saturating_add(1);
    state.panel = panel;
    state.items = items;
    state.dirty = false;
    if let Some(panel) = &state.panel {
        state.root = Some(widgets::build(
            &mut commands,
            &skins,
            state.generation,
            state.serial,
            panel,
            &state.items,
            &state.selection,
        ));
    }
}
fn activate(
    event: On<Activate>,
    controls: Query<&Control>,
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
    guard: ModalGuard,
) {
    if !guard.allows(event.entity) {
        return;
    }
    if let Ok(control) = controls.get(event.entity) {
        perform(&owner, &mut state, control);
    }
}
fn perform(owner: &HostOwner, state: &mut State, control: &Control) {
    if control.generation != owner.generation()
        || control.serial != state.serial
        || owner.modal_active()
        || owner.phase != Phase::Playing
    {
        return;
    }
    let (Some(play), Some(panel)) = (&owner.play, &state.panel) else {
        return;
    };
    if play.link.imbue_panel() != *panel {
        return;
    }
    if !matches!(control.kind, Kind::Close) && play.view.gameplay_input_blocked(&play.link) {
        return;
    }
    let action = match control.kind {
        Kind::Close => ImbueViewAction::Close,
        Kind::Refresh => ImbueViewAction::Request(Intent::Refresh),
        Kind::Item(item) => ImbueViewAction::Item(item),
        Kind::Slot(slot) => {
            state.selection.confirmation = None;
            state.selection.slot = Some(slot);
            ImbueViewAction::Slot(slot)
        }
        Kind::Offer(offer) => {
            state.selection.confirmation = None;
            state.selection.offer = Some(offer);
            ImbueViewAction::Offer(offer)
        }
        Kind::BaseType(tier) => {
            state.selection.base_type = Some(tier);
            state.selection.offer = None;
            state.selection.confirmation = None;
            state.dirty = true;
            return;
        }
        kind => {
            let intent = state.selection.intent(panel, kind);
            state.dirty = true;
            let Some(intent) = intent else {
                return;
            };
            ImbueViewAction::Request(intent)
        }
    };
    state.dirty = true;
    play.link.imbue_dispatch_bound(panel, action);
}
