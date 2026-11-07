//! General condition logout/login persistence, alongside the existing gourmet save point.
use super::*;
use crate::durability::character_condition_snapshot::CharacterConditionSnapshot;
use crate::durability::item_use::ItemUseError;
use oteryn_gameplay_foundation::foundation::condition::ConditionStore;

/// Validate durable data before creating any actor state: a refused retry cannot
/// bypass restore merely because a prior attempt allocated an empty state entry.
pub(super) fn prepare(
    saved: Option<&CharacterConditionSnapshot>,
    lease_generation: u64,
    content_digest: [u8; 32],
    now: u64,
) -> Option<Option<ConditionStore<String>>> {
    let Some(saved) = saved else {
        return Some(None);
    };
    if saved.lease_generation >= lease_generation {
        return None;
    }
    // Native source spells advance in whole milliseconds. Keep the fresh store
    // on that same clock so durable food projections can be restored afterwards.
    let next =
        ConditionStore::from_persistent_snapshot(&saved.conditions, now / 1000 * 1000).ok()?;
    // Primitive condition values are durable data, not a new content activation.
    // Asset-bearing appearances still require their exact compatible content pin.
    if saved.content_digest != content_digest
        && next.instances().iter().any(|i| {
            i.definition().condition_type()
                == oteryn_gameplay_domain::domain::condition::ConditionType::Outfit
        })
    {
        return None;
    }
    Some(Some(next))
}

pub(super) fn restore(
    state: &mut crate::spell::cast::PlayerSpellState,
    prepared: Option<&ConditionStore<String>>,
    gourmet: oteryn_gameplay_domain::domain::hireling::food::GourmetConditions,
    now_us: u64,
    bed_rest: Option<[u32; 4]>,
) -> Option<()> {
    use oteryn_gameplay_domain::domain::condition::{ConditionType, ConflictKey};
    if let Some(prepared) = prepared {
        let mut next = prepared.clone();
        // These two families already have a durable native USE/checkpoint owner.
        // A prior logout snapshot cannot resurrect or refresh their source rows.
        for key in [ConflictKey::Attributes, ConflictKey::Speed] {
            if let Some(kind) = next
                .get(key)
                .filter(|i| i.definition().key().starts_with("item.hireling.gourmet.i"))
                .map(|i| i.definition().condition_type())
            {
                next.remove_type(kind);
            }
        }
        // A qualified bed rest may have consumed food while the actor was offline.
        if bed_rest.is_some() {
            next.remove_type(ConditionType::FoodRegeneration);
        }
        let before = state.owned_conditions().clone();
        state.apply_batch_conditions(&before, &next).ok()?;
        if let Some(rest) = bed_rest {
            state.restore_bed_rest(rest, now_us)?;
        }
    }
    crate::spell::actor_conditions::restore_gourmet_food(state, gourmet, now_us / 1000).ok()
}

impl ComposedFreshAdmission<'_, '_, '_> {
    /// Called only after the release fence stops this actor's mutation lanes.
    pub(super) async fn save_conditions_at_actor_end(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> bool {
        if self
            .resolve_pending_cake_food(actor, session)
            .await
            .is_err()
        {
            return false;
        }
        if self
            .resolve_pending_npc_ajax_effect(actor, session)
            .await
            .is_err()
        {
            return false;
        }
        if self
            .resolve_pending_npc_hazard_effect(actor, session)
            .await
            .is_err()
        {
            return false;
        }
        let store = FreshAdmissionStore::from_root(self.root.clone());
        let Ok((current, _)) = store.current_session_at(session).await else {
            return false;
        };
        let Some(fence) = item_fence_of(current) else {
            return false;
        };
        let saved = {
            let runtime = self.runtime.lock().await;
            let valid = match fence.runtime_scope {
                RuntimeScopeRefV1::Channel {
                    world_id,
                    channel_id,
                } => {
                    let binding = runtime.binding();
                    binding.world_id() == world_id
                        && binding.channel_id() == Some(channel_id)
                        && binding.scope_generation() == fence.scope_ownership_generation
                        && runtime.player_control_facts(actor, session).is_ok()
                }
                RuntimeScopeRefV1::Instance { .. } => {
                    runtime.house_actor_state(actor, session).is_ok_and(|h| {
                        h.scope == fence.runtime_scope
                            && h.generation == fence.scope_ownership_generation
                    })
                }
            };
            if !valid {
                return true;
            }
            let states = self.states.spell_states.lock().await;
            let Some((_, _, state)) = states
                .actors
                .iter()
                .find(|(a, s, _)| *a == actor && *s == session)
            else {
                return true;
            };
            let Ok(conditions) = state
                .owned_conditions()
                .persistent_snapshot(self.owner_now().get())
            else {
                return false;
            };
            CharacterConditionSnapshot {
                lease_generation: fence.character_lease_generation,
                content_digest: runtime.content_pin().server_artifact_digest(),
                conditions,
            }
        };
        // Never retain runtime/player locks across the database await. The release
        // fence remains held, and SQL independently locks and proves the live lease.
        match self
            .root
            .save_character_conditions(
                self.character,
                self.holder,
                fence,
                *self.channel_id.as_bytes(),
                saved,
            )
            .await
        {
            Ok(()) | Err(ItemUseError::AuthorityRejected) => true,
            Err(_) => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn condition_persistence_refuses_future_lease_corrupt_data_and_foreign_appearance()
    -> Result<(), &'static str> {
        let mut saved = CharacterConditionSnapshot {
            lease_generation: 2,
            content_digest: [7; 32],
            conditions: ConditionStore::<String>::new()
                .persistent_snapshot(0)
                .map_err(|_| "snapshot")?,
        };
        assert!(prepare(Some(&saved), 3, [7; 32], 1).is_some());
        assert!(prepare(Some(&saved), 2, [7; 32], 1).is_none());
        assert!(prepare(Some(&saved), 1, [7; 32], 1).is_none());
        assert!(
            prepare(Some(&saved), 3, [8; 32], 1).is_some(),
            "unrelated content changes cannot block an empty checkpoint"
        );
        use oteryn_gameplay_foundation::foundation::condition::{
            ApplicationFacts, ConditionDefinition, ConditionSourceKind,
        };
        let root = oteryn_simulation_determinism::GameplayDecisionRoot::from_bytes([1; 32]);
        let mut store = ConditionStore::<String>::new();
        let definition = ConditionDefinition::new(
            "test.c08.outfit",
            1,
            oteryn_gameplay_domain::domain::condition::ConditionValues::Outfit {
                duration_ms: 1000,
                look_type: 128,
            },
        )
        .ok_or("outfit")?;
        store
            .apply(
                &definition,
                None,
                ConditionSourceKind::SelfUse,
                &[],
                &ApplicationFacts {
                    now: 0,
                    base_speed: 220,
                    mana_shield_capacity: 0,
                    target_reentry_protected: false,
                    source_reentry_protected: false,
                    target_is_player: true,
                    decision_root: &root,
                    occurrence: oteryn_simulation_determinism::DecisionOccurrenceId::from_bytes(
                        [1; 16],
                    ),
                },
            )
            .map_err(|_| "outfit application")?;
        saved.conditions = store
            .persistent_snapshot(0)
            .map_err(|_| "outfit snapshot")?;
        assert!(prepare(Some(&saved), 3, [7; 32], 1).is_some());
        assert!(
            prepare(Some(&saved), 3, [8; 32], 1).is_none(),
            "an incompatible appearance must remain closed"
        );
        saved.conditions = serde_json::json!({"schema":2,"conditions":[]});
        assert!(prepare(Some(&saved), 3, [7; 32], 1).is_none());
        Ok(())
    }
    #[test]
    fn condition_persistence_keeps_the_durable_gourmet_owner_authoritative()
    -> Result<(), &'static str> {
        use oteryn_gameplay_domain::domain::hireling::food::{GourmetBuff, GourmetConditions};
        let actor = || {
            crate::spell::cast::PlayerSpellState::new(super::super::actor_spell::tests::FACTS, 0, 0)
                .ok_or("actor")
        };
        let old = crate::spell::actor_conditions::stage_gourmet_food(&actor()?, 29408, 0)
            .map_err(|_| "old attributes")?;
        let old = crate::spell::actor_conditions::stage_gourmet_food(&old, 29412, 0)
            .map_err(|_| "old haste")?;
        let saved = CharacterConditionSnapshot {
            lease_generation: 2,
            content_digest: [7; 32],
            conditions: old
                .owned_conditions()
                .persistent_snapshot(0)
                .map_err(|_| "saved")?,
        };
        for current in [
            GourmetConditions::default(),
            GourmetConditions {
                attributes: Some(GourmetBuff {
                    item_id: 29409,
                    remaining_ms: 9_000,
                }),
                haste: Some(GourmetBuff {
                    item_id: 29412,
                    remaining_ms: 8_000,
                }),
            },
        ] {
            let mut fresh = actor()?;
            let prepared = prepare(Some(&saved), 3, [7; 32], 123).ok_or("prepared")?;
            restore(&mut fresh, prepared.as_ref(), current, 123, None).ok_or("restored")?;
            assert_eq!(
                crate::spell::actor_conditions::gourmet_conditions(&fresh, 0),
                current,
                "a previous logout cannot resurrect or refresh a separately checkpointed gourmet source"
            );
        }
        Ok(())
    }

    #[test]
    fn condition_persistence_keeps_offline_bed_food_consumption_authoritative()
    -> Result<(), &'static str> {
        let actor = || {
            crate::spell::cast::PlayerSpellState::new(super::super::actor_spell::tests::FACTS, 0, 0)
                .ok_or("actor")
        };
        let mut old = actor()?;
        crate::spell::actor_conditions::eat_food(&mut old, 60, 0).map_err(|_| "old food")?;
        let saved = CharacterConditionSnapshot {
            lease_generation: 2,
            content_digest: [7; 32],
            conditions: old
                .owned_conditions()
                .persistent_snapshot(0)
                .map_err(|_| "saved food")?,
        };
        for food_ms in [0, 9_000] {
            let rest = [20, food_ms, 500, 400];
            let mut fresh = actor()?;
            fresh.restore_bed_rest(rest, 123).ok_or("bed admission")?;
            let prepared = prepare(Some(&saved), 3, [7; 32], 123).ok_or("prepared food")?;
            restore(
                &mut fresh,
                prepared.as_ref(),
                Default::default(),
                123,
                Some(rest),
            )
            .ok_or("restored bed")?;
            assert_eq!(
                fresh.bed_rest_snapshot(0),
                rest,
                "a stale logout cannot undo durable offline food consumption"
            );
        }
        Ok(())
    }
}
