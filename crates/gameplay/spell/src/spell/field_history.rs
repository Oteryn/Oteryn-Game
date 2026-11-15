//! Attack history belongs to the existing player HP owner. Only a successfully
//! staged/committed real combat batch installs these successor data.
use crate::foundation::{ExactActorRef, GameSessionId};
#[derive(Debug, Clone, PartialEq, Eq)]
struct Hit {
    source: ExactActorRef,
    session: GameSessionId,
    lease: u64,
    at_us: u64,
}
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FieldAttackHistory {
    hits: Vec<Hit>,
    incomplete_at_us: Option<u64>,
}
impl FieldAttackHistory {
    /// Data staging only; the actual player-batch seal and physical commit are
    /// the issuer. Uncommanded environmental ticks have no attacker to record.
    pub fn record_staged_player_hit(
        &mut self,
        source: ExactActorRef,
        session: GameSessionId,
        lease: u64,
        at_us: u64,
    ) -> Result<(), oteryn_protocol_oteryn::actor_spell::SpellCastDisposition> {
        use oteryn_protocol_oteryn::actor_spell::SpellCastDisposition as D;
        if lease == 0 || self.hits.iter().any(|hit| hit.at_us > at_us) {
            return Err(D::Rejected);
        }
        if let Some(hit) = self.hits.iter_mut().find(|hit| hit.source == source) {
            *hit = Hit {
                source,
                session,
                lease,
                at_us,
            };
            return Ok(());
        }
        // Retain the newest bounded set. A discarded hit makes attribution
        // uncertain only while that hit can still be inside the world's fight window.
        if self.hits.len() >= crate::foundation::COMBAT01_DAMAGE_CONTRIBUTORS_PER_CREATURE_MAX {
            let oldest = self
                .hits
                .iter()
                .enumerate()
                .min_by_key(|(_, hit)| hit.at_us)
                .map(|(index, _)| index)
                .ok_or(D::Rejected)?;
            let discarded = self.hits.swap_remove(oldest);
            self.incomplete_at_us = Some(
                self.incomplete_at_us
                    .map_or(discarded.at_us, |at| at.max(discarded.at_us)),
            );
        }
        self.hits.try_reserve(1).map_err(|_| D::Rejected)?;
        self.hits.push(Hit {
            source,
            session,
            lease,
            at_us,
        });
        Ok(())
    }
    /// None is unavailable history, distinct from a proven absence. The World
    /// policy supplies genuine PZ_LOCKED duration; no default timer is invented.
    pub fn has_been_attacked(
        &self,
        source: ExactActorRef,
        session: GameSessionId,
        lease: u64,
        now_us: u64,
        in_fight_ms: u32,
    ) -> Option<bool> {
        let window_us = u64::from(in_fight_ms) * 1000;
        if self
            .incomplete_at_us
            .is_some_and(|at| now_us < at || now_us - at <= window_us)
            || self.hits.iter().any(|hit| hit.at_us > now_us)
        {
            return None;
        }
        Some(self.hits.iter().any(|hit| {
            hit.source == source
                && hit.session == session
                && hit.lease == lease
                && now_us - hit.at_us <= window_us
        }))
    }
    pub fn clear_on_lifecycle(&mut self) {
        self.hits.clear();
        self.incomplete_at_us = None;
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::*;
    use crate::foundation::{ActorLocalGeneration, ChannelId, WorldId};

    fn identity(generation: u64) -> (ExactActorRef, GameSessionId) {
        let id = |n| [n, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, n];
        let mut actor = ExactActorRef::transport_fixture(
            WorldId::decode(&id(1)).expect("world"),
            ChannelId::decode(&id(2)).expect("channel"),
        );
        actor.0.actor_local_generation = ActorLocalGeneration(generation);
        (actor, GameSessionId::decode(&id(3)).expect("session"))
    }

    #[test]
    fn expired_contributors_do_not_exhaust_field_history() {
        let mut history = FieldAttackHistory::default();
        for generation in 1..=17 {
            let (source, session) = identity(generation);
            let now = generation * 10_000;
            history
                .record_staged_player_hit(source, session, 1, now)
                .expect("hit");
            assert_eq!(
                history.has_been_attacked(source, session, 1, now, 1),
                Some(true)
            );
        }
    }

    #[test]
    fn overflow_uncertainty_expires_with_discarded_hits() {
        let mut history = FieldAttackHistory::default();
        for generation in 1..=17 {
            let (source, session) = identity(generation);
            history
                .record_staged_player_hit(source, session, 1, generation)
                .expect("hit");
        }
        let (source, session) = identity(1);
        assert_eq!(
            history.has_been_attacked(source, session, 1, 1_001, 1),
            None
        );
        assert_eq!(
            history.has_been_attacked(source, session, 1, 1_018, 1),
            Some(false)
        );
        let (source, session) = identity(18);
        history
            .record_staged_player_hit(source, session, 1, 2_000)
            .expect("new fight");
        assert_eq!(
            history.has_been_attacked(source, session, 1, 2_000, 1),
            Some(true)
        );
        assert!(
            history.hits.len() <= crate::foundation::COMBAT01_DAMAGE_CONTRIBUTORS_PER_CREATURE_MAX
        );
    }
}
