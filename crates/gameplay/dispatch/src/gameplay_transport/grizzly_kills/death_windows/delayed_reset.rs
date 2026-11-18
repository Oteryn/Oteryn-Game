use crate::foundation::{ChannelRuntimeV1, CreatureDeathOccurrenceKey, ExactActorRef};
use crate::map::{
    boot::BundleMap,
    facts::QuestCallbackProjection,
    overlay::TilePos,
    view::{MapFacts, placement_key},
};
use oteryn_gameplay_entities::carrier::native_source_death::SourcePlayerDamageShare;
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, DeathMapReset};

#[derive(Debug)]
pub(super) struct PendingReset {
    callback: String,
    quest: String,
    creature: String,
    rule: DeathMapReset,
    death: DeathEvidence,
    damagers: Vec<SourcePlayerDamageShare>,
    pub(super) due_ms: u64,
    map_applied: bool,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct DeathEvidence {
    pub(super) actor: ExactActorRef,
    pub(super) key: CreatureDeathOccurrenceKey,
    pub(super) position: (u16, u16, i8),
    pub(super) at_ms: u64,
}

#[derive(Debug)]
pub(in crate::gameplay_transport) struct ReadyReset {
    callback: String,
    quest: String,
    creature: String,
    transition: String,
    death: DeathEvidence,
    hit: SourcePlayerDamageShare,
}

impl PendingReset {
    // Crystal 201189f8 creaturescripts_bosses_kill.lua:136-144 schedules one
    // reset for each direct damaging player who is still present at expiry.
    pub(super) fn capture(
        runtime: &ChannelRuntimeV1,
        death: DeathEvidence,
        key: &str,
        quest: &str,
        creature: &str,
        rule: &DeathMapReset,
    ) -> Option<Self> {
        let mut damagers = runtime.source_player_damage_shares(death.actor).ok()?;
        damagers.retain(|hit| hit.total > 0 && live(runtime, hit));
        (!damagers.is_empty()).then(|| Self {
            callback: key.to_owned(),
            quest: quest.to_owned(),
            creature: creature.to_owned(),
            rule: rule.clone(),
            death,
            damagers,
            due_ms: death.at_ms.saturating_add(u64::from(rule.delay_ms)),
            map_applied: false,
        })
    }

    pub(super) fn matches(&self, callback: &Callback) -> bool {
        self.callback == callback.key
            && callback
                .effects
                .death_window
                .as_ref()
                .and_then(|rule| rule.delayed_reset.as_ref())
                == Some(&self.rule)
    }

    pub(super) fn next_map_due_ms(&self) -> Option<u64> {
        (!self.map_applied).then_some(self.due_ms)
    }

    /// Return whether the pending map write should be retried.
    pub(super) fn advance_if_live(&mut self, runtime: &ChannelRuntimeV1, map: &BundleMap) -> bool {
        let scope = runtime.binding();
        if self.death.actor.runtime_scope() != scope.runtime_scope()
            || self.death.actor.scope_generation() != scope.scope_generation()
        {
            return false;
        }
        self.damagers.retain(|hit| live(runtime, hit));
        if self.damagers.is_empty() {
            return false;
        }
        if !self.map_applied {
            if !project(map, &self.rule) {
                return true;
            }
            self.map_applied = true;
        }
        self.rule.character_reset.is_some()
    }

    pub(super) fn ready(&self) -> impl Iterator<Item = ReadyReset> + '_ {
        self.rule
            .character_reset
            .iter()
            .filter(|_| self.map_applied)
            .flat_map(|reset| {
                self.damagers.iter().map(move |hit| ReadyReset {
                    callback: self.callback.clone(),
                    quest: self.quest.clone(),
                    creature: self.creature.clone(),
                    transition: reset.transition.clone(),
                    death: self.death,
                    hit: *hit,
                })
            })
    }

    pub(super) fn acknowledge(&mut self, ready: &ReadyReset) {
        if self.callback == ready.callback && self.death.key == ready.death.key {
            self.damagers.retain(|hit| *hit != ready.hit);
        }
    }
}

impl ReadyReset {
    pub(in crate::gameplay_transport) async fn settle(
        &self,
        owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    ) -> bool {
        use crate::durability::{
            character_progression::CharacterProgressionError,
            quest_state::{
                QuestTransitionOutcome,
                native_kills::{NativeDeathSource, NativeQuestDeath},
            },
        };
        let Some(catalogue) = &owner.quest_catalogue else {
            return true;
        };
        let Some(hash) = catalogue.definition_hash(&self.quest) else {
            return true;
        };
        let fence = match owner.current_quest_fence(self.hit.session).await {
            Ok(Some(fence))
                if fence.character_id.as_bytes() == self.hit.character.as_bytes()
                    && fence.character_lease_generation == self.hit.lease_generation =>
            {
                fence
            }
            Ok(_) => return true,
            Err(()) => return false,
        };
        let Ok(cause) = NativeQuestDeath::delayed_reset_cause(self.death.key) else {
            return true;
        };
        let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
        let runtime = owner.runtime.lock().await;
        if runtime.owner_fence().is_err() || !live(&runtime, &self.hit) {
            return true;
        }
        let death = NativeQuestDeath {
            transition: self.transition.clone(),
            creature: self.creature.clone(),
            definition_hash: Some(hash),
            observed_revision: fence.expected_character_revision.get(),
            source: NativeDeathSource::DelayedReset {
                death: self.death.key,
                session: self.hit.session,
                character: self.hit.character,
                lease_generation: self.hit.lease_generation,
                position: self.death.position,
                death_at_ms: self.death.at_ms,
                observed_at_ms: owner.owner_now().get() / 1_000,
            },
        };
        let outcome = slot
            .commit_native_quest_kill_with_admission(
                owner.root,
                owner.character,
                owner.holder,
                fence,
                cause,
                &death,
                std::sync::Arc::clone(catalogue),
                &owner.premium_coordinator,
            )
            .await;
        drop(runtime);
        drop(slot);
        match outcome {
            Ok(
                QuestTransitionOutcome::Committed(_) | QuestTransitionOutcome::AlreadyCommitted(_),
            ) => {
                owner.refresh_quest_session(self.hit.session, false).await;
            }
            Err(
                CharacterProgressionError::Unavailable(_)
                | CharacterProgressionError::CharacterRevisionMismatch,
            ) => return false,
            Err(error) => crate::gameplay_transport::operator_event(&format!(
                "quest_delayed_death_reset_refused reason={error:?}"
            )),
            Ok(QuestTransitionOutcome::Refused(_) | QuestTransitionOutcome::ObligationClosed) => {}
        }
        true
    }
}

fn live(runtime: &ChannelRuntimeV1, hit: &SourcePlayerDamageShare) -> bool {
    runtime
        .bound_attacker_lease(hit.actor, hit.session)
        .is_ok_and(|lease| {
            lease.character_id() == hit.character && lease.generation() == hit.lease_generation
        })
}

fn project(map: &BundleMap, rule: &DeathMapReset) -> bool {
    // As in the donor, absent or already inactive crystals require no write.
    // The remaining active objects share one version-checked map projection.
    let mut changes = rule.positions.iter().filter_map(|position| {
        let pos = TilePos {
            x: position.x,
            y: position.y,
            floor: position.floor,
        };
        let base = map.overlay.base().tile(pos.x, pos.y, pos.floor)?;
        let mut candidates = base
            .ids()
            .iter()
            .zip(base.depths())
            .filter(|(_, depth)| **depth == 0)
            .enumerate()
            .filter_map(|(ordinal, (id, _))| {
                let ordinal = u8::try_from(ordinal).ok()?;
                let key = placement_key(pos, ordinal)?;
                let current = map.facts.base_entry(pos, ordinal, *id)?;
                (!map.facts.base_hidden(key)
                    && !current.pickupable
                    && current.appearance_id == rule.from)
                    .then_some(key)
            });
        let key = candidates.next()?;
        if candidates.next().is_some() {
            return None;
        }
        let after = map.facts.quest_callback_replacement(key, rule.to)?;
        (!after.pickupable).then(|| (key, map.facts.object_revision(key), after))
    });
    let Some((key, revision, after)) = changes.next() else {
        return true;
    };
    let entries = changes
        .map(|(key, revision, after)| (key, revision, Some(after)))
        .collect();
    map.facts.apply_quest_callback(
        key,
        revision,
        after,
        &[],
        &[],
        false,
        Some(&QuestCallbackProjection::ReadsetTransforms {
            entries,
            reset: None,
        }),
    )
}
