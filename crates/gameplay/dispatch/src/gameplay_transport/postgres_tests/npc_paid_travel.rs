//! TRAVEL: production NPC_TALK router, runtime movement owner and runtime-role PostgreSQL.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::content::native_gameplay::test_support::activated_with_loot_tables;
use crate::content::{NpcDataCatalogue, npc_catalogue_preproduction_limits};
use crate::foundation::{
    ChannelContentPin, MovementFacing, NpcPlacement, NpcPlacementTable, NpcPlacements,
};
use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
use crate::gameplay_transport::npc_talk::NpcTalkBook;
use crate::gameplay_transport::spell_premium_coordinator::{
    SpellPremiumCoordinator, purchase_fixture,
};
use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTalkIntent};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use std::sync::Arc;

#[test]
fn npc_paid_travel_postgres_dispatch_atomic_payment_arrival_replay_and_refusals() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "paid travel thread panicked")??;
    Ok(())
}

async fn talk(
    owner: &super::ComposedFreshAdmission<'_, '_, '_>,
    dependencies: &Dependencies,
    npc: EntityRef,
    command_id: u64,
    text: &str,
) -> TestResult<crate::gameplay_transport::npc_command::NpcTalkOutcome> {
    let fence = owner
        .current_quest_fence(dependencies.admitted.game_session_id)
        .await
        .map_err(|_| "current fence")?
        .ok_or("live session")?
        .item_fence();
    let result = owner
        .npc_talk(
            dependencies.actor,
            UseCommand {
                game_session_id: dependencies.admitted.game_session_id,
                command_id,
                item_fence: Some(fence),
            },
            NpcTalkIntent {
                npc_actor: npc,
                text: text.into(),
            },
        )
        .await;
    assert_eq!(
        result.disposition,
        NpcIntentDisposition::Ok,
        "{text}: {}",
        line(&result)
    );
    Ok(result)
}

fn line(outcome: &crate::gameplay_transport::npc_command::NpcTalkOutcome) -> String {
    outcome
        .conversation
        .as_ref()
        .map(|conversation| conversation.lines.join(" "))
        .unwrap_or_default()
}

fn scenario() -> TestResult {
    run(async |admin| {
        reset();
        let h = Harness::create_with_admission_seed(admin, "npc_paid_travel", true, false).await?;
        eprintln!("POSTGRES_STARTED TRAVEL npc_paid_travel");
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
        let (controller, pin) = activated_with_loot_tables(world, None);
        let active = controller.active().ok_or("active content")?;
        let bundle = travel_world(world)?;
        let base_room = pin.qualified_room().clone();
        let cells = bundle
            .movement_cells(base_room.movement_cells())
            .map_err(debug)?;
        let start = bundle.start();
        let channel_pin = ChannelContentPin::from_activation(
            world,
            pin.activation_sequence(),
            pin.identity().server_artifact_digest(),
            pin.identity().client_artifact_digest(),
            bundle.frame_binding_digest(world),
            bundle.map_revision_digest(),
            (
                i32::from(start.x),
                i32::from(start.y),
                -i16::from(start.floor),
            ),
        );
        let mut dependencies = Dependencies::with_content(
            channel_pin.clone(),
            base_room,
            crate::spell::cast::v1_spell_book().map_err(debug)?,
        )?;
        // One player plus both NPCs need three carrier slots; the shared seam defaults to two.
        let mut runtime = crate::foundation::ChannelRuntimeV1::from_committed_assignment(
            world,
            dependencies.admitted.channel_id,
            crate::foundation::NodeId::decode(&id(1)).map_err(debug)?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            4,
            channel_pin,
        )
        .map_err(debug)?;
        let reservation = runtime
            .reserve_fresh_session(dependencies.admitted.game_session_id)
            .map_err(debug)?;
        dependencies.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        dependencies.admitted.runtime_actor = Some(dependencies.actor);
        dependencies.runtime = tokio::sync::Mutex::new(runtime);
        let (consumer, clock, activation, _) = purchase_fixture::current(id(40));
        h.root
            .latch_premium_activation(
                &activation.activation_id,
                activation.switch_over_us,
                activation.switch_over_us + 60_000_000,
            )
            .await
            .map_err(debug)?;
        let coordinator = SpellPremiumCoordinator::new(
            consumer,
            Some(clock.clone()),
            Some(activation.clone()),
            Some(h.root.clone()),
        );
        let free = SpellPremiumCoordinator::new(
            Arc::default(),
            Some(clock),
            Some(activation),
            Some(h.root.clone()),
        );
        let mut owner = dependencies.owner(&h, &authority);
        owner.active_generation = Some(active);
        owner.movement_cells = &cells;
        owner.premium_coordinator = Some(&coordinator);
        owner.admit_premium(id(40), dependencies.admitted.game_session_id);
        let catalogue = NpcDataCatalogue::load(
            std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content")),
            std::ffi::OsStr::new("world"),
            npc_catalogue_preproduction_limits(),
        )
        .map_err(debug)?;
        let model = catalogue.service_model_for_project().map_err(debug)?;
        owner
            .states
            .chat
            .install_npc_talk(Arc::new(NpcTalkBook::from_model(&model)));
        let session = dependencies.admitted.game_session_id;
        owner
            .states
            .chat
            .admit(session, "Fixture Hero", owner.owner_now());
        owner.states.chat.set_npc_service(session);
        let npc = {
            let mut runtime = dependencies.runtime.lock().await;
            runtime
                .initialize_first_entry_position(dependencies.actor)
                .map_err(debug)?;
            let spot = runtime
                .read_actor_position(dependencies.actor)
                .map_err(debug)?
                .position();
            runtime
                .place_npcs(
                    &NpcPlacementTable {
                        npcs: vec![
                            NpcPlacements {
                                key: "oteryn:npc.captain_bluebear".into(),
                                placements: vec![NpcPlacement {
                                    floor: -i8::try_from(spot.floor)?,
                                    x: u16::try_from(spot.x + 1)?,
                                    y: u16::try_from(spot.y)?,
                                    direction: MovementFacing::South,
                                }],
                            },
                            NpcPlacements {
                                key: "oteryn:npc.pino".into(),
                                placements: vec![NpcPlacement {
                                    floor: -i8::try_from(spot.floor)?,
                                    x: u16::try_from(spot.x + 2)?,
                                    y: u16::try_from(spot.y)?,
                                    direction: MovementFacing::South,
                                }],
                            },
                        ],
                    },
                    |_| true,
                    true,
                )
                .map_err(debug)?;
            let placed = runtime.visible_entities().npcs[0];
            EntityRef {
                identity: placed.actor.placement_identity(),
                generation: placed.generation,
            }
        };
        talk(&owner, &dependencies, npc, 1, "hi").await?;
        assert!(line(&talk(&owner, &dependencies, npc, 2, "venore").await?).contains("170 gold"));
        let refused = talk(&owner, &dependencies, npc, 3, "yes").await?;
        assert!(
            line(&refused).contains("don't have enough money"),
            "{}",
            line(&refused)
        );
        assert!(!refused.inventory_changed);
        let empty: i64 = sqlx::query_scalar("SELECT count(*) FROM game_newhaven_travel_receipts")
            .fetch_one(&h.pool)
            .await?;
        assert_eq!(empty, 0);
        // Current Premium is checked by the native owner in the quote and again in COMMIT.
        owner.premium_coordinator = Some(&free);
        assert!(
            line(&talk(&owner, &dependencies, npc, 4, "edron").await?).contains("premium account")
        );
        owner.premium_coordinator = Some(&coordinator);
        tokio::time::sleep(std::time::Duration::from_millis(1050)).await;
        talk(&owner, &dependencies, npc, 5, "venore").await?;
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx)
            .await?;
        sqlx::query("INSERT INTO game_character_pvp_state(character_id,world_id,pz_block_until) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,clock_timestamp()+interval '1 minute')")
            .bind(id(41).as_slice()).bind(id(42).as_slice()).execute(&mut *tx).await?;
        tx.commit().await?;
        assert!(line(&talk(&owner, &dependencies, npc, 6, "yes").await?).contains("blood stains"));
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx)
            .await?;
        // A logout block alone permits passage. Fifty carried gold plus 120 bank gold pays 170.
        sqlx::query("UPDATE game_character_pvp_state SET pz_block_until=NULL,logout_block_until=clock_timestamp()+interval '1 minute' WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).execute(&mut *tx).await?;
        sqlx::raw_sql(sqlx::AssertSqlSafe(format!(
            "{}{}",
            crate::durability::item_fee_burn::test_support::item(
                70,
                "oteryn:item.tibia.i2854",
                "definition-r1",
                1
            ),
            crate::durability::item_fee_burn::test_support::item(
                72,
                "oteryn:item.tibia.i3031",
                "definition-r1",
                50
            )
        )))
        .execute(&mut *tx)
        .await?;
        sqlx::query("INSERT INTO game_item_container_slots(character_id,item_instance_id,world_id,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid)").bind(id(41).as_slice()).bind(id(70).as_slice()).bind(id(42).as_slice()).bind(id(74).as_slice()).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_item_container_entries(character_id,parent_item_instance_id,item_instance_id,world_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,1,encode($5,'hex')::uuid)").bind(id(41).as_slice()).bind(id(70).as_slice()).bind(id(72).as_slice()).bind(id(42).as_slice()).bind(id(75).as_slice()).execute(&mut *tx).await?;
        sqlx::raw_sql("INSERT INTO game_type2_audit_activation VALUES(1,now())")
            .execute(&mut *tx)
            .await?;
        // A well-shaped migration-owner seed retains a ledger head for the bank balance.
        seed_bank(&mut tx).await?;
        tx.commit().await?;
        // Fail after payment but before arrival insertion. The transaction must roll back all value.
        sqlx::raw_sql("CREATE FUNCTION travel_test_fail() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'TRAVEL injected arrival failure'; END; $$; CREATE TRIGGER travel_test_fail BEFORE INSERT ON game_character_pending_arrivals FOR EACH ROW EXECUTE FUNCTION travel_test_fail()").execute(&h.pool).await?;
        talk(&owner, &dependencies, npc, 7, "venore").await?;
        let current = owner
            .current_quest_fence(session)
            .await
            .map_err(|_| "fence")?
            .ok_or("session")?;
        let failed = owner
            .npc_talk(
                dependencies.actor,
                UseCommand {
                    game_session_id: session,
                    command_id: 8,
                    item_fence: Some(current.item_fence()),
                },
                NpcTalkIntent {
                    npc_actor: npc,
                    text: "yes".into(),
                },
            )
            .await;
        assert!(failed.unknown, "{}", line(&failed));
        assert!(owner.states.newhaven_travel.held(session));
        assert_eq!(bank(&h).await?, 200);
        assert_eq!(fees(&h).await?, 0);
        let carried: i64 = sqlx::query_scalar("SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid").bind(id(72).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(carried, 50);
        sqlx::raw_sql("DROP TRIGGER travel_test_fail ON game_character_pending_arrivals; DROP FUNCTION travel_test_fail()").execute(&h.pool).await?;
        let request = {
            let holds = owner
                .states
                .newhaven_travel
                .holds
                .lock()
                .map_err(|_| "holds")?;
            match &holds.get(&session).ok_or("held travel")?.route {
                crate::gameplay_transport::newhaven_travel::Departure::Paid(request) => {
                    (**request).clone()
                }
                _ => return Err("paid request".into()),
            }
        };
        // Simulate COMMIT acknowledged by PostgreSQL but lost to the connection.
        assert!(
            h.root
                .commit_npc_travel(&authority, &h.node, current, request.clone(), &coordinator)
                .await
                .map_err(debug)?
                .map_err(debug)?
                .is_some()
        );
        assert_eq!(bank(&h).await?, 80);
        assert_eq!(fees(&h).await?, 1);
        let resumed = owner
            .resume_npc_bank(
                dependencies.actor,
                UseCommand {
                    game_session_id: session,
                    command_id: 8,
                    item_fence: Some(current.item_fence()),
                },
            )
            .await
            .ok_or("resume retained travel")?;
        assert!(!resumed.unknown);
        assert!(resumed.inventory_changed);
        let position = dependencies
            .runtime
            .lock()
            .await
            .read_actor_position(dependencies.actor)
            .map_err(debug)?
            .position();
        assert_eq!((position.x, position.y, position.floor), (32954, 32022, 6));
        let saved:(i32,i32,i16)=sqlx::query_as("SELECT tile_x,tile_y,tile_floor FROM game_character_last_positions WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(saved, (32954, 32022, 6));
        let carried: i64 = sqlx::query_scalar("SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid").bind(id(72).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(carried, 0);
        let pending: i64 =
            sqlx::query_scalar("SELECT count(*) FROM game_character_pending_arrivals")
                .fetch_one(&h.pool)
                .await?;
        assert_eq!(pending, 0);
        assert_eq!(
            h.root
                .commit_npc_travel(&authority, &h.node, current, request.clone(), &coordinator)
                .await
                .map_err(debug)?
                .map_err(debug)?,
            None
        );
        assert_eq!(bank(&h).await?, 80);
        assert_eq!(fees(&h).await?, 1);
        // Replaying old work under a forged successor session generation cannot reacquire authority.
        let mut stale = current;
        stale.connection_generation =
            crate::foundation::ConnectionGeneration::new(current.connection_generation.get() + 1)
                .map_err(debug)?;
        assert!(
            h.root
                .commit_npc_travel(&authority, &h.node, stale, request, &coordinator)
                .await
                .is_err()
        );
        assert_eq!(fees(&h).await?, 1);
        // The same production talk owner serves Premium carpets and their route-specific discount.
        tokio::time::sleep(std::time::Duration::from_millis(1050)).await;
        sqlx::query("UPDATE game_newhaven_travel_receipts SET arrived_at=clock_timestamp()-interval '4 seconds'").execute(&h.pool).await?;
        let pilot = {
            let mut runtime = dependencies.runtime.lock().await;
            runtime
                .place_admitted_respawn(
                    dependencies.actor,
                    session,
                    Some(crate::foundation::MovementLocalPosition {
                        x: 32310,
                        y: 32210,
                        floor: 6,
                    }),
                    |_| true,
                )
                .map_err(debug)?;
            let placed = runtime
                .visible_entities()
                .npcs
                .into_iter()
                .find(|npc| runtime.npc_key(npc.actor) == Some("oteryn:npc.pino"))
                .ok_or("carpet pilot")?;
            EntityRef {
                identity: placed.actor.placement_identity(),
                generation: placed.generation,
            }
        };
        talk(&owner, &dependencies, pilot, 10, "hi").await?;
        assert!(
            line(&talk(&owner, &dependencies, pilot, 11, "darashia").await?).contains("40 gold")
        );
        owner.premium_coordinator = Some(&free);
        assert!(
            line(&talk(&owner, &dependencies, pilot, 12, "yes").await?).contains("premium account")
        );
        assert_eq!(bank(&h).await?, 80);
        owner.premium_coordinator = Some(&coordinator);
        talk(&owner, &dependencies, pilot, 13, "darashia").await?;
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx)
            .await?;
        sqlx::query("INSERT INTO game_character_quest_tracks(character_id,track_key,quest_key,value,committed_character_revision) VALUES(encode($1,'hex')::uuid,'oteryn:quest-progress/quest/u7_24/the_postman_missions/mission06','oteryn:quest.the_postman_missions_quest',13,2)").bind(id(41).as_slice()).execute(&mut *tx).await?;
        tx.commit().await?;
        tokio::time::sleep(std::time::Duration::from_millis(1050)).await;
        assert!(
            line(&talk(&owner, &dependencies, pilot, 14, "yes").await?)
                .contains("offer has expired")
        );
        assert_eq!(bank(&h).await?, 80);
        tokio::time::sleep(std::time::Duration::from_millis(1050)).await;
        assert!(line(&talk(&owner, &dependencies, pilot, 15, "darama").await?).contains("30 gold"));
        let travelled = talk(&owner, &dependencies, pilot, 16, "yes").await?;
        assert!(travelled.inventory_changed);
        assert_eq!(bank(&h).await?, 50);
        assert_eq!(fees(&h).await?, 2);
        let position = dependencies
            .runtime
            .lock()
            .await
            .read_actor_position(dependencies.actor)
            .map_err(debug)?
            .position();
        assert_eq!((position.x, position.y, position.floor), (33270, 32441, 6));
        // Exhaustion is checked before attempting a fee, unlike the donor's debit-first order.
        dependencies
            .runtime
            .lock()
            .await
            .place_admitted_respawn(
                dependencies.actor,
                session,
                Some(crate::foundation::MovementLocalPosition {
                    x: 32310,
                    y: 32210,
                    floor: 6,
                }),
                |_| true,
            )
            .map_err(debug)?;
        talk(&owner, &dependencies, pilot, 17, "hi").await?;
        assert!(
            line(&talk(&owner, &dependencies, pilot, 18, "darashia").await?)
                .contains("wait three seconds")
        );
        assert_eq!(bank(&h).await?, 50);
        assert_eq!(fees(&h).await?, 2);
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

async fn bank(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT balance FROM game_account_bank_balances WHERE account_id=encode($1,'hex')::uuid",
    )
    .bind(id(40).as_slice())
    .fetch_one(&h.pool)
    .await?)
}
async fn fees(h: &Harness) -> TestResult<i64> {
    Ok(
        sqlx::query_scalar("SELECT count(*) FROM game_item_fee_burns WHERE cause_kind=8")
            .fetch_one(&h.pool)
            .await?,
    )
}

async fn seed_bank(tx: &mut sqlx::Transaction<'_, sqlx::Postgres>) -> TestResult {
    let hex = |n: u8| id(n).iter().map(|b| format!("{b:02x}")).collect::<String>();
    sqlx::raw_sql(sqlx::AssertSqlSafe(format!("INSERT INTO game_account_bank_operations(operation_occurrence_id,transaction_id,event_id,kind,request_binding,acting_character_id,account_id,world_id,channel_id,runtime_scope_ownership_generation,amount,outcome,occurred_at,envelope_sha256,committed_at,planned_platinum_item_instance_id,planned_gold_item_instance_id,backpack_item_instance_id) VALUES('{occ}'::uuid,'{op}'::uuid,'{event}'::uuid,1,sha256('seed'),'{character}'::uuid,'{account}'::uuid,'{world}'::uuid,'{channel}'::uuid,1,1,0,1,sha256('seed'),0,'{p}'::uuid,'{g}'::uuid,'{backpack}'::uuid); INSERT INTO game_account_bank_entries(entry_id,transaction_id,account_id,world_id,kind,amount,balance_before,balance_after,acting_character_id) VALUES('{entry}'::uuid,'{op}'::uuid,'{account}'::uuid,'{world}'::uuid,1,200,0,200,'{character}'::uuid); INSERT INTO game_account_bank_balances VALUES('{account}'::uuid,'{world}'::uuid,200,'{entry}'::uuid);",occ=hex(80),op=hex(81),event=hex(82),character=hex(41),account=hex(40),world=hex(42),channel=hex(43),p=hex(83),g=hex(84),backpack=hex(70),entry=hex(85)))).execute(&mut **tx).await?;
    Ok(())
}

fn travel_world(world: crate::foundation::WorldId) -> TestResult<crate::map::boot::BundleWorld> {
    use oteryn_world_bundle::sector::{Attrs, Item, Tile};
    use oteryn_world_bundle::writer::{
        self as writer, BuildClass, Extent, Family, Identity, Manifest, PaletteEntry, Sector,
        Terrain, TerrainKind,
    };
    let manifest = Manifest {
        format: writer::FORMAT.into(),
        min_reader_version: writer::VERSION,
        projection_class: "server".into(),
        compiler_version: "engineering".into(),
        build_class: BuildClass::Production,
        identity: Identity {
            project_format_version: "OTERYN_WORLD_PROJECT/v2".into(),
            world_schema_version: "world-schema-1".into(),
            content_revision: "rev-1".into(),
            ..Identity::default()
        },
        world: Extent {
            min_x: 0,
            min_y: 0,
            max_x: 65535,
            max_y: 65535,
            floors: vec![-7, -6],
        },
        palette: vec![PaletteEntry {
            key: "terrain:grass".into(),
            family: Family::Terrain,
            id: 0,
            terrain: Some(Terrain {
                kind: TerrainKind::Ground,
                walkable: Some(true),
                ground_speed: Some(150),
            }),
        }],
        draft_areas: vec![],
        skipped_provisional_keys: vec![],
        dropped_teleports: vec![],
        spawns: Default::default(),
        npcs: Default::default(),
    };
    let points = [
        (32310u16, 32210u16, -6i8),
        (32311, 32210, -6),
        (32954, 32022, -6),
        (33175, 31764, -6),
        (32734, 31668, -6),
        (33270, 32441, -6),
    ];
    let mut grouped = std::collections::BTreeMap::<_, Vec<Tile>>::new();
    for (x, y, floor) in points {
        grouped
            .entry((floor, y / 32, x / 32))
            .or_default()
            .push(Tile {
                x,
                y,
                flags: 0,
                house: 0,
                zones: vec![],
                items: vec![Item {
                    palette: 0,
                    depth: 0,
                    attrs: Attrs::default(),
                }],
            });
    }
    let sectors = grouped
        .into_iter()
        .map(|((floor, sy, sx), tiles)| Sector {
            floor,
            sx,
            sy,
            tiles,
        })
        .collect::<Vec<_>>();
    let bytes = writer::write(&manifest, &sectors, &Default::default()).map_err(debug)?;
    let digest = writer::read(&bytes).map_err(debug)?.digest;
    let pins = crate::map::boot::BootPins {
        bundle: crate::map::BundlePins {
            digest,
            project_format_version: "OTERYN_WORLD_PROJECT/v2".into(),
            world_schema_version: "world-schema-1".into(),
            content_revision: "rev-1".into(),
            production: false,
        },
        map_revision: format!(
            "sha256:{}",
            digest
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
        ),
        start: crate::map::overlay::TilePos {
            x: 32310,
            y: 32210,
            floor: -6,
        },
    };
    crate::map::boot::boot(
        &bytes,
        &pins,
        world,
        crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?,
        |_| None,
    )
    .map_err(debug)
}
