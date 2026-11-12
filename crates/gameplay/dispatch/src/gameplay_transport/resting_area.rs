//! RESTING-AREA-0: session-local resting projection and owner cadence. Since Winter Update
//! 2026 the projected daily streak no longer selects the bonus tier.

use crate::foundation::{ExactActorRef, GameSessionId};
use std::{collections::BTreeMap, sync::Mutex};

pub(super) fn soul_interval(vocation: crate::spell::Vocation, premium: bool) -> u16 {
    use crate::domain::premium::PremiumStatus;
    use crate::spell::Vocation;
    let promoted = matches!(
        vocation,
        Vocation::EliteKnight
            | Vocation::RoyalPaladin
            | Vocation::MasterSorcerer
            | Vocation::ElderDruid
            | Vocation::ExaltedMonk
    );
    let gate = if premium {
        PremiumStatus::Current
    } else {
        PremiumStatus::NotCurrent
    }
    .gate();
    oteryn_gameplay_characters::soul::soul_schedule(promoted, gate).interval_seconds as u16
}

/// Winter Update 2026 (thread 565): resting-area bonuses no longer depend on the daily
/// reward streak. Inside an area every Character gets the full tier its account allows:
/// Free HP and mana regeneration, Premium doubled HP and mana plus stamina and soul (the
/// spell owner caps a Free account at tier 3).
pub(super) const RESTING_TIER: u32 = 7;

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
    /// A retained actor reloads only the durable streak. Its partial cadence
    /// belongs to the existing owner and is not restarted by a House resume.
    pub(super) fn refresh(&self, session: GameSessionId, actor: ExactActorRef, streak: u32) {
        if let Ok(mut sessions) = self.0.lock() {
            match sessions.get_mut(&session) {
                Some(held) if held.actor == actor => held.streak = streak,
                _ => {
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
        }
    }

    /// Called only after the same native actor's actual committed handoff.
    /// Missing data grants no cadence; a conflicting actor fails closed.
    pub(super) fn transfer_session(
        &self,
        source: GameSessionId,
        destination: GameSessionId,
        actor: ExactActorRef,
    ) -> Result<(), ()> {
        if source == destination {
            return Err(());
        }
        let mut sessions = self.0.lock().map_err(|_| ())?;
        if sessions
            .get(&source)
            .is_some_and(|held| held.actor != actor)
            || sessions
                .get(&destination)
                .is_some_and(|held| held.actor != actor)
            || (sessions.contains_key(&source) && sessions.contains_key(&destination))
        {
            return Err(());
        }
        if let Some(held) = sessions.remove(&source) {
            sessions.insert(destination, held);
        }
        Ok(())
    }

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
    /// The regeneration tier credited this second; a session without its admitted
    /// projection gets none.
    pub(super) fn tier(&self, session: GameSessionId, actor: ExactActorRef, inside: bool) -> u32 {
        if inside && self.streak(session, actor).is_some() {
            RESTING_TIER
        } else {
            0
        }
    }
    pub(super) fn cadence(
        &self,
        session: GameSessionId,
        actor: ExactActorRef,
        inside: bool,
        premium: bool,
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
        // Winter Update 2026: stamina and soul regenerate for every Premium character
        // inside, whatever its streak. Premium is read per second so an expiry
        // immediately closes these cadences.
        let stamina_due = if premium && stamina < 2520 {
            p.stamina_seconds = p.stamina_seconds.saturating_add(1);
            // Match characters::stamina: the top three hours start at minute 2340.
            let interval = if stamina >= 2340 { 360 } else { 180 };
            if p.stamina_seconds >= interval {
                p.stamina_seconds = 0;
                true
            } else {
                false
            }
        } else {
            false
        };
        let soul_due = if premium {
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
    fn daily_resting_soul_interval_uses_promotion_rather_than_premium_cap() {
        use crate::spell::Vocation;
        for vocation in [
            Vocation::None,
            Vocation::Knight,
            Vocation::Paladin,
            Vocation::Sorcerer,
            Vocation::Druid,
            Vocation::Monk,
        ] {
            assert_eq!(soul_interval(vocation, true), 120);
            assert_eq!(soul_interval(vocation, false), 120);
        }
        for vocation in [
            Vocation::EliteKnight,
            Vocation::RoyalPaladin,
            Vocation::MasterSorcerer,
            Vocation::ElderDruid,
            Vocation::ExaltedMonk,
        ] {
            assert_eq!(soul_interval(vocation, true), 15);
            assert_eq!(soul_interval(vocation, false), 120);
        }
    }

    #[test]
    fn daily_resting_thresholds_and_stamina_model_boundaries() {
        for streak in 0..=8 {
            for inside in [false, true] {
                for premium in [false, true] {
                    for (stamina, interval) in [
                        (2339, 180),
                        (2340, 360),
                        (2341, 360),
                        (2519, 360),
                        (2520, 360),
                    ] {
                        let owner = RestingAreaSessions::default();
                        owner.install(session(), actor(), streak);
                        let mut gains = (0, 0);
                        for second in 1..=interval {
                            let (stamina_due, soul_due) =
                                owner.cadence(session(), actor(), inside, premium, stamina, 120);
                            gains.0 += u32::from(stamina_due);
                            gains.1 += u32::from(soul_due);
                            if second < interval {
                                assert_eq!(
                                    gains.0, 0,
                                    "streak={streak}, stamina={stamina}, second={second}"
                                );
                            }
                        }
                        // Winter Update 2026: the streak no longer gates either cadence.
                        assert_eq!(gains.0, u32::from(inside && premium && stamina < 2520));
                        assert_eq!(gains.1, if inside && premium { interval / 120 } else { 0 });
                    }
                }
            }
        }
    }

    #[test]
    fn leaving_the_area_removes_due_stamina_and_soul_progress() {
        let owner = RestingAreaSessions::default();
        owner.install(session(), actor(), 7);
        for _ in 0..179 {
            owner.cadence(session(), actor(), true, true, 1000, 120);
        }
        assert_eq!(
            owner.cadence(session(), actor(), false, true, 1000, 120),
            (false, false)
        );
        for _ in 0..179 {
            owner.cadence(session(), actor(), true, true, 1000, 120);
        }
        assert!(owner.cadence(session(), actor(), true, true, 1000, 120).0);
    }

    #[test]
    fn house_handoff_and_retained_refresh_preserve_partial_resting_cadences() {
        let owner = RestingAreaSessions::default();
        let destination = GameSessionId::decode(&id(5)).expect("destination");
        owner.install(session(), actor(), 7);
        for _ in 0..179 {
            owner.cadence(session(), actor(), true, true, 1000, 120);
        }
        owner
            .transfer_session(session(), destination, actor())
            .expect("native handoff");
        assert_eq!(owner.streak(session(), actor()), None);
        owner.refresh(destination, actor(), 7);
        assert_eq!(
            owner.cadence(destination, actor(), true, true, 1000, 120),
            (true, false)
        );
        owner
            .transfer_session(session(), destination, actor())
            .expect("idempotent replay");
        for _ in 0..59 {
            assert!(!owner.cadence(destination, actor(), true, true, 1000, 120).1);
        }
        assert!(owner.cadence(destination, actor(), true, true, 1000, 120).1);
        owner
            .transfer_session(destination, session(), actor())
            .expect("native return");
        assert_eq!(owner.streak(destination, actor()), None);
        assert_eq!(owner.streak(session(), actor()), Some(7));
    }

    #[test]
    fn premium_is_required_for_stamina_and_soul_tiers() {
        let owner = RestingAreaSessions::default();
        owner.install(session(), actor(), 7);
        for _ in 0..360 {
            assert_eq!(
                owner.cadence(session(), actor(), true, false, 1000, 1),
                (false, false)
            );
        }
        assert_eq!(
            owner.cadence(session(), actor(), true, true, 1000, 1),
            (false, true)
        );
    }

    #[test]
    fn resting_free_account_gets_hp_and_mana_without_streak() {
        // Winter Update 2026: the area grants its bonus tier without any reward streak;
        // the spell owner caps a Free account at HP and mana regeneration.
        let owner = RestingAreaSessions::default();
        owner.install(session(), actor(), 0);
        assert_eq!(owner.tier(session(), actor(), true), RESTING_TIER);
        assert_eq!(owner.tier(session(), actor(), false), 0);
        assert_eq!(
            RestingAreaSessions::default().tier(session(), actor(), true),
            0
        );
        for _ in 0..360 {
            assert_eq!(
                owner.cadence(session(), actor(), true, false, 1000, 1),
                (false, false)
            );
        }
    }

    #[test]
    fn resting_premium_gets_double_and_stamina_soul_without_streak() {
        let owner = RestingAreaSessions::default();
        owner.install(session(), actor(), 0);
        assert_eq!(owner.tier(session(), actor(), true), RESTING_TIER);
        let mut gains = (0, 0);
        for _ in 0..180 {
            let (stamina, soul) = owner.cadence(session(), actor(), true, true, 1000, 120);
            gains.0 += u32::from(stamina);
            gains.1 += u32::from(soul);
        }
        assert_eq!(gains, (1, 1));
    }
}
