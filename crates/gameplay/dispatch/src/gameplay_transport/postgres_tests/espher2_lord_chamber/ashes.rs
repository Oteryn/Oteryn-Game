use super::super::{ComposedFreshAdmission, fixture::Harness, item_look};
use super::{TestResult, debug};
use crate::durability::item_transfer::CurrentCharacterItemFence;
use crate::foundation::{CharacterId, CharacterLease, CommandId, CommandRef, ExactActorRef};
use crate::gameplay_transport::{
    connection::{FreshAdmissionAuthority, UseCommand},
    item_view::ItemKey,
    kill_reward::{DrainOutcome, KillSink},
};
use oteryn_protocol_oteryn::world_object::{MapUseTarget, UseDisposition, UseIntent, UseTarget};
use std::num::NonZeroU64;

const LORD: &str = "oteryn:creature.lord_of_the_elements";

pub(super) async fn exercise(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    players: [(ExactActorRef, CurrentCharacterItemFence); 2],
) -> TestResult {
    let map = owner.bundle_map().ok_or("chamber map")?;
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("gameplay")?;
    for (bag, (_, fence)) in [70, 75].into_iter().zip(players) {
        item_look::seed_character_main_backpack(
            h,
            *fence.character_id.as_bytes(),
            bag,
            &[("oteryn:item.tibia.i5908", 1)],
        )
        .await?;
    }

    let queue = owner.states.attack.lock().await.kills().clone();
    let observed = owner.owner_now();
    let (attacker, fence) = players[0];
    let session = fence.game_session_id;
    {
        let mut runtime = owner.runtime.lock().await;
        let target = *runtime
            .current_live_creature_census()
            .map_err(debug)?
            .first()
            .ok_or("summoned Lord")?;
        let health = runtime.companion_snapshot(target).map_err(debug)?.health;
        let center = runtime
            .read_actor_position(target)
            .map_err(debug)?
            .position();
        let closed = crate::map::floor::BundleDoors(&map.facts);
        for (offset, (actor, item_fence)) in [-1, 1].into_iter().zip(players) {
            let before = runtime.read_actor_position(actor).map_err(debug)?;
            let proof = crate::map::floor::prepare_quest_encounter_ejection(
                &runtime,
                &owner.movement_cells,
                actor,
                item_fence.game_session_id,
                before,
                crate::foundation::MovementLocalPosition {
                    x: center.x + offset,
                    ..center
                },
                &closed,
            )
            .map_err(debug)?;
            runtime.commit_source_step(proof).map_err(debug)?;
        }
        let lease = CharacterLease::new(
            CharacterId::decode(fence.character_id.as_bytes()).map_err(debug)?,
            fence.character_lease_generation,
        )
        .map_err(debug)?;
        runtime
            .bind_attacker_lease(attacker, session, lease)
            .map_err(debug)?;
        runtime
            .borrow_exact_actor_commit_at(observed.get() / 1000)
            .commit_damage_for_bound_attacker(
                target,
                attacker,
                CommandRef::new(session, CommandId::new(101).map_err(debug)?),
                0,
                crate::foundation::OwnerDamageCommand {
                    target: LORD.as_bytes(),
                    occurrence: session.as_bytes(),
                    binding: b"lord-chamber",
                    damage: health,
                },
            )
            .map_err(debug)?;
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
                actor: attacker,
            },
            observed.get() / 1000,
            Some(observed),
        );
    }
    assert!(
        queue.has_session(session),
        "the real death must reach kill settlement"
    );
    assert_eq!(
        owner.drain_kill_rewards(session).await,
        DrainOutcome::Drained
    );
    let visible = owner
        .runtime
        .lock()
        .await
        .visible_corpses()
        .first()
        .cloned()
        .ok_or("Lord's committed ashes")?;
    let corpse = visible.item.item_instance_id;
    assert!(
        owner
            .root
            .read_corpse_contents(owner.character, corpse)
            .await
            .map_err(debug)?
            .is_some(),
        "the original ashes expose loot before the deadline"
    );
    let target_handle = NonZeroU64::new(2).ok_or("target handle")?;
    let use_knife =
        |(actor, item_fence): (ExactActorRef, CurrentCharacterItemFence), bag: u8, command_id| {
            owner.use_craft(
                actor,
                UseCommand {
                    game_session_id: item_fence.game_session_id,
                    command_id,
                    item_fence: Some(item_fence),
                },
                UseIntent {
                    target: UseTarget::Item(NonZeroU64::MIN),
                    use_with: None,
                    use_with_map_item: Some(MapUseTarget::Item(target_handle)),
                    use_with_position: None,
                },
                Some(ItemKey::Instance(super::id(bag + 1))),
                Some(ItemKey::Entity(visible.identity)),
            )
        };
    assert_eq!(
        use_knife(players[0], 70, 200)
            .await
            .ok_or("original ashes USE dispatch")?
            .disposition,
        UseDisposition::Committed
    );

    let mut tx = h.pool.begin().await?;
    // Drive the existing timer with an aged test receipt, without sleeping ten seconds.
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    let schedules: Vec<(i64, i64)> = sqlx::query_as(
        "SELECT s.duration_millis,s.expires_at_unix_ms-r.materialized_at
         FROM game_spell_field_temporal_schedules s JOIN game_item_mint_receipts r USING(item_instance_id)
         WHERE s.item_instance_id=encode($1,'hex')::uuid ORDER BY s.source_stage",
    ).bind(corpse.as_slice()).fetch_all(&mut *tx).await?;
    assert_eq!(
        schedules,
        vec![(10_000, 10_000), (300_000, 310_000), (300_000, 610_000)]
    );
    let aged: i64 = sqlx::query_scalar(
        "SELECT floor(extract(epoch FROM clock_timestamp())*1000)::bigint - 11000",
    )
    .fetch_one(&mut *tx)
    .await?;
    sqlx::query("UPDATE game_item_mint_receipts SET materialized_at=$2 WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(corpse.as_slice()).bind(aged).execute(&mut *tx).await?;
    sqlx::query("UPDATE game_spell_field_temporal_schedules SET expires_at_unix_ms=$2+10000+300000*(source_stage-1) WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(corpse.as_slice()).bind(aged).execute(&mut *tx).await?;
    tx.commit().await?;
    assert!(
        owner
            .root
            .read_corpse_contents(owner.character, corpse)
            .await
            .map_err(debug)?
            .is_none(),
        "the ten-second deadline closes loot before the timer catches up"
    );
    owner.drain_source_item_deadlines().await;
    let body: String = sqlx::query_scalar("SELECT definition_production_key FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(corpse.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(body, "oteryn:item.tibia.i8182");

    for (index, (bag, player)) in [70, 75].into_iter().zip(players).enumerate() {
        let (_, item_fence) = player;
        assert_eq!(
            use_knife(player, bag, 201)
                .await
                .ok_or("ashes USE dispatch")?
                .disposition,
            UseDisposition::Committed
        );
        assert_eq!(
            use_knife(player, bag, 201)
                .await
                .ok_or("ashes USE replay")?
                .disposition,
            UseDisposition::Committed
        );
        assert_eq!(
            use_knife(player, bag, 202)
                .await
                .ok_or("ashes second USE")?
                .disposition,
            UseDisposition::NotPossible
        );
        let quantity: i64 = sqlx::query_scalar("SELECT coalesce(sum(i.quantity),0)::bigint FROM game_item_instances i JOIN game_item_container_entries e USING(item_instance_id) WHERE e.character_id=encode($1,'hex')::uuid AND i.definition_production_key='oteryn:item.tibia.i954'")
            .bind(item_fence.character_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(quantity, if index == 0 { 2 } else { 1 });
        let phases: Vec<i16> = sqlx::query_scalar(
            "SELECT phase FROM game_lord_ashes_claims
             WHERE character_id=encode($1,'hex')::uuid ORDER BY phase",
        )
        .bind(item_fence.character_id.as_bytes().as_slice())
        .fetch_all(&h.pool)
        .await?;
        assert_eq!(phases, if index == 0 { &[1, 2][..] } else { &[2][..] });
    }
    Ok(())
}
