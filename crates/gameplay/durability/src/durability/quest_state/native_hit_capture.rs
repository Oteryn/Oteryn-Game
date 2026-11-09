//! Allocate quest metadata before damage; retain its first exact committed hit without allocation.
use super::native_hits::NativeQuestHit;
use crate::foundation::{CharacterLease, ExactActorRef, GameSessionId};
use oteryn_gameplay_entities::runtime_actor_carrier::CommittedNativeHit;
use oteryn_gameplay_quests::quest::{
    QuestStateCatalogue,
    authored_bindings::hits::{self, HitSource},
};
use std::collections::BTreeMap;

#[derive(Debug)]
struct PreparedFields {
    attacker: ExactActorRef,
    target: ExactActorRef,
    lease: CharacterLease,
    session: GameSessionId,
    transition: String,
    creature: String,
    item: String,
    spell: Option<String>,
    observed_revision: u64,
    definition_hash: [u8; 32],
}

/// One preallocated capture slot. Keep it queued until durable settlement succeeds;
/// retries borrow the retained hit rather than replacing its original mission snapshot.
#[derive(Debug)]
pub struct PreparedQuestHit {
    armed: Option<PreparedFields>,
    pending: Option<NativeQuestHit>,
}
impl PreparedQuestHit {
    #[allow(
        clippy::too_many_arguments,
        reason = "explicit immutable quest snapshot and exact native actor/lease fences"
    )]
    pub fn prepare(
        catalogue: &QuestStateCatalogue,
        transition: &str,
        tracks: &BTreeMap<String, i64>,
        observed_revision: u64,
        attacker: ExactActorRef,
        target: ExactActorRef,
        lease: CharacterLease,
        session: GameSessionId,
    ) -> Option<Self> {
        let (quest, objective) = hits::objective_for(catalogue, transition)?;
        if observed_revision == 0
            || !objective.active(catalogue, tracks)
            || attacker == target
            || attacker.runtime_scope() != target.runtime_scope()
            || attacker.scope_generation() != target.scope_generation()
        {
            return None;
        }
        let (item, spell) = match &objective.source {
            HitSource::Weapon { item } => (item.clone(), None),
            HitSource::Rune { item, spell } => (item.clone(), Some(spell.clone())),
        };
        Some(Self {
            armed: Some(PreparedFields {
                attacker,
                target,
                lease,
                session,
                transition: objective.key.clone(),
                creature: objective.creature.clone(),
                item,
                spell,
                observed_revision,
                definition_hash: catalogue.definition_hash(quest)?,
            }),
            pending: None,
        })
    }

    /// Called in the native commit turn. Wrong actors, leases or actual sources leave
    /// the slot armed; a second hit never overwrites the first captured occurrence.
    pub fn capture(
        &mut self,
        witness: CommittedNativeHit,
        creature: &str,
        item: &str,
        spell: Option<&str>,
    ) -> bool {
        let Some(fields) = self.armed.as_ref() else {
            return false;
        };
        let (character, generation, session, _, _) = witness.origin();
        if witness.attacker() != fields.attacker
            || witness.target() != fields.target
            || character != fields.lease.character_id()
            || generation != fields.lease.generation()
            || session != fields.session
            || creature != fields.creature
            || item != fields.item
            || spell != fields.spell.as_deref()
        {
            return false;
        }
        let Some(fields) = self.armed.take() else {
            return false;
        };
        self.pending = Some(NativeQuestHit::from_committed(
            fields.transition,
            fields.creature,
            fields.item,
            fields.spell,
            fields.observed_revision,
            fields.definition_hash,
            witness,
        ));
        true
    }
    pub fn pending(&self) -> Option<&NativeQuestHit> {
        self.pending.as_ref()
    }
}
