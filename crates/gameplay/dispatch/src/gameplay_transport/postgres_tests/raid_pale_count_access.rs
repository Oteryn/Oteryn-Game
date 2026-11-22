use super::fixture::{Harness, debug, id};
use super::{Dependencies, TestResult};
use crate::foundation::MovementLocalPosition;
use crate::gameplay_transport::{ComposedFreshAdmission, raid_live::RaidEffectProgress};
use oteryn_gameplay_encounter_rules::raid::RaidCatalogue;
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_protocol_oteryn::world_spatial::{StepDirection, StepDisposition};

const EXIT: MovementLocalPosition = MovementLocalPosition {
    x: 32967,
    y: 32420,
    floor: 14,
};
const ENTRY: MovementLocalPosition = MovementLocalPosition {
    x: 32968,
    y: 32420,
    floor: 14,
};

#[test]
fn pale_count_raid_gates_entry_and_ejects_players_fifteen_minutes_after_the_warning() -> TestResult
{
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e_fixtures::scenario_thread::run("pale-count-access", move || {
        crate::bestiary_postgres_harness::runtime()?.block_on(async {
            let h = Harness::create_with_admission_seed(admin, "pale_count_access", false, false)
                .await?;
            let result = exercise(&h).await;
            let cleanup = h.cleanup().await;
            result?;
            cleanup
        })
    })?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED pale_count_raid_gates_entry_and_ejects_players_fifteen_minutes_after_the_warning"
    );
    Ok(())
}

async fn exercise(h: &Harness) -> TestResult {
    let scene = super::srx3_soul_war_world_fixture_fixtures::source_world(EXIT)?;
    let deps = Dependencies::with_content_and_capacity(
        scene.pin.clone(),
        scene.room.clone(),
        crate::spell::cast::v1_spell_book().map_err(debug)?,
        16,
    )?;
    let evidence = super::house_transfer_fixtures::RecoverySource::start(h).await?;
    super::house_transfer_fixtures::prepare_owned_session(h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let mut owner = deps.owner(h, &authority);
    owner.evidence = &evidence.evidence;
    owner.active_generation = scene.controller.active();
    owner.rooms = crate::gameplay_transport::SeamRooms::new(Some(&scene.room), Some(&scene.room));
    owner.reset_epoch = 1;
    owner
        .active_generation
        .and_then(|active| active.native_gameplay())
        .ok_or("native policies")?
        .install_companion_policies(&mut *owner.runtime.lock().await)
        .map_err(debug)?;
    let mut admitted = deps.admitted;
    super::srx3_activity_source_fixture::prepare_player(h, &owner, &mut admitted, 1999).await?;

    let raw: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../../content/encounters/raids/catalogue.json"
    ))?;
    let key = crate::content::ProductionKey::new("raid:darashia.thepalecount").map_err(debug)?;
    let mut source = raw["records"]
        .as_array()
        .ok_or("raid records")?
        .iter()
        .find(|record| record["key"] == key.as_str())
        .ok_or("Pale Count record")?
        .clone();
    source["active"] = serde_json::json!(true);
    source["inactive_reasons"] = serde_json::json!([]);
    source["end"] = serde_json::json!({"kind": "despawn_after", "despawn_after_ms": 900_000});
    // Use a bounded test chamber on the real crypt map next to the sourced boss
    // position. The production content lane supplies its full lair geometry.
    source["pale_count_lair"] = serde_json::json!({
        "from": {"x": ENTRY.x, "y": ENTRY.y, "floor": ENTRY.floor},
        "to": {"x": 32970, "y": 32420, "floor": 15},
        "exit": {"x": EXIT.x, "y": EXIT.y, "floor": EXIT.floor}
    });
    let catalogue = RaidCatalogue::from_json(
        &serde_json::json!({
            "schema": raw["schema"], "records": [source],
        })
        .to_string(),
    )
    .map_err(debug)?;
    let firing = id(91);
    let now = i64::try_from(crate::world_clock::now_utc_ms().ok_or("UTC clock")?)?;
    // The clock is four-and-a-half minutes past the warning. Poll the owner
    // at its earlier instants explicitly to qualify the delivery predecessor.
    let fired_at = now - 271_000;
    let closes_at = fired_at + 901_000;
    let map = owner.bundle_map().ok_or("current map")?;
    let digest = map.overlay.base().digest();
    seed(
        h,
        &owner,
        key.as_str(),
        firing,
        digest,
        (fired_at, closes_at),
    )
    .await?;

    let mut progress = RaidEffectProgress::default();
    owner
        .drive_raid_firing(
            firing,
            &key,
            &catalogue,
            digest,
            fired_at + 999,
            &mut progress,
        )
        .await
        .map_err(|_| "Pale Count before warning")?;
    let actor = admitted.runtime_actor.ok_or("current player")?;
    let session = admitted.game_session_id;
    let walk = || owner.ordinary_paced_step(actor, session, StepDirection::East, None, None);
    assert_eq!(walk().await.0.disposition, StepDisposition::Blocked);
    owner
        .drive_raid_firing(
            firing,
            &key,
            &catalogue,
            digest,
            fired_at + 1_000,
            &mut progress,
        )
        .await
        .map_err(|_| "Pale Count announcement")?;
    assert_eq!(walk().await.0.disposition, StepDisposition::Blocked);
    owner
        .drive_raid_firing(
            firing,
            &key,
            &catalogue,
            digest,
            fired_at + 241_000,
            &mut progress,
        )
        .await
        .map_err(|_| "Pale Count four-minute opening")?;
    assert_eq!(walk().await.0.disposition, StepDisposition::Moved);
    assert_eq!(
        owner
            .runtime
            .lock()
            .await
            .read_actor_position(actor)
            .map_err(debug)?
            .position(),
        ENTRY
    );
    owner
        .drive_raid_firing(
            firing,
            &key,
            &catalogue,
            digest,
            closes_at - 1,
            &mut progress,
        )
        .await
        .map_err(|_| "Pale Count before timeout")?;
    assert_eq!(
        owner
            .runtime
            .lock()
            .await
            .read_actor_position(actor)
            .map_err(debug)?
            .position(),
        ENTRY
    );
    owner
        .drive_raid_firing(firing, &key, &catalogue, digest, closes_at, &mut progress)
        .await
        .map_err(|_| "Pale Count timeout")?;
    assert_eq!(
        owner
            .runtime
            .lock()
            .await
            .read_actor_position(actor)
            .map_err(debug)?
            .position(),
        EXIT
    );
    owner.states.test_owner_time.store(
        owner.owner_now().get() + 5_000_000,
        std::sync::atomic::Ordering::Relaxed,
    );
    assert_eq!(walk().await.0.disposition, StepDisposition::Blocked);
    Ok(())
}

async fn seed(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    key: &str,
    firing: [u8; 16],
    digest: [u8; 32],
    window: (i64, i64),
) -> TestResult {
    let world = owner.world_id.as_bytes().as_slice();
    sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,1,$2,'raid-test-map',$2,$2,$2,'TIBIA_SERVER_SAVE_BERLIN_06',1,to_timestamp(0),'1970-01')")
        .bind(world).bind(digest.as_slice()).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,1)")
        .bind(world)
        .execute(&h.pool)
        .await?;
    sqlx::query("INSERT INTO game_world_raid_schedules VALUES(encode($1,'hex')::uuid,$2,2,1,1,2,$3,$4,NULL,false)")
        .bind(world).bind(key).bind(window.0 + 60_000).bind(window.0).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_world_raid_firings VALUES(encode($1,'hex')::uuid,$2,1,encode($3,'hex')::uuid,1,$4,$5,$6,1)")
        .bind(world).bind(key).bind(firing.as_slice()).bind(window.0).bind(window.1).bind(digest.as_slice()).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_world_raid_targets VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1)")
        .bind(firing.as_slice()).bind(owner.channel_id.as_bytes().as_slice()).execute(&h.pool).await?;
    Ok(())
}
