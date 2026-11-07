//! Call the real one-second item-clock owner; wait for the real database deadline.
//! No clock replacement, deadline rewrite, raw deletion or WorldReset flag.
use super::*;
async fn carried(h: &Harness) -> TestResult<serde_json::Value> {
    Ok(sqlx::query_scalar("SELECT coalesce(jsonb_agg(jsonb_build_object('item',to_jsonb(i),'location',c.location,'kind',c.kind) ORDER BY i.item_instance_id),'[]'::jsonb) FROM game_quest_callback_carried(encode($1,'hex')::uuid,encode($2,'hex')::uuid)c JOIN game_item_instances i ON i.item_instance_id=c.item")
  .bind(id(41).as_slice()).bind(id(42).as_slice()).fetch_one(&h.pool).await?)
}
pub(super) async fn reclaim(
    h: &Harness,
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    all: bool,
) -> TestResult {
    let before = carried(h).await?;
    let began = std::time::Instant::now();
    loop {
        owner.drain_corpse_deadlines().await;
        let scope = owner.runtime.lock().await.binding().runtime_scope();
        let schedule = owner
            .root
            .read_scoped_corpse_decay_schedule(owner.character, scope)
            .await
            .map_err(debug)?;
        if schedule.corpses.is_empty() || (!all && schedule.corpses.len() < 56) {
            break;
        }
        assert!(
            schedule
                .corpses
                .iter()
                .all(|c| c.definition.production_key == "oteryn:item.tibia.i12325"),
            "only actual Stampor source bodies in isolated fixture"
        );
        let earliest = schedule
            .corpses
            .iter()
            .map(|c| c.decay_at_unix_ms)
            .min()
            .ok_or("live body deadline")?;
        let delay = u64::try_from((earliest - schedule.database_now_unix_ms).clamp(1, 20_000))?;
        assert!(
            began.elapsed() < std::time::Duration::from_secs(90),
            "real60s corpse owner did not reclaim source capacity"
        );
        eprintln!(
            "SRV7_CHONDUR_REAL_CORPSE_CLOCK_WAIT live={} wait_ms={delay} deadline={earliest} database_now={}",
            schedule.corpses.len(),
            schedule.database_now_unix_ms
        );
        tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
    }
    assert_eq!(
        carried(h).await?,
        before,
        "actual material custody/quantity survives conserved corpse retirement"
    );
    Ok(())
}
