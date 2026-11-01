//! Qualified autoattack observations joining the existing final player-vitals owner.
//! This carries no durable authority and cannot be constructed from a batch Boolean.

use super::PlayerSpellState;
use crate::foundation::{ChannelRuntimeV1, ExactActorRef, GameSessionId};
use crate::player_lethal::PlayerHitKind;

#[derive(Debug, Clone, Copy)]
pub struct WheelAttackActor<'a> {
    pub actor: ExactActorRef,
    pub session: GameSessionId,
    pub state: &'a PlayerSpellState,
}

#[derive(Debug, Clone, Copy)]
pub struct WheelAttackComponents {
    pub primary: (u32, PlayerHitKind),
    pub secondary: (u32, PlayerHitKind),
}

#[derive(Debug, Clone)]
pub struct PreparedWheelAttackHit {
    attacker: (ExactActorRef, GameSessionId, PlayerSpellState),
    target: (ExactActorRef, GameSessionId, PlayerSpellState),
    shielding: u32,
    now_us: u64,
    magnitude: u32,
    components: (u32, u32),
}

impl PreparedWheelAttackHit {
    /// Both inputs are actual ChannelSpellStates reads under the held runtime borrow.
    /// Existing attack owners have already resolved defence and armor. Their equipment
    /// owner is called after baseline mitigation and before gem resistance/Avatar, matching
    /// Crystal Creature::blockHit→Player::blockHit→attacker Wheel. The callback retains
    /// the existing equipment formula and its qualified facts.
    pub fn prepare(
        runtime: &ChannelRuntimeV1,
        attacker: WheelAttackActor<'_>,
        target: WheelAttackActor<'_>,
        parts: WheelAttackComponents,
        perfect_damage: i64,
        now_us: u64,
        after_equipment: &mut dyn FnMut(u32, PlayerHitKind) -> Option<u32>,
    ) -> Option<Self> {
        if attacker.actor == target.actor || perfect_damage < 0 {
            return None;
        }
        if runtime
            .player_control_facts(attacker.actor, attacker.session)
            .ok()?
            .control_loss
            .is_some()
        {
            return None;
        }
        // Losing client control does not remove a target from the physical combat owner.
        // Preserve the existing PvP target rule; only the initiating attacker needs control.
        runtime
            .player_control_facts(target.actor, target.session)
            .ok()?;
        let shielding = target
            .state
            .wheel_battle_instinct(runtime, target.actor, now_us)?
            .shielding;
        let mut block = |(magnitude, kind)| {
            let magnitude = target
                .state
                .after_player_baseline(magnitude, kind, shielding, now_us)?;
            let magnitude = after_equipment(magnitude, kind)?;
            target.state.after_wheel_block(magnitude, kind, now_us)
        };
        let primary = block(parts.primary)?;
        let secondary = block(parts.secondary)?;
        let adjacent = super::wheel_adjacent_target(runtime, attacker.actor, target.actor)?;
        let (primary, secondary) = attacker.state.finish_wheel_attack_components(
            i64::from(primary),
            i64::from(secondary),
            u64::from(target.state.health),
            u64::from(target.state.character_facts().max_health),
            now_us,
            adjacent,
        )?;
        let primary = u32::try_from(primary.checked_add(perfect_damage)?).ok()?;
        let secondary = u32::try_from(secondary).ok()?;
        let magnitude = primary.checked_add(secondary)?;
        Some(Self {
            attacker: (attacker.actor, attacker.session, attacker.state.clone()),
            target: (target.actor, target.session, target.state.clone()),
            shielding,
            now_us,
            magnitude,
            components: (primary, secondary),
        })
    }

    pub const fn magnitude(&self) -> u32 {
        self.magnitude
    }

    /// The validated single-hit batch retains both components through target reductions.
    /// Its staging owner may already have reserved a successor revision, but no other
    /// player fact may differ from the observation used to compute this exact hit.
    pub fn stage_target_hit(
        &self,
        state: &PlayerSpellState,
        now_us: u64,
    ) -> Result<
        (
            PlayerSpellState,
            crate::ability::creature_bite::FlooredDamage,
        ),
        oteryn_protocol_oteryn::actor_spell::SpellCastDisposition,
    > {
        let mut observed = state.clone();
        observed.revision = self.target.2.revision;
        if now_us != self.now_us || observed != self.target.2 {
            return Err(oteryn_protocol_oteryn::actor_spell::SpellCastDisposition::Rejected);
        }
        crate::spell::actor_conditions::stage_preprocessed_player_components(
            state,
            self.components.0,
            self.components.1,
            now_us,
        )
    }

    /// Validate the exact single hit and both current owner states before final HP staging.
    pub fn qualify_batch<'a>(
        &self,
        runtime: &ChannelRuntimeV1,
        batch: &crate::spell::combat_batch::OwnerCombatBatch,
        player: &dyn Fn(ExactActorRef, GameSessionId) -> Option<&'a PlayerSpellState>,
    ) -> bool {
        use crate::spell::combat_batch::OwnerCombatChange;
        if batch.caster != self.attacker.0
            || batch.command.game_session_id() != self.attacker.1
            || batch.now_us.map(|time| time.get()) != Some(self.now_us)
            || batch.now_ms != self.now_us / 1_000
            || batch.effects.len() != 1
            || player(self.attacker.0, self.attacker.1) != Some(&self.attacker.2)
            || player(self.target.0, self.target.1) != Some(&self.target.2)
            || self
                .target
                .2
                .wheel_battle_instinct(runtime, self.target.0, self.now_us)
                .map(|bonus| bonus.shielding)
                != Some(self.shielding)
        {
            return false;
        }
        let effect = &batch.effects[0];
        effect.target == self.target.0
            && matches!(effect.change, OwnerCombatChange::Damage { magnitude, .. } if magnitude == i64::from(self.magnitude))
    }
}
