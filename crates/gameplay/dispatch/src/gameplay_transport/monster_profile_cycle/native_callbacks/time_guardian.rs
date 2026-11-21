//! The two pinned scripts swap distinct live actors. A retained addEvent reads
//! the CURRENT parking top and arena spectators, even after its original caster died.
use super::*;
use crate::content::native_monster_callbacks::{
    NativeCallbackList, QualifiedNativeCallback, qualify_native_callback,
};
use crate::content::{PublishedWorldSourceFrame, QualifiedCallbackSourceContext};
use crate::foundation::owner_timer::{
    CatchUpPolicy, FamilyPolicy, OwnerClock, OwnerTimerLane, SemanticTimeMicros, TimerFamily,
};
mod returns;
const BASE: &[u8] = b"oteryn:creature.the_time_guardian";
const FIRE: &[u8] = b"oteryn:creature.the_blazing_time_guardian";
const ICE: &[u8] = b"oteryn:creature.the_freezing_time_guardian";
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Family;
impl TimerFamily for Family {
    fn registered_maximum(self) -> usize {
        64
    }
}
struct Now(u64);
impl OwnerClock for Now {
    fn now(&self) -> SemanticTimeMicros {
        SemanticTimeMicros::from_micros(self.0)
    }
}
#[derive(Debug, Clone)]
struct Pending {
    key: u64,
    // Descriptive only; firing never requires this original issuer alive.
    #[cfg(test)]
    actor: crate::foundation::ExactActorRef,
    parking: MovementLocalPosition,
    old: MovementLocalPosition,
    alternate: bool,
    source: QualifiedNativeCallback,
    occurrence: String,
}
#[derive(Debug, Default)]
pub(super) struct NativeTimeGuardian {
    timers: Option<OwnerTimerLane<Family, u64>>,
    pending: Vec<Pending>,
    next: u64,
    context: Option<QualifiedCallbackSourceContext>,
    memos: ActorMemoTable<Ref, Memo>,
}
fn at(frame: PublishedWorldSourceFrame, point: [i32; 3]) -> Result<MovementLocalPosition, ()> {
    let (x, y, floor) = frame.physical_position(point).ok_or(())?;
    Ok(MovementLocalPosition { x, y, floor })
}
fn is_pair(p: &ProfileAbilityProposal) -> bool {
    matches!(
        p.ability.key.as_str(),
        "oteryn:ability.spell.time_guardian" | "oteryn:ability.spell.time_guardiann"
    )
}
fn binding(occurrence: &str, target: crate::foundation::ExactActorRef, health: i64) -> Vec<u8> {
    let mut out = occurrence.as_bytes().to_vec();
    out.push(0);
    out.extend_from_slice(&target.placement_identity());
    out.extend_from_slice(&target.actor_local_generation().to_be_bytes());
    out.extend_from_slice(&health.to_be_bytes());
    out
}
async fn exists(
    map: &mut CurrentMonsterMap<'_, '_>,
    runtime: &ChannelRuntimeV1,
    point: MovementLocalPosition,
    context: &QualifiedCallbackSourceContext,
) -> Result<bool, ()> {
    map.read(runtime, point).await;
    map.source_tile_exists_in_callback(runtime, point, context)
        .ok_or(())
}
impl NativeTimeGuardian {
    #[cfg(test)]
    pub(super) fn cast_position_for_test(
        &self,
        actor: crate::foundation::ExactActorRef,
    ) -> Option<MovementLocalPosition> {
        self.pending
            .iter()
            .rev()
            .find(|pending| pending.actor == actor)
            .map(|pending| pending.old)
    }
    pub(super) fn next_due(&self) -> Option<u64> {
        self.timers.as_ref()?.earliest_due().map(|at| at.get())
    }
    #[allow(
        clippy::too_many_arguments,
        reason = "one source callback consumes the current native/map owner inputs"
    )]
    pub(super) async fn drive(
        &mut self,
        active: &crate::content::ActiveGeneration,
        runtime: &mut ChannelRuntimeV1,
        room: &QualifiedNativeEntryRoom,
        native: &NativeGameplayState,
        map: &mut CurrentMonsterMap<'_, '_>,
        proposals: &[ProfileAbilityProposal],
        now: u64,
    ) -> Result<(), ()> {
        self.drive_inner(active, runtime, room, native, map, proposals, now, now)
            .await
    }
    #[allow(clippy::too_many_arguments)]
    pub(super) async fn drive_due_through(
        &mut self,
        active: &crate::content::ActiveGeneration,
        runtime: &mut ChannelRuntimeV1,
        room: &QualifiedNativeEntryRoom,
        native: &NativeGameplayState,
        map: &mut CurrentMonsterMap<'_, '_>,
        eligible_through_us: u64,
        now: u64,
    ) -> Result<(), ()> {
        self.drive_inner(
            active,
            runtime,
            room,
            native,
            map,
            &[],
            eligible_through_us,
            now,
        )
        .await
    }
    #[allow(clippy::too_many_arguments)]
    async fn drive_inner(
        &mut self,
        active: &crate::content::ActiveGeneration,
        runtime: &mut ChannelRuntimeV1,
        room: &QualifiedNativeEntryRoom,
        native: &NativeGameplayState,
        map: &mut CurrentMonsterMap<'_, '_>,
        proposals: &[ProfileAbilityProposal],
        eligible_through_us: u64,
        now: u64,
    ) -> Result<(), ()> {
        if self.pending.is_empty() && !proposals.iter().any(is_pair) {
            return Ok(());
        }
        if !self
            .context
            .as_ref()
            .is_some_and(|proof| proof.matches(room, runtime.content_pin()))
        {
            self.context=active.qualify_callback_source_context(room,runtime.content_pin(),
                crate::content::bosses::activity_template::PinnedActivitySourceRoom::CallbackTimeGuardian);
        }
        let Some(context) = self.context.clone() else {
            return Ok(());
        };
        if self.timers.is_none() {
            self.timers = Some(
                OwnerTimerLane::for_generation(
                    runtime.binding().runtime_scope(),
                    runtime.binding().scope_generation(),
                    [(
                        Family,
                        FamilyPolicy {
                            max_pending: 64,
                            catch_up: CatchUpPolicy::DeadlineState,
                        },
                    )],
                )
                .map_err(|_| ())?,
            );
        }
        self.drain(runtime, native, map, &context, eligible_through_us, now)
            .await?;
        for p in proposals.iter().filter(|p| is_pair(p)) {
            if !runtime.matches_live_creature_identity(p.issuer, BASE) {
                continue;
            }
            let snapshot = runtime.companion_snapshot(p.issuer).map_err(|_| ())?;
            let Some(record) = native.creature_profiles().records.iter().find(|record| {
                record.profile.target.key == snapshot.state.policy.definition_key
                    && record.profile.target.revision == snapshot.state.policy.definition_revision
            }) else {
                continue;
            };
            let Some(source) =
                SevenSource::qualify_native(native, &record.profile.target, p.entry_index, p.list)
                    .map_err(|_| ())?
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
                    .map(|byte| format!("{byte:02x}"))
                    .collect::<String>()
            );
            normalized.intent =
                crate::ability::AiAbilityAdapter::normalize(&atom, &[&atom]).map_err(|_| ())?;
            let stamp = runtime.issue_owner_work().map_err(|_| ())?;
            let work = source
                .prepare_map_work(
                    runtime,
                    runtime.owner_fence().map_err(|_| ())?,
                    stamp,
                    &normalized,
                )
                .map_err(|_| ())?;
            if self
                .memos
                .get(p.issuer, &p.ability)
                .is_some_and(|memo| memo.sequence >= work.sequence)
            {
                continue;
            }
            self.memos.evict_stale_at(p.issuer);
            self.memos
                .prune(|actor| runtime.contains_live_creature(actor));
            if self.memos.get(p.issuer, &p.ability).is_none()
                && self.memos.len() >= CREATURE_DEFENCE_MEMOS_PER_CHANNEL_MAX
            {
                return Err(());
            }
            let list = match p.list {
                crate::foundation::profile_schedule::ScheduleList::Attack => {
                    NativeCallbackList::Attack
                }
                crate::foundation::profile_schedule::ScheduleList::Defence => {
                    NativeCallbackList::Defence
                }
            };
            let proof =
                qualify_native_callback(native, &record.profile.target, p.entry_index, list)
                    .map_err(|_| ())?
                    .ok_or(())?;
            let alternate = work.kind == SevenKind::TimeGuardiann;
            let source_at = context
                .source_frame()
                .source_position((
                    snapshot.position.x,
                    snapshot.position.y,
                    snapshot.position.floor,
                ))
                .ok_or(())?;
            // Original script draws before its floor guard; alternate guards first.
            if alternate && source_at[2] != 14 {
                self.memos.insert(
                    p.issuer,
                    p.ability.clone(),
                    Memo {
                        proposal: p.clone(),
                        sequence: work.sequence,
                    },
                );
                continue;
            }
            let hash = Sha256::digest(p.occurrence.id().as_str().as_bytes());
            let mut id = [0; 16];
            id.copy_from_slice(&hash[..16]);
            let draw = deterministic_decision_u64(
                &GameplayDecisionRoot::from_bytes(native.source_digest()),
                DecisionOccurrenceId::from_bytes(id),
                "time_guardian_source_phase",
                0,
            )
            .map_err(|_| ())?;
            let index = oteryn_gameplay_domain::domain::uniform_draw::uniform_draw(draw, 1, 2);
            if source_at[2] == 14 {
                let parking = at(
                    context.source_frame(),
                    if alternate {
                        if index == 1 {
                            [32810, 32664, 14]
                        } else {
                            [32815, 32664, 14]
                        }
                    } else if index == 1 {
                        [32980, 31664, 13]
                    } else {
                        [32975, 31664, 13]
                    },
                )?;
                if exists(map, runtime, parking, &context).await?
                    && exists(map, runtime, snapshot.position, &context).await?
                    && let Some(form) =
                        runtime.current_top_actor_on_cell(parking).map_err(|_| ())?
                {
                    if !map.validate_transaction(runtime).await {
                        return Err(());
                    }
                    self.pending.try_reserve(1).map_err(|_| ())?;
                    let key = self.next.checked_add(1).ok_or(())?;
                    let deadline =
                        SemanticTimeMicros::from_micros(now.checked_add(30_000_000).ok_or(())?);
                    self.timers
                        .as_mut()
                        .ok_or(())?
                        .schedule(
                            runtime.owner_fence().map_err(|_| ())?,
                            stamp,
                            Family,
                            key,
                            None,
                            deadline,
                        )
                        .map_err(|_| ())?;
                    let swapped = (|| -> Result<(), ()> {
                        let sample = runtime
                            .capture_native_null_health_sample(native, &proof, p.issuer)
                            .map_err(|_| ())?;
                        map.view()
                            .relocate_source_creature(runtime, p.issuer, parking)
                            .map_err(|_| ())?;
                        let target = runtime.companion_snapshot(form).map_err(|_| ())?;
                        let occurrence = format!("{}:time-sync", p.occurrence.id().as_str());
                        runtime
                            .commit_native_null_health_sync_at(
                                native,
                                &proof,
                                &sample,
                                &target,
                                occurrence.as_bytes(),
                                &binding(&occurrence, form, sample.health()),
                                now,
                            )
                            .map_err(|_| ())?;
                        map.view()
                            .relocate_source_creature(runtime, form, snapshot.position)
                            .map_err(|_| ())?;
                        Ok(())
                    })();
                    if swapped.is_err() {
                        self.timers.as_mut().ok_or(())?.cancel(Family, key);
                        return Err(());
                    }
                    self.next = key;
                    self.pending.push(Pending {
                        key,
                        #[cfg(test)]
                        actor: p.issuer,
                        parking,
                        old: snapshot.position,
                        alternate,
                        source: proof,
                        occurrence: p.occurrence.id().as_str().to_owned(),
                    });
                }
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
