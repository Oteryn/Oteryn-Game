//! The selected native callback owns its original delayed stages. Current facts
//! are prepared separately for each stage, including saved source positions.
use super::super::physical_spell::{self, ReferenceParties};
use super::*;
use crate::creature_damage_spell::SpellWorldReader;
use crate::foundation::owner_timer::{OwnerClock, SemanticTimeMicros};
use oteryn_gameplay_encounters::bosses::source_callbacks::source_callback_cast::{
    CallbackCastOwner, CallbackStageId, CallbackStageReaders, SourceCallbackCast,
};

#[derive(Default)]
pub(super) struct NativeCasts {
    owner: Option<CallbackCastOwner>,
    eruption: super::eruption::EruptionPreludes,
    tenebris: super::tenebris::TenebrisPreludes,
    outburst: super::outburst_cast::OutburstCasts,
}
impl std::fmt::Debug for NativeCasts {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("NativeCasts")
            .field(
                "pending",
                &self.owner.as_ref().map_or(0, |owner| owner.casts.len()),
            )
            .finish_non_exhaustive()
    }
}
struct Now(u64);
impl OwnerClock for Now {
    fn now(&self) -> SemanticTimeMicros {
        SemanticTimeMicros::from_micros(self.0)
    }
}
struct StageReader<'native, 'turn> {
    id: CallbackStageId,
    reader: physical_spell::composed::CurrentComposed<'native, 'turn>,
    source: crate::creature_damage_spell::composed::ComposedSource,
    proposal: ProfileAbilityProposal,
    eruption_prelude_completed: bool,
    cues: Vec<(
        Option<crate::foundation::ExactActorRef>,
        crate::map::facts::CreatureVisualCue,
    )>,
}
type StagePostlude<'a> = dyn FnMut(
        CallbackStageId,
        &mut ChannelRuntimeV1,
        crate::foundation::RuntimeWorkStamp,
        SemanticTimeMicros,
    ) -> Result<(), crate::creature_auto_attack::AttackError>
    + 'a;
struct Readers<'native, 'turn, 'map, 'post> {
    facts: Option<&'map crate::map::facts::BundleFacts>,
    postlude: &'post mut StagePostlude<'post>,
    stages: Vec<StageReader<'native, 'turn>>,
    terminal: Vec<(CallbackStageId, crate::creature_auto_attack::AttackError)>,
    owner: crate::creature_damage_spell::composed::ComposedOwner,
    catalog: crate::creature_area_heal::CreatureHealCatalog,
}
impl CallbackStageReaders for Readers<'_, '_, '_, '_> {
    fn terminal_error(
        &self,
        stage: CallbackStageId,
    ) -> Option<crate::creature_auto_attack::AttackError> {
        self.terminal
            .iter()
            .find(|(id, _)| *id == stage)
            .map(|(_, error)| *error)
    }
    fn ready(&self, stage: CallbackStageId) -> bool {
        self.stages.iter().any(|entry| entry.id == stage)
    }
    fn reader(&mut self, stage: CallbackStageId) -> Option<&mut (dyn SpellWorldReader + '_)> {
        for entry in &mut self.stages {
            if entry.id == stage {
                return Some(&mut entry.reader.base);
            }
        }
        None
    }
    fn execute_current(
        &mut self,
        stage: CallbackStageId,
        runtime: &mut ChannelRuntimeV1,
        stamp: crate::foundation::RuntimeWorkStamp,
        vitals: &mut dyn crate::player_lethal::PlayerLethalVitals,
        now: SemanticTimeMicros,
    ) -> Option<
        Result<
            crate::creature_damage_spell::SpellOutcome,
            crate::creature_auto_attack::AttackError,
        >,
    > {
        let entry = self.stages.iter_mut().find(|entry| entry.id == stage)?;
        // Canary/Crystal eruption_of_destruction_explosion.lua dereferences
        // master:addHealth before creating Demon or executing Combat. A missing
        // master aborts that one Lua event; it never grants explosion/removal.
        if entry.source.creature_key() == "oteryn:creature.eruption_of_destruction"
            && !entry.eruption_prelude_completed
            && !current_source_creature_role(runtime, entry.proposal.issuer)
                .ok()
                .and_then(|role| role.intrinsic_master())
                .is_some_and(|master| runtime.contains_live_creature(master))
        {
            return Some(Err(crate::creature_auto_attack::AttackError::InvalidSource));
        }
        let result = self
            .owner
            .execute_current(
                runtime,
                stamp,
                vitals,
                &self.catalog,
                &entry.source,
                &entry.proposal,
                &mut entry.reader,
                now,
            )
            .and_then(|outcome| {
                outcome
                    .primary
                    .ok_or(crate::creature_auto_attack::AttackError::InvalidPlan)
            });
        #[cfg(test)]
        if entry.source.creature_key() == "oteryn:creature.foam_stalker" {
            eprintln!(
                "SRX3_RETAINED_FOAM_COMBAT players={} result={result:?}",
                entry.reader.base.players.len()
            );
        }
        #[cfg(test)]
        if entry.source.creature_key() == "oteryn:creature.charging_outburst" {
            eprintln!(
                "SRX3_CHARGING_COMBAT stage={stage:?} result={:?}",
                result.as_ref().map(|_| ())
            );
        }
        if let (Some(facts), Ok(outcome)) = (self.facts, &result) {
            for (target, cue) in entry.cues.drain(..) {
                if target.is_none_or(|target| {
                    outcome
                        .targets
                        .iter()
                        .any(|(actor, hit)| *actor == target && hit.is_ok())
                }) {
                    facts.emit_creature_visual(runtime, cue);
                }
            }
        }
        let result = result.and_then(|outcome| {
            let postlude = (self.postlude)(stage, runtime, stamp, now);
            #[cfg(test)]
            if entry.source.creature_key() == "oteryn:creature.charging_outburst" {
                eprintln!("SRX3_CHARGING_POSTLUDE stage={stage:?} result={postlude:?}");
            }
            postlude?;
            Ok(outcome)
        });
        Some(result)
    }
}
fn callback(proposal: &ProfileAbilityProposal) -> bool {
    matches!(
        proposal.ability.key.as_str(),
        "oteryn:ability.spell.outburst_explode"
            | "oteryn:ability.spell.energy_pulse_explosion"
            | "oteryn:ability.spell.eruption_of_destruction_explosion"
            | "oteryn:ability.creature.foam_stalker.field-fill-foamsplash"
            | "oteryn:ability.spell.gaz_haragoth_death"
            | "oteryn:ability.spell.tenebris_ultimate"
            | "oteryn:ability.spell.plagirath_bog"
            | "oteryn:ability.spell.rotthingshaper"
            | "oteryn:ability.spell.rotthingwave"
    )
}
impl NativeCasts {
    #[cfg(test)]
    pub(super) fn eruption_receipt_for_test(
        &self,
        caster: crate::foundation::ExactActorRef,
    ) -> Option<super::eruption::EruptionMasterReceipt> {
        self.eruption.receipt(caster)
    }
    #[cfg(test)]
    pub(super) fn from_owner_for_test(owner: CallbackCastOwner) -> Self {
        Self {
            owner: Some(owner),
            ..Default::default()
        }
    }
    #[cfg(test)]
    pub(super) fn source_parent_status_for_test(
        &self,
        issuer: crate::foundation::ExactActorRef,
    ) -> (bool, bool) {
        self.owner.as_ref().map_or((false, false), |owner| {
            owner.source_parent_status_for_test(issuer)
        })
    }
    #[cfg(test)]
    pub(super) fn source_stage_done_for_test(&self, issuer: ExactActorRef) -> Option<Vec<bool>> {
        self.owner.as_ref()?.source_stage_done_for_test(issuer)
    }
    #[cfg(test)]
    pub(super) fn outburst_receipt(
        &self,
        caster: ExactActorRef,
    ) -> Option<super::outburst_cast::OutburstCastReceipt> {
        self.outburst.receipt(caster)
    }
    #[allow(
        dead_code,
        reason = "native NULL completion drain remains an explicit unfinished production port"
    )]
    pub(super) fn take_outburst_null_deaths(
        &mut self,
    ) -> Vec<super::outburst_cast::OutburstNullDeath> {
        self.outburst.take_deaths()
    }
    pub(super) fn acknowledge_outburst_null_death(
        &mut self,
        runtime: &ChannelRuntimeV1,
        completed: &oteryn_gameplay_entities::carrier::native_source_death::SourceCreatureDeathCompletion,
    ) -> Result<(), ()> {
        self.outburst.acknowledge_death(runtime, completed)
    }
    pub(super) fn next_due(&self) -> Option<u64> {
        self.owner
            .as_ref()
            .and_then(|owner| owner.next_due_time())
            .map(|due| due.get())
            .into_iter()
            .chain(self.tenebris.next_due())
            .min()
    }
    #[allow(
        clippy::too_many_arguments,
        reason = "existing physical callback owner inputs"
    )]
    pub(super) async fn drive(
        &mut self,
        active: &crate::content::ActiveGeneration,
        globals: &mut super::outburst_state::OutburstState,
        runtime: &mut ChannelRuntimeV1,
        states: &mut ChannelSpellStates,
        attack: &mut ChannelAttackStates,
        room: &QualifiedNativeEntryRoom,
        native: &NativeGameplayState,
        map: &mut CurrentMonsterMap<'_, '_>,
        proposals: &[ProfileAbilityProposal],
        parties: &ReferenceParties,
        now: u64,
        native_health_names: &crate::gameplay_transport::source_names::NativeHealthNamesData,
    ) -> Result<(), ()> {
        self.drive_bound(
            active,
            globals,
            runtime,
            states,
            attack,
            room,
            native,
            map,
            proposals,
            parties,
            now,
            None,
            native_health_names,
        )
        .await
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "original deadline cutoff and real execution time are independent"
    )]
    pub(super) async fn drive_due_through(
        &mut self,
        active: &crate::content::ActiveGeneration,
        globals: &mut super::outburst_state::OutburstState,
        runtime: &mut ChannelRuntimeV1,
        states: &mut ChannelSpellStates,
        attack: &mut ChannelAttackStates,
        room: &QualifiedNativeEntryRoom,
        native: &NativeGameplayState,
        map: &mut CurrentMonsterMap<'_, '_>,
        parties: &ReferenceParties,
        eligible_through_us: u64,
        now: u64,
        native_health_names: &crate::gameplay_transport::source_names::NativeHealthNamesData,
    ) -> Result<(), ()> {
        self.drive_bound(
            active,
            globals,
            runtime,
            states,
            attack,
            room,
            native,
            map,
            &[],
            parties,
            now,
            Some(eligible_through_us),
            native_health_names,
        )
        .await
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "source stages share the existing current physical transaction and HP owner"
    )]
    async fn drive_bound(
        &mut self,
        active: &crate::content::ActiveGeneration,
        globals: &mut super::outburst_state::OutburstState,
        runtime: &mut ChannelRuntimeV1,
        states: &mut ChannelSpellStates,
        attack: &mut ChannelAttackStates,
        room: &QualifiedNativeEntryRoom,
        native: &NativeGameplayState,
        map: &mut CurrentMonsterMap<'_, '_>,
        proposals: &[ProfileAbilityProposal],
        parties: &ReferenceParties,
        now: u64,
        eligible_through_us: Option<u64>,
        native_health_names: &crate::gameplay_transport::source_names::NativeHealthNamesData,
    ) -> Result<(), ()> {
        if self.owner.is_none() && !proposals.iter().any(callback) {
            return Ok(());
        }
        let stamp = runtime.issue_owner_work().map_err(|_| ())?;
        for proposal in proposals.iter().filter(|proposal| callback(proposal)) {
            if !matches!(
                current_source_creature_role(runtime, proposal.issuer),
                Ok(CurrentSourceCreatureRole::Ordinary
                    | CurrentSourceCreatureRole::IntrinsicMonsterSummon(_))
            ) {
                continue;
            }
            let snapshot = runtime
                .companion_snapshot(proposal.issuer)
                .map_err(|_| ())?;
            let record = native
                .creature_profiles()
                .records
                .iter()
                .find(|record| {
                    record.profile.target.key == snapshot.state.policy.definition_key
                        && record.profile.target.revision
                            == snapshot.state.policy.definition_revision
                })
                .ok_or(())?;
            let source = SourceCallbackCast::qualify_native(
                native,
                &record.profile.target,
                proposal.entry_index,
                proposal.list,
            )
            .map_err(|_| ())?
            .ok_or(())?;
            if self.owner.is_none() {
                self.owner = Some(CallbackCastOwner::new_current(runtime).map_err(|_| ())?);
            }
            self.tenebris.reserve(&source)?;
            let scheduled = self
                .owner
                .as_mut()
                .ok_or(())?
                .schedule_current(
                    runtime,
                    stamp,
                    &source,
                    proposal,
                    SemanticTimeMicros::from_micros(now),
                )
                .map_err(|_| ())?;
            if scheduled {
                self.tenebris.enqueue(&source, proposal, now);
                let _ = super::bog::apply(runtime, states, native, &source, proposal, now);
            }
        }
        self.tenebris
            .drive(active, runtime, room, native, map, eligible_through_us)
            .await?;
        let Some(owner) = self.owner.as_mut() else {
            return Ok(());
        };
        let plans = owner
            .due_stages_current_through(
                runtime,
                stamp,
                SemanticTimeMicros::from_micros(eligible_through_us.unwrap_or(now)),
                SemanticTimeMicros::from_micros(now),
            )
            .map_err(|_| ())?;
        for plan in &plans {
            if plan.source().creature_key == "oteryn:creature.eruption_of_destruction"
                && current_source_creature_role(runtime, plan.proposal().issuer)
                    .ok()
                    .and_then(|role| role.intrinsic_master())
                    .is_some_and(|master| runtime.contains_live_creature(master))
            {
                owner
                    .freeze_eruption_event_current(
                        runtime,
                        stamp,
                        plan.id(),
                        SemanticTimeMicros::from_micros(now),
                    )
                    .map_err(|_| ())?;
            }
        }
        let plans = owner
            .due_stages_current_through(
                runtime,
                stamp,
                SemanticTimeMicros::from_micros(eligible_through_us.unwrap_or(now)),
                SemanticTimeMicros::from_micros(now),
            )
            .map_err(|_| ())?;
        let mut retry = Vec::new();
        for plan in &plans {
            if !self
                .outburst
                .prepare(active, runtime, globals, room, native, map, plan, now)
                .await?
            {
                retry.push(plan.id());
            }
            if !self
                .eruption
                .prepare(
                    runtime,
                    native,
                    map,
                    &mut states.monster_melee.creature_ai,
                    plan,
                    stamp,
                    now,
                )
                .await?
            {
                retry.push(plan.id());
            }
        }
        let catalog =
            crate::creature_area_heal::CreatureHealCatalog::from_current_native(runtime, native)
                .map_err(|_| ())?;
        use crate::map::boot::BundleCollisionIndexExt;
        let facts = match room.movement_cells().index() {
            crate::content::native_cell_lookup::NativeMovementCollisionIndex::Bundle(bundle) => {
                Some(&bundle.map().facts)
            }
            _ => None,
        };
        let mut stage_readers = Vec::with_capacity(plans.len());
        let mut terminal_errors = Vec::new();
        for plan in &plans {
            if retry.contains(&plan.id()) || self.tenebris.blocks(plan) {
                continue;
            }
            if plan.source().creature_key == "oteryn:creature.lady_tenebris" {
                runtime
                    .set_native_tenebris_movement_locked(
                        native,
                        plan.native_stage().ok_or(())?.parent(),
                        plan.proposal().issuer,
                        false,
                    )
                    .map_err(|_| ())?;
            }
            if let Err(error) = plan.anchor_position(runtime) {
                terminal_errors.push((plan.id(), error));
                continue;
            }
            if let Some(reader) = physical_spell::prepare_callback_reader(
                runtime, states, attack, room, native, map, plan, parties, now, stamp,
            )
            .await
            {
                let descriptor = plan.native_stage().ok_or(())?;
                let source =
                    crate::creature_damage_spell::composed::ComposedSource::from_callback_stage(
                        native,
                        descriptor,
                        plan.position_anchor().cloned(),
                    )
                    .map_err(|_| ())?;
                let source = match plan.retained_caster() {
                    Some(caster) => source
                        .with_native_retained_caster(descriptor, caster.clone())
                        .map_err(|_| ())?,
                    None => return Err(()),
                };
                if let Some(mut reader) = physical_spell::composed::CurrentComposed::prepare(
                    runtime, attack, room, &source, reader, now,
                ) {
                    reader
                        .bind_native_health_names(runtime, native_health_names)
                        .ok_or(())?;
                    let mut cues = Vec::new();
                    if facts.is_some() {
                        let effect = match plan.source_impact_binding() {
                            Some("canary.appearance:effect/energyarea") => Some(38),
                            Some("canary.appearance:effect/watersplash") => Some(54),
                            Some("canary.appearance:effect/purpleenergy") => Some(48),
                            Some("canary.appearance:effect/firearea") => Some(7),
                            Some("canary.appearance:effect/mortarea") => Some(18),
                            Some("canary.appearance:effect/smallplants") => Some(46),
                            Some("canary.appearance:effect/fireattack") => Some(37),
                            Some("canary.appearance:effect/holydamage") => Some(40),
                            Some("canary.appearance:effect/groundshaker") => Some(35),
                            // Literal CONST_ME_POISON is undefined in both pins;
                            // Lua getNumber(nil) makes its magic effect NONE.
                            _ => None,
                        };
                        if let Some(effect) = effect {
                            // Source Lua sends the tile effect after its
                            // creature loop, even when no player takes damage.
                            let targets = reader
                                .base
                                .source_visual_positions()
                                .into_iter()
                                .map(|at| (None, at));
                            for (target, at) in targets {
                                if let Some(cue) = super::visual_position(at).and_then(|position| {
                                    crate::map::facts::CreatureVisualCue::effect(
                                        runtime,
                                        stamp,
                                        plan.proposal().issuer,
                                        position,
                                        effect,
                                    )
                                }) {
                                    cues.push((target, cue));
                                }
                            }
                        }
                        if source.creature_key() == "oteryn:creature.plagirath"
                            && let (Ok(from), Ok(Some(to))) = (
                                runtime.read_actor_position(plan.proposal().issuer),
                                plan.anchor_position(runtime),
                            )
                            && let (Some(from), Some(to)) = (
                                super::visual_position(from.position()),
                                super::visual_position(to),
                            )
                            && let Some(cue) = crate::map::facts::CreatureVisualCue::missile(
                                runtime,
                                stamp,
                                plan.proposal().issuer,
                                from,
                                to,
                                15,
                            )
                        {
                            cues.push((None, cue));
                        }
                    }
                    stage_readers.push(StageReader {
                        id: plan.id(),
                        reader,
                        source,
                        proposal: plan.proposal().clone(),
                        eruption_prelude_completed: self.eruption.completed(plan.id()),
                        cues,
                    });
                }
            }
        }
        if !map.validate_transaction(runtime).await {
            return Err(());
        }
        let mut players = std::collections::BTreeMap::new();
        for entry in &stage_readers {
            for &(actor, session) in &entry.reader.base.players {
                if players
                    .insert(actor.placement_identity(), (actor, session))
                    .is_some_and(|previous| previous != (actor, session))
                {
                    return Err(());
                }
            }
        }
        let players = players.into_values().collect::<Vec<_>>();
        #[cfg(test)]
        if plans
            .iter()
            .any(|plan| plan.source().creature_key == "oteryn:creature.charging_outburst")
        {
            eprintln!(
                "SRX3_CHARGING_READY due={} readers={} retry={} terminal={terminal_errors:?}",
                plans.len(),
                stage_readers.len(),
                retry.len()
            );
        }
        let damage = std::mem::take(&mut states.monster_spells);
        let (outcomes, composed) = {
            let mut postlude =
                |id, runtime: &mut ChannelRuntimeV1, stamp, at: SemanticTimeMicros| {
                    self.outburst
                        .commit(id, runtime, globals, native, map, stamp, at.get())
                };
            let mut readers = Readers {
                facts,
                stages: stage_readers,
                terminal: terminal_errors,
                owner: std::mem::take(&mut states.monster_composed),
                catalog,
                postlude: &mut postlude,
            };
            let outcomes = if let Some(cutoff) = eligible_through_us {
                owner.drain_current_prepared_through(
                    runtime,
                    stamp,
                    states,
                    &damage,
                    &mut readers,
                    SemanticTimeMicros::from_micros(cutoff),
                    &Now(now),
                )
            } else {
                owner.drain_current_prepared(
                    runtime,
                    stamp,
                    states,
                    &damage,
                    &mut readers,
                    &Now(now),
                )
            };
            (outcomes, readers.owner)
        };
        states.monster_spells = damage;
        states.monster_composed = composed;
        self.outburst.register_births(
            runtime,
            native,
            &mut states.monster_melee.creature_ai,
            now,
        )?;
        super::spawns::emit_speech(room, owner.take_source_speech());
        let removals = owner.take_source_removals();
        if removals.iter().any(|(_, result)| result.is_err()) {
            return Err(());
        }
        for (issuer, _, _, outcome) in outcomes.map_err(|_| ())? {
            if let Ok(outcome) = outcome {
                physical_spell::record_hits(runtime, attack, Some(issuer), &players, outcome, now);
            }
        }
        Ok(())
    }
}
