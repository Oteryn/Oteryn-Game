//! WU26-06 Temple Teleport Scroll (`docs/contracts/TEMPLE_TELEPORT_SCROLL_V1.md`): the Channel
//! owner's use of item 25718 from the carried tree. PREPARE refuses a NOLOGOUT cell, a logout
//! block and a protection-zone block, resolves the home temple and checks its landing; the
//! native item-use writer burns the scroll with its use row and, before COMMIT, moves the actor
//! to the free landing closest to the temple. No landing rolls the burn back, so the scroll is
//! never consumed without the move. An expired scroll is removed by the Item deadline owner,
//! not by a use; a use that races its deadline is refused with nothing written. English
//! TibiaWiki "Temple Teleport Scroll"; Winter Update 2026 thread 565.
use super::*;
use crate::content::NativeEntryMovementCells;
use crate::durability::item_use::temple_scroll::{TempleLanding, TempleScrollOutcome};
use crate::foundation::MovementLocalPosition;
use crate::map::floor::CellBlocker;
use oteryn_simulation_determinism::SemanticTimeMicros;
use std::future::Future;
use std::pin::Pin;

/// The landing order around the temple, after the temple cell itself: Crystal's placement
/// ring (`Map::placeCreature`'s normal relative list, the closest-free-tile search a temple
/// teleport uses), row by row from the north-west.
pub(super) const LANDING_RING: [(i32, i32); 8] = [
    (-1, -1),
    (0, -1),
    (1, -1),
    (-1, 0),
    (1, 0),
    (-1, 1),
    (0, 1),
    (1, 1),
];

/// A PREPARE refusal of the scroll; each burns, moves and writes nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum TempleScrollRefusal {
    /// The actor's cell is NOLOGOUT, or its zone evidence is unreadable.
    NoLogoutZone,
    /// The `LOGOUT_INTENT` predicate refuses: a live PvP lock or the PvE in-fight deadline.
    InFight,
    /// The live PvP lock blocks protection-zone entry.
    PzBlocked,
}

impl TempleScrollRefusal {
    pub(super) const fn disposition(self) -> UseDisposition {
        match self {
            Self::NoLogoutZone | Self::InFight => UseDisposition::NotPossible,
            Self::PzBlocked => UseDisposition::PzBlocked,
        }
    }
}

/// The contract's refusal order: the zone, then the logout block, then the PZ block.
pub(super) const fn temple_scroll_refusal(
    no_logout: Option<bool>,
    in_fight: bool,
    pz_blocked: bool,
) -> Option<TempleScrollRefusal> {
    if !matches!(no_logout, Some(false)) {
        Some(TempleScrollRefusal::NoLogoutZone)
    } else if in_fight {
        Some(TempleScrollRefusal::InFight)
    } else if pz_blocked {
        Some(TempleScrollRefusal::PzBlocked)
    } else {
        None
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    /// One scroll use. The caller holds the spell lane permit and has admitted the command on
    /// the actor's item-use gate.
    pub(super) async fn use_temple_scroll(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        fence: CurrentCharacterItemFence,
        inventory: &ItemUseInventory,
        entry: &BackpackEntry,
    ) -> Result<UseDisposition, UseDisposition> {
        let session = command.game_session_id;
        if !matches!(fence.runtime_scope, RuntimeScopeRefV1::Channel { .. }) {
            return Err(UseDisposition::Rejected);
        }
        // An unreadable fence or PvP row proves no fight deadline (LOGOUT_INTENT's own rule).
        if !self.refresh_pvp_locks(actor, session).await {
            return Err(UseDisposition::Rejected);
        }
        let Some(zones) = self.logout_zones(actor).await else {
            return Err(UseDisposition::NotPossible);
        };
        let no_logout = self.no_logout_tile(&mut *self.runtime.lock().await, actor);
        let fact = self.combat_lock_fact(actor, session).await;
        let in_fight = super::super::logout::in_fight_logout_retry(
            fact.pvp == super::super::combat_lock_fact::PvpLockSource::LiveBlocked,
            fact.pve_in_fight_until.map(|until| until.get()),
            fact.sampled_at().get(),
            zones.protection_zone,
        )
        .is_some();
        let pz_blocked = super::super::pvp_live::pz_blocked(
            &*self.states.spell_states.lock().await,
            actor,
            session,
            self.owner_now(),
        );
        if let Some(refusal) = temple_scroll_refusal(no_logout, in_fight, pz_blocked) {
            return Err(refusal.disposition());
        }
        let temple = self
            .home_temple(session)
            .await
            .ok_or(UseDisposition::NotPossible)?;
        // The closed doors are read before the runtime guard, as the paced movement owner does.
        let (bundle_doors, legacy);
        let closed: &dyn CellBlocker = if let Some(map) = self.bundle_map() {
            bundle_doors = crate::map::floor::BundleDoors(&map.facts);
            &bundle_doors
        } else {
            legacy = self.door.lock().await.blocking_cells().clone();
            &legacy
        };
        let source = entry.item.item_instance_id;
        let ids = crate::operator_files::uuid_v7()
            .and_then(|transaction| Ok((transaction, crate::operator_files::uuid_v7()?)));
        let (Ok((transaction_id, event_id)), Ok(occurred_at_unix_ms)) = (ids, unix_ms()) else {
            return Err(UseDisposition::Rejected);
        };
        let request = ItemUseRequest {
            kind: ItemUseKind::TempleScroll,
            command_id: command.command_id,
            content_revision: inventory.content_revision.clone(),
            ruleset_revision: inventory.ruleset_revision.clone(),
            sim_revision: entry_chest::SIM_REVISION.to_owned(),
            transaction_id,
            event_id,
            occurred_at_unix_ms,
            server_build_id: SERVER_BUILD_ID.to_owned(),
            source_item_instance_id: source,
            source_definition: entry.item.definition.clone(),
            source_quantity_before: entry.item.quantity,
            flask: ItemUseFlaskPlan::None,
        };
        // PREPARE: the actor and the landing, then the gate (one use in flight).
        {
            let mut runtime = self.runtime.lock().await;
            let mut states = self.states.spell_states.lock().await;
            if states.is_dead(actor) || !states.spell_idle(&runtime, actor, session) {
                return Err(UseDisposition::Rejected);
            }
            if !temple_landing(
                &mut runtime,
                &states,
                self.movement_cells(),
                actor,
                session,
                temple,
                closed,
                self.owner_now(),
                false,
            ) {
                return Err(UseDisposition::NotPossible);
            }
            let gate = states
                .item_uses
                .gate(&runtime, actor, session)
                .ok_or(UseDisposition::Rejected)?;
            if gate.unspendable.contains(&source) {
                return Err(UseDisposition::StaleState);
            }
            gate.start(ItemUseKind::TempleScroll, self.owner_ms())?;
        }
        // The move runs inside the burn's transaction, before COMMIT (the writer calls it).
        let landing = ScrollLanding {
            runtime: &*self.runtime,
            spell_states: &self.states.spell_states,
            cells: self.movement_cells(),
            actor,
            session,
            temple,
            closed,
            now: self.owner_now(),
        };
        let commit = self.root.commit_temple_scroll_use(
            self.character,
            self.holder,
            fence,
            request,
            &landing,
        );
        let bound = Duration::from_millis(ITEMUSE0_RL_04_AMBIGUOUS_MS);
        let (disposition, ambiguous) = match tokio::time::timeout(bound, commit).await {
            // Every landing was taken at commit, or the scroll's deadline passed before the
            // Item deadline owner removed it: the burn rolled back and nothing was written.
            Ok(Ok(TempleScrollOutcome::NoLanding | TempleScrollOutcome::Expired)) => {
                (UseDisposition::NotPossible, false)
            }
            Ok(Ok(TempleScrollOutcome::Used | TempleScrollOutcome::AlreadyCommitted)) => {
                (UseDisposition::Committed, false)
            }
            Ok(Err(ItemUseError::StaleState)) => (UseDisposition::StaleState, false),
            Ok(Err(ItemUseError::Unavailable(_))) | Err(_) => (UseDisposition::Rejected, true),
            Ok(Err(_)) => (UseDisposition::Rejected, false),
        };
        // A use moved the actor before its COMMIT; a replay applied its effect then.
        let runtime = self.runtime.lock().await;
        let mut states = self.states.spell_states.lock().await;
        let reserved = [source];
        let unspendable: &[[u8; 16]] = if ambiguous { &reserved } else { &[] };
        if let Some(gate) = states.item_uses.gate(&runtime, actor, session) {
            gate.settle(command.command_id, disposition, unspendable);
        }
        Ok(disposition)
    }
}

/// The burn's move to the temple: under the runtime and spell-state guards, in their
/// established order, the first free landing of the temple and its ring.
struct ScrollLanding<'s> {
    runtime: &'s tokio::sync::Mutex<ChannelRuntimeV1>,
    spell_states: &'s tokio::sync::Mutex<ChannelSpellStates>,
    cells: &'s NativeEntryMovementCells,
    actor: ExactActorRef,
    session: GameSessionId,
    temple: MovementLocalPosition,
    closed: &'s dyn CellBlocker,
    now: SemanticTimeMicros,
}

impl TempleLanding for ScrollLanding<'_> {
    fn land(&self) -> Pin<Box<dyn Future<Output = bool> + Send + '_>> {
        Box::pin(async move {
            let mut runtime = self.runtime.lock().await;
            let states = self.spell_states.lock().await;
            !states.is_dead(self.actor)
                && temple_landing(
                    &mut runtime,
                    &states,
                    self.cells,
                    self.actor,
                    self.session,
                    self.temple,
                    self.closed,
                    self.now,
                    true,
                )
        })
    }
}

/// The landings in order: the temple cell, then [`LANDING_RING`] on the temple's floor.
pub(super) fn landing_candidates(temple: MovementLocalPosition) -> [MovementLocalPosition; 9] {
    let mut candidates = [temple; 9];
    for (candidate, (dx, dy)) in candidates.iter_mut().skip(1).zip(LANDING_RING) {
        candidate.x = temple.x.saturating_add(dx);
        candidate.y = temple.y.saturating_add(dy);
    }
    candidates
}

/// The callback teleport proof, the one the citizenship and quest teleports use, from the
/// actor's current position to the first landing that admits it (walkable, open, unoccupied,
/// not closed by the PvP block); `commit` applies it. `false` when no landing admits one.
#[allow(
    clippy::too_many_arguments,
    reason = "binds the exact actor, session, landing, closed doors, clock and the commit step"
)]
fn temple_landing(
    runtime: &mut ChannelRuntimeV1,
    states: &ChannelSpellStates,
    cells: &NativeEntryMovementCells,
    actor: ExactActorRef,
    session: GameSessionId,
    temple: MovementLocalPosition,
    closed: &dyn CellBlocker,
    now: SemanticTimeMicros,
    commit: bool,
) -> bool {
    let Ok(entered) = runtime.read_actor_position(actor) else {
        return false;
    };
    let blocked = super::super::pvp_live::pz_blocked(states, actor, session, now);
    for landing in landing_candidates(temple) {
        if let Ok(proof) = crate::map::floor::prepare_quest_callback_teleport_facing(
            runtime, cells, actor, session, entered, landing, blocked, closed, None,
        ) {
            return !commit || runtime.commit_source_step(proof).is_ok();
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn temple_scroll_refusals_follow_the_contract_order() {
        assert_eq!(temple_scroll_refusal(Some(false), false, false), None);
        assert_eq!(
            temple_scroll_refusal(Some(true), true, true),
            Some(TempleScrollRefusal::NoLogoutZone)
        );
        assert_eq!(
            temple_scroll_refusal(None, false, false),
            Some(TempleScrollRefusal::NoLogoutZone)
        );
        assert_eq!(
            temple_scroll_refusal(Some(false), true, true),
            Some(TempleScrollRefusal::InFight)
        );
        assert_eq!(
            temple_scroll_refusal(Some(false), false, true),
            Some(TempleScrollRefusal::PzBlocked)
        );
    }

    #[test]
    fn temple_landings_start_at_the_temple_then_its_ring() {
        let temple = MovementLocalPosition {
            x: 10,
            y: 20,
            floor: 7,
        };
        let candidates = landing_candidates(temple);
        assert_eq!(candidates[0], temple);
        assert_eq!(
            candidates[1],
            MovementLocalPosition {
                x: 9,
                y: 19,
                floor: 7
            }
        );
        assert_eq!(
            candidates[8],
            MovementLocalPosition {
                x: 11,
                y: 21,
                floor: 7
            }
        );
        let mut seen = candidates.to_vec();
        seen.sort_by_key(|p| (p.x, p.y));
        seen.dedup();
        assert_eq!(seen.len(), 9);
        assert!(
            candidates.iter().all(|p| p.floor == 7
                && (p.x - temple.x).abs() <= 1
                && (p.y - temple.y).abs() <= 1)
        );
    }

    #[test]
    fn temple_scroll_refusals_use_the_existing_dispositions() {
        assert_eq!(
            TempleScrollRefusal::NoLogoutZone.disposition(),
            UseDisposition::NotPossible
        );
        assert_eq!(
            TempleScrollRefusal::InFight.disposition(),
            UseDisposition::NotPossible
        );
        assert_eq!(
            TempleScrollRefusal::PzBlocked.disposition(),
            UseDisposition::PzBlocked
        );
    }
}
