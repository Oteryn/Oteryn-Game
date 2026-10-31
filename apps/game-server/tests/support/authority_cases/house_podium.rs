//! PODIUM-1 owner-only storage qualification over the catalogue's real podium items
//! (W18, migration 0561). `house_podium_fenced` drives the runtime path.
use crate::durability::house_interior::{
    HouseCharacterSource, HouseItemAction, HouseItemOutcome, HouseItemRequest,
};
use crate::durability::house_scope_handoff::{
    HouseCommitOutcome, HouseEntryCommit, HouseEntryRequest, HouseHandoffDirection, HouseId,
    HousePrepareOutcome, HouseScopeAssignmentCommand, HouseScopeAssignmentOutcome,
    HouseScopeAssignmentRequest,
};
use crate::durability::item_transfer::{
    CurrentCharacterItemFence, ItemStackClass, ItemTransferOutcome,
};
use crate::durability::runtime_scope_assignment::{ControlActor, OperationKey};
use crate::foundation::{CharacterId, CommandId, CommandRef, GameSessionId, WorldId};
use crate::item_transfer_postgres_cases as native;
use crate::item_transfer_postgres_cases::{
    CHARACTER, Harness, TestResult, WORLD, configured_admin, id, runtime,
};
use sqlx::{Connection, PgConnection};
use std::collections::BTreeMap;

const HOUSE: &str = "oteryn:content.house.podium_test";
const SET: &str = "SELECT game_house_podium_set(encode($1,'hex')::uuid,$2,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5::smallint,$6::smallint,$7,$8::jsonb,$9::text::numeric)::text";
const LOAD: &str =
    "SELECT display_key,revision FROM game_house_podium_load(encode($1,'hex')::uuid,$2)";

pub(super) const RENOWN: &str = "oteryn:item.tibia.i35973";
pub(super) const VIGOUR: &str = "oteryn:item.tibia.i38707";
pub(super) const TENACITY: &str = "oteryn:item.tibia.i42367";
const NON_PODIUM: &str = "oteryn:content.item.synthetic_non_podium";

// Independent test catalogue only; these weights do not assert donor parity.
struct PodiumFixtureWeights;
impl crate::durability::item_transfer::ItemWeightLookup for PodiumFixtureWeights {
    fn generation_digest(&self) -> [u8; 32] {
        crate::durability::item_weight_lookup::GENERATION_DIGEST
    }
    fn weight_of(
        &self,
        definition: &crate::durability::item_mint::TypedDefinitionRef,
    ) -> Result<u32, crate::durability::item_transfer::ItemWeightRefused> {
        if definition.family == "Item"
            && definition.revision_ref == "definition-r1"
            && matches!(
                definition.production_key.as_str(),
                RENOWN | VIGOUR | TENACITY | NON_PODIUM
            )
        {
            Ok(1)
        } else {
            crate::durability::item_weight_lookup::TEST_WEIGHTS.weight_of(definition)
        }
    }
}

fn assert_error(result: Result<String, sqlx::Error>, message: &str) {
    let error = result.expect_err(message);
    assert_eq!(
        error.as_database_error().map(|e| e.message()),
        Some(message)
    );
}

pub(super) fn current_house() -> TestResult<HouseId> {
    Ok(HouseId::new(
        WorldId::decode(&native::id(native::WORLD)).map_err(native::debug)?,
        HOUSE,
    )
    .ok_or("house")?)
}
pub(super) fn house_fence() -> TestResult<CurrentCharacterItemFence> {
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
    sqlx::query(include_str!("../owned_house_fixture.sql"))
        .bind(id(WORLD).as_slice())
        .bind(HOUSE)
        .bind(id(CHARACTER).as_slice())
        .execute(&mut c)
        .await?;
    sqlx::query("INSERT INTO game_house_tiles VALUES(encode($1,'hex')::uuid,$2,$3)")
        .bind(id(WORLD).as_slice())
        .bind(HOUSE)
        .bind(vec![1_u8, 2, 3])
        .execute(&mut c)
        .await?;
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

/// Renown 91, vigour 92, non-podium 93 and tenacity 94, placed in the entered House with
/// real MINT, inventory TRANSFER, house entry and placement; no triggers or constraints are
/// disabled for fixture creation.
pub(super) async fn placed_podiums(h: &Harness) -> TestResult<BTreeMap<u8, [u8; 16]>> {
    let seal = h.recovery.seal_current().map_err(native::debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(native::debug)?;
    let backpack = h.mint(&authority, native::BACKPACK, 1).await?;
    assert!(matches!(
        h.transfer(
            &authority,
            native::fence()?,
            native::to_slot(native::command(1)?, backpack, native::backpack_facts())
        )
        .await
        .map_err(native::debug)?,
        ItemTransferOutcome::Committed(_)
    ));
    let mut items = BTreeMap::new();
    for (seed, key) in [
        (91_u8, RENOWN),
        (92, VIGOUR),
        (93, NON_PODIUM),
        (94, TENACITY),
    ] {
        let item = h.mint(&authority, key, 1).await?;
        let fence = native::fence()?;
        let request = native::to_backpack(
            native::command(u64::from(seed))?,
            item,
            native::facts(key, ItemStackClass::NonStackable),
        );
        let mut frozen = h
            .root
            .freeze_item_transfer(&authority, &h.node, fence, request, &PodiumFixtureWeights)
            .await
            .map_err(native::debug)?;
        let permit = crate::durability::spell_owner_commit::SpellLanePermit::of_fresh_scope(
            fence.runtime_scope,
        )
        .await
        .map_err(native::debug)?;
        assert!(matches!(
            h.root
                .commit_item_transfer(
                    &permit,
                    &authority,
                    &h.node,
                    fence,
                    &mut frozen,
                    &PodiumFixtureWeights
                )
                .await
                .map_err(native::debug)?,
            ItemTransferOutcome::Committed(_)
        ));
        items.insert(seed, item);
    }
    seed_house(h).await?;
    for (ordinal, item) in items.values().enumerate() {
        let request = item_request(
            *item,
            100 + ordinal as u64,
            HouseItemAction::Place {
                source: HouseCharacterSource::Inventory {
                    parent: backpack,
                    ordinal: ordinal as u64 + 1,
                },
                position: vec![1, 2, 3],
            },
        )?;
        let frozen = h
            .root
            .freeze_house_item(&authority, &h.node, house_fence()?, request)
            .await
            .map_err(native::debug)?;
        assert!(matches!(
            h.root
                .commit_house_item(&authority, &h.node, house_fence()?, &frozen)
                .await
                .map_err(native::debug)?,
            HouseItemOutcome::Committed(_)
        ));
    }
    Ok(items)
}

#[test]
fn house_podium_set_persists_across_connection_restart_and_refuses_missing_access() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async {
        let h = Harness::create(admin, "house_podium").await?;
        let result: TestResult = async {
            let items = placed_podiums(&h).await?;
            let item_id = |seed| items.get(&seed).copied().unwrap_or(id(seed));
            for (seed, kind, display_kind, key) in [
                (91, 1_i16, 1_i16, "oteryn:content.outfit.synthetic_test"),
                (92, 2, 3, "oteryn:creature.synthetic_boss"),
                (94, 3, 3, "oteryn:creature.synthetic_rat"),
            ] {
                let revision: String = sqlx::query_scalar(SET)
                    .bind(id(WORLD).as_slice()).bind(HOUSE).bind(item_id(seed).as_slice())
                    .bind(id(CHARACTER).as_slice()).bind(kind).bind(display_kind)
                    .bind(key).bind(r#"{"direction":2}"#).bind("0")
                    .fetch_one(&h.pool).await?;
                assert_eq!(revision, "1");
            }
            // A distinct physical connection proves committed persistence.
            let mut original = h.pool.acquire().await?;
            let original_pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
                .fetch_one(&mut *original).await?;
            let mut restarted = sqlx::PgConnection::connect(&h.database.url).await?;
            let restarted_pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
                .fetch_one(&mut restarted).await?;
            assert_ne!(original_pid, restarted_pid);
            original.close().await?;
            let persisted: Vec<(String, String)> = sqlx::query_as(LOAD)
                .bind(id(WORLD).as_slice()).bind(HOUSE).fetch_all(&mut restarted).await?;
            let mut persisted = persisted;
            persisted.sort();
            assert_eq!(persisted, vec![
                ("oteryn:content.outfit.synthetic_test".into(), "1".into()),
                ("oteryn:creature.synthetic_boss".into(), "1".into()),
                ("oteryn:creature.synthetic_rat".into(), "1".into()),
            ]);
            let variant: String = sqlx::query_scalar("SELECT display_variant::text FROM game_house_podium_load(encode($1,'hex')::uuid,$2) WHERE podium_kind=1")
                .bind(id(WORLD).as_slice()).bind(HOUSE).fetch_one(&mut restarted).await?;
            assert_eq!(variant, r#"{"direction": 2}"#);
            restarted.close().await?;

            // Every refusal asserts its actual reason, rather than accepting
            // any SQL error, and leaves the previously stored row unchanged.
            for (seed, actor, kind, display_kind, key, variant, expected, reason) in [
                (91, 99, Some(1_i16), Some(1_i16), Some("oteryn:content.outfit.synthetic_test"), "{}", "1", "PODIUM_NO_ACCESS"),
                (93, CHARACTER, Some(1), Some(1), Some("oteryn:content.outfit.synthetic_test"), "{}", "0", "PODIUM_WRONG_ITEM"),
                (99, CHARACTER, Some(1), Some(1), Some("oteryn:content.outfit.synthetic_test"), "{}", "0", "PODIUM_NOT_HOUSE_ITEM"),
                (91, CHARACTER, Some(2), Some(1), Some("oteryn:content.outfit.synthetic_test"), "{}", "1", "PODIUM_WRONG_ITEM"),
                (91, CHARACTER, Some(1), Some(1), Some("oteryn:content.outfit.synthetic_test"), "{}", "0", "PODIUM_CHANGED"),
                (91, CHARACTER, None, Some(1), Some("oteryn:content.outfit.synthetic_test"), "{}", "1", "PODIUM_INVALID_INPUT"),
                (91, CHARACTER, Some(1), None, Some("oteryn:content.outfit.synthetic_test"), "{}", "1", "PODIUM_INVALID_INPUT"),
                (91, CHARACTER, Some(1), Some(1), None, "{}", "1", "PODIUM_INVALID_INPUT"),
                (91, CHARACTER, Some(1), Some(1), Some("oteryn:mount.synthetic_test"), "{}", "1", "PODIUM_INVALID_INPUT"),
                (91, CHARACTER, Some(1), Some(1), Some("oteryn:content.mount.synthetic_test"), "{}", "1", "PODIUM_INVALID_INPUT"),
                (91, CHARACTER, Some(1), Some(3), Some("oteryn:creature.synthetic_rat"), "{}", "1", "PODIUM_WRONG_DISPLAY"),
                (92, CHARACTER, Some(2), Some(2), Some("oteryn:content.mount.synthetic_test"), "{}", "1", "PODIUM_WRONG_DISPLAY"),
                (94, CHARACTER, Some(3), Some(1), Some("oteryn:content.outfit.synthetic_test"), "{}", "1", "PODIUM_WRONG_DISPLAY"),
                (94, CHARACTER, Some(2), Some(3), Some("oteryn:creature.synthetic_rat"), "{}", "1", "PODIUM_WRONG_ITEM"),
                (91, CHARACTER, Some(4), Some(1), Some("oteryn:content.outfit.synthetic_test"), "{}", "1", "PODIUM_INVALID_INPUT"),
                (91, CHARACTER, Some(1), Some(1), Some("invalid"), "{}", "1", "PODIUM_INVALID_INPUT"),
                (91, CHARACTER, Some(1), Some(1), Some("oteryn:content.outfit.synthetic_test"), "[]", "1", "PODIUM_INVALID_INPUT"),
                (91, CHARACTER, Some(1), Some(1), Some("oteryn:content.outfit.synthetic_test"), "{}", "-1", "PODIUM_INVALID_INPUT"),
                (91, CHARACTER, Some(1), Some(1), Some("oteryn:content.outfit.synthetic_test"), "{}", "0.5", "PODIUM_INVALID_INPUT"),
                (91, CHARACTER, Some(1), Some(1), Some("oteryn:content.outfit.synthetic_test"), "{}", "NaN", "PODIUM_INVALID_INPUT"),
                (91, CHARACTER, Some(1), Some(1), Some("oteryn:content.outfit.synthetic_test"), "{}", "18446744073709551615", "PODIUM_INVALID_INPUT"),
            ] {
                assert_error(sqlx::query_scalar(SET)
                    .bind(id(WORLD).as_slice()).bind(HOUSE).bind(item_id(seed).as_slice())
                    .bind(id(actor).as_slice()).bind(kind).bind(display_kind)
                    .bind(key).bind(variant).bind(expected).fetch_one(&h.pool).await, reason);
            }
            let oversized = format!(r#"{{"value":"{}"}}"#, "x".repeat(1024));
            assert_error(sqlx::query_scalar(SET)
                .bind(id(WORLD).as_slice()).bind(HOUSE).bind(item_id(91).as_slice())
                .bind(id(CHARACTER).as_slice()).bind(1_i16).bind(1_i16)
                .bind("oteryn:content.outfit.synthetic_test").bind(oversized).bind("1")
                .fetch_one(&h.pool).await, "PODIUM_INVALID_INPUT");
            let unchanged: Vec<(String, String)> = sqlx::query_as(LOAD)
                .bind(id(WORLD).as_slice()).bind(HOUSE).fetch_all(&h.pool).await?;
            let mut unchanged = unchanged;
            unchanged.sort();
            assert_eq!(unchanged, persisted);

            let revision: String = sqlx::query_scalar(SET)
                .bind(id(WORLD).as_slice()).bind(HOUSE).bind(item_id(91).as_slice())
                .bind(id(CHARACTER).as_slice()).bind(1_i16).bind(2_i16)
                .bind("oteryn:content.mount.synthetic_test").bind("{}").bind("1")
                .fetch_one(&h.pool).await?;
            assert_eq!(revision, "2");
            let other_house: Vec<(String, String)> = sqlx::query_as(LOAD)
                .bind(id(WORLD).as_slice()).bind("oteryn:content.house.other")
                .fetch_all(&h.pool).await?;
            assert!(other_house.is_empty());
            // A transaction-local removal must immediately hide the display.
            let mut tx = h.pool.begin().await?;
            sqlx::query("DELETE FROM game_item_house_interior_locations WHERE item_instance_id=encode($1,'hex')::uuid")
                .bind(item_id(91).as_slice()).execute(&mut *tx).await?;
            let visible: Vec<(String, String)> = sqlx::query_as(LOAD)
                .bind(id(WORLD).as_slice()).bind(HOUSE).fetch_all(&mut *tx).await?;
            assert_eq!(visible, vec![
                ("oteryn:creature.synthetic_boss".into(), "1".into()),
                ("oteryn:creature.synthetic_rat".into(), "1".into()),
            ]);
            tx.rollback().await?;
            let privileges: (bool, bool, bool, bool) = sqlx::query_as(
                "SELECT has_function_privilege('oteryn_game_runtime','game_house_podium_set(uuid,text,uuid,uuid,smallint,smallint,text,jsonb,numeric)','EXECUTE'),has_function_privilege('oteryn_game_runtime','game_house_podium_load(uuid,text)','EXECUTE'),has_function_privilege('oteryn_game_control','game_house_podium_set(uuid,text,uuid,uuid,smallint,smallint,text,jsonb,numeric)','EXECUTE'),has_table_privilege('oteryn_game_runtime','game_house_podium_displays','INSERT')")
                .fetch_one(&h.pool).await?;
            assert_eq!(privileges, (false, false, false, false));
            // Runtime reaches podium storage only through the fenced functions.
            let fenced: (bool, bool, bool, bool, bool, bool) = sqlx::query_as(
                "SELECT has_function_privilege('oteryn_game_runtime','game_house_podium_set_fenced(uuid,uuid,numeric,numeric,numeric,uuid,numeric,uuid,text,uuid,smallint,text,jsonb,numeric)','EXECUTE'),has_function_privilege('oteryn_game_runtime','game_house_podium_load_fenced(uuid,uuid,numeric,numeric,numeric,uuid,numeric,uuid,text)','EXECUTE'),has_function_privilege('oteryn_game_runtime','game_house_podium_kind_fenced(uuid,uuid,numeric,numeric,numeric,uuid,numeric,uuid,text,uuid)','EXECUTE'),has_function_privilege('oteryn_game_runtime','game_house_podium_fence(uuid,uuid,numeric,numeric,numeric,uuid,numeric,uuid,text)','EXECUTE'),has_function_privilege('oteryn_game_control','game_house_podium_set_fenced(uuid,uuid,numeric,numeric,numeric,uuid,numeric,uuid,text,uuid,smallint,text,jsonb,numeric)','EXECUTE'),has_function_privilege('public','game_house_podium_load_fenced(uuid,uuid,numeric,numeric,numeric,uuid,numeric,uuid,text)','EXECUTE')")
                .fetch_one(&h.pool).await?;
            assert_eq!(fenced, (true, true, true, false, false, false));
            // An unfenced storage helper must never become a runtime bypass.
            let mut tx = h.pool.begin().await?;
            sqlx::query("SET LOCAL ROLE oteryn_game_runtime").execute(&mut *tx).await?;
            let denied = sqlx::query(SET)
                .bind(id(WORLD).as_slice()).bind(HOUSE).bind(item_id(91).as_slice())
                .bind(id(CHARACTER).as_slice()).bind(1_i16).bind(1_i16)
                .bind("oteryn:content.outfit.synthetic_test").bind("{}").bind("2")
                .execute(&mut *tx).await.expect_err("runtime has no unfenced writer");
            assert_eq!(denied.as_database_error().and_then(|e| e.code()).as_deref(), Some("42501"));
            tx.rollback().await?;
            Ok(())
        }.await;
        h.cleanup().await?;
        result
    })
}
