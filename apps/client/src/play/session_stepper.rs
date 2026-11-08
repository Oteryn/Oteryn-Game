//! The production `Stepper`: every session call the play task needs, served by `Session`.

use super::{Outbox, Stepper};
use crate::combat_input::{CombatCommand, CombatReport, run_combat};
use crate::spell::SpellFeedback;
use oteryn_session::{
    LogoutResult, Session, SessionError, SessionEvent, SessionStream, StepDirection, StepOutcome,
    WorldSpatialEntity,
};
use std::future::Future;
use std::time::Duration;

/// Hands the shell what the session pushed, and the visible entities when they changed.
pub(super) fn publish<T: Stepper>(
    session: &mut T,
    outbox: &Outbox,
    held: &mut Vec<WorldSpatialEntity>,
) {
    outbox.hotkey_equip_items(session.hotkey_equip_items());
    outbox.chat_private_publish(session);
    outbox.targeting_refresh(session);
    outbox.achievement_notices_publish(session);
    outbox.guild_chat_publish(session);
    outbox.depot_publish(session);
    outbox.stash_publish(session);
    if let Some(inventory) = session.inventory() {
        outbox.browse_field_inventory(inventory);
    }
    outbox.vitals(session.actor_vitals());
    outbox.appearances(session.visible_appearances());
    if let Some(payload) = session.feature_snapshot(98) {
        outbox.cyclopedia_tracker_push(payload);
    }
    if let Some(entities) = session.entities_changed(held) {
        held.clone_from(&entities);
        outbox.entities(entities);
    }
    outbox.battle_publish(session.battle_state(), session.battle_floor(), held);
    if let Some((rooms, lines)) = session.chat_state() {
        outbox.chat_public_state(rooms, lines);
    }
}

#[path = "session_adapter.rs"]
mod session_adapter;
