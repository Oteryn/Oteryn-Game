use oteryn_gameplay_entities::carrier::Slot;
use oteryn_gameplay_entities::*;
pub trait WelterCarrierExt {
    #[allow(clippy::too_many_arguments)]
    fn bind_welter_pinned_definition(
        &self,
        boss_key: &str,
        boss_name: &str,
        max_health: u64,
        egg_key: &str,
        egg_name: &str,
        spawn_key: &str,
        spawn_name: &str,
    ) -> Result<WelterSourceRegistration, WelterConsumeError>;
    #[allow(clippy::too_many_arguments)]
    fn commit_welter_consume(
        &mut self,
        ledger: &mut WelterConsumeLedger,
        registration: &WelterSourceRegistration,
        caster: ExactActorRef,
        sequence: u64,
    ) -> Result<WelterConsumeResult, WelterConsumeError>;

    #[allow(clippy::too_many_arguments)]
    // Source-cue consumer reads the native committed table, never a caller-authored result.
    fn welter_committed_consume(
        &self,
        ledger: &WelterConsumeLedger,
        registration: &WelterSourceRegistration,
        caster: ExactActorRef,
        sequence: u64,
    ) -> Result<Option<(u64, WelterConsumeResult)>, WelterConsumeError>;
}
// Main reconciliation Foundation-3: source-qualified native boss helpers.
// Source-specific proposal appended inside foundation/runtime_actor_carrier.rs.
// Canary47dfd51f the_welter_heal.lua SHAead257f505dfcd8c6f2cd80bbaaff2185c0bc6e4e164e2d28d815ad328145cc6.
// No generic Encounter dispatcher. Owner lock/cast scheduling and emitted cues remain caller work.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WelterPrey {
    Egg,
    Spawn,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WelterConsumeResult {
    pub newly_committed: bool,
    pub consumed: Option<(ExactActorRef, WelterPrey)>,
    pub caster_position: MovementLocalPosition,
    pub prey_position: Option<MovementLocalPosition>,
    pub health_before: i64,
    pub health_after: i64,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WelterConsumeError {
    Carrier(CarrierError),
    InvalidSequence,
    SupersededOccurrence,
    WrongBossDefinition,
    InvalidBossHealth,
    MissingPosition,
    LedgerAllocation,
    RegistrationContentPinMismatch,
}
impl From<CarrierError> for WelterConsumeError {
    fn from(value: CarrierError) -> Self {
        Self::Carrier(value)
    }
}
#[derive(Debug, Clone, Copy)]
struct WelterConsumeEntry {
    caster: ExactActorRef,
    sequence: u64,
    result: WelterConsumeResult,
}
/// Beside ChannelRuntimeV1 under the same physical owner lock, exactly like CreatureBiteLedger.
/// At most one last occurrence per live caster, bounded by that carrier's actual slots.len().
#[derive(Debug, Default)]
pub struct WelterConsumeLedger {
    entries: Vec<WelterConsumeEntry>,
}
/// Constructed once by the native Content loader from current Creature profiles/bindings.
/// No token obtained from network data; private fields forbid caller struct construction.
#[derive(Debug, Clone, Copy)]
pub struct WelterSourceRegistration {
    world: WorldId,
    activation_sequence: u64,
    server_digest: [u8; 32],
    client_digest: [u8; 32],
    frame_digest: [u8; 32],
    map_digest: [u8; 32],
    max_health: i64,
}
impl WelterCarrierExt for ChannelRuntimeV1 {
    /// Content-loader seam: caller supplies values resolved from the three typed active profiles.
    /// The sourcebinding receipt is validated by that loader (Canary47dfd51f spell/monster SHA).
    /// Foundation accepts no generic profile graph and introduces no Content dependency.
    #[allow(clippy::too_many_arguments)]
    fn bind_welter_pinned_definition(
        &self,
        boss_key: &str,
        boss_name: &str,
        max_health: u64,
        egg_key: &str,
        egg_name: &str,
        spawn_key: &str,
        spawn_name: &str,
    ) -> Result<WelterSourceRegistration, WelterConsumeError> {
        if boss_key != "oteryn:creature.the_welter"
            || boss_name != "The Welter"
            || max_health != 25000
            || egg_key != "oteryn:creature.egg"
            || egg_name != "Egg"
            || spawn_key != "oteryn:creature.spawn_of_the_welter"
            || spawn_name != "Spawn of the Welter"
        {
            return Err(WelterConsumeError::WrongBossDefinition);
        }
        Ok(WelterSourceRegistration {
            world: self.content_pin().world_id(),
            activation_sequence: self.content_pin().activation_sequence(),
            server_digest: self.content_pin().server_artifact_digest(),
            client_digest: self.content_pin().client_artifact_digest(),
            frame_digest: self.content_pin().frame_binding_digest(),
            map_digest: self.content_pin().map_revision_digest(),
            max_health: max_health as i64,
        })
    }

    /// Native PROJECT approximation: stable carrier-slot spectator census order. Canary's
    /// supplied Game.getSpectators ordering is not specified by its spell. Explicit flag:
    /// PROJECT_NATIVE_SPECTATOR_ORDER_GLOBAL_UNVERIFIED. Not a claim of identical order.
    /// Exact source semantics otherwise: same floor ±10 square, first eligible live Monster,
    /// Egg/Spawn of the Welter, administrative removal (no death reward), heal25000 capped25000.
    fn commit_welter_consume(
        &mut self,
        ledger: &mut WelterConsumeLedger,
        registration: &WelterSourceRegistration,
        caster: ExactActorRef,
        sequence: u64,
    ) -> Result<WelterConsumeResult, WelterConsumeError> {
        commit_welter_consume_inner(self, ledger, registration, caster, sequence, false)
    }

    fn welter_committed_consume(
        &self,
        ledger: &WelterConsumeLedger,
        registration: &WelterSourceRegistration,
        caster: ExactActorRef,
        sequence: u64,
    ) -> Result<Option<(u64, WelterConsumeResult)>, WelterConsumeError> {
        if registration.world != self.content_pin().world_id()
            || registration.activation_sequence != self.content_pin().activation_sequence()
            || registration.server_digest != self.content_pin().server_artifact_digest()
            || registration.client_digest != self.content_pin().client_artifact_digest()
            || registration.frame_digest != self.content_pin().frame_binding_digest()
            || registration.map_digest != self.content_pin().map_revision_digest()
        {
            return Err(WelterConsumeError::RegistrationContentPinMismatch);
        }
        if !self.matches_live_creature_identity(caster, b"oteryn:creature.the_welter") {
            return Err(WelterConsumeError::Carrier(
                CarrierError::StaleActorGeneration,
            ));
        }
        Ok(ledger
            .entries
            .iter()
            .find(|e| e.caster == caster && e.sequence == sequence)
            .map(|e| (caster.0.actor_local_generation.0, e.result)))
    }
}

fn commit_welter_consume_inner(
    runtime: &mut ChannelRuntimeV1,
    ledger: &mut WelterConsumeLedger,
    registration: &WelterSourceRegistration,
    caster: ExactActorRef,
    sequence: u64,
    fail_before_write: bool,
) -> Result<WelterConsumeResult, WelterConsumeError> {
    if registration.world != runtime.content_pin().world_id()
        || registration.activation_sequence != runtime.content_pin().activation_sequence()
        || registration.server_digest != runtime.content_pin().server_artifact_digest()
        || registration.client_digest != runtime.content_pin().client_artifact_digest()
        || registration.frame_digest != runtime.content_pin().frame_binding_digest()
        || registration.map_digest != runtime.content_pin().map_revision_digest()
    {
        return Err(WelterConsumeError::RegistrationContentPinMismatch);
    }
    if sequence == 0 {
        return Err(WelterConsumeError::InvalidSequence);
    }
    let boss = runtime.encounter_actor_index(caster)?;
    runtime.assert_encounter_slot_unreserved(boss)?;
    if !matches!(runtime.encounter_slot(boss),Slot::CreatureOccupied{generation,..} if *generation==caster.0.actor_local_generation.0)
    {
        return Err(WelterConsumeError::Carrier(
            CarrierError::StaleActorGeneration,
        ));
    }
    let context = runtime.encounter_position_context();
    let (before, at) = match runtime.encounter_slot(boss) {
        Slot::CreatureOccupied {
            target_identity,
            health,
            position: Some(p),
            ..
        } if target_identity.as_ref() == b"oteryn:creature.the_welter"
            && *health > 0
            && p.context == context =>
        {
            (*health, p.position)
        }
        Slot::CreatureOccupied {
            target_identity, ..
        } if target_identity.as_ref() != b"oteryn:creature.the_welter" => {
            return Err(WelterConsumeError::WrongBossDefinition);
        }
        Slot::CreatureOccupied { health, .. } if *health <= 0 => {
            return Err(WelterConsumeError::InvalidBossHealth);
        }
        _ => return Err(WelterConsumeError::MissingPosition),
    };
    // Source max health25000 is pinned to selected Creature data at registration.
    if before > registration.max_health {
        return Err(WelterConsumeError::InvalidBossHealth);
    }
    ledger
        .entries
        .retain(|e| runtime.contains_live_creature(e.caster));
    if let Some(e) = ledger.entries.iter().find(|e| e.caster == caster) {
        if sequence < e.sequence {
            return Err(WelterConsumeError::SupersededOccurrence);
        }
        if sequence == e.sequence {
            return Ok(WelterConsumeResult {
                newly_committed: false,
                ..e.result
            });
        }
    }
    let selected = runtime
        .encounter_slots()
        .iter()
        .enumerate()
        .find_map(|(i, s)| {
            if i == boss {
                return None;
            }
            match s {
                Slot::CreatureOccupied {
                    generation,
                    target_identity,
                    health,
                    position: Some(p),
                    ..
                } if *health > 0
                    && p.context == context
                    && p.position.floor == at.floor
                    && i64::from(p.position.x).abs_diff(i64::from(at.x)) <= 10
                    && i64::from(p.position.y).abs_diff(i64::from(at.y)) <= 10 =>
                {
                    let prey = match target_identity.as_ref() {
                        b"oteryn:creature.egg" => WelterPrey::Egg,
                        b"oteryn:creature.spawn_of_the_welter" => WelterPrey::Spawn,
                        _ => return None,
                    };
                    Some((
                        ExactActorRef(ActorRef::from_owner_parts(
                            runtime.binding().world_id(),
                            runtime.binding().channel_id(),
                            runtime.binding().scope_generation(),
                            ActorLocalId((i + 1) as u32),
                            ActorLocalGeneration(*generation),
                        )),
                        prey,
                        p.position,
                    ))
                }
                _ => None,
            }
        });
    // All fallible allocation precedes either actor write. No fixed guessed ledger ceiling.
    if !ledger.entries.iter().any(|e| e.caster == caster) {
        if ledger.entries.len() >= runtime.encounter_slots().len() {
            return Err(WelterConsumeError::LedgerAllocation);
        }
        ledger
            .entries
            .try_reserve(1)
            .map_err(|_| WelterConsumeError::LedgerAllocation)?;
    }
    if fail_before_write {
        return Err(WelterConsumeError::Carrier(
            CarrierError::InjectedCommitFailure,
        ));
    }
    let after = if selected.is_some() {
        before.saturating_add(25000).min(registration.max_health)
    } else {
        before
    };
    if let Some((prey, _, _)) = selected {
        runtime.commit_encounter_consumption(prey, caster, after)?;
    }
    let result = WelterConsumeResult {
        newly_committed: true,
        consumed: selected.map(|(actor, prey, _)| (actor, prey)),
        caster_position: MovementLocalPosition {
            x: at.x,
            y: at.y,
            floor: at.floor,
        },
        prey_position: selected.map(|(_, _, p)| MovementLocalPosition {
            x: p.x,
            y: p.y,
            floor: p.floor,
        }),
        health_before: before,
        health_after: after,
    };
    if let Some(e) = ledger.entries.iter_mut().find(|e| e.caster == caster) {
        *e = WelterConsumeEntry {
            caster,
            sequence,
            result,
        }
    } else {
        ledger.entries.push(WelterConsumeEntry {
            caster,
            sequence,
            result,
        })
    }
    Ok(result)
}

#[cfg(test)]
#[path = "native_owner_tests.rs"]
mod tests;
