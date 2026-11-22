//! Bind dispatch journey fixtures to a specific save before they send USE or STEP.
use super::*;

pub(super) async fn seed(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    key: &str,
    in_season: bool,
) -> TestResult {
    let world = owner.world_id;
    let (month, day) = match (key, in_season) {
        ("tinder_box", true) => (5, 1),
        ("last_creep_standing", true) => (5, 20),
        _ => (6, 1),
    };
    let pin = owner.runtime.lock().await.content_pin().clone();
    let map = owner.bundle_map().ok_or("seasonal callback map")?;
    sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,1,$2,$6,$3,$4,$5,'TIBIA_SERVER_SAVE_BERLIN_06',1,make_timestamptz(2026,$7,$8,6,0,0,'Europe/Berlin'),to_char(make_date(2026,$7,$8),'YYYY-MM'))")
        .bind(world.as_bytes().as_slice())
        .bind(map.overlay.base().digest().as_slice())
        .bind(pin.server_artifact_digest().as_slice())
        .bind(pin.client_artifact_digest().as_slice())
        .bind(pin.frame_binding_digest().as_slice())
        .bind(crate::map::overlay::map_revision(map.overlay.base()))
        .bind(i32::from(month)).bind(i32::from(day))
        .execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_world_event_epochs(world_id,reset_epoch,active,announcement) VALUES(encode($1,'hex')::uuid,1,false,'')")
        .bind(world.as_bytes().as_slice()).execute(&h.pool).await?;
    for event in
        oteryn_gameplay_world_change::seasonal::at_save(world.as_bytes(), 1, 2026, month, day)
    {
        sqlx::query("INSERT INTO game_world_seasonal_occurrences(world_id,reset_epoch,event_key,occurrence_id) VALUES(encode($1,'hex')::uuid,1,$2,$3)")
            .bind(world.as_bytes().as_slice()).bind(event.key).bind(event.id.as_slice())
            .execute(&h.pool).await?;
    }
    sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,1)")
        .bind(world.as_bytes().as_slice())
        .execute(&h.pool)
        .await?;
    owner.reset_epoch = 1;
    Ok(())
}
