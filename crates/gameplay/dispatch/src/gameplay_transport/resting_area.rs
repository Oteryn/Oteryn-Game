//! RESTING-AREA-0: session-local daily-streak projection and owner cadence.

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
        // Global unlocks tiers four through seven only for Premium characters.  Keep the
        // streak projection independent of Premium so an expiry immediately closes these
        // cadences without weakening the durable daily-reward owner.
        let stamina_due = if premium && p.streak >= 4 && stamina < 2520 {
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
        let soul_due = if premium && p.streak >= 7 {
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
                        assert_eq!(
                            gains.0,
                            u32::from(inside && premium && streak >= 4 && stamina < 2520)
                        );
                        assert_eq!(
                            gains.1,
                            if inside && premium && streak >= 7 {
                                interval / 120
                            } else {
                                0
                            }
                        );
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
}
