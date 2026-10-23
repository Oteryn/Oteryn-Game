//! Live combat locks and their fenced readmission refresh.
use super::*;

pub(in crate::gameplay_transport) fn lock_mut(
    states: &mut ChannelSpellStates,
    actor: ExactActorRef,
    session: crate::foundation::GameSessionId,
) -> &mut oteryn_gameplay_combat_rules::pvp::CombatLock {
    let index = match states
        .pvp_locks
        .iter()
        .position(|(a, s, _)| *a == actor && *s == session)
    {
        Some(i) => i,
        None => {
            states.pvp_locks.push((actor, session, Default::default()));
            states.pvp_locks.len() - 1
        }
    };
    &mut states.pvp_locks[index].2
}
pub(in crate::gameplay_transport) fn logout_blocked(
    states: &ChannelSpellStates,
    actor: ExactActorRef,
    session: crate::foundation::GameSessionId,
    now: oteryn_simulation_determinism::SemanticTimeMicros,
) -> bool {
    !states.is_dead(actor)
        && states
            .pvp_locks
            .iter()
            .any(|(a, s, l)| *a == actor && *s == session && l.blocks_logout(now, None))
}
pub(in crate::gameplay_transport) fn pz_blocked(
    states: &ChannelSpellStates,
    actor: ExactActorRef,
    session: crate::foundation::GameSessionId,
    now: oteryn_simulation_determinism::SemanticTimeMicros,
) -> bool {
    states
        .pvp_locks
        .iter()
        .any(|(a, s, l)| *a == actor && *s == session && l.blocks_pz_entry(now))
}
impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn refresh_pvp_locks(
        &self,
        actor: ExactActorRef,
        session: crate::foundation::GameSessionId,
    ) -> bool {
        let Ok(Some(fence)) = self.current_quest_fence(session).await else {
            return false;
        };
        let Ok(stored) = self
            .root
            .read_pvp_state(self.character, self.holder, fence)
            .await
        else {
            return false;
        };
        let runtime = self.runtime.lock().await;
        if runtime.player_control_facts(actor, session).is_err() {
            return false;
        }
        let mut states = self.states.spell_states.lock().await;
        states
            .pvp_locks
            .retain(|(a, s, _)| runtime.player_control_facts(*a, *s).is_ok());
        lock_mut(&mut states, actor, session);
        if let Some(stored) = stored {
            let now = self.owner_now().get();
            let deadline = |left: crate::durability::pvp_state::PvpRemainingMicros| {
                (left.get() > 0).then(|| {
                    oteryn_simulation_determinism::SemanticTimeMicros::from_micros(
                        now.saturating_add(left.get()),
                    )
                })
            };
            lock_mut(&mut states, actor, session).restore(
                deadline(stored.readmission_remaining.logout),
                deadline(stored.readmission_remaining.pz),
                deadline(stored.readmission_remaining.kill),
            );
        }
        true
    }
}
pub(in crate::gameplay_transport) struct PvpBlocker<'a> {
    pub inner: &'a dyn crate::map::floor::CellBlocker,
    pub cells: &'a crate::content::NativeEntryMovementCells,
    pub blocked: bool,
}
impl crate::map::floor::CellBlocker for PvpBlocker<'_> {
    fn blocks(&self, cell: crate::content::LogicalCell) -> bool {
        self.inner.blocks(cell)
            || (self.blocked
                && self
                    .cells
                    .spell_tiles()
                    .lookup(self.cells.scope(), cell)
                    .map_or(true, |t| t.flags().protection_zone))
    }
}
