//! Crystal201189f data-global/scripts/systems/hazard_primal.lua:23-81:
//! physical, timestamped Primal pods.
use super::ComposedFreshAdmission;
use crate::content::native_cell_lookup::{BundleCollisionIndex, NativeMovementCollisionIndex};
use crate::foundation::owner_timer::SemanticTimeMicros;
use crate::foundation::{
    ChannelRuntimeV1, ExactActorRef, GameSessionId, MovementLocalPosition, RuntimeScopeRefV1,
};
use crate::map::boot::BundleCollisionIndexExt;
use crate::map::overlay::TilePos;
use crate::map::view::{EntrySource, MapFacts};
use std::sync::{Arc, Mutex};

// Crystal config.lua.dist:247,255-259, exact201189f source defaults.
const DAMAGE_TIME_MS: u64 = 2_000;
const SPAWN_TIME_MS: u64 = 4_000;
const DAMAGE_PERCENT: u64 = 5;
const POD: u16 = 39176; // data/items/items.xml:71987 (00ce02a5 pinned donor).
// Crystal src/utils/utils_definitions.hpp:56,66-67.
const DRAW_BLOOD: u16 = 1;
const TELEPORT: u16 = 11;
const ENERGY_HIT: u16 = 12;

#[derive(Clone, Copy, PartialEq, Eq)]
struct Pin {
    scope: RuntimeScopeRefV1,
    generation: u64,
    content: [u8; 32],
    frame: [u8; 32],
    map: [u8; 32],
}
impl Pin {
    fn current(runtime: &ChannelRuntimeV1) -> Option<Self> {
        runtime.owner_fence().ok()?;
        Some(Self {
            scope: runtime.binding().runtime_scope(),
            generation: runtime.binding().scope_generation().get(),
            content: runtime.content_pin().server_artifact_digest(),
            frame: runtime.content_pin().frame_binding_digest(),
            map: runtime.content_pin().map_revision_digest(),
        })
    }
}

enum Projection {
    Bundle(BundleCollisionIndex),
    Source(Arc<super::source_volatile_items::SourceVolatileProjection>),
}
fn tile(position: MovementLocalPosition) -> Option<TilePos> {
    Some(TilePos {
        x: position.x.try_into().ok()?,
        y: position.y.try_into().ok()?,
        floor: i8::try_from(position.floor.checked_neg()?).ok()?,
    })
}
impl Projection {
    fn current(
        owner: &ComposedFreshAdmission<'_, '_, '_>,
        runtime: &ChannelRuntimeV1,
    ) -> Option<Self> {
        match owner.movement_cells.index() {
            NativeMovementCollisionIndex::Bundle(index)
                if index.map().overlay.runtime_scope() == runtime.binding().runtime_scope() =>
            {
                Some(Self::Bundle(index.clone()))
            }
            NativeMovementCollisionIndex::Source(_) => {
                owner.source_volatile_projection(runtime).map(Self::Source)
            }
            _ => None,
        }
    }
    fn create(
        &self,
        owner: &ComposedFreshAdmission<'_, '_, '_>,
        runtime: &ChannelRuntimeV1,
        position: MovementLocalPosition,
    ) -> Option<u64> {
        match self {
            Self::Source(projection) => projection.create_pod(owner, runtime, position),
            Self::Bundle(index) => {
                // Reuse only an admitted Item policy and this actual map's facts.
                let policy = owner
                    .active_generation?
                    .native_gameplay()?
                    .item_policy("oteryn:item.tibia.i39176", "definition-r1")?;
                if !policy.record().admission.materializable {
                    return None;
                }
                let position = tile(position)?;
                let facts = &index.map().facts;
                if !facts.create_movement_item(position, POD, 0, || true) {
                    return None;
                }
                facts
                    .base_only()
                    .movement_items(position)
                    .iter()
                    .filter_map(|entry| match entry.source {
                        EntrySource::Added(id) if entry.facts.appearance_id == POD => {
                            Some(id.get())
                        }
                        _ => None,
                    })
                    .max()
            }
        }
    }
    fn remove(&self, position: MovementLocalPosition, key: u64) -> bool {
        match self {
            Self::Source(projection) => projection.remove_pod(position, key),
            Self::Bundle(index) => tile(position)
                .is_some_and(|position| index.map().facts.remove_movement_item(position, key, POD)),
        }
    }
    fn effect(
        &self,
        owner: &ComposedFreshAdmission<'_, '_, '_>,
        runtime: &ChannelRuntimeV1,
        position: MovementLocalPosition,
        effect: u16,
    ) -> bool {
        match self {
            Self::Source(projection) => projection.emit_effect(owner, runtime, position, effect),
            Self::Bundle(index) => tile(position)
                .is_some_and(|position| index.map().facts.emit_movement_effect(position, effect)),
        }
    }
}

struct Impact {
    actor: ExactActorRef,
    session: GameSessionId,
    character: [u8; 16],
    lease: u64,
    amount: u32,
}
struct Pod {
    pin: Pin,
    projection: Projection,
    position: MovementLocalPosition,
    key: Option<u64>,
    created_ms: u64,
    impact: Option<Impact>,
}
impl Pod {
    fn consume(&mut self) -> bool {
        self.key
            .take()
            .is_some_and(|key| self.projection.remove(self.position, key))
    }
}
impl Drop for Pod {
    fn drop(&mut self) {
        self.consume();
    }
}
#[derive(Default)]
pub(super) struct State(Mutex<Vec<Pod>>);

fn damage(max_health: u32, hazard: u16) -> Option<u32> {
    // hazard_primal.lua:42-48: ceil each percentage separately, then add500.
    let base = u64::from(max_health)
        .checked_mul(DAMAGE_PERCENT)?
        .div_ceil(100);
    u32::try_from(
        base.checked_mul(100 + u64::from(hazard))?
            .div_ceil(100)
            .checked_add(500)?,
    )
    .ok()
}
fn damage_window(created: u64, now: u64) -> Option<bool> {
    let elapsed = now.checked_sub(created)?;
    Some((DAMAGE_TIME_MS..SPAWN_TIME_MS).contains(&elapsed))
}

pub(super) fn spawn(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    runtime: &mut ChannelRuntimeV1,
    position: MovementLocalPosition,
    now_ms: u64,
) -> bool {
    let Some(pin) = Pin::current(runtime) else {
        return false;
    };
    if !matches!(pin.scope, RuntimeScopeRefV1::Instance { .. }) {
        return false;
    }
    let scope = owner.movement_cells.scope();
    if scope.world_id != pin.scope.world_id()
        || scope.generation_digest != pin.content
        || owner
            .movement_cells
            .index()
            .lookup(
                scope,
                crate::content::LogicalCell {
                    x: position.x,
                    y: position.y,
                    z: i32::from(position.floor),
                },
            )
            .ok()
            != Some(crate::content::CollisionClass::Walkable)
    {
        return false;
    }

    let Ok(mut pods) = owner.states.primal_pods.0.lock() else {
        return false;
    };
    // Share the existing physical callback bound, rather than inventing a new
    // unbounded timer/impact queue. Consuming a pod reuses its slot for HP retry.
    if pods.len() >= crate::map::overlay::OVERLAY_BUDGET_BYTES / 512 || pods.try_reserve(1).is_err()
    {
        return false;
    }
    let Some(projection) = Projection::current(owner, runtime) else {
        return false;
    };
    let Some(key) = projection.create(owner, runtime, position) else {
        return false;
    };
    pods.push(Pod {
        pin,
        projection,
        position,
        key: Some(key),
        created_ms: now_ms,
        impact: None,
    });
    true
}

pub(super) fn sparkle(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    runtime: &mut ChannelRuntimeV1,
    position: MovementLocalPosition,
    _now_ms: u64,
) -> bool {
    if Pin::current(runtime).is_none() {
        return false;
    }
    Projection::current(owner, runtime)
        .is_some_and(|projection| projection.effect(owner, runtime, position, TELEPORT))
}

#[derive(Debug, PartialEq, Eq)]
enum ImpactResult {
    Deferred,
    Applied,
    Stale,
}

fn hit(
    runtime: &ChannelRuntimeV1,
    states: &mut super::actor_spell::ChannelSpellStates,
    impact: &Impact,
    now_ms: u64,
) -> ImpactResult {
    // An accepted step's delayed impact can never address a reused actor/session
    // or replacement Character lease. No invented creature is the attacker.
    if !runtime
        .bound_attacker_lease(impact.actor, impact.session)
        .is_ok_and(|lease| {
            lease.character_id().as_bytes() == &impact.character
                && lease.generation() == impact.lease
        })
        || !runtime
            .player_control_facts(impact.actor, impact.session)
            .is_ok_and(|facts| facts.control_loss.is_none())
        || states.is_dead(impact.actor)
    {
        return ImpactResult::Stale;
    }
    let Some(now) = now_ms.checked_mul(1000) else {
        return ImpactResult::Deferred;
    };
    if states
        .apply_creature_damage_kind(
            runtime,
            impact.actor,
            impact.session,
            impact.amount,
            crate::player_lethal::PlayerHitKind::LifeDrain,
            SemanticTimeMicros::from_micros(now),
            oteryn_protocol_oteryn::analyser::AnalyserDamageSource::None,
        )
        .is_none()
    {
        return ImpactResult::Deferred;
    }
    ImpactResult::Applied
}
fn apply_impact(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    runtime: &ChannelRuntimeV1,
    states: &mut super::actor_spell::ChannelSpellStates,
    pod: &Pod,
    now_ms: u64,
) -> bool {
    let Some(impact) = &pod.impact else {
        return true;
    };
    match hit(runtime, states, impact, now_ms) {
        ImpactResult::Deferred => false,
        ImpactResult::Stale => true,
        ImpactResult::Applied => {
            pod.projection
                .effect(owner, runtime, pod.position, ENERGY_HIT);
            pod.projection
                .effect(owner, runtime, pod.position, DRAW_BLOOD);
            true
        }
    }
}

/// Called only after the actual paced STEP committed under its runtime/HP owners.
pub(super) fn step(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    runtime: &mut ChannelRuntimeV1,
    states: &mut super::actor_spell::ChannelSpellStates,
    actor: ExactActorRef,
    session: GameSessionId,
    now_ms: u64,
) {
    let Some(pin) = Pin::current(runtime) else {
        return;
    };
    let Ok(at) = runtime.read_actor_position(actor) else {
        return;
    };
    if runtime.player_control_facts(actor, session).is_err() {
        return;
    }
    let position = at.position();
    let Ok(mut pods) = owner.states.primal_pods.0.lock() else {
        return;
    };
    pods.retain(|pod| pod.pin.scope != pin.scope || pod.pin == pin);
    let mut index = 0;
    while index < pods.len() {
        let pod = &mut pods[index];
        if pod.pin != pin || pod.position != position || pod.impact.is_some() {
            index += 1;
            continue;
        }
        let Some(late) = damage_window(pod.created_ms, now_ms) else {
            index += 1;
            continue;
        };
        let impact = if late {
            let Some(state) = states.get(runtime, actor, session) else {
                index += 1;
                continue;
            };
            let Ok(lease) = runtime.bound_attacker_lease(actor, session) else {
                index += 1;
                continue;
            };
            let Some(amount) = damage(
                state.vitals().max_health,
                runtime.hazard_level(actor, session).unwrap_or(0),
            ) else {
                index += 1;
                continue;
            };
            Some(Impact {
                actor,
                session,
                character: *lease.character_id().as_bytes(),
                lease: lease.generation(),
                amount,
            })
        } else {
            None
        };
        if !pod.consume() {
            pods.swap_remove(index);
            continue;
        }
        pod.impact = impact;
        if pod.impact.is_none() || apply_impact(owner, runtime, states, pod, now_ms) {
            pods.swap_remove(index);
        } else {
            index += 1;
        }
    }
}

pub(super) fn tick(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    runtime: &mut ChannelRuntimeV1,
    now_ms: u64,
) {
    let Some(pin) = Pin::current(runtime) else {
        clear(owner, runtime);
        return;
    };
    let mut states = owner.states.spell_states.try_lock().ok();
    let mut spawn_positions = Vec::new();
    let Ok(mut pods) = owner.states.primal_pods.0.lock() else {
        return;
    };
    pods.retain(|pod| pod.pin.scope != pin.scope || pod.pin == pin);
    let mut index = 0;
    while index < pods.len() {
        let pod = &mut pods[index];
        if pod.pin != pin {
            index += 1;
            continue;
        }
        if pod.impact.is_some() {
            if states
                .as_mut()
                .is_some_and(|states| apply_impact(owner, runtime, states, pod, now_ms))
            {
                pods.swap_remove(index);
            } else {
                index += 1;
            }
        } else if now_ms
            .checked_sub(pod.created_ms)
            .is_some_and(|elapsed| elapsed >= SPAWN_TIME_MS)
        {
            if spawn_positions.try_reserve(1).is_err() {
                index += 1;
                continue;
            }
            if pod.consume() {
                spawn_positions.push(pod.position);
            }
            pods.swap_remove(index);
        } else {
            index += 1;
        }
    }
    drop(pods);
    drop(states);
    // hazard_primal.lua:60-68: only a still-present pod can realize this spawn;
    // failed native creature admission still removes the expired physical pod.
    for position in spawn_positions {
        let _ = super::primal_menace::spawn_fungosaurus_from_pod(owner, runtime, position);
    }
}

pub(super) fn clear(owner: &ComposedFreshAdmission<'_, '_, '_>, runtime: &ChannelRuntimeV1) {
    if let Ok(mut pods) = owner.states.primal_pods.0.lock() {
        let scope = runtime.binding().runtime_scope();
        pods.retain(|pod| pod.pin.scope != scope);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_pod_damage_keeps_both_ceiling_operations() {
        assert_eq!(damage(101, 1), Some(507));
        assert_eq!(damage(201, 12), Some(513));
        assert_eq!(damage(1, 0), Some(501));
    }
    #[test]
    fn source_pod_damage_window_is_inclusive_then_exclusive() {
        assert_eq!(damage_window(100, 2099), Some(false));
        assert_eq!(damage_window(100, 2100), Some(true));
        assert_eq!(damage_window(100, 4099), Some(true));
        assert_eq!(damage_window(100, 4100), Some(false));
        assert_eq!(damage_window(100, 99), None);
    }
    #[test]
    fn pod_lifedrain_uses_native_hp_death_and_exact_character_lease() -> Result<(), String> {
        use crate::content::channel_content_pin::ChannelContentPin;
        use crate::foundation::{ChannelId, CharacterId, CharacterLease, NodeId, WorldId};
        let id = |seed| [1, 0, 0, 0, 0, seed, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, seed];
        fn debug(error: impl std::fmt::Debug) -> String {
            format!("{error:?}")
        }
        let world = WorldId::decode(&id(1)).map_err(debug)?;
        let room = crate::content::qualify_native_entry_room(world).map_err(debug)?;
        let start = room.entry_start();
        let pin = ChannelContentPin::from_activation(
            world,
            1,
            room.compiled().server_digest(),
            room.compiled().client_digest(),
            room.frame_binding().digest(),
            room.map_revision_digest(),
            (start.x, start.y, start.floor),
        );
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            world,
            ChannelId::decode(&id(2)).map_err(debug)?,
            NodeId::decode(&id(3)).map_err(debug)?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            2,
            pin,
        )
        .map_err(debug)?;
        let session = GameSessionId::decode(&id(4)).map_err(debug)?;
        let reservation = runtime.reserve_fresh_session(session).map_err(debug)?;
        let actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .initialize_first_entry_position(actor)
            .map_err(debug)?;
        let character = CharacterId::decode(&id(5)).map_err(debug)?;
        runtime
            .bind_attacker_lease(
                actor,
                session,
                CharacterLease::new(character, 1).map_err(debug)?,
            )
            .map_err(debug)?;
        let mut states = super::super::actor_spell::ChannelSpellStates::default();
        states
            .initialize(
                &runtime,
                actor,
                session,
                crate::spell::cast::CharacterCastFacts {
                    vocation: crate::spell::Vocation::Knight,
                    level: 291,
                    magic_level: 10,
                    max_health: 1000,
                    max_mana: 500,
                    max_soul: 100,
                },
                (0, 0),
                oteryn_simulation_determinism::SemanticTimeMicros::from_micros(0),
            )
            .ok_or("initialize native pod player HP")?;
        runtime
            .install_hazard_level(actor, session, 12)
            .map_err(debug)?;
        let amount = damage(
            states
                .get(&runtime, actor, session)
                .ok_or("actor state")?
                .vitals()
                .max_health,
            runtime
                .hazard_level(actor, session)
                .ok_or("actual selected hazard")?,
        )
        .ok_or("source pod damage")?;
        assert_eq!(amount, 556);
        let mut impact = Impact {
            actor,
            session,
            character: *character.as_bytes(),
            lease: 2,
            amount,
        };
        assert_eq!(
            hit(&runtime, &mut states, &impact, 2000),
            ImpactResult::Stale
        );
        assert_eq!(
            states
                .get(&runtime, actor, session)
                .ok_or("actor state")?
                .vitals()
                .health,
            1000
        );
        impact.lease = 1;
        assert_eq!(
            hit(&runtime, &mut states, &impact, 2000),
            ImpactResult::Applied
        );
        assert_eq!(
            states
                .get(&runtime, actor, session)
                .ok_or("actor state")?
                .vitals()
                .health,
            444
        );
        impact.amount = 500;
        assert_eq!(
            hit(&runtime, &mut states, &impact, 2001),
            ImpactResult::Applied
        );
        let death = states
            .player_death(&runtime, actor, session)
            .ok_or("native pod death")?;
        assert_eq!(
            hit(&runtime, &mut states, &impact, 2002),
            ImpactResult::Stale
        );
        assert_eq!(
            states
                .player_death(&runtime, actor, session)
                .ok_or("retained native death")?,
            death
        );
        Ok(())
    }
}
