//! Server owner clock; independent of client packets and selected capabilities.
use super::timed_item::*;
use crate::durability::item_timed_runtime::{TimedOperation, TimedRequest};
use crate::foundation::GameSessionId;
use sha2::{Digest, Sha256};
use std::collections::HashSet;

impl super::ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn stop_timed_actor(
        &self,
        actor: crate::foundation::ExactActorRef,
        session: GameSessionId,
    ) -> bool {
        if !self
            .states
            .timed_items
            .sessions
            .lock()
            .await
            .get(&session)
            .is_some_and(|clock| !clock.items.is_empty())
        {
            return true;
        }
        let Some(_permit) = self.spell_lane_permit().await else {
            return false;
        };
        self.checkpoint_timed_equipment(actor, session, true).await
    }

    pub(super) async fn tick_timed_items(&self) {
        let now = u64::try_from(self.clock_origin.elapsed().as_millis()).unwrap_or(u64::MAX);
        use std::sync::atomic::Ordering;
        let next = &self.states.timed_items.next_tick_ms;
        if now < next.load(Ordering::Relaxed) {
            return;
        }
        next.store(now.saturating_add(1_000), Ordering::Relaxed);

        let sessions = {
            let runtime = self.runtime.lock().await;
            let Ok(census) = runtime.positioned_melee_census() else {
                return;
            };
            census
                .into_iter()
                .filter_map(|(actor, _, session)| session.map(|session| (actor, session)))
                .collect::<Vec<_>>()
        };
        let present: HashSet<GameSessionId> =
            sessions.iter().map(|(_, session)| *session).collect();
        self.states
            .timed_items
            .sessions
            .lock()
            .await
            .retain(|session, _| present.contains(session));
        for (actor, session) in sessions {
            let Some(_permit) = self.spell_lane_permit().await else {
                return;
            };
            self.checkpoint_timed_equipment(actor, session, false).await;
        }
    }

    /// Caller holds the spell lane. Pending ambiguous requests retain their identity and
    /// original elapsed time until replay confirms the write; a later pass never debits twice.
    pub(super) async fn checkpoint_timed_equipment(
        &self,
        actor: crate::foundation::ExactActorRef,
        session: GameSessionId,
        force: bool,
    ) -> bool {
        let Ok(Some(fence)) = self.current_quest_fence(session).await else {
            return false;
        };
        let native = {
            let runtime = self.runtime.lock().await;
            if runtime.owner_fence().is_err()
                || runtime.player_control_facts(actor, session).is_err()
                || super::item_ref_admission::pinned_index(&runtime, self.active_generation)
                    .is_none()
            {
                return false;
            }
            let Some(native) = self
                .active_generation
                .and_then(|active| active.native_gameplay())
            else {
                return false;
            };
            native
        };
        // A retained actor keeps its item clock during reconnect grace. The item reader
        // proves the current active/reconnectable fence independently of a client view.
        let item_fence = fence.item_fence();
        let equipment = match self
            .root
            .read_movement_equipment(
                self.character,
                self.holder,
                &item_fence,
                native.source_digest(),
            )
            .await
        {
            Ok(read)
                if read.fence() == &item_fence
                    && read.equipment().content_digest == native.source_digest() =>
            {
                read.equipment().clone()
            }
            _ => return false,
        };
        let now = u64::try_from(self.clock_origin.elapsed().as_millis()).unwrap_or(u64::MAX);
        let Some(mut active_mana_shield) =
            super::equipment_move::equipment_effects_owner::equipped_mana_shield(
                &equipment, native,
            )
        else {
            return false;
        };
        let mut clocks = self.states.timed_items.sessions.lock().await;
        let clock = clocks.entry(session).or_default();
        let held: HashSet<_> = equipment
            .items
            .iter()
            .map(|item| item.item_instance_id)
            .collect();
        // An ambiguous expiry may already have burned an equipped item. Reconcile
        // its original receipt before pruning custody; absence from equipment alone
        // cannot prove whether the value-moving transaction committed.
        let mut retired_equipment = false;
        for (id, lane) in &clock.items {
            if !held.contains(id)
                && let Some((request, _)) = &lane.pending
            {
                match self
                    .root
                    .commit_timed_action(self.character, self.holder, fence, request.clone())
                    .await
                {
                    Ok(_) | Err(crate::durability::item_timed_state::TimedWriteError::NotHeld) => {}
                    Err(_) => return false,
                }
                retired_equipment = true;
            }
        }
        clock.items.retain(|id, _| held.contains(id));
        for item in equipment.items {
            let Some((definition, target)) = facts(native, &item.definition) else {
                continue;
            };
            if definition.full.remaining_ms().is_none() {
                continue;
            }
            let lane = clock
                .items
                .entry(item.item_instance_id)
                .or_insert(ItemClock {
                    at_ms: now,
                    pending: None,
                });
            if lane.pending.is_none() {
                let elapsed = now.saturating_sub(lane.at_ms);
                if elapsed == 0 {
                    continue;
                }
                let Ok(row) = self
                    .root
                    .read_timed_row(self.character, self.holder, fence, item.item_instance_id)
                    .await
                else {
                    return false;
                };
                if row.is_some_and(|row| row.values.is_none()) {
                    continue;
                }
                let remaining = row
                    .and_then(|row| row.values)
                    .unwrap_or(definition.full)
                    .remaining_ms()
                    .unwrap_or(0);
                if !force && elapsed < 60_000 && elapsed < remaining {
                    continue;
                }

                clock.serial = match clock.serial.checked_add(1) {
                    Some(n) => n,
                    None => return false,
                };
                // The owner incarnation, session and item are independent of client command ids.
                let mut hash = Sha256::new();
                hash.update(b"oteryn:timed-owner-clock:v1");
                hash.update(session.as_bytes());
                hash.update(item.item_instance_id);
                hash.update(self.holder.fact().registration_revision().to_be_bytes());
                hash.update(fence.connection_generation.get().to_be_bytes());
                hash.update(row.map_or(0, |row| row.revision).to_be_bytes());
                hash.update(clock.serial.to_be_bytes());
                let binding: [u8; 32] = hash.finalize().into();
                let mut id = [0; 16];
                id.copy_from_slice(&binding[..16]);
                id[6] = (id[6] & 15) | 0x70;
                id[8] = (id[8] & 63) | 0x80;
                let mut event = id;
                event[15] ^= 1;
                lane.pending = Some((
                    TimedRequest {
                        transaction_id: id,
                        content_digest: native.source_digest(),
                        event_id: event,
                        binding,
                        item: item.item_instance_id,
                        expected_revision: row.map_or(0, |row| row.revision),
                        definition,
                        target,
                        operation: TimedOperation::Advance(elapsed.min(604_800_000)),
                        occurred_at_unix_ms: super::npc_trade_command::unix_ms_now(),
                    },
                    now,
                ));
            }
            let Some((request, at)) = lane.pending.clone() else {
                continue;
            };
            match self
                .root
                .commit_timed_action(self.character, self.holder, fence, request)
                .await
            {
                Ok(result)
                    if result.disposition
                        == oteryn_protocol_oteryn::timed_item::TimedDisposition::Ok =>
                {
                    retired_equipment |= result.retired;
                    lane.at_ms = at;
                    lane.pending = None;
                }
                // A distinct command changed the revision. No old elapsed time may be applied
                // to its successor lifetime; reload from that committed row.
                Ok(_) => {
                    lane.at_ms = now;
                    lane.pending = None;
                }
                Err(_) => return false,
            }
        }
        drop(clocks);
        // Expiry may burn the ring without a client move. Re-read committed custody so
        // its contribution cannot outlive the timed budget. An ordinary checkpoint uses
        // its initial read, also repairing a previous expiry's failed acknowledgement/read.
        if retired_equipment {
            let Some(equipment) =
                super::equipment_move::read_current_equipment(self, session).await
            else {
                return false;
            };
            let Some(active) = super::equipment_move::equipment_effects_owner::equipped_mana_shield(
                &equipment, native,
            ) else {
                return false;
            };
            active_mana_shield = active;
        }
        let runtime = self.runtime.lock().await;
        let mut states = self.states.spell_states.lock().await;
        if let Some(state) = states.get_mut(&runtime, actor, session) {
            state.install_equipped_mana_shield(active_mana_shield);
        }
        true
    }
}
