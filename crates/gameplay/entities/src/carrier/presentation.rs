//! Volatile presentation facts, fenced by the exact actor and position revision.
use super::*;

#[derive(Debug)]
pub(super) struct ActorPresentation {
    actor: ExactActorRef,
    walk: Option<(u64, u32)>,
    house_walk: Option<(GameSessionId, u64, u32)>,
    mount: u32,
    speed_bonus: u16,
}

impl ChannelRuntimeV1 {
    /// Called in the same owner turn as a successful timed step. Any later
    /// translation without this receipt is observed as a teleport, even adjacent.
    pub fn record_walk(
        &mut self,
        actor: ExactActorRef,
        snapshot: MovementPositionSnapshot,
        duration: std::time::Duration,
    ) {
        if self
            .carrier
            .read_movement_position(&self.continuity, actor.0)
            .ok()
            != Some(snapshot)
        {
            return;
        }
        let Ok(ms) = u32::try_from(duration.as_millis()) else {
            return;
        };
        let entry = self.presentation_entry(actor);
        entry.walk = Some((snapshot.last_translation_revision(), ms));
    }

    fn presentation_entry(&mut self, actor: ExactActorRef) -> &mut ActorPresentation {
        let entry = self
            .presentations
            .entry(actor.actor_local_id())
            .or_insert(ActorPresentation {
                actor,
                walk: None,
                house_walk: None,
                mount: 0,
                speed_bonus: 0,
            });
        if entry.actor != actor {
            *entry = ActorPresentation {
                actor,
                walk: None,
                house_walk: None,
                mount: 0,
                speed_bonus: 0,
            };
        }
        entry
    }

    /// The validated selected mount is supplied by the appearance owner; riding
    /// never changes that selection. Control-loss and stale sessions cannot write.
    pub fn set_riding(
        &mut self,
        actor: ExactActorRef,
        session: GameSessionId,
        mount: u32,
        speed_bonus: u16,
    ) -> Result<(), CarrierError> {
        self.owner_fence()?;
        if self
            .player_control_facts(actor, session)?
            .control_loss
            .is_some()
        {
            return Err(CarrierError::PositionSnapshotMismatch);
        }
        self.record_riding(actor, mount, speed_bonus);
        Ok(())
    }

    pub fn set_house_riding(
        &mut self,
        actor: ExactActorRef,
        session: GameSessionId,
        mount: u32,
        speed_bonus: u16,
    ) -> Result<(), CarrierError> {
        if self
            .house_player_control_facts(actor, session)?
            .control_loss
            .is_some()
            || self.house_player_writes_fenced(actor, session)?
        {
            return Err(CarrierError::PositionSnapshotMismatch);
        }
        self.record_riding(actor, mount, speed_bonus);
        Ok(())
    }

    pub fn house_actor_motion(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> Option<(u64, u32, u32, u16)> {
        let movement = self.read_house_movement(actor, session).ok()?;
        let entry = self
            .presentations
            .get(&actor.actor_local_id())
            .filter(|p| p.actor == actor);
        Some((
            movement.revision(),
            entry
                .and_then(|p| p.house_walk)
                .filter(|(owner, at, _)| *owner == session && *at == movement.revision())
                .map_or(0, |(_, _, ms)| ms),
            entry.map_or(0, |p| p.mount),
            entry.map_or(0, |p| p.speed_bonus),
        ))
    }

    /// Record the timed step under the same House owner turn that moved the actor.
    pub fn record_house_walk(
        &mut self,
        actor: ExactActorRef,
        session: GameSessionId,
        snapshot: &super::house_custody::HouseMovementSnapshot,
        duration: std::time::Duration,
    ) {
        if self.read_house_movement(actor, session).as_ref().ok() != Some(snapshot) {
            return;
        }
        if let Ok(ms) = u32::try_from(duration.as_millis()) {
            self.presentation_entry(actor).house_walk = Some((session, snapshot.revision(), ms));
        }
    }

    fn record_riding(&mut self, actor: ExactActorRef, mount: u32, speed_bonus: u16) {
        let entry = self.presentation_entry(actor);
        entry.mount = mount;
        entry.speed_bonus = if mount == 0 { 0 } else { speed_bonus };
    }

    pub fn actor_motion(&self, actor: ExactActorRef) -> Option<(u64, u32, u32, u16)> {
        let snapshot = self
            .carrier
            .read_movement_position(&self.continuity, actor.0)
            .ok()?;
        let entry = self
            .presentations
            .get(&actor.actor_local_id())
            .filter(|p| p.actor == actor);
        let revision = snapshot.last_translation_revision();
        Some((
            revision,
            entry
                .and_then(|p| p.walk)
                .filter(|(at, _)| *at == revision)
                .map_or(0, |(_, ms)| ms),
            entry.map_or(0, |p| p.mount),
            entry.map_or(0, |p| p.speed_bonus),
        ))
    }
}
