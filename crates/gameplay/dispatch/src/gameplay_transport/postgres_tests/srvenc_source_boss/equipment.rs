use super::*;
use super::super::fixture::id;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Case {
    Neither,
    Tick,
    Contended,
    Entry,
    Return,
    LostExpiry,
}

#[test]
fn periodic_activity_equipment_yields_when_admission_is_locked_postgres() -> TestResult {
    scenario_thread(Case::Contended)
}

#[test]
fn activity_entry_settles_partial_equipment_time_postgres() -> TestResult {
    scenario_thread(Case::Entry)
}

#[test]
fn activity_return_settles_partial_equipment_time_postgres() -> TestResult {
    scenario_thread(Case::Return)
}

#[test]
fn lost_activity_participant_can_settle_expiring_equipment_postgres() -> TestResult {
    scenario_thread(Case::LostExpiry)
}

pub(super) async fn settle_lost_expiry(
    h: &Harness,
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
) -> TestResult {
    use crate::gameplay_transport::connection::{ControlLossResult, FreshAdmissionAuthority};
    let actor = admitted.runtime_actor.ok_or("activity actor")?;
    let session = admitted.game_session_id;
    let ring = "oteryn:item.tibia.i3088";
    let revision = owner.active_generation.and_then(|active| active.native_gameplay())
        .and_then(|native| native.item_index()).and_then(|index| index.revision(ring))
        .ok_or("equipped energy ring definition")?;
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id)
        VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item',$3,$4,1,1,game_character_uuid_v7())")
        .bind(id(73).as_slice()).bind(id(42).as_slice()).bind(ring).bind(revision).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_character_equipment_slots(character_id,slot,item_instance_id,world_id,placed_transaction_id)
        VALUES(encode($1,'hex')::uuid,8,encode($2,'hex')::uuid,encode($3,'hex')::uuid,game_character_uuid_v7())")
        .bind(id(41).as_slice()).bind(id(73).as_slice()).bind(id(42).as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    let permit = owner.spell_lane_permit().await.ok_or("activity spell lane")?;
    assert!(owner.checkpoint_timed_equipment(actor, session, true).await);
    drop(permit);
    assert_eq!(
        owner.lose_control(admitted, std::time::Duration::ZERO, None).await,
        ControlLossResult::Recorded,
    );
    owner.clock_origin -= std::time::Duration::from_secs(601);
    let permit = owner.spell_lane_permit().await.ok_or("activity spell lane")?;
    assert!(owner.settle_activity_equipment(actor, session).await);
    drop(permit);
    let equipped: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM game_character_equipment_slots WHERE item_instance_id=encode($1,'hex')::uuid)")
        .bind(id(73).as_slice()).fetch_one(&h.pool).await?;
    assert!(!equipped, "the durable settlement retired the expired energy ring");
    Ok(())
}

fn scenario_thread(case: Case) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || scenario(case).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "activity equipment scenario panicked")??;
    Ok(())
}

pub(super) async fn activity_equipment_clocks_charge_worn_items(
    h: &Harness,
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    case: Case,
) -> TestResult {
    use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
    use crate::gameplay_transport::item_move::{EquipmentMove, EquipmentMoveResult};
    use crate::gameplay_transport::item_view::ItemKey;
    use oteryn_protocol_oteryn::item_view::EquipmentSlot;
    let actor = admitted.runtime_actor.ok_or("activity actor")?;
    let equip = owner.move_equipment(actor, UseCommand {
        game_session_id: admitted.game_session_id, command_id: 100,
        item_fence: admitted.item_fence,
    }, EquipmentMove::Equip { item: ItemKey::Instance(id(71)), to: EquipmentSlot::Feet }).await;
    assert!(!matches!(equip, EquipmentMoveResult::Unknown), "{equip:?}");
    let native = owner.active_generation.and_then(|active| active.native_gameplay()).ok_or("native content")?;
    let boots = "oteryn:item.tibia.i6529";
    let revision = native.item_index().and_then(|index| index.revision(boots)).ok_or("boots revision")?;
    sqlx::query("INSERT INTO game_type2_audit_activation VALUES(1,now()) ON CONFLICT DO NOTHING")
        .execute(&h.pool).await?;
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_item_imbuements(item_instance_id,slot,category,imbuement_key,imbuement_revision,tier,remaining_ms,applying_occurrence_id,applying_transaction_id) VALUES(encode($1,'hex')::uuid,0,8,'oteryn:imbuement.featherweight','r1',1,72000000,game_character_uuid_v7(),game_character_uuid_v7())")
        .bind(id(70).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id)
        VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item',$3,$4,1,1,game_character_uuid_v7())")
        .bind(id(72).as_slice()).bind(id(42).as_slice()).bind(boots).bind(revision).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_character_equipment_slots(character_id,slot,item_instance_id,world_id,placed_transaction_id)
        VALUES(encode($1,'hex')::uuid,7,encode($2,'hex')::uuid,encode($3,'hex')::uuid,game_character_uuid_v7())")
        .bind(id(41).as_slice()).bind(id(72).as_slice()).bind(id(42).as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    let permit = owner.spell_lane_permit().await.ok_or("activity spell lane")?;
    assert!(owner.checkpoint_timed_equipment(actor, admitted.game_session_id, true).await);
    drop(permit);
    owner.clock_origin -= std::time::Duration::from_secs(61);
    if case == Case::Contended {
        owner.quest_catalogue = Some(std::sync::Arc::new(
            oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
                .map_err(debug)?.catalogue().clone(),
        ));
        let mut lock = h.pool.begin().await?;
        sqlx::query("LOCK TABLE game_durability_admission_account_guards IN EXCLUSIVE MODE")
            .execute(&mut *lock).await?;
        tokio::time::timeout(std::time::Duration::from_millis(500), owner.tick_timed_items())
            .await.map_err(|_| "periodic activity equipment checkpoint waited on the admission writer")?;
        lock.rollback().await?;
        owner.clock_origin -= std::time::Duration::from_secs(1);
    }
    owner.tick_timed_items().await;
    let (boots_remaining, remaining) = remaining_budgets(h).await?;
    assert!(remaining < 72_000_000, "a worn imbuement must tick in an activity");
    assert!(boots_remaining < 14_400_000, "timed equipment must tick in an activity");
    Ok(())
}

pub(super) async fn remaining_budgets(h: &Harness) -> TestResult<(i64, i64)> {
    let remaining: i64 = sqlx::query_scalar("SELECT remaining_ms FROM game_item_imbuements WHERE item_instance_id=encode($1,'hex')::uuid AND slot=0")
        .bind(id(70).as_slice()).fetch_one(&h.pool).await?;
    let boots_remaining: i64 = sqlx::query_scalar("SELECT remaining_ms FROM game_item_timed_states WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(id(72).as_slice()).fetch_one(&h.pool).await?;
    Ok((boots_remaining, remaining))
}


pub(super) async fn assert_elapsed(
    h: &Harness,
    before: (i64, i64),
    since: std::time::Instant,
    advanced_ms: i64,
) -> TestResult<(i64, i64)> {
    let remaining = remaining_budgets(h).await?;
    let maximum = advanced_ms + i64::try_from(since.elapsed().as_millis())? + 1;
    for (item, debit) in [("boots", before.0 - remaining.0), ("imbuement", before.1 - remaining.1)] {
        assert!((advanced_ms..=maximum).contains(&debit), "{item} charged {debit} ms outside {advanced_ms}..={maximum}");
    }
    Ok(remaining)
}
