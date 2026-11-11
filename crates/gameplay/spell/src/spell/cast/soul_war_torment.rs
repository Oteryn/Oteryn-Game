//! Source counter only; shared native HP and presentation owners remain authoritative.
use super::PlayerSpellState;
impl PlayerSpellState {
    /// Lua changes counter/icon independently of addHealth's result. A visible
    /// HP0 Player keeps HP0; Game rejects that separate health call.
    pub fn stage_soul_war_torment_counter_on_dead_player(
        &self,
        value: u8,
        visible: bool,
        occurrence: ([u8; 16], u64),
    ) -> Option<Self> {
        if self.health != 0
            || value > 36
            || self
                .soul_war_torment_occurrence
                .is_some_and(|old| old.0 == occurrence.0 && old.1 >= occurrence.1)
        {
            return None;
        }
        let mut next = self.clone();
        next.soul_war_torment = value;
        next.soul_war_torment_icon_visible = visible;
        next.soul_war_torment_occurrence = Some(occurrence);
        next.revision = self.revision.checked_add(1)?;
        Some(next)
    }
    /// One canonical revision carries the source counter/icon, actual Lua HP
    /// successor and exact occurrence. No detached HP or counter publication.
    pub fn stage_soul_war_torment_pulse(
        &self,
        value: u8,
        visible: bool,
        occurrence: ([u8; 16], u64),
        damage: u32,
        now: u64,
    ) -> Option<(Self, crate::ability::creature_bite::FlooredDamage)> {
        if value > 36
            || self
                .soul_war_torment_occurrence
                .is_some_and(|old| old.0 == occurrence.0 && old.1 >= occurrence.1)
        {
            return None;
        }
        let (mut next, hit) =
            crate::spell::actor_conditions::stage_soul_war_lua_health_loss(self, damage, now)
                .ok()?;
        next.soul_war_torment = value;
        next.soul_war_torment_icon_visible = visible;
        next.soul_war_torment_occurrence = Some(occurrence);
        next.revision = self.revision.checked_add(1)?;
        Some((next, hit))
    }
    pub const fn soul_war_torment_count(&self) -> u8 {
        self.soul_war_torment
    }
    pub fn soul_war_torment_icon_count(&self) -> Option<u16> {
        self.soul_war_torment_icon_visible
            .then_some(u16::from(self.soul_war_torment))
    }
    /// Crystal zone.cpp130-137 removes this named icon without resetting KV.
    pub fn stage_soul_war_torment_icon_removed(&self) -> Option<Self> {
        let mut next = self.clone();
        if next.soul_war_torment_icon_visible {
            next.soul_war_torment_icon_visible = false;
            next.revision = next.revision.checked_add(1)?;
        }
        Some(next)
    }
    pub fn stage_soul_war_torment_counter(&self, value: u8) -> Option<Self> {
        if value > 36 {
            return None;
        }
        let mut next = self.clone();
        let visible = value != 0;
        if next.soul_war_torment != value || next.soul_war_torment_icon_visible != visible {
            next.soul_war_torment = value;
            next.soul_war_torment_icon_visible = visible;
            next.revision = next.revision.checked_add(1)?;
        }
        Some(next)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::spell::{Vocation, cast::CharacterCastFacts};
    #[test]
    fn srx3_torment_direct_lua_death_health_keeps_spell_shield_and_ring_two_to_one()
    -> Result<(), &'static str> {
        use crate::spell::actor_conditions::apply_combat_change;
        use crate::spell::combat_batch::{AvatarState, ManaShieldState, OwnerCombatChange};
        let facts = CharacterCastFacts {
            vocation: Vocation::Druid,
            level: 250,
            magic_level: 0,
            max_health: 10000,
            max_mana: 2000,
            max_soul: 100,
        };
        let mut base = PlayerSpellState::new(facts, 0, 0)
            .ok_or("source player")?
            .stage_soul_war_torment_counter(24)
            .ok_or("source counter24")?;
        apply_combat_change(
            &mut base,
            &OwnerCombatChange::Avatar(AvatarState {
                expires_ms: 15000,
                outfit_look_type: 1590,
                incoming_reduction_percent: 80,
                critical_chance_percent: 100,
                critical_extra_percentage_points: 15,
            }),
            0,
        )
        .map_err(|_| "canonical avatar")?;
        let mut shield = base.clone();
        apply_combat_change(
            &mut shield,
            &OwnerCombatChange::ManaShield(ManaShieldState {
                capacity: 1000,
                expires_ms: 2000,
            }),
            0,
        )
        .map_err(|_| "canonical spell shield")?;
        let before = shield.clone();
        let (next, hit) = shield
            .stage_soul_war_torment_pulse(25, true, ([7; 16], 1), 1400, 1_000_000)
            .ok_or("combined source shield pulse")?;
        assert_eq!((next.vitals().health, next.vitals().mana), (9600, 1000));
        assert_eq!(hit.health_after, 9600);
        assert_eq!(
            (
                next.soul_war_torment_count(),
                next.soul_war_torment_icon_count()
            ),
            (25, Some(25))
        );
        assert_eq!(next.revision(), before.revision() + 1);
        assert_eq!(
            shield, before,
            "staging must not publish the shield/counter separately"
        );
        assert!(
            next.stage_soul_war_torment_pulse(26, true, ([7; 16], 1), 1400, 1_000_000)
                .is_none()
        );
        let mut ring = base;
        ring.install_equipped_mana_shield(true);
        let (next, hit) = ring
            .stage_soul_war_torment_pulse(25, true, ([7; 16], 1), 1400, 1_000_000)
            .ok_or("combined source ring pulse")?;
        assert_eq!(
            (next.vitals().health, next.vitals().mana),
            (9600, 0),
            "2000 ring mana absorbs1000 source damage, then400 reaches HP; Avatar must not reduce Lua loss"
        );
        assert_eq!(hit.health_after, 9600);
        assert_eq!(next.soul_war_torment_count(), 25);
        Ok(())
    }
    #[test]
    fn srx3_torment_player_owned_counter_icon_survives_scope_clone_and_login_resets()
    -> Result<(), &'static str> {
        let facts = CharacterCastFacts {
            vocation: Vocation::Druid,
            level: 250,
            magic_level: 0,
            max_health: 1000,
            max_mana: 1000,
            max_soul: 100,
        };
        let state = PlayerSpellState::new(facts, 0, 0).ok_or("source player")?;
        assert_eq!(
            (
                state.soul_war_torment_count(),
                state.soul_war_torment_icon_count()
            ),
            (0, None)
        );
        let next = state
            .stage_soul_war_torment_counter(24)
            .ok_or("source torment successor")?;
        assert_eq!(
            (
                next.soul_war_torment_count(),
                next.soul_war_torment_icon_count()
            ),
            (24, Some(24))
        );
        assert_eq!(next.clone().soul_war_torment_icon_count(), Some(24));
        assert_eq!(
            next.vitals(),
            state.vitals(),
            "counter staging must not duplicate HP mutation"
        );
        assert!(state.stage_soul_war_torment_counter(37).is_none());
        let login = PlayerSpellState::new(facts, 0, 0).ok_or("login player")?;
        assert_eq!(login.soul_war_torment_icon_count(), None);
        assert_eq!(
            next.stage_soul_war_torment_counter(0)
                .ok_or("reset")?
                .soul_war_torment_icon_count(),
            None
        );
        Ok(())
    }
    #[test]
    fn srx3_torment_source_zone_kick_hides_icon_and_preserves_counter() -> Result<(), &'static str>
    {
        let facts = CharacterCastFacts {
            vocation: Vocation::Druid,
            level: 250,
            magic_level: 0,
            max_health: 1000,
            max_mana: 1000,
            max_soul: 100,
        };
        let state = PlayerSpellState::new(facts, 0, 0)
            .ok_or("source player")?
            .stage_soul_war_torment_counter(30)
            .ok_or("source torment")?;
        let hidden = state
            .stage_soul_war_torment_icon_removed()
            .ok_or("source zone kick")?;
        assert_eq!(
            hidden.soul_war_torment_count(),
            30,
            "Zone removePlayers does not reset the source counter"
        );
        assert_eq!(
            hidden.soul_war_torment_icon_count(),
            None,
            "source named-icon removal must survive a complete entity projection"
        );
        assert_eq!(hidden.vitals(), state.vitals());
        assert!(hidden.revision() > state.revision());
        let clamped = hidden
            .stage_soul_war_torment_counter(30)
            .ok_or("next source pulse at maximum")?;
        assert_eq!(
            clamped.soul_war_torment_icon_count(),
            Some(30),
            "a real source setIcon after clamping reinstates the icon even when the counter is unchanged"
        );
        Ok(())
    }
}
