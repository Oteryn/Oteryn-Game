//! Native canopic-jar USE ingress; the World owner serializes all Channels.
use super::*;
use crate::durability::world_event::changes::timed::{EventKind, EventState};

#[derive(Clone)]
pub(super) struct Pending {
    command: UseCommand,
    fence: CurrentCharacterGameplayFence,
    action: u16,
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn reconcile_mummy_use(&self, session: GameSessionId) -> bool {
        let pending = match self.states.quest_callbacks.mummy.lock() {
            Ok(pending) => pending.get(&session).cloned(),
            Err(_) => return false,
        };
        let Some(pending) = pending else {
            return true;
        };
        let Ok(id) = CommandId::new(pending.command.command_id) else {
            return false;
        };
        let result = self
            .root
            .reconcile_mummy_use(
                self.character,
                pending.fence.character_id,
                self.world_id,
                self.reset_epoch,
                CommandRef::new(session, id),
                pending.action,
            )
            .await;
        let result = match result {
            Ok(Some(true)) => UseDisposition::Committed,
            Ok(Some(false)) | Ok(None) => UseDisposition::NothingToUse,
            Err(_) => return false,
        };
        if self
            .states
            .command_ingress
            .complete_quest_callback(CommandRef::new(session, id), result)
            .is_none()
        {
            return false;
        }
        match self.states.quest_callbacks.mummy.lock() {
            Ok(mut pending) => {
                pending.remove(&session);
                true
            }
            Err(_) => false,
        }
    }

    pub(super) async fn use_mummy_jar(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        map: &BundleMap,
        target: &WorldObjectTarget,
    ) -> Option<UseOutcome> {
        let source = super::super::world_map::MapViewSource {
            overlay: &map.overlay,
            facts: &map.facts,
            content_generation: self.active_generation?.identity().client_artifact_digest(),
            reset_epoch: self.reset_epoch,
        };
        let resolved =
            super::super::world_map::resolve_map_target(&source, target, |_| None::<()>).ok()?;
        let action = (50006..=50010).find(|action| {
            map.facts
                .quest_callback_matches(resolved.placement_key, None, Some(*action), None)
        })?;
        let result=tokio::time::timeout(std::time::Duration::from_secs(10),async {
            let Some(_permit)=self.spell_lane_permit().await else { return UseDisposition::Rejected; };
            let Ok(Some(fence))=self.current_quest_fence(command.game_session_id).await else { return UseDisposition::Rejected; };
            if command.item_fence != Some(fence.item_fence()) { return UseDisposition::Rejected; }
            {
                let runtime=self.runtime.lock().await;
                if !runtime.player_control_facts(actor,command.game_session_id).is_ok_and(|f|f.control_loss.is_none())
                    || !runtime.bound_attacker_lease(actor,command.game_session_id).is_ok_and(|l| l.character_id().as_bytes()==fence.character_id.as_bytes() && l.generation()==fence.character_lease_generation) {
                    return UseDisposition::Rejected;
                }
                let Ok(position)=runtime.read_actor_position(actor) else { return UseDisposition::Rejected; };
                let Some(position)=super::native(position.position()) else { return UseDisposition::Rejected; };
                if !super::super::map_door::reachable(position,resolved.pos) { return UseDisposition::TooFar; }
                if map.facts.object_revision(resolved.placement_key)!=target.expected_revision { return UseDisposition::StaleState; }
            }
            {
                let states=self.states.spell_states.lock().await;
                if states.is_dead(actor) { return UseDisposition::Rejected; }
            }
            if !self.states.world_gate.lock().is_ok_and(|mut g|g.admit(actor,self.clock_origin.elapsed())) { return UseDisposition::Exhausted; }
            let Ok(id)=CommandId::new(command.command_id) else { return UseDisposition::Rejected; };
            {
                let Ok(mut pending)=self.states.quest_callbacks.mummy.lock() else { return UseDisposition::Rejected; };
                pending.insert(command.game_session_id,Pending{command,fence,action});
            }
            let result=self.root.use_mummy_jar(self.character,self.holder,fence,self.reset_epoch,CommandRef::new(command.game_session_id,id),action).await;
            let (applied,state)=match result {
                Ok(Some(result))=>result,
                Ok(None)=> {
                    if let Ok(mut pending)=self.states.quest_callbacks.mummy.lock() { pending.remove(&command.game_session_id); }
                    return UseDisposition::RequirementNotMet;
                }
                Err(_)=>return UseDisposition::Rejected, // retained pending input keeps ingress held
            };
            if let Ok(mut pending)=self.states.quest_callbacks.mummy.lock() { pending.remove(&command.game_session_id); }
            if applied {
                self.project_mummy_state(map,&state);
                if state.objects == 5 && self.wake_mummy_raid().await.is_err() {
                    super::super::operator_event("wevents_mummy_spawn_admission_retry");
                }
                UseDisposition::Committed
            } else {
                self.states.chat.push_quest_feedback(command.game_session_id,"You are too afraid to destroy this object. Wait 30 minutes before trying this jar again.");
                UseDisposition::NothingToUse
            }
        }).await.unwrap_or(UseDisposition::Rejected);
        Some(UseOutcome {
            disposition: result,
            ..UseOutcome::rejected()
        })
    }

    fn project_mummy_state(&self, map: &BundleMap, state: &EventState) {
        let Ok(runtime) = self.runtime.try_lock() else {
            return;
        };
        if runtime.owner_fence().is_err()
            || map.overlay.runtime_scope() != runtime.binding().runtime_scope()
        {
            return;
        }
        let Some(now) = crate::world_clock::now_utc_ms().and_then(|n| i64::try_from(n).ok()) else {
            return;
        };
        let (barrier, wall) = state.mummy_walls(now);
        if !map
            .facts
            .project_mummy_objects(map.overlay.base(), state.objects, barrier, wall)
        {
            super::super::operator_event("wevents_mummy_projection_retry");
        }
    }

    pub(in crate::gameplay_transport) async fn project_timed_events(&self) {
        let Some(map) = self.bundle_map() else {
            return;
        };
        self.project_shared_event_objects(map).await;
        if let Ok(Some(state)) = self
            .root
            .timed_world_event(self.world_id, self.reset_epoch, EventKind::MummysCurse)
            .await
        {
            self.project_mummy_state(map, &state);
        }
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    async fn wake_mummy_raid(&self) -> Result<(), ()> {
        use crate::durability::raid_schedule::{
            CalendarRaidScheduleCandidate, RaidScheduleCandidate, RaidSchedulerCheck,
        };
        use oteryn_gameplay_encounter_rules::raid::{CatalogueSchedule, RaidCatalogue};
        use sha2::{Digest, Sha256};
        let now = crate::world_clock::now_utc_ms().ok_or(())?;
        let now_ms = i64::try_from(now).map_err(|_| ())?;
        let digest = self.raid_bundle_digest().ok_or(())?;
        let generation = { self.runtime.lock().await.binding().scope_generation() };
        let key =
            crate::content::ProductionKey::new("raid:world-events.horestis").map_err(|_| ())?;
        let mut hash = Sha256::new();
        hash.update(b"oteryn:wevents:horestis:v1\0");
        hash.update(self.world_id.as_bytes());
        hash.update(self.reset_epoch.to_be_bytes());
        let mut firing = [0; 16];
        firing.copy_from_slice(&hash.finalize()[..16]);
        firing[..6].copy_from_slice(&now.to_be_bytes()[2..]);
        firing[6] = 0x70 | (firing[6] & 15);
        firing[8] = 0x80 | (firing[8] & 63);
        let catalogue = RaidCatalogue::embedded().map_err(|_| ())?;
        let raid = catalogue.get(&key).ok_or(())?;
        let selected = raid.select(digest, firing).map_err(|_| ())?;
        let CatalogueSchedule::Calendar {
            window: Some(window),
            interval_ms: Some(interval_ms),
            repeat_persistent,
            start_immediately,
        } = raid.schedule
        else {
            return Err(());
        };
        self.root
            .check_raid_schedules(
                self.character,
                self.holder,
                RaidSchedulerCheck {
                    world: self.world_id,
                    channel: self.channel_id,
                    generation,
                    reset_epoch: self.reset_epoch,
                    now_ms,
                    bundle_digest: digest,
                    candidates: vec![RaidScheduleCandidate {
                        key,
                        definition: selected.definition,
                        eligible: true,
                        calendar: None,
                        firing_id: firing,
                        global_margin_ms: 0,
                        lua: None,
                        calendar_window: Some(CalendarRaidScheduleCandidate {
                            window: window.window(),
                            interval_ms,
                            repeat_persistent,
                            start_immediately,
                        }),
                    }],
                },
            )
            .await
            .map_err(|_| ())?;
        Ok(())
    }
}
