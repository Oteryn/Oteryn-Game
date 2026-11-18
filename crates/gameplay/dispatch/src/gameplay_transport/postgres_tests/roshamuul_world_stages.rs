//! An inactive Crystal reconciliation must never run as a World-stage raid.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::content::ProductionKey;
use crate::gameplay_transport::raid_live::RaidEffectProgress;
use oteryn_gameplay_encounter_rules::raid::RaidCatalogue;

#[test]
fn roshamuul_world_stage_raid_refuses_execution_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    assert!(!admin.is_empty());
    super::kill_reward_e2e::scenario_thread::run("roshamuul-world-stage-refusal", || {
        run(async |admin| {
            let harness =
                Harness::create_with_admission_seed(admin, "rosh_world_stage_refusal", false, false)
                    .await?;
            prepare_current_session(&harness).await?;
            let seal = harness.recovery.seal_current().map_err(debug)?;
            let authority = harness.root.open_character_authority(&seal).await.map_err(debug)?;
            let dependencies = Dependencies::new()?;
            let owner = dependencies.owner(&harness, &authority);
            let catalogue = RaidCatalogue::embedded()?;
            let key = ProductionKey::new("raid:roshamuul.gaz_haragoth")?;
            let source = catalogue.get(&key).ok_or("pinned Crystal raid")?;
            assert!(!source.active);
            assert!(crate::gameplay_transport::kill_reward::world_change_credits(
                "oteryn:creature.gaz_haragoth",
                crate::foundation::MovementLocalPosition {
                    x: 33538,
                    y: 32381,
                    floor: 12,
                },
            ).is_empty());
            let deadline = source.variants.iter()
                .flat_map(|variant| &variant.waves)
                .map(|wave| i64::from(wave.delay_ms))
                .max().ok_or("pinned raid spawn delay")?;
            let digest = dependencies.runtime.lock().await.content_pin().server_artifact_digest();
            let world = owner.world_id.as_bytes().as_slice();
            let firing = id(91);
            sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month)
                         VALUES(encode($1,'hex')::uuid,1,$2,'raid-test-map',$2,$2,$2,'TIBIA_SERVER_SAVE_BERLIN_06',1,to_timestamp(0),'1970-01')")
                .bind(world).bind(digest.as_slice()).execute(&harness.pool).await?;
            sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,1)")
                .bind(world).execute(&harness.pool).await?;
            sqlx::query("INSERT INTO game_world_raid_schedules VALUES(encode($1,'hex')::uuid,$2,2,1,1,2,60000,0,NULL,false)")
                .bind(world).bind(key.as_str()).execute(&harness.pool).await?;
            sqlx::query("INSERT INTO game_world_raid_firings VALUES(encode($1,'hex')::uuid,$2,1,encode($3,'hex')::uuid,1,0,$4,$5,1)")
                .bind(world).bind(key.as_str()).bind(firing.as_slice()).bind(deadline)
                .bind(digest.as_slice()).execute(&harness.pool).await?;
            sqlx::query("INSERT INTO game_world_raid_targets VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1)")
                .bind(firing.as_slice()).bind(owner.channel_id.as_bytes().as_slice())
                .execute(&harness.pool).await?;

            let mut progress = RaidEffectProgress::default();
            assert!(owner.drive_raid_firing(firing, &key, &catalogue, digest, deadline, &mut progress)
                .await.is_err());
            let runs: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM game_world_raid_channel_runs WHERE firing_id=encode($1,'hex')::uuid",
            ).bind(firing.as_slice()).fetch_one(&harness.pool).await?;
            assert_eq!(runs, 0);

            drop(owner);
            drop(authority);
            drop(seal);
            harness.cleanup().await
        })
    })
}
