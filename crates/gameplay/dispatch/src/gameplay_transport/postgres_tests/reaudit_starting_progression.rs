//! SRVGAP-C1: the production first-entry caller must populate its starter backpack.
use super::fixture::{Harness, debug, run};
use super::tutorial_vitals_fixture::admit;
use super::{Dependencies, TestResult, prepare_current_session};

#[test]
fn reaudit_starting_progression_first_entry_postgres() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e::scenario_thread::run("starting-progression", || {
        run(async |admin| {
            let h =
                Harness::create_with_admission_seed(admin, "starting_progression", false, false)
                    .await?;
            let request = prepare_current_session(&h).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let world =
                crate::foundation::WorldId::decode(&super::fixture::id(42)).map_err(debug)?;
            let (controller, pin) =
                super::death_ml_loss_composed::production_generation_with_pin(world, "content-1")?;
            super::tutorial_vitals_fixture::publish_native_activation(&h, &pin).await?;
            let room = pin.qualified_room().clone();
            let start = room.entry_start();
            let dependencies = Dependencies::with_content(
                crate::foundation::ChannelContentPin::from_activation(
                    world,
                    pin.activation_sequence(),
                    pin.identity().server_artifact_digest(),
                    pin.identity().client_artifact_digest(),
                    pin.frame_binding().digest(),
                    room.map_revision_digest(),
                    (start.x, start.y, start.floor),
                ),
                room,
                crate::spell::cast::v1_spell_book().map_err(debug)?,
            )?;
            let mut owner = dependencies.owner(&h, &authority);
            owner.active_generation = controller.active();
            admit(&owner, &dependencies, &request).await?;
            let items: Vec<(String, i64)> = sqlx::query_as(
                "SELECT i.definition_production_key,i.quantity FROM game_item_container_entries e \
                 JOIN game_item_instances i USING(item_instance_id) ORDER BY e.placement_ordinal",
            )
            .fetch_all(&h.pool)
            .await?;
            assert_eq!(
                items,
                vec![
                    ("oteryn:item.tibia.i3003".into(), 1),
                    ("oteryn:item.tibia.i3457".into(), 1)
                ],
                "vocationless first-entry kit: rope and shovel"
            );
            admit(&owner, &dependencies, &request).await?;
            let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_instances")
                .fetch_one(&h.pool)
                .await?;
            assert_eq!(count, 3, "replay must not mint the kit again");
            drop(owner);
            drop(authority);
            drop(seal);
            h.cleanup().await
        })
    })?;
    eprintln!(
        "SRVGAP_C1_PG_CASE_EXECUTED elapsed_ms={}",
        started.elapsed().as_millis()
    );
    Ok(())
}

#[test]
fn reaudit_starting_progression_vocations_and_departure_postgres() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e::scenario_thread::run("starting-vocations", || {
        run(async |admin| {
            use super::fixture::{fence, id};
            use crate::durability::character_build::{
                BuildCause, BuildChangeRequest, BuildOccurrence, convert_vocation,
            };
            use crate::foundation::{ChannelContentPin, MovementLocalPosition};
            use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
            use oteryn_gameplay_items::starter_grant::starting_loadout;
            use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};
            let world_id = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
            let (controller, pin) = super::death_ml_loss_composed::production_generation_with_pin(
                world_id,
                "content-1",
            )?;
            let active = controller.active().ok_or("active")?;
            let formula = active
                .native_gameplay()
                .and_then(|native| native.training_formula())
                .ok_or("training formula")?;
            for (vocation, unique, confirmed) in [
                ("sorcerer", 22001, true),
                ("druid", 22002, true),
                ("paladin", 22003, true),
                ("knight", 22004, true),
                ("monk", 0, true),
                ("knight", 22004, false),
            ] {
                let mut h = Harness::create_with_admission_seed(
                    admin.clone(),
                    &format!("starting_{vocation}_{confirmed}"),
                    true,
                    false,
                )
                .await?;
                let request = prepare_current_session(&h).await?;
                let seal = h.recovery.seal_current().map_err(debug)?;
                let authority = h
                    .root
                    .open_character_authority(&seal)
                    .await
                    .map_err(debug)?;
                super::tutorial_vitals_fixture::publish_native_activation(&h, &pin).await?;
                let before = h
                    .root
                    .read_character_build_state(&authority, fence(1)?.character_id)
                    .await
                    .map_err(debug)?;
                let after = convert_vocation(formula, &before, vocation).map_err(debug)?;
                let mut current = fence(1)?;
                current.character_lease_generation = 2;
                h.root
                    .commit_character_build(
                        &authority,
                        &h.node,
                        current,
                        BuildChangeRequest {
                            occurrence: BuildOccurrence::from_bytes(id(90)).map_err(debug)?,
                            cause: if confirmed {
                                BuildCause::VocationChoice
                            } else {
                                BuildCause::VocationTrial
                            },
                            before,
                            after,
                            pruned_stance: None,
                        },
                        formula,
                    )
                    .await
                    .map_err(debug)?;
                let wrong = if unique == 22001 { 22002 } else { 22001 };
                let world = super::quest_callbacks::source_world_choices(
                    (32054, 31883, -6),
                    &[(32054, unique), (32052, wrong), (32074, 0)],
                    100,
                )?;
                let cells = world
                    .movement_cells(pin.qualified_room().movement_cells())
                    .map_err(debug)?;
                let deps = Dependencies::with_content(
                    ChannelContentPin::from_activation(
                        world_id,
                        pin.activation_sequence(),
                        pin.identity().server_artifact_digest(),
                        pin.identity().client_artifact_digest(),
                        world.frame_binding_digest(world_id),
                        world.map_revision_digest(),
                        (32053, 31883, 6),
                    ),
                    pin.qualified_room().clone(),
                    crate::spell::cast::v1_spell_book().map_err(debug)?,
                )?;
                let mut owner = deps.owner(&h, &authority);
                owner.active_generation = Some(active);
                owner.movement_cells = (&cells).into();
                admit(&owner, &deps, &request).await?;
                let mut actual: Vec<(String,i64,i16)> = sqlx::query_as(
                "SELECT i.definition_production_key,i.quantity,COALESCE(e.slot,0)::smallint FROM game_item_instances i \
                 LEFT JOIN game_character_equipment_slots e USING(item_instance_id) WHERE i.definition_production_key<>'oteryn:item.tibia.i2854' ORDER BY i.definition_production_key")
                .fetch_all(&h.pool).await?;
                let mut expected: Vec<_> = starting_loadout::items(vocation)
                    .ok_or("kit")?
                    .iter()
                    .map(|&(id, count, slot)| {
                        (
                            format!("oteryn:item.tibia.i{id}"),
                            i64::from(count),
                            i16::from(slot),
                        )
                    })
                    .collect();
                actual.sort();
                expected.sort();
                assert_eq!(actual, expected, "full {vocation} kit");
                let mut count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_instances")
                    .fetch_one(&h.pool)
                    .await?;
                admit(&owner, &deps, &request).await?;
                assert_eq!(
                    sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_item_instances")
                        .fetch_one(&h.pool)
                        .await?,
                    count
                );
                if unique != 0 {
                    let session = deps.admitted.game_session_id;
                    let (current, _) =
                        crate::durability::fresh_admission::FreshAdmissionStore::from_root(
                            h.root.clone(),
                        )
                        .current_session_at(session)
                        .await?;
                    let item_fence =
                        crate::gameplay_transport::item_fence_of(current).ok_or("item fence")?;
                    let target = |x| -> TestResult<WorldObjectTarget> {
                        let key = crate::map::view::placement_key(
                            crate::map::overlay::TilePos {
                                x,
                                y: 31883,
                                floor: -6,
                            },
                            1,
                        )
                        .ok_or("placement")?;
                        let mut placement = world.base().digest().to_vec();
                        placement.extend_from_slice(&key.to_be_bytes());
                        Ok(WorldObjectTarget {
                            placement,
                            expected_revision: 0,
                        })
                    };
                    let command_fence = std::cell::Cell::new(item_fence);
                    let command = |id| UseCommand {
                        game_session_id: session,
                        command_id: id,
                        item_fence: Some(command_fence.get()),
                    };
                    assert_eq!(
                        owner
                            .use_object(deps.actor, command(1), target(32052)?)
                            .await
                            .disposition,
                        UseDisposition::Rejected
                    );
                    owner.clock_origin = owner
                        .clock_origin
                        .checked_sub(std::time::Duration::from_secs(1))
                        .ok_or("clock")?;
                    if !confirmed {
                        assert_eq!(
                            owner
                                .use_object(deps.actor, command(2), target(32054)?)
                                .await
                                .disposition,
                            UseDisposition::Rejected,
                            "trial alone cannot open a departure door"
                        );
                        assert_eq!(
                            sqlx::query_scalar::<_, i64>(
                                "SELECT count(*) FROM game_dawnport_departures"
                            )
                            .fetch_one(&h.pool)
                            .await?,
                            0
                        );
                        drop(owner);
                        drop(authority);
                        drop(seal);
                        h.cleanup().await?;
                        continue;
                    }
                    let mut stale = command(2);
                    let mut stale_fence = item_fence;
                    stale_fence.character_lease_generation += 1;
                    stale.item_fence = Some(stale_fence);
                    assert_eq!(
                        owner
                            .use_object(deps.actor, stale, target(32054)?)
                            .await
                            .disposition,
                        UseDisposition::Rejected
                    );
                    assert!(!owner.states.dawnport_departures.held(session));
                    owner.clock_origin = owner
                        .clock_origin
                        .checked_sub(std::time::Duration::from_secs(1))
                        .ok_or("clock")?;
                    if vocation == "knight" {
                        seed_nested_cleanup(&h, active).await?;
                        count += 4;
                    }
                    // A deferred failure rolls back every removal, equipment revision and landing.
                    if vocation == "knight" {
                        sqlx::query("CREATE FUNCTION starting_progression_fail() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'injected departure failure'; END $$") .execute(&h.pool).await?;
                        sqlx::query("CREATE CONSTRAINT TRIGGER starting_progression_fail AFTER INSERT ON game_dawnport_departures DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION starting_progression_fail()") .execute(&h.pool).await?;
                        assert_eq!(
                            owner
                                .use_object(deps.actor, command(2), target(32054)?)
                                .await
                                .disposition,
                            UseDisposition::Rejected
                        );
                        assert_eq!(
                            sqlx::query_scalar::<_, i64>(
                                "SELECT count(*) FROM game_item_instances WHERE lifecycle=1"
                            )
                            .fetch_one(&h.pool)
                            .await?,
                            count
                        );
                        assert_eq!(
                            sqlx::query_scalar::<_, i64>(
                                "SELECT count(*) FROM game_dawnport_departures"
                            )
                            .fetch_one(&h.pool)
                            .await?,
                            0
                        );
                        sqlx::query(
                            "DROP TRIGGER starting_progression_fail ON game_dawnport_departures",
                        )
                        .execute(&h.pool)
                        .await?;
                    }
                    if vocation == "druid" {
                        owner.states.dawnport_departures.interrupt_after_commit.store(true, std::sync::atomic::Ordering::SeqCst);
                        assert_eq!(owner.use_object(deps.actor, command(2), target(32054)?).await.disposition, UseDisposition::Rejected);
                        sqlx::query("UPDATE game_durability_reconnect_sessions SET current_generation=current_generation+1 WHERE game_session_id=encode($1,'hex')::uuid")
                            .bind(session.as_bytes().as_slice()).execute(&h.pool).await?;
                        let (current, _) = crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone()).current_session_at(session).await?;
                        let refreshed = crate::gameplay_transport::item_fence_of(current).ok_or("refreshed fence")?;
                        command_fence.set(refreshed);
                        let mut resumed = deps.admitted;
                        resumed.item_fence = Some(refreshed);
                        owner.finish_resumed_session(resumed, false).await;
                        let (_, (x,y,floor)) = starting_loadout::departure(unique).ok_or("door")?;
                        assert_eq!(owner.runtime.lock().await.read_actor_position(deps.actor).map_err(debug)?.position(), MovementLocalPosition{x,y,floor}, "resume must install a committed departure before accepting input");
                    }
                    let result = owner
                        .use_object(deps.actor, command(2), target(32054)?)
                        .await;
                    assert_eq!(
                        result.disposition,
                        UseDisposition::Committed,
                        "{vocation}: {result:?}"
                    );
                    let (_, (x, y, floor)) = starting_loadout::departure(unique).ok_or("door")?;
                    assert_eq!(
                        owner
                            .runtime
                            .lock()
                            .await
                            .read_actor_position(deps.actor)
                            .map_err(debug)?
                            .position(),
                        MovementLocalPosition { x, y, floor }
                    );
                    let equipment = crate::gameplay_transport::equipment_move::read_current_equipment(&owner, session).await.ok_or("post-cleanup equipment")?;
                    let armor = crate::gameplay_transport::equipment_move::equipment_effects_owner::equipped_armor(&equipment, active.native_gameplay().ok_or("native")?).ok_or("post-cleanup armor")?;
                    {
                        let runtime = owner.runtime.lock().await;
                        let states = owner.states.spell_states.lock().await;
                        assert_eq!(states.get(&runtime, deps.actor, session).ok_or("live state")?.equipped_armor(), armor, "cleanup publishes the surviving equipment effects");
                    }
                    let saved: (i32, i32, i16) = sqlx::query_as(
                        "SELECT tile_x,tile_y,tile_floor FROM game_character_last_positions",
                    )
                    .fetch_one(&h.pool)
                    .await?;
                    assert_eq!(saved, (x, y, floor));
                    let prohibited:i64=sqlx::query_scalar("SELECT count(*) FROM game_item_instances WHERE lifecycle=1 AND game_dawnport_prohibited(definition_production_key)").fetch_one(&h.pool).await?;
                    assert_eq!(prohibited, 0, "all prohibited kit items removed");
                    if vocation == "knight" {
                        let nested: Vec<(String,i16)>=sqlx::query_as("SELECT definition_production_key,lifecycle FROM game_item_instances WHERE item_instance_id IN(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid) ORDER BY definition_production_key")
                            .bind(id(100).as_slice()).bind(id(101).as_slice()).bind(id(102).as_slice()).bind(id(103).as_slice()).fetch_all(&h.pool).await?;
                        assert_eq!(
                            nested,
                            vec![
                                ("oteryn:item.tibia.i268".into(), 2),
                                ("oteryn:item.tibia.i2853".into(), 1),
                                ("oteryn:item.tibia.i3447".into(), 2),
                                ("oteryn:item.tibia.i35562".into(), 2)
                            ]
                        );
                    }
                    assert_eq!(
                        sqlx::query_scalar::<_, i64>(
                            "SELECT count(*) FROM game_dawnport_departures"
                        )
                        .fetch_one(&h.pool)
                        .await?,
                        1
                    );
                    owner.clock_origin = owner
                        .clock_origin
                        .checked_sub(std::time::Duration::from_secs(1))
                        .ok_or("clock")?;
                    // Move neither value nor position on a new command after the once departure.
                    assert_eq!(
                        owner
                            .use_object(deps.actor, command(3), target(32054)?)
                            .await
                            .disposition,
                        if (x - 32054).abs() <= 1 {
                            UseDisposition::NothingToUse
                        } else {
                            UseDisposition::TooFar
                        }
                    );
                    assert_eq!(
                        sqlx::query_scalar::<_, i64>(
                            "SELECT count(*) FROM game_dawnport_departures"
                        )
                        .fetch_one(&h.pool)
                        .await?,
                        1
                    );
                }
                drop(owner);
                drop(authority);
                // Druid exercised a newer resumed connection; its old fresh-admission receipt
                // is correctly stale. The other three doors exercise cold first-entry restore.
                if unique != 0 && confirmed && vocation != "druid" {
                    h.root = super::fixture::tutorial_vitals_fixture::reopen_root(&h)?;
                    assert!(h.root.maintain_ready_once().await?);
                    let reopened = h
                        .root
                        .open_character_authority(&seal)
                        .await
                        .map_err(debug)?;
                    let restored = Dependencies::with_content(
                        ChannelContentPin::from_activation(
                            world_id,
                            pin.activation_sequence(),
                            pin.identity().server_artifact_digest(),
                            pin.identity().client_artifact_digest(),
                            world.frame_binding_digest(world_id),
                            world.map_revision_digest(),
                            (32053, 31883, 6),
                        ),
                        pin.qualified_room().clone(),
                        crate::spell::cast::v1_spell_book().map_err(debug)?,
                    )?;
                    let mut owner = restored.owner(&h, &reopened);
                    owner.active_generation = Some(active);
                    owner.movement_cells = (&cells).into();
                    admit(&owner, &restored, &request).await?;
                    let (_, (x, y, floor)) =
                        starting_loadout::departure(unique).ok_or("restored door")?;
                    assert_eq!(
                        restored
                            .runtime
                            .lock()
                            .await
                            .read_actor_position(restored.actor)
                            .map_err(debug)?
                            .position(),
                        MovementLocalPosition { x, y, floor }
                    );
                    assert_eq!(
                        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_item_instances")
                            .fetch_one(&h.pool)
                            .await?,
                        count,
                        "cold admission never remints the kit"
                    );
                    assert_eq!(
                        sqlx::query_scalar::<_, i64>(
                            "SELECT count(*) FROM game_dawnport_departures"
                        )
                        .fetch_one(&h.pool)
                        .await?,
                        1
                    );
                    drop(owner);
                    drop(reopened);
                }
                drop(seal);
                h.cleanup().await?;
            }
            Ok(())
        })
    })?;
    eprintln!(
        "SRVGAP_C1_VOCATIONS_PG_CASE_EXECUTED elapsed_ms={}",
        started.elapsed().as_millis()
    );
    Ok(())
}

/// Fixture-only carried graph: an equipped prohibited quiver containing permitted arrows,
/// and a retained bag containing a prohibited potion. All mechanic mutations use runtime SQL.
async fn seed_nested_cleanup(h: &Harness, active: &crate::content::ActiveGeneration) -> TestResult {
    use super::fixture::id;
    let index = active
        .native_gameplay()
        .and_then(|native| native.item_index())
        .ok_or("native item index")?;
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    for (tag, item, quantity) in [
        (100, 35562, 1),
        (101, 3447, 50),
        (102, 2853, 1),
        (103, 268, 4),
    ] {
        let key = format!("oteryn:item.tibia.i{item}");
        let revision = index.revision(&key).ok_or("fixture definition")?;
        sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item',$3,$4,$5,1,encode($6,'hex')::uuid)")
            .bind(id(tag).as_slice()).bind(id(42).as_slice()).bind(key).bind(revision).bind(i64::from(quantity)).bind(id(105).as_slice()).execute(&mut *tx).await?;
    }
    sqlx::query("INSERT INTO game_character_equipment_slots(character_id,slot,item_instance_id,world_id,placed_transaction_id) VALUES(encode($1,'hex')::uuid,10,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid)")
        .bind(id(41).as_slice()).bind(id(100).as_slice()).bind(id(42).as_slice()).bind(id(105).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_item_container_entries(character_id,item_instance_id,parent_item_instance_id,placement_ordinal,world_id,placed_transaction_id) SELECT character_id,encode($1,'hex')::uuid,item_instance_id,30,world_id,encode($2,'hex')::uuid FROM game_item_container_slots")
        .bind(id(102).as_slice()).bind(id(105).as_slice()).execute(&mut *tx).await?;
    for (child, parent) in [(101, 100), (103, 102)] {
        sqlx::query("INSERT INTO game_item_nested_entries(item_instance_id,parent_item_instance_id,placement_ordinal,world_id,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,encode($3,'hex')::uuid,encode($4,'hex')::uuid)")
            .bind(id(child).as_slice()).bind(id(parent).as_slice()).bind(id(42).as_slice()).bind(id(105).as_slice()).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    Ok(())
}
