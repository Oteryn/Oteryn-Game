//! Immediate source prelude, retained across a cancelled current-map read.
//! Its delayed Combat remains on the original CallbackCastOwner timer.
use super::*;
use oteryn_gameplay_encounters::bosses::source_callbacks::source_callback_cast::{
    CallbackStagePlan, SourceCallbackCast,
};
struct Target {
    actor: crate::foundation::ExactActorRef,
    session: Option<crate::foundation::GameSessionId>,
    done: bool,
}
struct Pending {
    source: SourceCallbackCast,
    proposal: ProfileAbilityProposal,
    targets: Option<Vec<Target>>,
    captured_at_us: u64,
}
#[derive(Default)]
pub(super) struct TenebrisPreludes(Vec<Pending>);
impl TenebrisPreludes {
    pub(super) fn next_due(&self) -> Option<u64> {
        self.0.iter().map(|pending| pending.captured_at_us).min()
    }
    pub(super) fn reserve(&mut self, source: &SourceCallbackCast) -> Result<(), ()> {
        if source.parent().key == "oteryn:ability.spell.tenebris_ultimate" {
            if self.0.len() >= 64 {
                return Err(());
            }
            self.0.try_reserve(1).map_err(|_| ())?;
        }
        Ok(())
    }
    // The caller reserves before committing the delayed stage. No await or
    // fallible operation separates that stage commit from this retained prelude.
    pub(super) fn enqueue(
        &mut self,
        source: &SourceCallbackCast,
        proposal: &ProfileAbilityProposal,
        captured_at_us: u64,
    ) {
        if source.parent().key != "oteryn:ability.spell.tenebris_ultimate" {
            return;
        }
        self.0.push(Pending {
            source: source.clone(),
            proposal: proposal.clone(),
            targets: None,
            captured_at_us,
        });
    }
    pub(super) fn blocks(&self, plan: &CallbackStagePlan) -> bool {
        self.0.iter().any(|pending| {
            pending.proposal.issuer == plan.proposal().issuer
                && plan
                    .proposal()
                    .occurrence
                    .id()
                    .as_str()
                    .starts_with(&format!(
                        "callback:{}:stage:",
                        pending.proposal.occurrence.id().as_str()
                    ))
        })
    }
    #[allow(
        clippy::too_many_arguments,
        reason = "one retained source callback shares current map/runtime/native owners"
    )]
    pub(super) async fn drive(
        &mut self,
        active: &crate::content::ActiveGeneration,
        runtime: &mut ChannelRuntimeV1,
        room: &QualifiedNativeEntryRoom,
        native: &NativeGameplayState,
        map: &mut CurrentMonsterMap<'_, '_>,
        eligible_through_us: Option<u64>,
    ) -> Result<(), ()> {
        self.0
            .retain(|pending| runtime.contains_live_creature(pending.proposal.issuer));
        let mut index = 0;
        while index < self.0.len() {
            let pending = &mut self.0[index];
            if eligible_through_us.is_some_and(|cutoff| pending.captured_at_us > cutoff) {
                index += 1;
                continue;
            }
            if !pending
                .source
                .validates_current_native_proposal(runtime, &pending.proposal)
            {
                return Err(());
            }
            let source = pending.source.native_parent().ok_or(())?;
            let target = if let Some(context) = active.qualify_callback_source_context(room, runtime.content_pin(),
                crate::content::bosses::activity_template::PinnedActivitySourceRoom::CallbackLadyTenebris) {
                context.source_frame().physical_position([32912, 31599, 14])
            } else if let Some(context) = active.qualify_published_world_context(room, runtime.content_pin()) {
                context.source_frame().physical_position([32912, 31599, 14])
            } else { return Err(()); }.ok_or(())?;
            let target = MovementLocalPosition {
                x: target.0,
                y: target.1,
                floor: target.2,
            };
            if pending.targets.is_none() {
                let mut targets = Vec::new();
                for (actor, position, session) in
                    runtime.positioned_actor_census().map_err(|_| ())?
                {
                    let at = position.position();
                    if position.context() != runtime.pinned_movement_context()
                        || at.floor != target.floor
                        || (i64::from(at.x) - i64::from(target.x)).abs() > 12
                        || (i64::from(at.y) - i64::from(target.y)).abs() > 12
                    {
                        continue;
                    }
                    if session.is_none()
                        && !runtime
                            .matches_live_creature_identity(actor, b"oteryn:creature.lady_tenebris")
                    {
                        continue;
                    }
                    if targets.len() >= 64 {
                        return Err(());
                    }
                    targets.push(Target {
                        actor,
                        session,
                        done: false,
                    });
                }
                pending.targets = Some(targets);
            }
            map.read(runtime, target).await;
            if !map.view().known(target) {
                index += 1;
                continue;
            }
            if !map.validate_transaction(runtime).await {
                return Err(());
            }
            for entry in pending.targets.as_mut().ok_or(())? {
                if entry.done {
                    continue;
                }
                let Ok(before) = runtime.read_actor_position(entry.actor) else {
                    entry.done = true;
                    continue;
                };
                if before.context() != runtime.pinned_movement_context() {
                    entry.done = true;
                    continue;
                }
                if let Some(session) = entry.session {
                    runtime
                        .player_control_facts(entry.actor, session)
                        .map_err(|_| ())?;
                    if before.position() != target {
                        map.view()
                            .relocate_source_player(runtime, entry.actor, session, target)
                            .map_err(|_| ())?;
                    }
                } else {
                    if before.position() != target {
                        map.view()
                            .relocate_source_creature(runtime, entry.actor, target)
                            .map_err(|_| ())?;
                    }
                    runtime
                        .set_native_tenebris_movement_locked(native, source, entry.actor, true)
                        .map_err(|_| ())?;
                }
                entry.done = true;
            }
            if let Some(line) = oteryn_gameplay_encounters::bosses::source_callbacks::callback_speech::CallbackSpeech::cast(
                runtime, pending.proposal.issuer, "oteryn:creature.lady_tenebris", false,
            ).map_err(|_| ())? { super::spawns::emit_speech(room, vec![line]); }
            self.0.remove(index);
        }
        Ok(())
    }
}
