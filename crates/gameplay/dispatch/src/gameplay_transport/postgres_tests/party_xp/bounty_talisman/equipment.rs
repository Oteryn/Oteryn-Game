//! An EXTRA move must not change the qualification of an earlier queued death.
use super::*;
use crate::gameplay_transport::connection::UseCommand;
use crate::gameplay_transport::item_move::{EquipmentMove, EquipmentMoveResult};
use crate::gameplay_transport::item_view::ItemKey;
use oteryn_protocol_oteryn::item_view::EquipmentSlot;

const UNEQUIP_COMMAND: u64 = 2_000_001;
const EQUIP_COMMAND: u64 = 2_000_002;

/// Historical custody alone is provisioned. Both moves below must qualify the
/// real admitted native definitions, slot policy, backpack and current fence.
pub(super) async fn prepare(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    character: crate::domain::CharacterId,
    talisman: [u8; 16],
) -> TestResult {
    let native = owner
        .active_generation
        .and_then(|active| active.native_gameplay())
        .ok_or("native gameplay")?;
    let talisman_policy = native
        .item_policy_for_source_id(51978)
        .ok_or("production bounty talisman item policy")?;
    let backpack_policy = native
        .item_policy_for_source_id(2854)
        .ok_or("production backpack item policy")?;
    let talisman_definition = &talisman_policy.record().production_definition;
    let backpack_definition = &backpack_policy.record().production_definition;
    let backpack = crate::bestiary_postgres_harness::id(123);
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE game_item_instances SET definition_production_key=$2,definition_revision_ref=$3 WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(talisman.as_slice()).bind(&talisman_definition.production_key)
        .bind(&talisman_definition.revision_ref).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item',$3,$4,1,1,encode($1,'hex')::uuid)")
        .bind(backpack.as_slice()).bind(owner.world_id.as_bytes().as_slice())
        .bind(&backpack_definition.production_key).bind(&backpack_definition.revision_ref)
        .execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_item_container_slots(character_id,world_id,item_instance_id,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($3,'hex')::uuid)")
        .bind(character.as_bytes().as_slice()).bind(owner.world_id.as_bytes().as_slice())
        .bind(backpack.as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

async fn move_extra(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    session: GameSessionId,
    equip: bool,
) -> TestResult<EquipmentMoveResult> {
    let fence = owner
        .current_quest_fence(session)
        .await
        .map_err(|()| "fence read")?
        .ok_or("current fence")?;
    let item = ItemKey::Instance(crate::bestiary_postgres_harness::id(119));
    let request = if equip {
        EquipmentMove::Equip {
            item,
            to: EquipmentSlot::Ammo,
        }
    } else {
        EquipmentMove::Unequip {
            item,
            from: EquipmentSlot::Ammo,
        }
    };
    Ok(crate::gameplay_transport::equipment_move::move_equipment(
        owner,
        actor,
        UseCommand {
            game_session_id: session,
            command_id: if equip {
                EQUIP_COMMAND
            } else {
                UNEQUIP_COMMAND
            },
            item_fence: Some(fence.item_fence()),
        },
        request,
    )
    .await)
}

async fn extra_is_equipped(h: &Harness) -> TestResult<bool> {
    Ok(sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_character_equipment_slots WHERE item_instance_id=encode($1,'hex')::uuid AND slot=10)")
        .bind(crate::bestiary_postgres_harness::id(119).as_slice()).fetch_one(&h.pool).await?)
}

pub(super) async fn unequip_after_queued_deaths(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    actor: ExactActorRef,
    session: GameSessionId,
) -> TestResult {
    let entry = super::super::capture_pending(owner, &[(actor, session)]).await?;
    assert!(
        entry.captured_talisman().is_none(),
        "the death has not reached its first qualification"
    );
    sqlx::query("CREATE FUNCTION talisman_equipment_retry() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'TALISMAN_EQUIPMENT_RETRY'; END $$")
        .execute(&h.pool).await?;
    sqlx::query("CREATE TRIGGER talisman_equipment_retry BEFORE INSERT ON game_character_bestiary_kill_receipts FOR EACH ROW EXECUTE FUNCTION talisman_equipment_retry()")
        .execute(&h.pool).await?;
    let attempted = move_extra(owner, actor, session, false).await;
    sqlx::query("DROP TRIGGER talisman_equipment_retry ON game_character_bestiary_kill_receipts")
        .execute(&h.pool)
        .await?;
    sqlx::query("DROP FUNCTION talisman_equipment_retry()")
        .execute(&h.pool)
        .await?;
    assert_eq!(
        attempted?,
        EquipmentMoveResult::Unknown,
        "unresolved earlier rewards keep the equipment command retryable"
    );
    assert!(
        extra_is_equipped(h).await?,
        "an unresolved kill cannot unequip its talisman"
    );
    assert_eq!(sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_character_equipment_receipts WHERE command_id=$1::text::numeric")
        .bind(UNEQUIP_COMMAND.to_string()).fetch_one(&h.pool).await?, 0);
    owner.root.maintain_ready_once().await?;
    assert_eq!(
        move_extra(owner, actor, session, false).await?,
        EquipmentMoveResult::Committed
    );
    assert!(!extra_is_equipped(h).await?);
    assert_eq!(
        owner.states.attack.lock().await.kills().counts().0,
        0,
        "the same equipment command settles both earlier deaths before changing EXTRA"
    );
    let bonuses = entry
        .captured_talisman()
        .ok_or("frozen death qualification")?
        .bonuses("oteryn:creature.rat");
    assert_eq!(
        (bonuses[2], bonuses[3]),
        (5000, 10000),
        "completing-kill loot and Bestiary retain the equipment at death"
    );
    assert_eq!(
        move_extra(owner, actor, session, false).await?,
        EquipmentMoveResult::Committed
    );
    assert_eq!(sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_character_equipment_receipts WHERE command_id=$1::text::numeric")
        .bind(UNEQUIP_COMMAND.to_string()).fetch_one(&h.pool).await?, 1,
        "retry and replay create only one equipment move");
    println!(
        "<BOUNTY_TALISMAN> POSTGRES_CASE_COMPLETED: queued_deaths_unequip_retry_and_frozen_final_loot"
    );
    Ok(())
}

pub(super) async fn equip_after_unworn_death(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    actor: ExactActorRef,
    session: GameSessionId,
) -> TestResult {
    assert!(!extra_is_equipped(h).await?);
    let fence = owner
        .current_quest_fence(session)
        .await
        .map_err(|()| "fence read")?
        .ok_or("current fence")?;
    // A later active bounty is historical input; its subsequent death and equipment
    // mutation use the production owners, including the current native slot policy.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE game_character_task_boards SET body=jsonb_set(jsonb_set(body,'{bounty,state}','\"Active\"'::jsonb),'{bounty,active,current}','0'::jsonb) WHERE character_id=encode($1,'hex')::uuid")
        .bind(fence.character_id.as_bytes().as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    // Reuse the fixture's normal respawn owner after its two original creatures
    // died; no invented death receipt or client-supplied bonus enters this case.
    for _ in 0..3 {
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(Duration::from_secs(65))
            .ok_or("clock")?;
        owner
            .runtime
            .lock()
            .await
            .drive_spawn_respawns(owner.owner_now().get(), |_, _| false)
            .map_err(debug)?;
    }
    let target = {
        let mut runtime = owner.runtime.lock().await;
        let row = runtime
            .visible_entities()
            .creatures
            .into_iter()
            .find(|row| {
                runtime.companion_snapshot(row.actor).is_ok_and(|snapshot| {
                    snapshot.state.policy.definition_key == "oteryn:creature.rat"
                })
            })
            .ok_or("respawned rat")?;
        runtime
            .set_test_creature_health(row.actor, 1)
            .map_err(debug)?;
        assert_eq!(
            owner.states.attack.lock().await.set_target(
                &runtime,
                owner.rooms.gate_auto_attack(),
                actor,
                session,
                owner.owner_now(),
                Some(oteryn_protocol_oteryn::world_spatial_entities::EntityRef {
                    identity: row.actor.placement_identity(),
                    generation: row.generation,
                }),
                3_000_001
            ),
            oteryn_protocol_oteryn::attack::AttackIntentDisposition::Ok
        );
        row.actor
    };
    for pass in 0..256 {
        owner.refresh_attack_weapon(actor, session).await;
        owner.drain_auto_attacks().await;
        if owner.states.attack.lock().await.kills().counts().0 == 1 {
            break;
        }
        assert!(
            pass < 255,
            "the unworn real kill must enter the pending queue"
        );
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(Duration::from_secs(3))
            .ok_or("clock")?;
    }
    let entry = super::super::capture_pending(owner, &[(actor, session)]).await?;
    assert_eq!(entry.target, target);
    assert!(entry.captured_talisman().is_none());
    assert_eq!(
        move_extra(owner, actor, session, true).await?,
        EquipmentMoveResult::Committed
    );
    assert!(extra_is_equipped(h).await?);
    assert_eq!(owner.states.attack.lock().await.kills().counts().0, 0);
    assert_eq!(
        entry
            .captured_talisman()
            .ok_or("unworn death qualification")?
            .bonuses("oteryn:creature.rat"),
        [0; 4],
        "equipping after a queued death cannot qualify any earlier talisman effect"
    );
    let increment: i16 = sqlx::query_scalar("SELECT kill_count_increment FROM game_character_bestiary_kill_receipts WHERE character_id=encode($1,'hex')::uuid AND bestiary_occurrence_id=encode($2,'hex')::uuid")
        .bind(fence.character_id.as_bytes().as_slice()).bind(entry.facts.occurrence.as_slice())
        .fetch_one(&h.pool).await?;
    assert_eq!(
        increment, 1,
        "the real death preceded equipping its talisman"
    );
    assert_eq!(
        move_extra(owner, actor, session, true).await?,
        EquipmentMoveResult::Committed
    );
    assert_eq!(sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_character_equipment_receipts WHERE command_id=$1::text::numeric")
        .bind(EQUIP_COMMAND.to_string()).fetch_one(&h.pool).await?, 1);
    println!(
        "<BOUNTY_TALISMAN> POSTGRES_CASE_COMPLETED: queued_unworn_death_precedes_real_talisman_equip"
    );
    Ok(())
}
