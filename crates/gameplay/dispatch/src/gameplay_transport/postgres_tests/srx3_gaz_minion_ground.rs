//! Real attached Minion/Corrupted Soul lethal -> same physical Ground Item owner.
use super::fixture::{Harness, debug, run};
use super::{Dependencies, TestResult, reset};
use crate::foundation::MovementLocalPosition;
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;

async fn assert_ground_history(h: &Harness) -> TestResult {
    let mut tx = h.pool.begin().await?;
    crate::durability::spell_item_transaction::verify_spell_item_chain(&mut tx)
        .await
        .map_err(debug)?;
    tx.rollback().await?;
    Ok(())
}

fn run_ground_case(corrupted: bool, followup: bool) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e::scenario_thread::run("srx3-gaz-minion-ground", move || {
        run(async |admin| {
            reset();
            let scene = super::srx3_activity_source_fixture::SourceScene::new(
                crate::content::bosses::activity_template::PinnedActivitySourceRoom::BrainHead,
                MovementLocalPosition {
                    x: 31953,
                    y: 32325,
                    floor: 10,
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

            let h =
                Harness::create_with_admission_seed(admin, "srx3_gaz_minion_ground", false, false)
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
                crate::content::bosses::activity_template::PinnedActivitySourceRoom::BrainHead,
            )?;
            super::boss_activity::issue_content_and_reset(&h, pin).await?;
            let mut admitted = deps.admitted;
            super::srx3_activity_source_fixture::prepare_player(&h, &owner, &mut admitted, 500)
                .await?;
            let player = admitted.runtime_actor.ok_or("actual admitted player")?;
            let key = if corrupted {
                "oteryn:creature.corrupted_soul"
            } else {
                "oteryn:creature.minion_of_gaz_haragoth"
            };
            let item = if corrupted {
                "oteryn:item.tibia.i23726"
            } else {
                "oteryn:item.tibia.i20121"
            };
            let mut source_at = None;
            let mut death_times = Vec::new();
            let mut item_ids: Vec<String> = Vec::new();
            let deaths = if followup {
                if corrupted { 4 } else { 2 }
            } else {
                1
            };
            for death_index in 0..deaths {
                if followup && !corrupted && death_index == 1 {
                    owner.clock_origin -= std::time::Duration::from_secs(5);
                }
                let (image, target) = {
                    let mut runtime = owner.runtime.lock().await;
                    native
                        .install_companion_policies(&mut runtime)
                        .map_err(debug)?;
                    let origin = runtime
                        .read_actor_position(player)
                        .map_err(debug)?
                        .position();
                    let at = source_at
                        .or_else(|| {
                            [(1, 0), (0, 1), (-1, 0), (0, -1)]
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
                        })
                        .ok_or("real adjacent walkable native cell")?;
                    source_at = Some(at);
                    let image = runtime
                        .spawn_arena_opponent(key, at, |r, p| {
                            Some(p == at && r.native_summon_cell_free(p))
                        })
                        .map_err(debug)?;
                    // Only the final-hit precondition is injured; native source policy,
                    // actual maxHP and the committed death path remain unchanged.
                    runtime.set_test_creature_health(image, 1).map_err(debug)?;
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
                        .select_attack_target(
                            player,
                            admitted.game_session_id,
                            947 + death_index,
                            Some(target)
                        )
                        .await,
                    oteryn_protocol_oteryn::attack::AttackIntentDisposition::Ok
                );
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
                        .is_ok_and(|snapshot| snapshot.health == 0)
                    {
                        break;
                    }
                }
                {
                    let mut runtime = owner.runtime.lock().await;
                    assert!(
                        runtime
                            .borrow_combat_death()
                            .committed_lethal_receipt(image)
                            .is_ok(),
                        "actual Player auto-attack must commit the native source lethal before testing source death Item"
                    );
                }
                owner.drain_monster_melee().await;
                let death_at = owner.owner_now().get();
                death_times.push(death_at);
                // The detached Item commit ACKs the copied handler. Its next
                // native turn completes the FIFO death task before later timers.
                owner.drain_monster_melee().await;
                assert!(
                    !owner
                        .runtime
                        .lock()
                        .await
                        .has_pending_source_creature_deaths(),
                    "actual copied death must finish before advancing its expiry clock"
                );
                if death_index == 0 {
                    let minted:i64=sqlx::query_scalar("SELECT count(*) FROM game_item_ground_locations g JOIN game_item_instances i USING(item_instance_id) JOIN game_spell_item_receipts r ON r.transaction_id=i.minted_transaction_id WHERE g.scope_kind=1 AND g.channel_id=encode($1,'hex')::uuid AND i.lifecycle=1 AND i.definition_production_key=$2 AND r.cause_kind=3 AND ($3 OR convert_from(r.intent,'UTF8')::jsonb->'source_event'->>'action_id'='33542')")
                .bind(owner.channel_id.as_bytes().as_slice()).bind(item).bind(corrupted).fetch_one(&h.pool).await?;
                    assert_eq!(
                        minted, 1,
                        "attached source death must create its exact Ground Item/action under the same physical Item owner"
                    );
                    owner.drain_monster_melee().await;
                    let replay:i64=sqlx::query_scalar("SELECT count(*) FROM game_item_ground_locations g JOIN game_item_instances i USING(item_instance_id) WHERE g.scope_kind=1 AND g.channel_id=encode($1,'hex')::uuid AND i.lifecycle=1 AND i.definition_production_key=$2")
                .bind(owner.channel_id.as_bytes().as_slice()).bind(item).fetch_one(&h.pool).await?;
                    assert_eq!(
                        replay, 1,
                        "committed source death replay cannot duplicate the Ground Item"
                    );
                    if corrupted {
                        owner.clock_origin -= std::time::Duration::from_secs(31);
                        owner.drain_monster_melee().await;
                        let remaining:i64=sqlx::query_scalar("SELECT count(*) FROM game_item_ground_locations g JOIN game_item_instances i USING(item_instance_id) WHERE g.scope_kind=1 AND g.channel_id=encode($1,'hex')::uuid AND i.lifecycle=1 AND i.definition_production_key=$2")
                    .bind(owner.channel_id.as_bytes().as_slice()).bind(item).fetch_one(&h.pool).await?;
                        assert_eq!(
                            remaining, 1,
                            "first CorruptedSoul creation returns BEFORE scheduling the30s current-item removal"
                        );
                    }
                }
                if followup {
                    let rows: Vec<(String,String)> = sqlx::query_as("SELECT i.item_instance_id::text,i.definition_production_key FROM game_item_ground_locations g JOIN game_item_instances i USING(item_instance_id) WHERE g.scope_kind=1 AND g.channel_id=encode($1,'hex')::uuid AND i.lifecycle=1 AND i.definition_production_key = ANY($2) ORDER BY g.stack_ordinal DESC")
                    .bind(owner.channel_id.as_bytes().as_slice())
                    .bind(vec!["oteryn:item.tibia.i20121","oteryn:item.tibia.i23726","oteryn:item.tibia.i23727","oteryn:item.tibia.i23728"])
                    .fetch_all(&h.pool).await?;
                    if corrupted {
                        let expected = match death_index {
                            0 => "oteryn:item.tibia.i23726",
                            1 => "oteryn:item.tibia.i23727",
                            _ => "oteryn:item.tibia.i23728",
                        };
                        assert_eq!(
                            rows.len(),
                            1,
                            "one CURRENT source vortex survives each copied handler"
                        );
                        assert_eq!(
                            rows[0].1, expected,
                            "actual successive native deaths must take literal26→27→28 and existing28 no-op branches"
                        );
                        if let Some(original) = item_ids.first() {
                            assert_eq!(
                                &rows[0].0, original,
                                "source transform preserves the same physical Item identity"
                            );
                        } else {
                            item_ids.push(rows[0].0.clone());
                        }
                    } else {
                        assert_eq!(
                            rows.len(),
                            death_index as usize + 1,
                            "both real Gaz deaths must precede the original60s expiry"
                        );
                        item_ids.push(rows[0].0.clone());
                    }
                }
                // Recovery must accept each committed physical successor, including
                // the retained existing23728 no-op and its unchanged Item history.
                assert_ground_history(&h).await?;
            }
            if followup {
                let first_due = if corrupted {
                    death_times[1] + 31_000_000
                } else {
                    death_times[0] + 61_000_000
                };
                let current = owner.owner_now().get();
                assert!(
                    current < first_due,
                    "actual follow-up lethals must complete before the first original script expiry"
                );
                owner.clock_origin -= std::time::Duration::from_micros(first_due - current);
                owner.drain_monster_melee().await;
                let remaining:Vec<String>=sqlx::query_scalar("SELECT i.item_instance_id::text FROM game_item_ground_locations g JOIN game_item_instances i USING(item_instance_id) WHERE g.scope_kind=1 AND g.channel_id=encode($1,'hex')::uuid AND i.lifecycle=1 AND i.definition_production_key=ANY($2)")
                    .bind(owner.channel_id.as_bytes().as_slice()).bind(vec!["oteryn:item.tibia.i20121","oteryn:item.tibia.i23726","oteryn:item.tibia.i23727","oteryn:item.tibia.i23728"]).fetch_all(&h.pool).await?;
                if corrupted {
                    assert!(
                        remaining.is_empty(),
                        "first original30s timer removes the CURRENT transformed23728"
                    );
                } else {
                    assert_eq!(
                        remaining,
                        vec![item_ids[0].clone()],
                        "first60s timer removes newest CURRENT20121, not its own originally minted UUID"
                    );
                }
                assert_ground_history(&h).await?;
                let last_due = death_times
                    .last()
                    .copied()
                    .ok_or("actual last lethal clock")?
                    + if corrupted { 31_000_000 } else { 61_000_000 };
                let current = owner.owner_now().get();
                owner.clock_origin -=
                    std::time::Duration::from_micros(last_due.saturating_sub(current));
                owner.drain_monster_melee().await;
                let remaining:i64=sqlx::query_scalar("SELECT count(*) FROM game_item_ground_locations g JOIN game_item_instances i USING(item_instance_id) WHERE g.scope_kind=1 AND g.channel_id=encode($1,'hex')::uuid AND i.lifecycle=1 AND i.definition_production_key=ANY($2)")
                    .bind(owner.channel_id.as_bytes().as_slice()).bind(vec!["oteryn:item.tibia.i20121","oteryn:item.tibia.i23726","oteryn:item.tibia.i23727","oteryn:item.tibia.i23728"]).fetch_one(&h.pool).await?;
                assert_eq!(
                    remaining, 0,
                    "all original current-item removal deadlines settle without recreating an Item"
                );
                let rule = if corrupted {
                    "w01.corrupted-soul.expire"
                } else {
                    "w01.gaz-minion.expire"
                };
                let expiries:i64=sqlx::query_scalar("SELECT count(*) FROM game_spell_item_receipts WHERE runtime_scope_kind=1 AND channel_id=encode($1,'hex')::uuid AND cause_kind=3 AND convert_from(intent,'UTF8')::jsonb->'source_event'->>'rule'=$2")
                    .bind(owner.channel_id.as_bytes().as_slice()).bind(rule).fetch_one(&h.pool).await?;
                assert_eq!(
                    expiries,
                    if corrupted { 3 } else { 2 },
                    "existing23728 zero-op must still schedule its own30s removal; first absent create schedules none"
                );
                assert_ground_history(&h).await?;
            }
            drop(owner);
            drop(authority);
            drop(seal);
            h.cleanup().await
        })
    })?;
    eprintln!(
        "SRX3_GAZ_MINION_NATIVE_ITEM_POSTGRES_EXECUTED elapsed {:?}",
        started.elapsed()
    );
    Ok(())
}

#[test]
fn srx3_gaz_minion_actual_player_lethal_creates_nightmare_vortex_postgres() -> TestResult {
    run_ground_case(false, false)
}
#[test]
fn srx3_corrupted_soul_actual_player_lethal_first_vortex_has_no_expiry_postgres() -> TestResult {
    run_ground_case(true, false)
}

#[test]
fn srx3_gaz_minion_actual_two_deaths_expire_current_item_not_original_uuid_postgres() -> TestResult
{
    run_ground_case(false, true)
}
#[test]
fn srx3_corrupted_soul_actual_transform_chain_and_existing28_expiry_postgres() -> TestResult {
    run_ground_case(true, true)
}
