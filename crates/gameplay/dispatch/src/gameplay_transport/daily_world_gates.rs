//! W05b source step events, bound to selected compiler-admitted daily layers.
use super::ComposedFreshAdmission;
use crate::durability::spell_owner_commit::SpellLanePermit;
use crate::foundation::{
    ExactActorRef, GameSessionId, MovementLocalPosition, MovementPositionSnapshot,
};
use crate::map::{
    boot::BundleMap,
    overlay::TilePos,
    view::{MapFacts, placement_key},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Gate {
    FuryEntry,
    To(u16, u16, i16),
}

// Crystal 201189f fury_gates.lua:5–15; values are legacy z, as in movement's sealed proof.
const FURY_EXITS: [(&str, u16, u16); 10] = [
    ("abdendriel", 32680, 31720),
    ("ankrahmun", 33269, 32841),
    ("carlin", 32263, 31848),
    ("darashia", 33304, 32371),
    ("edron", 33221, 31923),
    ("kazordoon", 32575, 31981),
    ("libertybay", 32348, 32693),
    ("porthope", 32530, 32712),
    ("thais", 32265, 32164),
    ("venore", 32834, 32082),
];
const NIGHTMARE_EXITS: [(&str, u16, u16); 3] = [
    ("ankrahmun-north", 33255, 32678),
    ("darashia-north", 33215, 32273),
    ("darashia-west", 33032, 32400),
];
// Crystal nightmare_isles.lua:75–92, keyed by the actual placed action id.
const LADDERS: [(u16, u16, u16, i16); 16] = [
    (64103, 33475, 32641, 10),
    (64104, 33473, 32647, 9),
    (64105, 33463, 32585, 8),
    (64106, 33457, 32580, 8),
    (64107, 33422, 32582, 8),
    (64108, 33430, 32600, 10),
    (64109, 33420, 32604, 10),
    (64120, 33446, 32616, 11),
    (64121, 33460, 32632, 10),
    (64122, 33429, 32626, 10),
    (64123, 33425, 32633, 8),
    (64124, 33435, 32631, 8),
    (64125, 33478, 32621, 10),
    (64126, 33484, 32629, 8),
    (64127, 33452, 32617, 11),
    (64128, 33419, 32589, 10),
];

fn selected_exit<const N: usize>(
    layers: &[String],
    prefix: &str,
    exits: &[(&str, u16, u16); N],
) -> Option<(u16, u16)> {
    let mut selected = layers.iter().filter_map(|key| {
        let suffix = key.strip_prefix(prefix)?;
        exits
            .iter()
            .find(|(name, _, _)| *name == suffix)
            .map(|(_, x, y)| (*x, *y))
    });
    let one = selected.next()?;
    selected.next().is_none().then_some(one)
}

fn gate(map: &BundleMap, at: MovementLocalPosition) -> Option<Gate> {
    let pos = TilePos {
        x: at.x.try_into().ok()?,
        y: at.y.try_into().ok()?,
        floor: i8::try_from(at.floor).ok()?.checked_neg()?,
    };
    let tile = map.overlay.base().tile(pos.x, pos.y, pos.floor)?;
    let active = map.overlay.base().active_layers();
    let fury = selected_exit(active, "fury_gates/", &FURY_EXITS);
    let fury_exit = selected_exit(map.overlay.base().exit_layers(), "fury_gates/", &FURY_EXITS);
    let nightmare = selected_exit(active, "nightmare_isle/", &NIGHTMARE_EXITS);
    let mut found = None;
    for ordinal in 0..tile.depths().iter().filter(|d| **d == 0).count() {
        let key = placement_key(pos, u8::try_from(ordinal).ok()?)?;
        if map.facts.base_hidden(key) {
            continue;
        }
        let matches = |unique, action| map.facts.quest_callback_matches(key, unique, action, None);
        let selected = if let Some((x, y)) = fury_exit.filter(|_| matches(None, Some(9715))) {
            Some(Gate::To(x, y, 7))
        } else if fury.is_some() && matches(None, Some(9710)) {
            Some(Gate::FuryEntry)
        } else if let Some((x, y)) = nightmare.filter(|_| matches(Some(35020), None)) {
            Some(Gate::To(x, y, 7))
        } else if nightmare.is_some()
            && [64001, 64002, 64003]
                .iter()
                .any(|id| matches(Some(*id), None))
        {
            Some(Gate::To(33497, 32616, 8))
        } else if nightmare.is_some() {
            LADDERS
                .iter()
                .find(|(id, _, _, _)| matches(None, Some(*id)))
                .map(|(_, x, y, z)| Gate::To(*x, *y, *z))
        } else {
            None
        };
        if let Some(selected) = selected {
            // Ambiguous native placements never acquire an incidental registration order.
            if found.is_some() {
                return None;
            }
            found = Some(selected);
        }
    }
    found
}

fn fury_admitted(premium: bool, promoted: bool, level: u32) -> bool {
    premium && promoted && level >= 60
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn step_daily_world_gate(
        &self,
        permit: &SpellLanePermit,
        actor: ExactActorRef,
        session: GameSessionId,
        origin: MovementPositionSnapshot,
        entered: MovementPositionSnapshot,
    ) -> MovementPositionSnapshot {
        let Some(map) = self.bundle_map() else {
            return entered;
        };
        let Some(selected) = gate(map, entered.position()) else {
            return entered;
        };
        // Resolve current durable owners before taking Channel guards; the movement snapshot
        // and the current state are checked again under the mutation owner lock below.
        let (access, promoted) = if selected == Gate::FuryEntry {
            let access = self.refresh_and_apply_spell_access(actor, session).await;
            let promoted = match self.current_quest_fence(session).await {
                Ok(Some(fence)) => self
                    .root
                    .read_promotion_for_session(self.character, self.holder, fence.item_fence())
                    .await
                    .unwrap_or(false),
                _ => false,
            };
            (Some(access), promoted)
        } else {
            (None, false)
        };
        let policy = self.special_movement_policy(permit, actor, session).await;
        let mut runtime = self.runtime.lock().await;
        let mut states = self.states.spell_states.lock().await;
        if runtime.read_actor_position(actor) != Ok(entered)
            || states.is_dead(actor)
            || states.has_pending_spell_commit(actor, session)
            || gate(map, entered.position()) != Some(selected)
        {
            return entered;
        }
        let now = self.owner_now();
        let destination = match selected {
            Gate::To(x, y, floor) => MovementLocalPosition {
                x: i32::from(x),
                y: i32::from(y),
                floor,
            },
            Gate::FuryEntry => {
                let admitted = states.get(&runtime, actor, session).is_some_and(|state| {
                    fury_admitted(
                        access
                            .as_ref()
                            .is_some_and(|view| view.premium_gate().premium_current())
                            && state.hunting_premium_current(now.get()),
                        promoted && state.wheel_promoted(),
                        state.character_facts().level,
                    )
                });
                if admitted {
                    MovementLocalPosition {
                        x: 33290,
                        y: 31786,
                        floor: 13,
                    }
                } else {
                    origin.position()
                }
            }
        };
        let closed = crate::map::floor::BundleDoors(&map.facts);
        let pz_blocked =
            crate::gameplay_transport::pvp_live::pz_blocked(&states, actor, session, now);
        let Ok(proof) = crate::map::floor::prepare_quest_callback_teleport(
            &runtime,
            self.movement_cells(),
            actor,
            session,
            entered,
            destination,
            pz_blocked,
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
            now.get(),
            Some(entered.position()),
            snapshot.position(),
            policy,
        );
        states.stage_underwater_step(
            &runtime,
            actor,
            session,
            now.get(),
            crate::gameplay_transport::paced_movement::underwater_drown::underwater(
                map,
                entered.position(),
            ),
            crate::gameplay_transport::paced_movement::underwater_drown::underwater(
                map,
                snapshot.position(),
            ),
        );
        if selected == Gate::FuryEntry && destination == origin.position() {
            self.states.chat.push_quest_feedback(session,"Only Premium promoted players of level 60 or higher are able to enter this portal.");
        }
        snapshot
    }
}

#[cfg(test)]
#[path = "daily_world_gates/tests.rs"]
mod tests;
