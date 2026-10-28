//! Forge classification lives in the existing spawn and actor owners.
use super::*;
use oteryn_gameplay_domain::domain::forge_creature::{
    FIENDISH_RESPAWN_MICROS, FIENDISH_SIMULTANEOUS_MAX, ForgeCreature, ForgeSpawnCandidate,
    INFLUENCED_SIMULTANEOUS_MAX, plan_fiendish_tick,
};
use sha2::{Digest, Sha256};

// Canary's Game::updateForgeableMonsters maintenance cadence, in owner semantic time.
pub(super) const POPULATION_TICK_MICROS: u64 = 10_000_000;

impl ChannelRuntimeV1 {
    fn forge_roll(&self, actor: ExactActorRef) -> u64 {
        let mut hash = Sha256::new();
        hash.update(b"FORGECREATURE/spawn/v1");
        hash.update(self.content.server_artifact_digest());
        hash.update(actor.placement_identity());
        hash.update(actor.actor_local_generation().to_be_bytes());
        let bytes: [u8; 32] = hash.finalize().into();
        u64::from_be_bytes(bytes[..8].try_into().unwrap_or_default())
    }

    fn apply_forge_classification(
        &mut self,
        actor: ExactActorRef,
        kind: ForgeCreature,
    ) -> Result<(), CarrierError> {
        self.assert_actor_spell_unreserved(actor)?;
        let snapshot = self.companion_snapshot(actor)?;
        if snapshot.state.master.is_some() || snapshot.state.policy.forge_difficulty.is_none() {
            return Err(CarrierError::InvalidCreatureTarget);
        }
        let maximum = i64::try_from(
            (i128::from(snapshot.state.policy.maximum_health) * i128::from(kind.health_percent())
                + 99)
                / 100,
        )
        .map_err(|_| CarrierError::DamageOverflow)?;
        let current = i64::try_from(
            i128::from(snapshot.health) * i128::from(maximum) / i128::from(snapshot.maximum_health),
        )
        .map_err(|_| CarrierError::DamageOverflow)?
        .max(1);
        let index = self.carrier.validate_ref(&self.continuity, actor.0)?;
        let Slot::CreatureOccupied {
            health,
            spell_combat,
            ..
        } = &mut self.carrier.slots[index]
        else {
            return Err(CarrierError::NotCreature);
        };
        *health = current;
        spell_combat.maximum_health = maximum;
        spell_combat.forge = Some(kind);
        Ok(())
    }

    pub(super) fn drive_forge_population(&mut self, now: u64) -> Result<(), CarrierError> {
        // Observe only the three retained incarnations on ordinary owner turns. A dead
        // fiend holds its population slot until its own replacement deadline.
        for index in 0..self.forge_fiends.len() {
            let (actor, due) = self.forge_fiends[index];
            if due.is_none() && self.companion_snapshot(actor).is_err() {
                let due = now.saturating_add(FIENDISH_RESPAWN_MICROS);
                self.forge_fiends[index].1 = Some(due);
                self.forge_next_tick_micros = self.forge_next_tick_micros.min(due);
            }
        }
        if now < self.forge_next_tick_micros {
            return Ok(());
        }
        self.forge_fiends
            .retain(|(_, due)| due.is_none_or(|due| now < due));
        let available =
            usize::from(FIENDISH_SIMULTANEOUS_MAX).saturating_sub(self.forge_fiends.len());
        let mut candidates = Vec::new();
        let mut actors = Vec::new();
        let mut existing = Vec::new();
        // Only native spawn points participate: summons, raid callbacks and encounter children
        // have no ordinary spawn point and cannot become Forge creatures.
        for source in &self.carrier.spawns {
            for (ordinal, cell) in source.cells.iter().enumerate() {
                let Some(actor) = cell.live else { continue };
                let Ok(snapshot) = self.companion_snapshot(actor) else {
                    continue;
                };
                let identity = (u64::from(source.source.0) << 8) | ordinal as u64;
                let eligible = snapshot.state.master.is_none()
                    && snapshot.state.policy.forge_difficulty.is_some()
                    && !matches!(snapshot.forge, Some(ForgeCreature::Influenced(_)));
                candidates.push(ForgeSpawnCandidate {
                    spawn_identity: identity,
                    eligible,
                    alive: true,
                });
                actors.push((identity, actor));
                if snapshot.forge == Some(ForgeCreature::Fiendish) {
                    existing.push(identity);
                }
            }
        }
        let seed = u64::from_be_bytes(
            self.content.server_artifact_digest()[..8]
                .try_into()
                .unwrap_or_default(),
        );
        let plan = plan_fiendish_tick(now, seed, &existing, &candidates);
        let mut selected = 0;
        for identity in &plan.selected {
            if selected < available
                && !existing.contains(identity)
                && let Some((_, actor)) = actors.iter().find(|(id, _)| id == identity)
            {
                self.apply_forge_classification(*actor, ForgeCreature::Fiendish)?;
                self.forge_fiends.push((*actor, None));
                selected += 1;
            }
        }
        // The pinned engines add one influenced creature per maintenance interval until the
        // configured population ceiling, rather than converting a fraction of every spawn set.
        let influenced_count = actors
            .iter()
            .filter(|(_, actor)| {
                self.companion_snapshot(*actor)
                    .is_ok_and(|s| matches!(s.forge, Some(ForgeCreature::Influenced(_))))
            })
            .count();
        let mut influenced: Vec<_> = actors
            .iter()
            .filter(|(_, actor)| {
                self.companion_snapshot(*actor).is_ok_and(|s| {
                    s.state.master.is_none()
                        && s.state.policy.forge_difficulty.is_some()
                        && s.forge.is_none()
                })
            })
            .map(|(_, actor)| (self.forge_roll(*actor), *actor))
            .collect();
        influenced.sort_unstable_by_key(|(roll, _)| *roll);
        for (roll, actor) in influenced.into_iter().take(
            usize::from(INFLUENCED_SIMULTANEOUS_MAX)
                .saturating_sub(influenced_count)
                .min(1),
        ) {
            if let Some(kind) = ForgeCreature::new_influenced(1 + (roll % 5) as u8) {
                self.apply_forge_classification(actor, kind)?;
            }
        }
        let next_scan = now.saturating_add(POPULATION_TICK_MICROS);
        self.forge_next_tick_micros = self
            .forge_fiends
            .iter()
            .filter_map(|(_, due)| *due)
            .min()
            .unwrap_or(next_scan)
            .min(next_scan);
        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::super::runtime_actor_companion::{
        CompiledCreaturePolicies, CompiledCreaturePolicy, CreatureFlags,
    };
    use super::*;

    #[test]
    fn forge_creature_native_spawn_scales_health_retains_marker_and_replaces_incarnation() {
        let mut current = super::super::tests::runtime(16);
        let pin = current.content_pin().clone();
        let policy = CompiledCreaturePolicy {
            definition_key: "oteryn:creature.rat".into(),
            definition_revision: "fixture:1".into(),
            display_name: "fixture Rat".into(),
            maximum_health: 100,
            forge_difficulty: Some(4),
            base_speed: 110,
            outfit_look_type: 21,
            object_look_type: None,
            summonable: false,
            convinceable: false,
            mana_cost: None,
            is_familiar: false,
            condition_immunities: vec![],
            defense: Some(1),
            armor: Some(1),
            mitigation: None,
            resistances: vec![],
            damage_immunities: vec![],
            healing_from_damage: vec![],
            preferred_distance: Some(1),
            reward_boss: Some(false),
            blocked_by_nearby_players: false,
            flags: CreatureFlags {
                attackable: true,
                illusionable: false,
                health_hidden: false,
            },
        };

        current
            .install_companion_policies(
                CompiledCreaturePolicies::from_active_artifact(
                    pin.server_artifact_digest(),
                    vec![policy],
                )
                .unwrap(),
            )
            .unwrap();
        let points = (2..8)
            .map(|x| {
                (
                    (x, 0, 0),
                    60_000,
                    "oteryn:creature.rat".to_owned(),
                    100,
                    false,
                )
            })
            .collect();
        current
            .begin_activation_spawn_sources(vec![(points, 5_000)])
            .unwrap();
        assert_eq!(
            current
                .realize_activation_spawn_window(0)
                .unwrap()
                .remaining,
            0
        );
        for (_, _, actor) in current.spawn_point_creatures() {
            let snapshot = current.companion_snapshot(actor.unwrap()).unwrap();
            assert_eq!(
                (snapshot.health, snapshot.maximum_health, snapshot.forge),
                (100, 100, None)
            );
        }
        current.drive_forge_population(9_999_999).unwrap();
        assert!(
            current
                .spawn_point_creatures()
                .into_iter()
                .all(|(_, _, actor)| {
                    current
                        .companion_snapshot(actor.unwrap())
                        .unwrap()
                        .forge
                        .is_none()
                })
        );
        current.drive_forge_population(10_000_000).unwrap();
        let fiends: Vec<_> = current
            .spawn_point_creatures()
            .into_iter()
            .filter_map(|(_, _, a)| a)
            .filter(|a| {
                current.companion_snapshot(*a).unwrap().forge == Some(ForgeCreature::Fiendish)
            })
            .collect();
        assert_eq!(fiends.len(), 3);
        let influenced: Vec<_> = current
            .spawn_point_creatures()
            .into_iter()
            .filter_map(|(_, _, actor)| actor)
            .filter_map(|actor| current.companion_snapshot(actor).ok())
            .filter(|snapshot| matches!(snapshot.forge, Some(ForgeCreature::Influenced(_))))
            .collect();
        assert_eq!(
            influenced.len(),
            1,
            "one influenced creature per owner interval"
        );
        for snapshot in influenced {
            let kind = snapshot.forge.unwrap();
            assert_eq!(snapshot.maximum_health, i64::from(kind.health_percent()));
            assert_eq!(snapshot.health, snapshot.maximum_health);
        }
        let actor = fiends[0];
        let before = current.companion_snapshot(actor).unwrap();
        assert_eq!((before.health, before.maximum_health), (360, 360));
        let session = super::super::tests::session(42);
        let reservation = current.reserve_fresh_session(session).unwrap();
        let owner = current.commit_fresh_session(reservation).unwrap();
        assert!(
            current
                .prepare_companion_assignment(owner, session, &before)
                .is_err()
        );
        assert!(
            current
                .compare_assign_companion(owner, session, &before)
                .is_err()
        );
        current.set_test_creature_health(actor, 180).unwrap();
        current.drive_forge_population(10_000_001).unwrap();
        let retained = current.companion_snapshot(actor).unwrap();
        assert_eq!(
            (retained.health, retained.maximum_health, retained.forge),
            (180, 360, Some(ForgeCreature::Fiendish))
        );
        let digest = current.content_pin().server_artifact_digest();
        let healed = current
            .commit_source_creature_heal_batch(
                digest,
                &[(actor, "oteryn:creature.rat".into(), 100, 500)],
            )
            .unwrap();
        assert_eq!(healed[0].health_after, 360);
        assert_eq!(
            current
                .read_source_creature_health(actor, "oteryn:creature.rat", 100)
                .unwrap(),
            360
        );
        current.set_test_creature_health(actor, 180).unwrap();
        let registration = current
            .bind_creature_self_heal(
                "oteryn:creature.rat",
                "oteryn:ability.rat_heal",
                100,
                500,
                500,
            )
            .unwrap();
        let mut ledger = super::super::creature_heal::CreatureSelfHealLedger::default();
        assert_eq!(
            current
                .commit_creature_self_heal(&mut ledger, &registration, actor, 1, 500)
                .unwrap()
                .health_after,
            360
        );
        current.set_test_creature_health(actor, 0).unwrap();
        current
            .drive_spawn_respawns(10_001_000, |_, _| true)
            .unwrap();
        current
            .drive_spawn_respawns(71_001_000, |_, _| true)
            .unwrap();
        current
            .drive_spawn_respawns(71_001_000 + spawn::SPAWN_WARNING_MICROS, |_, _| true)
            .unwrap();
        assert!(current.companion_snapshot(actor).is_err());
        let replacement = current
            .spawn_point_creatures()
            .into_iter()
            .filter_map(|(_, _, actor)| actor)
            .filter_map(|actor| current.companion_snapshot(actor).ok())
            .find(|snapshot| snapshot.forge.is_none())
            .unwrap();
        assert_eq!((replacement.health, replacement.maximum_health), (100, 100));
        current.drive_forge_population(280_000_000).unwrap();
        assert_eq!(
            current
                .spawn_point_creatures()
                .into_iter()
                .filter_map(|(_, _, a)| a)
                .filter(|a| current.companion_snapshot(*a).unwrap().forge
                    == Some(ForgeCreature::Fiendish))
                .count(),
            2
        );
        current.drive_forge_population(280_001_000).unwrap();
        assert_eq!(
            current
                .spawn_point_creatures()
                .into_iter()
                .filter_map(|(_, _, a)| a)
                .filter(|a| current.companion_snapshot(*a).unwrap().forge
                    == Some(ForgeCreature::Fiendish))
                .count(),
            3
        );
    }
}
