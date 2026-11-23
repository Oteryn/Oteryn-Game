//! Actor-local protection for the first Mimar Haffar encounter.
use super::*;

pub const BOSS: &str = "oteryn:creature.mimar_haffar";
const FIGHT_LIMIT_MS: u64 = 900_000;
const CRYSTAL: &str = "oteryn:item.tibia.i54267";
const CHARGE_PER_CRYSTAL: u8 = 10;
const FULL_CHARGE: u8 = 100;
// TibiaWiki, Make Believe Quest/Spoiler, revision 2026-10-27, Mimar phase 2.
const VULNERABILITY_MS: u64 = 45_000;

#[derive(Clone, Copy, Debug)]
pub enum Cannon {
    Southwest,
    Southeast,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct MimarShield {
    charges: [u8; 2],
    start_ms: u64,
    deadline_ms: u64,
    vulnerability: Option<(u64, u64)>,
}

impl MimarShield {
    pub(super) fn vulnerable(&self, now_ms: Option<u64>) -> bool {
        self.vulnerability
            .zip(now_ms)
            .is_some_and(|((start, end), now)| (start..end).contains(&now))
    }

    pub(super) fn movement_locked(&self) -> bool {
        self.vulnerability.is_none()
    }
}

impl ChannelRuntimeV1 {
    pub fn bind_mimar_shield(
        &mut self,
        boss: ExactActorRef,
        start_ms: u64,
    ) -> Result<(), CarrierError> {
        let shield = MimarShield {
            charges: [0; 2],
            start_ms,
            deadline_ms: start_ms
                .checked_add(FIGHT_LIMIT_MS)
                .ok_or(CarrierError::CapacityArithmeticOverflow)?,
            vulnerability: None,
        };
        let slot = self.mimar_shield_slot(boss)?;
        if slot.is_some() {
            return Err(CarrierError::PlanConflict);
        }
        *slot = Some(shield);
        Ok(())
    }

    /// The item owner must fence one owned, unexpired crystal and return true
    /// only when this invocation consumes it. A refusal or replay returns false.
    /// Once consumption succeeds, publishing the new charge is infallible.
    pub fn load_mimar_cannon(
        &mut self,
        boss: ExactActorRef,
        cannon: Cannon,
        now_ms: u64,
        consume: impl FnOnce(&str) -> bool,
    ) -> Result<bool, CarrierError> {
        let shield = self
            .mimar_shield_slot(boss)?
            .as_mut()
            .ok_or(CarrierError::PlanConflict)?;
        let index = match cannon {
            Cannon::Southwest => 0,
            Cannon::Southeast => 1,
        };
        if shield.vulnerability.is_some()
            || !(shield.start_ms..shield.deadline_ms).contains(&now_ms)
            || shield.charges[index] == FULL_CHARGE
        {
            return Ok(false);
        }
        if !consume(CRYSTAL) {
            return Ok(false);
        }
        shield.charges[index] += CHARGE_PER_CRYSTAL;
        if shield.charges == [FULL_CHARGE; 2] {
            let end = now_ms
                .saturating_add(VULNERABILITY_MS)
                .min(shield.deadline_ms);
            shield.vulnerability = Some((now_ms, end));
        }
        Ok(true)
    }

    pub fn advance_mimar_shield(
        &mut self,
        boss: ExactActorRef,
        center: MovementLocalPosition,
        now_ms: u64,
        map_admits: impl FnOnce(&Self, MovementLocalPosition) -> bool,
    ) -> Result<(), CarrierError> {
        let shield = self
            .mimar_shield_slot(boss)?
            .as_mut()
            .ok_or(CarrierError::PlanConflict)?;
        if now_ms >= shield.deadline_ms {
            return self.remove_encounter_actor(boss);
        }
        if shield.charges != [FULL_CHARGE; 2] || shield.vulnerable(Some(now_ms)) {
            return Ok(());
        }
        if shield
            .vulnerability
            .is_some_and(|(start, _)| now_ms < start)
        {
            return Ok(());
        }
        // An expired window stays protected while a blocked reposition retries.
        shield.vulnerability = None;
        let position = self.read_actor_position(boss)?;
        if position.position() != center {
            if !map_admits(self, center) {
                return Err(CarrierError::PlanConflict);
            }
            self.borrow_movement_position()
                .commit_creature_relocation(boss, position, center)?;
        }
        self.mimar_shield_slot(boss)?
            .as_mut()
            .ok_or(CarrierError::PlanConflict)?
            .charges = [0; 2];
        Ok(())
    }

    fn mimar_shield_slot(
        &mut self,
        boss: ExactActorRef,
    ) -> Result<&mut Option<MimarShield>, CarrierError> {
        self.owner_fence()?;
        let index = self.carrier.validate_ref(&self.continuity, boss.0)?;
        self.carrier.assert_slot_spell_unreserved(index)?;
        match &mut self.carrier.slots[index] {
            Slot::CreatureOccupied {
                health,
                target_identity,
                spell_combat,
                ..
            } if *health > 0 && target_identity.as_ref() == BOSS.as_bytes() => {
                Ok(&mut spell_combat.mimar_shield)
            }
            _ => Err(CarrierError::CreatureTargetMismatch),
        }
    }
}
