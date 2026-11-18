//! The four plates, shared source schedule, and durable survivor credit through STEP.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::foundation::{ChannelContentPin, ChannelRuntimeV1, MovementLocalPosition};
use crate::foundation::{ExactActorRef, GameSessionId};
use crate::map::{overlay::TilePos, view::MapFacts};
use oteryn_protocol_oteryn::world_spatial::{StepDirection, StepDisposition};
use oteryn_world_bundle::sector::{Attrs, Item, Tile};
use oteryn_world_bundle::writer::Sector;
use std::collections::BTreeMap;
use std::sync::atomic::Ordering;

const STAIR: TilePos = TilePos {
    x: 33265,
    y: 31116,
    floor: -8,
};
const RETURN: MovementLocalPosition = MovementLocalPosition {
    x: 33353,
    y: 31410,
    floor: 8,
};
const QUESTLINE: &str = "oteryn:quest-progress/quest/u8_54/children_of_the_revolution/questline";

#[test]
fn cotrev_phantom_army_step_in_waves_and_survivor_credit() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || scenario(admin).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "Phantom Army scenario panicked")??;
    Ok(())
}

fn scenario(admin: String) -> TestResult {
    run(async move |_| {
        let harness =
            Harness::create_with_admission_seed(admin, "cotrev_army", false, false).await?;
        prepare_current_session(&harness).await?;
        let now = sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
            .fetch_one(&harness.pool)
            .await?;
        let guest = crate::world_party_runtime_postgres_cases::committed_fresh_player_owned(
            &crate::world_party_runtime_postgres_cases::FreshPlayerOwners {
                root: harness.root.clone(),
                node: harness.node.clone(),
                recovery: &harness.recovery,
            },
            61,
            63,
            71,
            now,
        )
        .await?;
        let guest_session = guest.current_game_session_id();
        let guest_lease = guest.current_character_lease();
        let guest_character = guest_lease.character_id();
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
        let stair = "oteryn:item.tibia.i1897";
        let open = "oteryn:item.tibia.i1977";
        let coin = "oteryn:item.tibia.i3031";
        let reference = |key: &str| index.definition_ref("Item", key, "definition-r1");
        let compact = |key: &str| {
            reference(key)
                .map(|value| value.get() - 1)
                .ok_or("item index")
        };
        let (template, mut pins) = crate::map::boot::tests::bundle_with_item_states(
            (stair, compact(stair)?),
            (coin, compact(coin)?),
            &[(open, compact(open)?)],
        );
        let mut data = oteryn_world_bundle::bundle::read(&template)?;
        data.manifest.world.min_x = 33240;
        data.manifest.world.min_y = 31070;
        data.manifest.world.max_x = 33355;
        data.manifest.world.max_y = 31412;
        data.manifest.world.floors = vec![-8, -7];
        let mut sectors = BTreeMap::<_, Vec<_>>::new();
        let mut add = |floor: i8, x: u16, y: u16, ids: &[u32]| {
            sectors
                .entry((floor, y / 32, x / 32))
                .or_default()
                .push(Tile {
                    x,
                    y,
                    flags: 0,
                    house: 0,
                    zones: Vec::new(),
                    items: ids
                        .iter()
                        .map(|&palette| Item {
                            palette,
                            depth: 0,
                            attrs: Attrs::default(),
                        })
                        .collect(),
                });
        };
        for (x, y) in [
            (33257, 31080),
            (33258, 31080),
            (33266, 31084),
            (33259, 31089),
            (33263, 31093),
            (33264, 31116),
            (33353, 31410),
        ] {
            add(-8, x, y, &[0]);
        }
        add(-8, STAIR.x, STAIR.y, &[0, 3]);
        for x in 33252..=33288 {
            for y in 31105..=31134 {
                add(-7, x, y, &[0]);
            }
        }
        let sectors = sectors
            .into_iter()
            .map(|((floor, sy, sx), mut tiles)| {
                tiles.sort_by_key(|tile| (tile.y, tile.x));
                Sector {
                    floor,
                    sx,
                    sy,
                    tiles,
                }
            })
            .collect::<Vec<_>>();
        let bytes = oteryn_world_bundle_compiler::bundle::write_with_npcs(
            &data.manifest,
            &sectors,
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
                    x: 33257,
                    y: 31080,
                    floor: -8,
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
                        x: 33257,
                        y: 31080,
                        floor: 8,
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
            (33257, 31080, 8),
        );
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            deps.admitted.world_id,
            deps.admitted.channel_id,
            crate::foundation::NodeId::decode(&id(1)).map_err(debug)?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            512,
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
                    x: 33257,
                    y: 31080,
                    floor: 8,
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
        owner.quest_catalogue = Some(std::sync::Arc::new(
            oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
                .map_err(debug)?
                .catalogue()
                .clone(),
        ));
        let mut seed = harness.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *seed)
            .await?;
        for character in [id(41), *guest_character.as_bytes()] {
            sqlx::query("UPDATE game_character_roots SET character_revision=greatest(character_revision,2) WHERE character_id=encode($1,'hex')::uuid")
                .bind(character.as_slice()).execute(&mut *seed).await?;
            sqlx::query("INSERT INTO game_character_quest_tracks(character_id,track_key,quest_key,value,committed_character_revision) VALUES(encode($1,'hex')::uuid,$2,'oteryn:quest.children_of_the_revolution_quest',19,2)")
                .bind(character.as_slice()).bind(QUESTLINE).execute(&mut *seed).await?;
        }
        seed.commit().await?;
        owner.admit_quest_session(&admitted).await;
        let players = {
            let mut runtime = owner.runtime.lock().await;
            let mut players = Vec::new();
            for (tag, x, y) in [(51, 33266, 31084), (52, 33259, 31089), (53, 33263, 31093)] {
                let session = if tag == 51 {
                    guest_session
                } else {
                    GameSessionId::decode(&id(tag)).map_err(debug)?
                };
                let reservation = runtime.reserve_fresh_session(session).map_err(debug)?;
                let actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
                if session == guest_session {
                    runtime
                        .bind_attacker_lease(actor, session, guest_lease)
                        .map_err(debug)?;
                }
                runtime
                    .initialize_pinned_test_position(
                        actor,
                        MovementLocalPosition { x, y, floor: 8 },
                    )
                    .map_err(debug)?;
                players.push((actor, session));
            }
            players
        };
        let (guest, guest_session) = players[0];
        deps.initialize_player(&owner, guest, guest_session).await;
        let key = crate::map::view::placement_key(STAIR, 1).ok_or("stair key")?;
        let appearance = || -> TestResult<u16> {
            Ok(map
                .facts()
                .base_entry(STAIR, 1, compact(stair)?)
                .ok_or("stair facts")?
                .appearance_id)
        };
        let _clock = owner.freeze_owner_clock_for_test();
        let time = |value| owner.states.test_owner_time.store(value, Ordering::Relaxed);
        let started = 1_000_000;
        time(started);
        assert_eq!(appearance()?, 1897);
        let step = owner
            .ordinary_paced_step(
                deps.actor,
                admitted.game_session_id,
                StepDirection::East,
                None,
                None,
            )
            .await
            .0;
        assert_eq!(step.disposition, StepDisposition::Moved);
        assert_eq!(appearance()?, 1977);
        assert_eq!(map.facts().object_revision(key), 1);

        arrive(
            &owner,
            deps.actor,
            admitted.game_session_id,
            MovementLocalPosition {
                x: 33264,
                y: 31116,
                floor: 8,
            },
        )
        .await?;
        time(started + 5_000_000);
        let step = owner
            .ordinary_paced_step(
                deps.actor,
                admitted.game_session_id,
                StepDirection::East,
                None,
                None,
            )
            .await
            .0;
        assert_eq!(step.disposition, StepDisposition::Moved);
        assert_eq!(
            owner
                .runtime
                .lock()
                .await
                .read_actor_position(deps.actor)
                .map_err(debug)?
                .position()
                .floor,
            7
        );
        arrive(
            &owner,
            deps.actor,
            admitted.game_session_id,
            MovementLocalPosition {
                x: 33268,
                y: 31119,
                floor: 7,
            },
        )
        .await?;
        arrive(
            &owner,
            guest,
            guest_session,
            MovementLocalPosition {
                x: 33264,
                y: 31116,
                floor: 8,
            },
        )
        .await?;
        time(started + 10_000_000);
        let step = owner
            .ordinary_paced_step(guest, guest_session, StepDirection::East, None, None)
            .await
            .0;
        assert_eq!(step.disposition, StepDisposition::Moved);
        assert_eq!(
            owner
                .runtime
                .lock()
                .await
                .read_actor_position(guest)
                .map_err(debug)?
                .position()
                .floor,
            7
        );

        time(started + 29_999_999);
        owner.drain_quest_encounter_deadlines().await;
        assert_eq!(
            owner
                .runtime
                .lock()
                .await
                .current_live_creature_census()
                .map_err(debug)?
                .len(),
            0
        );
        time(started + 30_000_000);
        owner.tick_timed_items().await;
        owner.drain_quest_encounter_deadlines().await;
        assert_eq!(appearance()?, 1897);
        assert_eq!(
            owner
                .runtime
                .lock()
                .await
                .current_live_creature_census()
                .map_err(debug)?
                .len(),
            20
        );
        time(started + 120_000_000);
        owner.drain_quest_encounter_deadlines().await;
        assert_eq!(
            owner
                .runtime
                .lock()
                .await
                .current_live_creature_census()
                .map_err(debug)?
                .len(),
            80
        );
        time(started + 600_000_000);
        owner.drain_quest_encounter_deadlines().await;
        let runtime = owner.runtime.lock().await;
        assert_eq!(
            runtime
                .read_actor_position(deps.actor)
                .map_err(debug)?
                .position(),
            RETURN
        );
        assert_eq!(
            runtime
                .read_actor_position(guest)
                .map_err(debug)?
                .position(),
            RETURN
        );
        assert_ne!(
            runtime
                .read_actor_position(players[1].0)
                .map_err(debug)?
                .position(),
            RETURN
        );
        assert!(
            runtime
                .current_live_creature_census()
                .map_err(debug)?
                .is_empty()
        );
        drop(runtime);
        for character in [id(41), *guest_character.as_bytes()] {
            let state: i64 = sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2")
                .bind(character.as_slice()).bind(QUESTLINE).fetch_one(&harness.pool).await?;
            assert_eq!(state, 20, "survivor credit reached PostgreSQL");
        }
        drop(owner);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

async fn arrive(
    owner: &super::ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    session: GameSessionId,
    destination: MovementLocalPosition,
) -> TestResult {
    let map = owner.bundle_map().ok_or("map")?;
    let mut runtime = owner.runtime.lock().await;
    let expected = runtime.read_actor_position(actor).map_err(debug)?;
    let closed = crate::map::floor::BundleDoors(map.facts.doors());
    let proof = crate::map::floor::prepare_quest_encounter_ejection(
        &runtime,
        owner.movement_cells(),
        actor,
        session,
        expected,
        destination,
        &closed,
    )
    .map_err(debug)?;
    runtime.commit_source_step(proof).map_err(debug)?;
    Ok(())
}
