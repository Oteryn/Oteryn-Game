//! PAR-M10: underwater ground drowns a character who steps onto it.
//!
//! Crystal `data/scripts/movements/drowning.lua` at
//! `201189f871d23597b82caf88fd611454a2ddefb8` (Canary
//! `47dfd51f45280a59a1d3e50ba7edd573d7234446` holds the same script) registers StepIn and
//! StepOut on twelve ground ids. StepIn adds a drown condition unless the head slot holds a
//! diving helmet; StepOut removes drown. The damage and the interval follow Global, not
//! Crystal: TibiaWiki `Helmet_of_the_Deep` revision 1148766 states "5 times 20 Drowning
//! damage kicking in after four seconds", and `Drowning` revision 854368 states 20 damage
//! every 4 seconds. The helmet protection is the equipment `SUPPRESS_DROWN` suppression,
//! which the condition store answers with `Immune`.
use crate::foundation::{ChannelRuntimeV1, ExactActorRef, GameSessionId, MovementLocalPosition};
use crate::gameplay_transport::actor_spell::ChannelSpellStates;
use crate::map::boot::BundleMap;
use crate::map::overlay::TilePos;
use crate::map::view::MapFacts;
use oteryn_gameplay_domain::domain::condition::{ConditionValues, ConflictKey, DotElement};
use oteryn_gameplay_foundation::foundation::condition::{
    ApplicationFacts, ConditionDefinition, ConditionProvenance, ConditionSourceKind,
};
use oteryn_simulation_determinism::{DecisionOccurrenceId, GameplayDecisionRoot};
use sha2::{Digest, Sha256};

/// The ground ids Crystal `drowning.lua` registers (5404-5409, 5743, 5764, 8755-8757, 9291).
pub(in crate::gameplay_transport) const UNDERWATER_GROUND_IDS: [u16; 12] = [
    5404, 5405, 5406, 5407, 5408, 5409, 5743, 5764, 8755, 8756, 8757, 9291,
];
/// The frozen source of the environment drown condition. No actor owns it.
pub(in crate::gameplay_transport) const DROWN_SOURCE: &str = "environment:drowning";
pub(in crate::gameplay_transport) const DROWN_DEFINITION_KEY: &str = "environment.drowning";
/// Global: 20 drowning damage per tick.
pub(in crate::gameplay_transport) const DROWN_DAMAGE_PER_TICK: u32 = 20;
/// Global: one tick every four seconds, the first four seconds after entry.
pub(in crate::gameplay_transport) const DROWN_INTERVAL_MS: u32 = 4_000;
/// Global: five ticks per entry.
pub(in crate::gameplay_transport) const DROWN_TICKS: u32 = 5;

/// The Global drown definition: five delayed ticks of 20 drowning damage, four seconds apart.
pub(in crate::gameplay_transport) fn drown_definition() -> Option<ConditionDefinition> {
    let total = DROWN_DAMAGE_PER_TICK.checked_mul(DROWN_TICKS)?;
    ConditionDefinition::new(
        DROWN_DEFINITION_KEY,
        1,
        ConditionValues::DamageOverTime {
            element: DotElement::Drown,
            total_min: total,
            total_max: total,
            per_tick: DROWN_DAMAGE_PER_TICK,
            interval_ms: DROWN_INTERVAL_MS,
            delayed: true,
        },
    )
}

/// The damage owners deal this provenance as environment damage: it has no creature or player
/// source, no PvP legality and no field lineage.
pub(in crate::gameplay_transport) fn is_environment_drown(
    provenance: &ConditionProvenance<String>,
) -> bool {
    provenance.source_kind == ConditionSourceKind::Field
        && provenance.source.as_deref() == Some(DROWN_SOURCE)
        && provenance.definition_key == DROWN_DEFINITION_KEY
        && provenance.lineage.is_none()
}

/// Whether the bundle ground of `position` is one of [`UNDERWATER_GROUND_IDS`].
pub(in crate::gameplay_transport) fn underwater(
    map: &BundleMap,
    position: MovementLocalPosition,
) -> bool {
    let Some(pos) = (|| {
        Some(TilePos {
            x: position.x.try_into().ok()?,
            y: position.y.try_into().ok()?,
            floor: i8::try_from(position.floor).ok()?.checked_neg()?,
        })
    })() else {
        return false;
    };
    map.overlay
        .base()
        .tile(pos.x, pos.y, pos.floor)
        .and_then(|tile| tile.ground())
        .and_then(|(ordinal, id)| map.facts.base_entry(pos, ordinal, id))
        .is_some_and(|entry| UNDERWATER_GROUND_IDS.contains(&entry.appearance_id))
}

impl ChannelSpellStates {
    /// One committed step: leaving underwater ground removes drown (Crystal StepOut), entering
    /// it applies a fresh environment drown (Crystal StepIn). A suppressed (protected) or dead
    /// character keeps its state. Returns whether the Player state changed.
    pub(in crate::gameplay_transport) fn stage_underwater_step(
        &mut self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        session: GameSessionId,
        now_us: u64,
        left: bool,
        entered: bool,
    ) -> bool {
        if !left && !entered {
            return false;
        }
        let Some(definition) = drown_definition() else {
            return false;
        };
        let content = runtime.content_pin().server_artifact_digest();
        let Some(state) = self.get_mut(runtime, actor, session) else {
            return false;
        };
        if state.vitals().health == 0 || !state.owned_conditions().accepts_time(now_us) {
            return false;
        }
        let mut next = state.clone();
        if left {
            next.conditions
                .dispel_key(ConflictKey::Element(DotElement::Drown));
        }
        if entered {
            let root = GameplayDecisionRoot::from_bytes(content);
            let digest = Sha256::new()
                .chain_update(b"oteryn:environment-drown:v1")
                .chain_update(actor.placement_identity())
                .chain_update(now_us.to_be_bytes())
                .finalize();
            let mut occurrence = [0; 16];
            occurrence.copy_from_slice(&digest[..16]);
            let facts = ApplicationFacts {
                now: now_us,
                base_speed: 1,
                mana_shield_capacity: 0,
                target_reentry_protected: false,
                source_reentry_protected: false,
                target_is_player: true,
                decision_root: &root,
                occurrence: DecisionOccurrenceId::from_bytes(occurrence),
            };
            // Immune (a diving helmet) and every other refusal leave the step's state as is.
            let _ = next.conditions.apply(
                &definition,
                Some(DROWN_SOURCE.to_owned()),
                ConditionSourceKind::Field,
                &[],
                &facts,
            );
        }
        if next == *state {
            return false;
        }
        let Some(revision) = state.revision().checked_add(1) else {
            return false;
        };
        next.source_set_batch_revision(revision);
        *state = next;
        true
    }
}

#[cfg(test)]
#[path = "underwater_drown_tests.rs"]
mod tests;
