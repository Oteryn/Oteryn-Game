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
                if !enabled {
                    spell_combat.tibiadrome_modifiers = None;
                }
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
    /// ARENA1 A4 / ARENA1-RL-20: round(base * 1.039^wave) for HP and damage.
    /// The shared damage curve is the owner-accepted engineering default.
    pub fn scale_tibiadrome_opponent(
        &mut self,
        actor: ExactActorRef,
        level: u32,
    ) -> Result<(), CarrierError> {
        self.owner_fence()?;
        if level > 300 {
            return Err(CarrierError::DamageOverflow);
        }
        let snapshot = self.companion_snapshot(actor)?;
        let base = self.companion_policy(&snapshot.state.policy.definition_key)?;
        if let Some(role) =
            super::tibiadrome_modifiers::DromeCreatureRole::from_key(&base.definition_key)
            && u64::try_from(base.maximum_health).ok() != Some(role.base_health())
        {
            return Err(CarrierError::InvalidCreatureHealth);
        }
        let maximum = (base.maximum_health as f64 * 1.039_f64.powi(level as i32)).round();
        if !maximum.is_finite() || maximum < 1.0 || maximum >= i64::MAX as f64 {
            return Err(CarrierError::DamageOverflow);
        }
        let maximum = maximum as i64;
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
        let mut policy = (*companion.policy).clone();
        policy.maximum_health = maximum;
        companion.policy = std::sync::Arc::new(policy);
        *health = maximum;
        spell_combat.maximum_health = maximum;
        spell_combat.tibiadrome = true;
        spell_combat.tibiadrome_wave = level;
        Ok(())
    }
    pub(super) fn tibiadrome_damage(
        &self,
        actor: ExactActorRef,
        raw: u64,
    ) -> Result<Option<u64>, CarrierError> {
        let index = self.carrier.validate_ref(&self.continuity, actor.0)?;
        let Slot::CreatureOccupied {
            spell_combat,
            health,
            ..
        } = &self.carrier.slots[index]
        else {
            return Ok(None);
        };
        if !spell_combat.tibiadrome {
            return Ok(None);
        }
        let wave = spell_combat.tibiadrome_wave
            + if self.drome_escalated(actor, *health, spell_combat.maximum_health) {
                5
            } else {
                0
            };
        let aura = self.drome_aura_percent(actor);
        if wave == 0 && aura == 0 {
            return Ok(Some(raw));
        }
        let scaled =
            (raw as f64 * 1.039_f64.powi(wave as i32) * f64::from(100 + aura) / 100.0).round();
        if !scaled.is_finite() || scaled >= u64::MAX as f64 {
            return Err(CarrierError::DamageOverflow);
        }
        Ok(Some(scaled as u64))
    }
}

impl ChannelRuntimeV1 {
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
