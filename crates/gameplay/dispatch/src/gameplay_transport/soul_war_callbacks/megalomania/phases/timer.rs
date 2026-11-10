use super::*;
use crate::gameplay_transport::ComposedFreshAdmission;

#[derive(Clone)]
pub(super) enum PreparedTimer {
    Phase(PreparedPhaseTimer),
    Splinter(super::super::splinter::PreparedSplinter),
}
#[derive(Clone)]
pub(super) struct PreparedPhaseTimer {
    pub(super) boss: ExactActorRef,
    pub(super) revision: u64,
    pub(super) due: u64,
}
impl ComposedFreshAdmission<'_, '_, '_> {
    /// One original source deadline, ahead of a newer copied death task. Map
    /// and all-role names are detached reads; only current seals publish it.
    pub(in crate::gameplay_transport) async fn drain_detached_megalomania_timer(
        &self,
        permit: &crate::durability::spell_owner_commit::SpellLanePermit,
        now: u64,
    ) -> Result<bool, ()> {
        let Some(room) = self.effective_rooms().gate_monster_ai().filter(|room|
            room.pinned_activity_source_room() == Some(crate::content::bosses::activity_template::PinnedActivitySourceRoom::GoshnarsMegalomania))
        else { return Ok(false); };
        let native = self
            .active_generation
            .and_then(|active| active.native_gameplay())
            .ok_or(())?;
        let (notice, timer, map_plan, names_plan, scope, generation) = {
            let runtime = self.runtime.lock().await;
            let states = self.states.spell_states.try_lock().map_err(|_| ())?;
            let Some(owner) = states.soul_war_callbacks.megalomania.as_ref() else {
                return Ok(false);
            };
            let phase = owner
                .next_due_ms()
                .map(|due| due.checked_mul(1000).map(|due_us| (due, due_us)).ok_or(()))
                .transpose()?;
            let splinter = super::super::splinter::next(&runtime, native)?;
            let selected = match (phase, splinter) {
                (Some((due, due_us)), splinter)
                    if splinter.is_none_or(|plan| due_us <= plan.due_us) =>
                {
                    Some((
                        due_us,
                        PreparedTimer::Phase(PreparedPhaseTimer {
                            boss: owner.boss,
                            revision: owner.phases.revision,
                            due,
                        }),
                    ))
                }
                (_, Some(plan)) => Some((plan.due_us, PreparedTimer::Splinter(plan))),
                _ => None,
            };
            let Some((due_us, timer)) = selected else {
                return Ok(false);
            };
            let notice = runtime.next_source_creature_death().map_err(|_| ())?;
            let cutoff = notice
                .as_ref()
                .map(|notice| notice.enqueued_at_us().ok_or(()))
                .transpose()?
                .unwrap_or(now);
            if due_us > cutoff
                || due_us > now
                || (matches!(&timer, PreparedTimer::Phase(_))
                    && (owner.phases.aspect.is_some()
                        || states.soul_war_callbacks.sanity.pending.is_some()))
                || ((notice.is_some() || states.native_profile_pass.has_pending())
                    && states
                        .native_callbacks
                        .next_source_due_time()?
                        .into_iter()
                        .chain(
                            states
                                .soul_war_callbacks
                                .next_native_due_without_megalomania(),
                        )
                        .chain(states.spite_global.next_due_us())
                        .chain(states.primal_callbacks.next_source_due_time()?)
                        .chain(
                            states
                                .rotten_blood_callbacks
                                .next_source_due_time(&runtime)?,
                        )
                        .any(|other| other <= due_us && other <= now))
                || states
                    .native_profile_pass
                    .next_due(&runtime)?
                    .is_some_and(|other| other < due_us && other <= now)
            {
                return Ok(false);
            }
            let mut cells = vec![HOME];
            for captured in owner.phases.captures.values() {
                if let Capture::Aspect(plan) = captured {
                    cells.extend_from_slice(plan.candidates());
                }
            }
            let objects = self.door.try_lock().map_err(|_| ())?;
            let map_plan = crate::movement::monster_map::PreparedMonsterMapRead::prepare(
                &runtime, room, &objects, &cells,
            )
            .ok_or(())?;
            let names_plan = self.prepare_source_names(&runtime, &[NAME])?;
            (
                notice,
                timer,
                map_plan,
                names_plan,
                runtime.binding().runtime_scope(),
                runtime.binding().scope_generation().get(),
            )
        };
        let names = self.read_source_names(permit, &names_plan).await?;
        let mut context = (
            self,
            permit,
            map_plan,
            Some(names),
            notice,
            timer,
            room,
            native,
            scope,
            generation,
            now,
        );
        self.root.try_issue_semantic_pass().map_err(|_| ())?
            .run_with_context(&mut context, |holder, deadline, context| {
                Box::pin(async move {
                    let (owner, permit, plan, names, notice, timer, room, native, scope, generation, now) = context;
                    let refuse = || crate::durability::DurabilityError::Unavailable;
                    let mut tx = crate::durability::spell_item_transaction::begin_spell_owner_transaction(holder, deadline).await?;
                    let authority = crate::durability::spell_item_transaction::assert_spell_item_scope_in_transaction(
                        &mut tx, permit, owner.root, owner.character, owner.holder, *scope, *generation,
                    ).await.map_err(|_| refuse())?;
                    let read = plan.read(&mut tx, &authority).await.ok_or_else(refuse)?;
                    {
                        let mut runtime = owner.runtime.lock().await;
                        let mut states = owner.states.spell_states.try_lock().map_err(|_| refuse())?;
                        let objects = owner.door.try_lock().map_err(|_| refuse())?;
                        if runtime.next_source_creature_death().map_err(|_| refuse())?.as_ref() != notice.as_ref() {
                            return Err(refuse());
                        }
                        let map = read.into_current_view(&runtime, room, &objects, native).ok_or_else(refuse)?;
                        let names = owner.seal_source_names(&runtime, names.take().ok_or_else(refuse)?).map_err(|_| refuse())?;
                        publish_timer(&mut runtime, &mut states, room, native, &map, timer, &names, *now).map_err(|_| refuse())?;
                    }
                    tx.rollback().await.map_err(|_| refuse())?;
                    Ok(true)
                })
            }).await.map_err(|_| ())
    }
}
