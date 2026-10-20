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
pub(crate) struct FieldAttackHistory {
    hits: Vec<Hit>,
    incomplete: bool,
}
impl FieldAttackHistory {
    /// Data staging only; the actual player-batch seal and physical commit are
    /// the issuer. Uncommanded environmental ticks have no attacker to record.
    pub(crate) fn record_staged_player_hit(
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
        // The existing contributor bound is retained. Exceeding it explicitly
        // makes future field attribution unavailable, never false known-none,
        // and does not drop an otherwise valid committed HP change.
        if self.hits.len() >= crate::foundation::COMBAT01_DAMAGE_CONTRIBUTORS_PER_CREATURE_MAX {
            self.incomplete = true;
            return Ok(());
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
    pub(crate) fn has_been_attacked(
        &self,
        source: ExactActorRef,
        session: GameSessionId,
        lease: u64,
        now_us: u64,
        in_fight_ms: u32,
    ) -> Option<bool> {
        if self.incomplete || self.hits.iter().any(|hit| hit.at_us > now_us) {
            return None;
        }
        Some(self.hits.iter().any(|hit| {
            hit.source == source
                && hit.session == session
                && hit.lease == lease
                && now_us - hit.at_us <= u64::from(in_fight_ms) * 1000
        }))
    }
    pub(crate) fn clear_on_lifecycle(&mut self) {
        self.hits.clear();
        self.incomplete = false;
    }
}
