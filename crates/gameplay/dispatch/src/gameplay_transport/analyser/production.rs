//! The owner-facing producer and connection reads. All live reads use their runtime guard.
use super::*;
use crate::gameplay_transport::{ComposedFreshAdmission, connection::AdmittedSession};

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn analyser_snapshot(
        &self,
        admitted: AdmittedSession,
    ) -> Option<bool> {
        let actor = admitted.runtime_actor?;
        let runtime = self.runtime.lock().await;
        if runtime
            .player_control_facts(actor, admitted.game_session_id)
            .is_err()
            && runtime
                .house_player_control_facts(actor, admitted.game_session_id)
                .is_err()
        {
            return None;
        }
        let mut spells = self.states.spell_states.lock().await;
        spells.analyser = Arc::clone(&self.states.analyser);
        self.states.analyser.snapshot_attributed(
            actor,
            admitted.game_session_id,
            admitted.continuity.connection_generation,
            self.active_generation
                .and_then(|g| g.native_gameplay())
                .and_then(|n| n.item_index())
                .cloned(),
            admitted
                .continuity
                .selected_capabilities
                .contains(oteryn_protocol_oteryn::analyser::CAPABILITY_ANALYSER_HIT_ATTRIBUTION_V1),
        )
    }

    pub(in crate::gameplay_transport) async fn analyser_next(
        &self,
        admitted: AdmittedSession,
    ) -> Option<Vec<u8>> {
        let actor = admitted.runtime_actor?;
        let runtime = self.runtime.lock().await;
        if runtime
            .player_control_facts(actor, admitted.game_session_id)
            .is_err()
            && runtime
                .house_player_control_facts(actor, admitted.game_session_id)
                .is_err()
        {
            return None;
        }
        self.states.analyser.next(
            actor,
            admitted.game_session_id,
            admitted.continuity.connection_generation,
        )
    }

    pub(in crate::gameplay_transport) async fn analyser_receipts(
        &self,
        target: ExactActorRef,
    ) -> Arc<Mutex<DeathReceipts>> {
        let runtime = self.runtime.lock().await;
        self.states
            .attack
            .lock()
            .await
            .kills()
            .analyser_receipts(&runtime, target)
    }

    pub(in crate::gameplay_transport) async fn analyser_loot(
        &self,
        entry: &crate::gameplay_transport::kill_reward::PendingKillSettlement,
        committed: &[(
            crate::durability::item_mint::CommittedItemMint,
            crate::combat::LootDefinitionRef,
            u32,
        )],
    ) {
        let receipts = self.analyser_receipts(entry.target).await;
        let runtime = self.runtime.lock().await;
        let principal = &entry.facts.principal;
        if runtime
            .bound_attacker_lease(principal.actor, principal.session)
            .is_err()
        {
            return;
        }
        let Some(index) = crate::gameplay_transport::item_ref_admission::pinned_index(
            &runtime,
            self.active_generation,
        ) else {
            return;
        };
        let Ok(mut receipts) = receipts.lock() else {
            return;
        };
        let mut items = Vec::new();
        let mut gold = 0u64;
        for (mint, definition, quantity) in committed {
            if receipts.loot.contains(&mint.item_instance_id) {
                continue;
            }
            let Some(item_type) = index.definition_ref(
                &definition.family,
                &definition.production_key,
                &definition.revision_ref,
            ) else {
                continue;
            };
            let Some(count) = u16::try_from(*quantity).ok().and_then(NonZeroU16::new) else {
                continue;
            };
            if let Some(coin) =
                crate::domain::currency::Coin::from_production_key(&definition.production_key)
            {
                gold = gold.saturating_add(coin.worth() * u64::from(*quantity));
            }
            items.push(AnalyserItemCount { item_type, count });
            receipts.loot.insert(mint.item_instance_id);
        }
        for (ordinal, chunk) in items
            .chunks(oteryn_protocol_oteryn::analyser::MAX_ANALYSER_LOOT_ITEMS)
            .enumerate()
        {
            self.states.analyser.push(
                principal.actor,
                principal.session,
                AnalyserFact::KillLoot {
                    race: race(&entry.creature),
                    corpse_items: chunk.to_vec(),
                    gold: if ordinal == 0 { gold } else { 0 },
                },
            );
        }
    }
}

impl crate::gameplay_transport::actor_spell::ChannelSpellStates {
    pub(in crate::gameplay_transport) fn analyser_batch(
        &self,
        runtime: &ChannelRuntimeV1,
        batch: &crate::spell::combat_batch::OwnerCombatBatch,
        receipt: &crate::spell::combat_batch::CombatBatchReceipt,
        element: DamageElement,
        split: Option<(u32, DamageElement, DamageElement)>,
    ) {
        use crate::spell::combat_batch::OwnerCombatChange as C;
        for effect in &batch.effects {
            let Some(health) = receipt
                .effects
                .iter()
                .find(|r| r.target == effect.target && r.sub_ordinal == effect.sub_ordinal)
                .and_then(|r| r.health)
            else {
                continue;
            };
            if !health.applied {
                continue;
            }
            let mut parts = [(0u64, element); 2];
            match effect.change {
                C::Heal { .. } => {
                    self.analyser.impact_amount(
                        batch.caster,
                        batch.command.game_session_id(),
                        u64::try_from(health.health_after.saturating_sub(health.health_before))
                            .unwrap_or(0),
                        None,
                    );
                    continue;
                }
                C::Damage { .. } => {
                    let value =
                        u64::try_from(health.health_before.saturating_sub(health.health_after))
                            .unwrap_or(0);
                    if let Some((primary, a, b)) = split {
                        let primary = u64::from(primary).min(value);
                        parts = [(primary, a), (value - primary, b)];
                    } else {
                        parts[0].0 = value;
                    }
                }
                C::DamageWithHealing { healing, .. } => {
                    let maximum = runtime
                        .companion_snapshot_including_dead(effect.target)
                        .ok()
                        .map_or(health.health_before, |view| view.maximum_health);
                    let healing = healing
                        .min(maximum.saturating_sub(health.health_before))
                        .max(0);
                    parts[0].0 = u64::try_from(
                        health
                            .health_before
                            .saturating_add(healing)
                            .saturating_sub(health.health_after),
                    )
                    .unwrap_or(0);
                }
                _ => continue,
            }
            for (value, element) in parts {
                self.analyser.impact_amount(
                    batch.caster,
                    batch.command.game_session_id(),
                    value,
                    Some(element),
                );
                if let Some((_, session, _)) = self.actors.iter().find(|(actor, session, _)| {
                    *actor == effect.target
                        && runtime.bound_attacker_lease(*actor, *session).is_ok()
                }) {
                    self.analyser.input_amount(
                        effect.target,
                        *session,
                        value,
                        element,
                        super::HitSource {
                            source: oteryn_protocol_oteryn::analyser::AnalyserDamageSource::Player,
                            attacker: Some(batch.caster),
                        },
                    );
                }
            }
        }
    }
}
