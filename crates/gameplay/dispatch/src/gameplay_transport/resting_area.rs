//! RESTING-AREA-0: session-local daily-streak projection and owner cadence.

use crate::foundation::{ExactActorRef, GameSessionId};
use std::{collections::BTreeMap, sync::Mutex};

#[derive(Debug, Clone, Copy)]
struct Projection {
    actor: ExactActorRef,
    streak: u32,
    stamina_seconds: u16,
    soul_seconds: u16,
}

#[derive(Default)]
pub(super) struct RestingAreaSessions(Mutex<BTreeMap<GameSessionId, Projection>>);

impl RestingAreaSessions {
    pub(super) fn install(&self, session: GameSessionId, actor: ExactActorRef, streak: u32) {
        if let Ok(mut sessions) = self.0.lock() {
            sessions.insert(
                session,
                Projection {
                    actor,
                    streak,
                    stamina_seconds: 0,
                    soul_seconds: 0,
                },
            );
        }
    }
    pub(super) fn streak(&self, session: GameSessionId, actor: ExactActorRef) -> Option<u32> {
        self.0
            .lock()
            .ok()?
            .get(&session)
            .filter(|p| p.actor == actor)
            .map(|p| p.streak)
    }
    pub(super) fn cadence(
        &self,
        session: GameSessionId,
        actor: ExactActorRef,
        inside: bool,
        stamina: u16,
        soul_interval: u16,
    ) -> (bool, bool) {
        let Ok(mut sessions) = self.0.lock() else {
            return (false, false);
        };
        let Some(p) = sessions.get_mut(&session).filter(|p| p.actor == actor) else {
            return (false, false);
        };
        if !inside {
            p.stamina_seconds = 0;
            p.soul_seconds = 0;
            return (false, false);
        }
        let stamina_due = if p.streak >= 4 && stamina < 2520 {
            p.stamina_seconds = p.stamina_seconds.saturating_add(1);
            let interval = if stamina > 2340 { 360 } else { 180 };
            if p.stamina_seconds >= interval {
                p.stamina_seconds = 0;
                true
            } else {
                false
            }
        } else {
            false
        };
        let soul_due = if p.streak >= 7 {
            p.soul_seconds = p.soul_seconds.saturating_add(1);
            if p.soul_seconds >= soul_interval {
                p.soul_seconds = 0;
                true
            } else {
                false
            }
        } else {
            false
        };
        (stamina_due, soul_due)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]
    use super::*;
    use crate::foundation::{ChannelId, WorldId};

    fn id(n: u8) -> [u8; 16] {
        [1, 0, 0, 0, 0, n, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, n]
    }
    fn actor() -> ExactActorRef {
        ExactActorRef::transport_fixture(
            WorldId::decode(&id(1)).expect("world"),
            ChannelId::decode(&id(2)).expect("channel"),
        )
    }
    fn session() -> GameSessionId {
        GameSessionId::decode(&id(4)).expect("session")
    }

    #[test]
    fn leaving_the_area_removes_due_stamina_and_soul_progress() {
        let owner = RestingAreaSessions::default();
        owner.install(session(), actor(), 7);
        for _ in 0..179 {
            owner.cadence(session(), actor(), true, 1000, 120);
        }
        assert_eq!(
            owner.cadence(session(), actor(), false, 1000, 120),
            (false, false)
        );
        for _ in 0..179 {
            owner.cadence(session(), actor(), true, 1000, 120);
        }
        assert!(owner.cadence(session(), actor(), true, 1000, 120).0);
    }
}
