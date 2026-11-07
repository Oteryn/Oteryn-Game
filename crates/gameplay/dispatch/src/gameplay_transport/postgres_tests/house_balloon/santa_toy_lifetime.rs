//! W05d source default durations start on the individually moved toy only.
use super::*;

fn target(source_id: u32) -> TestResult<&'static str> {
    match source_id {
        6566 => Ok("oteryn:item.tibia.i5791"),
        6567 => Ok("oteryn:item.tibia.i6511"),
        6568 => Ok("oteryn:item.tibia.i5080"),
        _ => Err("not a closed source-qualified Santa toy".into()),
    }
}

pub(super) async fn before_first_move(h: &Harness, source: [u8; 16], source_id: u32) -> TestResult {
    target(source_id)?;
    let row:(String,i64,i64)=sqlx::query_as("SELECT definition_production_key,(SELECT count(*) FROM game_spell_item_temporal_schedules WHERE item_instance_id=i.item_instance_id),(SELECT count(*) FROM game_item_timed_states WHERE item_instance_id=i.item_instance_id) FROM game_item_instances i WHERE item_instance_id=encode($1,'hex')::uuid")
  .bind(source.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        row,
        (format!("oteryn:item.tibia.i{source_id}"), 0, 0),
        "unmoved source gift has default duration, without a running clock"
    );
    Ok(())
}

pub(super) async fn after_first_move(
    h: &Harness,
    source: [u8; 16],
    source_id: u32,
) -> TestResult<i64> {
    let target = target(source_id)?;
    let timer:Option<(String,String,i64,i64,bool)>=sqlx::query_as("SELECT p.definition_production_key,p.target_production_key,p.duration_millis,p.expires_at_unix_ms,r.created_xact_id=p.created_xact_id AND r.source_item_instance_id=p.item_instance_id AND p.expires_at_unix_ms>=r.occurred_at+3000 FROM game_spell_item_temporal_schedules p JOIN game_item_transfer_receipts r ON r.transaction_id=p.source_transaction_id WHERE p.item_instance_id=encode($1,'hex')::uuid")
  .bind(source.as_slice()).fetch_optional(&h.pool).await?;
    let timer=timer.ok_or("first actual individual House Place must start the source-qualified three-second toy clock")?;
    assert_eq!(
        (&timer.0, &timer.1, timer.2, timer.4),
        (
            &format!("oteryn:item.tibia.i{source_id}"),
            &target.to_string(),
            3000,
            true
        ),
        "actual same-transaction movement starts the exact source clock"
    );
    Ok(timer.3)
}

pub(super) async fn assert_replay_deadline(
    h: &Harness,
    source: [u8; 16],
    deadline: i64,
) -> TestResult {
    let actual:i64=sqlx::query_scalar("SELECT expires_at_unix_ms FROM game_spell_item_temporal_schedules WHERE item_instance_id=encode($1,'hex')::uuid")
  .bind(source.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        actual, deadline,
        "movement replay cannot restart the three-second source duration"
    );
    Ok(())
}

pub(super) async fn expire_on_house_owner(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    source: [u8; 16],
    source_id: u32,
) -> TestResult {
    let target = target(source_id)?;
    let actor = admitted
        .runtime_actor
        .ok_or("actual House toy clock actor")?;
    tokio::time::timeout(std::time::Duration::from_secs(10),async {
  loop {
   if owner.root.has_ready_demand(){owner.root.maintain_ready_once().await.map_err(debug)?;}
   owner.expire_house_furniture_on_tick(actor,admitted.game_session_id).await.ok_or("actual House absolute clock pass")?;
   let after:String=sqlx::query_scalar("SELECT definition_production_key FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid").bind(source.as_slice()).fetch_one(&h.pool).await?;
   if after==target {break;}
   tokio::time::sleep(std::time::Duration::from_millis(25)).await;
  }
  Ok::<_,Box<dyn std::error::Error>>(())
 }).await??;
    let retired:i64=sqlx::query_scalar("SELECT count(*) FROM game_house_furniture_absolute_expiries WHERE item_instance_id=encode($1,'hex')::uuid").bind(source.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        retired, 1,
        "same existing House clock settles exactly one source deadline"
    );
    let native = owner
        .active_generation
        .and_then(|a| a.native_gameplay())
        .ok_or("current native toy")?;
    let reference = native
        .item_index()
        .ok_or("current source index")?
        .definition_ref("Item", target, "definition-r1")
        .ok_or("expired base native reference")?;
    let body = encode_item_use_intent(&UseIntent {
        target: UseTarget::Item(std::num::NonZeroU64::MIN),
        use_with: None,
        use_with_map_item: None,
        use_with_position: None,
    })
    .map_err(debug)?;
    let (_, used, _, _) = exchange(
        owner,
        admitted,
        2,
        &body,
        HouseExchange {
            current: Some(reference),
            rotate: false,
            after: None,
            nested: None,
            foreign_after_snapshot: None,
            capture: None,
        },
    )
    .await?;
    assert_eq!(
        used,
        UseDisposition::Committed,
        "native House snapshot selects and USEs actual expired base on same UID"
    );
    Ok(())
}
