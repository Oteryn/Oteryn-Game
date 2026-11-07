//! The immediate Charging Lua event shares its original cast stage and globals.
use super::*;
use crate::content::native_monster_callbacks::{
    QualifiedNativeCallback, QualifiedNativeCallbackSpawn,
};
use crate::foundation::{GameSessionId, RuntimeWorkStamp};
use crate::movement::source_placement::SourcePlacementPlan;
use crate::source_callback_cast::{CallbackStageId, CallbackStagePlan};
use crate::source_callback_spawn::CallbackSpawnOwner;
use oteryn_gameplay_entities::{
    CommittedLethalReceipt, NativeEncounterSpawnSpec, NativeNullHealthSample,
};

#[derive(Debug, Clone)]
pub(in crate::gameplay_transport) struct OutburstCastReceipt {
    pub caster: ExactActorRef,
    pub replacement: Option<ExactActorRef>,
    #[cfg(test)]
    pub global_health: u64,
    pub synchronized_health: Option<i64>,
    pub committed_null_lethal: bool,
    pub no_player_credit: bool,
}
#[derive(Debug)]
#[allow(
    dead_code,
    reason = "retained native NULL death proof awaits its production completion consumer"
)]
pub(in crate::gameplay_transport) struct OutburstNullDeath {
    pub actor: ExactActorRef,
    pub receipt: CommittedLethalReceipt,
    pub now_us: u64,
    pub stamp: RuntimeWorkStamp,
    pub pin: crate::foundation::ChannelContentPin,
    pub source: QualifiedNativeCallback,
    pub spawn_source: QualifiedNativeCallbackSpawn,
}
#[derive(Debug)]
struct Target {
    actor: ExactActorRef,
    session: GameSessionId,
    done: bool,
}
#[derive(Debug)]
struct Pending {
    id: CallbackStageId,
    proof: QualifiedNativeCallback,
    token: QualifiedNativeCallbackSpawn,
    spec: NativeEncounterSpawnSpec,
    child: Ref,
    context: crate::content::QualifiedCallbackSourceContext,
    sample: NativeNullHealthSample,
    placement: SourcePlacementPlan,
    center: MovementLocalPosition,
    destination: MovementLocalPosition,
    targets: Vec<Target>,
    prepared_at: Option<u64>,
    complete: bool,
    registered: bool,
    receipt: OutburstCastReceipt,
    death: Option<OutburstNullDeath>,
}
#[derive(Debug, Default)]
pub(super) struct OutburstCasts(Vec<Pending>);
impl OutburstCasts {
    #[cfg(test)]
    pub(super) fn receipt(&self, caster: ExactActorRef) -> Option<OutburstCastReceipt> {
        self.0
            .iter()
            .rev()
            .find(|p| p.receipt.caster == caster && p.complete)
            .map(|p| p.receipt.clone())
    }
    #[allow(
        dead_code,
        reason = "native NULL completion drain remains an explicit unfinished production port"
    )]
    pub(super) fn take_deaths(&mut self) -> Vec<OutburstNullDeath> {
        self.0
            .iter_mut()
            .filter_map(|pending| pending.death.take())
            .collect()
    }
    #[allow(
        clippy::too_many_arguments,
        reason = "one closed source event prepares current map facts before physical Combat"
    )]
    pub(super) async fn prepare(
        &mut self,
        active: &crate::content::ActiveGeneration,
        runtime: &mut ChannelRuntimeV1,
        globals: &outburst_state::OutburstState,
        room: &QualifiedNativeEntryRoom,
        native: &NativeGameplayState,
        map: &mut CurrentMonsterMap<'_, '_>,
        plan: &CallbackStagePlan,
        now: u64,
    ) -> Result<bool, ()> {
        if plan.source().creature_key != "oteryn:creature.charging_outburst" {
            return Ok(true);
        }
        if !self.0.iter().any(|p| p.id == plan.id()) {
            if self.0.len() >= 128 {
                let index = self
                    .0
                    .iter()
                    .position(|p| p.complete && p.death.is_none())
                    .ok_or(())?;
                self.0.remove(index);
            }
            self.0.try_reserve(1).map_err(|_| ())?;
            let context=active.qualify_callback_source_context(room,runtime.content_pin(),
                crate::content::bosses::activity_template::PinnedActivitySourceRoom::CallbackOutburst).ok_or(())?;
            let project = |at| {
                context
                    .source_frame()
                    .physical_position(at)
                    .map(|(x, y, floor)| MovementLocalPosition { x, y, floor })
                    .ok_or(())
            };
            let center = project([32234, 31284, 14])?;
            let destination = project([32234, 31280, 14])?;
            let spectators = project([32234, 31285, 14])?;
            let proof = plan.native_stage().ok_or(())?.parent().clone();
            let child = native
                .creature_profiles()
                .records
                .iter()
                .find(|r| r.profile.target.key == "oteryn:creature.outburst")
                .ok_or(())?
                .profile
                .target
                .clone();
            let token = native
                .callback_spawn_source(
                    &proof.entry().creature,
                    proof.entry().index,
                    crate::content::native_monster_callbacks::NativeCallbackList::Attack,
                    &child,
                )
                .map_err(|_| ())?
                .ok_or(())?;
            let spec = NativeEncounterSpawnSpec::qualified(
                &token.parent().key,
                &child.key,
                &child.revision,
                token.source_digest(),
            )
            .map_err(|_| ())?;
            let sample = if let Some(sample) = globals.source_health_sample() {
                sample.clone()
            } else if globals.source_health() == 0 {
                runtime
                    .native_null_health_default_sample(native, &proof)
                    .map_err(|_| ())?
            } else if globals.source_health() == 290000 {
                runtime
                    .native_null_health_entry_sample(native, &proof)
                    .map_err(|_| ())?
            } else {
                return Err(());
            };
            let seed = CallbackSpawnOwner::draw(
                native.source_digest(),
                plan.proposal(),
                "outburst-replacement-placement",
            )
            .map_err(|_| ())?;
            let mut counter = 0_u64;
            let placement =
                SourcePlacementPlan::prepare(center, token.extended(), token.forced(), |bound| {
                    let digest = Sha256::new()
                        .chain_update(seed.to_le_bytes())
                        .chain_update(counter.to_le_bytes())
                        .finalize();
                    counter = counter.saturating_add(1);
                    let mut bytes = [0; 8];
                    bytes.copy_from_slice(&digest[..8]);
                    crate::spell::uniform_draw(u64::from_le_bytes(bytes), 0, i64::from(bound - 1))
                        as u32
                })
                .map_err(|_| ())?;
            let census = runtime.positioned_actor_census().map_err(|_| ())?;
            let mut targets = Vec::new();
            targets.try_reserve_exact(census.len()).map_err(|_| ())?;
            for (actor, at, session) in census {
                let Some(session) = session else {
                    continue;
                };
                let at = at.position();
                if at.floor == spectators.floor
                    && (i64::from(at.x) - i64::from(spectators.x)).abs() <= 10
                    && (i64::from(at.y) - i64::from(spectators.y)).abs() <= 10
                {
                    targets.push(Target {
                        actor,
                        session,
                        done: false,
                    });
                }
            }
            self.0.push(Pending {
                id: plan.id(),
                proof,
                token,
                spec,
                child,
                context,
                sample,
                placement,
                center,
                destination,
                targets,
                prepared_at: None,
                complete: false,
                registered: false,
                receipt: OutburstCastReceipt {
                    caster: plan.proposal().issuer,
                    replacement: None,
                    #[cfg(test)]
                    global_health: globals.source_health(),
                    synchronized_health: None,
                    committed_null_lethal: false,
                    no_player_credit: false,
                },
                death: None,
            });
        }
        let pending = self.0.iter_mut().find(|p| p.id == plan.id()).ok_or(())?;
        if pending.complete {
            return Ok(true);
        }
        if !pending.context.matches(room, runtime.content_pin()) {
            return Err(());
        }
        map.read(runtime, pending.destination).await;
        let destination_exists = map
            .source_tile_exists_in_callback(runtime, pending.destination, &pending.context)
            .ok_or(())?;
        map.read(runtime, pending.center).await;
        let center_exists = map
            .source_tile_exists_in_callback(runtime, pending.center, &pending.context)
            .ok_or(())?;
        if !center_exists {
            for at in pending.placement.candidates() {
                map.read(runtime, *at).await;
                if map
                    .source_placement_cell_in_callback(
                        runtime,
                        &pending.child.key,
                        *at,
                        &pending.context,
                    )
                    .is_none()
                {
                    return Ok(false);
                }
            }
        }
        if !map.validate_transaction(runtime).await {
            return Err(());
        }
        for target in &mut pending.targets {
            if target.done {
                continue;
            }
            if runtime
                .player_control_facts(target.actor, target.session)
                .is_ok()
                && destination_exists
            {
                map.view()
                    .relocate_source_player(
                        runtime,
                        target.actor,
                        target.session,
                        pending.destination,
                    )
                    .map_err(|_| ())?;
            }
            target.done = true;
        }
        pending.prepared_at = Some(now);
        Ok(true)
    }
    #[allow(
        clippy::too_many_arguments,
        reason = "stage identity, current source globals, native policy, map, work stamp and real clock are independently checked"
    )]
    pub(super) fn commit(
        &mut self,
        id: CallbackStageId,
        runtime: &mut ChannelRuntimeV1,
        globals: &mut outburst_state::OutburstState,
        native: &NativeGameplayState,
        map: &CurrentMonsterMap<'_, '_>,
        stamp: RuntimeWorkStamp,
        now: u64,
    ) -> Result<(), crate::creature_auto_attack::AttackError> {
        use crate::creature_auto_attack::AttackError as E;
        let Some(p) = self.0.iter_mut().find(|p| p.id == id) else {
            return Ok(());
        };
        if p.complete {
            return Ok(());
        }
        if p.prepared_at != Some(now) {
            #[cfg(test)]
            eprintln!(
                "SRX3_CHARGING_POSTLUDE_CLOCK prepared={:?} current={now}",
                p.prepared_at
            );
            return Err(E::MissingCombatFacts);
        }
        globals.mark_exploded();
        let exists = map
            .source_tile_exists_in_callback(runtime, p.center, &p.context)
            .ok_or(E::MissingCombatFacts)?;
        let selected = if exists {
            Some(p.center)
        } else {
            p.placement
                .select(
                    |at| {
                        map.source_placement_cell_in_callback(runtime, &p.child.key, at, &p.context)
                    },
                    |_, _| Some(true),
                )
                .map_err(|_| E::MissingCombatFacts)?
        };
        if let Some(at) = selected {
            match runtime.admit_current_native_callback_masterless(
                native,
                stamp,
                p.receipt.caster,
                &p.spec,
                &p.token,
                p.center,
                at,
                |runtime, at| {
                    if at == p.center && p.token.forced() {
                        map.source_tile_exists_in_callback(runtime, at, &p.context)
                    } else {
                        map.source_placement_cell_in_callback(runtime, &p.child.key, at, &p.context)
                            .map(|cell| cell.query_add)
                    }
                },
            ) {
                Ok(actor) => {
                    p.receipt.replacement = Some(actor);
                    let snapshot = runtime
                        .companion_snapshot(actor)
                        .map_err(|_| E::StaleIssuer)?;
                    let occurrence = format!(
                        "native-outburst-cast-null:{:?}:{}",
                        p.receipt.caster.placement_identity(),
                        p.receipt.caster.actor_local_generation()
                    );
                    let mut binding = occurrence.as_bytes().to_vec();
                    binding.push(0);
                    binding.extend_from_slice(b"native-outburst-null-health:v1");
                    let result = runtime
                        .commit_native_null_health_sync_at(
                            native,
                            &p.proof,
                            &p.sample,
                            &snapshot,
                            occurrence.as_bytes(),
                            &binding,
                            now,
                        )
                        .map_err(|_| E::InvalidPlan)?;
                    p.receipt.synchronized_health = Some(result.health_after);
                    p.receipt.no_player_credit = runtime
                        .borrow_combat_death()
                        .current_session_damage_shares(actor)
                        .map_err(|_| E::InvalidPlan)?
                        .is_empty();
                    if result.health_after == 0 {
                        let receipt = runtime
                            .borrow_combat_death()
                            .committed_lethal_receipt(actor)
                            .map_err(|_| E::InvalidPlan)?;
                        p.receipt.committed_null_lethal = true;
                        p.death = Some(OutburstNullDeath {
                            actor,
                            receipt,
                            now_us: now,
                            stamp,
                            pin: runtime.content_pin().clone(),
                            source: p.proof.clone(),
                            spawn_source: p.token.clone(),
                        });
                    }
                }
                Err(oteryn_gameplay_entities::EncounterSpawnError::MissingMap) => {
                    return Err(E::MissingCombatFacts);
                }
                Err(oteryn_gameplay_entities::EncounterSpawnError::StaleOwner) => {
                    return Err(E::StaleOwner);
                }
                // Known Game.createMonster failure continues the original Lua event.
                Err(error) => {
                    #[cfg(test)]
                    eprintln!("SRX3_CHARGING_SOURCE_BIRTH_REFUSAL {error:?}");
                    let _ = error;
                }
            }
        }
        p.complete = true;
        Ok(())
    }
    pub(super) fn register_births(
        &mut self,
        runtime: &ChannelRuntimeV1,
        native: &NativeGameplayState,
        ai: &mut oteryn_gameplay_monster_ai::ai_think::CreatureAiTable,
        now: u64,
    ) -> Result<(), ()> {
        for pending in &mut self.0 {
            if !pending.complete || pending.registered {
                continue;
            }
            if let Some(actor) = pending
                .receipt
                .replacement
                .filter(|actor| runtime.contains_live_creature(*actor))
            {
                ai.admit_native_creature(runtime, native, actor, &pending.child, now)
                    .map_err(|_| ())?;
            }
            pending.registered = true;
        }
        Ok(())
    }
}
