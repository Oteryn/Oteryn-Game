//! Loaded mechanics death events retain their real corpse and optional one-unit loot.
use super::{
    TestResult,
    fixture::{Harness, debug},
};
use crate::content::bosses::activity_template::PinnedActivitySourceRoom;
use crate::durability::item_mint::BakragoreDeathLoot;
use crate::foundation::{CommandId, CommandRef, MovementLocalPosition, RuntimeScopeRefV1};
use crate::gameplay_transport::{ComposedFreshAdmission, connection::AdmittedSession};

#[test]
fn srx3_murcion_empty_copied_death_has_proven_uncredited_corpse_postgres() -> TestResult {
    super::srx3_rotten_advanced::copied_death(
        PinnedActivitySourceRoom::Murcion,
        MovementLocalPosition {
            x: 32974,
            y: 32365,
            floor: 15,
        },
        "oteryn:creature.murcion",
        false,
        false,
    )
}

#[test]
fn srx3_ichgahal_empty_copied_death_has_proven_uncredited_corpse_postgres() -> TestResult {
    super::srx3_rotten_advanced::copied_death(
        PinnedActivitySourceRoom::Ichgahal,
        MovementLocalPosition {
            x: 32978,
            y: 32333,
            floor: 15,
        },
        "oteryn:creature.ichgahal",
        false,
        false,
    )
}

#[test]
fn srx3_vemiath_copied_death_has_proven_uncredited_corpse_postgres() -> TestResult {
    super::srx3_rotten_advanced::copied_death(
        PinnedActivitySourceRoom::Vemiath,
        MovementLocalPosition {
            x: 33078,
            y: 32333,
            floor: 15,
        },
        "oteryn:creature.vemiath",
        false,
        false,
    )
}

#[test]
fn srx3_chagorz_copied_death_preserves_credited_corpse_owner_postgres() -> TestResult {
    super::srx3_rotten_advanced::copied_death(
        PinnedActivitySourceRoom::Chagorz,
        MovementLocalPosition {
            x: 33074,
            y: 32367,
            floor: 15,
        },
        "oteryn:creature.chagorz",
        true,
        false,
    )
}

#[test]
fn srx3_bakragore_uncredited_death_retains_actual_first_think_essence_postgres() -> TestResult {
    super::srx3_rotten_advanced::copied_death(
        PinnedActivitySourceRoom::Bakragore,
        MovementLocalPosition {
            x: 33010,
            y: 32392,
            floor: 15,
        },
        "oteryn:creature.bakragore",
        false,
        true,
    )
}

#[test]
fn srx3_bakragore_credited_death_retains_actual_first_think_essence_postgres() -> TestResult {
    super::srx3_rotten_advanced::copied_death(
        PinnedActivitySourceRoom::Bakragore,
        MovementLocalPosition {
            x: 33010,
            y: 32392,
            floor: 15,
        },
        "oteryn:creature.bakragore",
        true,
        true,
    )
}

pub(super) async fn exercise(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    key: &str,
    credited: bool,
    expected_essences: u8,
) -> TestResult {
    let fence = admitted.item_fence.ok_or("actual source death fence")?;
    let RuntimeScopeRefV1::Instance { instance_id, .. } = fence.runtime_scope else {
        return Err("actual source death Instance".into());
    };
    let registered = owner
        .activity_runtimes
        .get(fence.runtime_scope, fence.scope_ownership_generation.get())
        .ok_or("actual source death runtime")?;
    let view = owner.runtime_view(&registered);
    let native = view
        .active_generation
        .and_then(|active| active.native_gameplay())
        .ok_or("actual source death content")?;
    let player = admitted.runtime_actor.ok_or("actual source death Player")?;
    let mut captured = None;
    // As in the existing source-death Quest regression, drain only genuine older
    // work before choosing the lethal owner clock. Never advance a timer to zero.
    for _ in 0..8 {
        view.drain_monster_melee().await;
        let mut runtime = view.runtime.lock().await;
        let states = view.states.spell_states.try_lock().map_err(debug)?;
        let attack = view.states.attack.try_lock().map_err(debug)?;
        let now = view.owner_now();
        if crate::gameplay_transport::source_death::next_source_work(&runtime, &states, native)
            .map_err(debug)?
            .is_some_and(|(_, due)| due <= now.get())
            || runtime.has_pending_source_creature_deaths()
        {
            continue;
        }
        let boss = runtime
            .current_live_creature_census()
            .map_err(debug)?
            .into_iter()
            .find(|actor| runtime.matches_live_creature_identity(*actor, key.as_bytes()))
            .ok_or("actual native source boss")?;
        assert!(
            runtime
                .source_player_damage_shares(boss)
                .map_err(debug)?
                .is_empty(),
            "no previous damage attribution may supply a fabricated principal"
        );
        let before_clock = states.rotten_blood_callbacks.portal_clock_for_test(boss);
        let empty = matches!(key, "oteryn:creature.murcion" | "oteryn:creature.ichgahal");
        if !empty {
            assert!(
                before_clock
                    .as_ref()
                    .is_some_and(|clock| clock.cycle_due.is_some()),
                "actual retained original local teleport cycle"
            );
        }
        let health = runtime.companion_snapshot(boss).map_err(debug)?.health;
        let damage = crate::foundation::OwnerDamageCommand {
            target: key.as_bytes(),
            occurrence: b"srx3:rotten:copied-death",
            binding: b"srx3:rotten:copied-death\0current-native-owner",
            damage: health,
        };
        let hit = if credited {
            runtime
                .borrow_exact_actor_commit_at_us(now.get())
                .commit_damage_for_bound_attacker(
                    boss,
                    player,
                    CommandRef::new(
                        admitted.game_session_id,
                        CommandId::new(32601).map_err(debug)?,
                    ),
                    0,
                    damage,
                )
                .map_err(debug)?
        } else {
            runtime
                .borrow_exact_actor_commit_at_us(now.get())
                .commit_damage(boss, damage)
                .map_err(debug)?
        };
        assert!(
            hit.applied && hit.health_before > 0 && hit.health_after == 0,
            "real canonical lethal, never a health-zero setter"
        );
        if credited {
            let lease = runtime
                .bound_attacker_lease(player, admitted.game_session_id)
                .map_err(debug)?;
            let activity = attack.shared_experience_cells();
            crate::gameplay_transport::kill_reward::record_creature_kill(
                &mut runtime,
                crate::gameplay_transport::kill_reward::KillSink {
                    prey: Some(&states),
                    activity: &activity,
                    queue: attack.kills(),
                    rewards: native.reward_table(),
                    charms: Some(view.imported_charms),
                },
                boss,
                crate::combat::CapturedRewardPrincipal {
                    character: lease.character_id(),
                    lease_generation: lease.generation(),
                    session: admitted.game_session_id,
                    actor: player,
                },
                now.get() / 1000,
                Some(now),
            );
        } else {
            let receipt = runtime
                .borrow_combat_death()
                .committed_lethal_receipt(boss)
                .map_err(debug)?;
            runtime
                .borrow_combat_death()
                .project_committed_lethal(receipt)
                .map_err(debug)?;
        }
        let notice = runtime
            .next_source_creature_death()
            .map_err(debug)?
            .ok_or("the qualified loaded registry must enqueue a real death")?;
        assert_eq!(notice.actor(), boss);
        assert_eq!(notice.enqueued_at_us(), Some(now.get()));
        let begin = runtime
            .begin_source_creature_death(&notice)
            .map_err(debug)?;
        assert_eq!(begin.has_reward_damage_credit(), credited);
        assert_eq!(begin.events().len(), usize::from(!empty));
        if !empty {
            assert_eq!(
                begin.events()[0].registration_index(),
                1,
                "the unloaded RottenBloodBossDeath declaration does not renumber the copied callback"
            );
        }
        assert_eq!(attack.kills().has_source_drop(&begin), credited);
        let loot = if key == "oteryn:creature.bakragore" {
            let count = states
                .rotten_blood_callbacks
                .capture_bakragore_death_essences(&runtime, &begin)
                .map_err(debug)?;
            assert_eq!(
                count, expected_essences,
                "actual original FIRST Think storage88888"
            );
            let plan = BakragoreDeathLoot::from_current_death(&runtime, native, &begin, count)
                .map_err(debug)?;
            assert_eq!(
                BakragoreDeathLoot::from_current_death(&runtime, native, &begin, count)
                    .map_err(debug)?,
                plan,
                "the same death/index never resamples its draw"
            );
            Some(plan)
        } else {
            None
        };
        captured = Some((boss, begin, now, before_clock, loot));
        break;
    }
    let (boss, begin, now, before_clock, loot) =
        captured.ok_or("real older source work must quiesce before the lethal fixture")?;
    super::srx3_native_source_cast::drain_source_death(&view, boss, now).await?;
    if credited {
        let outcome = view.drain_kill_rewards(admitted.game_session_id).await;
        assert_eq!(
            outcome,
            crate::gameplay_transport::kill_reward::DrainOutcome::Drained
        );
    }
    {
        let runtime = view.runtime.lock().await;
        assert!(
            runtime
                .source_registered_creature_census()
                .map_err(debug)?
                .iter()
                .all(|(actor, _)| *actor != boss)
        );
        assert!(
            !runtime.corpse_projection_pinned(boss),
            "physical ACK releases the exact pin"
        );
        let states = view.states.spell_states.lock().await;
        assert!(states.source_death_corpse.is_none());
        assert!(
            states
                .rotten_blood_callbacks
                .pending_bakragore_death_loot(&begin)
                .map_err(debug)?
                .is_none()
        );
        if let Some(before_clock) = before_clock {
            let after_clock = states
                .rotten_blood_callbacks
                .portal_clock_for_test(boss)
                .ok_or(
                    "local bossStates timer must not be confused with GlobalRottenBloodStates",
                )?;
            assert_eq!(
                after_clock.cycle_due, before_clock.cycle_due,
                "literal mechanics death does not cancel the independently retained local90s callback"
            );
            assert_eq!(after_clock.think_sequence, before_clock.think_sequence);
        }
    }
    let rows = receipts(h, instance_id, begin.death_key()).await?;
    let selected = loot.as_ref().and_then(BakragoreDeathLoot::selected_item);
    assert_eq!(
        rows.len(),
        1 + usize::from(selected.is_some()),
        "one body and only the actual copied handler's optional one-unit child"
    );
    let body = rows
        .iter()
        .find(|row| row.0 == "CORPSE_MATERIALIZATION")
        .ok_or("actual body MINT receipt")?;
    assert_eq!(
        body.5,
        native
            .reward_table()
            .row(key)
            .map_err(debug)?
            .corpse_item
            .as_ref()
            .ok_or("actual native body definition")?
            .production_key
    );
    assert_eq!(body.6, 1);
    assert!(body.7.is_empty());
    let witness: (i64, bool) = sqlx::query_as(
        "SELECT count(w.transaction_id),coalesce(bool_and(\
           w.item_instance_id=r.item_instance_id AND w.created_xact_id=r.created_xact_id \
           AND r.corpse_top_damage_character_id IS NULL) FILTER(WHERE w.transaction_id IS NOT NULL),true) \
         FROM game_item_mint_receipts r LEFT JOIN game_source_corpse_mints w USING(transaction_id) \
         WHERE r.transaction_id=$1::uuid",
    )
    .bind(&body.2)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(witness, (i64::from(!credited), true));
    if !credited {
        refuses_unwitnessed_corpse(h, &body.2).await?;
    }
    if let Some(item) = selected {
        let child = rows
            .iter()
            .find(|row| row.0 == item.production_key)
            .ok_or("actual selected child MINT")?;
        assert_eq!(child.5, item.production_key);
        assert_eq!(child.6, 1);
        assert_eq!(
            child.7, body.3,
            "physical child belongs to this exact corpse"
        );
        assert_eq!(
            child.8, body.8,
            "corpse and conditional child commit in the same SQL transaction"
        );
    }
    let key = crate::durability::party_kill_xp::encode_death_key(begin.death_key());
    let xp: i64 =
        sqlx::query_scalar("SELECT count(*) FROM game_party_kill_xp_receipts WHERE death_key=$1")
            .bind(key)
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(
        xp,
        i64::from(credited),
        "no invented reward principal for uncredited source damage"
    );
    view.drain_monster_melee_at(now).await;
    if credited {
        let _ = view.drain_kill_rewards(admitted.game_session_id).await;
    }
    assert_eq!(
        receipts(h, instance_id, begin.death_key()).await?,
        rows,
        "replayed source and physical owners retain event, transaction, Item and intent identities"
    );
    Ok(())
}

type MintReceipt = (
    String,
    String,
    String,
    String,
    String,
    String,
    i64,
    String,
    String,
);

async fn receipts(
    h: &Harness,
    instance: [u8; 16],
    death: crate::foundation::CreatureDeathOccurrenceKey,
) -> TestResult<Vec<MintReceipt>> {
    Ok(sqlx::query_as("SELECT r.loot_purpose_key,r.event_id::text,r.transaction_id::text,r.item_instance_id::text,encode(r.intent_binding,'hex'),i.definition_production_key,i.quantity::bigint,coalesce(c.parent_item_instance_id::text,''),r.xmin::text FROM game_item_mint_receipts r JOIN game_item_instances i USING(item_instance_id) LEFT JOIN game_item_corpse_container_entries c USING(item_instance_id) WHERE r.death_scope_kind=2 AND r.death_instance_id=encode($1,'hex')::uuid AND r.death_scope_ownership_generation=$2::text::numeric(20,0) AND r.death_actor_local_id=$3 AND r.death_actor_local_generation=$4::text::numeric(20,0) ORDER BY r.loot_purpose_key,r.event_id")
        .bind(instance.as_slice()).bind(death.scope_ownership_generation().get().to_string())
        .bind(i64::from(death.actor_local_id())).bind(death.actor_local_generation().to_string())
        .fetch_all(&h.pool).await?)
}

/// Attempt a fresh NULL-principal corpse with the exact physical shape of the
/// real death above, but no sealed completion witness. Force only the new
/// reciprocal constraint, before the independent MINT/audit guards, and roll
/// back. An unconditional relaxation of the0013 CHECK would accept this pass.
async fn refuses_unwitnessed_corpse(h: &Harness, original: &str) -> TestResult {
    let item = crate::operator_files::uuid_v7().map_err(debug)?;
    let transaction = crate::operator_files::uuid_v7().map_err(debug)?;
    let event = crate::operator_files::uuid_v7().map_err(debug)?;
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
        .execute(&mut *tx)
        .await?;
    // These are deliberately attempted forgeries, never fixture admission or
    // an alternative source writer. Initial Item insertion is deferred just as
    // in the ordinary MINT transaction; all rows below are rolled back.
    sqlx::query(
        "INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,\
          definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) \
         SELECT encode($2,'hex')::uuid,i.world_id,i.definition_family,i.definition_production_key,\
          i.definition_revision_ref,1,1,encode($3,'hex')::uuid \
         FROM game_item_instances i JOIN game_item_mint_receipts r USING(item_instance_id) \
         WHERE r.transaction_id=$1::uuid",
    )
    .bind(original)
    .bind(item.as_slice())
    .bind(transaction.as_slice())
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "INSERT INTO game_item_mint_receipts(death_world_id,death_channel_id,\
          death_scope_ownership_generation,death_actor_local_id,death_actor_local_generation,\
          loot_table_family,loot_table_production_key,loot_table_revision_ref,loot_purpose_key,\
          draw_ordinal,intent_binding,transaction_id,event_id,item_instance_id,occurred_at,\
          envelope_sha256,committed_at,corpse_top_damage_character_id,death_instance_id,\
          death_scope_kind,death_scope_key) \
         SELECT death_world_id,death_channel_id,death_scope_ownership_generation,death_actor_local_id,\
          death_actor_local_generation+1,loot_table_family,loot_table_production_key,loot_table_revision_ref,\
          loot_purpose_key,draw_ordinal,intent_binding,encode($2,'hex')::uuid,encode($3,'hex')::uuid,\
          encode($4,'hex')::uuid,occurred_at,envelope_sha256,committed_at,NULL,death_instance_id,\
          death_scope_kind,death_scope_key FROM game_item_mint_receipts WHERE transaction_id=$1::uuid",
    )
    .bind(original)
    .bind(transaction.as_slice())
    .bind(event.as_slice())
    .bind(item.as_slice())
    .execute(&mut *tx)
    .await?;
    let error = sqlx::query("SET CONSTRAINTS game_source_corpse_mint_proven IMMEDIATE")
        .execute(&mut *tx)
        .await
        .err()
        .ok_or("an unwitnessed no-principal corpse passed its reciprocal guard")?;
    let database = error
        .as_database_error()
        .ok_or("source corpse refusal must come from the database constraint")?;
    assert_eq!(database.code().as_deref(), Some("23514"));
    assert!(
        database
            .message()
            .contains("no-principal corpse requires its exact source completion witness")
    );
    tx.rollback().await?;
    let leftovers: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid",
    )
    .bind(item.as_slice())
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(leftovers, 0, "refused body must not leave physical custody");
    Ok(())
}
