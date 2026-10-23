//! Tutorial composition. Reads are retried from the durable owner rather than dropping
//! a session's tracking after a failed read. No Channel lock is held during PostgreSQL work.
use super::ComposedFreshAdmission;
use crate::durability::character_progression::CurrentCharacterGameplayFence;
use crate::durability::character_tutorial::{StoredTutorialProgress, TutorialProgressOutcome};
use crate::foundation::{GameSessionId, MovementLocalPosition};
use oteryn_gameplay_quests::tutorial::{
    ISLET_TEMPLE, TutorialEvent, TutorialTransition, tutorial_transition,
};
use oteryn_protocol_oteryn::tutorial_notices::TutorialNotice;
use std::collections::{HashMap, VecDeque};
use std::sync::Mutex;

#[derive(Default)]
pub(super) struct TutorialSessions(Mutex<HashMap<GameSessionId, VecDeque<TutorialNotice>>>);

impl TutorialSessions {
    pub(super) fn notice(&self, session: GameSessionId, notice: TutorialNotice) {
        if let Ok(mut sessions) = self.0.lock() {
            let queue = sessions.entry(session).or_default();
            // A slow client cannot turn repeated hole attempts into an unbounded queue.
            if queue.len() < 8 {
                queue.push_back(notice);
            }
        }
    }

    pub(super) fn take(&self, session: GameSessionId) -> Option<TutorialNotice> {
        self.0.lock().ok()?.get_mut(&session)?.pop_front()
    }

    pub(super) fn forget(&self, session: GameSessionId) {
        if let Ok(mut sessions) = self.0.lock() {
            sessions.remove(&session);
        }
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn complete_tutorial_islet_admission(
        &self,
        actor: crate::foundation::ExactActorRef,
        session: GameSessionId,
    ) -> bool {
        if !self.tutorial_enabled().await {
            return true;
        }
        let Ok(Some(fence)) = self.current_quest_fence(session).await else {
            return false;
        };
        let stored = match self.tutorial_progress(fence).await {
            Ok(Some(stored)) => stored,
            Ok(None) => return true,
            Err(()) => return false,
        };
        if !stored.pending_islet {
            return true;
        }
        let placed = {
            let runtime = self.runtime.lock().await;
            if runtime.player_control_facts(actor, session).is_err()
                || runtime.owner_fence().is_err()
            {
                return false;
            }
            let Ok(position) = runtime.read_actor_position(actor) else {
                return false;
            };
            self.placed_record(&runtime, position.position())
        };
        match self
            .root
            .complete_tutorial_islet_placement(self.character, self.holder, fence, placed)
            .await
        {
            Ok(true) => {
                self.track_respawned(session, actor, &placed, fence);
                true
            }
            Ok(false) => true,
            Err(_) => false,
        }
    }

    pub(super) async fn tutorial_enabled(&self) -> bool {
        let start = self.runtime.lock().await.respawn_position();
        (start.x, start.y, start.floor) == ISLET_TEMPLE
    }

    pub(super) async fn tutorial_event(
        &self,
        fence: CurrentCharacterGameplayFence,
        event: TutorialEvent,
    ) -> Result<Option<TutorialTransition>, ()> {
        if !self.tutorial_enabled().await {
            return Ok(None);
        }
        let build = self
            .root
            .read_character_build_state(self.character, fence.character_id)
            .await
            .map_err(|_| ())?;
        let group = self
            .root
            .read_familiar_group(self.character, self.holder, fence)
            .await
            .map_err(|_| ())?
            .ok_or(())?;
        let mut stored = match self
            .root
            .character_tutorial_progress(self.character, self.holder, fence, None)
            .await
            .map_err(|_| ())?
        {
            TutorialProgressOutcome::Read(stored) => stored,
            _ => return Err(()),
        };
        for _ in 0..3 {
            let Some(next) = tutorial_transition(
                true,
                stored.map(|row| row.progress),
                event,
                build.vocation(),
                group.access(),
            ) else {
                return Ok(None);
            };
            let result = self
                .root
                .character_tutorial_progress(
                    self.character,
                    self.holder,
                    fence,
                    Some((stored.map_or(0, |row| row.version), next.progress)),
                )
                .await
                .map_err(|_| ())?;
            match result {
                TutorialProgressOutcome::Written(_) => {
                    if let Some(hint) = next.hint {
                        self.states
                            .tutorial_sessions
                            .notice(fence.game_session_id, TutorialNotice::Hint(hint));
                    }
                    return Ok(Some(next));
                }
                TutorialProgressOutcome::Conflict(current) => stored = current,
                TutorialProgressOutcome::Unchanged(_) => return Ok(None),
                TutorialProgressOutcome::Read(_) => return Err(()),
            }
        }
        Err(())
    }

    pub(super) async fn tutorial_progress(
        &self,
        fence: CurrentCharacterGameplayFence,
    ) -> Result<Option<StoredTutorialProgress>, ()> {
        match self
            .root
            .character_tutorial_progress(self.character, self.holder, fence, None)
            .await
            .map_err(|_| ())?
        {
            TutorialProgressOutcome::Read(row) => Ok(row),
            _ => Err(()),
        }
    }

    pub(super) async fn read_tutorial_admission_position(
        &self,
        fence: CurrentCharacterGameplayFence,
    ) -> Result<super::last_position::AdmissionPosition, ()> {
        use super::last_position::AdmissionPosition;
        let decided = self.read_admission_position(fence).await?;
        if matches!(decided, AdmissionPosition::Respawn(_)) {
            return Ok(decided);
        }
        if let Some(pending) = self
            .root
            .read_npc_pending_arrival(self.character, self.holder, fence)
            .await
            .map_err(|_| ())?
        {
            let (vitals, rest) = match &decided {
                AdmissionPosition::Stored(row) => (row.record.vitals, row.record.rest),
                _ => (None, None),
            };
            return Ok(AdmissionPosition::Arrival(pending, vitals, rest));
        }
        if !self.tutorial_enabled().await {
            return Ok(decided);
        }
        if self
            .tutorial_progress(fence)
            .await?
            .is_some_and(|stored| stored.pending_islet)
        {
            return Ok(AdmissionPosition::FirstLogin);
        }
        if self
            .tutorial_event(fence, TutorialEvent::Login)
            .await?
            .is_some_and(|next| next.place_islet)
        {
            return Ok(AdmissionPosition::FirstLogin);
        }
        if let AdmissionPosition::Stored(row) = &decided {
            self.reconcile_tutorial_position(
                fence,
                MovementLocalPosition {
                    x: row.record.cell.x,
                    y: row.record.cell.y,
                    floor: row.record.cell.floor,
                },
            )
            .await?;
        }
        Ok(decided)
    }

    pub(super) async fn reconcile_tutorial_position(
        &self,
        fence: CurrentCharacterGameplayFence,
        cell: MovementLocalPosition,
    ) -> Result<(), ()> {
        use oteryn_gameplay_quests::tutorial::TutorialStep;
        if !tutorial_cell(cell) {
            return Ok(());
        }
        if !((32532..=32539).contains(&cell.x) && (32511..=32518).contains(&cell.y))
            && self
                .tutorial_progress(fence)
                .await?
                .is_some_and(|row| row.progress.step == TutorialStep::Islet)
        {
            self.tutorial_event(fence, TutorialEvent::Forcefield)
                .await?;
        }
        if matches!((cell.x, cell.y, cell.floor), (32561 | 32562, 32485, 7)) {
            self.tutorial_event(fence, TutorialEvent::Hunting).await?;
        }
        Ok(())
    }
}

pub(super) fn tutorial_cell(cell: MovementLocalPosition) -> bool {
    crate::content::newhaven_tutorial_land::contains(cell.x, cell.y, cell.floor)
}
