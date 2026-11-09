//! Accepted authoring13.7 Q6a requires a real Player hit to leave a pool.
//! This explicitly supersedes the historical donor-only signed/no-pool inference;
//! raw signed event inputs for every other policy remain unchanged.
//! Use an actual pinned source map/native body and real Character writers;
//! no fictional boss recipe or source-coordinate authority is introduced.
use super::fixture::{Harness, debug, run};
use super::{Dependencies, TestResult, reset};
use crate::foundation::MovementLocalPosition;
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;

#[test]
fn srx3_cloak_accepted_q6a_actual_normal_player_hit_mints_blood_once_postgres() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e::scenario_thread::run("srx3-cloak-blood", || {
        run(async |admin| {
            reset();
            let scene = super::srx3_activity_source_fixture::SourceScene::new(
                crate::content::bosses::activity_template::PinnedActivitySourceRoom::GoshnarsGreed,
                MovementLocalPosition {
                    x: 33776,
                    y: 31665,
                    floor: 14,
                },
            )?;
            let room = &scene.room;
            let pin = &scene.pin;
            let deps = Dependencies::with_content_and_capacity(
                pin.clone(),
                room.clone(),
                crate::spell::cast::v1_spell_book().map_err(debug)?,
                8,
            )?;
            let active = scene
                .controller
                .active()
                .ok_or("actual native activation")?;
            let native = active
                .native_gameplay()
                .ok_or("actual full native gameplay")?;
            assert!(native.qualifies_soul_war_callbacks(native.source_digest()));
            let h = Harness::create_with_admission_seed(admin, "srx3_cloak_blood", false, false)
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
            super::srx3_activity_source_fixture::install_scene(
                &mut owner,
                &scene,
                crate::content::bosses::activity_template::PinnedActivitySourceRoom::GoshnarsGreed,
            )?;
            super::boss_activity::issue_content_and_reset(&h, pin).await?;
            let mut admitted = deps.admitted;
            // The literal Cloak has defense 107 and armor 107. The previous
            // level-500 unarmed setup completed sixteen swings with zero HP
            // loss, never reaching this health-event/MINT contract. Use the real
            // XP writer to prepare a penetrating attack; retain all target stats.
            super::srx3_activity_source_fixture::prepare_player(&h, &owner, &mut admitted, 2000)
                .await?;
            let player = admitted.runtime_actor.ok_or("actual admitted player")?;
            let (image, target) = {
                let mut runtime = owner.runtime.lock().await;
                native
                    .install_companion_policies(&mut runtime)
                    .map_err(debug)?;
                let origin = runtime
                    .read_actor_position(player)
                    .map_err(debug)?
                    .position();
                let at = [(1, 0), (0, 1), (-1, 0), (0, -1)]
                    .into_iter()
                    .map(|(x, y)| MovementLocalPosition {
                        x: origin.x + x,
                        y: origin.y + y,
                        floor: origin.floor,
                    })
                    .find(|at| {
                        room.movement_cells().index().lookup(
                            room.movement_cells().scope(),
                            crate::content::LogicalCell {
                                x: at.x,
                                y: at.y,
                                z: i32::from(at.floor),
                            },
                        ) == Ok(crate::content::CollisionClass::Walkable)
                            && runtime.native_summon_cell_free(*at)
                    })
                    .ok_or("real adjacent walkable native cell")?;
                let image = runtime
                    .spawn_arena_opponent("oteryn:creature.cloak_of_terror", at, |r, p| {
                        Some(p == at && r.native_summon_cell_free(p))
                    })
                    .map_err(debug)?;
                assert!(
                    runtime
                        .soul_war_health_callback(image)
                        .map_err(debug)?
                        .is_some(),
                    "actual bounded native source callback token"
                );
                let row = runtime
                    .visible_entities()
                    .creatures
                    .into_iter()
                    .find(|row| row.actor == image)
                    .ok_or("actual visible source image")?;
                (
                    image,
                    oteryn_protocol_oteryn::world_spatial_entities::EntityRef {
                        identity: image.placement_identity(),
                        generation: row.generation,
                    },
                )
            };
            assert_eq!(
                owner
                    .select_attack_target(player, admitted.game_session_id, 947, Some(target))
                    .await,
                oteryn_protocol_oteryn::attack::AttackIntentDisposition::Ok
            );
            // Reject the first physical callback at its real SQL boundary. The
            // HP event must remain pending, with no partial Item, until retry.
            sqlx::raw_sql("CREATE FUNCTION boss2_cloak_mint_fault() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.cause_kind=2 AND NEW.creature_definition_key='oteryn:creature.cloak_of_terror' THEN RAISE EXCEPTION 'boss2 retained Cloak callback fault'; END IF; RETURN NEW; END $$; CREATE TRIGGER boss2_cloak_mint_fault BEFORE INSERT ON game_spell_item_receipts FOR EACH ROW EXECUTE FUNCTION boss2_cloak_mint_fault();")
                .execute(&h.pool).await?;
            owner
                .refresh_attack_weapon(player, admitted.game_session_id)
                .await;
            for _ in 0..16 {
                owner.clock_origin = owner
                    .clock_origin
                    .checked_sub(std::time::Duration::from_secs(3))
                    .ok_or("semantic swing clock")?;
                owner
                    .refresh_attack_weapon(player, admitted.game_session_id)
                    .await;
                owner.drain_auto_attacks().await;
                if owner
                    .runtime
                    .lock()
                    .await
                    .companion_snapshot_including_dead(image)
                    .is_ok_and(|snapshot| snapshot.health < snapshot.maximum_health)
                {
                    break;
                }
            }
            {
                let runtime = owner.runtime.lock().await;
                let cloak = runtime
                    .companion_snapshot_including_dead(image)
                    .map_err(debug)?;
                assert!(
                    cloak.health < cloak.maximum_health,
                    "actual canonical auto attack must damage the native Cloak before testing its signed health-event predicate"
                );
                assert_eq!(
                    runtime.pending_cloak_blood_events(64).map_err(debug)?.len(),
                    1,
                    "Q6a real Player hit must retain its genuine canonical HP-event before testing the physical MINT consumer"
                );
            }
            let before_retry: i64 = sqlx::query_scalar("SELECT count(*) FROM game_spell_item_receipts WHERE cause_kind=2 AND creature_definition_key='oteryn:creature.cloak_of_terror'").fetch_one(&h.pool).await?;
            assert_eq!(
                before_retry, 0,
                "failed physical callback commits no receipt"
            );
            sqlx::raw_sql("DROP TRIGGER boss2_cloak_mint_fault ON game_spell_item_receipts; DROP FUNCTION boss2_cloak_mint_fault();").execute(&h.pool).await?;
            owner.drain_monster_melee().await;
            assert!(
                owner
                    .runtime
                    .lock()
                    .await
                    .pending_cloak_blood_events(32)
                    .map_err(debug)?
                    .is_empty(),
                "known physical COMMIT acknowledges the retained HP callback"
            );
            let blood:i64=sqlx::query_scalar("SELECT count(*) FROM game_spell_item_receipts r JOIN game_spell_item_lines l USING(transaction_id) JOIN game_item_ground_locations g USING(item_instance_id) WHERE r.cause_kind=2 AND r.spell_family='Encounter' AND r.spell_production_key='oteryn:encounter.soul_war_taint_zones' AND r.runtime_scope_kind=1 AND r.channel_id=encode($1,'hex')::uuid AND l.operation_kind=1 AND l.definition_production_key='oteryn:item.tibia.i33854' AND g.scope_kind=1 AND g.channel_id=r.channel_id").bind(owner.channel_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(
                blood, 1,
                "Q6a accepted Player hit must create one real first-stage blood Item through the existing physical owner"
            );
            owner.drain_monster_melee().await;
            let replay:i64=sqlx::query_scalar("SELECT count(*) FROM game_spell_item_receipts r JOIN game_spell_item_lines l USING(transaction_id) WHERE r.cause_kind=2 AND r.spell_production_key='oteryn:encounter.soul_war_taint_zones' AND r.channel_id=encode($1,'hex')::uuid AND l.operation_kind=1 AND l.definition_production_key='oteryn:item.tibia.i33854'").bind(owner.channel_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(
                replay, 1,
                "normal owner retry must not mint the acknowledged blood event twice"
            );
            drop(owner);
            drop(authority);
            drop(seal);
            h.cleanup().await
        })
    })?;
    eprintln!(
        "SRX3_CLOAK_BLOOD_NATIVE_ITEM_POSTGRES_EXECUTED elapsed {:?}",
        started.elapsed()
    );
    Ok(())
}
