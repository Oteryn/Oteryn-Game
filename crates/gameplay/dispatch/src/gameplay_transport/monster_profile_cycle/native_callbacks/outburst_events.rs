//! Pinned creaturescripts_outburst_charge.lua:1-36. Shared Lua DATA stays in
//! OutburstState; source birth proofs and the physical carrier survive removal.
use super::*;
use crate::content::native_monster_callbacks::{
    NativeOutburstEvent, QualifiedNativeOutburstEvents,
};
use crate::foundation::ThinkOccurrence;
use oteryn_gameplay_entities::PreparedNativeCreatureEventBirth;
#[derive(Debug)]
struct Birth {
    captured_at_us: u64,
    prepared: PreparedNativeCreatureEventBirth,
    child: Ref,
    at: MovementLocalPosition,
    done: bool,
}
#[derive(Debug, Default)]
pub(super) struct NativeOutburstEvents {
    context: Option<crate::content::QualifiedCallbackSourceContext>,
    source: Option<QualifiedNativeOutburstEvents>,
    seen: Vec<ThinkOccurrence>,
    births: Vec<Birth>,
    phase: Option<u8>,
}
impl NativeOutburstEvents {
    pub(super) fn has_work(&self) -> bool {
        !self.births.is_empty()
    }
    pub(super) fn next_due(&self) -> Option<u64> {
        self.births
            .iter()
            .filter(|birth| !birth.done)
            .map(|birth| birth.captured_at_us)
            .min()
    }
    #[allow(
        clippy::too_many_arguments,
        reason = "real native think consumes current owner, source frame and physical birth clocks"
    )]
    pub(super) async fn drive(
        &mut self,
        globals: &mut outburst_state::OutburstState,
        active: &crate::content::ActiveGeneration,
        runtime: &mut ChannelRuntimeV1,
        room: &QualifiedNativeEntryRoom,
        native: &NativeGameplayState,
        map: &mut CurrentMonsterMap<'_, '_>,
        ai: &mut oteryn_gameplay_monster_ai::ai_think::CreatureAiTable,
        occurrences: &[ThinkOccurrence],
        now: u64,
    ) -> Result<(), ()> {
        self.drive_inner(
            globals,
            active,
            runtime,
            room,
            native,
            map,
            ai,
            occurrences,
            now,
            now,
        )
        .await
    }
    #[allow(
        clippy::too_many_arguments,
        reason = "shared source globals, qualified content, physical map, AI owner and source cutoff use the real execution clock"
    )]
    pub(super) async fn drive_due_through(
        &mut self,
        globals: &mut outburst_state::OutburstState,
        active: &crate::content::ActiveGeneration,
        runtime: &mut ChannelRuntimeV1,
        room: &QualifiedNativeEntryRoom,
        native: &NativeGameplayState,
        map: &mut CurrentMonsterMap<'_, '_>,
        ai: &mut oteryn_gameplay_monster_ai::ai_think::CreatureAiTable,
        eligible_through_us: u64,
        now: u64,
    ) -> Result<(), ()> {
        self.drive_inner(
            globals,
            active,
            runtime,
            room,
            native,
            map,
            ai,
            &[],
            eligible_through_us,
            now,
        )
        .await
    }
    #[allow(
        clippy::too_many_arguments,
        reason = "shared source globals, qualified content, physical map, AI owner and source cutoff use the real execution clock"
    )]
    async fn drive_inner(
        &mut self,
        globals: &mut outburst_state::OutburstState,
        active: &crate::content::ActiveGeneration,
        runtime: &mut ChannelRuntimeV1,
        room: &QualifiedNativeEntryRoom,
        native: &NativeGameplayState,
        map: &mut CurrentMonsterMap<'_, '_>,
        ai: &mut oteryn_gameplay_monster_ai::ai_think::CreatureAiTable,
        occurrences: &[ThinkOccurrence],
        eligible_through_us: u64,
        now: u64,
    ) -> Result<(), ()> {
        const KEY: &[u8] = b"oteryn:creature.outburst";
        if !self.has_work()
            && !occurrences
                .iter()
                .any(|o| runtime.matches_live_creature_identity(o.actor, KEY))
        {
            return Ok(());
        }
        if !self
            .context
            .as_ref()
            .is_some_and(|proof| proof.matches(room, runtime.content_pin()))
        {
            self.context=active.qualify_callback_source_context(room,runtime.content_pin(),crate::content::bosses::activity_template::PinnedActivitySourceRoom::CallbackOutburst);
        }
        let Some(context) = self.context.clone() else {
            return Ok(());
        };
        if self.source.is_none() {
            self.source = Some(native.outburst_events_source().map_err(|_| ())?);
        }
        let source = self.source.as_ref().ok_or(())?;
        for occurrence in occurrences {
            if !runtime.matches_live_creature_identity(occurrence.actor, KEY)
                || self.seen.iter().any(|seen| {
                    seen.actor == occurrence.actor && seen.sequence >= occurrence.sequence
                })
            {
                continue;
            }
            let snapshot = runtime
                .companion_snapshot(occurrence.actor)
                .map_err(|_| ())?;
            if snapshot.state.master.is_some()
                || runtime.native_summon_master(occurrence.actor).is_some()
            {
                continue;
            }
            let sample = runtime
                .capture_native_null_health_sample(
                    native,
                    source.callback_source(),
                    occurrence.actor,
                )
                .map_err(|_| ())?;
            globals.observe_health_sample(sample).ok_or(())?;
            let stage = globals.source_stage();
            let threshold = match stage {
                0 => 80,
                1 => 60,
                2 => 40,
                3 => 20,
                _ => 0,
            };
            if threshold > 0
                && i128::from(snapshot.health) * 100
                    <= i128::from(snapshot.state.policy.maximum_health) * threshold
            {
                if !self.births.is_empty() {
                    return Err(());
                }
                let mut births = Vec::new();
                births.try_reserve(5).map_err(|_| ())?;
                let stamp = runtime.issue_owner_work().map_err(|_| ())?;
                const POS: [[i32; 3]; 5] = [
                    [32229, 31282, 14],
                    [32230, 31287, 14],
                    [32237, 31287, 14],
                    [32238, 31282, 14],
                    [32234, 31284, 14],
                ];
                // Prepare all five closed proofs while the original actor still exists.
                for (index, point) in POS.into_iter().enumerate() {
                    let child = if index == 4 {
                        source.charging()
                    } else {
                        source.spark()
                    };
                    let token = source
                        .spawn_source(NativeOutburstEvent::Think, child)
                        .ok_or(())?;
                    let prepared = runtime
                        .prepare_native_creature_event_birth(
                            native,
                            stamp,
                            occurrence.actor,
                            &token,
                        )
                        .map_err(|_| ())?;
                    let (x, y, floor) = context.source_frame().physical_position(point).ok_or(())?;
                    births.push(Birth {
                        captured_at_us: now,
                        prepared,
                        child: child.clone(),
                        at: MovementLocalPosition { x, y, floor },
                        done: false,
                    });
                }
                let phase = stage.checked_add(1).ok_or(())?;
                runtime
                    .remove_current_native_encounter_subject(
                        stamp,
                        occurrence.actor,
                        &source.outburst().key,
                        native.source_digest(),
                    )
                    .map_err(|_| ())?;
                self.births = births;
                self.phase = Some(phase);
            }
            self.seen
                .retain(|seen| runtime.contains_live_creature(seen.actor));
            self.seen.retain(|seen| seen.actor != occurrence.actor);
            if self.seen.len() >= CREATURE_DEFENCE_MEMOS_PER_CHANNEL_MAX {
                return Err(());
            }
            self.seen.push(*occurrence);
        }
        for birth in &mut self.births {
            if birth.done || birth.captured_at_us > eligible_through_us {
                continue;
            }
            map.read(runtime, birth.at).await;
            let exists = map
                .source_tile_exists_in_callback(runtime, birth.at, &context)
                .ok_or(())?;
            if !map.validate_transaction(runtime).await {
                return Err(());
            }
            // Each captured requested tile exists in the immutable pinned map. Source
            // force=true ignores occupants and queryAdd properties at that actual tile.
            if !exists {
                birth.done = true;
                continue;
            }
            let actor = runtime
                .admit_prepared_native_creature_event_birth(
                    native,
                    &mut birth.prepared,
                    birth.at,
                    birth.at,
                    |_, _| Some(true),
                )
                .map_err(|_| ())?;
            ai.admit_native_creature(runtime, native, actor, &birth.child, now)
                .map_err(|_| ())?;
            birth.done = true;
        }
        if !self.births.is_empty() && self.births.iter().all(|birth| birth.done) {
            globals
                .advance_phase(self.phase.take().ok_or(())?)
                .ok_or(())?;
            self.births.clear();
        }
        Ok(())
    }
}
