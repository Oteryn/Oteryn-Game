use super::*;
use crate::map::facts::QuestCallbackProjection;
use crate::map::view::EntryFacts;
use oteryn_gameplay_quests::quest::world_callbacks::Facing;

pub(super) fn facing(direction: Facing) -> crate::foundation::MovementFacing {
    use crate::foundation::MovementFacing as F;
    match direction {
        Facing::North => F::North,
        Facing::East => F::East,
        Facing::South => F::South,
        Facing::West => F::West,
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) fn prepare_quest_projection(
        &self,
        map: &BundleMap,
        callback: &Callback,
        close: bool,
        key: u64,
        after: &mut EntryFacts,
    ) -> Result<Option<QuestCallbackProjection>, UseDisposition> {
        if let Some(target) = &callback.effects.target_transform {
            return super::source_transforms::prepare(map, target).map(Some);
        }
        if let Some(cycle) = &callback.effects.item_layout_cycle {
            let step = cycle
                .source_appearances
                .iter()
                .position(|appearance| *appearance == after.appearance_id)
                .ok_or(UseDisposition::NothingToUse)?;
            let next = cycle.source_appearances[(step + 1) % cycle.source_appearances.len()];
            *after = map
                .facts
                .quest_callback_replacement(key, next)
                .ok_or(UseDisposition::Rejected)?;
            let positions = cycle
                .positions
                .iter()
                .copied()
                .map(tile)
                .collect::<Vec<_>>();
            if positions.iter().any(|position| {
                map.facts.house_tile(*position)
                    || map
                        .overlay
                        .base()
                        .tile(position.x, position.y, position.floor)
                        .is_none()
            }) {
                return Err(UseDisposition::Rejected);
            }
            let create = positions
                .get(step)
                .map(|position| {
                    map.facts
                        .movement_item_facts(cycle.appearance)
                        .filter(|entry| !entry.pickupable)
                        .map(|entry| (*position, entry))
                        .ok_or(UseDisposition::Rejected)
                })
                .transpose()?;
            let mut native_removals = Vec::new();
            for &position in &positions {
                let base = map
                    .overlay
                    .base()
                    .tile(position.x, position.y, position.floor)
                    .ok_or(UseDisposition::Rejected)?;
                for (ordinal, (id, _)) in base
                    .ids()
                    .iter()
                    .zip(base.depths())
                    .filter(|(_, depth)| **depth == 0)
                    .enumerate()
                {
                    let ordinal = u8::try_from(ordinal).map_err(|_| UseDisposition::Rejected)?;
                    let key = placement_key(position, ordinal).ok_or(UseDisposition::Rejected)?;
                    if !map.facts.base_hidden(key)
                        && map
                            .facts
                            .base_entry(position, ordinal, *id)
                            .is_some_and(|entry| {
                                cycle.clear_appearances.contains(&entry.appearance_id)
                                    && !entry.pickupable
                            })
                    {
                        native_removals.push(crate::map::facts::QuestSceneMutation::Remove {
                            key,
                            revision: map.facts.object_revision(key),
                        });
                    }
                }
            }
            return Ok(Some(QuestCallbackProjection::ItemLayout {
                positions,
                native_removals,
                create,
            }));
        }
        if let Some(presence) = &callback.effects.item_presence {
            let pos = tile(presence.position);
            let facts = map
                .facts
                .movement_item_facts(presence.appearance)
                .ok_or(UseDisposition::Rejected)?;
            // Initially admit known walkable ladders; projections cannot mint portable value
            // or introduce collision without an owning collision catalogue.
            if map.facts.house_tile(pos)
                || facts.pickupable
                || !crate::map::floor::is_ladder_appearance(facts.appearance_id)
                || map.overlay.base().tile(pos.x, pos.y, pos.floor).is_none()
            {
                return Err(UseDisposition::Rejected);
            }
            return Ok(Some(QuestCallbackProjection::ItemPresence {
                position: pos,
                entry: facts,
            }));
        }
        if callback.effects.scene.is_some() {
            return self.prepare_quest_scene(map, callback, close);
        }
        if let Some(pair) = &callback.effects.portals {
            let facts = map
                .facts
                .movement_item_facts(pair.appearance)
                .ok_or(UseDisposition::Rejected)?;
            if facts.pickupable {
                return Err(UseDisposition::Rejected);
            }
            let endpoint = |index: usize| {
                let portal = &pair.portals[index];
                let at = tile(portal.position);
                let to = tile(portal.destination);
                if map.facts.house_tile(at)
                    || map.facts.house_tile(to)
                    || map.overlay.base().tile(at.x, at.y, at.floor).is_none()
                    || map.overlay.base().tile(to.x, to.y, to.floor).is_none()
                {
                    return Err(UseDisposition::Rejected);
                }
                Ok((at, to, facts))
            };
            let expires = pair
                .lifetime_us
                .map(|delay| {
                    self.owner_now()
                        .get()
                        .checked_add(delay)
                        .ok_or(UseDisposition::Exhausted)
                })
                .transpose()?;
            return Ok(Some(QuestCallbackProjection::Portals {
                endpoints: [endpoint(0)?, endpoint(1)?],
                expires,
            }));
        }
        if let Some(cycle) = &callback.effects.item_cycle {
            let pos = tile(cycle.position);
            if map.facts.house_tile(pos) {
                return Err(UseDisposition::Rejected);
            }
            let base = map
                .overlay
                .base()
                .tile(pos.x, pos.y, pos.floor)
                .ok_or(UseDisposition::Rejected)?;
            let mut candidates = base
                .ids()
                .iter()
                .zip(base.depths())
                .filter(|(_, d)| **d == 0)
                .enumerate()
                .filter_map(|(o, (id, _))| {
                    let ordinal = u8::try_from(o).ok()?;
                    let k = placement_key(pos, ordinal)?;
                    let facts = map.facts.base_entry(pos, ordinal, *id)?;
                    (facts.appearance_id == cycle.remove && !map.facts.base_hidden(k)).then_some(k)
                });
            let remove = candidates.next();
            if candidates.next().is_some() {
                return Err(UseDisposition::Rejected);
            }
            // Crystal's asymmetric branch never resurrects the removed 1270.
            // A subsequent use without it creates 1025 without toggling the lever.
            let create = if close || remove.is_none() {
                if !close {
                    after.appearance_id = callback.lever.ok_or(UseDisposition::Rejected)?[0];
                }
                Some(
                    map.facts
                        .movement_item_facts(cycle.create)
                        .ok_or(UseDisposition::Rejected)?,
                )
            } else {
                None
            };
            if create.is_some_and(|facts| facts.pickupable) {
                return Err(UseDisposition::Rejected);
            }
            return Ok(Some(QuestCallbackProjection::ItemCycle {
                position: pos,
                remove: if create.is_none() { remove } else { None },
                create,
            }));
        }
        Ok(None)
    }

    pub(super) async fn step_dynamic_quest_portal(
        &self,
        permit: &SpellLanePermit,
        actor: ExactActorRef,
        session: GameSessionId,
        entered: MovementPositionSnapshot,
    ) -> MovementPositionSnapshot {
        let Some(map) = self.bundle_map() else {
            return entered;
        };
        let Some(pos) = native(entered.position()) else {
            return entered;
        };
        use crate::map::facts::QuestPortalTravel;

        let (corpse, to, travel) = if let Some((to, travel)) = map
            .facts
            .quest_portal_traversal(pos, self.owner_now().get())
        {
            (
                None,
                super::local(Position {
                    x: to.x,
                    y: to.y,
                    floor: to.floor,
                }),
                travel,
            )
        } else if let Some((corpse, to)) = self
            .runtime
            .lock()
            .await
            .quest_corpse_portal_at(entered.position())
        {
            (Some(corpse), to, QuestPortalTravel::Native { effect: None })
        } else {
            return entered;
        };
        let optional_pvp = self.special_movement_policy(permit, actor, session).await;
        let locks = tokio::time::timeout(std::time::Duration::from_secs(10), async {
            loop {
                let runtime = self.runtime.lock().await;
                if let Ok(states) = self.states.spell_states.try_lock() {
                    break (runtime, states);
                }
                // Keep the established runtime -> spell-state lock order, but
                // wait for a busy state owner without holding the runtime.
                drop(runtime);
                drop(self.states.spell_states.lock().await);
            }
        })
        .await;
        let Ok((mut runtime, mut states)) = locks else {
            return entered;
        };
        if runtime.read_actor_position(actor) != Ok(entered)
            || states.is_dead(actor)
            || !states.spell_idle(&runtime, actor, session)
            || !self.states.player_trades.physical(session, None)
        {
            return entered;
        }
        // The policy lookup awaited outside the owner. A lever/expiry may have
        // removed the pair while it was in flight; never teleport through that stale read.
        let current = match corpse {
            Some(corpse) => {
                runtime.quest_corpse_portal_at(entered.position()) == Some((corpse, to))
            }
            None => {
                map.facts
                    .quest_portal_traversal(pos, self.owner_now().get())
                    == native(to).map(|destination| (destination, travel))
            }
        };
        if !current {
            return entered;
        }
        let closed = crate::map::floor::BundleDoors(&map.facts);
        let blocked = super::super::pvp_live::pz_blocked(&states, actor, session, self.owner_now());
        // Crystal 201189f8, src/game/movement/teleport.cpp:95-104 turns entrants
        // along the x axis, even when the landing already contains a creature.
        let native_teleport = matches!(travel, QuestPortalTravel::Native { .. });
        let portal_facing = native_teleport.then(|| {
            if entered.position().x > to.x {
                crate::foundation::MovementFacing::West
            } else {
                crate::foundation::MovementFacing::East
            }
        });
        let Ok(proof) = crate::map::floor::prepare_quest_encounter_landing(
            &runtime,
            &self.movement_cells,
            actor,
            session,
            entered,
            to,
            blocked,
            &closed,
            native_teleport,
            portal_facing,
        ) else {
            return entered;
        };
        let Ok(snapshot) = runtime.commit_source_step(proof) else {
            return entered;
        };
        if let QuestPortalTravel::Native {
            effect: Some(effect),
        } = travel
            && let Some(landing) = native(snapshot.position())
        {
            map.facts.emit_movement_effect(pos, effect);
            map.facts.emit_movement_effect(landing, effect);
        }
        if pos == tile(oteryn_gameplay_quests::quest::dreamer_fire_bug::SOURCE)
            && to == local(oteryn_gameplay_quests::quest::dreamer_fire_bug::DESTINATION)
        {
            map.facts.emit_movement_effect(
                tile(oteryn_gameplay_quests::quest::dreamer_fire_bug::DESTINATION),
                11,
            );
        }
        states.stage_special_movement_step(
            &runtime,
            map,
            actor,
            session,
            self.owner_now().get(),
            Some(entered.position()),
            snapshot.position(),
            optional_pvp,
        );
        states.stage_underwater_step(
            &runtime,
            actor,
            session,
            self.owner_now().get(),
            underwater(map, entered.position()),
            underwater(map, snapshot.position()),
        );
        snapshot
    }
}
