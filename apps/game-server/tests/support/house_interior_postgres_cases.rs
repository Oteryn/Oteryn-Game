#![allow(clippy::expect_used)]
//! Registered House shape/receipt/audit cases. Positive access and tile facts are
//! independent §10 test DATA; current session/lease/assignment remain native.

use crate::domain::ItemInstanceId;
use crate::durability::house_interior::{
    HOUSE_INTERIOR_ITEMS_MAX, HouseInteriorEffect, HouseInteriorError, HouseInteriorLocation,
    HouseItemRole, HousingReclaimProvenance,
};
use crate::durability::house_interior_audit::HouseInteriorAuditChange;
use crate::durability::house_scope_handoff::HouseId;
use crate::foundation::{CharacterId, WorldId};
use crate::house_test_fixture::{self, TestResult};
use sqlx::{Connection, PgConnection};

const HOUSE: &str = "oteryn:content.house.thais_1";
const WORLD: &str = "01010101-0101-7101-8101-010101010101";
const ACTOR: &str = "02020202-0202-7202-8202-020202020202";

fn uuid(seed: u8) -> [u8; 16] {
    let mut value = [seed; 16];
    value[6] = 0x70 | (seed & 0x0f);
    value[8] = 0x80 | (seed & 0x3f);
    value
}

fn actor(seed: u8) -> CharacterId {
    CharacterId::decode(&uuid(seed)).expect("valid immutable house fixture")
}

fn change() -> HouseInteriorAuditChange {
    let item = ItemInstanceId::from_bytes(uuid(3)).expect("valid immutable house fixture");
    HouseInteriorAuditChange {
        item_instance_id: item,
        actor: actor(2),
        effect: HouseInteriorEffect::Place {
            destination: HouseInteriorLocation {
                house: HouseId::new(
                    WorldId::decode(&uuid(1)).expect("valid immutable house fixture"),
                    HOUSE,
                )
                .expect("valid immutable house fixture"),
                spatial_position: vec![1, 2, 3],
                stack_ordinal: 1,
                scope_ownership_generation: 1,
                placement_transaction_id: uuid(4),
            },
            provenance: HousingReclaimProvenance {
                house: HouseId::new(
                    WorldId::decode(&uuid(1)).expect("valid immutable house fixture"),
                    HOUSE,
                )
                .expect("valid immutable house fixture"),
                item_instance_id: item,
                reclaim_subject: actor(2),
                placement_transaction_id: uuid(4),
                revision: 1,
            },
        },
    }
}

fn tile_move() -> HouseInteriorAuditChange {
    let mut value = change();
    let HouseInteriorEffect::Place {
        destination: source,
        provenance,
    } = value.effect
    else {
        unreachable!()
    };
    let mut destination = source.clone();
    destination.spatial_position = vec![4, 5, 6];
    destination.placement_transaction_id = uuid(5);
    let mut after = provenance.clone();
    after.placement_transaction_id = uuid(5);
    after.revision = 2;
    value.effect = HouseInteriorEffect::TileMove {
        source,
        destination,
        provenance_before: provenance,
        provenance_after: after,
    };
    value
}

#[test]
fn house_interior_accepted_effects_and_role_table() {
    let place = change();
    let moved = tile_move();
    let HouseInteriorEffect::Place {
        destination,
        provenance,
    } = place.effect.clone()
    else {
        unreachable!()
    };
    let mut take = place.clone();
    take.effect = HouseInteriorEffect::Take {
        source: destination,
        provenance,
    };
    for effect in [&place, &moved, &take] {
        assert_eq!(effect.validate(), Ok(()));
        for role in [
            HouseItemRole::Owner,
            HouseItemRole::Subowner,
            HouseItemRole::Guest,
        ] {
            assert_eq!(role.check_policy(effect.actor, &effect.effect), Ok(()));
        }
    }
    for role in [HouseItemRole::Owner, HouseItemRole::Subowner] {
        assert_eq!(role.check_policy(actor(8), &take.effect), Ok(()));
    }
    assert_eq!(
        HouseItemRole::Guest.check_policy(actor(8), &take.effect),
        Err(HouseInteriorError::NoAccess)
    );
    assert_eq!(
        HouseItemRole::Guest.check_policy(actor(8), &moved.effect),
        Ok(())
    );
    assert_eq!(HOUSE_INTERIOR_ITEMS_MAX, 2_000);
}

#[test]
fn house_interior_unknown_access_role_is_closed() {
    for role in ["", "owner", "ADMIN", "OWNER "] {
        assert_eq!(HouseItemRole::from_access_role(role), None);
    }
    assert_eq!(
        HouseItemRole::from_access_role("OWNER"),
        Some(HouseItemRole::Owner)
    );
    assert_eq!(
        HouseItemRole::from_access_role("SUBOWNER"),
        Some(HouseItemRole::Subowner)
    );
    assert_eq!(
        HouseItemRole::from_access_role("GUEST"),
        Some(HouseItemRole::Guest)
    );
}

#[test]
fn house_interior_location_bounds() {
    let HouseInteriorEffect::Place { destination, .. } = change().effect else {
        unreachable!()
    };
    for position in [vec![], vec![1; 129]] {
        let mut bad = destination.clone();
        bad.spatial_position = position;
        assert_eq!(bad.validate(), Err(HouseInteriorError::InvalidLocation));
    }
    let mut edge = destination.clone();
    edge.spatial_position = vec![1; 128];
    edge.stack_ordinal = u64::MAX;
    assert_eq!(edge.validate(), Ok(()));
    for field in 0..3 {
        let mut bad = destination.clone();
        match field {
            0 => bad.stack_ordinal = 0,
            1 => bad.scope_ownership_generation = 0,
            _ => bad.placement_transaction_id = [0; 16],
        }
        assert_eq!(bad.validate(), Err(HouseInteriorError::InvalidLocation));
    }
}

#[test]
fn house_interior_placement_binds_item_placer_transaction_and_revision() {
    for mutation in 0..5 {
        let mut bad = change();
        let HouseInteriorEffect::Place { provenance, .. } = &mut bad.effect else {
            unreachable!()
        };
        match mutation {
            0 => {
                provenance.item_instance_id =
                    ItemInstanceId::from_bytes(uuid(9)).expect("valid immutable house fixture")
            }
            1 => provenance.reclaim_subject = actor(9),
            2 => provenance.placement_transaction_id = uuid(9),
            3 => provenance.revision = 0,
            _ => {
                provenance.house = HouseId::new(
                    WorldId::decode(&uuid(1)).expect("valid immutable house fixture"),
                    "oteryn:content.house.other",
                )
                .expect("valid immutable house fixture")
            }
        }
        assert_eq!(bad.validate(), Err(HouseInteriorError::InvalidProvenance));
    }
}

#[test]
fn house_interior_tile_move_cannot_substitute_reclaim_or_generation() {
    for mutation in 0..5 {
        let mut bad = tile_move();
        let HouseInteriorEffect::TileMove {
            source,
            destination,
            provenance_after,
            ..
        } = &mut bad.effect
        else {
            unreachable!()
        };
        match mutation {
            0 => provenance_after.reclaim_subject = actor(9),
            1 => provenance_after.revision = 3,
            2 => destination.scope_ownership_generation = 2,
            3 => {
                destination.placement_transaction_id = source.placement_transaction_id;
                provenance_after.placement_transaction_id = source.placement_transaction_id;
            }
            _ => {
                provenance_after.item_instance_id =
                    ItemInstanceId::from_bytes(uuid(9)).expect("valid immutable house fixture")
            }
        }
        assert_eq!(bad.validate(), Err(HouseInteriorError::InvalidProvenance));
    }
}

#[test]
fn house_interior_tile_move_rejects_cross_house_and_cross_world() {
    for world in [1, 9] {
        let mut bad = tile_move();
        let HouseInteriorEffect::TileMove {
            destination,
            provenance_after,
            ..
        } = &mut bad.effect
        else {
            unreachable!()
        };
        let key = if world == 1 {
            "oteryn:content.house.other"
        } else {
            HOUSE
        };
        destination.house = HouseId::new(
            WorldId::decode(&uuid(world)).expect("valid immutable house fixture"),
            key,
        )
        .expect("valid immutable house fixture");
        provenance_after.house = destination.house.clone();
        assert_eq!(bad.validate(), Err(HouseInteriorError::CrossHouseMove));
    }
}

#[test]
fn house_interior_provenance_revision_cannot_wrap() {
    let mut bad = tile_move();
    let HouseInteriorEffect::TileMove {
        provenance_before,
        provenance_after,
        ..
    } = &mut bad.effect
    else {
        unreachable!()
    };
    provenance_before.revision = u64::MAX;
    provenance_after.revision = 1;
    assert_eq!(bad.validate(), Err(HouseInteriorError::InvalidProvenance));
}

async fn database_case() -> TestResult {
    let Ok(admin_url) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Err("House PostgreSQL cases require the canonical PG17.6 executor URL".into());
    };
    if !admin_url.starts_with("postgresql://oteryn_test_admin:")
        || !admin_url.ends_with("@127.0.0.1:5432/postgres")
    {
        return Err("unsafe PostgreSQL test admin URL".into());
    }
    let suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_nanos();
    let name = format!("house_interior_{suffix}");
    let mut admin = PgConnection::connect(&admin_url).await?;
    sqlx::query(sqlx::AssertSqlSafe(format!("CREATE DATABASE {name}")))
        .execute(&mut admin)
        .await?;
    let url = format!(
        "{}/{name}",
        admin_url.strip_suffix("/postgres").ok_or("admin URL")?
    );
    let outcome = check_database(&url).await;
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "DROP DATABASE {name} WITH (FORCE)"
    )))
    .execute(&mut admin)
    .await?;
    outcome
}

async fn check_database(url: &str) -> TestResult {
    let mut connection = PgConnection::connect(url).await?;
    let version: String = sqlx::query_scalar("SHOW server_version_num")
        .fetch_one(&mut connection)
        .await?;
    assert_eq!(version, "170006", "canonical target is PostgreSQL 17.6");
    sqlx::migrate!("./migrations").run(&mut connection).await?;
    let stub: i64 =
        sqlx::query_scalar("SELECT count(*) FROM game_house_access($1::uuid,$2,$3::uuid)")
            .bind(WORLD)
            .bind(HOUSE)
            .bind(ACTOR)
            .fetch_one(&mut connection)
            .await?;
    assert_eq!(stub, 0, "production stub admits nobody");
    for table in [
        "game_item_house_interior_locations",
        "game_item_house_reclaim_provenance",
        "game_item_house_placement_transactions",
    ] {
        for privilege in ["INSERT", "UPDATE", "DELETE"] {
            let granted: bool =
                sqlx::query_scalar("SELECT has_table_privilege('oteryn_game_runtime', $1, $2)")
                    .bind(table)
                    .bind(privilege)
                    .fetch_one(&mut connection)
                    .await?;
            assert!(!granted, "runtime unexpectedly has {privilege} on {table}");
        }
    }
    house_test_fixture::install(&mut connection).await?;
    sqlx::query("INSERT INTO test_house_properties VALUES ($1::uuid,$2,1,false)")
        .bind(WORLD)
        .bind(HOUSE)
        .execute(&mut connection)
        .await?;
    for role in ["OWNER", "SUBOWNER", "GUEST"] {
        sqlx::query("INSERT INTO test_house_grants VALUES ($1::uuid,$2,$3::uuid,$4,NULL) ON CONFLICT (world_id,house_key,character_id) DO UPDATE SET role=EXCLUDED.role")
            .bind(WORLD).bind(HOUSE).bind(ACTOR).bind(role).execute(&mut connection).await?;
        let actual: String =
            sqlx::query_scalar("SELECT role FROM game_house_access($1::uuid,$2,$3::uuid)")
                .bind(WORLD)
                .bind(HOUSE)
                .bind(ACTOR)
                .fetch_one(&mut connection)
                .await?;
        assert_eq!(actual, role);
    }
    let mut reader = connection.begin().await?;
    let _: String = sqlx::query_scalar("SELECT role FROM game_house_access($1::uuid,$2,$3::uuid)")
        .bind(WORLD)
        .bind(HOUSE)
        .bind(ACTOR)
        .fetch_one(&mut *reader)
        .await?;
    let mut revoker = PgConnection::connect(url).await?;
    let mut revocation = revoker.begin().await?;
    sqlx::query("SET LOCAL lock_timeout = '100ms'")
        .execute(&mut *revocation)
        .await?;
    let error = sqlx::query(
        "UPDATE test_house_properties SET acl_revision=2 WHERE world_id=$1::uuid AND house_key=$2",
    )
    .bind(WORLD)
    .bind(HOUSE)
    .execute(&mut *revocation)
    .await
    .expect_err("revocation must wait for the locked access row");
    assert_eq!(
        error.as_database_error().and_then(|e| e.code()).as_deref(),
        Some("55P03")
    );
    revocation.rollback().await?;
    reader.commit().await?;
    sqlx::query(
        "UPDATE test_house_properties SET acl_revision=2 WHERE world_id=$1::uuid AND house_key=$2",
    )
    .bind(WORLD)
    .bind(HOUSE)
    .execute(&mut revoker)
    .await?;
    Ok(())
}

#[test]
fn house_interior_postgres_stub_privileges_and_fixture_locking() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(database_case())
}

use crate::durability::house_interior::{
    HouseBackpackFacts, HouseCharacterSource, HouseItemAction, HouseItemOutcome, HouseItemRequest,
    HouseWriterError,
};
use crate::durability::house_interior_audit as house_audit;
use crate::durability::house_scope_handoff::{
    HouseCommitOutcome, HouseEntryCommit, HouseEntryRequest, HouseHandoffDirection,
    HousePrepareOutcome, HouseScopeAssignmentCommand, HouseScopeAssignmentOutcome,
    HouseScopeAssignmentRequest,
};
use crate::durability::item_mint_audit as native_audit;
use crate::durability::item_transfer::{
    CurrentCharacterItemFence, ItemDefinitionFacts, ItemStackClass, ItemTransferOutcome,
};
use crate::durability::runtime_scope_assignment::{
    BootstrapSecret, ControlActor, NodeIncarnationProof, OperationKey,
};
use crate::foundation::{CommandId, CommandRef, ConnectionGeneration, GameSessionId};
use crate::item_transfer_postgres_cases as native;
use prost::Message;
use sqlx::Row;

fn current_house() -> TestResult<HouseId> {
    Ok(HouseId::new(
        WorldId::decode(&native::id(native::WORLD)).map_err(native::debug)?,
        HOUSE,
    )
    .ok_or("house")?)
}
fn house_fence() -> TestResult<CurrentCharacterItemFence> {
    Ok(CurrentCharacterItemFence {
        game_session_id: GameSessionId::decode(&native::id(61)).map_err(native::debug)?,
        character_lease_generation: 2,
        runtime_scope: current_house()?.runtime_scope()?,
        ..native::fence()?
    })
}
fn item_request(
    item: [u8; 16],
    command: u64,
    action: HouseItemAction,
) -> TestResult<HouseItemRequest> {
    Ok(HouseItemRequest {
        command: CommandRef::new(
            house_fence()?.game_session_id,
            CommandId::new(command).map_err(native::debug)?,
        ),
        house: current_house()?,
        item_instance_id: item,
        action,
        expected_acl_revision: 1,
        expected_guild_revisions: None,
        backpack: None,
        content_revision: "content-1".into(),
        ruleset_revision: "ruleset-1".into(),
        sim_revision: "sim-1".into(),
    })
}
async fn seed_house(h: &native::Harness) -> TestResult {
    let mut c = PgConnection::connect(&h.database.url).await?;
    house_test_fixture::install(&mut c).await?;
    sqlx::query("INSERT INTO test_house_properties VALUES(encode($1,'hex')::uuid,$2,1,false)")
        .bind(native::id(native::WORLD).as_slice())
        .bind(HOUSE)
        .execute(&mut c)
        .await?;
    sqlx::query("INSERT INTO test_house_grants VALUES(encode($1,'hex')::uuid,$2,encode($3,'hex')::uuid,'OWNER',NULL)").bind(native::id(native::WORLD).as_slice()).bind(HOUSE).bind(native::id(native::CHARACTER).as_slice()).execute(&mut c).await?;
    for pos in [vec![1u8, 2, 3], vec![4u8, 5, 6]] {
        sqlx::query("INSERT INTO test_house_tiles VALUES(encode($1,'hex')::uuid,$2,$3)")
            .bind(native::id(native::WORLD).as_slice())
            .bind(HOUSE)
            .bind(pos)
            .execute(&mut c)
            .await?;
    }
    sqlx::query(
        "INSERT INTO game_durability_session_use_ledgers VALUES(encode($1,'hex')::uuid,1,true,1,1)",
    )
    .bind(native::id(native::CHARACTER).as_slice())
    .execute(&mut c)
    .await?;
    sqlx::query("INSERT INTO game_durability_session_use_memberships VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL)").bind(native::id(native::SESSION).as_slice()).bind(native::id(native::CHARACTER).as_slice()).execute(&mut c).await?;
    sqlx::query("INSERT INTO game_control_house_scope_grants SELECT session_user,encode($1,'hex')::uuid,$2,op FROM unnest(ARRAY[1,2,3]::SMALLINT[]) op").bind(native::id(native::WORLD).as_slice()).bind(HOUSE).execute(&mut c).await?;
    let outcome = h
        .root
        .assign_house_scope(HouseScopeAssignmentRequest {
            operation_key: OperationKey::from_bytes([8; 32]),
            actor: ControlActor::new("oteryn_test_admin").map_err(native::debug)?,
            actor_role: "oteryn_test_admin".into(),
            command: HouseScopeAssignmentCommand::Assign {
                house: current_house()?,
                target: h.node.fact(),
            },
        })
        .await
        .map_err(native::debug)?;
    assert!(matches!(outcome, HouseScopeAssignmentOutcome::Committed(_)));
    let seal = h.recovery.seal_current().map_err(native::debug)?;
    let a = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(native::debug)?;
    let f = native::fence()?;
    let prepared = h
        .root
        .prepare_house_entry(
            &a,
            &h.node,
            HouseEntryRequest {
                direction: HouseHandoffDirection::Entry,
                handoff_id: native::id(60),
                character_id: CharacterId::decode(f.character_id.as_bytes())
                    .map_err(native::debug)?,
                source_game_session_id: f.game_session_id,
                source_connection_generation: 1,
                source_character_lease_generation: 1,
                source_scope: f.runtime_scope,
                source_scope_ownership_generation: 1,
                house: current_house()?,
                house_scope_ownership_generation: 1,
                acl_revision: 1,
                guild_revisions: None,
                reserved_tile: vec![1, 2, 3],
                prepared_at: 100,
            },
        )
        .await
        .map_err(native::debug)?;
    assert!(matches!(prepared, HousePrepareOutcome::Prepared(_)));
    let committed = h
        .root
        .commit_house_entry(
            &a,
            &h.node,
            HouseEntryCommit {
                handoff_id: native::id(60),
                destination_game_session_id: house_fence()?.game_session_id,
                committed_at: 200,
            },
        )
        .await
        .map_err(native::debug)?;
    assert!(matches!(committed, HouseCommitOutcome::Committed(_)));
    Ok(())
}
async fn funded(h: &native::Harness) -> TestResult<([u8; 16], [u8; 16])> {
    let seal = h.recovery.seal_current().map_err(native::debug)?;
    let a = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(native::debug)?;
    let backpack = h.mint(&a, native::BACKPACK, 1).await?;
    assert!(matches!(
        h.transfer(
            &a,
            native::fence()?,
            native::to_slot(native::command(1)?, backpack, native::backpack_facts())
        )
        .await
        .map_err(native::debug)?,
        ItemTransferOutcome::Committed(_)
    ));
    let item = h.mint(&a, "fixture:house.item", 1).await?;
    let facts = ItemDefinitionFacts {
        definition: crate::durability::item_mint::TypedDefinitionRef {
            family: "Item".into(),
            production_key: "fixture:house.item".into(),
            revision_ref: "definition-r1".into(),
        },
        stack: ItemStackClass::NonStackable,
        container_capacity: None,
        container_slot_equip_pattern: false,
    };
    assert!(matches!(
        h.transfer(
            &a,
            native::fence()?,
            native::to_backpack(native::command(2)?, item, facts)
        )
        .await
        .map_err(native::debug)?,
        ItemTransferOutcome::Committed(_)
    ));
    Ok((backpack, item))
}
fn run_native(
    tag: &str,
    case: impl AsyncFnOnce(&native::Harness, [u8; 16], [u8; 16]) -> TestResult,
) -> TestResult {
    let Some(admin) = native::configured_admin() else {
        return Err("House PostgreSQL cases require the canonical PG17.6 executor URL".into());
    };
    native::runtime()?.block_on(async {
        let h = native::Harness::create(admin, tag).await?;
        let result = async {
            let (b, i) = funded(&h).await?;
            seed_house(&h).await?;
            case(&h, b, i).await
        }
        .await;
        let cleanup = h.cleanup().await;
        result?;
        cleanup
    })
}
fn committed(
    outcome: HouseItemOutcome,
) -> TestResult<crate::durability::house_interior::HouseItemReceipt> {
    match outcome {
        HouseItemOutcome::Committed(r) => Ok(r),
        other => Err(format!("expected committed: {other:?}").into()),
    }
}

#[test]
fn house_shapes_commit_replay_provenance_and_native_restart() -> TestResult {
    run_native("house_interior_shapes", async |h, backpack, item| {
        let seal = h.recovery.seal_current().map_err(native::debug)?;
        let a = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(native::debug)?;
        let place = item_request(
            item,
            100,
            HouseItemAction::Place {
                source: HouseCharacterSource::Inventory {
                    parent: backpack,
                    ordinal: 1,
                },
                position: vec![1, 2, 3],
            },
        )?;
        let frozen = h
            .root
            .freeze_house_item(&a, &h.node, house_fence()?, place.clone())
            .await
            .map_err(native::debug)?;
        let peer = crate::durability::DurabilityRoot::connect_test_runtime(&h.database.url)?;
        assert!(peer.maintain_ready_once().await?);
        let peer_authority = peer
            .open_character_authority(&seal)
            .await
            .map_err(native::debug)?;
        let (first, second) = native::join_two(
            h.root
                .commit_house_item(&a, &h.node, house_fence()?, &frozen),
            peer.commit_house_item(&peer_authority, &h.node, house_fence()?, &frozen),
        )
        .await;
        let receipt = match (
            first.map_err(native::debug)?,
            second.map_err(native::debug)?,
        ) {
            (HouseItemOutcome::Committed(r), HouseItemOutcome::AlreadyCommitted(other))
            | (HouseItemOutcome::AlreadyCommitted(other), HouseItemOutcome::Committed(r)) => {
                assert_eq!(r, other);
                r
            }
            outcomes => {
                return Err(format!("one commit and one replay required: {outcomes:?}").into());
            }
        };
        let (_, event) = house_audit::decode_house_transfer_envelope(
            receipt.envelope.as_deref().ok_or("live audit expected")?,
        )
        .map_err(native::debug)?;
        assert_eq!(event.before, event.after);
        assert_eq!(h.count("game_item_house_interior_locations").await?, 1);
        for (placement, revision) in [(uuid(9), 1), (receipt.transaction_id, 2)] {
            let request = item_request(
                item,
                110 + revision,
                HouseItemAction::TileMove {
                    placement,
                    provenance_revision: revision,
                    position: vec![4, 5, 6],
                },
            )?;
            assert!(matches!(
                h.root
                    .freeze_house_item(&a, &h.node, house_fence()?, request)
                    .await,
                Err(HouseWriterError::SourceChanged)
            ));
        }
        sqlx::query("DELETE FROM test_house_tiles WHERE position=$1")
            .bind([1u8, 2, 3].as_slice())
            .execute(&h.pool)
            .await?;
        let request = item_request(
            item,
            113,
            HouseItemAction::TileMove {
                placement: receipt.transaction_id,
                provenance_revision: 1,
                position: vec![4, 5, 6],
            },
        )?;
        assert!(matches!(
            h.root
                .freeze_house_item(&a, &h.node, house_fence()?, request)
                .await,
            Err(HouseWriterError::NotHouseTile)
        ));
        sqlx::query(
            "INSERT INTO test_house_tiles VALUES(encode($1,'hex')::uuid,$2,'\\x010203'::bytea)",
        )
        .bind(native::id(native::WORLD).as_slice())
        .bind(HOUSE)
        .execute(&h.pool)
        .await?;
        let before = h.count("game_item_audit_outbox").await?;
        let mut stale = house_fence()?;
        stale.connection_generation = ConnectionGeneration::new(99).map_err(native::debug)?;
        assert_eq!(
            h.root
                .commit_house_item(&a, &h.node, stale, &frozen)
                .await
                .map_err(native::debug)?,
            HouseItemOutcome::AlreadyCommitted(receipt.clone())
        );
        let mut changed = place;
        changed.expected_acl_revision = 2;
        assert!(matches!(
            h.root
                .freeze_house_item(&a, &h.node, house_fence()?, changed)
                .await,
            Err(HouseWriterError::ConflictingOccurrence)
        ));
        let moved = h
            .root
            .freeze_house_item(
                &a,
                &h.node,
                house_fence()?,
                item_request(
                    item,
                    101,
                    HouseItemAction::TileMove {
                        placement: receipt.transaction_id,
                        provenance_revision: 1,
                        position: vec![4, 5, 6],
                    },
                )?,
            )
            .await
            .map_err(native::debug)?;
        let moved = committed(
            h.root
                .commit_house_item(&a, &h.node, house_fence()?, &moved)
                .await
                .map_err(native::debug)?,
        )?;
        let mut take = item_request(
            item,
            102,
            HouseItemAction::Take {
                placement: moved.transaction_id,
                provenance_revision: 2,
            },
        )?;
        take.backpack = Some(HouseBackpackFacts {
            definition: native::backpack_facts().definition,
            capacity: 20,
        });
        let frozen_take = h
            .root
            .freeze_house_item(&a, &h.node, house_fence()?, take)
            .await
            .map_err(native::debug)?;
        let taken = committed(
            h.root
                .commit_house_item(&a, &h.node, house_fence()?, &frozen_take)
                .await
                .map_err(native::debug)?,
        )?;
        assert_eq!(h.count("game_item_house_interior_locations").await?, 0);
        assert_eq!(h.count("game_item_house_reclaim_provenance").await?, 0);
        assert_eq!(h.count("game_item_audit_outbox").await?, before + 2);
        assert_eq!(h.item_state(item).await?, (1, 1));
        let restarted = crate::durability::DurabilityRoot::connect_test_runtime(&h.database.url)?;
        assert!(restarted.maintain_ready_once().await?);
        let a2 = restarted
            .open_character_authority(&seal)
            .await
            .map_err(native::debug)?;
        assert_eq!(
            restarted
                .reconcile_house_item(&a2, &frozen_take)
                .await
                .map_err(native::debug)?,
            Some(taken.clone())
        );
        let mut admin = PgConnection::connect(&h.database.url).await?;
        let original = taken.envelope.as_deref().ok_or("retained live audit")?;
        house_test_fixture::retained_audit_transaction_data_probe(
            &mut admin,
            &taken.event_id,
            &uuid(99),
        )
        .await?;
        assert!(matches!(
            restarted.reconcile_house_item(&a2, &frozen_take).await,
            Err(HouseWriterError::Durability(
                crate::durability::DurabilityError::InvalidStoredState
            ))
        ));
        house_test_fixture::retained_audit_transaction_data_probe(
            &mut admin,
            &taken.event_id,
            &taken.transaction_id,
        )
        .await?;
        let mut corrupt = original.to_vec();
        corrupt[0] ^= 1;
        house_test_fixture::retained_audit_data_probe(
            &mut admin,
            &taken.transaction_id,
            &taken.event_id,
            Some(&corrupt),
        )
        .await?;
        assert!(matches!(
            restarted.reconcile_house_item(&a2, &frozen_take).await,
            Err(HouseWriterError::Durability(
                crate::durability::DurabilityError::InvalidStoredState
            ))
        ));
        house_test_fixture::retained_audit_data_probe(
            &mut admin,
            &taken.transaction_id,
            &taken.event_id,
            Some(original),
        )
        .await?;
        assert_eq!(
            restarted
                .reconcile_house_item(&a2, &frozen_take)
                .await
                .map_err(native::debug)?,
            Some(taken.clone())
        );
        house_test_fixture::retained_audit_data_probe(
            &mut admin,
            &taken.transaction_id,
            &taken.event_id,
            None,
        )
        .await?;
        sqlx::raw_sql("CREATE OR REPLACE FUNCTION game_house_access(p_world_id UUID,p_house_key TEXT,p_character_id UUID) RETURNS TABLE(role TEXT,content_fenced BOOLEAN,acl_revision NUMERIC(20,0),guild_revisions JSONB) LANGUAGE sql AS $$ SELECT NULL::text,NULL::boolean,NULL::numeric(20,0),NULL::jsonb WHERE false $$").execute(&h.pool).await?;
        let mut absent = taken.clone();
        absent.envelope = None;
        assert_eq!(
            restarted
                .reconcile_house_item(&a2, &frozen_take)
                .await
                .map_err(native::debug)?,
            Some(absent.clone())
        );
        let unpossessed =
            NodeIncarnationProof::new(h.node.fact(), BootstrapSecret::from_bytes([98; 32]));
        assert_eq!(
            restarted
                .commit_house_item(&a2, &unpossessed, stale, &frozen_take)
                .await
                .map_err(native::debug)?,
            HouseItemOutcome::AlreadyCommitted(absent)
        );
        let work: i16 = sqlx::query_scalar(
            "SELECT work_units_used FROM game_item_transfer_reservations WHERE command_id=102",
        )
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(work, 1, "historical replay did not retry a value operation");
        let revision:String=sqlx::query_scalar("SELECT character_revision::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid").bind(native::id(native::CHARACTER).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(revision, "1");
        Ok(())
    })
}

#[test]
fn house_shapes_fail_closed_current_fences_access_and_tile() -> TestResult {
    run_native("house_interior_fences", async |h, backpack, item| {
        let seal = h.recovery.seal_current().map_err(native::debug)?;
        let a = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(native::debug)?;
        let request = |command| {
            item_request(
                item,
                command,
                HouseItemAction::Place {
                    source: HouseCharacterSource::Inventory {
                        parent: backpack,
                        ordinal: 1,
                    },
                    position: vec![1, 2, 3],
                },
            )
        };
        let before = h.count("game_item_audit_outbox").await?;
        for mutation in 0..4 {
            let mut f = house_fence()?;
            match mutation {
                0 => {
                    f.connection_generation = ConnectionGeneration::new(2).map_err(native::debug)?
                }
                1 => f.character_lease_generation = 3,
                2 => {
                    f.scope_ownership_generation =
                        crate::foundation::ScopeOwnershipGeneration::new(2)
                            .map_err(native::debug)?
                }
                _ => {
                    f.game_session_id =
                        GameSessionId::decode(&native::id(62)).map_err(native::debug)?
                }
            };
            assert!(matches!(
                h.root
                    .freeze_house_item(&a, &h.node, f, request(200 + mutation)?)
                    .await,
                Err(HouseWriterError::NotCurrent)
            ));
        }
        let wrong = NodeIncarnationProof::new(h.node.fact(), BootstrapSecret::from_bytes([99; 32]));
        assert!(matches!(
            h.root
                .freeze_house_item(&a, &wrong, house_fence()?, request(210)?)
                .await,
            Err(HouseWriterError::NotCurrent)
        ));
        let candidate = h
            .root
            .freeze_house_item(&a, &h.node, house_fence()?, request(211)?)
            .await
            .map_err(native::debug)?;
        sqlx::query("UPDATE test_house_properties SET acl_revision=2")
            .execute(&h.pool)
            .await?;
        assert!(matches!(
            h.root
                .commit_house_item(&a, &h.node, house_fence()?, &candidate)
                .await,
            Err(HouseWriterError::AccessChanged)
        ));
        sqlx::query("UPDATE test_house_properties SET acl_revision=1,content_fenced=true")
            .execute(&h.pool)
            .await?;
        assert!(matches!(
            h.root
                .commit_house_item(&a, &h.node, house_fence()?, &candidate)
                .await,
            Err(HouseWriterError::HouseClosed)
        ));
        sqlx::query("UPDATE test_house_properties SET content_fenced=false")
            .execute(&h.pool)
            .await?;
        sqlx::query("DELETE FROM test_house_tiles")
            .execute(&h.pool)
            .await?;
        assert!(matches!(
            h.root
                .commit_house_item(&a, &h.node, house_fence()?, &candidate)
                .await,
            Err(HouseWriterError::NotHouseTile)
        ));
        let restarted = crate::durability::DurabilityRoot::connect_test_runtime(&h.database.url)?;
        assert!(restarted.maintain_ready_once().await?);
        let after_restart = restarted
            .open_character_authority(&seal)
            .await
            .map_err(native::debug)?;
        assert!(matches!(
            restarted
                .commit_house_item(&after_restart, &h.node, house_fence()?, &candidate)
                .await,
            Err(HouseWriterError::RetryBudgetExhausted)
        ));
        assert!(matches!(
            restarted
                .reconcile_house_item(&after_restart, &candidate)
                .await,
            Err(HouseWriterError::RetryBudgetExhausted)
        ));
        let work: i16 = sqlx::query_scalar(
            "SELECT work_units_used FROM game_item_transfer_reservations WHERE command_id=211",
        )
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(
            work, 3,
            "durable retry/reconciliation cap survived a native Root restart"
        );
        assert_eq!(h.count("game_item_audit_outbox").await?, before);
        assert_eq!(h.count("game_item_house_interior_locations").await?, 0);
        assert_eq!(h.count("game_item_container_entries").await?, 1);
        Ok(())
    })
}

// Compare actual rows, including provenance and item revisions, rather than only
// cardinalities. Reservations are checked separately because a refused apply
// intentionally consumes one durable retry unit.
async fn house_value_snapshot(h: &native::Harness) -> TestResult<Vec<serde_json::Value>> {
    let mut rows = Vec::new();
    for table in [
        "game_item_instances",
        "game_item_container_entries",
        "game_item_container_slots",
        "game_item_house_interior_locations",
        "game_item_house_reclaim_provenance",
        "game_item_house_placement_transactions",
        "game_item_house_shape_proofs",
        "game_item_transfer_receipts",
        "game_item_audit_outbox",
    ] {
        rows.push(sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
            "SELECT coalesce(jsonb_agg(to_jsonb(t) ORDER BY to_jsonb(t)::text),'[]'::jsonb) FROM {table} t"
        ))).fetch_one(&h.pool).await?);
    }
    Ok(rows)
}

async fn house_request_for_shape(
    h: &native::Harness,
    backpack: [u8; 16],
    item: [u8; 16],
    shape: u8,
) -> TestResult<HouseItemRequest> {
    let place = item_request(
        item,
        900,
        HouseItemAction::Place {
            source: HouseCharacterSource::Inventory {
                parent: backpack,
                ordinal: 1,
            },
            position: vec![1, 2, 3],
        },
    )?;
    if shape == 5 {
        return Ok(place);
    }
    let seal = h.recovery.seal_current().map_err(native::debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(native::debug)?;
    let candidate = h
        .root
        .freeze_house_item(&authority, &h.node, house_fence()?, place)
        .await
        .map_err(native::debug)?;
    let placed = committed(
        h.root
            .commit_house_item(&authority, &h.node, house_fence()?, &candidate)
            .await
            .map_err(native::debug)?,
    )?;
    let action = match shape {
        6 => HouseItemAction::Take {
            placement: placed.transaction_id,
            provenance_revision: 1,
        },
        7 => HouseItemAction::TileMove {
            placement: placed.transaction_id,
            provenance_revision: 1,
            position: vec![4, 5, 6],
        },
        _ => return Err("unknown test shape".into()),
    };
    let mut request = item_request(item, 901, action)?;
    if shape == 6 {
        request.backpack = Some(HouseBackpackFacts {
            definition: native::backpack_facts().definition,
            capacity: 20,
        });
    }
    Ok(request)
}

#[test]
fn house_uncommitted_apply_rechecks_independent_current_authority() -> TestResult {
    // One database per operator: never restore a terminal authority or let an
    // earlier refusal exhaust the retry budget and mask a later predicate.
    for shape in [5, 6, 7] {
        for operator in 0..9 {
            run_native(
                &format!("house_interior_a{shape}_{operator}"),
                async |h, backpack, item| {
                    let request = house_request_for_shape(h, backpack, item, shape).await?;
                    let command = request.command.command_id().get().to_string();
                    let seal = h.recovery.seal_current().map_err(native::debug)?;
                    let authority = h
                        .root
                        .open_character_authority(&seal)
                        .await
                        .map_err(native::debug)?;
                    let candidate = h
                        .root
                        .freeze_house_item(&authority, &h.node, house_fence()?, request)
                        .await
                        .map_err(native::debug)?;
                    let before = house_value_snapshot(h).await?;
                    let reservation_before: serde_json::Value = sqlx::query_scalar(
                    "SELECT to_jsonb(r)-'work_units_used' FROM game_item_transfer_reservations r WHERE command_id=$1::text::numeric"
                ).bind(&command).fetch_one(&h.pool).await?;
                    let expected = match operator {
                        0..=2 => {
                            let query = match operator {
                                0 => {
                                    "UPDATE game_durability_reconnect_sessions SET current_generation=current_generation+1 WHERE game_session_id=encode($1,'hex')::uuid"
                                }
                                1 => {
                                    "UPDATE game_durability_reconnect_sessions SET character_lease_generation=character_lease_generation+1 WHERE game_session_id=encode($1,'hex')::uuid"
                                }
                                _ => {
                                    "UPDATE game_durability_reconnect_sessions SET session_state=3 WHERE game_session_id=encode($1,'hex')::uuid"
                                }
                            };
                            assert_eq!(
                                sqlx::query(query)
                                    .bind(house_fence()?.game_session_id.as_bytes().as_slice())
                                    .execute(&h.pool)
                                    .await?
                                    .rows_affected(),
                                1
                            );
                            HouseWriterError::NotCurrent
                        }
                        3 | 4 => {
                            let row = sqlx::query("SELECT ownership_generation::text,source_revision::text FROM game_runtime_scope_assignments WHERE scope_key=game_house_scope_key(encode($1,'hex')::uuid,$2)")
                            .bind(native::id(native::WORLD).as_slice()).bind(HOUSE).fetch_one(&h.pool).await?;
                            let predecessor =
                                crate::durability::house_scope_handoff::HouseScopePredecessor {
                                    ownership_generation: row
                                        .try_get::<String, _>("ownership_generation")?
                                        .parse()?,
                                    source_revision: row
                                        .try_get::<String, _>("source_revision")?
                                        .parse()?,
                                };
                            let command = if operator == 3 {
                                HouseScopeAssignmentCommand::Replace {
                                    house: current_house()?,
                                    predecessor,
                                    target: h.node.fact(),
                                }
                            } else {
                                HouseScopeAssignmentCommand::Revoke {
                                    house: current_house()?,
                                    predecessor,
                                }
                            };
                            assert!(matches!(
                                h.root
                                    .assign_house_scope(HouseScopeAssignmentRequest {
                                        operation_key: OperationKey::from_bytes(
                                            [90 + operator; 32]
                                        ),
                                        actor: ControlActor::new("oteryn_test_admin")
                                            .map_err(native::debug)?,
                                        actor_role: "oteryn_test_admin".into(),
                                        command,
                                    })
                                    .await
                                    .map_err(native::debug)?,
                                HouseScopeAssignmentOutcome::Committed(_)
                            ));
                            HouseWriterError::NotCurrent
                        }
                        5 => {
                            h.root
                                .revoke_node_registration(h.node.fact())
                                .await
                                .map_err(native::debug)?;
                            HouseWriterError::NotCurrent
                        }
                        6 => {
                            sqlx::query("UPDATE test_house_grants SET guild_revisions=jsonb_build_object('fixture:guild',2)")
                            .execute(&h.pool).await?;
                            HouseWriterError::AccessChanged
                        }
                        7 => {
                            sqlx::query("UPDATE test_house_properties SET acl_revision=2")
                                .execute(&h.pool)
                                .await?;
                            HouseWriterError::AccessChanged
                        }
                        _ => {
                            sqlx::query("DELETE FROM test_house_grants")
                                .execute(&h.pool)
                                .await?;
                            HouseWriterError::NoAccess
                        }
                    };
                    // Expectations remain the original independently prepared facts;
                    // never manufacture a matching fence from the mutated DB row.
                    let result = h
                        .root
                        .commit_house_item(&authority, &h.node, house_fence()?, &candidate)
                        .await;
                    assert!(
                        matches!(
                            (&result, &expected),
                            (
                                Err(HouseWriterError::NotCurrent),
                                HouseWriterError::NotCurrent
                            ) | (
                                Err(HouseWriterError::AccessChanged),
                                HouseWriterError::AccessChanged
                            ) | (Err(HouseWriterError::NoAccess), HouseWriterError::NoAccess)
                        ),
                        "{result:?}"
                    );
                    let row = sqlx::query("SELECT transaction_id::text,house_intent,work_units_used,to_jsonb(r)-'work_units_used' AS immutable FROM game_item_transfer_reservations r WHERE command_id=$1::text::numeric")
                    .bind(&command).fetch_one(&h.pool).await?;
                    assert_eq!(row.try_get::<i16, _>("work_units_used")?, 1);
                    assert_eq!(
                        row.try_get::<serde_json::Value, _>("immutable")?,
                        reservation_before
                    );
                    let mut p: serde_json::Value = row.try_get("house_intent")?;
                    p["transaction"] = row.try_get::<String, _>("transaction_id")?.into();
                    p["scope_generation"] = "1".into();
                    p["connection_generation"] = "1".into();
                    p["lease_generation"] = "2".into();
                    p["phase"] = "apply".into();
                    let query = match shape {
                        5 => {
                            "SELECT game_house_item_place($1,encode($2,'hex')::uuid,$3::text::numeric,$4)"
                        }
                        6 => {
                            "SELECT game_house_item_take($1,encode($2,'hex')::uuid,$3::text::numeric,$4)"
                        }
                        _ => {
                            "SELECT game_house_item_move($1,encode($2,'hex')::uuid,$3::text::numeric,$4)"
                        }
                    };
                    let mut tx = h.pool.begin().await?;
                    sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
                        .execute(&mut *tx)
                        .await?;
                    let error = sqlx::query(query)
                        .bind(p)
                        .bind(h.node.fact().node_id().as_bytes().as_slice())
                        .bind(h.node.fact().registration_revision().to_string())
                        .bind([1u8; 32].as_slice())
                        .execute(&mut *tx)
                        .await
                        .expect_err(
                            "direct apply must independently reject the same stale authority",
                        );
                    let message = match expected {
                        HouseWriterError::NotCurrent => "NOT_CURRENT",
                        HouseWriterError::AccessChanged => "ACCESS_CHANGED",
                        _ => "NO_ACCESS",
                    };
                    assert_eq!(
                        error.as_database_error().and_then(|e| e.code()).as_deref(),
                        Some("P0001")
                    );
                    assert_eq!(
                        error.as_database_error().map(|e| e.message()),
                        Some(message)
                    );
                    tx.rollback().await?;
                    assert_eq!(house_value_snapshot(h).await?, before);
                    Ok(())
                },
            )?;
        }
    }
    Ok(())
}

#[test]
fn house_distinct_commands_compete_for_one_real_source() -> TestResult {
    for shape in [5, 6, 7] {
        run_native(
            &format!("house_interior_race{shape}"),
            async |h, backpack, item| {
                let first_request = house_request_for_shape(h, backpack, item, shape).await?;
                let mut second_request = first_request.clone();
                second_request.command = CommandRef::new(
                    house_fence()?.game_session_id,
                    CommandId::new(902).map_err(native::debug)?,
                );
                let seal = h.recovery.seal_current().map_err(native::debug)?;
                let authority = h
                    .root
                    .open_character_authority(&seal)
                    .await
                    .map_err(native::debug)?;
                let peer =
                    crate::durability::DurabilityRoot::connect_test_runtime(&h.database.url)?;
                assert!(peer.maintain_ready_once().await?);
                let peer_authority = peer
                    .open_character_authority(&seal)
                    .await
                    .map_err(native::debug)?;
                let first = h
                    .root
                    .freeze_house_item(&authority, &h.node, house_fence()?, first_request)
                    .await
                    .map_err(native::debug)?;
                let second = peer
                    .freeze_house_item(&peer_authority, &h.node, house_fence()?, second_request)
                    .await
                    .map_err(native::debug)?;
                let events = h.count("game_item_audit_outbox").await?;
                let receipts = h.count("game_item_transfer_receipts").await?;
                let proofs = h.count("game_item_house_shape_proofs").await?;
                let before: (String, i64, i16) = sqlx::query_as("SELECT state_revision::text,quantity,lifecycle FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid")
                .bind(item.as_slice()).fetch_one(&h.pool).await?;
                let (one, two) = native::join_two(
                    h.root
                        .commit_house_item(&authority, &h.node, house_fence()?, &first),
                    peer.commit_house_item(&peer_authority, &h.node, house_fence()?, &second),
                )
                .await;
                let (winner, loser) = match (one, two) {
                    (Ok(HouseItemOutcome::Committed(r)), Err(HouseWriterError::SourceChanged)) => {
                        (r, &second)
                    }
                    (Err(HouseWriterError::SourceChanged), Ok(HouseItemOutcome::Committed(r))) => {
                        (r, &first)
                    }
                    outcomes => return Err(format!(
                        "distinct causes require one winner and a source-change loser: {outcomes:?}"
                    )
                    .into()),
                };
                assert_eq!(h.count("game_item_audit_outbox").await?, events + 1);
                assert_eq!(h.count("game_item_transfer_receipts").await?, receipts + 1);
                assert_eq!(h.count("game_item_house_shape_proofs").await?, proofs + 1);
                let after: (String, i64, i16, String) = sqlx::query_as("SELECT state_revision::text,quantity,lifecycle,last_transaction_id::text FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid")
                .bind(item.as_slice()).fetch_one(&h.pool).await?;
                assert_eq!(after.0.parse::<u64>()?, before.0.parse::<u64>()? + 1);
                assert_eq!((after.1, after.2), (before.1, before.2));
                assert_eq!(
                    after.3.replace('-', ""),
                    winner
                        .transaction_id
                        .iter()
                        .map(|b| format!("{b:02x}"))
                        .collect::<String>()
                );
                assert_eq!(
                    h.count("game_item_house_interior_locations").await?,
                    i64::from(shape != 6)
                );
                assert_eq!(
                    h.count("game_item_house_reclaim_provenance").await?,
                    i64::from(shape != 6)
                );
                let first_command: u64 = if shape == 5 { 900 } else { 901 };
                let work: Vec<i16> = sqlx::query_scalar("SELECT work_units_used FROM game_item_transfer_reservations WHERE game_session_id=encode($1,'hex')::uuid AND command_id IN($2::text::numeric,902) ORDER BY command_id")
                    .bind(house_fence()?.game_session_id.as_bytes().as_slice())
                    .bind(first_command.to_string()).fetch_all(&h.pool).await?;
                assert_eq!(work, vec![1, 1]);
                let retained: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_transfer_receipts r JOIN game_item_house_shape_proofs p USING(transaction_id,event_id) WHERE r.transaction_id=encode($1,'hex')::uuid AND r.event_id=encode($2,'hex')::uuid AND r.source_item_instance_id=encode($3,'hex')::uuid AND r.shape=$4 AND p.shape=r.shape AND p.item_instance_id=r.source_item_instance_id")
                    .bind(winner.transaction_id.as_slice()).bind(winner.event_id.as_slice())
                    .bind(item.as_slice()).bind(i16::from(shape)).fetch_one(&h.pool).await?;
                assert_eq!(retained, 1);
                let (envelope, event) = house_audit::decode_house_transfer_envelope(
                    winner.envelope.as_deref().ok_or("retained winning event")?,
                )
                .map_err(native::debug)?;
                assert_eq!(envelope.event_id, winner.event_id);
                assert_eq!(
                    envelope
                        .transaction_event
                        .as_ref()
                        .ok_or("transaction event")?
                        .transaction_id,
                    winner.transaction_id
                );
                assert_eq!(event.before, event.after);
                assert_eq!(
                    event.before.as_ref().ok_or("item state")?.item_instance_id,
                    item
                );
                if shape == 6 {
                    let destination: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_container_entries WHERE item_instance_id=encode($1,'hex')::uuid AND character_id=encode($2,'hex')::uuid AND parent_item_instance_id=encode($3,'hex')::uuid AND placement_ordinal=1 AND placed_transaction_id=encode($4,'hex')::uuid")
                        .bind(item.as_slice()).bind(native::id(native::CHARACTER).as_slice())
                        .bind(backpack.as_slice()).bind(winner.transaction_id.as_slice())
                        .fetch_one(&h.pool).await?;
                    assert_eq!(destination, 1);
                } else {
                    assert_eq!(h.count("game_item_container_entries").await?, 0);
                    let position = if shape == 5 {
                        vec![1u8, 2, 3]
                    } else {
                        vec![4u8, 5, 6]
                    };
                    let destination: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_house_interior_locations l JOIN game_item_house_reclaim_provenance p USING(item_instance_id,world_id,house_key) WHERE l.item_instance_id=encode($1,'hex')::uuid AND l.world_id=encode($2,'hex')::uuid AND l.house_key=$3 AND l.spatial_position=$4 AND l.stack_ordinal=1 AND l.placed_transaction_id=encode($5,'hex')::uuid AND p.placement_transaction_id=l.placed_transaction_id AND p.reclaim_subject_character_id=encode($6,'hex')::uuid AND p.provenance_revision=$7::text::numeric")
                        .bind(item.as_slice()).bind(native::id(native::WORLD).as_slice()).bind(HOUSE)
                        .bind(position).bind(winner.transaction_id.as_slice())
                        .bind(native::id(native::CHARACTER).as_slice())
                        .bind(if shape == 5 { "1" } else { "2" }).fetch_one(&h.pool).await?;
                    assert_eq!(destination, 1);
                }
                let values = house_value_snapshot(h).await?;
                assert!(
                    h.root
                        .reconcile_house_item(&authority, loser)
                        .await
                        .map_err(native::debug)?
                        .is_none()
                );
                assert_eq!(house_value_snapshot(h).await?, values);
                Ok(())
            },
        )?;
    }
    Ok(())
}

async fn wait_house_blocker(
    h: &native::Harness,
    blocker: i32,
    waiter: Option<i32>,
) -> TestResult<i32> {
    let observation = tokio::time::timeout(std::time::Duration::from_secs(2), async {
        loop {
            let pid: Option<i32> = sqlx::query_scalar("SELECT a.pid FROM pg_stat_activity a WHERE a.datname=current_database() AND a.pid<>pg_backend_pid() AND $1=ANY(pg_blocking_pids(a.pid)) AND ($2::integer IS NULL OR a.pid=$2) LIMIT 1")
                .bind(blocker).bind(waiter).fetch_optional(&h.pool).await?;
            if let Some(pid) = pid {
                return Ok::<i32, sqlx::Error>(pid);
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    }).await;
    match observation {
        Ok(result) => Ok(result?),
        Err(_) => Err("actual House writer/revoker lock interleaving was not observed".into()),
    }
}

#[test]
fn house_actual_apply_serializes_both_orders_of_acl_revocation() -> TestResult {
    for writer_first in [false, true] {
        let tag = if writer_first {
            "house_interior_writer_first"
        } else {
            "house_interior_revoke_first"
        };
        run_native(tag, async |h, backpack, item| {
            let request = house_request_for_shape(h, backpack, item, 5).await?;
            let seal = h.recovery.seal_current().map_err(native::debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(native::debug)?;
            let candidate = h
                .root
                .freeze_house_item(&authority, &h.node, house_fence()?, request)
                .await
                .map_err(native::debug)?;
            let before = house_value_snapshot(h).await?;
            if !writer_first {
                let mut revoker = h.pool.begin().await?;
                let pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
                    .fetch_one(&mut *revoker)
                    .await?;
                sqlx::query("UPDATE test_house_properties SET acl_revision=2")
                    .execute(&mut *revoker)
                    .await?;
                let (result, observed) = native::join_two(
                    h.root
                        .commit_house_item(&authority, &h.node, house_fence()?, &candidate),
                    async {
                        if let Err(error) = wait_house_blocker(h, pid, None).await {
                            revoker.rollback().await?;
                            return Err(error);
                        }
                        revoker.commit().await?;
                        Ok::<(), Box<dyn std::error::Error>>(())
                    },
                )
                .await;
                observed?;
                assert!(
                    matches!(result, Err(HouseWriterError::AccessChanged)),
                    "{result:?}"
                );
                assert_eq!(house_value_snapshot(h).await?, before);
            } else {
                // Scheduling only: pause the ACTUAL native shape after it has read
                // and locked current access. All original value/authority guards
                // remain installed; this trigger supplies no proof or privilege.
                sqlx::raw_sql("CREATE FUNCTION test_house_pause_shape() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN PERFORM pg_advisory_xact_lock(2163902); RETURN NEW; END $$; CREATE TRIGGER test_house_pause_shape BEFORE INSERT ON game_item_house_interior_locations FOR EACH ROW EXECUTE FUNCTION test_house_pause_shape()")
                    .execute(&h.pool).await?;
                let mut gate = PgConnection::connect(&h.database.url).await?;
                let gate_pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
                    .fetch_one(&mut gate)
                    .await?;
                sqlx::query("SELECT pg_advisory_lock(2163902)")
                    .execute(&mut gate)
                    .await?;
                let mut gate = Some(gate);
                let (result, observed) = native::join_two(
                    h.root
                        .commit_house_item(&authority, &h.node, house_fence()?, &candidate),
                    async move {
                        let observation: TestResult = async {
                            let writer_pid = wait_house_blocker(h, gate_pid, None).await?;
                            let mut revoker = h.pool.begin().await?;
                            let revoker_pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
                                .fetch_one(&mut *revoker)
                                .await?;
                            let (revoked, blocked) = native::join_two(
                                async {
                                    sqlx::query("UPDATE test_house_properties SET acl_revision=2")
                                        .execute(&mut *revoker)
                                        .await?;
                                    revoker.commit().await
                                },
                                async {
                                    let blocked =
                                        wait_house_blocker(h, writer_pid, Some(revoker_pid)).await;
                                    gate.take()
                                        .ok_or("missing scheduling connection")?
                                        .close()
                                        .await?;
                                    blocked?;
                                    Ok::<(), Box<dyn std::error::Error>>(())
                                },
                            )
                            .await;
                            blocked?;
                            revoked?;
                            Ok::<(), Box<dyn std::error::Error>>(())
                        }
                        .await;
                        // Release on observation errors before join waits for the
                        // actual writer; never rely on its deadline for cleanup.
                        if let Some(gate) = gate.take() {
                            gate.close().await?;
                        }
                        observation
                    },
                )
                .await;
                observed?;
                let winner = committed(result.map_err(native::debug)?)?;
                assert_eq!(h.count("game_item_house_interior_locations").await?, 1);
                let row: (String, String) = sqlx::query_as("SELECT placed_transaction_id::text,p.provenance_revision::text FROM game_item_house_interior_locations l JOIN game_item_house_reclaim_provenance p USING(item_instance_id,world_id,house_key) WHERE l.item_instance_id=encode($1,'hex')::uuid")
                    .bind(item.as_slice()).fetch_one(&h.pool).await?;
                assert_eq!(
                    row.0.replace('-', ""),
                    winner
                        .transaction_id
                        .iter()
                        .map(|b| format!("{b:02x}"))
                        .collect::<String>()
                );
                assert_eq!(row.1, "1");
                let values = house_value_snapshot(h).await?;
                let mut take = item_request(
                    item,
                    903,
                    HouseItemAction::Take {
                        placement: winner.transaction_id,
                        provenance_revision: 1,
                    },
                )?;
                take.backpack = Some(HouseBackpackFacts {
                    definition: native::backpack_facts().definition,
                    capacity: 20,
                });
                assert!(matches!(
                    h.root
                        .freeze_house_item(&authority, &h.node, house_fence()?, take)
                        .await,
                    Err(HouseWriterError::AccessChanged)
                ));
                assert_eq!(house_value_snapshot(h).await?, values);
            }
            let revision: String =
                sqlx::query_scalar("SELECT acl_revision::text FROM test_house_properties")
                    .fetch_one(&h.pool)
                    .await?;
            assert_eq!(revision, "2");
            Ok(())
        })?;
    }
    Ok(())
}

fn audit_move() -> house_audit::OneItemHouseTransferV1 {
    let state = native_audit::OneItemStateV1 {
        item_instance_id: uuid(3).to_vec(),
        world_id: uuid(1).to_vec(),
        definition: Some(native_audit::OneItemTypedDefinitionRevisionV1 {
            family: "Item".into(),
            production_key: "fixture:house.item".into(),
            revision_ref: "definition-r1".into(),
        }),
        quantity: 1,
        lifecycle: 1,
        key_number: 0,
    };
    let source = house_audit::OneItemHouseLocationV1 {
        world_id: uuid(1).to_vec(),
        house_key: HOUSE.into(),
        spatial_position: vec![1, 2, 3],
        stack_ordinal: 1,
        scope_ownership_generation: 1,
        placement_transaction_id: uuid(4).to_vec(),
        reclaim_subject: uuid(2).to_vec(),
        provenance_revision: 1,
    };
    let destination = house_audit::OneItemHouseLocationV1 {
        spatial_position: vec![4, 5, 6],
        placement_transaction_id: uuid(5).to_vec(),
        provenance_revision: 2,
        ..source.clone()
    };
    house_audit::OneItemHouseTransferV1 {
        before: Some(state.clone()),
        after: Some(state),
        source: Some(house_audit::HouseTransferSourceV1::House(source)),
        destination: Some(house_audit::HouseTransferDestinationV1::House(destination)),
        cause: Some(
            crate::durability::item_transfer_audit::OneItemTransferCauseV1 {
                typed_cause: "house_interior_transfer".into(),
                command_ref: Some(
                    crate::durability::item_transfer_audit::OneItemCommandRefV1 {
                        game_session_id: uuid(6).to_vec(),
                        command_id: 1,
                    },
                ),
                content_revision: "content-1".into(),
                ruleset_revision: "ruleset-1".into(),
                sim_revision: "sim-1".into(),
            },
        ),
        character_id: uuid(2).to_vec(),
        connection_generation: 1,
        character_lease_generation: 1,
    }
}
fn encode(
    message: house_audit::OneItemHouseTransferV1,
    tx: [u8; 16],
) -> Result<Vec<u8>, native_audit::AuditError> {
    house_audit::encode_house_transfer_event(
        crate::durability::item_transfer_audit::TransferEventIdentity {
            event_id: uuid(8),
            transaction_id: tx,
            occurred_at_unix_ms: 1000,
            server_build_id: "house-native-test",
        },
        message,
        native_audit::Type2EventTuple::V1,
    )
}
#[test]
fn house_audit_current_placement_tx_and_historical_take_are_distinct() {
    let moved = audit_move();
    assert!(encode(moved.clone(), uuid(5)).is_ok());
    assert!(encode(moved.clone(), uuid(9)).is_err());
    let mut place = moved.clone();
    place.source = Some(house_audit::HouseTransferSourceV1::Character(
        house_audit::OneItemHouseCharacterLocationV1 {
            character_id: uuid(2).to_vec(),
            parent_item_instance_id: uuid(7).to_vec(),
            placement_ordinal: 1,
            equipment_slot: 0,
        },
    ));
    if let Some(house_audit::HouseTransferDestinationV1::House(h)) = &mut place.destination {
        h.provenance_revision = 1;
    }
    assert!(encode(place.clone(), uuid(5)).is_ok());
    assert!(encode(place, uuid(9)).is_err());
    let mut take = moved;
    take.destination = Some(house_audit::HouseTransferDestinationV1::Inventory(
        house_audit::OneItemHouseCharacterLocationV1 {
            character_id: uuid(2).to_vec(),
            parent_item_instance_id: uuid(7).to_vec(),
            placement_ordinal: 1,
            equipment_slot: 0,
        },
    ));
    assert!(
        encode(take, uuid(9)).is_ok(),
        "historical source placement is not the new take transaction"
    );
}
#[test]
fn house_audit_maximum_bound_and_canonical_corruption() {
    let mut value = audit_move();
    let long_house = format!(
        "oteryn:content.house.{}",
        "h".repeat(512 - "oteryn:content.house.".len())
    );
    if let Some(house_audit::HouseTransferSourceV1::House(h)) = &mut value.source {
        h.house_key = long_house.clone();
        h.spatial_position = vec![255; 128];
        h.stack_ordinal = u64::MAX;
        h.scope_ownership_generation = u64::MAX;
        h.provenance_revision = u64::MAX - 1;
    }
    if let Some(house_audit::HouseTransferDestinationV1::House(h)) = &mut value.destination {
        h.house_key = long_house;
        h.spatial_position = vec![254; 128];
        h.stack_ordinal = u64::MAX;
        h.scope_ownership_generation = u64::MAX;
        h.provenance_revision = u64::MAX;
    }
    let before = value.before.as_mut().expect("before");
    let definition = before.definition.as_mut().expect("definition");
    definition.family = "f".repeat(128);
    definition.production_key = "k".repeat(512);
    definition.revision_ref = "r".repeat(512);
    before.key_number = 65535;
    value.after = value.before.clone();
    let cause = value.cause.as_mut().expect("cause");
    cause.command_ref.as_mut().expect("command").command_id = u64::MAX;
    cause.content_revision = "c".repeat(512);
    cause.ruleset_revision = "r".repeat(512);
    cause.sim_revision = "s".repeat(512);
    value.connection_generation = u64::MAX;
    value.character_lease_generation = u64::MAX;
    let bytes = house_audit::encode_house_transfer_event(
        crate::durability::item_transfer_audit::TransferEventIdentity {
            event_id: uuid(8),
            transaction_id: uuid(5),
            occurred_at_unix_ms: i64::MAX - native_audit::AUDIT_RETENTION_P90D_MS,
            server_build_id: &"b".repeat(128),
        },
        value,
        native_audit::Type2EventTuple::V1,
    )
    .expect("bounded worst-case House move");
    let (envelope, _) = house_audit::decode_house_transfer_envelope(&bytes).expect("round trip");
    println!(
        "HOUSE worst case payload={} envelope={} (all fields at admitted maxima)",
        envelope.payload.len(),
        bytes.len()
    );
    assert!(envelope.payload.len() <= native_audit::RL07_PAYLOAD_BYTES_MAX);
    assert!(bytes.len() + 128 <= native_audit::RL07_ENVELOPE_BYTES_MAX);
    let mut unknown = envelope.payload.clone();
    unknown.extend_from_slice(&[0x78, 0x01]);
    assert!(house_audit::decode_house_transfer_payload(&unknown).is_err());
    assert!(house_audit::decode_house_transfer_envelope(&bytes[..bytes.len() - 1]).is_err());
    let mut wrong = envelope;
    wrong.channel_id = Some(uuid(9).to_vec());
    assert!(house_audit::decode_house_transfer_envelope(&wrong.encode_to_vec()).is_err());
}

#[test]
fn house_sql_privileged_shapes_require_secret_and_unique_current_access() -> TestResult {
    run_native("house_interior_sql", async |h, backpack, item| {
        let seal = h.recovery.seal_current().map_err(native::debug)?;
        let a = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(native::debug)?;
        let request = item_request(
            item,
            300,
            HouseItemAction::Place {
                source: HouseCharacterSource::Inventory {
                    parent: backpack,
                    ordinal: 1,
                },
                position: vec![1, 2, 3],
            },
        )?;
        let candidate = h
            .root
            .freeze_house_item(&a, &h.node, house_fence()?, request)
            .await
            .map_err(native::debug)?;
        // Charge one real attempt without changing custody, then restore independently current ACL data.
        sqlx::query("UPDATE test_house_properties SET acl_revision=2")
            .execute(&h.pool)
            .await?;
        assert!(matches!(
            h.root
                .commit_house_item(&a, &h.node, house_fence()?, &candidate)
                .await,
            Err(HouseWriterError::AccessChanged)
        ));
        sqlx::query("UPDATE test_house_properties SET acl_revision=1")
            .execute(&h.pool)
            .await?;
        let row=sqlx::query("SELECT transaction_id::text,house_intent FROM game_item_transfer_reservations WHERE command_id=300").fetch_one(&h.pool).await?;
        let mut p: serde_json::Value = row.try_get("house_intent")?;
        p["transaction"] = row.try_get::<String, _>("transaction_id")?.into();
        p["scope_generation"] = "1".into();
        p["connection_generation"] = "1".into();
        p["lease_generation"] = "2".into();
        p["phase"] = "apply".into();
        for function in [
            "game_house_item_place",
            "game_house_item_take",
            "game_house_item_move",
        ] {
            let mut tx = h.pool.begin().await?;
            sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
                .execute(&mut *tx)
                .await?;
            let query = match function {
                "game_house_item_place" => {
                    "SELECT game_house_item_place($1,encode($2,'hex')::uuid,$3::text::numeric,$4)"
                }
                "game_house_item_take" => {
                    "SELECT game_house_item_take($1,encode($2,'hex')::uuid,$3::text::numeric,$4)"
                }
                _ => "SELECT game_house_item_move($1,encode($2,'hex')::uuid,$3::text::numeric,$4)",
            };
            let result = sqlx::query(query)
                .bind(p.clone())
                .bind(h.node.fact().node_id().as_bytes().as_slice())
                .bind(h.node.fact().registration_revision().to_string())
                .bind([99u8; 32].as_slice())
                .execute(&mut *tx)
                .await
                .expect_err("public IDs do not prove opaque possession");
            assert_eq!(
                result.as_database_error().map(|e| e.message()),
                Some("NOT_CURRENT")
            );
            tx.rollback().await?;
        }
        // A real possession proof cannot substitute a different endpoint of the frozen command.
        let mut changed = p.clone();
        changed["destination_position"] = "040506".into();
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
            .execute(&mut *tx)
            .await?;
        let error = sqlx::query(
            "SELECT game_house_item_place($1,encode($2,'hex')::uuid,$3::text::numeric,$4)",
        )
        .bind(changed)
        .bind(h.node.fact().node_id().as_bytes().as_slice())
        .bind(h.node.fact().registration_revision().to_string())
        .bind([1u8; 32].as_slice())
        .execute(&mut *tx)
        .await
        .expect_err("immutable endpoint binding");
        assert_eq!(
            error.as_database_error().map(|e| e.message()),
            Some("CONFLICTING_OCCURRENCE")
        );
        tx.rollback().await?;
        // Same actual source and valid secret reach the mutation. Roll back explicitly: no
        // audit/receipt was inserted, and no value is claimed as committed by this control.
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
            .execute(&mut *tx)
            .await?;
        let facts: serde_json::Value = sqlx::query_scalar(
            "SELECT game_house_item_place($1,encode($2,'hex')::uuid,$3::text::numeric,$4)",
        )
        .bind(p.clone())
        .bind(h.node.fact().node_id().as_bytes().as_slice())
        .bind(h.node.fact().registration_revision().to_string())
        .bind([1u8; 32].as_slice())
        .fetch_one(&mut *tx)
        .await?;
        assert_eq!(facts["destination_position"], "\\x010203");
        tx.rollback().await?;
        sqlx::raw_sql("CREATE OR REPLACE FUNCTION game_house_access(p_world_id UUID,p_house_key TEXT,p_character_id UUID) RETURNS TABLE(role TEXT,content_fenced BOOLEAN,acl_revision NUMERIC(20,0),guild_revisions JSONB) LANGUAGE sql AS $$ SELECT 'OWNER'::text,false,1::numeric,NULL::jsonb UNION ALL SELECT 'GUEST'::text,false,1::numeric,NULL::jsonb $$").execute(&h.pool).await?;
        assert!(matches!(
            h.root
                .commit_house_item(&a, &h.node, house_fence()?, &candidate)
                .await,
            Err(HouseWriterError::NoAccess)
        ));
        assert_eq!(h.count("game_item_house_interior_locations").await?, 0);
        assert_eq!(h.count("game_item_container_entries").await?, 1);
        h.root
            .revoke_node_registration(h.node.fact())
            .await
            .map_err(native::debug)?;
        for query in [
            "SELECT game_house_item_place($1,encode($2,'hex')::uuid,$3::text::numeric,$4)",
            "SELECT game_house_item_take($1,encode($2,'hex')::uuid,$3::text::numeric,$4)",
            "SELECT game_house_item_move($1,encode($2,'hex')::uuid,$3::text::numeric,$4)",
        ] {
            let error = sqlx::query(query)
                .bind(p.clone())
                .bind(h.node.fact().node_id().as_bytes().as_slice())
                .bind(h.node.fact().registration_revision().to_string())
                .bind([1u8; 32].as_slice())
                .execute(&h.pool)
                .await
                .expect_err("ended incarnation is not current even with the genuine old secret");
            assert_eq!(
                error.as_database_error().map(|e| e.message()),
                Some("NOT_CURRENT")
            );
        }

        Ok(())
    })
}

#[test]
fn house_container_slot_requires_empty_and_preserves_single_event() -> TestResult {
    run_native("house_interior_empty", async |h, backpack, item| {
        let seal = h.recovery.seal_current().map_err(native::debug)?;
        let a = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(native::debug)?;
        let req = |command| {
            item_request(
                backpack,
                command,
                HouseItemAction::Place {
                    source: HouseCharacterSource::ContainerSlot,
                    position: vec![1, 2, 3],
                },
            )
        };
        for statement in [
            "UPDATE game_item_container_slots SET world_id=world_id WHERE item_instance_id=encode($1,'hex')::uuid",
            "DELETE FROM game_item_container_slots WHERE item_instance_id=encode($1,'hex')::uuid",
        ] {
            let error = sqlx::query(statement)
                .bind(backpack.as_slice())
                .execute(&h.pool)
                .await
                .expect_err("no owner-produced current physical House proof");
            assert_eq!(
                error.as_database_error().and_then(|e| e.code()).as_deref(),
                Some("23514")
            );
            assert_eq!(
                error.as_database_error().map(|e| e.message()),
                Some("DUR-03 item record is immutable")
            );
        }
        let can_delete:bool=sqlx::query_scalar("SELECT has_table_privilege('oteryn_game_runtime','game_item_container_slots','DELETE')").fetch_one(&h.pool).await?;
        assert!(
            !can_delete,
            "the House source exception grants no runtime table DELETE"
        );
        assert!(matches!(
            h.root
                .freeze_house_item(&a, &h.node, house_fence()?, req(401)?)
                .await,
            Err(HouseWriterError::ItemHasContents)
        ));
        let own = h
            .root
            .freeze_house_item(
                &a,
                &h.node,
                house_fence()?,
                item_request(
                    item,
                    402,
                    HouseItemAction::Place {
                        source: HouseCharacterSource::Inventory {
                            parent: backpack,
                            ordinal: 1,
                        },
                        position: vec![1, 2, 3],
                    },
                )?,
            )
            .await
            .map_err(native::debug)?;
        committed(
            h.root
                .commit_house_item(&a, &h.node, house_fence()?, &own)
                .await
                .map_err(native::debug)?,
        )?;
        let bag = h
            .root
            .freeze_house_item(&a, &h.node, house_fence()?, req(401)?)
            .await
            .map_err(native::debug)?;
        let error = sqlx::query(
            "DELETE FROM game_item_container_slots WHERE item_instance_id=encode($1,'hex')::uuid",
        )
        .bind(backpack.as_slice())
        .execute(&h.pool)
        .await
        .expect_err(
            "prepared reservation and another committed item's proof cannot remove this slot",
        );
        assert_eq!(
            error.as_database_error().and_then(|e| e.code()).as_deref(),
            Some("23514")
        );
        // Reach the real privileged removal, then prove it cannot commit without
        // its same-transaction receipt/event. The rollback must restore the slot.
        sqlx::query("UPDATE test_house_properties SET acl_revision=2")
            .execute(&h.pool)
            .await?;
        assert!(matches!(
            h.root
                .commit_house_item(&a, &h.node, house_fence()?, &bag)
                .await,
            Err(HouseWriterError::AccessChanged)
        ));
        sqlx::query("UPDATE test_house_properties SET acl_revision=1")
            .execute(&h.pool)
            .await?;
        let row=sqlx::query("SELECT transaction_id::text,house_intent FROM game_item_transfer_reservations WHERE command_id=401").fetch_one(&h.pool).await?;
        let mut p: serde_json::Value = row.try_get("house_intent")?;
        p["transaction"] = row.try_get::<String, _>("transaction_id")?.into();
        p["scope_generation"] = "1".into();
        p["connection_generation"] = "1".into();
        p["lease_generation"] = "2".into();
        p["phase"] = "apply".into();
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
            .execute(&mut *tx)
            .await?;
        let facts: serde_json::Value = sqlx::query_scalar(
            "SELECT game_house_item_place($1,encode($2,'hex')::uuid,$3::text::numeric,$4)",
        )
        .bind(p)
        .bind(h.node.fact().node_id().as_bytes().as_slice())
        .bind(h.node.fact().registration_revision().to_string())
        .bind([1u8; 32].as_slice())
        .fetch_one(&mut *tx)
        .await?;
        assert_eq!(facts["source_slot"], 9);
        let error = sqlx::query("SET CONSTRAINTS ALL IMMEDIATE")
            .execute(&mut *tx)
            .await
            .expect_err("a House slot removal cannot commit without its receipt/event");
        assert!(matches!(
            error.as_database_error().and_then(|e| e.code()).as_deref(),
            Some("23514" | "23503")
        ));
        tx.rollback().await?;
        assert_eq!(h.count("game_item_container_slots").await?, 1);
        assert_eq!(h.count("game_item_house_interior_locations").await?, 1);
        let bag = committed(
            h.root
                .commit_house_item(&a, &h.node, house_fence()?, &bag)
                .await
                .map_err(native::debug)?,
        )?;
        let (_, event) = house_audit::decode_house_transfer_envelope(
            bag.envelope.as_deref().ok_or("live audit expected")?,
        )
        .map_err(native::debug)?;
        let Some(house_audit::HouseTransferSourceV1::Character(source)) = event.source else {
            return Err("container source".into());
        };
        assert_eq!(source.equipment_slot, 9);
        assert_eq!(h.count("game_item_container_slots").await?, 0);
        assert_eq!(h.count("game_item_house_interior_locations").await?, 2);
        Ok(())
    })
}

struct EquipmentFixture;
impl crate::durability::equipment_policy_abi::policy_registration::Registered for EquipmentFixture {}
impl crate::durability::equipment_policy_abi::EquipmentPolicyLookup for EquipmentFixture {
    fn source_digest(&self) -> [u8; 32] {
        [1; 32]
    }
    fn equipment_policy(
        &self,
        key: &str,
        revision: &str,
    ) -> Option<crate::durability::equipment_policy_abi::QualifiedEquipmentPolicy> {
        use crate::durability::equipment_policy_abi::{EquipmentClaims, QualifiedEquipmentPolicy};
        if revision != "definition-r1" || ![native::BACKPACK, "fixture:house.item"].contains(&key) {
            return None;
        }
        let claims = [1, 2, 3, 4, 5, 6, 7, 8, 10]
            .into_iter()
            .map(|slot| {
                (
                    slot,
                    if slot == 5 && key == "fixture:house.item" {
                        Ok(EquipmentClaims {
                            slots: vec![5],
                            groups: vec![],
                            level: 0,
                            vocations: vec![],
                        })
                    } else {
                        Err("fixture slot incompatible")
                    },
                )
            })
            .collect();
        QualifiedEquipmentPolicy::from_qualified_content(
            [1; 32],
            claims,
            (key == native::BACKPACK).then_some(20),
        )
    }
}
impl crate::durability::character_equipment::EquipmentMoveContent for EquipmentFixture {
    fn is_plain_item(&self, key: &str, _revision: &str) -> bool {
        key == "fixture:house.item"
    }
    fn is_timed_item(&self, _key: &str, _revision: &str) -> bool {
        false
    }
}
#[test]
fn house_equipped_source_uses_native_owner_without_second_receipt() -> TestResult {
    let Some(admin) = native::configured_admin() else {
        return Err("House PostgreSQL cases require the canonical PG17.6 executor URL".into());
    };
    native::runtime()?.block_on(async{
        let h=native::Harness::create(admin,"house_interior_equipment").await?;
        let result: TestResult=async{
            let(_,item)=funded(&h).await?;let seal=h.recovery.seal_current().map_err(native::debug)?;let a=h.root.open_character_authority(&seal).await.map_err(native::debug)?;
            sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)").bind(native::id(native::WORLD).as_slice()).bind(native::id(native::CHANNEL).as_slice()).execute(&h.pool).await?;
            sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)").bind(native::id(native::WORLD).as_slice()).bind(native::id(native::CHANNEL).as_slice()).bind([1u8;32].as_slice()).bind([2u8;32].as_slice()).bind([3u8;32].as_slice()).execute(&h.pool).await?;
            let mut tx=h.pool.begin().await?;sqlx::query("SET LOCAL ROLE oteryn_game_runtime").execute(&mut *tx).await?;
            let equipment=crate::durability::character_equipment::assert_equipment_authority_in_transaction(&mut tx,&h.root,&a,&h.node,&native::fence()?,native::command(90)?,[1;32]).await.map_err(native::debug)?;
            crate::durability::character_equipment::initialize_equipment_in_transaction(&mut tx,&equipment).await.map_err(native::debug)?;tx.commit().await?;
            let equipped=h.root.commit_equipment_move(&a,&h.node,&native::fence()?,native::command(91)?,&EquipmentFixture,crate::durability::character_equipment::EquipmentMove{item,from_slot:None,to_slot:Some(5)}).await?.map_err(native::debug)?;
            assert_eq!(equipped.revision_after,2);seed_house(&h).await?;
            let candidate=h.root.freeze_house_item(&a,&h.node,house_fence()?,item_request(item,500,HouseItemAction::Place{source:HouseCharacterSource::Equipment{slot:5},position:vec![1,2,3]})?).await.map_err(native::debug)?;
            let receipt=committed(h.root.commit_house_item(&a,&h.node,house_fence()?,&candidate).await.map_err(native::debug)?)?;
            let(_,event)=house_audit::decode_house_transfer_envelope(receipt.envelope.as_deref().ok_or("live audit expected")?).map_err(native::debug)?;
            assert!(matches!(event.source,Some(house_audit::HouseTransferSourceV1::Character(ref source)) if source.equipment_slot==5));
            let revision:String=sqlx::query_scalar("SELECT revision::text FROM game_character_equipment_state").fetch_one(&h.pool).await?;
            assert_eq!(revision,"3");assert_eq!(h.count("game_character_equipment_receipts").await?,1);assert_eq!(h.count("game_character_equipment_audit_outbox").await?,1);assert_eq!(h.count("game_character_equipment_slots").await?,0);Ok(())
        }.await;let cleanup=h.cleanup().await;result?;cleanup
    })
}

#[test]
fn house_production_stub_preflight_leaves_no_trace_for_all_shapes() -> TestResult {
    run_native("house_interior_noaccess", async |h, backpack, item| {
        let seal = h.recovery.seal_current().map_err(native::debug)?;
        let a = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(native::debug)?;
        // Genuine native House session/assignment was committed while independently granted.
        // Restoring the exact production no-row access body revokes all current eligibility.
        sqlx::raw_sql("CREATE OR REPLACE FUNCTION game_house_access(p_world_id UUID,p_house_key TEXT,p_character_id UUID) RETURNS TABLE(role TEXT,content_fenced BOOLEAN,acl_revision NUMERIC(20,0),guild_revisions JSONB) LANGUAGE sql AS $$ SELECT NULL::text,NULL::boolean,NULL::numeric(20,0),NULL::jsonb WHERE false $$").execute(&h.pool).await?;
        let relations = [
            "game_item_transfer_reservations",
            "game_item_transfer_receipts",
            "game_item_house_shape_proofs",
            "game_item_audit_outbox",
            "game_item_house_interior_locations",
            "game_item_house_reclaim_provenance",
            "game_item_container_entries",
        ];
        let mut before = Vec::new();
        for table in relations {
            before.push(h.count(table).await?);
        }
        for (offset, action) in [
            HouseItemAction::Place {
                source: HouseCharacterSource::Inventory {
                    parent: backpack,
                    ordinal: 1,
                },
                position: vec![1, 2, 3],
            },
            HouseItemAction::Take {
                placement: uuid(8),
                provenance_revision: 1,
            },
            HouseItemAction::TileMove {
                placement: uuid(8),
                provenance_revision: 1,
                position: vec![4, 5, 6],
            },
        ]
        .into_iter()
        .enumerate()
        {
            let mut request = item_request(item, 600 + offset as u64, action)?;
            if matches!(request.action, HouseItemAction::Take { .. }) {
                request.backpack = Some(HouseBackpackFacts {
                    definition: native::backpack_facts().definition,
                    capacity: 20,
                });
            }
            assert!(matches!(
                h.root
                    .freeze_house_item(&a, &h.node, house_fence()?, request)
                    .await,
                Err(HouseWriterError::NoAccess)
            ));
        }
        let mut after = Vec::new();
        for table in relations {
            after.push(h.count(table).await?);
        }
        assert_eq!(before, after);
        Ok(())
    })
}

#[test]
fn house_storage_limit_synthetic_occupancy_genuine_source_boundary() -> TestResult {
    run_native("house_interior_capacity", async |h, backpack, item| {
        let mut admin = PgConnection::connect(&h.database.url).await?;
        house_test_fixture::seed_inactive_occupancy(
            &mut admin,
            native::id(native::WORLD).as_slice(),
            HOUSE,
            native::id(native::CHARACTER).as_slice(),
            1999,
        )
        .await?;
        let seal = h.recovery.seal_current().map_err(native::debug)?;
        let a = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(native::debug)?;
        assert_eq!(h.count("game_item_house_interior_locations").await?, 1999);
        let events = h.count("game_item_audit_outbox").await?;
        let request = item_request(
            item,
            700,
            HouseItemAction::Place {
                source: HouseCharacterSource::Inventory {
                    parent: backpack,
                    ordinal: 1,
                },
                position: vec![1, 2, 3],
            },
        )?;
        let candidate = h
            .root
            .freeze_house_item(&a, &h.node, house_fence()?, request)
            .await
            .map_err(native::debug)?;
        let r = committed(
            h.root
                .commit_house_item(&a, &h.node, house_fence()?, &candidate)
                .await
                .map_err(native::debug)?,
        )?;
        let (_, event) = house_audit::decode_house_transfer_envelope(
            r.envelope.as_deref().ok_or("live audit expected")?,
        )
        .map_err(native::debug)?;
        assert!(
            matches!(event.destination,Some(house_audit::HouseTransferDestinationV1::House(ref h)) if h.stack_ordinal==2000)
        );
        assert_eq!(h.count("game_item_house_interior_locations").await?, 2000);
        assert_eq!(h.count("game_item_audit_outbox").await?, events + 1);
        let request = item_request(
            backpack,
            701,
            HouseItemAction::Place {
                source: HouseCharacterSource::ContainerSlot,
                position: vec![4, 5, 6],
            },
        )?;
        assert!(matches!(
            h.root
                .freeze_house_item(&a, &h.node, house_fence()?, request)
                .await,
            Err(HouseWriterError::StorageFull)
        ));
        assert_eq!(h.count("game_item_house_interior_locations").await?, 2000);
        assert_eq!(h.count("game_item_audit_outbox").await?, events + 1);
        assert_eq!(h.count("game_item_container_slots").await?, 1);
        let no_reservation: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_item_transfer_reservations WHERE command_id=701",
        )
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(no_reservation, 0);
        Ok(())
    })
}

#[test]
fn house_sql_role_table_own_inventory_and_foreign_reclaim_data() -> TestResult {
    run_native("house_interior_roles", async |h, backpack, item| {
        let seal = h.recovery.seal_current().map_err(native::debug)?;
        let a = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(native::debug)?;
        for (index, role) in ["OWNER", "SUBOWNER", "GUEST"].into_iter().enumerate() {
            sqlx::query("UPDATE test_house_grants SET role=$1")
                .bind(role)
                .execute(&h.pool)
                .await?;
            let ordinal:String=sqlx::query_scalar("SELECT placement_ordinal::text FROM game_item_container_entries WHERE item_instance_id=encode($1,'hex')::uuid").bind(item.as_slice()).fetch_one(&h.pool).await?;
            let request = item_request(
                item,
                800 + index as u64 * 2,
                HouseItemAction::Place {
                    source: HouseCharacterSource::Inventory {
                        parent: backpack,
                        ordinal: ordinal.parse()?,
                    },
                    position: vec![1, 2, 3],
                },
            )?;
            let frozen = h
                .root
                .freeze_house_item(&a, &h.node, house_fence()?, request)
                .await
                .map_err(native::debug)?;
            let placed = committed(
                h.root
                    .commit_house_item(&a, &h.node, house_fence()?, &frozen)
                    .await
                    .map_err(native::debug)?,
            )?;
            let mut request = item_request(
                item,
                801 + index as u64 * 2,
                HouseItemAction::Take {
                    placement: placed.transaction_id,
                    provenance_revision: 1,
                },
            )?;
            request.backpack = Some(HouseBackpackFacts {
                definition: native::backpack_facts().definition,
                capacity: 20,
            });
            let frozen = h
                .root
                .freeze_house_item(&a, &h.node, house_fence()?, request)
                .await
                .map_err(native::debug)?;
            committed(
                h.root
                    .commit_house_item(&a, &h.node, house_fence()?, &frozen)
                    .await
                    .map_err(native::debug)?,
            )?;
        }
        let mut admin = PgConnection::connect(&h.database.url).await?;
        let subject = house_test_fixture::seed_inactive_reclaim_subject(
            &mut admin,
            native::id(native::CHARACTER).as_slice(),
        )
        .await?;
        for (index, role) in ["SUBOWNER", "OWNER"].into_iter().enumerate() {
            house_test_fixture::seed_inactive_occupancy(
                &mut admin,
                native::id(native::WORLD).as_slice(),
                HOUSE,
                &subject,
                1,
            )
            .await?;
            let(item,placement):(Vec<u8>,Vec<u8>)=sqlx::query_as("SELECT uuid_send(item_instance_id),uuid_send(placed_transaction_id) FROM game_item_house_interior_locations").fetch_one(&h.pool).await?;
            let mut request = item_request(
                item.try_into().map_err(|_| "item UUID")?,
                820 + index as u64,
                HouseItemAction::Take {
                    placement: placement.try_into().map_err(|_| "placement UUID")?,
                    provenance_revision: 1,
                },
            )?;
            request.backpack = Some(HouseBackpackFacts {
                definition: native::backpack_facts().definition,
                capacity: 20,
            });
            sqlx::query("UPDATE test_house_grants SET role='GUEST'")
                .execute(&h.pool)
                .await?;
            let before = h.count("game_item_audit_outbox").await?;
            assert!(matches!(
                h.root
                    .freeze_house_item(&a, &h.node, house_fence()?, request.clone())
                    .await,
                Err(HouseWriterError::NoAccess)
            ));
            assert_eq!(h.count("game_item_audit_outbox").await?, before);
            assert_eq!(h.count("game_item_house_interior_locations").await?, 1);
            sqlx::query("UPDATE test_house_grants SET role=$1")
                .bind(role)
                .execute(&h.pool)
                .await?;
            let frozen = h
                .root
                .freeze_house_item(&a, &h.node, house_fence()?, request)
                .await
                .map_err(native::debug)?;
            committed(
                h.root
                    .commit_house_item(&a, &h.node, house_fence()?, &frozen)
                    .await
                    .map_err(native::debug)?,
            )?;
            assert_eq!(h.count("game_item_house_interior_locations").await?, 0);
        }
        Ok(())
    })
}

#[test]
fn house_audit_closed_endpoints_and_duplicate_oneof_are_refused() {
    for field in 0..4 {
        let mut bad = audit_move();
        match field {
            0 => bad.source = None,
            1 => bad.destination = None,
            2 => bad.after.as_mut().expect("state").quantity = 2,
            _ => {
                if let Some(house_audit::HouseTransferDestinationV1::House(h)) =
                    &mut bad.destination
                {
                    h.house_key.push('x');
                }
            }
        }
        assert!(house_audit::check_house_transfer(&bad).is_err());
    }
    let mut oversized = audit_move();
    if let Some(house_audit::HouseTransferSourceV1::House(h)) = &mut oversized.source {
        h.house_key = format!(
            "oteryn:content.house.{}",
            "h".repeat(513 - "oteryn:content.house.".len())
        );
    }
    assert!(
        house_audit::check_house_transfer(&oversized).is_err(),
        "the initial513-byte maximum probe is a max+1 refusal"
    );
    let value = audit_move();
    let alternative = house_audit::OneItemHouseCharacterLocationV1 {
        character_id: uuid(2).to_vec(),
        parent_item_instance_id: uuid(7).to_vec(),
        placement_ordinal: 1,
        equipment_slot: 0,
    }
    .encode_to_vec();
    let mut operation = vec![0x22];
    prost::encoding::encode_varint(alternative.len() as u64, &mut operation);
    operation.extend(alternative);
    operation.extend(value.encode_to_vec());
    let mut payload = vec![0x08, 1, 0x5a];
    prost::encoding::encode_varint(operation.len() as u64, &mut payload);
    payload.extend(operation);
    let ordinary = native_audit::OneItemTransactionV1::decode(payload.as_slice())
        .expect("ordinary protobuf decodes last source");
    let Some(native_audit::OneItemOperationV1::HouseTransfer(last)) = ordinary.operation else {
        unreachable!()
    };
    assert!(
        house_audit::check_house_transfer(&last).is_ok(),
        "independent last-wins semantic control"
    );
    assert!(
        house_audit::decode_house_transfer_payload(&payload).is_err(),
        "canonical decoder refuses duplicate sources even when last-wins value is valid"
    );
    assert!(matches!(
        house_audit::decode_house_transfer_payload(&vec![
            0;
            native_audit::RL07_PAYLOAD_BYTES_MAX + 1
        ]),
        Err(native_audit::AuditError::CapacityExceeded)
    ));
}

#[test]
fn house_loader_requires_independent_current_incarnation_and_exact_assignment() -> TestResult {
    use crate::durability::house_interior::{
        HouseInteriorLoadError, load_house_interior_in_transaction,
    };
    run_native("house_loader_current", async |h, _, _| {
        let wrong = NodeIncarnationProof::new(h.node.fact(), BootstrapSecret::from_bytes([99; 32]));
        let other_house = HouseId::new(current_house()?.world_id(), "oteryn:content.house.other")
            .ok_or("other house")?;
        let other_world = HouseId::new(
            WorldId::decode(&native::id(97)).map_err(native::debug)?,
            HOUSE,
        )
        .ok_or("other world")?;
        for (house, generation, node) in [
            (current_house()?, 1, &wrong),
            (current_house()?, 2, &h.node),
            (other_house, 1, &h.node),
            (other_world, 1, &h.node),
        ] {
            let mut tx = h.pool.begin().await?;
            sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
                .execute(&mut *tx)
                .await?;
            assert!(matches!(
                load_house_interior_in_transaction(&mut tx, &house, generation, node).await,
                Err(HouseInteriorLoadError::NotCurrent)
            ));
            tx.rollback().await?;
        }
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
            .execute(&mut *tx)
            .await?;
        let empty = load_house_interior_in_transaction(&mut tx, &current_house()?, 1, &h.node)
            .await
            .map_err(native::debug)?;
        assert!(empty.items.is_empty());
        assert_eq!(empty.house, current_house()?);
        assert_eq!(empty.ownership_generation, 1);
        let readable: bool = sqlx::query_scalar("SELECT has_table_privilege(current_user,'game_item_house_interior_locations','SELECT') OR has_table_privilege(current_user,'game_item_house_reclaim_provenance','SELECT')")
            .fetch_one(&mut *tx).await?;
        assert!(!readable, "loader must not open blanket table reads");
        tx.rollback().await?;
        Ok(())
    })
}

#[test]
fn house_loader_reads_actual_native_shape_identity_location_and_provenance() -> TestResult {
    use crate::durability::house_interior::load_house_interior_in_transaction;
    run_native("house_loader_native", async |h, backpack, item| {
        let seal = h.recovery.seal_current().map_err(native::debug)?;
        let a = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(native::debug)?;
        let request = item_request(
            item,
            390,
            HouseItemAction::Place {
                source: HouseCharacterSource::Inventory {
                    parent: backpack,
                    ordinal: 1,
                },
                position: vec![1, 2, 3],
            },
        )?;
        let candidate = h
            .root
            .freeze_house_item(&a, &h.node, house_fence()?, request)
            .await
            .map_err(native::debug)?;
        let receipt = committed(
            h.root
                .commit_house_item(&a, &h.node, house_fence()?, &candidate)
                .await
                .map_err(native::debug)?,
        )?;
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
            .execute(&mut *tx)
            .await?;
        let snapshot = load_house_interior_in_transaction(&mut tx, &current_house()?, 1, &h.node)
            .await
            .map_err(native::debug)?;
        assert_eq!(snapshot.items.len(), 1);
        let loaded = &snapshot.items[0];
        assert_eq!(loaded.item_instance_id, item);
        assert_eq!(loaded.definition.family, "Item");
        assert_eq!(loaded.definition.production_key, "fixture:house.item");
        assert_eq!(loaded.definition.revision_ref, "definition-r1");
        assert_eq!(loaded.quantity, 1);
        assert_eq!(loaded.spatial_position, vec![1, 2, 3]);
        assert_eq!(loaded.placement_transaction_id, receipt.transaction_id);
        assert_eq!(
            loaded.reclaim_subject_character_id,
            native::id(native::CHARACTER)
        );
        assert_eq!(loaded.provenance_revision, 1);
        tx.rollback().await?;
        Ok(())
    })
}

#[test]
fn house_loader_retains_registration_and_assignment_locks_in_callers_transaction() -> TestResult {
    use crate::durability::house_interior::load_house_interior_in_transaction;
    run_native("house_loader_locks", async |h, _, _| {
        let mut held = h.pool.begin().await?;
        sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
            .execute(&mut *held)
            .await?;
        load_house_interior_in_transaction(&mut held, &current_house()?, 1, &h.node)
            .await
            .map_err(native::debug)?;
        for query in [
            "SELECT 1 FROM game_node_registrations WHERE node_id=encode($1,'hex')::uuid FOR UPDATE NOWAIT",
            "SELECT 1 FROM game_runtime_scope_assignments WHERE holder_node_id=encode($1,'hex')::uuid AND scope_kind=2 FOR UPDATE NOWAIT",
        ] {
            let mut competitor = h.pool.begin().await?;
            let result = sqlx::query(query)
                .bind(h.node.fact().node_id().as_bytes().as_slice())
                .execute(&mut *competitor)
                .await;
            assert!(
                matches!(result, Err(sqlx::Error::Database(ref e)) if e.code().as_deref()==Some("55P03")),
                "current authority must stay locked until caller TX ends"
            );
            competitor.rollback().await?;
        }
        held.rollback().await?;
        let mut successor = h.pool.begin().await?;
        sqlx::query(
            "SELECT 1 FROM game_runtime_scope_assignments WHERE scope_kind=2 FOR UPDATE NOWAIT",
        )
        .execute(&mut *successor)
        .await?;
        successor.rollback().await?;
        Ok(())
    })
}
