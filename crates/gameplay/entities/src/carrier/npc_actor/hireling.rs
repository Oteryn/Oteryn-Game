//! House Hirelings occupy existing NPC slots; current House custody gates every observation.
use super::*;
use crate::carrier::{GameSessionId, MovementLocalPosition};
use oteryn_gameplay_foundation::foundation::{RuntimeScopeRefV1, ScopeOwnershipGeneration};
use std::collections::BTreeMap;

const PREFIX: &str = "oteryn:npc.hireling:";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HirelingProjection {
    pub key: String,
    pub name: String,
    /// Native floor is negative legacy z, exactly like every NpcPlacement.
    pub placement: NpcPlacement,
    pub look_type: u16,
    pub colors: [u8; 4],
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[doc(hidden)]
pub struct HouseHirelingBinding {
    house: String,
    scope: RuntimeScopeRefV1,
    generation: ScopeOwnershipGeneration,
    projection: HirelingProjection,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HouseHirelingEntity {
    pub actor: ExactActorRef,
    pub key: String,
    pub name: String,
    pub position: MovementLocalPosition,
    pub direction: MovementFacing,
    pub look_type: u16,
    pub colors: [u8; 4],
    pub revision: u64,
}

fn matches_house(
    binding: &HouseHirelingBinding,
    state: &crate::carrier::house_custody::HouseActorState,
) -> bool {
    binding.house == state.house
        && binding.scope == state.scope
        && binding.generation == state.generation
}

impl ChannelRuntimeV1 {
    /// Rebuild from the bounded current durable House read while holding the runtime lock.
    /// An identical read preserves actor identities and revisions. A rejected projection is atomic.
    pub fn reconcile_house_hirelings(
        &mut self,
        observer: ExactActorRef,
        session: GameSessionId,
        projections: &[HirelingProjection],
    ) -> Result<Vec<ExactActorRef>, NpcPlacementError> {
        self.carrier
            .validate_current_continuity(&self.continuity)
            .map_err(NpcPlacementError::Carrier)?;
        let state = self
            .house_actor_state(observer, session)
            .map_err(NpcPlacementError::Carrier)?;
        if projections.len() > 10 {
            return Err(NpcPlacementError::PlacementsPerNpcOverLimit);
        }
        let mut wanted = BTreeMap::new();
        let mut cells = BTreeSet::new();
        for projection in projections {
            let suffix = projection
                .key
                .strip_prefix(PREFIX)
                .ok_or(NpcPlacementError::InvalidKey)?;
            if suffix.len() != 32
                || !suffix
                    .bytes()
                    .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
                || suffix.as_bytes()[12] != b'7'
                || !matches!(suffix.as_bytes()[16], b'8' | b'9' | b'a' | b'b')
                || !(3..=18).contains(&projection.name.len())
                || !projection.name.is_ascii()
                || projection
                    .name
                    .bytes()
                    .any(|b| !b.is_ascii_alphabetic() && b != b' ')
                || projection.look_type == 0
            {
                return Err(NpcPlacementError::InvalidKey);
            }
            let position = cell(projection.placement)?;
            if wanted.insert(projection.key.as_str(), projection).is_some() {
                return Err(NpcPlacementError::KeysNotAscending);
            }
            if !cells.insert((position.floor, position.y, position.x)) {
                return Err(NpcPlacementError::SharedCell);
            }
        }
        let existing: Vec<_> = self
            .carrier
            .occupied
            .iter()
            .filter_map(|raw| {
                let i = usize::try_from(*raw).ok()?;
                match self.carrier.slots.get(i)? {
                    Slot::NpcOccupied {
                        hireling: Some(binding),
                        ..
                    } if matches_house(binding, &state) => Some((i, binding)),
                    _ => None,
                }
            })
            .collect();
        if existing.len() == wanted.len()
            && existing.iter().all(|(_, binding)| {
                wanted
                    .get(binding.projection.key.as_str())
                    .is_some_and(|value| **value == binding.projection)
            })
        {
            let mut by_key = existing;
            by_key.sort_by(|a, b| a.1.projection.key.cmp(&b.1.projection.key));
            return by_key
                .iter()
                .map(|(index, _)| {
                    self.hireling_actor(*index)
                        .map_err(NpcPlacementError::Carrier)
                })
                .collect();
        }
        // At most ten current, ten stale and ten newly allocated Hireling slots need undo.
        // Never copy the carrier's full reserved capacity during a live refresh.
        let mut undo = Vec::with_capacity(30);
        let occupied = self.carrier.occupied.clone();
        let free_head = self.carrier.free_head;
        let result = self.install_house_hirelings(&state, &wanted, &mut undo);
        if result.is_err() {
            for (index, slot) in undo {
                self.carrier.slots[index] = slot;
            }
            self.carrier.occupied = occupied;
            self.carrier.free_head = free_head;
        }
        result
    }

    fn hireling_actor(&self, index: usize) -> Result<ExactActorRef, CarrierError> {
        match &self.carrier.slots[index] {
            Slot::NpcOccupied { position, .. } => {
                Ok(ExactActorRef(ActorRef::from_scoped_owner_parts(
                    self.carrier.runtime_scope,
                    self.carrier.scope_generation,
                    position.actor_local_id,
                    position.actor_local_generation,
                )))
            }
            _ => Err(CarrierError::WrongScope),
        }
    }

    fn install_house_hirelings(
        &mut self,
        state: &crate::carrier::house_custody::HouseActorState,
        wanted: &BTreeMap<&str, &HirelingProjection>,
        undo: &mut Vec<(usize, Slot)>,
    ) -> Result<Vec<ExactActorRef>, NpcPlacementError> {
        let remove: Vec<_> = self
            .carrier
            .occupied
            .iter()
            .filter_map(|raw| {
                let index = usize::try_from(*raw).ok()?;
                match self.carrier.slots.get(index)? {
                    Slot::NpcOccupied {
                        key,
                        hireling: Some(binding),
                        ..
                    } if (binding.house == state.house
                        && (!matches_house(binding, state)
                            || !wanted.contains_key(key.as_ref())))
                        || (wanted.contains_key(key.as_ref())
                            && !matches_house(binding, state)) =>
                    {
                        Some(index)
                    }
                    _ => None,
                }
            })
            .collect();
        for index in remove {
            self.remember_hireling_slot(index, undo)?;
            self.release_hireling_slot(index)?;
        }
        let context = self.pinned_position_context();
        let mut actors = Vec::with_capacity(wanted.len());
        for projection in wanted.values() {
            let existing=self.carrier.occupied.iter().find_map(|raw| {
                let index=usize::try_from(*raw).ok()?;
                matches!(self.carrier.slots.get(index),Some(Slot::NpcOccupied {key,hireling:Some(binding),..}) if key.as_ref()==projection.key && matches_house(binding,state)).then_some(index)
            });
            let actor = if let Some(index) = existing {
                self.remember_hireling_slot(index, undo)?;
                if let Slot::NpcOccupied {
                    position,
                    hireling: Some(binding),
                    ..
                } = &mut self.carrier.slots[index]
                    && binding.projection != **projection
                {
                    position.revision =
                        position
                            .revision
                            .checked_add(1)
                            .ok_or(NpcPlacementError::Carrier(
                                CarrierError::PositionRevisionExhausted,
                            ))?;
                    position.position = cell(projection.placement)?;
                    position.facing = Some(projection.placement.direction);
                    binding.projection = (*projection).clone();
                }
                self.hireling_actor(index)
                    .map_err(NpcPlacementError::Carrier)?
            } else {
                if let Some(free) = self.carrier.free_head {
                    self.remember_hireling_slot(
                        usize::try_from(free).map_err(|_| {
                            NpcPlacementError::Carrier(CarrierError::CapacityArithmeticOverflow)
                        })?,
                        undo,
                    )?;
                }
                let actor = self.admit_npc(
                    Arc::from(projection.key.as_str()),
                    projection.placement,
                    context,
                )?;
                let index = usize::try_from(actor.actor_local_id()).map_err(|_| {
                    NpcPlacementError::Carrier(CarrierError::CapacityArithmeticOverflow)
                })? - 1;
                if let Slot::NpcOccupied { hireling, .. } = &mut self.carrier.slots[index] {
                    *hireling = Some(Box::new(HouseHirelingBinding {
                        house: state.house.clone(),
                        scope: state.scope,
                        generation: state.generation,
                        projection: (*projection).clone(),
                    }));
                }
                actor
            };
            actors.push(actor);
        }
        Ok(actors)
    }

    fn remember_hireling_slot(
        &self,
        index: usize,
        undo: &mut Vec<(usize, Slot)>,
    ) -> Result<(), NpcPlacementError> {
        if undo.iter().any(|(held, _)| *held == index) {
            return Ok(());
        }
        if undo.len() == 30 {
            return Err(NpcPlacementError::PlacementsOverLimit);
        }
        let slot = self
            .carrier
            .slots
            .get(index)
            .ok_or(NpcPlacementError::Carrier(
                CarrierError::InvalidActorIdentity,
            ))?;
        undo.push((index, slot.clone()));
        Ok(())
    }

    fn release_hireling_slot(&mut self, index: usize) -> Result<(), NpcPlacementError> {
        let Slot::NpcOccupied {
            generation,
            hireling: Some(_),
            ..
        } = &self.carrier.slots[index]
        else {
            return Err(NpcPlacementError::Carrier(CarrierError::WrongScope));
        };
        let generation = *generation;
        let free = u32::try_from(index)
            .map_err(|_| NpcPlacementError::Carrier(CarrierError::CapacityArithmeticOverflow))?;
        self.carrier.slots[index] = Slot::VacantReusable {
            generation,
            next_free: self.carrier.free_head,
        };
        self.carrier.free_head = Some(free);
        self.carrier.occupied.retain(|held| *held != free);
        Ok(())
    }

    /// The House custody owner calls this only after its last committed resident exit.
    /// These are disposable projections; durable identity/lamp state remains owned by the DB.
    /// No fallible operation follows the already committed player-slot transition.
    pub(crate) fn retire_house_hireling_projection(
        &mut self,
        house: &str,
        scope: RuntimeScopeRefV1,
        generation: ScopeOwnershipGeneration,
    ) {
        let retired: Vec<_> = self
            .carrier
            .occupied
            .iter()
            .filter_map(|raw| match self.carrier.slots.get(*raw as usize)? {
                Slot::NpcOccupied {
                    generation: slot_generation,
                    hireling: Some(binding),
                    ..
                } if binding.house == house
                    && binding.scope == scope
                    && binding.generation == generation =>
                {
                    Some((*raw, *slot_generation))
                }
                _ => None,
            })
            .collect();
        for (raw, slot_generation) in retired {
            self.carrier.slots[raw as usize] = Slot::VacantReusable {
                generation: slot_generation,
                next_free: self.carrier.free_head,
            };
            self.carrier.free_head = Some(raw);
            self.carrier.occupied.retain(|held| *held != raw);
        }
    }

    /// Scoped NPC visibility: same current House and floor, with durable name and appearance.
    pub fn house_hireling_entities(
        &self,
        observer: ExactActorRef,
        session: GameSessionId,
    ) -> Result<Vec<HouseHirelingEntity>, CarrierError> {
        let state = self.house_actor_state(observer, session)?;
        let mut entities = Vec::new();
        for raw in &self.carrier.occupied {
            let index =
                usize::try_from(*raw).map_err(|_| CarrierError::CapacityArithmeticOverflow)?;
            let Some(slot) = self.carrier.slots.get(index) else {
                return Err(CarrierError::InvalidActorIdentity);
            };
            if let Slot::NpcOccupied {
                position,
                hireling: Some(binding),
                ..
            } = slot
                && matches_house(binding, &state)
                && position.position.floor == state.position.floor
            {
                entities.push(HouseHirelingEntity {
                    actor: self.hireling_actor(index)?,
                    key: binding.projection.key.clone(),
                    name: binding.projection.name.clone(),
                    position: MovementLocalPosition {
                        x: position.position.x,
                        y: position.position.y,
                        floor: position.position.floor,
                    },
                    direction: binding.projection.placement.direction,
                    look_type: binding.projection.look_type,
                    colors: binding.projection.colors,
                    revision: position.revision,
                });
            }
        }
        entities.sort_by(|a, b| a.key.cmp(&b.key));
        Ok(entities)
    }

    /// The caller still applies its existing NPC talk/trade reach rule after resolving this key.
    pub fn house_hireling_key(
        &self,
        observer: ExactActorRef,
        session: GameSessionId,
        npc: ExactActorRef,
    ) -> Option<&str> {
        let state = self.house_actor_state(observer, session).ok()?;
        let index = self.carrier.validate_ref(&self.continuity, npc.0).ok()?;
        match &self.carrier.slots[index] {
            Slot::NpcOccupied {
                generation,
                key,
                position,
                hireling: Some(binding),
                ..
            } if *generation == npc.actor_local_generation()
                && matches_house(binding, &state)
                && position.position.floor == state.position.floor =>
            {
                Some(key)
            }
            _ => None,
        }
    }

    pub fn house_hireling_cell_occupied(
        &self,
        observer: ExactActorRef,
        session: GameSessionId,
        at: LocalPosition,
    ) -> Result<bool, CarrierError> {
        let state = self.house_actor_state(observer, session)?;
        Ok(self.carrier.occupied.iter().filter_map(|index|self.carrier.slots.get(*index as usize)).any(|slot|matches!(slot,Slot::NpcOccupied {position,hireling:Some(binding),..} if matches_house(binding,&state)&&position.position==at)))
    }
}

#[cfg(test)]
#[path = "hireling_tests.rs"]
mod tests;
