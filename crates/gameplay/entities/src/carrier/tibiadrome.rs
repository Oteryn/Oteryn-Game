//! Drome membership lives on the exact actor slot, so reuse cannot inherit an exemption.
use super::*;
impl ChannelRuntimeV1 {
    pub fn set_tibiadrome(
        &mut self,
        actor: ExactActorRef,
        enabled: bool,
    ) -> Result<(), CarrierError> {
        self.owner_fence()?;
        let origin = enabled
            .then(|| self.read_actor_position(actor).map(|p| p.position()))
            .transpose()?;
        let index = self.carrier.validate_ref(&self.continuity, actor.0)?;
        match &mut self.carrier.slots[index] {
            Slot::Occupied { spell_combat, .. } | Slot::CreatureOccupied { spell_combat, .. } => {
                spell_combat.tibiadrome = enabled;
                spell_combat.tibiadrome_origin = origin;
                Ok(())
            }
            _ => Err(CarrierError::NotCreature),
        }
    }
    pub fn is_tibiadrome(&self, actor: ExactActorRef) -> bool {
        let Ok(index) = self.carrier.validate_ref(&self.continuity, actor.0) else {
            return false;
        };
        let combat = match &self.carrier.slots[index] {
            Slot::Occupied { spell_combat, .. } => return spell_combat.tibiadrome,
            Slot::CreatureOccupied {
                spell_combat,
                companion,
                ..
            } => {
                if spell_combat.tibiadrome {
                    return true;
                }
                companion.as_ref().and_then(|c| c.master)
            }
            _ => None,
        };
        let Some(master) = combat else {
            return false;
        };
        let Ok(index) = self.carrier.validate_ref(&self.continuity, master.actor.0) else {
            return false;
        };
        matches!(&self.carrier.slots[index],Slot::Occupied{spell_combat,..} if spell_combat.tibiadrome)
    }
}

impl ChannelRuntimeV1 {
    /// oteryn-choice: +10% native health/damage per Drome level; the current
    /// manual combat.md:297 and pinned donors have no numeric Drome scaling.
    pub fn scale_tibiadrome_opponent(
        &mut self,
        actor: ExactActorRef,
        level: u32,
    ) -> Result<(), CarrierError> {
        self.owner_fence()?;
        let index = self.carrier.validate_ref(&self.continuity, actor.0)?;
        let Slot::CreatureOccupied {
            health,
            spell_combat,
            companion: Some(companion),
            ..
        } = &mut self.carrier.slots[index]
        else {
            return Err(CarrierError::NotCreature);
        };
        let scale = 100u64.saturating_add(u64::from(level).saturating_mul(10));
        let maximum =
            i64::try_from(i128::from(companion.policy.maximum_health) * i128::from(scale) / 100)
                .map_err(|_| CarrierError::DamageOverflow)?;
        let mut policy = (*companion.policy).clone();
        policy.maximum_health = maximum;
        companion.policy = std::sync::Arc::new(policy);
        *health = maximum;
        spell_combat.maximum_health = maximum;
        spell_combat.tibiadrome = true;
        spell_combat.tibiadrome_scale = scale;
        Ok(())
    }
    pub(super) fn tibiadrome_damage(
        &self,
        actor: ExactActorRef,
        raw: u64,
    ) -> Result<Option<u64>, CarrierError> {
        let index = self.carrier.validate_ref(&self.continuity, actor.0)?;
        let Slot::CreatureOccupied {
            health,
            spell_combat,
            ..
        } = &self.carrier.slots[index]
        else {
            return Ok(None);
        };
        if !spell_combat.tibiadrome {
            return Ok(None);
        }
        let extra_levels = if spell_combat.tibiadrome_escalation
            && *health > 0
            && i128::from(*health) * 4 < i128::from(spell_combat.maximum_health)
        {
            50
        } else {
            0
        };
        let scale = spell_combat.tibiadrome_scale.saturating_add(extra_levels);
        u64::try_from(u128::from(raw) * u128::from(scale) / 100)
            .map(Some)
            .map_err(|_| CarrierError::DamageOverflow)
    }
}

impl ChannelRuntimeV1 {
    /// Manual combat.md:271: the last hitter is the exact retained lethal
    /// receipt's session, not the top-damage contributor or a client target.
    pub fn tibiadrome_last_hitter(&self, actor: ExactActorRef) -> Option<GameSessionId> {
        self.owner_fence().ok()?;
        let index = self.carrier.validate_ref(&self.continuity, actor.0).ok()?;
        let Slot::CreatureOccupied {
            health,
            committed,
            spell_combat,
            ..
        } = &self.carrier.slots[index]
        else {
            return None;
        };
        if !spell_combat.tibiadrome || *health != 0 {
            return None;
        }
        committed
            .entries
            .iter()
            .find(|record| {
                record.result.applied
                    && record.result.health_before > 0
                    && record.result.health_after == 0
            })
            .and_then(|record| record.origin)
            .map(|origin| origin.2)
    }

    /// Manual combat.md:273: below 25% HP, use the existing working damage
    /// curve at five additional wave levels. Never heal or re-scale current HP.
    pub fn set_tibiadrome_escalation(
        &mut self,
        actor: ExactActorRef,
        enabled: bool,
    ) -> Result<(), CarrierError> {
        self.owner_fence()?;
        let index = self.carrier.validate_ref(&self.continuity, actor.0)?;
        let Slot::CreatureOccupied { spell_combat, .. } = &mut self.carrier.slots[index] else {
            return Err(CarrierError::NotCreature);
        };
        if !spell_combat.tibiadrome {
            return Err(CarrierError::NotCreature);
        }
        spell_combat.tibiadrome_escalation = enabled;
        Ok(())
    }

    /// Manual combat.md:270: a current native Drome opponent teleports only
    /// after its successful melee turn and the caller's same-lock tile proof.
    pub fn relocate_tibiadrome_opponent(
        &mut self,
        actor: ExactActorRef,
        expected: MovementPositionSnapshot,
        at: MovementLocalPosition,
    ) -> Result<(), CarrierError> {
        self.owner_fence()?;
        self.assert_actor_spell_unreserved(actor)?;
        let snapshot = self.companion_snapshot_including_dead(actor)?;
        if !self.is_tibiadrome(actor) || snapshot.health <= 0 || expected.0.actor_ref != actor.0 {
            return Err(CarrierError::PositionSnapshotMismatch);
        }
        self.carrier.compare_commit_position(
            &self.continuity,
            expected.0,
            expected.0.version.context,
            LocalPosition {
                x: at.x,
                y: at.y,
                floor: at.floor,
            },
        )?;
        Ok(())
    }

    /// Manual combat.md:280: the server returns a still-present participant
    /// even after control loss. Exact actor/session/current scope and reservation
    /// checks remain mandatory; this grants no playable control or reconnect.
    pub fn return_tibiadrome_player(
        &mut self,
        actor: ExactActorRef,
        session: GameSessionId,
        expected: MovementPositionSnapshot,
        at: MovementLocalPosition,
    ) -> Result<(), CarrierError> {
        self.owner_fence()?;
        self.player_control_facts(actor, session)?;
        self.assert_actor_spell_unreserved(actor)?;
        if !self.is_tibiadrome(actor) || expected.0.actor_ref != actor.0 {
            return Err(CarrierError::PositionSnapshotMismatch);
        }
        self.carrier.compare_commit_position(
            &self.continuity,
            expected.0,
            expected.0.version.context,
            LocalPosition {
                x: at.x,
                y: at.y,
                floor: at.floor,
            },
        )?;
        Ok(())
    }
}
