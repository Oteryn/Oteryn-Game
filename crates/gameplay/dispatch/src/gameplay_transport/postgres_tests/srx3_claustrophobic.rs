//! Source sand-timer STEP must start the real World raid owner.
use super::fixture::{Harness, debug, run};
use super::{TestResult, reset};
use crate::foundation::MovementLocalPosition;

#[test]
fn srx3_claustrophobic_actual_world_step_starts_source_raid_postgres() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e::scenario_thread::run("srx3-claustrophobic-world", || {
        run(async |admin| {
            reset();
            // Crystal201189f soul_war.lua:141-146; the first raid rejects
            // arrivals FROM y=31050, not the actual northern ingress used here.
            let scene = super::srx3_soul_war_world_fixture::source_world(MovementLocalPosition {
                x: 34013,
                y: 31048,
                floor: 9,
            })?;
            let deps = super::Dependencies::with_content_and_capacity(
                scene.pin.clone(),
                scene.room.clone(),
                crate::spell::cast::v1_spell_book().map_err(debug)?,
                256,
            )?;
            let h = Harness::create_with_admission_seed(
                admin,
                "srx3_claustrophobic_world",
                false,
                false,
            )
            .await?;
            let evidence = super::house_transfer::RecoverySource::start(&h).await?;
            super::house_transfer::prepare_owned_session(&h).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let mut owner = deps.owner(&h, &authority);
            owner.evidence = &evidence.evidence;
            owner.active_generation = scene.controller.active();
            owner.rooms =
                crate::gameplay_transport::SeamRooms::new(Some(&scene.room), Some(&scene.room));
            // A published World reset binds the map bundle separately from
            // the native server artifact; the private activity fixture uses
            // a server-only target and cannot establish this predecessor.
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
            sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,0,$2,'published-world-fixture',$3,$4,$5,'TIBIA_SERVER_SAVE_BERLIN_06',1,clock_timestamp(),'2026-10')")
                .bind(owner.world_id.as_bytes().as_slice()).bind(bundle_digest.as_slice())
                .bind(scene.pin.server_artifact_digest().as_slice()).bind(scene.pin.client_artifact_digest().as_slice())
                .bind(scene.pin.frame_binding_digest().as_slice()).execute(&h.pool).await?;
            sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,0)")
                .bind(owner.world_id.as_bytes().as_slice())
                .execute(&h.pool)
                .await?;
            let mut admitted = deps.admitted;
            super::srx3_activity_source_fixture::prepare_player(&h, &owner, &mut admitted, 1999)
                .await?;
            let actor = admitted.runtime_actor.ok_or("actual World player")?;
            let before = owner
                .runtime
                .lock()
                .await
                .read_actor_position(actor)
                .map_err(debug)?;
            let context = scene
                .controller
                .active()
                .ok_or("actual World active")?
                .qualify_published_world_context(&scene.room, &scene.pin)
                .ok_or("actual current World frame")?;
            let frame = context.source_frame();
            let position = before.position();
            assert_eq!(
                frame.source_position((position.x, position.y, position.floor)),
                Some([34013, 31048, 9])
            );
            let (moved, _) = owner
                .ordinary_paced_step(
                    actor,
                    admitted.game_session_id,
                    oteryn_protocol_oteryn::world_spatial::StepDirection::South,
                    Some(before),
                    None,
                )
                .await;
            assert_eq!(
                moved.disposition,
                oteryn_protocol_oteryn::world_spatial::StepDisposition::Moved,
                "actual current World movement must reach the source sand timer"
            );
            let now = owner
                .runtime
                .lock()
                .await
                .read_actor_position(actor)
                .map_err(debug)?;
            let position = now.position();
            assert_eq!(
                frame.source_position((position.x, position.y, position.floor)),
                Some([34013, 31049, 9])
            );
            let firings:i64=sqlx::query_scalar("SELECT count(*) FROM game_world_raid_firings WHERE world_id=encode($1,'hex')::uuid AND raid_key='raid:soul-war.claustrophobic-inferno-1'")
            .bind(owner.world_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(
                firings, 1,
                "actual committed sand-timer STEP starts one source-qualified durable World raid"
            );
            let (firing_bytes,fired_at): (Vec<u8>,i64)=sqlx::query_as("SELECT uuid_send(firing_id),fired_at_ms FROM game_world_raid_firings WHERE world_id=encode($1,'hex')::uuid AND raid_key='raid:soul-war.claustrophobic-inferno-1'").bind(owner.world_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
            let firing: [u8; 16] = firing_bytes.try_into().map_err(|_| "source firing ID")?;
            let key = crate::content::ProductionKey::new("raid:soul-war.claustrophobic-inferno-1")
                .map_err(debug)?;
            let catalogue =
                oteryn_gameplay_encounter_rules::raid::RaidCatalogue::embedded().map_err(debug)?;
            let mut progress = crate::gameplay_transport::raid_live::RaidEffectProgress::default();
            for elapsed in [1000, 2000, 3000] {
                owner
                    .drive_raid_firing(
                        firing,
                        &key,
                        &catalogue,
                        [1; 32],
                        fired_at + elapsed,
                        &mut progress,
                    )
                    .await
                    .map_err(|_| "source fixed raid physical cue")?;
            }
            let runtime = owner.runtime.lock().await;
            let receipts = runtime.raid_creatures(firing).map_err(debug)?;
            assert_eq!(
                receipts.len(),
                19,
                "all literal first-stage source spawn requests use the real native raid owner"
            );
            assert!(
                receipts.iter().any(|receipt| matches!(
                    receipt.outcome,
                    oteryn_gameplay_entities::RaidSpawnOutcome::Placed(_)
                )),
                "source raid creates current native actors"
            );
            drop(runtime);
            for (label, query) in [
                (
                    "cursor_without_receipt",
                    "UPDATE game_world_raid_channel_runs SET source_state=source_state || '{\"untrusted\":true}'::jsonb WHERE firing_id=encode($1,'hex')::uuid",
                ),
                (
                    "immutable_source_cue",
                    "UPDATE game_world_raid_effects SET source_cue=source_cue || '{\"untrusted\":true}'::jsonb WHERE firing_id=encode($1,'hex')::uuid AND action_kind=5 AND step_ordinal=0",
                ),
            ] {
                let measured = std::time::Instant::now();
                let mut tx = h.pool.begin().await?;
                let error = sqlx::query(query)
                    .bind(firing.as_slice())
                    .execute(&mut *tx)
                    .await
                    .err()
                    .ok_or(
                        "source metadata unexpectedly mutated without its exact receipt successor",
                    )?;
                assert_eq!(
                    error
                        .as_database_error()
                        .and_then(|error| error.code())
                        .as_deref(),
                    Some("23514"),
                    "{label}: {error}"
                );
                tx.rollback().await?;
                eprintln!(
                    "SRX3_CLAUSTROPHOBIC_SOURCE_METADATA_NEGATIVE case={label} duration={:?}",
                    measured.elapsed()
                );
            }
            drop(owner);
            drop(authority);
            drop(seal);
            h.cleanup().await
        })
    })?;
    eprintln!(
        "SRX3_CLAUSTROPHOBIC_WORLD_POSTGRES_CASE_EXECUTED duration={:?}",
        started.elapsed()
    );
    Ok(())
}
