//! Projection of GameplayIdleState for the source World-wide raid predicate.
//! The connection remains the only writer of activity and training clocks.
use super::ComposedFreshAdmission;
use super::connection::{AdmittedSession, GameplayIdleState};
use crate::durability::raid_schedule::{
    MAX_RAID_ACTIVITY_PLAYERS, RaidActivityPlayer, RaidActivitySnapshot,
};
use crate::foundation::{
    ChannelRuntimeV1, ExactActorRef, GameSessionId, RuntimeScopeRefV1, ScopeOwnershipGeneration,
};
use oteryn_gameplay_encounter_rules::raid::{CHECK_INTERVAL_MS, LUA_RAID_ACTIVE_IDLE_MS};
use std::collections::HashMap;
use std::sync::Mutex;
use tokio::time::Instant;

#[derive(Clone, Copy)]
struct Observation {
    actor: ExactActorRef,
    connection_generation: u64,
    idle: GameplayIdleState,
}

#[derive(Clone, PartialEq, Eq)]
struct Publication {
    reset_epoch: u64,
    scope: RuntimeScopeRefV1,
    generation: ScopeOwnershipGeneration,
    window: i64,
    expected: usize,
    sources: Vec<(GameSessionId, ExactActorRef, Option<Instant>)>,
}

struct PendingPublication {
    publication: Publication,
    players: Option<Vec<RaidActivityPlayer>>,
}

#[derive(Default)]
struct Presence {
    sessions: HashMap<GameSessionId, Observation>,
    published: Option<Publication>,
}

#[derive(Default)]
pub(super) struct GameplayIdlePresence {
    inner: Mutex<Presence>,
}

impl GameplayIdlePresence {
    pub(super) fn observe(&self, admitted: AdmittedSession) {
        let Some(actor) = admitted.runtime_actor else {
            return;
        };
        let Ok(mut presence) = self.inner.lock() else {
            return;
        };
        match presence.sessions.get(&admitted.game_session_id) {
            Some(previous)
                if previous.connection_generation > admitted.continuity.connection_generation =>
            {
                return;
            }
            // The complete census below refuses a partial projection.
            None if presence.sessions.len() >= MAX_RAID_ACTIVITY_PLAYERS => return,
            _ => {}
        }
        presence.sessions.insert(
            admitted.game_session_id,
            Observation {
                actor,
                connection_generation: admitted.continuity.connection_generation,
                idle: admitted.continuity.gameplay_idle,
            },
        );
    }

    pub(super) fn forget(&self, session: GameSessionId) {
        if let Ok(mut presence) = self.inner.lock() {
            presence.sessions.remove(&session);
        }
    }

    /// Same-process House handoff recovery may carry the original idle owner
    /// state only for the exact retained physical actor and GameSession.
    pub(super) fn retained_idle(
        &self,
        session: GameSessionId,
        actor: ExactActorRef,
    ) -> Option<GameplayIdleState> {
        let presence = self.inner.lock().ok()?;
        let retained = presence.sessions.get(&session)?;
        (retained.actor == actor).then_some(retained.idle)
    }

    fn pending(
        &self,
        runtime: &ChannelRuntimeV1,
        reset_epoch: u64,
        now: Instant,
        now_ms: i64,
    ) -> Result<Option<PendingPublication>, ()> {
        let expected = runtime.current_player_session_count().map_err(|_| ())?;
        if expected > MAX_RAID_ACTIVITY_PLAYERS || now_ms < 0 {
            return Err(());
        }
        let presence = self.inner.lock().map_err(|_| ())?;
        let mut sources = Vec::with_capacity(expected);
        for (&session, observation) in &presence.sessions {
            // Physical grace actors remain players. A removed/rebound slot is
            // not counted just because a connection's final copy still exists.
            if runtime
                .player_control_facts(observation.actor, session)
                .is_ok()
                || runtime
                    .house_actor_state(observation.actor, session)
                    .is_ok()
            {
                sources.push((session, observation.actor, observation.idle.last_activity()));
            }
        }
        sources.sort_unstable_by_key(|(session, _, _)| *session.as_bytes());
        let publication = Publication {
            reset_epoch,
            scope: runtime.binding().runtime_scope(),
            generation: runtime.binding().scope_generation(),
            window: now_ms / CHECK_INTERVAL_MS,
            expected,
            sources,
        };
        if presence.published.as_ref() == Some(&publication) {
            return Ok(None);
        }
        if publication.sources.len() != expected
            || publication
                .sources
                .iter()
                .any(|(_, _, since)| since.is_none())
        {
            // Replace a previously complete report with unknown immediately;
            // leaving the old row usable would invent facts about a new actor.
            return Ok(Some(PendingPublication {
                publication,
                players: None,
            }));
        }
        let mut players = Vec::new();
        for &(session, _, since) in &publication.sources {
            if let Some(active_until_ms) = active_until(since.ok_or(())?, now, now_ms)? {
                players.push(RaidActivityPlayer {
                    session,
                    active_until_ms,
                });
            }
        }
        Ok(Some(PendingPublication {
            publication,
            players: Some(players),
        }))
    }

    fn acknowledge(&self, publication: Publication) -> Result<(), ()> {
        self.inner.lock().map_err(|_| ())?.published = Some(publication);
        Ok(())
    }
}

fn active_until(since: Instant, now: Instant, now_ms: i64) -> Result<Option<i64>, ()> {
    let elapsed = now.saturating_duration_since(since).as_millis();
    if elapsed >= u128::from(LUA_RAID_ACTIVE_IDLE_MS) {
        return Ok(None);
    }
    let remaining = u128::from(LUA_RAID_ACTIVE_IDLE_MS) - elapsed;
    Ok(Some(
        now_ms
            .checked_add(i64::try_from(remaining).map_err(|_| ())?)
            .ok_or(())?,
    ))
}

impl ComposedFreshAdmission<'_, '_, '_> {
    /// Publish only a changed source/roster or the next source check window.
    /// SQL proves the node, physical scope generation and active World reset again.
    pub(super) async fn publish_raid_activity(&self, now_ms: i64) -> Result<(), ()> {
        let pending = {
            let runtime = self.runtime.lock().await;
            if runtime.binding().world_id() != self.world_id {
                return Err(());
            }
            self.states.gameplay_idle_presence.pending(
                &runtime,
                self.reset_epoch,
                Instant::now(),
                now_ms,
            )?
        };
        let Some(PendingPublication {
            publication,
            players,
        }) = pending
        else {
            return Ok(());
        };
        let snapshot = RaidActivitySnapshot {
            world: self.world_id,
            scope: publication.scope,
            generation: publication.generation,
            reset_epoch: self.reset_epoch,
            observed_at_ms: now_ms,
            players,
        };
        if !self
            .root
            .publish_raid_activity_snapshot(self.holder, snapshot)
            .await
            .map_err(|_| ())?
        {
            return Err(());
        }
        self.states.gameplay_idle_presence.acknowledge(publication)
    }

    /// Activity participants remain Game players. Enumerate the existing
    /// registered physical owners, including empty rooms: an absent report is
    /// unknown, so an empty ready Instance must explicitly publish known zero.
    /// The caller polls this pass alongside the existing autonomous room jobs.
    pub(super) async fn publish_activity_raid_presence(&self) -> Result<(), ()> {
        let owners = self.activity_runtimes.activity_owners()?;
        let mut complete = true;
        for owner in owners {
            let view = self.runtime_view(&owner);
            let now_ms = crate::world_clock::now_utc_ms()
                .and_then(|now| i64::try_from(now).ok())
                .ok_or(())?;
            complete &= view.publish_raid_activity(now_ms).await.is_ok();
        }
        if complete { Ok(()) } else { Err(()) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn projection_keeps_the_source_idle_boundary_and_never_refreshes_it() {
        let since = Instant::now();
        assert_eq!(active_until(since, since, 10_000), Ok(Some(310_000)));
        assert_eq!(
            active_until(since, since + Duration::from_millis(299_999), 309_999),
            Ok(Some(310_000))
        );
        assert_eq!(
            active_until(since, since + Duration::from_millis(300_000), 310_000),
            Ok(None)
        );
        assert_eq!(
            active_until(since, since + Duration::from_millis(900_000), 910_000),
            Ok(None)
        );
    }

    #[test]
    fn projection_rejects_utc_overflow() {
        let since = Instant::now();
        assert_eq!(active_until(since, since, i64::MAX), Err(()));
    }
}
