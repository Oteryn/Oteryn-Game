//! Thornfire's current native source homes credit the existing World counter owner.
use super::world_change_kills::{activate, seed_epoch};
use super::*;
use crate::durability::world_event::changes::{
    self,
    counters::{CounterKind as K, CounterState, Credit},
};
use crate::foundation::{CharacterLease, MovementLocalPosition};
use crate::gameplay_transport::kill_reward::{DrainOutcome, KillSink, TakeOutcome};
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use std::sync::Arc;

#[test]
fn wvar_thornfire_native_homes_replay_currentness_and_next_save() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        eprintln!("WVAR_THORNFIRE_POSTGRES_SKIPPED no_admin_url");
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "Thornfire native PG thread")??;
    Ok(())
}
fn scenario() -> TestResult {
    run(async |admin| {
        let started = std::time::Instant::now();
        reset();
        let h = Harness::create_with_admission_seed(admin, "wvar_thornfire", false, false).await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut deps = Dependencies::new()?;
        let (controller, activated) =
            super::death_ml_loss_composed::production_generation_with_pin(
                deps.admitted.world_id,
                "content-1",
            )?;
        let generation = controller.active().ok_or("generation")?;
        let native = generation.native_gameplay().ok_or("native")?;
        let world = deps.admitted.world_id;
        let epoch = 200_u64;
        seed_epoch(
            &h,
            world,
            epoch,
            generation.identity().server_artifact_digest(),
            generation.identity().client_artifact_digest(),
            activated.frame_binding().digest(),
        )
        .await?;
        let sources = &changes::counters::bindings()?.definitions[K::Thornfire.index()].sources;
        assert_eq!(sources.len(), 54);
        let first = home(&sources[0])?;
        let origin = MovementLocalPosition {
            x: first.x - 1,
            ..first
        };
        let day = h
            .root
            .world_changes(world, epoch)
            .await?
            .ok_or("initial Thornfire decision")?;
        assert!(day.world_counters);
        assert_eq!(day.counter_stages[K::Thornfire.index()], 1);
        let pin = ChannelContentPin::from_activation(
            world,
            1,
            generation.identity().server_artifact_digest(),
            generation.identity().client_artifact_digest(),
            activated.frame_binding().digest(),
            [8; 32],
            (origin.x, origin.y, origin.floor),
        );
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            world,
            deps.admitted.channel_id,
            crate::foundation::NodeId::decode(&id(1)).map_err(debug)?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            8,
            pin,
        )
        .map_err(debug)?;
        native
            .install_companion_policies(&mut runtime)
            .map_err(debug)?;
        runtime
            .install_world_change_cycles(
                epoch,
                K::ALL
                    .into_iter()
                    .map(|kind| (kind.key().to_owned(), day.counter_generations[kind.index()]))
                    .collect(),
            )
            .map_err(debug)?;
        let reservation = runtime
            .reserve_fresh_session(deps.admitted.game_session_id)
            .map_err(debug)?;
        deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .initialize_pinned_test_position(deps.actor, origin)
            .map_err(debug)?;
        deps.runtime = Mutex::new(runtime);
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(generation);
        owner.reset_epoch = epoch;
        owner.quest_catalogue = Some(Arc::new(
            oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
                .map_err(debug)?
                .catalogue()
                .clone(),
        ));
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.runtime_actor = Some(deps.actor);
        let session = admitted.game_session_id;
        let fence = owner
            .current_quest_fence(session)
            .await
            .map_err(|()| "fence")?
            .ok_or("fence")?;
        admitted.item_fence = Some(fence.item_fence());
        owner
            .runtime
            .lock()
            .await
            .bind_attacker_lease(
                deps.actor,
                session,
                CharacterLease::new(
                    crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                        .map_err(debug)?,
                    fence.character_lease_generation,
                )
                .map_err(debug)?,
            )
            .map_err(debug)?;
        activate(
            &h,
            generation.identity().server_artifact_digest(),
            generation.identity().client_artifact_digest(),
            activated.frame_binding().digest(),
        )
        .await?;
        let fresh = super::fresh_fixture::fresh::Source::new(0)?.request()?;
        h.root
            .initialize_admission_equipment(
                &authority,
                &h.node,
                &fence.item_fence(),
                generation.identity().server_artifact_digest(),
                fresh.operation(),
            )
            .await
            .map_err(debug)?;
        owner.admit_progression_session(&mut admitted).await;
        deps.initialize(&owner).await;
        owner.admit_quest_session(&admitted).await;
        owner
            .states
            .chat
            .admit(session, "WVAR native tester", owner.owner_now());

        let initial_xp = xp(&h).await?;
        let first_source = &sources[0];
        // A source-looking death tile is insufficient without the native spawn home.
        let event = kill_home(&owner, deps.actor, session, 600, first_source, None, false).await?;
        assert!(event.is_none());
        assert_eq!(state(&h, world).await?.counts, [0; 3]);
        // Exact native identity must match this exact authored home.
        let mut wrong = changes::counters::bindings()?.definitions[K::Thornfire.index()].sources
            [51]
        .creature
        .clone();
        if wrong == first_source.creature {
            wrong = "oteryn:creature.elf".into();
        }
        let event = kill_home(
            &owner,
            deps.actor,
            session,
            601,
            first_source,
            Some(&wrong),
            true,
        )
        .await?;
        assert!(event.is_none());
        assert_eq!(state(&h, world).await?.counts, [0; 3]);
        let mut first_event = None;
        for (index, source) in sources.iter().enumerate() {
            let event = kill_home(
                &owner,
                deps.actor,
                session,
                700 + u16::try_from(index)?,
                source,
                None,
                true,
            )
            .await?
            .ok_or("missing exact native home credit")?;
            assert!(
                event
                    .credits
                    .contains(&(K::Thornfire, Credit::Kill(u8::try_from(index)?)))
            );
            if index == 0 {
                stale_native_fences(&owner, session, epoch, &event).await?;
                first_event = Some(event);
            }
            let current = state(&h, world).await?;
            assert_eq!(
                current.stage, 1,
                "kills never activate burning before server save"
            );
            assert_eq!(current.counts[0].count_ones(), u32::try_from(index + 1)?);
            assert_eq!(
                current.counts[1].count_ones(),
                u32::try_from((index + 1).saturating_sub(51))?
            );
            if index == 0 {
                // A distinct respawn cannot substitute for any of the other53 homes.
                kill_home(&owner, deps.actor, session, 900, source, None, true).await?;
                assert_eq!(state(&h, world).await?.counts[0], 1);
            }
        }
        let complete = state(&h, world).await?;
        assert_eq!(complete.counts, [(1_u64 << 54) - 1, 7, 0]);
        assert_eq!(complete.stage, 1);
        assert!(
            xp(&h).await? > initial_xp,
            "actual native reward owner awards XP"
        );
        let corpses:i64=sqlx::query_scalar("SELECT count(*) FROM game_item_mint_receipts WHERE loot_purpose_key='CORPSE_MATERIALIZATION' AND materialized_at IS NOT NULL").fetch_one(&h.pool).await?;
        assert_eq!(
            corpses, 57,
            "54unique+one respawn+two ineligible source deaths have real native bodies"
        );
        assert!(h.root.world_changes(world, epoch + 1).await?.is_none());
        seed_epoch(
            &h,
            world,
            epoch + 1,
            generation.identity().server_artifact_digest(),
            generation.identity().client_artifact_digest(),
            activated.frame_binding().digest(),
        )
        .await?;
        let burning = h
            .root
            .world_changes(world, epoch + 1)
            .await?
            .ok_or("burning stored decision")?;
        assert_eq!(burning.counter_stages[K::Thornfire.index()], 2);
        assert_eq!(burning.variant("thornfire"), Some(0));
        assert!(
            owner
                .runtime
                .lock()
                .await
                .install_world_change_cycles(
                    epoch + 1,
                    K::ALL
                        .into_iter()
                        .map(|kind| (
                            kind.key().to_owned(),
                            burning.counter_generations[kind.index()]
                        ))
                        .collect()
                )
                .is_err(),
            "old runtime cannot install a decision from a new reset epoch"
        );
        assert_eq!(h.root.world_changes(world, epoch + 1).await?, Some(burning));
        let stale = first_event.ok_or("first captured event")?;
        let (scope, scope_generation) = {
            let runtime = owner.runtime.lock().await;
            (
                runtime.binding().runtime_scope(),
                runtime.binding().scope_generation(),
            )
        };
        assert_eq!(
            h.root
                .credit_world_change_event(
                    &authority,
                    &h.node,
                    scope,
                    scope_generation,
                    epoch,
                    stale
                )
                .await?,
            changes::counters::CreditOutcome::Refused
        );
        seed_epoch(
            &h,
            world,
            epoch + 2,
            generation.identity().server_artifact_digest(),
            generation.identity().client_artifact_digest(),
            activated.frame_binding().digest(),
        )
        .await?;
        let restored = h
            .root
            .world_changes(world, epoch + 2)
            .await?
            .ok_or("restored stored decision")?;
        assert_eq!(restored.counter_stages[K::Thornfire.index()], 1);
        assert_eq!(restored.variant("thornfire"), Some(1));
        assert_eq!(state(&h, world).await?.counts, [0; 3]);
        println!(
            "WVAR_THORNFIRE POSTGRES_CASE_COMPLETED native54homes,unique54bits,threeOverseers,source-home-not-death-position,wrong-source-refused,respawn-no-extra-bit,current-epoch,native-XP-corpse,next-save-burning,next-save-restored; duration={:?}",
            started.elapsed()
        );
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}
fn home(
    source: &oteryn_gameplay_world_change::counter::KillSource,
) -> TestResult<MovementLocalPosition> {
    let [x, _, y, _, z, _] = source.area;
    Ok(MovementLocalPosition {
        x,
        y,
        floor: i16::try_from(z)?,
    })
}
async fn state(h: &Harness, world: crate::foundation::WorldId) -> TestResult<CounterState> {
    let raw:serde_json::Value=sqlx::query_scalar("SELECT state FROM game_world_change_counters WHERE world_id=encode($1,'hex')::uuid AND change_key='thornfire'").bind(world.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    Ok(serde_json::from_value(raw)?)
}
async fn xp(h: &Harness) -> TestResult<i64> {
    let value:String=sqlx::query_scalar("SELECT total_experience::text FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    Ok(value.parse()?)
}

async fn kill_home(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    session: GameSessionId,
    source_id: u16,
    source: &oteryn_gameplay_world_change::counter::KillSource,
    replacement: Option<&str>,
    authored: bool,
) -> TestResult<Option<changes::counters::CreditEvent>> {
    owner.refresh_quest_session(session, false).await;
    let queue = owner.states.attack.lock().await.kills().clone();
    let native = owner
        .active_generation
        .and_then(|generation| generation.native_gameplay())
        .ok_or("native generation")?;
    let key = replacement.unwrap_or(&source.creature);
    native
        .reward_table()
        .row(key)
        .map_err(|error| format!("Thornfire native reward {key}: {error:?}"))?;
    let at = home(source)?;
    let observed = owner.owner_now();
    let target;
    let lease;
    {
        let mut runtime = owner.runtime.lock().await;
        let policy = runtime.companion_policy(key).map_err(debug)?;
        let hp = policy.maximum_health;
        if authored {
            let pin = runtime.content_pin().clone();
            let realized = runtime
                .realize_native_qualification_spawn_for_source(source_id, pin, key, at)
                .map_err(|error| format!("Thornfire source {source_id} {key} {at:?}: {error:?}"))?;
            target = realized.actor;
            assert_eq!(realized.health, hp);
            assert_eq!(runtime.authored_spawn_home(target), Some(at));
            // A genuine current movement-owner relocation cannot rewrite source membership.
            if source_id == 700 {
                let before = runtime.read_actor_position(target).map_err(debug)?;
                runtime
                    .borrow_movement_position()
                    .commit_creature_relocation(
                        target,
                        before,
                        MovementLocalPosition { x: at.x + 10, ..at },
                    )
                    .map_err(debug)?;
            }
        } else {
            target = runtime.admit_encounter_actor(key, hp, at).map_err(debug)?;
            runtime
                .install_creature_policy(target, key)
                .map_err(debug)?;
            assert_eq!(runtime.authored_spawn_home(target), None);
        }
        lease = runtime
            .bound_attacker_lease(actor, session)
            .map_err(debug)?;
        runtime
            .borrow_exact_actor_commit_at(observed.get() / 1000)
            .commit_damage_for_bound_attacker(
                target,
                actor,
                crate::foundation::CommandRef::new(
                    session,
                    crate::foundation::CommandId::new(u64::from(source_id)).map_err(debug)?,
                ),
                0,
                crate::foundation::OwnerDamageCommand {
                    target: key.as_bytes(),
                    occurrence: &source_id.to_be_bytes(),
                    binding: b"wvar-thornfire-source-native",
                    damage: hp,
                },
            )
            .map_err(debug)?;
        for _ in 0..2 {
            crate::gameplay_transport::kill_reward::record_creature_kill(
                &mut runtime,
                KillSink {
                    prey: None,
                    activity: &[],
                    queue: &queue,
                    rewards: native.reward_table(),
                    charms: None,
                },
                target,
                crate::combat::CapturedRewardPrincipal {
                    character: lease.character_id(),
                    lease_generation: lease.generation(),
                    session,
                    actor,
                },
                observed.get() / 1000,
                Some(observed),
            );
        }
        let proof = runtime.native_credited_kill_proof(target).map_err(debug)?;
        assert_eq!(proof.source_home(), authored.then_some(at));
        if source_id == 700 {
            assert_ne!(
                proof.position(),
                at,
                "lured corpse retains exact authored home"
            );
        }
    }
    let TakeOutcome::Taken(taken) = queue.take(session, lease.generation()) else {
        return Err("Thornfire native queue entry".into());
    };
    let event = taken
        .entry()
        .and_then(|entry| entry.world_change.clone())
        .map(|(_, event)| event)
        .filter(|event| event.credits.iter().any(|(kind, _)| *kind == K::Thornfire));
    drop(taken);
    assert_eq!(
        owner.drain_kill_rewards(session).await,
        DrainOutcome::Drained,
        "real native source {source_id} {key}"
    );
    owner
        .runtime
        .lock()
        .await
        .remove_encounter_actor(target)
        .map_err(debug)?;
    Ok(event)
}

async fn stale_native_fences(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    session: GameSessionId,
    epoch: u64,
    event: &changes::counters::CreditEvent,
) -> TestResult {
    let fence = owner
        .current_quest_fence(session)
        .await
        .map_err(|()| "native current fence")?
        .ok_or("native current fence")?
        .item_fence();
    for mode in 0..4 {
        let mut stale = fence;
        match mode {
            0 => {
                stale.connection_generation = crate::foundation::ConnectionGeneration::new(
                    fence.connection_generation.get() + 1,
                )
                .map_err(debug)?
            }
            1 => stale.character_lease_generation += 1,
            2 => {
                stale.scope_ownership_generation = crate::foundation::ScopeOwnershipGeneration::new(
                    fence.scope_ownership_generation.get() + 1,
                )
                .map_err(debug)?
            }
            _ => stale.game_session_id = GameSessionId::decode(&id(91)).map_err(debug)?,
        }
        assert_eq!(
            owner
                .root
                .credit_world_change_native_event(
                    owner.character,
                    owner.holder,
                    stale,
                    epoch,
                    event.clone()
                )
                .await?,
            changes::counters::CreditOutcome::Refused,
            "stale native source fence {mode}"
        );
    }
    assert_eq!(
        owner
            .root
            .credit_world_change_native_event(
                owner.character,
                owner.holder,
                fence,
                epoch,
                event.clone()
            )
            .await?,
        changes::counters::CreditOutcome::AlreadyApplied,
        "same accepted native event is idempotent"
    );
    Ok(())
}
