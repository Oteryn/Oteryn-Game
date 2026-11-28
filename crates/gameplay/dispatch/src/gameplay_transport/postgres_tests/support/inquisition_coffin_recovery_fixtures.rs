//! Exercise both committed coffin selectors through dispatch and the durable writer.
use super::fixture::{Harness, debug};
use super::{
    AdmittedSession, ComposedFreshAdmission, TestResult, quest_world_callbacks_fixtures as fixture,
};
use crate::gameplay_transport::connection::UseCommand;
use crate::map::overlay::TilePos;
use crate::map::view::{MapFacts, placement_key};
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Position};
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};

pub(super) const RESTART: &str = "inquisition-weakened-coffin";
pub(super) const FOREIGN_GENERATION: &str = "inquisition-weakened-coffin-side";

pub(super) fn matches(name: &str) -> bool {
    matches!(name, RESTART | FOREIGN_GENERATION)
}

pub(super) fn staging(name: &str) -> TestResult<Position> {
    Ok(fixture::callback(name)?
        .effects
        .encounter
        .as_ref()
        .ok_or("coffin encounter")?
        .players[0]
        .from)
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    weakened: &Callback,
    variant: &str,
) -> TestResult {
    let ordinary_restart = matches!(
        variant,
        "ordinary_restart" | "ordinary_published_restart" | "ordinary_rolled_back_delete"
    );
    let published_restart = matches!(
        variant,
        "ordinary_published_restart" | "weakened_published_restart"
    );
    let rolled_back_delete = matches!(
        variant,
        "rolled_back_delete" | "ordinary_rolled_back_delete"
    );
    let lost_ack = variant == "lost_delivery_ack";
    let live_without_reservation = variant == "live_without_reservation";
    let delivered_case = variant == "delivered" || rolled_back_delete;
    let native = owner
        .active_generation
        .and_then(|generation| generation.native_gameplay())
        .ok_or("native gameplay")?;
    owner
        .runtime
        .lock()
        .await
        .install_companion_policies(native.policies().clone())
        .map_err(debug)?;
    let requirements = weakened
        .storage
        .iter()
        .map(|gate| (gate.track.as_str(), gate.minimum))
        .collect::<Vec<_>>();
    super::rottin_rewards_fixtures::seed_tracks(h, owner, admitted, &requirements).await?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(debug)?
        .ok_or("coffin fence")?;
    if !ordinary_restart {
        for (index, offering) in weakened.offerings.iter().enumerate() {
            fixture::offering(
                owner,
                fence.item_fence(),
                offering,
                80 + u64::try_from(index)?,
            )
            .await?;
        }
    }
    owner.states.loop_ground.invalidate();
    let foreign_generation = variant.is_empty() && weakened.key.ends_with(FOREIGN_GENERATION);
    if foreign_generation {
        // Fixtures own generation 1. Represent carried-over drops with a distinct
        // generation while keeping their location, stack and item identity intact.
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx)
            .await?;
        let changed = sqlx::query("UPDATE game_item_ground_locations SET runtime_scope_ownership_generation=2 WHERE runtime_scope_ownership_generation=1 AND game_item_loop_drop_ref(corpse_ref)")
            .execute(&mut *tx)
            .await?;
        assert_eq!(
            changed.rows_affected(),
            u64::try_from(weakened.offerings.len())?
        );
        tx.commit().await?;
    } else if !delivered_case && !lost_ack && !published_restart && !live_without_reservation {
        owner
            .states
            .quest_callbacks
            .interrupt_after_commit
            .store(true, std::sync::atomic::Ordering::Release);
        assert_eq!(
            use_coffin(owner, admitted, weakened, 61).await?,
            UseDisposition::Rejected
        );
        assert!(owner.has_pending_quest_callback(admitted.game_session_id));
        assert!(
            owner
                .runtime
                .lock()
                .await
                .visible_entities()
                .creatures
                .is_empty()
        );
        assert_eq!(
            burns(h).await?,
            if ordinary_restart {
                0
            } else {
                i64::try_from(weakened.offerings.len())?
            },
        );
        owner.forget_coffin_projection_for_test(admitted.game_session_id)?;
        owner.states.loop_ground.invalidate();
        owner
            .refresh_quest_session(admitted.game_session_id, false)
            .await;
    }
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    if rolled_back_delete {
        sqlx::raw_sql(
            "CREATE FUNCTION reject_coffin_delivery_write() RETURNS trigger LANGUAGE plpgsql AS $$
             BEGIN RAISE EXCEPTION 'coffin delivery rolled back'; END $$;
             CREATE CONSTRAINT TRIGGER reject_coffin_delivery_write
             AFTER INSERT OR UPDATE OF death_id ON game_pending_vampire_coffin_payments
             DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
             WHEN (NEW.death_id IS NOT NULL) EXECUTE FUNCTION reject_coffin_delivery_write()",
        )
        .execute(&h.pool)
        .await?;
        assert_eq!(
            use_coffin(owner, admitted, weakened, 61).await?,
            UseDisposition::Rejected,
            "a rolled-back delivery cannot publish its Count"
        );
        assert!(
            owner
                .runtime
                .lock()
                .await
                .visible_entities()
                .creatures
                .is_empty()
        );
        let delivered: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_pending_vampire_coffin_payments WHERE death_id IS NOT NULL",
        )
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(
            delivered, 0,
            "the failed delivery persisted no spawn identity"
        );
        sqlx::query(
            "DROP TRIGGER reject_coffin_delivery_write ON game_pending_vampire_coffin_payments",
        )
        .execute(&h.pool)
        .await?;
        owner.forget_coffin_projection_for_test(admitted.game_session_id)?;
        owner.states.loop_ground.invalidate();
        owner
            .refresh_quest_session(admitted.game_session_id, false)
            .await;
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(std::time::Duration::from_secs(2))
            .ok_or("clock")?;
    }
    if lost_ack {
        owner
            .states
            .quest_callbacks
            .lose_coffin_delivery_ack
            .store(true, std::sync::atomic::Ordering::Release);
        assert_eq!(
            use_coffin(owner, admitted, weakened, 62).await?,
            UseDisposition::Rejected
        );
        let stored: Vec<u8> = sqlx::query_scalar(
            "SELECT uuid_send(death_id) FROM game_pending_vampire_coffin_payments",
        )
        .fetch_one(&h.pool)
        .await?;
        let kind = weakened.effects.encounter.as_ref().ok_or("encounter")?;
        let spawn = &kind.monsters[0];
        let unrelated_count = {
            let mut runtime = owner.runtime.lock().await;
            assert!(runtime.visible_entities().creatures.is_empty());
            let policy = runtime.companion_policy(&spawn.creature).map_err(debug)?;
            let actor = runtime
                .admit_encounter_actor(
                    &spawn.creature,
                    policy.maximum_health,
                    fixture::local(spawn.position),
                )
                .map_err(debug)?;
            runtime
                .install_creature_policy(actor, &spawn.creature)
                .map_err(debug)?;
            actor
        };
        let unrelated_death_id = crate::durability::boss_encounter::reward_chest::scoped_death_id(
            unrelated_count.runtime_scope(),
            unrelated_count.scope_generation().get(),
            unrelated_count.actor_local_id(),
            unrelated_count.actor_local_generation(),
        );
        assert_ne!(
            stored.as_slice(),
            unrelated_death_id,
            "abandoned spawn IDs cannot be reused"
        );
        defeat_count(owner, admitted, unrelated_count).await?;
        let settled: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM game_boss_death_settlements WHERE death_id=encode($1,'hex')::uuid)",
        )
        .bind(unrelated_death_id.as_slice())
        .fetch_one(&h.pool)
        .await?;
        assert!(
            settled,
            "the unrelated Count settled under its own death ID"
        );
        owner.states.loop_ground.invalidate();
        owner
            .refresh_quest_session(admitted.game_session_id, false)
            .await;
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(std::time::Duration::from_secs(2))
            .ok_or("clock")?;
    }
    let delivery = use_coffin(owner, admitted, weakened, if lost_ack { 63 } else { 62 });
    let outcome = if lost_ack {
        super::source_boss_handoff_fixtures::with_unrelated_admission(owner, crate::foundation::MovementLocalPosition { x: 80, y: 80, floor: 7 }, delivery).await??
    } else {
        delivery.await?
    };
    assert_eq!(outcome, UseDisposition::Committed);
    let expected = if foreign_generation || ordinary_restart {
        let fallback = weakened
            .vampire_coffin_fallback()
            .ok_or("ordinary coffin")?;
        fixture::callback(fallback)?
    } else {
        weakened.clone()
    };
    let runtime = owner.runtime.lock().await;
    let creatures = runtime.visible_entities().creatures;
    assert_eq!(creatures.len(), 1);
    assert_eq!(
        runtime
            .companion_snapshot(creatures[0].actor)
            .map_err(debug)?
            .state
            .policy
            .definition_key,
        expected
            .effects
            .encounter
            .as_ref()
            .ok_or("expected encounter")?
            .monsters[0]
            .creature,
    );
    drop(runtime);
    if delivered_case || published_restart {
        assert_eq!(
            owner
                .root
                .read_pending_vampire_coffin(owner.character, fence.character_id)
                .await
                .map_err(debug)?,
            Some(if ordinary_restart {
                oteryn_gameplay_quests::quest::world_callbacks::VampireCoffinKind::Ordinary
            } else {
                oteryn_gameplay_quests::quest::world_callbacks::VampireCoffinKind::Weakened
            }),
            "publishing a live Count must leave its payment recoverable",
        );
    }
    let mut count = creatures[0].actor;
    if live_without_reservation {
        let death_id = async || -> TestResult<Vec<u8>> {
            Ok(sqlx::query_scalar(
                "SELECT uuid_send(death_id) FROM game_pending_vampire_coffin_payments",
            )
            .fetch_one(&h.pool)
            .await?)
        };
        let original = death_id().await?;
        return_to_coffin(owner, admitted).await?;
        // The original Count has walked off its spawn square. Occupancy at the
        // spawn point cannot mask losing the per-Channel reservation.
        {
            use crate::domain::carrier_values::LocalPosition;
            let mut runtime = owner.runtime.lock().await;
            assert_eq!(
                owner.switch_encounter_lock(
                    &runtime,
                    admitted.runtime_actor.ok_or("coffin user")?,
                    admitted.game_session_id,
                ),
                None,
                "the player can leave this Channel after leaving the Count's room",
            );
            let context = runtime.encounter_position_context();
            let at = runtime
                .read_actor_position(count)
                .map_err(debug)?
                .position();
            let (carrier, continuity) = runtime.encounter_parts_mut();
            let position = carrier.read_position(continuity, count.0).map_err(debug)?;
            carrier
                .compare_commit_position(
                    continuity,
                    position,
                    context,
                    LocalPosition {
                        x: at.x - 1,
                        y: at.y,
                        floor: at.floor,
                    },
                )
                .map_err(debug)?;
        }
        owner.forget_coffin_cohort_for_test()?;
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(std::time::Duration::from_secs(2))
            .ok_or("clock")?;
        assert_ne!(
            use_coffin(owner, admitted, weakened, 63).await?,
            UseDisposition::Committed,
            "a live Count still owns the original payment"
        );
        assert_eq!(death_id().await?, original);
        {
            let mut runtime = owner.runtime.lock().await;
            assert!(runtime.contains_live_creature(count));
            assert_eq!(runtime.visible_entities().creatures.len(), 1);
            runtime.remove_encounter_actor(count).map_err(debug)?;
        }
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(std::time::Duration::from_secs(2))
            .ok_or("clock")?;
        assert_eq!(
            use_coffin(owner, admitted, weakened, 64).await?,
            UseDisposition::Committed,
            "the source owner can prove retirement and safely recover the payment",
        );
        assert_ne!(death_id().await?, original);
        assert_eq!(
            owner
                .runtime
                .lock()
                .await
                .visible_entities()
                .creatures
                .len(),
            1
        );
        return Ok(());
    }

    if published_restart || rolled_back_delete {
        return_to_coffin(owner, admitted).await?;
        owner
            .runtime
            .lock()
            .await
            .remove_encounter_actor(creatures[0].actor)
            .map_err(debug)?;
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(std::time::Duration::from_secs(2))
            .ok_or("clock")?;
        assert_eq!(
            use_coffin(owner, admitted, weakened, 63).await?,
            UseDisposition::Committed,
            "a published Count remains recoverable until its death settles",
        );
        let runtime = owner.runtime.lock().await;
        let creatures = runtime.visible_entities().creatures;
        assert_eq!(creatures.len(), 1);
        assert_ne!(creatures[0].actor, count);
        count = creatures[0].actor;
        assert_eq!(
            runtime
                .companion_snapshot(count)
                .map_err(debug)?
                .state
                .policy
                .definition_key,
            expected
                .effects
                .encounter
                .as_ref()
                .ok_or("encounter")?
                .monsters[0]
                .creature,
        );
    }
    if delivered_case {
        defeat_count(owner, admitted, count).await?;
        let catalogue = owner.quest_catalogue.as_deref().ok_or("quest catalogue")?;
        let hand_in = oteryn_gameplay_quests::quest::reward_callbacks::offered_for(catalogue)?
            .into_iter()
            .find(|callback| callback.key.ends_with("inquisition-vampire-hunt-ring"))
            .ok_or("ring hand-in")?;
        let [ring] = hand_in.take.as_slice() else {
            return Err("expected one ring input".into());
        };
        let released: i64 = if ordinary_restart {
            sqlx::query_scalar(
                "SELECT coalesce(sum(i.quantity),0)::bigint FROM game_item_mint_receipts r \
                 JOIN game_item_corpse_container_entries e ON e.parent_item_instance_id=r.item_instance_id \
                 JOIN game_item_instances i ON i.item_instance_id=e.item_instance_id \
                 WHERE r.loot_purpose_key='CORPSE_MATERIALIZATION' \
                 AND r.loot_table_production_key='oteryn:item.tibia.i8109' \
                 AND r.death_actor_local_id=$3 AND r.death_actor_local_generation=$4::text::numeric \
                 AND i.definition_production_key=$1 AND r.corpse_top_damage_character_id=encode($2,'hex')::uuid",
            )
            .bind(&ring.item)
            .bind(fence.character_id.as_bytes().as_slice())
            .bind(i64::from(count.actor_local_id()))
            .bind(count.actor_local_generation().to_string())
            .fetch_one(&h.pool)
            .await?
        } else {
            let death = crate::durability::boss_encounter::reward_chest::scoped_death_id(
                count.runtime_scope(),
                count.scope_generation().get(),
                count.actor_local_id(),
                count.actor_local_generation(),
            );
            sqlx::query_scalar(
                "SELECT coalesce(sum(quantity),0)::bigint FROM game_boss_reward_items \
                 WHERE definition_production_key=$1 AND character_id=encode($2,'hex')::uuid \
                 AND death_id=encode($3,'hex')::uuid AND collected_transaction_id IS NULL AND NOT expired",
            )
            .bind(&ring.item)
            .bind(fence.character_id.as_bytes().as_slice())
            .bind(death.as_slice())
            .fetch_one(&h.pool)
            .await?
        };
        assert_eq!(
            released,
            i64::from(ring.count),
            "the Count's ring was delivered for {variant}"
        );
        assert!(
            owner
                .root
                .read_pending_vampire_coffin(owner.character, fence.character_id)
                .await
                .map_err(debug)?
                .is_none(),
            "a defeated Count still has a redeemable garlic payment",
        );
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(std::time::Duration::from_secs(2))
            .ok_or("clock")?;
        let result = use_coffin(owner, admitted, weakened, 64).await?;
        assert_eq!(
            result,
            UseDisposition::Rejected,
            "coffin use after defeating its Count"
        );
        for (index, offering) in weakened.offerings.iter().enumerate() {
            fixture::offering(
                owner,
                fence.item_fence(),
                offering,
                90 + u64::try_from(index)?,
            )
            .await?;
        }
        owner.states.loop_ground.invalidate();
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(std::time::Duration::from_secs(2))
            .ok_or("clock")?;
        assert_eq!(
            use_coffin(owner, admitted, weakened, 65).await?,
            UseDisposition::Rejected,
            "fresh garlic cannot buy another Count after stage 8"
        );
        assert!(
            owner
                .runtime
                .lock()
                .await
                .visible_entities()
                .creatures
                .is_empty()
        );
    }
    let expected_burns = if foreign_generation || ordinary_restart {
        0
    } else {
        i64::try_from(weakened.offerings.len())?
    };
    assert_eq!(burns(h).await?, expected_burns);
    Ok(())
}

async fn use_coffin(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    weakened: &Callback,
    command_id: u64,
) -> TestResult<UseDisposition> {
    let Position { x, y, floor } = weakened.selector.position.ok_or("coffin position")?;
    let key = placement_key(TilePos { x, y, floor }, 1).ok_or("coffin placement")?;
    let map = owner.bundle_map().ok_or("coffin map")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(debug)?
        .ok_or("coffin fence")?;
    let result = owner
        .use_quest_callback(
            admitted.runtime_actor.ok_or("actor")?,
            UseCommand {
                game_session_id: admitted.game_session_id,
                command_id,
                item_fence: Some(fence.item_fence()),
            },
            map,
            &WorldObjectTarget {
                placement,
                expected_revision: map.facts.object_revision(key),
            },
        )
        .await
        .ok_or("committed coffin selector was not dispatched")?;
    Ok(result.disposition)
}

async fn burns(h: &Harness) -> TestResult<i64> {
    Ok(
        sqlx::query_scalar("SELECT count(*) FROM game_quest_callback_consumptions")
            .fetch_one(&h.pool)
            .await?,
    )
}

async fn defeat_count(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    count: crate::foundation::ExactActorRef,
) -> TestResult {
    use crate::foundation::{CommandId, CommandRef, OwnerDamageCommand};
    use crate::gameplay_transport::kill_reward::{DrainOutcome, KillSink, record_creature_kill};

    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    let actor = admitted.runtime_actor.ok_or("attacker")?;
    let session = admitted.game_session_id;
    let observed = owner.owner_now();
    let now = observed.get() / 1_000;
    let queue = owner.states.attack.lock().await.kills().clone();
    let mut runtime = owner.runtime.lock().await;
    let lease = runtime
        .bound_attacker_lease(actor, session)
        .map_err(debug)?;
    let snapshot = runtime.companion_snapshot(count).map_err(debug)?;
    let hit = runtime
        .borrow_exact_actor_commit_at(now)
        .commit_damage_for_bound_attacker(
            count,
            actor,
            CommandRef::new(session, CommandId::new(610).map_err(debug)?),
            0,
            OwnerDamageCommand {
                target: snapshot.state.policy.definition_key.as_bytes(),
                occurrence: session.as_bytes(),
                binding: b"inquisition-coffin-defeat",
                damage: snapshot.health,
            },
        )
        .map_err(debug)?;
    assert!(hit.applied);
    assert_eq!(hit.health_after, 0);
    record_creature_kill(
        &mut runtime,
        KillSink {
            prey: None,
            activity: &[],
            queue: &queue,
            rewards: native.reward_table(),
            charms: None,
        },
        count,
        crate::combat::CapturedRewardPrincipal {
            character: lease.character_id(),
            lease_generation: lease.generation(),
            session,
            actor,
        },
        now,
        Some(observed),
    );
    assert!(
        queue.has_session(session),
        "the Count's lethal hit must enter durable settlement",
    );
    drop(runtime);
    assert_eq!(
        Box::pin(owner.drain_kill_rewards(session)).await,
        DrainOutcome::Drained
    );

    return_to_coffin(owner, admitted).await
}

async fn return_to_coffin(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
) -> TestResult {
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let session = admitted.game_session_id;
    let mut runtime = owner.runtime.lock().await;
    let expected = runtime.read_actor_position(actor).map_err(debug)?;
    let map = owner.bundle_map().ok_or("coffin map")?;
    let doors = crate::map::floor::BundleDoors(map.facts.doors());
    let proof = crate::map::floor::prepare_quest_callback_teleport(
        &runtime,
        &owner.movement_cells,
        actor,
        session,
        expected,
        fixture::local(staging(RESTART)?),
        false,
        &doors,
    )
    .map_err(debug)?;
    runtime.commit_source_step(proof).map_err(debug)?;
    Ok(())
}
