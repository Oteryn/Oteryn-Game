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
        after: &mut EntryFacts,
    ) -> Result<Option<QuestCallbackProjection>, UseDisposition> {
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
        let Some(to) = map
            .facts
            .quest_portal_destination(pos, self.owner_now().get())
        else {
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
        if map
            .facts
            .quest_portal_destination(pos, self.owner_now().get())
            != Some(to)
        {
            return entered;
        }
        let closed = crate::map::floor::BundleDoors(&map.facts);
        let blocked = super::super::pvp_live::pz_blocked(&states, actor, session, self.owner_now());
        let Ok(proof) = crate::map::floor::prepare_quest_callback_teleport(
            &runtime,
            &self.movement_cells,
            actor,
            session,
            entered,
            local(Position {
                x: to.x,
                y: to.y,
                floor: to.floor,
            }),
            blocked,
            &closed,
        ) else {
            return entered;
        };
        let Ok(snapshot) = runtime.commit_source_step(proof) else {
            return entered;
        };
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
