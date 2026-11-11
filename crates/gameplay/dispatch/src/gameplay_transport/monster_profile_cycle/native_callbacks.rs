//! Source-qualified native callbacks selected by the existing profile scheduler.
use super::*;
mod bog;
mod casts;
mod conditions;
mod death_items;
mod eruption;
mod gaz;
mod omrafir;
mod outburst_cast;
mod outburst_entry;
mod outburst_events;
mod outburst_state;
mod smelly;
mod spawns;
mod teleports;
mod tenebris;
mod thresholds;
mod time_guardian;
mod vortex;
use crate::content::ProjectV2DefinitionRef as Ref;
use crate::creature_auto_attack::ProfileAbilityProposal;
use crate::foundation::{ExactActorRef, MovementLocalPosition};
use crate::monster_summon::{CurrentSourceCreatureRole, current_source_creature_role};
use oteryn_gameplay_encounters::bosses::seven_encounters::source_encounter_seven::{
    SevenKind, SevenSource,
};
use oteryn_gameplay_entities::{ActorMemoTable, CREATURE_DEFENCE_MEMOS_PER_CHANNEL_MAX};
use oteryn_simulation_determinism::{DecisionOccurrenceId, deterministic_decision_u64};
use sha2::{Digest, Sha256};

#[derive(Debug)]
struct Memo {
    proposal: ProfileAbilityProposal,
    sequence: u64,
}
#[derive(Debug, Default)]
pub(in crate::gameplay_transport) struct NativeCallbackState {
    spawns: spawns::NativeSpawns,
    time_guardian: time_guardian::NativeTimeGuardian,
    vortex: vortex::NativeVortex,
    thresholds: thresholds::NativeThresholds,
    teleports: teleports::NativeTeleports,
    casts: casts::NativeCasts,
    death_items: death_items::NativeDeathItems,
    outburst: outburst_state::OutburstState,
    outburst_events: outburst_events::NativeOutburstEvents,
    outburst_entry: Option<outburst_entry::Entry>,
    omrafir: omrafir::NativeOmrafir,
    gaz: gaz::NativeGaz,
    smelly: smelly::NativeSmelly,
    binding: Option<(
        crate::foundation::RuntimeScopeRefV1,
        crate::foundation::ScopeOwnershipGeneration,
        u64,
        [[u8; 32]; 4],
    )>,
    memos: ActorMemoTable<Ref, Memo>,
    catalog: Option<crate::creature_area_heal::CreatureHealCatalog>,
    source_context: Option<crate::content::QualifiedCallbackSourceContext>,
}
impl NativeCallbackState {
    #[cfg(test)]
    pub(in crate::gameplay_transport) fn source_cast_status_for_test(
        &self,
        issuer: crate::foundation::ExactActorRef,
    ) -> (bool, bool) {
        self.casts.source_parent_status_for_test(issuer)
    }
    #[cfg(test)]
    pub(in crate::gameplay_transport) fn time_guardian_cast_position_for_test(
        &self,
        actor: crate::foundation::ExactActorRef,
    ) -> Option<MovementLocalPosition> {
        self.time_guardian.cast_position_for_test(actor)
    }

    pub(in crate::gameplay_transport) fn bind_current(
        &mut self,
        runtime: &ChannelRuntimeV1,
        room: &QualifiedNativeEntryRoom,
        native: &NativeGameplayState,
    ) -> Result<(), ()> {
        if runtime.content_pin().server_artifact_digest() != native.source_digest()
            || runtime.content_pin().frame_binding_digest() != room.frame_binding().digest()
            || runtime.content_pin().map_revision_digest() != room.map_revision_digest()
        {
            return Err(());
        }
        let binding = (
            runtime.binding().runtime_scope(),
            runtime.binding().scope_generation(),
            runtime.content_pin().activation_sequence(),
            [
                native.source_digest(),
                runtime.content_pin().client_artifact_digest(),
                runtime.content_pin().frame_binding_digest(),
                runtime.content_pin().map_revision_digest(),
            ],
        );
        if self.binding != Some(binding) {
            *self = Self {
                binding: Some(binding),
                ..Default::default()
            };
        }
        Ok(())
    }
    pub(in crate::gameplay_transport) fn next_due_time(&self) -> Option<u64> {
        self.spawns
            .next_due()
            .into_iter()
            .chain(self.casts.next_due())
            .chain(self.time_guardian.next_due())
            .chain(self.vortex.next_due())
            .chain(self.thresholds.next_due())
            .chain(self.teleports.next_due())
            .chain(self.omrafir.next_due())
            .chain(self.gaz.next_due())
            .chain(self.smelly.next_due())
            .chain(self.outburst_events.next_due())
            .chain(self.death_items.next_due())
            .min()
    }
    #[allow(
        clippy::too_many_arguments,
        reason = "actual reserved think and retained birth work share one current native callback owner"
    )]
    pub(in crate::gameplay_transport) async fn think(
        &mut self,
        active: &crate::content::ActiveGeneration,
        runtime: &mut ChannelRuntimeV1,
        room: &QualifiedNativeEntryRoom,
        native: &NativeGameplayState,
        map: &mut CurrentMonsterMap<'_, '_>,
        ai: &mut oteryn_gameplay_monster_ai::ai_think::CreatureAiTable,
        occurrences: &[crate::foundation::ThinkOccurrence],
        now: u64,
    ) -> Result<(), ()> {
        self.bind_current(runtime, room, native)?;
        self.outburst_events
            .drive(
                &mut self.outburst,
                active,
                runtime,
                room,
                native,
                map,
                ai,
                occurrences,
                now,
            )
            .await?;
        self.omrafir
            .think(active, runtime, room, native, map, ai, occurrences, now)
            .await?;
        self.gaz.think(runtime, room, native, occurrences, now)
    }
    #[allow(
        clippy::too_many_arguments,
        reason = "selected source callbacks share current map, player HP and native physical readers"
    )]
    pub(in crate::gameplay_transport) async fn execute(
        &mut self,
        active: &crate::content::ActiveGeneration,
        runtime: &mut ChannelRuntimeV1,
        states: &mut ChannelSpellStates,
        attack: &mut ChannelAttackStates,
        room: &QualifiedNativeEntryRoom,
        native: &NativeGameplayState,
        map: &mut CurrentMonsterMap<'_, '_>,
        proposals: &[ProfileAbilityProposal],
        parties: &super::physical_spell::ReferenceParties,
        now: u64,
        native_health_names: &crate::gameplay_transport::source_names::NativeHealthNamesData,
    ) -> Result<(), ()> {
        self.bind_current(runtime, room, native)?;
        if let Some(expiry) = self.death_items.next_due().filter(|due| *due <= now) {
            self.drain_retained_due_through(
                active,
                runtime,
                states,
                attack,
                room,
                native,
                map,
                parties,
                expiry.saturating_sub(1),
                now,
                native_health_names,
            )
            .await?;
            return Err(());
        }

        self.omrafir
            .execute(
                active, runtime, states, attack, room, native, map, proposals, parties, now,
            )
            .await?;
        self.thresholds
            .drive(runtime, room, native, proposals, now)?;
        self.teleports
            .drive(active, runtime, room, native, map, proposals, now)
            .await?;
        self.spawns
            .drive(active, runtime, room, native, map, proposals, now)
            .await?;
        self.casts
            .drive(
                active,
                &mut self.outburst,
                runtime,
                states,
                attack,
                room,
                native,
                map,
                proposals,
                parties,
                now,
                native_health_names,
            )
            .await?;
        self.smelly
            .drive(runtime, states, attack, room, native, map, proposals, now)
            .await?;
        self.time_guardian
            .drive(active, runtime, room, native, map, proposals, now)
            .await?;
        self.vortex
            .drive(active, runtime, room, native, map, proposals, now)
            .await?;
        if !proposals.iter().any(|proposal| {
            proposal.ability.key == "oteryn:ability.creature.cerebellum.field-fill-heal_brain_head"
        }) {
            return Ok(());
        }
        if !self
            .source_context
            .as_ref()
            .is_some_and(|proof| proof.matches(room, runtime.content_pin()))
        {
            self.source_context = active.qualify_callback_source_context(
                room,
                runtime.content_pin(),
                crate::content::bosses::activity_template::PinnedActivitySourceRoom::BrainHead,
            );
        }
        let Some(context) = self.source_context.as_ref() else {
            return Ok(());
        };
        let Some((x, y, floor)) = context.source_frame().physical_position([31954, 32325, 10])
        else {
            return Ok(());
        };
        let target_position = MovementLocalPosition { x, y, floor };
        for p in proposals {
            if p.ability.key != "oteryn:ability.creature.cerebellum.field-fill-heal_brain_head" {
                continue;
            }
            if !matches!(
                current_source_creature_role(runtime, p.issuer),
                Ok(CurrentSourceCreatureRole::Ordinary
                    | CurrentSourceCreatureRole::IntrinsicMonsterSummon(_))
            ) {
                continue;
            }
            let Ok(snapshot) = runtime.companion_snapshot(p.issuer) else {
                continue;
            };
            let Some(record) = native.creature_profiles().records.iter().find(|record| {
                record.profile.target.key == snapshot.state.policy.definition_key
                    && record.profile.target.revision == snapshot.state.policy.definition_revision
            }) else {
                continue;
            };
            let Ok(Some(source)) =
                SevenSource::qualify_native(native, &record.profile.target, p.entry_index, p.list)
            else {
                continue;
            };
            if oteryn_gameplay_encounters::bosses::source_callbacks::source_callback_intent::validate(runtime,p).is_err() {continue;}
            let mut normalized = p.clone();
            normalized.target = p.issuer;
            let atom = format!(
                "actor:{}",
                p.issuer
                    .placement_identity()
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect::<String>()
            );
            let Ok(intent) = crate::ability::AiAbilityAdapter::normalize(&atom, &[&atom]) else {
                continue;
            };
            normalized.intent = intent;
            let Ok(stamp) = runtime.issue_owner_work() else {
                continue;
            };
            let Ok(fence) = runtime.owner_fence() else {
                continue;
            };
            let Ok(work) = source.prepare_map_work(runtime, fence, stamp, &normalized) else {
                continue;
            };
            if work.kind != SevenKind::BrainHeal {
                continue;
            }
            if let Some(previous) = self.memos.get(p.issuer, &p.ability) {
                if previous.sequence >= work.sequence {
                    continue;
                }
                // A fresh scheduler occurrence must retain the original native proposal.
                if previous.proposal.issuer != p.issuer {
                    continue;
                }
            }
            self.memos.evict_stale_at(p.issuer);
            if self.memos.get(p.issuer, &p.ability).is_none() {
                if self.memos.len() >= CREATURE_DEFENCE_MEMOS_PER_CHANNEL_MAX {
                    self.memos
                        .prune(|actor| runtime.contains_live_creature(actor));
                }
                if self.memos.len() >= CREATURE_DEFENCE_MEMOS_PER_CHANNEL_MAX {
                    continue;
                }
            }
            let at = target_position;
            if map.read(runtime, at).await.is_none() || !map.validate_transaction(runtime).await {
                continue;
            }
            let Ok(top) = runtime.current_top_actor_on_cell(at) else {
                continue;
            };
            let Some(origin) = runtime
                .read_actor_position(p.issuer)
                .ok()
                .map(|position| position.position())
            else {
                continue;
            };
            let Some(from) = visual_position(origin) else {
                continue;
            };
            let Some(to) = visual_position(at) else {
                continue;
            };
            let (
                Some(crate::spell::spell_presentations::Cue::Projectile(missile)),
                Some(crate::spell::spell_presentations::Cue::Effect(effect)),
            ) = (
                crate::spell::spell_presentations::resolve_source_cue(
                    "canary.appearance:missile/holy",
                ),
                crate::spell::spell_presentations::resolve_source_cue(
                    "canary.appearance:effect/magic_blue",
                ),
            )
            else {
                continue;
            };
            let Some(missile) = crate::map::facts::CreatureVisualCue::missile(
                runtime, stamp, p.issuer, from, to, missile,
            ) else {
                continue;
            };
            let Some(effect) =
                crate::map::facts::CreatureVisualCue::effect(runtime, stamp, p.issuer, to, effect)
            else {
                continue;
            };
            if let Some(target) = top.filter(|actor| {
                runtime.matches_live_creature_identity(*actor, b"oteryn:creature.brain_head")
            }) {
                if self.catalog.is_none() {
                    self.catalog =
                        crate::creature_area_heal::CreatureHealCatalog::from_current_native(
                            runtime, native,
                        )
                        .ok();
                }
                let Some(catalog) = self.catalog.as_ref() else {
                    continue;
                };
                let Ok((key, max)) = catalog.current_target(runtime, target) else {
                    continue;
                };
                let hash = Sha256::new()
                    .chain_update(b"SRX3_SOURCE_BRAIN_HEAL/v1")
                    .chain_update(p.occurrence.id().as_str().as_bytes())
                    .chain_update(p.issuer.placement_identity())
                    .chain_update(target.placement_identity())
                    .finalize();
                let mut id = [0; 16];
                id.copy_from_slice(&hash[..16]);
                let root = GameplayDecisionRoot::from_bytes(native.source_digest());
                let Ok(draw) = deterministic_decision_u64(
                    &root,
                    DecisionOccurrenceId::from_bytes(id),
                    "source_heal_magnitude",
                    p.entry_index as u64,
                ) else {
                    continue;
                };
                let draw =
                    oteryn_gameplay_domain::domain::uniform_draw::uniform_draw(draw, 300, 500)
                        as u64;
                if runtime
                    .commit_source_creature_heal_batch(
                        native.source_digest(),
                        &[(target, key, max, draw)],
                    )
                    .is_err()
                {
                    continue;
                }
            }
            use crate::map::boot::BundleCollisionIndexExt;
            if let crate::content::native_cell_lookup::NativeMovementCollisionIndex::Bundle(
                collision,
            ) = room.movement_cells().index()
            {
                collision.map().facts.emit_creature_visual(runtime, missile);
                collision.map().facts.emit_creature_visual(runtime, effect);
            }
            self.memos.insert(
                p.issuer,
                p.ability.clone(),
                Memo {
                    proposal: p.clone(),
                    sequence: work.sequence,
                },
            );
        }
        Ok(())
    }
}

fn visual_position(position: MovementLocalPosition) -> Option<crate::map::overlay::TilePos> {
    Some(crate::map::overlay::TilePos {
        x: u16::try_from(position.x).ok()?,
        y: u16::try_from(position.y).ok()?,
        floor: i8::try_from(position.floor).ok()?.checked_neg()?,
    })
}

#[cfg(test)]
mod retention_tests;

/// The canonical callback owner remains installed while its current Map reader
/// awaits. Cancelling a turn drops only the lease, preserving original timers.
#[derive(Debug, Clone, Default)]
pub(in crate::gameplay_transport) struct NativeCallbacks {
    owner: std::sync::Arc<tokio::sync::Mutex<NativeCallbackState>>,
}
impl NativeCallbacks {
    /// Source-pad qualification is independent of final entry acceptance.
    /// Effects remain neutral until the source prefix regression executes.
    pub(in crate::gameplay_transport) async fn apply_outburst_pad_extra(
        &self,
        runtime: &mut ChannelRuntimeV1,
        active: &crate::content::ActiveGeneration,
        room: &QualifiedNativeEntryRoom,
        native: &NativeGameplayState,
        extra: &crate::durability::activity_handoff::boss_room::QualifiedBossPadExtra,
    ) -> Result<(), ()> {
        use crate::durability::activity_handoff::boss_room::SourceBossLeverExtra;
        if extra.kind() != SourceBossLeverExtra::Outburst {
            return Err(());
        }
        let source = extra.source();
        let position = extra.position();
        let pin = runtime.content_pin();
        if source.runtime_scope != runtime.binding().runtime_scope()
            || source.scope_ownership_generation != runtime.binding().scope_generation()
            || position.world_id != pin.world_id()
            || position.coordinate_frame_digest != pin.frame_binding_digest()
            || position.map_revision_digest != pin.map_revision_digest()
            || position.server_artifact_digest != pin.server_artifact_digest()
            || extra.pad_ordinal() >= 5
        {
            return Err(());
        }
        let context=active.qualify_callback_source_context(room,pin,
            crate::content::bosses::activity_template::PinnedActivitySourceRoom::CallbackOutburst).ok_or(())?;
        let expected = context
            .source_frame()
            .physical_position([32207, 31284 + i32::from(extra.pad_ordinal()), 14])
            .ok_or(())?;
        let (actor, current) = runtime
            .positioned_player_for_session(source.game_session_id)
            .map_err(|_| ())?
            .ok_or(())?;
        let actual = current.position();
        let lease = runtime
            .bound_attacker_lease(actor, source.game_session_id)
            .map_err(|_| ())?;
        if lease.character_id().as_bytes() != source.character_id.as_bytes()
            || lease.generation() != source.character_lease_generation
            || (actual.x, actual.y, actual.floor) != expected
            || position.cell.x != actual.x
            || position.cell.y != actual.y
            || position.cell.floor != actual.floor
        {
            return Err(());
        }
        let mut owner = self.owner.lock().await;
        owner.bind_current(runtime, room, native)?;
        // The real source effect is deliberately absent in this baseline.
        Ok(())
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "accepted source party handoff retains its exact runtime, room, AI, prefix and receipt identity"
    )]
    pub(in crate::gameplay_transport) fn initialize_outburst_accepted_entry(
        &self,
        runtime: &mut ChannelRuntimeV1,
        active: &crate::content::ActiveGeneration,
        room: &QualifiedNativeEntryRoom,
        native: &NativeGameplayState,
        ai: &mut oteryn_gameplay_monster_ai::ai_think::CreatureAiTable,
        prefix: &crate::durability::activity_handoff::boss_room::QualifiedBossPadPrefix,
        handoff: [u8; 16],
        now: u64,
    ) -> Result<(), ()> {
        let mut owner = self.owner.try_lock().map_err(|_| ())?;
        owner.bind_current(runtime, room, native)?;
        outburst_entry::initialize(
            &mut owner, runtime, active, room, native, ai, prefix, handoff, now,
        )
    }
    pub(in crate::gameplay_transport) fn prepare_source_death_action(
        &self,
        runtime: &mut ChannelRuntimeV1,
        room: &QualifiedNativeEntryRoom,
        native: &NativeGameplayState,
        action: &crate::gameplay_transport::source_death::PreparedSourceDeathAction,
    ) -> Result<bool, ()> {
        if !action.current_for(runtime) {
            return Err(());
        }
        let Ok(mut owner) = self.owner.try_lock() else {
            return Ok(false);
        };
        owner.bind_current(runtime, room, native)?;
        owner.death_items.prepare(runtime, room, native, action)
    }
    pub(in crate::gameplay_transport) async fn finish_source_death_action(
        &self,
        admission: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
        permit: &crate::durability::spell_owner_commit::SpellLanePermit,
        action: &crate::gameplay_transport::source_death::PreparedSourceDeathAction,
    ) -> Result<bool, ()> {
        let callback = { self.owner.lock().await.death_items.callback(action)? };
        let Some(callback) = callback else {
            return Ok(true);
        };
        let result = admission
            .root
            .commit_w01_death_ground(permit, admission.holder, &callback, action.now_us())
            .await;
        let receipt = match result {
            Ok(receipt) => receipt,
            Err(error) => {
                #[cfg(test)]
                eprintln!(
                    "SRX3_W01_DEATH_ITEM_REFUSED event={} error={error:?}",
                    action.event().name()
                );
                #[cfg(not(test))]
                let _ = error;
                return Err(());
            }
        };
        admission.states.loop_ground.invalidate();
        self.owner
            .lock()
            .await
            .death_items
            .record(action, receipt.into_expiry())?;
        Ok(true)
    }
    pub(in crate::gameplay_transport) fn publish_source_death_action(
        &self,
        runtime: &mut ChannelRuntimeV1,
        action: &crate::gameplay_transport::source_death::PreparedSourceDeathAction,
    ) -> Result<bool, ()> {
        if !action.current_for(runtime) {
            return Err(());
        }
        let Ok(mut owner) = self.owner.try_lock() else {
            return Ok(false);
        };
        owner.death_items.publish(runtime, action)
    }
    pub(in crate::gameplay_transport) fn due_death_item(
        &self,
        runtime: &ChannelRuntimeV1,
        room: &QualifiedNativeEntryRoom,
        native: &NativeGameplayState,
        cutoff: u64,
        now: u64,
    ) -> Result<
        Option<std::sync::Arc<crate::durability::creature_source_items::W01DeathGroundCallback>>,
        (),
    > {
        let mut owner = self.owner.try_lock().map_err(|_| ())?;
        owner.bind_current(runtime, room, native)?;
        let Some(callback) = owner.death_items.due(runtime, cutoff, now) else {
            return Ok(None);
        };
        if owner
            .next_due_time()
            .is_some_and(|other| other < callback.due_us())
        {
            return Ok(None);
        }
        Ok(Some(callback))
    }
    pub(in crate::gameplay_transport) fn complete_death_item(
        &self,
        runtime: &ChannelRuntimeV1,
        callback: &crate::durability::creature_source_items::W01DeathGroundCallback,
        now: u64,
    ) -> Result<(), ()> {
        self.owner
            .try_lock()
            .map_err(|_| ())?
            .death_items
            .finish_expiry(runtime, callback, now)
    }
    /// Execute only previously retained callback work ordered before a source
    /// death notice. `now` remains the real execution clock; the cutoff only
    /// selects original pending deadlines and never creates a native think.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::gameplay_transport) async fn drain_due_through(
        &self,
        active: &crate::content::ActiveGeneration,
        runtime: &mut ChannelRuntimeV1,
        states: &mut ChannelSpellStates,
        attack: &mut ChannelAttackStates,
        room: &QualifiedNativeEntryRoom,
        native: &NativeGameplayState,
        map: &mut CurrentMonsterMap<'_, '_>,
        parties: &super::physical_spell::ReferenceParties,
        eligible_through_us: u64,
        now: u64,
        native_health_names: &crate::gameplay_transport::source_names::NativeHealthNamesData,
    ) -> Result<(), ()> {
        let mut owner = self.owner.lock().await;
        owner.bind_current(runtime, room, native)?;
        owner
            .drain_retained_due_through(
                active,
                runtime,
                states,
                attack,
                room,
                native,
                map,
                parties,
                eligible_through_us,
                now,
                native_health_names,
            )
            .await
    }
    #[cfg(test)]
    #[allow(
        clippy::expect_used,
        reason = "tests read only after the current source turn releases its lease"
    )]
    pub(in crate::gameplay_transport) fn eruption_master_receipt_for_test(
        &self,
        caster: crate::foundation::ExactActorRef,
    ) -> Option<eruption::EruptionMasterReceipt> {
        self.owner
            .try_lock()
            .expect("current callback owner is idle")
            .casts
            .eruption_receipt_for_test(caster)
    }
    #[cfg(test)]
    pub(in crate::gameplay_transport) fn outburst_cast_receipt_for_test(
        &self,
        caster: ExactActorRef,
    ) -> Option<outburst_cast::OutburstCastReceipt> {
        self.owner.try_lock().ok()?.casts.outburst_receipt(caster)
    }
    #[allow(
        dead_code,
        reason = "native NULL completion drain remains an explicit unfinished production port"
    )]
    pub(in crate::gameplay_transport) async fn take_outburst_null_deaths(
        &self,
    ) -> Vec<outburst_cast::OutburstNullDeath> {
        self.owner.lock().await.casts.take_outburst_null_deaths()
    }
    pub(in crate::gameplay_transport) fn acknowledge_outburst_null_death(
        &self,
        runtime: &ChannelRuntimeV1,
        completed: &oteryn_gameplay_entities::carrier::native_source_death::SourceCreatureDeathCompletion,
    ) -> Result<(), ()> {
        self.owner
            .try_lock()
            .map_err(|_| ())?
            .casts
            .acknowledge_outburst_null_death(runtime, completed)
    }
    pub(in crate::gameplay_transport) fn retained_owner(
        &self,
    ) -> std::sync::Arc<tokio::sync::Mutex<NativeCallbackState>> {
        std::sync::Arc::clone(&self.owner)
    }
    pub(in crate::gameplay_transport) fn next_due_time(&self) -> Option<u64> {
        self.owner
            .try_lock()
            .map_or(Some(0), |owner| owner.next_due_time())
    }
    /// Source-death ordering needs a known original deadline. The ordinary
    /// scheduler's zero wake on a busy owner is not a historical callback.
    pub(in crate::gameplay_transport) fn next_source_due_time(&self) -> Result<Option<u64>, ()> {
        Ok(self.owner.try_lock().map_err(|_| ())?.next_due_time())
    }
    #[cfg(test)]
    pub(in crate::gameplay_transport) async fn enable_omrafir_declared_think_review_for_test(
        &self,
        runtime: &ChannelRuntimeV1,
        room: &QualifiedNativeEntryRoom,
        native: &NativeGameplayState,
    ) -> Result<(), ()> {
        let mut owner = self.owner.lock().await;
        owner.bind_current(runtime, room, native)?;
        owner.omrafir.enable_declared_think_review();
        Ok(())
    }
    #[cfg(test)]
    pub(in crate::gameplay_transport) async fn omrafir_terminal_failure_for_test(
        &self,
        actor: ExactActorRef,
    ) -> Option<&'static str> {
        self.owner
            .lock()
            .await
            .omrafir
            .terminal_failure_for_test(actor)
    }
    pub(in crate::gameplay_transport) async fn expire_native_conditions(
        &self,
        runtime: &mut ChannelRuntimeV1,
        room: &QualifiedNativeEntryRoom,
        native: &NativeGameplayState,
        actors: &[ExactActorRef],
        now: u64,
    ) -> Result<(), ()> {
        let mut owner = self.owner.lock().await;
        owner.bind_current(runtime, room, native)?;
        conditions::expire(runtime, native, actors, now)?;
        Ok(())
    }
    pub(in crate::gameplay_transport) fn has_native_work(&self) -> bool {
        self.next_due_time().is_some()
    }
    pub(in crate::gameplay_transport) fn has_native_due(&self, now: u64) -> bool {
        self.next_due_time().is_some_and(|due| due <= now)
    }
    #[cfg(test)]
    pub(in crate::gameplay_transport) fn with_source_cast_owner_for_test(
        owner: oteryn_gameplay_encounters::bosses::source_callbacks::source_callback_cast::CallbackCastOwner,
    ) -> Self {
        Self {
            owner: std::sync::Arc::new(tokio::sync::Mutex::new(NativeCallbackState {
                casts: casts::NativeCasts::from_owner_for_test(owner),
                ..Default::default()
            })),
        }
    }
    #[cfg(test)]
    #[allow(
        clippy::expect_used,
        reason = "tests read only after dropping the current owner lease"
    )]
    pub(in crate::gameplay_transport) fn source_cast_status_for_test(
        &self,
        issuer: crate::foundation::ExactActorRef,
    ) -> (bool, bool) {
        self.owner
            .try_lock()
            .expect("current callback owner is idle")
            .source_cast_status_for_test(issuer)
    }
    #[cfg(test)]
    pub(in crate::gameplay_transport) async fn source_stage_done_for_test(
        &self,
        issuer: ExactActorRef,
    ) -> Option<Vec<bool>> {
        self.owner
            .lock()
            .await
            .casts
            .source_stage_done_for_test(issuer)
    }
    #[cfg(test)]
    #[allow(
        clippy::expect_used,
        reason = "tests read only after dropping the current owner lease"
    )]
    pub(in crate::gameplay_transport) fn time_guardian_cast_position_for_test(
        &self,
        actor: crate::foundation::ExactActorRef,
    ) -> Option<MovementLocalPosition> {
        self.owner
            .try_lock()
            .expect("current callback owner is idle")
            .time_guardian_cast_position_for_test(actor)
    }
    #[allow(
        clippy::too_many_arguments,
        reason = "same actual reserved native callback transaction"
    )]
    pub(in crate::gameplay_transport) async fn think(
        &self,
        active: &crate::content::ActiveGeneration,
        runtime: &mut ChannelRuntimeV1,
        room: &QualifiedNativeEntryRoom,
        native: &NativeGameplayState,
        map: &mut CurrentMonsterMap<'_, '_>,
        ai: &mut oteryn_gameplay_monster_ai::ai_think::CreatureAiTable,
        occurrences: &[crate::foundation::ThinkOccurrence],
        now: u64,
    ) -> Result<(), ()> {
        self.retained_owner()
            .lock_owned()
            .await
            .think(active, runtime, room, native, map, ai, occurrences, now)
            .await
    }
    #[allow(
        clippy::too_many_arguments,
        reason = "same current physical callback transaction"
    )]
    pub(in crate::gameplay_transport) async fn execute(
        &self,
        active: &crate::content::ActiveGeneration,
        runtime: &mut ChannelRuntimeV1,
        states: &mut ChannelSpellStates,
        attack: &mut ChannelAttackStates,
        room: &QualifiedNativeEntryRoom,
        native: &NativeGameplayState,
        map: &mut CurrentMonsterMap<'_, '_>,
        proposals: &[ProfileAbilityProposal],
        parties: &super::physical_spell::ReferenceParties,
        now: u64,
        native_health_names: &crate::gameplay_transport::source_names::NativeHealthNamesData,
    ) -> Result<(), ()> {
        self.retained_owner()
            .lock_owned()
            .await
            .execute(
                active,
                runtime,
                states,
                attack,
                room,
                native,
                map,
                proposals,
                parties,
                now,
                native_health_names,
            )
            .await
    }
}

impl NativeCallbackState {
    #[allow(
        clippy::too_many_arguments,
        reason = "same existing retained callback arbiter shares its current combat/map owners"
    )]
    async fn drain_retained_due_through(
        &mut self,
        active: &crate::content::ActiveGeneration,
        runtime: &mut ChannelRuntimeV1,
        states: &mut ChannelSpellStates,
        attack: &mut ChannelAttackStates,
        room: &QualifiedNativeEntryRoom,
        native: &NativeGameplayState,
        map: &mut CurrentMonsterMap<'_, '_>,
        parties: &super::physical_spell::ReferenceParties,
        eligible_through_us: u64,
        now: u64,
        native_health_names: &crate::gameplay_transport::source_names::NativeHealthNamesData,
    ) -> Result<(), ()> {
        // Recompute the minimum after each retained family advances; do not
        // run a later deadline merely because its leaf appears earlier here.
        for _ in 0..512 {
            let pending = [
                self.casts.next_due(),
                self.smelly.next_due(),
                self.time_guardian.next_due(),
                self.omrafir.next_due(),
                self.thresholds.next_due(),
                self.teleports.next_due(),
                self.spawns.next_due(),
                self.vortex.next_due(),
                self.outburst_events.next_due(),
                self.death_items.next_due(),
            ];
            let Some((family, through)) = pending
                .into_iter()
                .enumerate()
                .filter_map(|(family, due)| due.map(|due| (family, due)))
                .filter(|(_, due)| *due <= eligible_through_us && *due <= now)
                .min_by_key(|(_, due)| *due)
            else {
                return Ok(());
            };
            match family {
                0 => {
                    let NativeCallbackState {
                        casts, outburst, ..
                    } = &mut *self;
                    casts
                        .drive_due_through(
                            active,
                            outburst,
                            runtime,
                            states,
                            attack,
                            room,
                            native,
                            map,
                            parties,
                            through,
                            now,
                            native_health_names,
                        )
                        .await?;
                }
                1 => {
                    self.smelly
                        .drive_due_through(runtime, states, attack, room, native, map, through, now)
                        .await?
                }
                2 => {
                    self.time_guardian
                        .drive_due_through(active, runtime, room, native, map, through, now)
                        .await?
                }
                3 => {
                    self.omrafir
                        .drive_due_through(
                            runtime, states, attack, room, native, map, parties, through, now,
                        )
                        .await?
                }
                4 => self
                    .thresholds
                    .drive_due_through(runtime, room, native, through, now)?,
                5 => {
                    self.teleports
                        .drive_due_through(active, runtime, room, native, map, through, now)
                        .await?
                }
                6 => {
                    self.spawns
                        .drive_due_through(active, runtime, room, native, map, through, now)
                        .await?
                }
                7 => {
                    self.vortex
                        .drive_due_through(active, runtime, room, native, map, through, now)
                        .await?
                }
                8 => {
                    let NativeCallbackState {
                        outburst,
                        outburst_events,
                        ..
                    } = &mut *self;
                    outburst_events
                        .drive_due_through(
                            outburst,
                            active,
                            runtime,
                            room,
                            native,
                            map,
                            &mut states.monster_melee.creature_ai,
                            through,
                            now,
                        )
                        .await?;
                }
                9 => return Err(()),
                _ => return Err(()),
            }
            let after = match family {
                0 => self.casts.next_due(),
                1 => self.smelly.next_due(),
                2 => self.time_guardian.next_due(),
                3 => self.omrafir.next_due(),
                4 => self.thresholds.next_due(),
                5 => self.teleports.next_due(),
                6 => self.spawns.next_due(),
                7 => self.vortex.next_due(),
                8 => self.outburst_events.next_due(),
                _ => return Err(()),
            };
            if after.is_some_and(|due| due <= through) {
                // Unknown current facts leave the original occurrence intact.
                return Err(());
            }
        }
        // Bounded progress remains retained; never begin a later source death
        // while an older timer is still pending.
        Err(())
    }
}
