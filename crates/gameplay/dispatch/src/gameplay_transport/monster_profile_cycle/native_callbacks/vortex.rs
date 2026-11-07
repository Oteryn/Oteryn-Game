//! Canary/Crystal charge_vortex.lua:16-31 changes the CURRENT ground; the
//! ten-second callback retains a position and survives its caster's retirement.
use super::*;
use crate::foundation::owner_timer::{
    CatchUpPolicy, FamilyPolicy, OwnerClock, OwnerTimerLane, SemanticTimeMicros, TimerFamily,
};
use crate::map::boot::BundleCollisionIndexExt;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Family;
impl TimerFamily for Family {
    fn registered_maximum(self) -> usize {
        128
    }
}
struct Now(u64);
impl OwnerClock for Now {
    fn now(&self) -> SemanticTimeMicros {
        SemanticTimeMicros::from_micros(self.0)
    }
}
#[derive(Debug, Default)]
pub(super) struct NativeVortex {
    context: Option<crate::content::QualifiedCallbackSourceContext>,
    timers: Option<OwnerTimerLane<Family, u64>>,
    pending: Vec<(u64, MovementLocalPosition)>,
    next: u64,
    memos: ActorMemoTable<Ref, Memo>,
}
fn ground(
    room: &QualifiedNativeEntryRoom,
    at: MovementLocalPosition,
) -> Option<(u64, crate::map::overlay::TilePos)> {
    let crate::content::native_cell_lookup::NativeMovementCollisionIndex::Bundle(index) =
        room.movement_cells().index()
    else {
        return None;
    };
    let pos = crate::gameplay_transport::soul_war_scene::wire_position(at)?;
    let tile =
        crate::map::view::compose_tile(&index.map().overlay, &index.map().facts, pos).ok()??;
    let first = tile.entries.first()?;
    if first.facts.terrain_kind != Some(oteryn_world_bundle::bundle::TerrainKind::Ground) {
        return None;
    }
    let crate::map::view::EntrySource::Base {
        ordinal: 0,
        placement_key,
        ..
    } = first.source
    else {
        return None;
    };
    Some((placement_key, pos))
}
fn transform(room: &QualifiedNativeEntryRoom, key: u64, id: u16) -> bool {
    let crate::content::native_cell_lookup::NativeMovementCollisionIndex::Bundle(index) =
        room.movement_cells().index()
    else {
        return false;
    };
    index.map().facts.transform_movement_item(key, id)
}
impl NativeVortex {
    pub(super) fn next_due(&self) -> Option<u64> {
        self.timers.as_ref()?.earliest_due().map(|at| at.get())
    }
    #[allow(
        clippy::too_many_arguments,
        reason = "source callback shares the current qualified native/map owner"
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
    #[allow(
        clippy::too_many_arguments,
        reason = "qualified source, current runtime, map, proposals and original cutoff require independent authority checks"
    )]
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
    #[allow(
        clippy::too_many_arguments,
        reason = "qualified source, current runtime, map, proposals and original cutoff require independent authority checks"
    )]
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
        const KEY: &str = "oteryn:ability.spell.charge_vortex";
        if self.pending.is_empty() && !proposals.iter().any(|p| p.ability.key == KEY) {
            return Ok(());
        }
        if !self
            .context
            .as_ref()
            .is_some_and(|proof| proof.matches(room, runtime.content_pin()))
        {
            self.context=active.qualify_callback_source_context(room,runtime.content_pin(),crate::content::bosses::activity_template::PinnedActivitySourceRoom::CallbackAnomaly);
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
                            max_pending: 128,
                            catch_up: CatchUpPolicy::DeadlineState,
                        },
                    )],
                )
                .map_err(|_| ())?,
            );
        }
        let timers = self.timers.as_ref().ok_or(())?;
        let eligible = self
            .pending
            .iter()
            .filter_map(|entry| {
                timers
                    .occurrence_due(Family, entry.0)
                    .filter(|due| due.get() <= eligible_through_us)
                    .map(|_| entry.0)
            })
            .collect::<Vec<_>>();
        let mut preview = timers.clone();
        for fired in preview.drain_due_matching(
            &Now(now),
            runtime.owner_fence().map_err(|_| ())?,
            |_| true,
            |_, occurrence| eligible.contains(&occurrence),
        ) {
            let at = self
                .pending
                .iter()
                .find(|entry| entry.0 == fired.occurrence)
                .ok_or(())?
                .1;
            map.read(runtime, at).await;
            if !map.validate_transaction(runtime).await {
                return Err(());
            }
            if let Some((key, _)) = ground(room, at)
                && !transform(room, key, 23049)
            {
                return Err(());
            }
            self.timers
                .as_mut()
                .ok_or(())?
                .cancel(Family, fired.occurrence);
            self.pending.retain(|entry| entry.0 != fired.occurrence);
        }
        for p in proposals.iter().filter(|p| p.ability.key == KEY) {
            let snapshot = runtime.companion_snapshot(p.issuer).map_err(|_| ())?;
            let Some(record) = native.creature_profiles().records.iter().find(|r| {
                r.profile.target.key == snapshot.state.policy.definition_key
                    && r.profile.target.revision == snapshot.state.policy.definition_revision
            }) else {
                continue;
            };
            let Some(source) =
                SevenSource::qualify_native(native, &record.profile.target, p.entry_index, p.list)
                    .map_err(|_| ())?
            else {
                continue;
            };
            if source.kind()!=SevenKind::Vortex||oteryn_gameplay_encounters::bosses::source_callbacks::source_callback_intent::validate(runtime,p).is_err(){continue;}
            let mut normalized = p.clone();
            normalized.target = p.issuer;
            let atom = format!(
                "actor:{}",
                oteryn_gameplay_encounters::bosses::seven_encounters::source_encounter_seven::hex(
                    p.issuer.placement_identity()
                )
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
                .is_some_and(|m| m.sequence >= work.sequence)
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
            const POS: [[i32; 3]; 11] = [
                [32264, 31253, 14],
                [32269, 31258, 14],
                [32275, 31255, 14],
                [32280, 31253, 14],
                [32271, 31248, 14],
                [32264, 31245, 14],
                [32270, 31240, 14],
                [32269, 31253, 14],
                [32275, 31245, 14],
                [32276, 31250, 14],
                [32266, 31249, 14],
            ];
            let hash = Sha256::digest(p.occurrence.id().as_str().as_bytes());
            let mut occurrence = [0; 16];
            occurrence.copy_from_slice(&hash[..16]);
            let draw = deterministic_decision_u64(
                &GameplayDecisionRoot::from_bytes(native.source_digest()),
                DecisionOccurrenceId::from_bytes(occurrence),
                "charge_vortex_source_position",
                0,
            )
            .map_err(|_| ())?;
            let index =
                oteryn_gameplay_domain::domain::uniform_draw::uniform_draw(draw, 0, 10) as usize;
            let (x, y, floor) = context
                .source_frame()
                .physical_position(POS[index])
                .ok_or(())?;
            let at = MovementLocalPosition { x, y, floor };
            map.read(runtime, at).await;
            if !map.validate_transaction(runtime).await {
                return Err(());
            }
            if let Some((ground_key, _)) = ground(room, at) {
                self.pending.try_reserve(1).map_err(|_| ())?;
                let key = self.next.checked_add(1).ok_or(())?;
                self.timers
                    .as_mut()
                    .ok_or(())?
                    .schedule(
                        runtime.owner_fence().map_err(|_| ())?,
                        stamp,
                        Family,
                        key,
                        None,
                        SemanticTimeMicros::from_micros(now.checked_add(10_000_000).ok_or(())?),
                    )
                    .map_err(|_| ())?;
                if !transform(room, ground_key, 22894) {
                    self.timers.as_mut().ok_or(())?.cancel(Family, key);
                    return Err(());
                }
                self.next = key;
                self.pending.push((key, at));
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
