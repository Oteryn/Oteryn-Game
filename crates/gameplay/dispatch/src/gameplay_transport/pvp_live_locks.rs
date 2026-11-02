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
/// Whether this session already carries a cached lock entry (from a PvP hit or a durable refresh).
pub(in crate::gameplay_transport) fn tracked(
    states: &ChannelSpellStates,
    actor: ExactActorRef,
    session: crate::foundation::GameSessionId,
) -> bool {
    states
        .pvp_locks
        .iter()
        .any(|(a, s, _)| *a == actor && *s == session)
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
        // NODEBOOT-1: both reads go through the shared holder that background passes also
        // take, so a busy holder is retried briefly instead of refusing the command.
        use crate::gameplay_transport::connection::AdmissionRefusal::{Rejected, Unavailable};
        let Ok((fence, stored, house_scope)) =
            crate::gameplay_transport::AdmissionRetryBudget::for_command(self.root)
                .retry_unavailable(async || {
                    let fence = self
                        .current_quest_fence(session)
                        .await
                        .map_err(|()| Unavailable)?
                        .ok_or(Rejected)?;
                    // Activities also use Instance scopes. Only a native House
                    // actor may select the named-House custody reader.
                    let house_scope = self
                        .runtime
                        .lock()
                        .await
                        .house_actor_state(actor, session)
                        .is_ok();
                    let stored = if house_scope {
                        self.root
                            .read_house_pvp_state(
                                self.character,
                                self.holder,
                                fence,
                                self.channel_id,
                            )
                            .await
                    } else {
                        self.root
                            .read_pvp_state(self.character, self.holder, fence)
                            .await
                    }
                    .map_err(|_| Unavailable)?;
                    Ok((fence, stored, house_scope))
                })
                .await
        else {
            return false;
        };
        let runtime = self.runtime.lock().await;
        if house_scope {
            let Ok(house) = runtime.house_actor_state(actor, session) else {
                return false;
            };
            if fence.game_session_id != session
                || house.scope != fence.runtime_scope
                || house.generation != fence.scope_ownership_generation
                || house.lease.character_id().as_bytes() != fence.character_id.as_bytes()
                || house.lease.generation() != fence.character_lease_generation
            {
                return false;
            }
        } else if runtime.player_control_facts(actor, session).is_err() {
            return false;
        }
        let mut states = self.states.spell_states.lock().await;
        states.pvp_locks.retain(|(a, s, _)| {
            runtime.player_control_facts(*a, *s).is_ok()
                || runtime.house_player_control_facts(*a, *s).is_ok()
        });
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
