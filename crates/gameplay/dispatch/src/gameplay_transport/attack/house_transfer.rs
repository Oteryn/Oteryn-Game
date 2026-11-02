//! Transfer only the existing actor's combat observations. This grants no House
//! attack authority and never moves a kill occurrence or its captured principal.
use super::*;

impl ChannelAttackStates {
    pub(in crate::gameplay_transport) fn house_transfer_ready(
        &self,
        destination: GameSessionId,
    ) -> bool {
        !self
            .entries
            .iter()
            .any(|entry| entry.session == destination)
    }

    /// Called inside the shared spell/attack publication after its native
    /// successor proof. Preserve modes, defence budgets and owner deadlines.
    pub(in crate::gameplay_transport) fn move_house(
        &mut self,
        actor: ExactActorRef,
        source: GameSessionId,
        destination: GameSessionId,
    ) {
        for entry in &mut self.entries {
            if entry.actor == actor && entry.session == source {
                entry.session = destination;
                // A held target names the previous scope. The native clear
                // operation preserves its in-fight deadline and stops chase.
                entry.clear_target();
                entry.weapon = None;
                entry.weapon_epoch = entry.weapon_epoch.wrapping_add(1);
            }
        }
        // step_clocks are already keyed by the same exact actor, not session.
        // Pending kill settlements retain every original CommandRef/principal.
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn house_handoff_keeps_combat_modes_and_actor_deadlines() {
        let (runtime, actor, source) =
            crate::gameplay_transport::actor_spell::tests::runtime_with_player(144);
        let destination =
            GameSessionId::decode(&[0, 0, 0, 0, 0, 1, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 197])
                .expect("destination");
        let mut owner = ChannelAttackStates::default();
        let mut modes = FightModes::DEFAULT;
        modes.fight_mode = WireFightMode::Defensive;
        assert!(owner.set_modes(&runtime, actor, source, modes));
        let now = SemanticTimeMicros::from_micros(8_000_000);
        owner.record_hit_taken(&runtime, actor, source, now);
        let fight_until = owner
            .in_fight_until(actor, source, now)
            .expect("native hit deadline");
        let holder = Arc::new(());
        let instant = Instant::now();
        let step_until = instant + Duration::from_millis(550);
        assert_eq!(owner.begin_step(actor, &holder, instant), None);
        owner.record_step_deadline(actor, &holder, instant, Some(step_until));
        assert!(owner.house_transfer_ready(destination));
        owner.move_house(actor, source, destination);
        assert!(owner.index(actor, source).is_none());
        assert_eq!(owner.combat_state(actor, destination, now).modes, modes);
        assert_eq!(owner.combat_state(actor, destination, now).target, None);
        assert_eq!(
            owner.in_fight_until(actor, destination, now),
            Some(fight_until)
        );
        assert_eq!(owner.step_deadline(actor, instant), Some(step_until));
        assert!(!owner.house_transfer_ready(destination));
    }
}
