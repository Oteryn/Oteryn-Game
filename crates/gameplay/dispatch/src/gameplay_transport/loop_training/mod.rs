//! LOOP-TRAIN-1 server composition. No client-selected skill, rate, clock or charge count.
mod exercise;
pub(super) mod wire;
use super::capabilities::OfferedCapability;
use super::connection::{AdmittedSession, FirstEntryOutcome, UseCommand};
use super::{ComposedFreshAdmission, item_view::ItemKey};
use crate::durability::character_progression::CurrentCharacterGameplayFence;
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::durability::item_transfer::CurrentCharacterItemFence;
use crate::durability::loop_training::{TrainingContext, TrainingLogout};
use crate::foundation::{ExactActorRef, GameSessionId};
use oteryn_gameplay_progression::loop_training::TrainingRates;
use oteryn_protocol_oteryn::loop_training::{CAPABILITY_TRAINING_V1, TrainingResult};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;

pub(in crate::gameplay_transport) struct TrainingRuntime {
    pub config: Option<Arc<TrainingRates>>,
    sessions: Mutex<HashMap<GameSessionId, Session>>,
}
#[derive(Default)]
struct Session {
    selected: Option<(CurrentCharacterItemFence, usize)>,
    exercise: Option<exercise::Run>,
    ready_at_us: u64,
    last_weapon: Option<crate::durability::exercise_training::TrainingWeapon>,
}
impl TrainingRuntime {
    pub(in crate::gameplay_transport) async fn forget(&self, session: GameSessionId) {
        self.sessions.lock().await.remove(&session);
    }
}
impl Default for TrainingRuntime {
    fn default() -> Self {
        let config = TrainingRates::from_json(include_bytes!(
            "../../../../../../rulesets/training/provisional-v1.json"
        ))
        .ok()
        .and_then(|mut rates| {
            // Explicit operator opt-in; absence and every other value keep the shipped gate closed.
            if std::env::var("OTERYN_ENABLE_PROVISIONAL_TRAINING")
                .ok()
                .as_deref()
                != Some("1")
            {
                return None;
            }
            rates.enabled = true;
            Some(Arc::new(rates))
        });
        Self {
            config,
            sessions: Mutex::new(HashMap::new()),
        }
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    fn training_context(&self) -> Option<TrainingContext> {
        let rates = self.states.training.config.as_ref()?.clone();
        if !rates.enabled {
            return None;
        }
        let native = self.active_generation?.native_gameplay()?;
        let index = native.item_index()?;
        if !rates
            .weapons
            .keys()
            .any(|key| native.item_policy(key, "definition-r1").is_some())
            || !rates
                .statues
                .keys()
                .any(|key| index.definition_ref("Item", key, "definition-r1").is_some())
            || !rates
                .dummies
                .iter()
                .any(|key| index.definition_ref("Item", key, "definition-r1").is_some())
        {
            return None;
        }
        let formula = native.training_formula()?.clone();
        Some(TrainingContext {
            rates,
            formula,
            progression: self
                .active_generation
                .and_then(|active| active.progression())
                .cloned(),
        })
    }
    async fn training_fence(
        &self,
        expected: CurrentCharacterItemFence,
    ) -> Option<CurrentCharacterGameplayFence> {
        let current = FreshAdmissionStore::from_root(self.root.clone())
            .current_session(expected.game_session_id)
            .await
            .ok()?;
        if super::item_fence_of(current) != Some(expected) {
            return None;
        }
        let revision = self
            .root
            .read_current_character(self.character, expected.character_id)
            .await
            .ok()?
            .revision;
        Some(CurrentCharacterGameplayFence {
            expected_character_revision: revision,
            character_id: expected.character_id,
            game_session_id: expected.game_session_id,
            connection_generation: expected.connection_generation,
            character_lease_generation: expected.character_lease_generation,
            runtime_scope: expected.runtime_scope,
            scope_ownership_generation: expected.scope_ownership_generation,
        })
    }
    pub(in crate::gameplay_transport) async fn admit_loop_training(
        &self,
        admitted: &mut AdmittedSession,
    ) {
        let Some(context) = self.training_context() else {
            return;
        };
        let Some(item) = admitted.item_fence else {
            admitted.first_entry = FirstEntryOutcome::RefusedUnavailable;
            return;
        };
        let mut slot = self.revision_sequencer.acquire(item.character_id).await;
        let Some(fence) = self.training_fence(item).await else {
            admitted.first_entry = FirstEntryOutcome::RefusedUnavailable;
            return;
        };
        let result = self
            .root
            .settle_offline_training(self.character, self.holder, fence, context)
            .await;
        slot.observe_loop_training();
        #[cfg(test)]
        if let Err(error) = &result {
            eprintln!("LOOP-TRAIN-1 admission: {error:?}");
        }
        if result.is_err() {
            admitted.first_entry = FirstEntryOutcome::RefusedUnavailable;
            return;
        }
        let Ok(build) = self
            .root
            .read_character_build_state(self.character, item.character_id)
            .await
        else {
            admitted.first_entry = FirstEntryOutcome::RefusedUnavailable;
            return;
        };
        let Ok(stamina) = self
            .root
            .read_hunting_stamina(self.character, item.character_id)
            .await
        else {
            admitted.first_entry = FirstEntryOutcome::RefusedUnavailable;
            return;
        };
        if let Some(actor) = admitted.runtime_actor {
            let runtime = self.runtime.lock().await;
            let mut states = self.states.spell_states.lock().await;
            if let Some(state) = states.get_mut(&runtime, actor, admitted.game_session_id) {
                state.rebase_after_death(&build);
            }
            states.load_stamina(&runtime, actor, admitted.game_session_id, stamina);
        }
    }
    pub(in crate::gameplay_transport) async fn training_release_store(
        &self,
        store: FreshAdmissionStore,
        admitted: &AdmittedSession,
        graceful: bool,
    ) -> Option<FreshAdmissionStore> {
        let Some(context) = self.training_context() else {
            return Some(store);
        };
        if !graceful {
            return Some(store);
        }
        let item = admitted.item_fence?;
        let selected = self
            .states
            .training
            .sessions
            .lock()
            .await
            .get(&admitted.game_session_id)
            .and_then(|state| state.selected);
        if selected.is_some_and(|(bound, _)| bound != item) {
            return None;
        }
        let fence = self.training_fence(item).await?;
        Some(
            store.with_training_logout(
                TrainingLogout::new(
                    self.root,
                    self.character,
                    self.holder,
                    fence,
                    context,
                    selected.map(|(_, skill)| skill),
                )
                .ok()?,
            ),
        )
    }

    async fn training_target(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        key: ItemKey,
    ) -> Option<String> {
        use crate::map::view::MapFacts;
        let fence = command.item_fence?;
        let mut runtime = self.runtime.lock().await;
        runtime.owner_fence().ok()?;
        if fence.runtime_scope
            != crate::foundation::RuntimeScopeRefV1::channel(self.world_id, self.channel_id)
            || fence.scope_ownership_generation.get() != runtime.binding().scope_generation().get()
        {
            return None;
        }
        let states = self.states.spell_states.lock().await;
        if !runtime
            .player_control_facts(actor, command.game_session_id)
            .is_ok_and(|facts| facts.control_loss.is_none())
            || states.is_dead(actor)
            || states.has_pending_spell_commit(actor, command.game_session_id)
            || fence.game_session_id != command.game_session_id
        {
            return None;
        }
        let index = super::item_ref_admission::pinned_index(&runtime, self.active_generation)?;
        let at = super::bundle_map_position(&mut runtime, self.movement_cells, actor)?;
        let ItemKey::MapBase {
            bundle_digest,
            placement_key,
            reset_epoch,
        } = key
        else {
            return None;
        };
        if bundle_digest != at.map.overlay.base().digest() || reset_epoch != self.reset_epoch {
            return None;
        }
        let (pos, ordinal) = crate::map::view::placement_of(placement_key)?;
        let player = crate::map::overlay::TilePos {
            x: u16::try_from(at.at.x).ok()?,
            y: u16::try_from(at.at.y).ok()?,
            floor: i8::try_from(at.at.floor).ok()?,
        };
        if at.map.facts.house_tile(player) || at.map.facts.house_tile(pos) {
            return None;
        }
        if !crate::map::overlay::pickup::within_reach(player, pos) {
            return None;
        }
        let tile = at.map.overlay.base().tile(pos.x, pos.y, pos.floor)?;
        let id = tile
            .ids()
            .iter()
            .zip(tile.depths())
            .filter(|(_, depth)| **depth == 0)
            .nth(usize::from(ordinal))
            .map(|(id, _)| *id)?;
        let fact = at.map.facts.base_entry(pos, ordinal, id)?;
        let oteryn_protocol_oteryn::world_map::MapDefinition::Item(reference) = fact.definition
        else {
            return None;
        };
        let definition = super::item_look::typed_definition(index, reference)?;
        // Both actor and target are server-qualified PZ cells. Unknown tile facts fail closed.
        let scope = self.movement_cells.scope();
        for cell in [
            crate::content::LogicalCell {
                x: at.at.x,
                y: at.at.y,
                z: -i32::from(at.at.floor),
            },
            crate::content::LogicalCell {
                x: i32::from(pos.x),
                y: i32::from(pos.y),
                z: -i32::from(pos.floor),
            },
        ] {
            if !self
                .movement_cells
                .spell_tiles()
                .lookup(scope, cell)
                .ok()?
                .flags()
                .protection_zone
            {
                return None;
            }
        }
        Some(definition.production_key)
    }

    pub(in crate::gameplay_transport) async fn offline_training(
        &self,
        admitted: AdmittedSession,
        command: UseCommand,
        target: ItemKey,
    ) -> Option<TrainingResult> {
        let Some(context) = self.training_context() else {
            return Some(TrainingResult::REFUSED);
        };
        let (Some(actor), Some(controller), Some(fence)) = (
            admitted.runtime_actor,
            admitted.controller,
            command.item_fence,
        ) else {
            return Some(TrainingResult::REFUSED);
        };
        let Some(key) = self.training_target(actor, command, target).await else {
            return Some(TrainingResult::REFUSED);
        };
        let Some(family) = context.rates.statues.get(&key).copied() else {
            return Some(TrainingResult::REFUSED);
        };
        let Some(premium) = self.premium_coordinator else {
            return Some(TrainingResult::REFUSED);
        };
        let current = premium
            .with_premium_gate(controller.account_id, |status, _| {
                Box::pin(
                    async move { matches!(status, crate::domain::premium::PremiumStatus::Current) },
                )
            })
            .await
            .ok()?;
        let Some(gameplay) = self.training_fence(fence).await else {
            return Some(TrainingResult::REFUSED);
        };
        let pvp = self
            .root
            .read_pvp_state(self.character, self.holder, gameplay)
            .await
            .ok()?;
        if pvp.is_some_and(|state| state.readmission_clock.has_running_lock()) {
            return Some(TrainingResult::REFUSED);
        }
        // NotActivated is never Premium for statues; the marker transaction rechecks durable PvP locks.
        if !current
            || self
                .combat_lock_fact(actor, admitted.game_session_id)
                .await
                .pve_in_fight_until
                .is_some()
        {
            return Some(TrainingResult::REFUSED);
        }
        if !self.states.world_gate.lock().ok()?.admit(
            actor,
            std::time::Duration::from_micros(self.owner_now().get()),
        ) {
            return Some(TrainingResult::REFUSED);
        }
        self.states
            .training
            .sessions
            .lock()
            .await
            .entry(admitted.game_session_id)
            .or_default()
            .selected = Some((fence, family));
        let reply = self.logout_session(admitted).await;
        match reply {
            super::logout::LogoutReply::Result(
                oteryn_protocol_oteryn::logout::LogoutResult::Accepted,
            ) => {
                let marker = self
                    .root
                    .read_training_logout(self.character, gameplay)
                    .await
                    .ok()??;
                Some(TrainingResult {
                    disposition: 3,
                    revision: marker.revision.get(),
                    pool_seconds: marker.offline.pool_seconds,
                    charges: 0,
                })
            }
            super::logout::LogoutReply::Result(_) => {
                if let Some(state) = self
                    .states
                    .training
                    .sessions
                    .lock()
                    .await
                    .get_mut(&admitted.game_session_id)
                {
                    state.selected = None;
                }
                Some(TrainingResult::REFUSED)
            }
            super::logout::LogoutReply::Unknown => None,
        }
    }
}

pub(super) fn offer(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    base: &'static [OfferedCapability],
) -> &'static [OfferedCapability] {
    if owner.bundle_map().is_none() || owner.training_context().is_none() {
        return base;
    }
    type Cache = Vec<(&'static [OfferedCapability], &'static [OfferedCapability])>;
    static CACHE: std::sync::Mutex<Cache> = std::sync::Mutex::new(Vec::new());
    let Ok(mut cache) = CACHE.lock() else {
        return base;
    };
    if let Some((_, offer)) = cache.iter().find(|(key, _)| *key == base) {
        return offer;
    }
    let mut offer = base.to_vec();
    offer.push(OfferedCapability {
        id: CAPABILITY_TRAINING_V1,
        requires: &[4, 18, 21],
    });
    let offer = Box::leak(offer.into_boxed_slice());
    cache.push((base, offer));
    offer
}
