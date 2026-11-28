//! Server owner clock; independent of client packets and selected capabilities.
use super::timed_item::*;
use crate::durability::character_equipment::EquipmentSnapshot;
use crate::durability::item_timed_runtime::{TimedOperation, TimedRequest};
use crate::durability::item_transfer::CurrentCharacterItemFence;
use crate::foundation::{ExactActorRef, GameSessionId};
use sha2::{Digest, Sha256};
use std::collections::HashSet;

#[cfg(test)]
tokio::task_local! {
    pub(super) static RETIRED_EQUIPMENT_READ: (
        std::sync::Arc<tokio::sync::Notify>,
        std::sync::Arc<tokio::sync::Notify>,
    );
}

impl super::ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn stop_timed_actor(
        &self,
        actor: crate::foundation::ExactActorRef,
        session: GameSessionId,
    ) -> bool {
        let has_timed_items = self
            .states
            .timed_items
            .sessions
            .lock()
            .await
            .get(&session)
            .is_some_and(|clock| !clock.items.is_empty() || clock.refresh_equipment);
        let has_imbuement_clock = self
            .states
            .timed_items
            .imbuements
            .lock()
            .await
            .contains_key(&session);
        if !has_timed_items && !has_imbuement_clock {
            return true;
        }
        let Some(_permit) = self.spell_lane_permit().await else {
            return false;
        };
        let settled = self.checkpoint_timed_equipment(actor, session, true).await;
        if settled {
            self.states
                .timed_items
                .imbuements
                .lock()
                .await
                .remove(&session);
        }
        settled
    }

    /// Both activity lanes are already held. Settle elapsed time and resolve any
    /// pending receipts under the source session fence before that fence is retired.
    pub(super) async fn settle_activity_equipment(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> bool {
        let tracked_items = self
            .states
            .timed_items
            .sessions
            .lock()
            .await
            .get(&session)
            .is_some_and(|clock| !clock.items.is_empty() || clock.refresh_equipment);
        let tracked_imbuements = self
            .states
            .timed_items
            .imbuements
            .lock()
            .await
            .contains_key(&session);
        if tracked_items || tracked_imbuements {
            self.checkpoint_timed_equipment(actor, session, true).await
        } else {
            true
        }
    }

    pub(super) async fn transfer_activity_equipment_clocks(
        &self,
        destination: &Self,
        old: GameSessionId,
        new: GameSessionId,
    ) {
        let mut source_items = self.states.timed_items.sessions.lock().await;
        let mut destination_items = destination.states.timed_items.sessions.lock().await;
        if let Some(clock) = source_items.remove(&old) {
            destination_items.insert(new, clock);
        }
        drop(destination_items);
        drop(source_items);
        let mut source_imbuements = self.states.timed_items.imbuements.lock().await;
        let mut destination_imbuements = destination.states.timed_items.imbuements.lock().await;
        if let Some(clock) = source_imbuements.remove(&old) {
            destination_imbuements.insert(new, clock);
        }
    }

    pub(super) async fn tick_timed_items(&self) {
        let now = u64::try_from(self.clock_origin.elapsed().as_millis()).unwrap_or(u64::MAX);
        use std::sync::atomic::Ordering;
        let next = &self.states.timed_items.next_tick_ms;
        if now < next.load(Ordering::Relaxed) {
            return;
        }
        next.store(now.saturating_add(1_000), Ordering::Relaxed);

        // Prune against a census taken under the same lane that moves a House clock.
        let Some(census_permit) = self.spell_lane_permit().await else {
            return;
        };
        let mut sessions = {
            let runtime = self.runtime.lock().await;
            let Ok(census) = runtime.positioned_melee_census() else {
                return;
            };
            let spells = self.states.spell_states.lock().await;
            let residents = spells.actors.iter().filter_map(|(actor, session, _)| {
                runtime.house_actor_state(*actor, *session).ok()?;
                Some((*actor, *session, true))
            });
            census
                .into_iter()
                .filter_map(|(actor, _, session)| session.map(|session| (actor, session, false)))
                .chain(residents)
                .collect::<Vec<_>>()
        };
        let present: HashSet<GameSessionId> =
            sessions.iter().map(|(_, session, _)| *session).collect();
        self.states
            .timed_items
            .sessions
            .lock()
            .await
            .retain(|session, _| present.contains(session));
        self.states
            .timed_items
            .imbuements
            .lock()
            .await
            .retain(|session, _| present.contains(session));
        let converters: Vec<GameSessionId> = {
            let mut active = self.states.timed_items.magic_converters.lock().await;
            active.retain(|session, _| present.contains(session));
            active.keys().copied().collect()
        };
        drop(census_permit);
        // Stable order, one cursor across equipment, quests and converters as
        // well as the two scope projections. A refused unit cannot restart a prefix.
        sessions.sort_by_key(|(_, session, _)| *session.as_bytes());
        let mut converters = converters;
        converters.sort_by_key(|session| *session.as_bytes());
        let count = 2 + sessions.len() * 2 + converters.len();
        let start = self.background_start("item-clock", count);
        for index in (start..count).chain(0..start) {
            self.background_unit("item-clock", index + 1, async {
                match index {
                    0 => self.drain_corpse_deadlines().await,
                    1 => self.refresh_terrain_tools().await,
                    index if index < 2 + sessions.len() * 2 => {
                        let (actor, session, house) = sessions[(index - 2) / 2];
                        if house && !(index - 2).is_multiple_of(2) {
                            return;
                        }
                        let equipment = (index - 2).is_multiple_of(2);
                        if equipment && !self.root.await_scheduled_holder_turn().await {
                            return;
                        }
                        let Some(_permit) = self.spell_lane_permit().await else {
                            return;
                        };
                        if equipment {
                            let checkpoint = self.checkpoint_timed_equipment(actor, session, false);
                            self.root.with_periodic_lock_budget(checkpoint).await;
                        } else {
                            self.root
                                .with_periodic_lock_budget(self.drain_quest_timers(actor, session))
                                .await;
                        }
                    }
                    index => {
                        let session = converters[index - 2 - sessions.len() * 2];
                        if !self.tick_magic_converter(session).await {
                            self.states
                                .timed_items
                                .magic_converters
                                .lock()
                                .await
                                .remove(&session);
                        }
                    }
                }
            })
            .await;
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
        self.root
            .with_background_operation(self.checkpoint_timed_equipment_inner(actor, session, force))
            .await
    }

    async fn checkpoint_timed_equipment_inner(
        &self,
        actor: crate::foundation::ExactActorRef,
        session: GameSessionId,
        force: bool,
    ) -> bool {
        let Ok(Some(fence)) = self.current_quest_fence(session).await else {
            return false;
        };
        let (native, house) = {
            let runtime = self.runtime.lock().await;
            let house = runtime.house_actor_state(actor, session).is_ok();
            if (!house
                && (runtime.owner_fence().is_err()
                    || runtime.player_control_facts(actor, session).is_err()))
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
            (native, house)
        };
        let house_origin = house.then(|| *self.channel_id.as_bytes());
        // A retained actor keeps its item clock during reconnect grace. The item reader
        // proves the current active/reconnectable fence independently of a client view.
        let item_fence = fence.item_fence();
        let Some(equipment) = self
            .read_clock_equipment(actor, item_fence, native.source_digest(), house)
            .await
        else {
            return false;
        };
        if !self
            .checkpoint_imbuement_equipment(actor, session, fence)
            .await
        {
            // IMBUE-FORGE0 §4.3: an unknown checkpoint suspends installed effects.
            // Keep only the independently fenced equipment projection until replay succeeds.
            use super::equipment_move::equipment_effects_owner as effects;
            if let Some(base) = (|| {
                Some((
                    effects::equipment_effects(&equipment, native)?,
                    effects::equipped_armor(&equipment, native)?,
                    effects::equipped_defence(&equipment, native)?,
                    effects::equipped_mana_shield(&equipment, native)?,
                    effects::equipped_offence(&equipment, native)?,
                    equipment.clone(),
                ))
            })() {
                let runtime = self.runtime.lock().await;
                if house {
                    let _ = self
                        .install_house_admission_suppressions(
                            &runtime,
                            actor,
                            session,
                            item_fence,
                            Some(base),
                        )
                        .await;
                } else {
                    let _ = self
                        .install_admission_suppressions(&runtime, actor, session, Some(base))
                        .await;
                }
            }
            return false;
        }
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
        let mut retired_equipment =
            clock.refresh_equipment || (house && clock.items.keys().any(|id| !held.contains(id)));
        for (id, lane) in &clock.items {
            if !held.contains(id)
                && let Some((request, _)) = &lane.pending
            {
                if !self.states.player_trades.physical(session, None) {
                    return false;
                }
                match self
                    .root
                    .commit_timed_action_in_scope(
                        self.character,
                        self.holder,
                        fence,
                        request.clone(),
                        house_origin,
                    )
                    .await
                {
                    Ok(_) | Err(crate::durability::item_timed_state::TimedWriteError::NotHeld) => {}
                    Err(_) => return false,
                }
                retired_equipment = true;
            }
        }
        clock.refresh_equipment = retired_equipment;
        if !house {
            clock.items.retain(|id, _| held.contains(id));
        }
        let mut proficiency_equipment = equipment.clone();
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
                    .read_timed_row_in_scope(
                        self.character,
                        self.holder,
                        fence,
                        item.item_instance_id,
                        house_origin,
                    )
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
                .commit_timed_action_in_scope(
                    self.character,
                    self.holder,
                    fence,
                    request,
                    house_origin,
                )
                .await
            {
                Ok(result)
                    if result.disposition
                        == oteryn_protocol_oteryn::timed_item::TimedDisposition::Ok =>
                {
                    retired_equipment |= result.retired;
                    clock.refresh_equipment |= result.retired;
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
        if house {
            // Expiry can change the resident's equipment and every derived effect.
            if !retired_equipment {
                return true;
            }
            if self
                .refresh_clock_equipment_effects(actor, session, item_fence, true)
                .await
                != Ok(true)
            {
                return false;
            }
            // Retain a missing item's clock until publication succeeds so a
            // transient post-expiry read failure can be repaired on a later tick.
            if let Some(clock) = self
                .states
                .timed_items
                .sessions
                .lock()
                .await
                .get_mut(&session)
            {
                clock.items.retain(|id, _| held.contains(id));
                clock.refresh_equipment = false;
            }
            return true;
        }
        // Expiry may burn the ring without a client move. Re-read committed custody so
        // its contribution cannot outlive the timed budget. An ordinary checkpoint uses
        // its initial read, also repairing a previous expiry's failed acknowledgement/read.
        let mut retired_effects = None;
        if retired_equipment {
            #[cfg(test)]
            if let Ok((entered, proceed)) = RETIRED_EQUIPMENT_READ.try_with(Clone::clone) {
                entered.notify_one();
                proceed.notified().await;
            }
            let retry = super::AdmissionRetryBudget::for_command(self.root);
            let Some(equipment) = super::equipment_move::read_equipment_within(
                self,
                &retry,
                &item_fence,
                native.source_digest(),
            )
            .await
            else {
                return false;
            };
            let Some(active) = super::equipment_move::equipment_effects_owner::equipped_mana_shield(
                &equipment, native,
            ) else {
                return false;
            };
            // I05: a burned dwarven ring's suppressdrunk ends with it, like the mana shield.
            let Some(effects) = super::equipment_move::equipment_effects_owner::equipment_effects(
                &equipment, native,
            ) else {
                return false;
            };
            active_mana_shield = active;
            retired_effects = Some(effects);
            proficiency_equipment = equipment;
        }
        let Some(invisible) = super::equipment_move::equipment_effects_owner::equipped_invisibility(
            &proficiency_equipment,
            native,
        ) else {
            return false;
        };
        let runtime = self.runtime.lock().await;
        if self
            .install_proficiency_equipment(&runtime, actor, session, &proficiency_equipment)
            .await
            .is_none()
        {
            return false;
        }
        let mut states = self.states.spell_states.lock().await;
        let Some(state) = states.get_mut(&runtime, actor, session) else {
            return !retired_equipment;
        };
        {
            state.install_equipped_mana_shield(active_mana_shield);
            state.install_equipped_invisibility(invisible);
            if let Some(effects) = retired_effects {
                super::equipment_move::equipment_effects_owner::install_suppressions(
                    state, effects,
                );
            }
        }
        drop(states);
        drop(runtime);
        if retired_equipment
            && let Some(clock) = self
                .states
                .timed_items
                .sessions
                .lock()
                .await
                .get_mut(&session)
        {
            clock.refresh_equipment = false;
        }
        true
    }

    async fn read_clock_equipment(
        &self,
        actor: ExactActorRef,
        fence: CurrentCharacterItemFence,
        digest: [u8; 32],
        house: bool,
    ) -> Option<EquipmentSnapshot> {
        if house {
            return super::equipment_move::read_house_clock_equipment(
                self,
                actor,
                fence.game_session_id,
                fence,
            )
            .await
            .filter(|equipment| equipment.content_digest == digest);
        }
        let read = self
            .root
            .read_movement_equipment(self.character, self.holder, &fence, digest)
            .await
            .ok()?;
        (read.fence() == &fence && read.equipment().content_digest == digest)
            .then(|| read.equipment().clone())
    }

    pub(super) async fn refresh_clock_equipment_effects(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        fence: CurrentCharacterItemFence,
        house: bool,
    ) -> Result<bool, ()> {
        let effects = if house {
            self.read_house_equipment_effects(actor, session, fence, true)
                .await?
        } else {
            self.read_fenced_equipment_effects(fence).await?
        };
        let runtime = self.runtime.lock().await;
        let installed = if house {
            self.install_house_admission_suppressions(&runtime, actor, session, fence, effects)
                .await
        } else {
            self.install_admission_suppressions(&runtime, actor, session, effects)
                .await
        };
        Ok(installed.is_some())
    }
}
