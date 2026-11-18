//! Zalamon's physical USE, with both source storage gates in PostgreSQL.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::foundation::{ChannelContentPin, ChannelRuntimeV1, MovementLocalPosition};
use crate::gameplay_transport::connection::UseCommand;
use crate::map::view::MapFacts;
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};

#[test]
fn cotrev_zalamon_door_checks_both_quests_and_opens_onto_door_tile() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || scenario(admin).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "Zalamon scenario panicked")??;
    Ok(())
}

fn scenario(admin: String) -> TestResult {
    run(async move |_| {
        let harness =
            Harness::create_with_admission_seed(admin, "cotrev_door", false, false).await?;
        prepare_current_session(&harness).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut deps = Dependencies::new()?;
        let (controller, pin) = super::death_ml_loss_composed::production_generation_with_pin(
            deps.admitted.world_id,
            "content-1",
        )?;
        deps.room = pin.qualified_room().clone();
        let active = controller.active().ok_or("active")?;
        let native = active.native_gameplay().ok_or("native")?;
        let index = native.item_index().ok_or("index")?;
        let door = "oteryn:item.tibia.i9874";
        let coin = "oteryn:item.tibia.i3031";
        let reference = |key: &str| index.definition_ref("Item", key, "definition-r1");
        let (bytes, mut pins) = crate::map::boot::tests::bundle_with_item_ids(
            (door, reference(door).ok_or("door")?.get() - 1),
            (coin, reference(coin).ok_or("coin")?.get() - 1),
        );
        let mut data = oteryn_world_bundle::bundle::read(&bytes)?;
        data.sectors
            .iter_mut()
            .flat_map(|sector| &mut sector.tiles)
            .find(|tile| tile.x == 6)
            .and_then(|tile| tile.items.get_mut(1))
            .ok_or("door placement")?
            .attrs
            .unique = Some(3170);
        let bytes = oteryn_world_bundle_compiler::bundle::write_with_npcs(
            &data.manifest,
            &data.sectors,
            &data.spawns,
            &data.npcs,
        )?;
        pins.digest = oteryn_world_bundle::bundle::read(&bytes)?.digest;
        let base = crate::map::load(&bytes, &pins).map_err(debug)?;
        let map = crate::map::boot::boot(
            &bytes,
            &crate::map::boot::BootPins {
                map_revision: crate::map::overlay::map_revision(&base),
                bundle: pins,
                start: crate::map::overlay::TilePos {
                    x: 7,
                    y: 0,
                    floor: -7,
                },
            },
            deps.admitted.world_id,
            deps.admitted.channel_id,
            |key| crate::map::boot::native_item_definition(Some(native), key),
        )
        .map_err(debug)?;
        let cells = map
            .movement_cells(deps.room.movement_cells())
            .map_err(debug)?;
        let frame = map.frame_binding_digest(deps.admitted.world_id);
        deps.combat_room = Some(
            deps.room
                .bundle_view(
                    cells.clone(),
                    frame,
                    map.map_revision_digest(),
                    crate::content::NativeEntryStart {
                        x: 7,
                        y: 0,
                        floor: 7,
                    },
                )
                .ok_or("bundle combat view")?,
        );
        let content_pin = ChannelContentPin::from_activation(
            deps.admitted.world_id,
            pin.activation_sequence(),
            pin.identity().server_artifact_digest(),
            pin.identity().client_artifact_digest(),
            frame,
            map.map_revision_digest(),
            (7, 0, 7),
        );
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            deps.admitted.world_id,
            deps.admitted.channel_id,
            crate::foundation::NodeId::decode(&id(1)).map_err(debug)?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            2,
            content_pin,
        )
        .map_err(debug)?;
        let reservation = runtime
            .reserve_fresh_session(deps.admitted.game_session_id)
            .map_err(debug)?;
        deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .initialize_pinned_test_position(
                deps.actor,
                MovementLocalPosition {
                    x: 7,
                    y: 0,
                    floor: 7,
                },
            )
            .map_err(debug)?;
        deps.runtime = tokio::sync::Mutex::new(runtime);
        let mut owner = deps.owner(&harness, &authority);
        owner.active_generation = Some(active);
        owner.movement_cells = (&cells).into();
        deps.initialize(&owner).await;
        let mut admitted = deps.admitted;
        admitted.runtime_actor = Some(deps.actor);
        admitted.first_entry = super::FirstEntryOutcome::Positioned;
        let key = crate::map::boot::tests::door_key();
        let (door_tile, ordinal) = crate::map::view::placement_of(key).ok_or("door tile")?;
        let door_id = reference(door).ok_or("door definition")?.get() - 1;
        let appearance = || -> TestResult<u16> {
            Ok(map
                .facts()
                .base_entry(door_tile, ordinal, door_id)
                .ok_or("door facts")?
                .appearance_id)
        };
        let mut placement = map.base().digest().to_vec();
        placement.extend_from_slice(&key.to_be_bytes());
        let target = WorldObjectTarget {
            placement,
            expected_revision: 0,
        };
        owner.quest_catalogue = Some(std::sync::Arc::new(
            oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
                .map_err(debug)?
                .catalogue()
                .clone(),
        ));
        for (command_id, seed, disposition) in [
            (1, None, UseDisposition::Locked),
            (
                2,
                Some((
                    "u8_6/wrath_of_the_emperor/mission11",
                    "wrath_of_the_emperor",
                    2,
                )),
                UseDisposition::Locked,
            ),
            (
                3,
                Some(("u8_54/the_new_frontier/mission08", "the_new_frontier", 2)),
                UseDisposition::Locked,
            ),
            (
                4,
                Some((
                    "u8_6/wrath_of_the_emperor/mission11",
                    "wrath_of_the_emperor",
                    1,
                )),
                UseDisposition::Committed,
            ),
        ] {
            if let Some((track, quest, value)) = seed {
                let mut tx = harness.pool.begin().await?;
                sqlx::query("SET LOCAL session_replication_role=replica")
                    .execute(&mut *tx)
                    .await?;
                sqlx::query("INSERT INTO game_character_quest_tracks(character_id,track_key,quest_key,value,committed_character_revision) VALUES(encode($1,'hex')::uuid,$2,$3,$4,2) ON CONFLICT(character_id,track_key) DO UPDATE SET value=excluded.value")
                    .bind(id(41).as_slice())
                    .bind(format!("oteryn:quest-progress/quest/{track}"))
                    .bind(format!("oteryn:quest.{quest}_quest"))
                    .bind(i64::from(value))
                    .execute(&mut *tx).await?;
                tx.commit().await?;
            }
            owner.admit_quest_session(&admitted).await;
            owner.clock_origin -= std::time::Duration::from_secs(1);
            let result = owner
                .use_world_object(
                    deps.actor,
                    UseCommand {
                        game_session_id: admitted.game_session_id,
                        command_id,
                        item_fence: None,
                    },
                    target.clone(),
                )
                .await;
            assert_eq!(result.disposition, disposition);
            let position = deps
                .runtime
                .lock()
                .await
                .read_actor_position(deps.actor)
                .map_err(debug)?;
            assert_eq!(appearance()?, if command_id == 4 { 9875 } else { 9874 });
            assert_eq!(position.position().x, if command_id == 4 { 6 } else { 7 });
            assert_eq!(
                map.facts().doors().revision(key),
                u64::from(command_id == 4)
            );
        }
        let opened_at = deps
            .runtime
            .lock()
            .await
            .read_actor_position(deps.actor)
            .map_err(debug)?;
        owner.clock_origin -= std::time::Duration::from_secs(1);
        let result = owner
            .use_world_object(
                deps.actor,
                UseCommand {
                    game_session_id: admitted.game_session_id,
                    command_id: 5,
                    item_fence: None,
                },
                WorldObjectTarget {
                    expected_revision: 1,
                    ..target
                },
            )
            .await;
        assert_eq!(result.disposition, UseDisposition::Committed);
        assert_eq!(appearance()?, 9875);
        assert_eq!(map.facts().doors().revision(key), 1);
        assert_eq!(
            deps.runtime
                .lock()
                .await
                .read_actor_position(deps.actor)
                .map_err(debug)?,
            opened_at
        );
        drop(owner);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}
