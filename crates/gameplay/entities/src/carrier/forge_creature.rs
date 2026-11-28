//! Forge classification lives in the existing spawn and actor owners.
use super::*;
use runtime_actor_companion::CompanionSnapshot;
#[cfg(feature = "qualification-spawn")]
#[path = "forge_population_draw.rs"]
mod qualification;
use oteryn_gameplay_domain::domain::echo_warden::EchoVariant;
use oteryn_gameplay_domain::domain::forge_creature::{
    FIENDISH_RESPAWN_MICROS, FIENDISH_SIMULTANEOUS_MAX, ForgeCreature, ForgeSpawnCandidate,
    forge_population_roll, influenced_available_slots, plan_fiendish_tick, plan_influenced_tick,
};
#[cfg(feature = "qualification-spawn")]
pub(super) use qualification::PendingDraw;
#[cfg(feature = "qualification-spawn")]
pub use qualification::{
    ForgePopulationCandidate, ForgePopulationCensus, ForgePopulationDrawRefusal,
    ForgePopulationSelection,
};

// Canary's Game::updateForgeableMonsters maintenance cadence, in owner semantic time.
pub(super) const POPULATION_TICK_MICROS: u64 = 10_000_000;

impl ChannelRuntimeV1 {
    /// Echo's influenced family uses the existing Forge stack draw and modifier owner.
    pub(super) fn apply_echo_influenced(
        &mut self,
        actor: ExactActorRef,
    ) -> Result<(), CarrierError> {
        let stacks = 1 + (self.forge_roll(actor) % 5) as u8;
        let kind = ForgeCreature::new_influenced(stacks).ok_or(CarrierError::InvalidDamage)?;
        self.apply_forge_classification(actor, kind)
    }

    fn forge_roll(&self, actor: ExactActorRef) -> u64 {
        self.population_roll(actor, None)
    }

    fn population_roll(&self, actor: ExactActorRef, seed: Option<u64>) -> u64 {
        forge_population_roll(
            self.content.server_artifact_digest(),
            actor.placement_identity(),
            actor.actor_local_generation(),
            seed,
        )
    }

    pub(super) fn apply_forge_classification(
        &mut self,
        actor: ExactActorRef,
        kind: ForgeCreature,
    ) -> Result<(), CarrierError> {
        self.apply_forge_classification_observed(actor, kind, |_| {})
    }

    fn apply_forge_classification_observed(
        &mut self,
        actor: ExactActorRef,
        kind: ForgeCreature,
        observe: impl FnOnce(&CompanionSnapshot),
    ) -> Result<(), CarrierError> {
        self.assert_actor_spell_unreserved(actor)?;
        let snapshot = self.companion_snapshot(actor)?;
        if snapshot.state.master.is_some() || snapshot.state.policy.forge_difficulty.is_none() {
            return Err(CarrierError::InvalidCreatureTarget);
        }
        // Echo's aura can enlist an ordinary spawn before the Forge population owner
        // classifies it. Crystal echo_warden.lua:85-88 floors its current maximum;
        // Monster::applyStacks (monster.cpp:2752-2757) then scales that maximum and
        // fills health. Rebuilding from the immutable species policy loses the aura.
        let echo_minion = self.echo_variant(actor) == Some(EchoVariant::Minion);
        let base_maximum = if echo_minion {
            snapshot.maximum_health
        } else {
            snapshot.state.policy.maximum_health
        };
        let maximum = i64::try_from(
            (i128::from(base_maximum) * i128::from(kind.health_percent()) + 99) / 100,
        )
        .map_err(|_| CarrierError::DamageOverflow)?;
        let current = if echo_minion {
            maximum
        } else {
            i64::try_from(
                i128::from(snapshot.health) * i128::from(maximum)
                    / i128::from(snapshot.maximum_health),
            )
            .map_err(|_| CarrierError::DamageOverflow)?
            .max(1)
        };
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
        observe(&snapshot);
        Ok(())
    }

    /// Set the classified spawn for an exact combat scenario without advancing the population.
    #[cfg(any(test, feature = "test-support"))]
    pub fn classify_forge_spawn_for_test(
        &mut self,
        actor: ExactActorRef,
        kind: ForgeCreature,
    ) -> Result<(), CarrierError> {
        self.apply_forge_classification(actor, kind)
    }

    pub(super) fn drive_forge_population(&mut self, now: u64) -> Result<(), CarrierError> {
        self.select_forge_population(now, None)
    }

    #[cfg(any(test, feature = "test-support"))]
    pub(super) fn select_qualification_forge_spawn(
        &mut self,
        source: SpawnSourceId,
        kind: ForgeCreature,
    ) -> Result<(), CarrierError> {
        // Qualification supplies the spawn census and the draw while the
        // ordinary owner still checks eligibility and selects the class.
        let next = std::mem::take(&mut self.forge_next_tick_micros);
        let fiends = std::mem::take(&mut self.forge_fiends);
        let result = self.select_forge_population(0, Some((source, kind)));
        self.forge_fiends = fiends;
        self.forge_next_tick_micros = next;
        result?;
        let actor = self
            .carrier
            .spawn_cell_state(source, 0)?
            .live
            .ok_or(CarrierError::InvalidCreatureTarget)?;
        if self.companion_snapshot(actor)?.forge != Some(kind) {
            return Err(CarrierError::InvalidCreatureTarget);
        }
        Ok(())
    }

    fn select_forge_population(
        &mut self,
        now: u64,
        fixture: Option<(SpawnSourceId, ForgeCreature)>,
    ) -> Result<(), CarrierError> {
        // Observe only the retained incarnations on ordinary owner turns. A dead
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
        #[cfg(feature = "qualification-spawn")]
        let mut draw = if fixture.is_none() {
            self.take_qualification_forge_draw(now)
        } else {
            None
        };
        #[cfg(feature = "qualification-spawn")]
        let population_seed = draw.as_ref().map(|draw| draw.seed);
        #[cfg(not(feature = "qualification-spawn"))]
        let population_seed = None;
        self.forge_fiends
            .retain(|(_, due)| due.is_none_or(|due| now < due));
        let available = if matches!(fixture, Some((_, ForgeCreature::Influenced(_)))) {
            0
        } else {
            usize::from(FIENDISH_SIMULTANEOUS_MAX).saturating_sub(self.forge_fiends.len())
        };
        #[cfg(feature = "qualification-spawn")]
        if let Some(draw) = &mut draw {
            draw.census.fiendish_available_slots = available;
            draw.census.retained.extend(
                self.forge_fiends
                    .iter()
                    .filter(|(actor, due)| {
                        due.is_some() && self.companion_snapshot(*actor).is_err()
                    })
                    .map(|(actor, _)| (*actor, ForgeCreature::Fiendish)),
            );
        }
        let mut candidates = Vec::new();
        let mut actors = Vec::new();
        let mut existing = Vec::new();
        // Only native spawn points participate: summons, raid callbacks and encounter children
        // have no ordinary spawn point and cannot become Forge creatures.
        for source in &self.carrier.spawns {
            if fixture.is_some_and(|(fixture_source, _)| source.source != fixture_source) {
                continue;
            }
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
                #[cfg(feature = "qualification-spawn")]
                if let Some(draw) = &mut draw {
                    draw.record_candidate(identity, &snapshot);
                    if eligible {
                        draw.census.eligible.push((identity, actor));
                    }
                    if let Some(kind) = snapshot.forge {
                        draw.census.retained.push((actor, kind));
                    }
                }
                if snapshot.forge == Some(ForgeCreature::Fiendish) {
                    existing.push(identity);
                }
            }
        }
        let seed = population_seed.unwrap_or_else(|| {
            u64::from_be_bytes(
                self.content.server_artifact_digest()[..8]
                    .try_into()
                    .unwrap_or_default(),
            )
        });
        let plan = plan_fiendish_tick(now, seed, &existing, &candidates);
        #[cfg(feature = "qualification-spawn")]
        if let Some(draw) = &mut draw {
            draw.census.fiendish_vacancy = plan.vacancy;
        }
        let mut selected = 0;
        for identity in &plan.selected {
            if selected < available
                && !existing.contains(identity)
                && let Some((_, actor)) = actors.iter().find(|(id, _)| id == identity)
            {
                let kind = ForgeCreature::Fiendish;
                self.apply_forge_classification_observed(*actor, kind, |snapshot| {
                    #[cfg(feature = "qualification-spawn")]
                    if let Some(draw) = &mut draw {
                        draw.record_selection(snapshot, kind);
                    }
                    #[cfg(not(feature = "qualification-spawn"))]
                    let _ = snapshot;
                })?;
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
        let influenced_available = influenced_available_slots(influenced_count);
        #[cfg(feature = "qualification-spawn")]
        if let Some(draw) = &mut draw {
            draw.census.influenced_count = influenced_count;
            draw.census.influenced_available_slots = influenced_available;
        }
        let influenced_rolls = actors
            .iter()
            .filter(|(_, actor)| {
                self.companion_snapshot(*actor).is_ok_and(|s| {
                    s.state.master.is_none()
                        && s.state.policy.forge_difficulty.is_some()
                        && s.forge.is_none()
                })
            })
            .map(|(_, actor)| {
                let roll = match fixture {
                    Some((_, ForgeCreature::Influenced(stacks))) => u64::from(stacks - 1),
                    _ => self.population_roll(*actor, population_seed),
                };
                (roll, *actor)
            });
        if let Some((actor, kind)) = plan_influenced_tick(influenced_available, influenced_rolls) {
            self.apply_forge_classification_observed(actor, kind, |snapshot| {
                #[cfg(feature = "qualification-spawn")]
                if let Some(draw) = &mut draw {
                    draw.record_selection(snapshot, kind);
                }
                #[cfg(not(feature = "qualification-spawn"))]
                let _ = snapshot;
            })?;
        }
        let next_scan = now.saturating_add(POPULATION_TICK_MICROS);
        self.forge_next_tick_micros = self
            .forge_fiends
            .iter()
            .filter_map(|(_, due)| *due)
            .min()
            .unwrap_or(next_scan)
            .min(next_scan);
        #[cfg(feature = "qualification-spawn")]
        if let Some(draw) = draw {
            let _ = draw.reply.send(Ok(draw.census));
        }
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
        let mut current = super::super::tests::runtime(32);
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

        let mut familiar = policy.clone();
        familiar.definition_key = "fixture:familiar".into();
        familiar.is_familiar = true;
        familiar.forge_difficulty = None;
        current
            .install_companion_policies(
                CompiledCreaturePolicies::from_active_artifact(
                    pin.server_artifact_digest(),
                    vec![familiar, policy],
                )
                .unwrap(),
            )
            .unwrap();
        let points = |range: std::ops::Range<i32>| {
            range
                .map(|x| {
                    (
                        (x, 0, 0),
                        60_000,
                        "oteryn:creature.rat".to_owned(),
                        100,
                        false,
                    )
                })
                .collect()
        };
        current
            .begin_activation_spawn_sources(vec![(points(2..7), 5_000), (points(7..11), 5_000)])
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
        let session = super::super::tests::session(42);
        let reservation = current.reserve_fresh_session(session).unwrap();
        let owner = current.commit_fresh_session(reservation).unwrap();
        let position = |x| MovementLocalPosition { x, y: 0, floor: 0 };
        let familiar = current
            .create_companion(
                owner,
                session,
                "fixture:familiar",
                position(11),
                None,
                None,
                1,
            )
            .unwrap();
        let owned_rat = current
            .create_companion(
                owner,
                session,
                "oteryn:creature.rat",
                position(12),
                None,
                None,
                0,
            )
            .unwrap();
        current
            .drive_spawn_respawns(9_999_999, |_, _| true)
            .unwrap();
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
        current
            .drive_spawn_respawns(10_000_000, |_, _| true)
            .unwrap();
        for owned in [familiar, owned_rat] {
            let snapshot = current.companion_snapshot(owned).unwrap();
            let master = snapshot.state.master.unwrap();
            assert_eq!((master.actor, master.session), (owner, session));
            assert_eq!(snapshot.forge, None);
            assert_eq!(
                current.apply_forge_classification(owned, ForgeCreature::Fiendish),
                Err(CarrierError::InvalidCreatureTarget)
            );
        }
        let fiends: Vec<_> = current
            .spawn_point_creatures()
            .into_iter()
            .filter_map(|(_, _, a)| a)
            .filter(|a| {
                current.companion_snapshot(*a).unwrap().forge == Some(ForgeCreature::Fiendish)
            })
            .collect();
        assert_eq!(fiends.len(), 4);
        let mut actual: Vec<_> = current
            .spawn_point_creatures()
            .into_iter()
            .filter_map(|(point, _, actor)| {
                fiends
                    .contains(&actor?)
                    .then_some((u64::from(point.source) << 8) | u64::from(point.ordinal))
            })
            .collect();
        let candidates: Vec<_> = current
            .spawn_point_creatures()
            .into_iter()
            .map(|(point, _, _)| ForgeSpawnCandidate {
                spawn_identity: (u64::from(point.source) << 8) | u64::from(point.ordinal),
                alive: true,
                eligible: true,
            })
            .collect();
        let seed = u64::from_be_bytes(pin.server_artifact_digest()[..8].try_into().unwrap());
        let mut expected = plan_fiendish_tick(10_000_000, seed, &[], &candidates).selected;
        actual.sort_unstable();
        expected.sort_unstable();
        assert_eq!(actual, expected);
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
            3
        );
        assert_eq!(current.forge_next_tick_micros, 280_001_000);
        assert!(
            current
                .spawn_point_creatures()
                .into_iter()
                .filter_map(|(_, _, actor)| actor)
                .any(|actor| current.companion_snapshot(actor).unwrap().forge.is_none()),
            "at least one eligible native spawn must remain for the retained slot"
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
            4
        );
    }
}
