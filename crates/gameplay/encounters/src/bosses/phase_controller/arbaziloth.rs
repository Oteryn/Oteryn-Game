//! Shield progression from Crystal 201189f8, no_rest_for_the_wicked/arbaziloth_fight.lua.
const THRESHOLDS: [i64; 4] = [80, 60, 40, 20];
const LEAVE_DELAY_MS: u64 = 3_000;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ShieldSequence {
    cleared: usize,
    shield: Shield,
    departure_due: Option<u64>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum Shield {
    #[default]
    Down,
    Raised,
    Cracked,
}

impl ShieldSequence {
    /// Apply this to positive incoming damage after replay qualification.
    pub fn limit_damage(&mut self, health: i64, maximum: i64, damage: i64) -> i64 {
        if self.shield != Shield::Down {
            return 0;
        }
        let Some(&percent) = THRESHOLDS.get(self.cleared) else {
            return damage;
        };
        let threshold = maximum / 100 * percent + maximum % 100 * percent / 100;
        let allowed = health.saturating_sub(threshold).max(0);
        if damage >= allowed {
            self.shield = Shield::Raised;
        }
        damage.min(allowed)
    }

    /// The map owner calls this when the exact boss steps onto the active fire.
    /// A true return consumes that fire.
    pub fn step_on_fire(&mut self) -> bool {
        if self.shield != Shield::Raised {
            return false;
        }
        self.shield = Shield::Cracked;
        true
    }

    /// The item owner supplies its roll in 1..=100 from the qualified rod use.
    pub fn use_rod(&mut self, roll: u8, now_ms: u64) -> bool {
        if self.shield != Shield::Cracked || !(51..=100).contains(&roll) {
            return false;
        }
        self.shield = Shield::Down;
        self.cleared += 1;
        if self.cleared == THRESHOLDS.len() {
            self.departure_due = now_ms.checked_add(LEAVE_DELAY_MS);
        }
        true
    }

    pub fn forgemaster_departure_due(&self, now_ms: u64) -> bool {
        self.departure_due.is_some_and(|due| now_ms >= due)
    }
}

use super::{NativePhaseOwner, Plan, Role};
use oteryn_gameplay_entities::{
    CarrierError, ChannelRuntimeV1, ExactActorRef, GameSessionId, MovementLocalPosition,
};

const BOSS: &str = "oteryn:creature.arbaziloth";
const FORGEMASTER: &str = "oteryn:creature.the_forgemaster";
const ENRAGED: &str = "oteryn:creature.enraged_arbaziloth";
const ENRAGED_ROLES: &[Role] = &[Role {
    creature: ENRAGED,
    after: 0,
}];

/// The replacement remains bound to the exact actors admitted for this attempt.
#[derive(Debug)]
pub struct ArbazilothEncounter {
    phase: NativePhaseOwner,
    boss: ExactActorRef,
    forgemaster: ExactActorRef,
    enraged: Option<ExactActorRef>,
    shields: ShieldSequence,
}

impl ArbazilothEncounter {
    pub fn bind(
        runtime: &ChannelRuntimeV1,
        players: &[(ExactActorRef, GameSessionId)],
        boss: ExactActorRef,
        forgemaster: ExactActorRef,
    ) -> Result<Self, CarrierError> {
        if !runtime.matches_live_creature_identity(boss, BOSS.as_bytes())
            || !runtime.matches_live_creature_identity(forgemaster, FORGEMASTER.as_bytes())
        {
            return Err(CarrierError::CreatureTargetMismatch);
        }
        runtime.companion_policy(ENRAGED)?;
        Ok(Self {
            phase: NativePhaseOwner::bind(
                runtime,
                Plan {
                    roles: ENRAGED_ROLES,
                    terminal: 1,
                },
                players,
            )?,
            boss,
            forgemaster,
            enraged: None,
            shields: ShieldSequence::default(),
        })
    }

    pub fn shields_for(&mut self, actor: ExactActorRef) -> Option<&mut ShieldSequence> {
        (actor == self.boss && self.enraged.is_none()).then_some(&mut self.shields)
    }

    /// Crystal removes the Forgemaster and normal boss when the delayed event
    /// fires, then puts the enraged form at the original boss's current tile.
    pub fn advance(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        now_ms: u64,
        map_admits: impl Fn(&ChannelRuntimeV1, MovementLocalPosition) -> bool,
    ) -> Result<Option<ExactActorRef>, CarrierError> {
        if self.enraged.is_some()
            || !self.shields.forgemaster_departure_due(now_ms)
            || !runtime.matches_live_creature_identity(self.boss, BOSS.as_bytes())
            || !runtime.matches_live_creature_identity(self.forgemaster, FORGEMASTER.as_bytes())
        {
            return Ok(None);
        }
        let position = runtime.read_actor_position(self.boss)?.position();
        let enraged = self.phase.spawn_role(
            runtime,
            0,
            position,
            &[self.boss, self.forgemaster],
            None,
            map_admits,
        )?;
        self.enraged = Some(enraged);
        Ok(Some(enraged))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arbaziloth_four_distinct_shields_qualify_timed_departure() {
        let mut sequence = ShieldSequence::default();
        for health in [100, 80, 60, 40] {
            assert_eq!(sequence.limit_damage(health, 100, 100), 20);
            assert_eq!(sequence.limit_damage(health - 20, 100, 100), 0);
            assert!(!sequence.use_rod(51, 100));
            assert!(sequence.step_on_fire());
            assert!(!sequence.step_on_fire());
            assert!(!sequence.use_rod(50, 100));
            assert!(sequence.use_rod(51, 100));
        }
        assert_eq!(sequence.limit_damage(20, 100, 20), 20);
        assert!(!sequence.forgemaster_departure_due(3_099));
        assert!(sequence.forgemaster_departure_due(3_100));
    }
}
