//! Orchestration below a test-only physical Instance grant. This is not an entry authority
//! fixture: PostgreSQL tests independently prove the actual handoff and participant fences.
use super::*;
use crate::domain::soulpit::SoulpitEncounter;
use crate::foundation::WorldId;

type TestResult<T = ()> = Result<T, String>;
fn debug(value: impl std::fmt::Debug) -> String {
    format!("{value:?}")
}
fn id(seed: u8) -> [u8; 16] {
    let mut value = [seed; 16];
    value[6] = 0x70;
    value[8] = 0x80;
    value
}

struct Scene {
    allow_spawn: bool,
    native_floor: Option<i16>,
    only_spawn: Option<MovementLocalPosition>,
    effects: std::sync::Mutex<Vec<(MovementLocalPosition, u16)>>,
}
impl ActivityHandoffScene for Scene {
    fn emit_spawn_effect(&self, at: MovementLocalPosition, effect: u16) -> bool {
        let Ok(mut effects) = self.effects.lock() else {
            return false;
        };
        effects.push((at, effect));
        true
    }
    fn permits_spawn(&self, at: MovementLocalPosition) -> bool {
        self.allow_spawn
            && self.native_floor.is_none_or(|floor| at.floor == floor)
            && self.only_spawn.is_none_or(|position| at == position)
    }
    fn attack_target(&self, _: ExactActorRef) -> Option<ExactActorRef> {
        None
    }
    fn permits_condition(
        &self,
        _: ExactActorRef,
        _: ExactActorRef,
        _: MovementLocalPosition,
        _: MovementLocalPosition,
    ) -> bool {
        true
    }
    fn player_attribute_damage_delta(
        &self,
        _: ExactActorRef,
        _: GameSessionId,
        _: u64,
    ) -> Option<i32> {
        Some(0)
    }
}

struct Owner {
    binding: SoulpitActivityBinding,
    carrier: ChannelActorCarrier,
    continuity: NamespaceContinuityGuard,
    content: ChannelContentPin,
    native: NativeGameplayState,
    scene: Scene,
    stored: SoulpitStoredEncounter,
    calls: Vec<(u64, SoulpitAction)>,
    registrations: Vec<Vec<SoulpitWaveActor>>,
    returns: Vec<EncounterPhase>,
    boss_rewards: Vec<SoulpitBossReward>,
    fail_spawn_ack: bool,
    fail_return: bool,
}

impl Owner {
    fn fixture(allow_spawn: bool) -> TestResult<Self> {
        Self::fixture_with_capacity(allow_spawn, 3)
    }
    fn fixture_with_capacity(allow_spawn: bool, capacity: usize) -> TestResult<Self> {
        let world = WorldId::decode(&id(1)).map_err(debug)?;
        let (controller,_)=oteryn_gameplay_content::content::native_gameplay::test_support::activated_with_loot_tables(world,None);
        let native = controller
            .active()
            .and_then(|active| active.native_gameplay())
            .ok_or("native fixture")?
            .clone();
        let scope = RuntimeScopeRefV1::instance(world, id(2)).map_err(debug)?;
        let generation = ScopeOwnershipGeneration::new(1).map_err(debug)?;
        let (carrier, continuity) =
            oteryn_gameplay_entities::carrier::soulpit::physical_test_kernel(
                scope, generation, capacity,
            )
            .map_err(debug)?;
        let content = ChannelContentPin::from_activation(
            world,
            1,
            native.source_digest(),
            [2; 32],
            [3; 32],
            [4; 32],
            (32375, 31151, 8),
        );
        Ok(Self {
            binding: SoulpitActivityBinding {
                handoff_id: id(3),
                scope,
                generation,
            },
            carrier,
            continuity,
            content,
            native,
            scene: Scene {
                allow_spawn,
                native_floor: None,
                only_spawn: None,
                effects: std::sync::Mutex::new(Vec::new()),
            },
            stored: SoulpitStoredEncounter {
                revision: 1,
                species: "oteryn:creature.rat".into(),
                encounter: SoulpitEncounter::new(250_000, &[0]).map_err(debug)?,
            },
            calls: Vec::new(),
            registrations: Vec::new(),
            returns: Vec::new(),
            boss_rewards: Vec::new(),
            fail_spawn_ack: false,
            fail_return: false,
        })
    }
}

impl ActivityHandoffOwnerRuntime for Owner {
    fn binding(&self) -> SoulpitActivityBinding {
        self.binding
    }
    fn with_kernel<T>(
        &mut self,
        operation: impl FnOnce(SoulpitKernel<'_>) -> Result<T, SoulpitRuntimeError>,
    ) -> Result<T, SoulpitRuntimeError> {
        operation(SoulpitKernel {
            carrier: &mut self.carrier,
            continuity: &self.continuity,
            content: &self.content,
            native: &self.native,
            scene: &self.scene,
        })
    }
    fn survivors(&self) -> Result<Vec<CurrentCharacterGameplayFence>, SoulpitRuntimeError> {
        Ok(Vec::new())
    }
    fn read_encounter(&mut self) -> ActivityFuture<'_, SoulpitStoredEncounter> {
        Box::pin(async { Ok(self.stored.clone()) })
    }
    fn commit_progress(
        &mut self,
        request: SoulpitStepRequest,
    ) -> ActivityFuture<'_, SoulpitStepOutcome> {
        Box::pin(async move {
            if self.stored.encounter.snapshot().phase.is_terminal()
                || request.expected_revision != self.stored.revision
            {
                return Err(SoulpitRuntimeError::InvalidState);
            }
            if self.fail_spawn_ack && matches!(request.action, SoulpitAction::Spawned { .. }) {
                self.fail_spawn_ack = false;
                return Err(SoulpitRuntimeError::AdapterUnavailable);
            }
            let now = request.now_micros;
            let encounter = &mut self.stored.encounter;
            let effect = match &request.action {
                SoulpitAction::Advance => encounter.advance(now),
                SoulpitAction::Spawned { wave, monster_ids } => encounter
                    .spawned(*wave, *monster_ids, now)
                    .map(|_| EncounterEffect::NoChange),
                SoulpitAction::Summoned {
                    wave,
                    monster_id,
                    stack,
                } => SoulpitStack::from_count(*stack)
                    .and_then(|stack| encounter.summoned(*wave, *monster_id, stack))
                    .and_then(|_| encounter.observe_terminal(now)),
                SoulpitAction::Defeated { wave, monster_id } => encounter
                    .defeated(*wave, *monster_id)
                    .and_then(|_| encounter.observe_terminal(now)),
                SoulpitAction::Leave { slot } => encounter
                    .leave(*slot, now)
                    .and_then(|_| encounter.observe_terminal(now)),
                SoulpitAction::Rejoin { slot } => encounter
                    .rejoin(*slot, now)
                    .and_then(|_| encounter.observe_terminal(now)),
                SoulpitAction::Interrupt => {
                    encounter.interrupt(now.max(encounter.snapshot().last_observed_at_micros))
                }
            }
            .map_err(|_| SoulpitRuntimeError::InvalidState)?;
            self.calls.push((now, request.action));
            self.stored.revision += 1;
            Ok(SoulpitStepOutcome {
                revision: self.stored.revision,
                encounter: self.stored.encounter.clone(),
                effect,
                replayed: false,
            })
        })
    }
    fn register_creatures(
        &mut self,
        actors: &[SoulpitWaveActor],
    ) -> Result<(), SoulpitRuntimeError> {
        self.registrations.push(actors.to_vec());
        Ok(())
    }
    fn settle_boss_rewards(&mut self, reward: SoulpitBossReward) -> ActivityFuture<'_, ()> {
        Box::pin(async move {
            if !self
                .boss_rewards
                .iter()
                .any(|prior| prior.occurrence == reward.occurrence)
            {
                self.boss_rewards.push(reward);
            }
            Ok(())
        })
    }
    async fn return_participants(
        &mut self,
        phase: EncounterPhase,
    ) -> Result<(), SoulpitRuntimeError> {
        self.returns.push(phase);
        if self.fail_return {
            self.fail_return = false;
            Err(SoulpitRuntimeError::AdapterUnavailable)
        } else {
            Ok(())
        }
    }
}

#[test]
fn srx3_soulpit_random_position_selects_only_source_walkable_cells() -> TestResult {
    tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(debug)?.block_on(async {
        let mut owner=Owner::fixture_with_capacity(true,32)?;
        let only=MovementLocalPosition{x:32374,y:31145,floor:8};
        owner.scene.only_spawn=Some(only);
        drive_soulpit_deadlines(&mut owner,4_250_000).await.map_err(debug)?;
        let actors=owner.carrier.soulpit_actors(&owner.continuity,owner.binding.handoff_id).map_err(debug)?;
        assert_eq!(actors.len(),1,"Zone:randomPosition filters walkability BEFORE its uniform sample; later occupied attempts do not redraw");
        assert_eq!(actors[0].position,only);
        Ok(())
    })
}

#[test]
fn srx3_soulpit_runtime_spawns_on_native_source_floor() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(debug)?
        .block_on(async {
            let mut owner = Owner::fixture_with_capacity(true, 32)?;
            owner.scene.native_floor = Some(-8);
            owner.content = ChannelContentPin::from_activation(
                owner.binding.scope.world_id(),
                1,
                owner.native.source_digest(),
                [2; 32],
                [3; 32],
                [4; 32],
                (32373, 31151, -8),
            );
            drive_soulpit_deadlines(&mut owner, 4_250_000)
                .await
                .map_err(debug)?;
            let actors = owner
                .carrier
                .soulpit_actors(&owner.continuity, owner.binding.handoff_id)
                .map_err(debug)?;
            assert_eq!(
                actors.len(),
                MONSTERS_PER_WAVE,
                "source z8 is native floor -8, not an empty successful wave"
            );
            Ok(())
        })
}

#[test]
fn precise_spawn_and_source_global_clearance_have_independent_deadlines() -> TestResult {
    tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(debug)?.block_on(async {
    let mut owner=Owner::fixture(false)?;
    assert_eq!(drive_soulpit_activity(&mut owner,1_000_000).await.map_err(debug)?,Some(1_250_000));
    assert_eq!(owner.stored.encounter.snapshot().phase,EncounterPhase::Preparing);
    assert_eq!(drive_soulpit_deadlines(&mut owner,4_250_000).await.map_err(debug)?,Some(600_250_000));
    assert_eq!(owner.stored.encounter.snapshot().phase,EncounterPhase::Fighting);
    assert_eq!(owner.stored.encounter.snapshot().monsters_alive,0);
    assert!(owner.calls.iter().any(|(now,action)|*now==4_250_000 && matches!(action,SoulpitAction::Spawned {monster_ids,..} if monster_ids.iter().all(Option::is_none))));
    let calls=owner.calls.len();
    drive_soulpit_deadlines(&mut owner,4_750_000).await.map_err(debug)?;
    assert_eq!(owner.calls.len(),calls,"a precise wake after the4.5s gate cannot run wave clearance");
    assert_eq!(owner.stored.encounter.snapshot().wave,1);
    assert_eq!(drive_soulpit_activity(&mut owner,5_000_000).await.map_err(debug)?,Some(6_000_000));
    assert_eq!(owner.stored.encounter.snapshot().wave,2);
    assert_eq!(owner.stored.encounter.snapshot().phase,EncounterPhase::Preparing);
    let prior_cues=owner.scene.effects.lock().map_err(debug)?.len();
    for (at,next,burst) in [(6_000_000,7_000_000,1),(7_000_000,8_000_000,2),(8_000_000,9_000_000,3)] {
        assert_eq!(drive_soulpit_deadlines(&mut owner,at).await.map_err(debug)?,Some(next));
        assert_eq!(owner.stored.encounter.snapshot().wave,2);
        assert_eq!(owner.stored.encounter.snapshot().phase,EncounterPhase::Preparing);
        assert_eq!(owner.scene.effects.lock().map_err(debug)?.len(),prior_cues,
            "no eligible source position means no fabricated visual");
        assert_eq!(owner.carrier.next_soulpit_wave_cue(&owner.continuity,owner.binding.handoff_id,2).map_err(debug)?,Some(burst*7));
        assert_eq!(owner.stored.encounter.snapshot().monsters_alive,0);
    }
    assert_eq!(drive_soulpit_deadlines(&mut owner,9_000_000).await.map_err(debug)?,Some(600_250_000));
    assert_eq!(owner.scene.effects.lock().map_err(debug)?.len(),prior_cues);
    assert_eq!(owner.carrier.next_soulpit_wave_cue(&owner.continuity,owner.binding.handoff_id,2).map_err(debug)?,None);
    assert_eq!(owner.stored.encounter.snapshot().phase,EncounterPhase::Fighting);
    assert_eq!(owner.stored.encounter.snapshot().wave,2);
    assert_eq!(owner.stored.encounter.snapshot().monsters_alive,0);
    assert!(owner.calls.iter().any(|(now,action)|*now==9_000_000 && matches!(action,SoulpitAction::Spawned{wave:2,monster_ids} if monster_ids.iter().all(Option::is_none))));
    Ok(())
    })
}

#[test]
fn failed_spawn_ack_reuses_physical_attempts_and_terminal_return_retries_without_new_event()
-> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(debug)?
        .block_on(async {
            let mut owner = Owner::fixture(true)?;
            owner.fail_spawn_ack = true;
            assert!(matches!(
                drive_soulpit_deadlines(&mut owner, 4_250_000).await,
                Err(SoulpitRuntimeError::AdapterUnavailable)
            ));
            let before = owner
                .carrier
                .soulpit_actors(&owner.continuity, owner.binding.handoff_id)
                .map_err(debug)?;
            assert_eq!(before.len(), 3, "native carrier capacity is retained");
            drive_soulpit_activity(&mut owner, 5_000_000)
                .await
                .map_err(debug)?;
            let after = owner
                .carrier
                .soulpit_actors(&owner.continuity, owner.binding.handoff_id)
                .map_err(debug)?;
            assert_eq!(
                before.iter().map(|v| v.actor).collect::<Vec<_>>(),
                after.iter().map(|v| v.actor).collect::<Vec<_>>()
            );
            assert_eq!(owner.registrations.len(), 2);
            assert_eq!(owner.registrations[0], owner.registrations[1]);
            assert_eq!(owner.stored.encounter.snapshot().monsters_alive, 3);
            owner.fail_return = true;
            assert!(matches!(
                interrupt_soulpit_activity(&mut owner, 5_100_000).await,
                Err(SoulpitRuntimeError::AdapterUnavailable)
            ));
            let terminal_revision = owner.stored.revision;
            assert_eq!(
                owner.stored.encounter.snapshot().phase,
                EncounterPhase::Interrupted
            );
            assert!(
                owner
                    .carrier
                    .soulpit_actors(&owner.continuity, owner.binding.handoff_id)
                    .map_err(debug)?
                    .is_empty()
            );
            interrupt_soulpit_activity(&mut owner, 0)
                .await
                .map_err(debug)?;
            assert_eq!(
                owner.stored.revision, terminal_revision,
                "recovery must not append an event to a terminal encounter"
            );
            assert_eq!(
                owner.returns,
                [EncounterPhase::Interrupted, EncounterPhase::Interrupted]
            );
            Ok(())
        })
}

#[test]
fn spawn_ack_retry_preserves_a_lethal_actor_until_durable_death_can_be_recorded() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(debug)?
        .block_on(async {
            let mut owner = Owner::fixture(true)?;
            owner.fail_spawn_ack = true;
            assert!(matches!(
                drive_soulpit_deadlines(&mut owner, 4_250_000).await,
                Err(SoulpitRuntimeError::AdapterUnavailable)
            ));
            assert_eq!(owner.stored.encounter.snapshot().phase, EncounterPhase::Preparing);
            assert_eq!(owner.stored.encounter.snapshot().monsters_alive, 0);
            let before = owner
                .carrier
                .soulpit_actors(&owner.continuity, owner.binding.handoff_id)
                .map_err(debug)?;
            assert_eq!(before.len(), 3);
            let victim = before.first().ok_or("physical unacknowledged spawn")?;
            owner
                .carrier
                .commit_encounter_single_damage(
                    &owner.continuity,
                    victim.actor.0,
                    oteryn_gameplay_entities::carrier::OwnerDamageCommand {
                        target: victim.species.as_bytes(),
                        occurrence: b"unacknowledged-spawn-lethal",
                        binding: b"unacknowledged-spawn-lethal\0source-lethal",
                        damage: victim.health,
                    },
                    None,
                    false,
                    None,
                    Some(4_500),
                )
                .map_err(debug)?;
            drive_soulpit_activity(&mut owner, 5_000_000)
                .await
                .map_err(debug)?;
            let acknowledged = owner
                .carrier
                .soulpit_actors(&owner.continuity, owner.binding.handoff_id)
                .map_err(debug)?;
            assert_eq!(
                before.iter().map(|actor| actor.actor).collect::<Vec<_>>(),
                acknowledged.iter().map(|actor| actor.actor).collect::<Vec<_>>(),
                "spawn acknowledgement must retain every original actor, including a pending death"
            );
            assert!(acknowledged.iter().any(|actor| actor.actor == victim.actor && actor.health == 0));
            assert_eq!(owner.registrations.len(), 2);
            assert_eq!(owner.registrations[0], owner.registrations[1]);
            assert_eq!(owner.stored.encounter.snapshot().phase, EncounterPhase::Fighting);
            assert_eq!(owner.stored.encounter.snapshot().monsters_alive, 3);
            drive_soulpit_activity(&mut owner, 6_000_000)
                .await
                .map_err(debug)?;
            assert_eq!(owner.stored.encounter.snapshot().monsters_alive, 2);
            let remaining = owner
                .carrier
                .soulpit_actors(&owner.continuity, owner.binding.handoff_id)
                .map_err(debug)?;
            assert_eq!(remaining.len(), 2);
            assert!(remaining.iter().all(|actor| actor.actor != victim.actor));
            assert_eq!(
                owner.calls.iter().filter(|(_, action)| matches!(
                    action,
                    SoulpitAction::Defeated { monster_id, .. } if *monster_id == victim.tag.monster_id
                )).count(),
                1,
                "the retained lethal receipt is acknowledged exactly once"
            );
            drive_soulpit_activity(&mut owner, 7_000_000)
                .await
                .map_err(debug)?;
            assert_eq!(owner.stored.encounter.snapshot().monsters_alive, 2);
            assert_eq!(owner.registrations.len(), 2, "retry cannot respawn the lost actor");
            Ok(())
        })
}

#[test]
fn recovery_rejects_a_foreign_instance_kernel_before_any_progress_or_return() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(debug)?
        .block_on(async {
            let mut owner = Owner::fixture(false)?;
            owner.binding.scope =
                RuntimeScopeRefV1::instance(owner.binding.scope.world_id(), id(9))
                    .map_err(debug)?;
            assert!(matches!(
                interrupt_soulpit_activity(&mut owner, 0).await,
                Err(SoulpitRuntimeError::InvalidScope)
            ));
            assert!(owner.calls.is_empty());
            assert!(owner.returns.is_empty());
            Ok(())
        })
}

#[test]
fn lethal_boss_reward_precedes_both_timeout_drivers_and_replayed_return_never_rewards_a_live_despawn()
-> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(debug)?
        .block_on(async {
            for (global_tick, killed) in [(false, true), (true, true), (false, false)] {
                let mut owner = Owner::fixture_with_capacity(true, 8)?;
                for wave in 1..=3 {
                    let start = owner.stored.encounter.snapshot().wave_started_at_micros;
                    owner
                        .stored
                        .encounter
                        .advance(start + 4_000_000)
                        .map_err(debug)?;
                    owner
                        .stored
                        .encounter
                        .spawned(wave, [None; 7], start + 4_000_000)
                        .map_err(debug)?;
                    owner
                        .stored
                        .encounter
                        .advance(start + 5_000_000)
                        .map_err(debug)?;
                }
                drive_soulpit_deadlines(&mut owner, 19_250_000)
                    .await
                    .map_err(debug)?;
                let boss = owner
                    .carrier
                    .soulpit_actors(&owner.continuity, owner.binding.handoff_id)
                    .map_err(debug)?
                    .into_iter()
                    .find(|actor| actor.tag.stack == SoulpitStack::Boss && !actor.tag.is_summon)
                    .ok_or("physical source boss")?;
                if killed {
                    owner
                        .carrier
                        .commit_encounter_single_damage(
                            &owner.continuity,
                            boss.actor.0,
                            oteryn_gameplay_entities::carrier::OwnerDamageCommand {
                                target: boss.species.as_bytes(),
                                occurrence: b"last-second-boss",
                                binding: b"last-second-boss\0source-lethal",
                                damage: 999_999,
                            },
                            None,
                            false,
                            None,
                            Some(599_900),
                        )
                        .map_err(debug)?;
                }
                owner.fail_return = true;
                assert!(matches!(
                    drive(&mut owner, 600_250_000, global_tick).await,
                    Err(SoulpitRuntimeError::AdapterUnavailable)
                ));
                assert_eq!(
                    owner.stored.encounter.snapshot().phase,
                    EncounterPhase::TimedOut,
                    "no survival or Animus reward at timeout"
                );
                assert_eq!(owner.boss_rewards.len(), usize::from(killed));
                if let Some(reward) = owner.boss_rewards.first() {
                    assert_eq!(reward.experience, 140, "source Rat5XP times28");
                    assert_eq!(
                        reward.death_at_ms, 599_900,
                        "actual lethal time survives the later timer wake"
                    );
                }
                let revision = owner.stored.revision;
                assert_eq!(
                    drive(&mut owner, 600_300_000, false).await.map_err(debug)?,
                    None
                );
                assert_eq!(owner.stored.revision, revision);
                assert_eq!(owner.boss_rewards.len(), usize::from(killed));
                assert!(
                    owner
                        .carrier
                        .soulpit_actors(&owner.continuity, owner.binding.handoff_id)
                        .map_err(debug)?
                        .is_empty()
                );
            }
            Ok(())
        })
}

#[test]
fn srx3_soulpit_prepared_positions_emit_four_native_cues_once_without_spawn_redraw() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(debug)?
        .block_on(async {
            let mut owner = Owner::fixture_with_capacity(true, 8)?;
            drive_soulpit_deadlines(&mut owner, 1_249_999)
                .await
                .map_err(debug)?;
            assert!(owner.scene.effects.lock().map_err(debug)?.is_empty());
            drive_soulpit_deadlines(&mut owner, 1_250_000)
                .await
                .map_err(debug)?;
            let first = owner.scene.effects.lock().map_err(debug)?.clone();
            assert_eq!(
                first.len(),
                7,
                "each Source-selected wave position emits its1s teleport cue"
            );
            assert!(first.iter().all(|(_, effect)| *effect == 11));
            drive_soulpit_deadlines(&mut owner, 1_250_000)
                .await
                .map_err(debug)?;
            assert_eq!(
                owner.scene.effects.lock().map_err(debug)?.len(),
                7,
                "same deadline never replays a cue"
            );
            // The source retained these positions before the creation callback.
            // A newly blocked scene prevents creation but must not redraw cues.
            owner.scene.allow_spawn = false;
            drive_soulpit_deadlines(&mut owner, 4_250_000)
                .await
                .map_err(debug)?;
            let effects = owner.scene.effects.lock().map_err(debug)?.clone();
            assert_eq!(effects.len(), 28, "source1/2/3/4second wave cues");
            for burst in effects.as_chunks::<7>().0.iter() {
                assert_eq!(burst, first.as_slice());
            }
            assert_eq!(owner.stored.encounter.snapshot().monsters_alive, 0);
            assert!(
                owner
                    .carrier
                    .soulpit_actors(&owner.continuity, owner.binding.handoff_id)
                    .map_err(debug)?
                    .is_empty()
            );
            Ok(())
        })
}
