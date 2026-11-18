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

    pub fn needs_fire(self) -> bool {
        self.shield == Shield::Raised
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

use super::*;

impl ChannelRuntimeV1 {
    pub fn bind_arbaziloth_shields(&mut self, actor: ExactActorRef) -> Result<(), CarrierError> {
        self.owner_fence()?;
        let index = self.carrier.validate_ref(&self.continuity, actor.0)?;
        self.carrier.assert_slot_spell_unreserved(index)?;
        let Slot::CreatureOccupied {
            target_identity,
            spell_combat,
            ..
        } = &mut self.carrier.slots[index]
        else {
            return Err(CarrierError::NotCreature);
        };
        if target_identity.as_ref() != b"oteryn:creature.arbaziloth"
            || spell_combat.arbaziloth_shields.is_some()
        {
            return Err(CarrierError::PlanConflict);
        }
        spell_combat.arbaziloth_shields = Some(ShieldSequence::default());
        self.carrier.arbaziloth_victory = None;
        Ok(())
    }

    pub fn open_arbaziloth_doomforge(&mut self, actor: ExactActorRef) -> Result<(), CarrierError> {
        self.owner_fence()?;
        let index = self.carrier.validate_ref(&self.continuity, actor.0)?;
        let Slot::CreatureOccupied {
            health: 0,
            target_identity,
            ..
        } = &self.carrier.slots[index]
        else {
            return Err(CarrierError::CommittedLethalUnavailable);
        };
        if target_identity.as_ref() != b"oteryn:creature.enraged_arbaziloth" {
            return Err(CarrierError::CreatureTargetMismatch);
        }
        let (death, _) = self.borrow_combat_death().projected_death(actor)?;
        self.carrier.arbaziloth_victory = Some(death);
        Ok(())
    }

    pub fn arbaziloth_doomforge_open(&self) -> bool {
        self.owner_fence().is_ok()
            && self.carrier.arbaziloth_victory.is_some_and(|death| {
                let (scope, generation) = self.encounter_owner_scope();
                death.runtime_scope() == scope && death.scope_ownership_generation() == generation
            })
    }

    pub fn arbaziloth_shields(&self, actor: ExactActorRef) -> Option<ShieldSequence> {
        let index = self.carrier.validate_ref(&self.continuity, actor.0).ok()?;
        match &self.carrier.slots[index] {
            Slot::CreatureOccupied {
                health,
                spell_combat,
                ..
            } if *health > 0 => spell_combat.arbaziloth_shields,
            _ => None,
        }
    }

    pub fn arbaziloth_shields_mut(&mut self, actor: ExactActorRef) -> Option<&mut ShieldSequence> {
        self.owner_fence().ok()?;
        let index = self.carrier.validate_ref(&self.continuity, actor.0).ok()?;
        self.carrier.assert_slot_spell_unreserved(index).ok()?;
        match &mut self.carrier.slots[index] {
            Slot::CreatureOccupied {
                health,
                spell_combat,
                ..
            } if *health > 0 => spell_combat.arbaziloth_shields.as_mut(),
            _ => None,
        }
    }
}
