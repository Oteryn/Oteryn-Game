//! Crystal201189 boss_lever.lua:337-338 / teleport.lua:5-27: Ahau's
//! Player-only STEP exit is a shared Instance return, never a Channel teleport.
use super::ComposedFreshAdmission;
use super::connection::AdmittedSession;
use super::native_activity::NativeActivityOutcome;
use crate::content::bosses::activity_template::QualifiedBossExitTeleporter;
use crate::durability::activity_handoff::ActivityNativeStepExit;
use crate::foundation::{ChannelRuntimeV1, ExactActorRef, GameSessionId, RuntimeScopeRefV1};
use oteryn_gameplay_entities::carrier::native_translation_events::CommittedMovementOccurrence;

fn portal_tile(trigger: crate::content::LogicalCell) -> Option<crate::map::overlay::TilePos> {
    Some(crate::map::overlay::TilePos {
        x: u16::try_from(trigger.x).ok()?,
        y: u16::try_from(trigger.y).ok()?,
        floor: i8::try_from(trigger.z.checked_neg()?).ok()?,
    })
}

fn has_portal(
    map: &crate::map::boot::BundleMap,
    tile: crate::map::overlay::TilePos,
    appearance: u16,
) -> bool {
    use crate::map::view::MapFacts;
    map.facts
        .movement_items(tile)
        .iter()
        .any(|entry| entry.facts.appearance_id == appearance)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SourceBossExitProof {
    native_step: ActivityNativeStepExit,
    actor: ExactActorRef,
    session: GameSessionId,
    exit: QualifiedBossExitTeleporter,
}
impl SourceBossExitProof {
    pub(super) fn matches(&self, runtime: &ChannelRuntimeV1, admitted: AdmittedSession) -> bool {
        let Some(fence) = admitted.item_fence else {
            return false;
        };
        runtime.owner_fence().is_ok()
            && matches!(
                runtime.binding().runtime_scope(),
                RuntimeScopeRefV1::Instance { .. }
            )
            && admitted.runtime_actor == Some(self.actor)
            && admitted.game_session_id == self.session
            && fence.runtime_scope == runtime.binding().runtime_scope()
            && fence.scope_ownership_generation == runtime.binding().scope_generation()
            && fence.game_session_id == self.session
            && fence.connection_generation.get() == admitted.continuity.connection_generation
            && runtime
                .bound_attacker_lease(self.actor, self.session)
                .is_ok_and(|lease| {
                    lease.character_id().as_bytes() == fence.character_id.as_bytes()
                        && lease.generation() == fence.character_lease_generation
                })
            && runtime
                .player_control_facts(self.actor, self.session)
                .is_ok()
            && ActivityNativeStepExit::from_current_owner(
                runtime,
                self.actor,
                self.session,
                self.movement(),
            )
            .as_ref()
                == Some(&self.native_step)
            && runtime.committed_movement_occurrence(self.actor) == Ok(Some(self.movement()))
            && runtime.read_actor_position(self.actor) == Ok(self.movement().after())
            && {
                let at = self.movement().after().position();
                let trigger = self.exit.trigger();
                at.x == trigger.x && at.y == trigger.y && i32::from(at.floor) == trigger.z
            }
    }
    pub(super) fn movement(&self) -> CommittedMovementOccurrence {
        self.native_step.movement()
    }
    pub(super) fn native_step(&self) -> ActivityNativeStepExit {
        self.native_step.clone()
    }
    pub(super) fn destination(&self) -> crate::content::LogicalCell {
        self.exit.destination()
    }
}
impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn record_activity_boss_defeat(
        &self,
        creature: &str,
        death: crate::foundation::CreatureDeathOccurrenceKey,
        at: i64,
    ) -> bool {
        use super::kill_reward::personal_death_id;
        use super::operator_event;
        let owner =
            crate::durability::activity_handoff::ActivityHandoffOwner::new(self.root.clone());
        use crate::durability::activity_handoff::boss_room::BossRoomDefeatDecision;
        match owner
            .record_boss_defeat(
                self.character,
                self.holder,
                death.runtime_scope(),
                death.scope_ownership_generation(),
                crate::durability::activity_handoff::boss_room::BossDefeatOccurrence {
                    boss_key: creature,
                    death: personal_death_id(death),
                    minted_unix_ms: at,
                },
            )
            .await
        {
            Ok(BossRoomDefeatDecision::Recorded)
                if creature == "oteryn:creature.herald_of_fire" =>
            {
                self.publish_source_death_portal(creature).await
            }
            Ok(
                BossRoomDefeatDecision::Recorded
                | BossRoomDefeatDecision::Unrelated
                | BossRoomDefeatDecision::Late,
            ) => true,
            result => {
                operator_event(&format!(
                    "activity_defeat_deferred creature={creature} result={result:?}"
                ));
                false
            }
        }
    }

    /// Consume the sealed condition death through the same queue as direct damage.
    pub(super) fn queue_herald_periodic_kill(
        &self,
        runtime: &mut ChannelRuntimeV1,
        states: &super::actor_spell::ChannelSpellStates,
        attack: &mut super::attack::ChannelAttackStates,
    ) -> Result<bool, ()> {
        if !matches!(
            runtime.binding().runtime_scope(),
            RuntimeScopeRefV1::Instance { .. }
        ) {
            return Ok(false);
        }
        let Some(kill) = runtime.herald_periodic_kill().map_err(|_| ())? else {
            return Ok(false);
        };
        let rewards = self.reward_table();
        let activity = attack.shared_experience_cells();
        let queue = attack.kills();
        let at = oteryn_simulation_determinism::SemanticTimeMicros::from_micros(
            kill.death_at_ms.saturating_mul(1000),
        );
        super::kill_reward::record_creature_kill(
            runtime,
            super::kill_reward::KillSink {
                prey: Some(states),
                queue,
                rewards,
                activity: &activity,
                charms: Some(self.imported_charms),
            },
            kill.target,
            crate::combat::CapturedRewardPrincipal {
                character: kill.principal.character,
                lease_generation: kill.principal.lease_generation,
                session: kill.principal.session,
                actor: kill.principal.actor,
            },
            kill.death_at_ms,
            Some(at),
        );
        if !queue.holds_death(kill.death) {
            return Err(());
        }
        runtime
            .acknowledge_herald_periodic_kill(kill.target)
            .map_err(|_| ())?;
        Ok(true)
    }

    /// Only a bound Herald's actual zero-HP receipt can take the uncredited exit path.
    pub(super) async fn drain_uncredited_herald_defeat(&self) -> bool {
        if !self.bundle_map().is_some_and(|map| {
            matches!(
                map.overlay.runtime_scope(),
                RuntimeScopeRefV1::Instance { .. }
            )
        }) {
            return false;
        }
        {
            let mut runtime = self.runtime.lock().await;
            if runtime
                .herald_periodic_kill()
                .is_ok_and(|kill| kill.is_some())
            {
                let states = self.states.spell_states.lock().await;
                let mut attack = self.states.attack.lock().await;
                if self
                    .queue_herald_periodic_kill(&mut runtime, &states, &mut attack)
                    .is_err()
                {
                    super::operator_event("herald_periodic_kill_deferred");
                }
                return true;
            }
        }
        let captured = self.runtime.lock().await.prepare_uncredited_herald_defeat(
            self.owner_now().get() / 1_000,
            super::npc_trade_command::unix_ms_now(),
        );
        let (death, at) = match captured {
            Ok(Some(defeat)) => defeat,
            Ok(None) => return false,
            Err(error) => {
                super::operator_event(&format!(
                    "herald_uncredited_defeat_read_deferred error={error:?}"
                ));
                return true;
            }
        };
        if self
            .record_activity_boss_defeat("oteryn:creature.herald_of_fire", death, at)
            .await
            && self
                .runtime
                .lock()
                .await
                .acknowledge_uncredited_herald_defeat(death, at)
                .is_err()
        {
            super::operator_event("herald_uncredited_defeat_ack_deferred");
        }
        true
    }

    /// The durable death receipt precedes the Instance-local portal projection.
    pub(super) async fn publish_source_death_portal(&self, creature: &str) -> bool {
        let runtime = self.runtime.lock().await;
        let scope = runtime.binding().runtime_scope();
        let Some(registered) = self
            .activity_runtimes
            .get(scope, runtime.binding().scope_generation().get())
        else {
            return false;
        };
        let Some(key) = registered.template.as_ref() else {
            return false;
        };
        let Some(template) = self.activity_runtimes.template(&key.key, &key.revision) else {
            return false;
        };
        if runtime.owner_fence().is_err() || template.creature_key() != creature {
            return false;
        }
        let Some(exit) = template.exit_teleporter() else {
            return false;
        };
        let (Some(appearance), Some(tile), Some(map)) = (
            exit.death_portal(),
            portal_tile(exit.trigger()),
            self.bundle_map(),
        ) else {
            return false;
        };
        if map.overlay.runtime_scope() != scope {
            return false;
        }
        has_portal(map, tile, appearance)
            || map
                .facts
                .create_movement_item(tile, appearance, 11, || true)
    }

    /// Flush the existing position owner's immutable receipt only after the
    /// caller finished its pace bookkeeping and released its movement guards.
    pub(super) async fn source_boss_exit_after_movement(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        moved_to: Option<oteryn_protocol_oteryn::world_spatial::WorldSpatialObservation>,
    ) {
        if !matches!(actor.runtime_scope(), RuntimeScopeRefV1::Instance { .. })
            || moved_to.is_none()
        {
            return;
        }
        let movement = {
            let runtime = self.runtime.lock().await;
            let Ok(Some(movement)) = runtime.committed_movement_occurrence(actor) else {
                return;
            };
            if runtime.read_actor_position(actor) != Ok(movement.after())
                || self.wire_observation(&runtime, movement.after().position()) != moved_to
            {
                return;
            }
            movement
        };
        self.source_boss_exit_movement(actor, session, movement)
            .await;
    }

    /// Observe physical Player movement after all owned movement guards are released.
    pub(super) async fn source_boss_exit_movement(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        movement: CommittedMovementOccurrence,
    ) {
        let Some(admitted) = self.activity_runtimes.connection(session) else {
            return;
        };
        let Some(fence) = admitted.item_fence else {
            return;
        };
        let Some(registered) = self
            .activity_runtimes
            .get(fence.runtime_scope, fence.scope_ownership_generation.get())
        else {
            return;
        };
        let Some(key) = registered.template.as_ref() else {
            return;
        };
        let Some(template) = self.activity_runtimes.template(&key.key, &key.revision) else {
            return;
        };
        let Some(exit) = template.exit_teleporter() else {
            return;
        };
        let proof = {
            let runtime = self.runtime.lock().await;
            let Some(native_step) =
                ActivityNativeStepExit::from_current_owner(&runtime, actor, session, movement)
            else {
                return;
            };
            if let Some(appearance) = exit.death_portal()
                && !self
                    .bundle_map()
                    .zip(portal_tile(exit.trigger()))
                    .is_some_and(|(map, tile)| has_portal(map, tile, appearance))
            {
                return;
            }
            SourceBossExitProof {
                native_step,
                actor,
                session,
                exit,
            }
        };
        {
            let runtime = self.runtime.lock().await;
            let pin = runtime.content_pin();
            if !proof.matches(&runtime, admitted)
                || template.room().frame_binding().digest() != pin.frame_binding_digest()
                || template.room().map_revision_digest() != pin.map_revision_digest()
                || template.room().compiled().server_digest() != pin.server_artifact_digest()
                || template.room().compiled().client_digest() != pin.client_artifact_digest()
            {
                return;
            }
        }
        let retry = super::AdmissionRetryBudget::for_command(self.root);
        let returned = retry
            .retry_unavailable(async || {
                self.return_source_boss_exit(admitted, proof.clone())
                    .await
                    .map_err(|()| super::connection::AdmissionRefusal::Unavailable)
            })
            .await;
        match returned {
            Ok(NativeActivityOutcome::Committed(commit)) => {
                if self.publish_activity_commit(&commit).is_err() {
                    super::operator_event("source_boss_exit_projection_retry");
                }
            }
            Ok(NativeActivityOutcome::Refused(_)) | Err(_) => {
                super::operator_event("source_boss_exit_return_deferred")
            }
        }
    }
    pub(super) async fn emit_source_boss_exit_effect(
        &self,
        admitted: AdmittedSession,
    ) -> Result<u64, ()> {
        let actor = admitted.runtime_actor.ok_or(())?;
        let mut runtime = self.runtime.lock().await;
        runtime.owner_fence().map_err(|_| ())?;
        if !runtime
            .player_control_facts(actor, admitted.game_session_id)
            .is_ok()
        {
            return Err(());
        }
        let at = super::bundle_map_position(&mut runtime, self.movement_cells(), actor).ok_or(())?;
        let cursor = at
            .map
            .facts
            .movement_effects_since(0)
            .last()
            .map_or(0, |row| row.0);
        let pos = crate::map::overlay::TilePos {
            x: at.at.x.try_into().map_err(|_| ())?,
            y: at.at.y.try_into().map_err(|_| ())?,
            floor: at.at.floor.try_into().map_err(|_| ())?,
        };
        if !at.map.facts.emit_movement_effect(pos, 11) {
            return Err(());
        }
        Ok(cursor)
    }
}
