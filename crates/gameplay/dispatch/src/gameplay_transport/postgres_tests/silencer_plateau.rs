//! Silencer chamber USE through the published World and real inventory owner.
use super::fixture::{Harness, debug, id, run};
use super::{TestResult, reset};
use crate::foundation::MovementLocalPosition;
use crate::gameplay_transport::{
    connection::UseCommand, item_view::ItemKey, raid_live::RaidEffectProgress,
};
use oteryn_gameplay_encounter_rules::raid::{
    RaidCatalogue,
    source_silencer_plateau::{CHAMBER_KEY, COOLDOWN_MS, RAID_KEY, successful_use},
};
use oteryn_protocol_oteryn::world_object::UseDisposition;

#[test]
fn silencer_plateau_carried_use_starts_waves_and_refuses_cooldown_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    assert!(!admin.is_empty());
    super::kill_reward_e2e::scenario_thread::run("silencer-plateau-use", || {
        run(async |admin| {
            reset();
            let scene = super::srx3_soul_war_world_fixture::source_world(MovementLocalPosition {
                x: 33650,
                y: 32527,
                floor: 5,
            })?;
            let deps = super::Dependencies::with_content_and_capacity(
                scene.pin.clone(),
                scene.room.clone(),
                crate::spell::cast::v1_spell_book().map_err(debug)?,
                256,
            )?;
            let h = Harness::create_with_admission_seed(
                admin,
                "silencer_plateau_use",
                false,
                false,
            )
            .await?;
            let evidence = super::house_transfer::RecoverySource::start(&h).await?;
            super::house_transfer::prepare_owned_session(&h).await?;
            super::item_look::seed_main_backpack(&h, &[(CHAMBER_KEY, 1), (CHAMBER_KEY, 1)]).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let mut owner = deps.owner(&h, &authority);
            owner.reset_epoch = 1;
            owner.evidence = &evidence.evidence;
            owner.active_generation = scene.controller.active();
            use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
            owner
                .active_generation
                .and_then(|active| active.native_gameplay())
                .ok_or("actual World native policies")?
                .install_companion_policies(&mut *owner.runtime.lock().await)
                .map_err(debug)?;
            owner.rooms =
                crate::gameplay_transport::SeamRooms::new(Some(&scene.room), Some(&scene.room));
            use crate::map::boot::BundleCollisionIndexExt;
            let crate::content::native_cell_lookup::NativeMovementCollisionIndex::Bundle(index) =
                scene.room.movement_cells().index()
            else {
                return Err("actual World collision index".into());
            };
            let bundle_digest = index.map().overlay.base().digest();
            sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
                .bind(owner.world_id.as_bytes().as_slice()).bind(owner.channel_id.as_bytes().as_slice()).execute(&h.pool).await?;
            sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
                .bind(owner.world_id.as_bytes().as_slice()).bind(owner.channel_id.as_bytes().as_slice())
                .bind(scene.pin.server_artifact_digest().as_slice()).bind(scene.pin.client_artifact_digest().as_slice())
                .bind(scene.pin.frame_binding_digest().as_slice()).execute(&h.pool).await?;
            sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,1,$2,'published-world-fixture',$3,$4,$5,'TIBIA_SERVER_SAVE_BERLIN_06',1,clock_timestamp(),'2026-10')")
                .bind(owner.world_id.as_bytes().as_slice()).bind(bundle_digest.as_slice())
                .bind(scene.pin.server_artifact_digest().as_slice()).bind(scene.pin.client_artifact_digest().as_slice())
                .bind(scene.pin.frame_binding_digest().as_slice()).execute(&h.pool).await?;
            sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,1)")
                .bind(owner.world_id.as_bytes().as_slice())
                .execute(&h.pool)
                .await?;
            let mut admitted = deps.admitted;
            super::srx3_activity_source_fixture::prepare_player(&h, &owner, &mut admitted, 1999)
                .await?;
            let actor = admitted.runtime_actor.ok_or("actual World player")?;
            let session = admitted.game_session_id;
            let command_id = (1..=64)
                .find(|id| successful_use(bundle_digest, *session.as_bytes(), *id) == Ok(true))
                .ok_or("fixture command with successful chamber draw")?;
            let command = |command_id| UseCommand {
                game_session_id: session,
                command_id,
                item_fence: admitted.item_fence,
            };
            let used = owner.use_backpack_item(actor, command(command_id), ItemKey::Instance(id(71))).await;
            assert_eq!(used.disposition, UseDisposition::Committed);
            let (firing, at, until): (Vec<u8>, i64, i64) = sqlx::query_as(
                "SELECT decode(replace(firing_id::text,'-',''),'hex'),fired_at_ms,deadline_ms FROM game_world_raid_firings WHERE raid_key=$1",
            ).bind(RAID_KEY).fetch_one(&h.pool).await?;
            assert_eq!(until - at, i64::from(COOLDOWN_MS));
            let blocked = owner.use_backpack_item(actor, command(command_id + 1), ItemKey::Instance(id(72))).await;
            assert_eq!(blocked.disposition, UseDisposition::Exhausted);
            let quantity: i64 = sqlx::query_scalar("SELECT sum(quantity)::bigint FROM game_item_instances WHERE definition_production_key=$1")
                .bind(CHAMBER_KEY).fetch_one(&h.pool).await?;
            assert_eq!(quantity, 1);

            let firing = firing.try_into().map_err(|_| "firing UUID length")?;
            let key = crate::content::ProductionKey::new(RAID_KEY)?;
            let mut progress = RaidEffectProgress::default();
            owner.drive_raid_firing(firing, &key, &RaidCatalogue::embedded()?, bundle_digest, at + 60_000, &mut progress)
                .await.map_err(|_| "first plateau wave")?;
            let planned: i64 = sqlx::query_scalar("SELECT count(*) FROM game_world_raid_creatures WHERE wave=0")
                .fetch_one(&h.pool).await?;
            assert!((8..=15).contains(&planned));

            drop(owner);
            drop(authority);
            drop(seal);
            drop(evidence);
            h.cleanup().await
        })
    })
}
