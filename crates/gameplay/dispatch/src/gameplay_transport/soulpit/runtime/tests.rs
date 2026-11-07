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
}
impl ActivityHandoffScene for Scene {
    fn permits_spawn(&self, _: MovementLocalPosition) -> bool {
        self.allow_spawn
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
            scene: Scene { allow_spawn },
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
    fn kernel(&mut self) -> Result<SoulpitKernel<'_>, SoulpitRuntimeError> {
        Ok(SoulpitKernel {
            carrier: &mut self.carrier,
            continuity: &self.continuity,
            content: &self.content,
            native: &self.native,
            scene: &self.scene,
        })
    }
    fn survivors(&self) -> Vec<CurrentCharacterGameplayFence> {
        Vec::new()
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
    fn return_participants(&mut self, phase: EncounterPhase) -> ActivityFuture<'_, ()> {
        Box::pin(async move {
            self.returns.push(phase);
            if self.fail_return {
                self.fail_return = false;
                Err(SoulpitRuntimeError::AdapterUnavailable)
            } else {
                Ok(())
            }
        })
    }
}

#[test]
fn precise_spawn_and_source_global_clearance_have_independent_deadlines() -> TestResult {
    tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(debug)?.block_on(async {
    let mut owner=Owner::fixture(false)?;
    assert_eq!(drive_soulpit_activity(&mut owner,1_000_000).await.map_err(debug)?,Some(4_250_000));
    assert_eq!(owner.stored.encounter.snapshot().phase,EncounterPhase::Preparing);
    assert_eq!(drive_soulpit_deadlines(&mut owner,4_250_000).await.map_err(debug)?,Some(600_250_000));
    assert_eq!(owner.stored.encounter.snapshot().phase,EncounterPhase::Fighting);
    assert_eq!(owner.stored.encounter.snapshot().monsters_alive,0);
    assert!(owner.calls.iter().any(|(now,action)|*now==4_250_000 && matches!(action,SoulpitAction::Spawned {monster_ids,..} if monster_ids.iter().all(Option::is_none))));
    let calls=owner.calls.len();
    drive_soulpit_deadlines(&mut owner,4_750_000).await.map_err(debug)?;
    assert_eq!(owner.calls.len(),calls,"a precise wake after the4.5s gate cannot run wave clearance");
    assert_eq!(owner.stored.encounter.snapshot().wave,1);
    assert_eq!(drive_soulpit_activity(&mut owner,5_000_000).await.map_err(debug)?,Some(9_000_000));
    assert_eq!(owner.stored.encounter.snapshot().wave,2);
    assert_eq!(owner.stored.encounter.snapshot().phase,EncounterPhase::Preparing);
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
