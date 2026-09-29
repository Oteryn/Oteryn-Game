// Shared D3-4 DUR-03 corpse-container TRANSFER cases (DUR-03 §39.4 "Pickup
// source"; decisions D133/D134): a `Container(parent = corpse)` entry as the
// TRANSFER source, gated by the 10 s top-damage exclusivity window judged by
// the database clock, and the corpse ItemInstance itself never a source. Both
// wrappers provide the same path-loaded crate root; the PostgreSQL bootstrap,
// the backpack fixture and the corpse MINT are the real B3-1 / D3-1 paths, and
// the corpse's loot entries are forged directly against the schema (no Rust
// loot-into-corpse caller exists before D3-2/D3-6), exactly as D3-1's own
// cases do.

use crate::domain::CharacterId;
use crate::durability::DurabilityRoot;
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::durability::item_mint::{
    CORPSE_MATERIALIZATION_PURPOSE_KEY, GroundPlacement, ItemMintCause, ItemMintOutcome,
    ItemMintRequest, TypedDefinitionRef,
};
use crate::durability::item_transfer::{
    CurrentCharacterItemFence, ItemDefinitionFacts, ItemStackClass, ItemTransferError,
    ItemTransferOutcome, ItemTransferRefusal, ItemTransferRequest, TransferShape,
};
use crate::foundation::{
    ChannelId, CommandId, CommandRef, ConnectionGeneration, GameSessionId,
    ScopeOwnershipGeneration, WorldId,
};
use crate::item_transfer_postgres_cases::{
    BACKPACK, CHANNEL, CHARACTER, Harness, SESSION, TestResult, WORLD, backpack_facts,
    configured_admin, debug, fence, id, join_two, refused, runtime, scope, to_backpack, to_slot,
    uuid_text,
};
use std::time::Duration;

/// A top-damage identity that is not any fenced Character of these cases.
pub(crate) const OTHER_TOP: u8 = 150;
const SECOND_ACCOUNT: u8 = 60;
const SECOND_CHARACTER: u8 = 61;
const SECOND_SESSION: u8 = 62;
/// The death's runtime-scope ownership generation of every corpse below.
const GENERATION: u64 = 1;

fn typed(family: &str, key: &str, revision: &str) -> TypedDefinitionRef {
    TypedDefinitionRef {
        family: family.into(),
        production_key: key.into(),
        revision_ref: revision.into(),
    }
}

fn corpse_facts() -> ItemDefinitionFacts {
    ItemDefinitionFacts {
        definition: typed("ItemType", "fixture:corpse.rat", "corpse-r1"),
        stack: ItemStackClass::NonStackable,
        container_capacity: None,
        container_slot_equip_pattern: false,
    }
}

/// The loot entry definition `put_loot` forges.
fn loot_facts() -> ItemDefinitionFacts {
    ItemDefinitionFacts {
        definition: typed("ItemType", "fixture:corpse-loot", "rev-1"),
        stack: ItemStackClass::NonStackable,
        container_capacity: None,
        container_slot_equip_pattern: false,
    }
}

fn command_of(session: u8, value: u64) -> TestResult<CommandRef> {
    Ok(CommandRef::new(
        GameSessionId::decode(&id(session)).map_err(debug)?,
        CommandId::new(value).map_err(debug)?,
    ))
}

fn fence_of(character: u8, session: u8) -> TestResult<CurrentCharacterItemFence> {
    Ok(CurrentCharacterItemFence {
        character_id: CharacterId::from_bytes(id(character)).map_err(debug)?,
        game_session_id: GameSessionId::decode(&id(session)).map_err(debug)?,
        connection_generation: ConnectionGeneration::new(1).map_err(debug)?,
        character_lease_generation: 1,
        runtime_scope: scope()?,
        scope_ownership_generation: ScopeOwnershipGeneration::new(GENERATION).map_err(debug)?,
    })
}

/// The second Character's bootstrap command binding (source revision 2).
fn second_bootstrap_binding() -> Vec<u8> {
    let mut binding = vec![1];
    binding.extend_from_slice(&id(63));
    binding.extend_from_slice(&2_i64.to_be_bytes());
    binding.extend_from_slice(&id(64));
    binding.extend_from_slice(&id(SECOND_ACCOUNT));
    binding.extend_from_slice(&id(WORLD));
    binding.extend_from_slice(&1_i64.to_be_bytes());
    binding.extend_from_slice(&120_i64.to_be_bytes());
    for value in ["profile-1", "ruleset-1", "content-1", "starter-1"] {
        binding.extend_from_slice(&u16::try_from(value.len()).expect("length").to_be_bytes());
        binding.extend_from_slice(value.as_bytes());
    }
    binding
}

/// A second player in the same World and Channel: Character 61 (account 60)
/// with an active GameSession 62, its admission guards and nothing else; the
/// runtime-scope assignment and readiness are the harness's, shared.
async fn seed_second_character(pool: &sqlx::PgPool) -> TestResult {
    sqlx::query("INSERT INTO game_character_account_guards VALUES (encode($1,'hex')::uuid)")
        .bind(id(SECOND_ACCOUNT).as_slice())
        .execute(pool)
        .await?;
    sqlx::query(
        "INSERT INTO game_character_roots VALUES \
         (encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,\
          1,1,'profile-1','ruleset-1','content-1','starter-1')",
    )
    .bind(id(SECOND_CHARACTER).as_slice())
    .bind(id(SECOND_ACCOUNT).as_slice())
    .bind(id(WORLD).as_slice())
    .execute(pool)
    .await?;
    // `open_character_authority` verifies Character integrity: every root needs
    // its bootstrap receipt, and the single intent floor must name the receipt
    // with the highest source revision (here 2, advancing the harness's 1).
    let binding = second_bootstrap_binding();
    sqlx::query(
        "INSERT INTO game_character_operation_receipts(\
           operation_id,command_binding,account_id,character_id,world_id,character_revision,\
           event_id,occurred_at,server_build_id,transaction_id,issuer_decision_id,\
           intent_source_revision,issued_at_source,expires_at_source) \
         VALUES (encode($1,'hex')::uuid,$2,encode($3,'hex')::uuid,encode($4,'hex')::uuid,\
           encode($5,'hex')::uuid,1,encode($6,'hex')::uuid,1,'test/1',\
           encode($7,'hex')::uuid,encode($8,'hex')::uuid,2,1,120)",
    )
    .bind(id(64).as_slice())
    .bind(&binding)
    .bind(id(SECOND_ACCOUNT).as_slice())
    .bind(id(SECOND_CHARACTER).as_slice())
    .bind(id(WORLD).as_slice())
    .bind(id(65).as_slice())
    .bind(id(66).as_slice())
    .bind(id(63).as_slice())
    .execute(pool)
    .await?;
    sqlx::query(
        "UPDATE game_character_bootstrap_intent_floors \
            SET source_revision = 2, issuer_decision_id = encode($1,'hex')::uuid, \
                intent_binding = $2 \
          WHERE issuer_scope = 1",
    )
    .bind(id(63).as_slice())
    .bind(&binding)
    .execute(pool)
    .await?;
    sqlx::query(
        "INSERT INTO game_durability_reconnect_sessions(\
           game_session_id,account_id,character_id,world_id,runtime_scope_kind,\
           runtime_scope_world_id,runtime_scope_channel_id,control_loss_epoch,\
           original_grace_deadline,predecessor_generation,character_lease_generation,\
           scope_ownership_generation,current_generation,current_transport_ref,session_state) \
         VALUES (encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,\
           encode($4,'hex')::uuid,1,encode($4,'hex')::uuid,encode($5,'hex')::uuid,\
           1,999999,1,1,1,1,$6,1)",
    )
    .bind(id(SECOND_SESSION).as_slice())
    .bind(id(SECOND_ACCOUNT).as_slice())
    .bind(id(SECOND_CHARACTER).as_slice())
    .bind(id(WORLD).as_slice())
    .bind(id(CHANNEL).as_slice())
    .bind([10_u8; 16].as_slice())
    .execute(pool)
    .await?;
    sqlx::query(
        "INSERT INTO game_durability_admission_account_guards VALUES \
         (encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,\
          1,'test',1,'account-current',1,0,'{}')",
    )
    .bind(id(SECOND_ACCOUNT).as_slice())
    .bind(id(SECOND_CHARACTER).as_slice())
    .bind(id(SECOND_SESSION).as_slice())
    .execute(pool)
    .await?;
    sqlx::query(
        "INSERT INTO game_durability_admission_character_guards VALUES \
         (encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,\
          true,1,encode($4,'hex')::uuid,1,'test',1,'character-current',1,0,'{}')",
    )
    .bind(id(SECOND_CHARACTER).as_slice())
    .bind(id(SECOND_ACCOUNT).as_slice())
    .bind(id(WORLD).as_slice())
    .bind(id(SECOND_SESSION).as_slice())
    .execute(pool)
    .await?;
    Ok(())
}

/// One corpse through the real D3-1 MINT path, its top-damage Character fixed
/// at materialization. `actor` is the death's actor slot (unique per corpse).
pub(crate) async fn mint_corpse(
    harness: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    actor: u32,
    top_damage: [u8; 16],
) -> TestResult<[u8; 16]> {
    let request = ItemMintRequest {
        cause: ItemMintCause::for_test(
            WorldId::decode(&id(WORLD)).map_err(debug)?,
            ChannelId::decode(&id(CHANNEL)).map_err(debug)?,
            ScopeOwnershipGeneration::new(GENERATION).map_err(debug)?,
            actor,
            1,
            typed("LootTable", "fixture:loot.alpha", "loot-r1"),
            CORPSE_MATERIALIZATION_PURPOSE_KEY.into(),
            0,
        ),
        item: typed("ItemType", "fixture:corpse.rat", "corpse-r1"),
        quantity: 1,
        ground: GroundPlacement {
            spatial_position: vec![9, 9, 0],
            corpse_ref: id(3).to_vec(),
            map_revision: "map-1".into(),
            content_revision: "content-1".into(),
            native_room_placement_context: id(6).to_vec(),
        },
        content_revision: "content-1".into(),
        ruleset_revision: "ruleset-1".into(),
        sim_revision: "sim-1".into(),
    };
    let mut candidate = harness
        .root
        .freeze_item_mint(authority, &harness.node, request)
        .await
        .map_err(debug)?;
    match harness
        .root
        .commit_corpse_mint(authority, &harness.node, &mut candidate, top_damage)
        .await
        .map_err(debug)?
    {
        ItemMintOutcome::Committed(result) => Ok(result.item_instance_id),
        other => Err(format!("unexpected corpse MINT outcome {other:?}").into()),
    }
}

/// Raw-SQL statements forging one loot entry's whole logical MINT into
/// `corpse` (reservation, receipt, audit event, item and its corpse container
/// entry), the same forged shape D3-1's own cases use. Occupies ids
/// `seed..seed + 3`.
fn forge_loot_entry(
    definition: &TypedDefinitionRef,
    actor: u32,
    corpse: [u8; 16],
    ordinal: u64,
    seed: u8,
) -> Vec<String> {
    let (family, key, revision) = (
        &definition.family,
        &definition.production_key,
        &definition.revision_ref,
    );
    let world = uuid_text(id(WORLD));
    let channel = uuid_text(id(CHANNEL));
    let corpse = uuid_text(corpse);
    let item = uuid_text(id(seed));
    let tx = uuid_text(id(seed + 1));
    let ev = uuid_text(id(seed + 2));
    let node = uuid_text(id(seed + 3));
    vec![
        format!(
            "INSERT INTO game_item_mint_reservations \
               (death_world_id, death_channel_id, death_scope_ownership_generation, \
                death_actor_local_id, death_actor_local_generation, loot_table_family, \
                loot_table_production_key, loot_table_revision_ref, loot_purpose_key, \
                draw_ordinal, intent_binding, transaction_id, event_id, item_instance_id, \
                occurred_at, envelope, fence_scope_ownership_generation, fence_holder_node_id, \
                fence_holder_registration_revision, work_units_used, reserved_at) \
             VALUES \
               ('{world}', '{channel}', {GENERATION}, {actor}, 1, \
                'LootTable', 'fixture:loot.alpha', 'loot-r1', 'fixture:purpose.drop', {ordinal}, \
                decode(repeat('cd',33),'hex'), '{tx}', '{ev}', '{item}', 1000, \
                decode(repeat('ab',16),'hex'), {GENERATION}, '{node}', 0, 0, 900)"
        ),
        format!(
            "INSERT INTO game_item_instances \
               (item_instance_id, world_id, definition_family, definition_production_key, \
                definition_revision_ref, quantity, lifecycle, minted_transaction_id) \
             VALUES ('{item}', '{world}', '{family}', '{key}', '{revision}', 1, 1, '{tx}')"
        ),
        format!(
            "INSERT INTO game_item_mint_receipts \
               (death_world_id, death_channel_id, death_scope_ownership_generation, \
                death_actor_local_id, death_actor_local_generation, loot_table_family, \
                loot_table_production_key, loot_table_revision_ref, loot_purpose_key, \
                draw_ordinal, intent_binding, transaction_id, event_id, item_instance_id, \
                occurred_at, envelope_sha256, committed_at, destination_parent_item_instance_id, \
                destination_ordinal) \
             VALUES \
               ('{world}', '{channel}', {GENERATION}, {actor}, 1, \
                'LootTable', 'fixture:loot.alpha', 'loot-r1', 'fixture:purpose.drop', {ordinal}, \
                decode(repeat('cd',33),'hex'), '{tx}', '{ev}', '{item}', 1000, \
                sha256(decode(repeat('ab',16),'hex')), 1000, '{corpse}', {ordinal})"
        ),
        format!(
            "INSERT INTO game_item_audit_outbox \
               (event_id, transaction_id, transaction_ordinal, transaction_count, \
                event_type_id, schema_revision, retention_profile_id, item_instance_id, \
                occurred_at, expires_at, envelope, envelope_sha256, publication_state) \
             VALUES \
               ('{ev}', '{tx}', 1, 1, 2, 1, 'DUR03_ONE_ITEM_DURABLE_AUDIT_RETENTION_V1', '{item}', \
                1000, 7776001000, decode(repeat('ab',16),'hex'), \
                sha256(decode(repeat('ab',16),'hex')), 1)"
        ),
        format!(
            "INSERT INTO game_item_corpse_container_entries \
               (item_instance_id, world_id, parent_item_instance_id, placement_ordinal, \
                placed_transaction_id) \
             VALUES ('{item}', '{world}', '{corpse}', {ordinal}, '{tx}')"
        ),
    ]
}

/// Commit `statements` as one physical transaction.
async fn run_statements(pool: &sqlx::PgPool, statements: Vec<String>) -> TestResult {
    let mut tx = pool.begin().await?;
    for statement in &statements {
        sqlx::query(sqlx::AssertSqlSafe(statement.clone()))
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    Ok(())
}

async fn put_loot(
    harness: &Harness,
    corpse: [u8; 16],
    actor: u32,
    ordinal: u64,
    seed: u8,
) -> TestResult<[u8; 16]> {
    put_loot_of(
        harness,
        &loot_facts().definition,
        corpse,
        actor,
        ordinal,
        seed,
    )
    .await
}

/// [`put_loot`] of a caller-chosen definition (D3-5: one Content can resolve).
pub(crate) async fn put_loot_of(
    harness: &Harness,
    definition: &TypedDefinitionRef,
    corpse: [u8; 16],
    actor: u32,
    ordinal: u64,
    seed: u8,
) -> TestResult<[u8; 16]> {
    run_statements(
        &harness.pool,
        forge_loot_entry(definition, actor, corpse, ordinal, seed),
    )
    .await?;
    Ok(id(seed))
}

/// Move a corpse's `materialized_at` to `age_ms` before the database's own
/// clock now, outside the runtime guards (test-only time travel; the runtime
/// itself can never write the column).
pub(crate) async fn set_materialized_ago(
    harness: &Harness,
    corpse: [u8; 16],
    age_ms: i64,
) -> TestResult {
    harness
        .tamper(&format!(
            "UPDATE game_item_mint_receipts \
                SET materialized_at = floor(extract(epoch FROM clock_timestamp())*1000)::bigint \
                                      - {age_ms} \
              WHERE item_instance_id = '{}' AND loot_purpose_key = 'CORPSE_MATERIALIZATION'",
            uuid_text(corpse)
        ))
        .await
}

/// Equip a fresh backpack for `fence`'s Character through the real TRANSFER.
async fn equip_backpack(
    harness: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    fence: CurrentCharacterItemFence,
    session: u8,
) -> TestResult<[u8; 16]> {
    let backpack = harness.mint(authority, BACKPACK, 1).await?;
    match harness
        .transfer(
            authority,
            fence,
            to_slot(command_of(session, 1)?, backpack, backpack_facts()),
        )
        .await
        .map_err(debug)?
    {
        ItemTransferOutcome::Committed(_) => Ok(backpack),
        other => Err(format!("unexpected equip outcome {other:?}").into()),
    }
}

fn take(session: u8, command: u64, item: [u8; 16]) -> TestResult<ItemTransferRequest> {
    Ok(to_backpack(
        command_of(session, command)?,
        item,
        loot_facts(),
    ))
}

async fn locations(harness: &Harness, item: [u8; 16]) -> TestResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT (SELECT count(*) FROM game_item_ground_locations WHERE item_instance_id = \
                   encode($1,'hex')::uuid) \
              + (SELECT count(*) FROM game_item_container_slots WHERE item_instance_id = \
                   encode($1,'hex')::uuid) \
              + (SELECT count(*) FROM game_item_container_entries WHERE item_instance_id = \
                   encode($1,'hex')::uuid) \
              + (SELECT count(*) FROM game_item_corpse_container_entries WHERE item_instance_id = \
                   encode($1,'hex')::uuid)",
    )
    .bind(item.as_slice())
    .fetch_one(&harness.pool)
    .await?)
}

async fn in_corpse(harness: &Harness, item: [u8; 16]) -> TestResult<bool> {
    Ok(sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM game_item_corpse_container_entries \
                         WHERE item_instance_id = encode($1,'hex')::uuid)",
    )
    .bind(item.as_slice())
    .fetch_one(&harness.pool)
    .await?)
}

async fn backpack_entry_of(
    harness: &Harness,
    item: [u8; 16],
) -> TestResult<Option<(String, String)>> {
    let row = sqlx::query_as::<_, (String, String)>(
        "SELECT character_id::text, parent_item_instance_id::text \
           FROM game_item_container_entries WHERE item_instance_id = encode($1,'hex')::uuid",
    )
    .bind(item.as_slice())
    .fetch_optional(&harness.pool)
    .await?;
    Ok(row)
}

fn committed_shape(outcome: ItemTransferOutcome) -> TestResult<TransferShape> {
    match outcome {
        ItemTransferOutcome::Committed(result) => Ok(result.shape),
        other => Err(format!("expected a fresh commit, got {other:?}").into()),
    }
}

/// D133: the owner is allowed inside the window, another player is refused
/// there (nothing written, the entry stays in the corpse), and after the
/// window both are allowed. A refusal is a pure freeze-time rejection, so the
/// same CommandRef succeeds unchanged once the window has closed.
#[test]
fn corpse_window_owner_first_others_after_ten_seconds() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let harness = Harness::create(admin, "corpsewindow").await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        equip_backpack(&harness, &authority, fence()?, SESSION).await?;

        let owned = mint_corpse(&harness, &authority, 1000, id(CHARACTER)).await?;
        let foreign = mint_corpse(&harness, &authority, 1001, id(OTHER_TOP)).await?;
        let owned_a = put_loot(&harness, owned, 1000, 1, 100).await?;
        let owned_b = put_loot(&harness, owned, 1000, 2, 104).await?;
        let foreign_a = put_loot(&harness, foreign, 1001, 1, 108).await?;
        let foreign_b = put_loot(&harness, foreign, 1001, 2, 112).await?;
        assert_eq!(
            harness.count("game_item_corpse_container_entries").await?,
            4
        );
        // Both windows are open with 9 s left.
        set_materialized_ago(&harness, owned, 1_000).await?;
        set_materialized_ago(&harness, foreign, 1_000).await?;

        // The top-damage Character takes an entry out inside the window.
        let shape = committed_shape(
            harness
                .transfer(&authority, fence()?, take(SESSION, 2, owned_a)?)
                .await
                .map_err(debug)?,
        )?;
        assert_eq!(shape, TransferShape::NewEntry);
        assert!(!in_corpse(&harness, owned_a).await?);
        assert_eq!(locations(&harness, owned_a).await?, 1);
        assert!(backpack_entry_of(&harness, owned_a).await?.is_some());
        assert_eq!(
            harness
                .count("game_item_corpse_entry_removal_evidence")
                .await?,
            1
        );

        // Another player is refused inside the window and nothing is written.
        let before = harness.footprint().await?;
        refused(
            harness
                .transfer(&authority, fence()?, take(SESSION, 3, foreign_a)?)
                .await,
            ItemTransferRefusal::CorpseExclusiveWindow,
        )?;
        assert_eq!(harness.footprint().await?, before);
        assert!(in_corpse(&harness, foreign_a).await?);
        assert_eq!(locations(&harness, foreign_a).await?, 1);

        // Once the window has closed, the same refused CommandRef succeeds ...
        set_materialized_ago(&harness, foreign, 11_000).await?;
        committed_shape(
            harness
                .transfer(&authority, fence()?, take(SESSION, 3, foreign_a)?)
                .await
                .map_err(debug)?,
        )?;
        assert!(!in_corpse(&harness, foreign_a).await?);
        // ... and so does the owner of the other corpse, after its window.
        set_materialized_ago(&harness, owned, 11_000).await?;
        committed_shape(
            harness
                .transfer(&authority, fence()?, take(SESSION, 4, owned_b)?)
                .await
                .map_err(debug)?,
        )?;
        assert!(!in_corpse(&harness, owned_b).await?);
        // The untouched entry stays where it was; every taken item has one place.
        assert!(in_corpse(&harness, foreign_b).await?);
        for item in [owned_a, owned_b, foreign_a, foreign_b] {
            assert_eq!(locations(&harness, item).await?, 1);
        }
        assert_eq!(
            harness.count("game_item_corpse_container_entries").await?,
            1
        );
        assert_eq!(
            harness
                .count("game_item_corpse_entry_removal_evidence")
                .await?,
            3
        );

        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

/// D134: the corpse ItemInstance is never a legal TRANSFER source, whether it
/// is empty, full or was emptied by pickups; the refusal is the Rust one, and
/// a raw-SQL forgery of such a TRANSFER (or a bare DELETE of the corpse's
/// Ground row or of a corpse entry) is rejected by the database at commit.
#[test]
fn the_corpse_itself_is_never_a_transfer_source() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let harness = Harness::create(admin, "corpseitself").await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let backpack = equip_backpack(&harness, &authority, fence()?, SESSION).await?;

        let empty = mint_corpse(&harness, &authority, 1000, id(CHARACTER)).await?;
        let full = mint_corpse(&harness, &authority, 1001, id(CHARACTER)).await?;
        let entry = put_loot(&harness, full, 1001, 1, 100).await?;
        let second = put_loot(&harness, full, 1001, 2, 104).await?;

        let before = harness.footprint().await?;
        for (n, corpse) in [(10_u64, empty), (11, full)] {
            for destination_slot in [false, true] {
                let command = command_of(SESSION, n + if destination_slot { 10 } else { 0 })?;
                let request = if destination_slot {
                    to_slot(command, corpse, corpse_facts())
                } else {
                    to_backpack(command, corpse, corpse_facts())
                };
                refused(
                    harness.transfer(&authority, fence()?, request).await,
                    ItemTransferRefusal::CorpseNotPickupable,
                )?;
            }
        }
        assert_eq!(harness.footprint().await?, before);
        assert!(harness.on_ground(empty).await?);
        assert!(harness.on_ground(full).await?);

        // Empty the corpse by real pickups: still not a source.
        for (command, item) in [(2_u64, entry), (3, second)] {
            harness
                .transfer(&authority, fence()?, take(SESSION, command, item)?)
                .await
                .map_err(debug)?;
        }
        assert_eq!(
            harness.count("game_item_corpse_container_entries").await?,
            0
        );
        let before = harness.footprint().await?;
        refused(
            harness
                .transfer(
                    &authority,
                    fence()?,
                    to_backpack(command_of(SESSION, 30)?, full, corpse_facts()),
                )
                .await,
            ItemTransferRefusal::CorpseNotPickupable,
        )?;
        assert_eq!(harness.footprint().await?, before);
        assert!(harness.on_ground(full).await?);

        // The database enforces it on its own: a forged shape-2 TRANSFER of the
        // corpse (a well-formed reservation, audit row, item change, Ground
        // DELETE, backpack entry and receipt) is rejected at commit ...
        let forged = forge_transfer(
            id(CHARACTER),
            backpack,
            empty,
            &format!(
                "DELETE FROM game_item_ground_locations WHERE item_instance_id = '{}'",
                uuid_text(empty)
            ),
            99,
            301,
            (id(240), id(241)),
        );
        assert!(run_statements(&harness.pool, forged).await.is_err());
        assert!(harness.on_ground(empty).await?);
        assert_eq!(locations(&harness, empty).await?, 1);
        // ... as is a bare DELETE of a corpse's Ground row.
        let bare = vec![format!(
            "DELETE FROM game_item_ground_locations WHERE item_instance_id = '{}'",
            uuid_text(full)
        )];
        assert!(run_statements(&harness.pool, bare).await.is_err());
        assert!(harness.on_ground(full).await?);

        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

/// Raw-SQL statements of one forged shape-2 (NewEntry) TRANSFER commit, the
/// exact sequence `apply_transfer` issues, with `removal` as the statement that
/// removes the source's custody. The reservation, receipt and audit rows are
/// otherwise well-formed, so only the guard under test can reject it.
fn forge_transfer(
    character: [u8; 16],
    backpack: [u8; 16],
    source: [u8; 16],
    removal: &str,
    ordinal: u64,
    command_id: u64,
    ids: ([u8; 16], [u8; 16]),
) -> Vec<String> {
    let character = uuid_text(character);
    let world = uuid_text(id(WORLD));
    let channel = uuid_text(id(CHANNEL));
    let source = uuid_text(source);
    let parent = uuid_text(backpack);
    let tx = uuid_text(ids.0);
    let ev = uuid_text(ids.1);
    vec![
        format!(
            "INSERT INTO game_item_transfer_reservations VALUES \
             ('{character}', {command_id}, '{character}', '{world}', '{channel}', \
               '{source}', 2, decode(repeat('ab',33),'hex'), '{tx}', '{ev}', 1000, 1, 900)"
        ),
        format!(
            "INSERT INTO game_item_audit_outbox VALUES \
             ('{ev}', '{tx}', 1, 1, 2, 1, 'DUR03_ONE_ITEM_DURABLE_AUDIT_RETENTION_V1', \
               '{source}', 1000, 7776001000, decode(repeat('ab',16),'hex'), \
               sha256(decode(repeat('ab',16),'hex')), 1, NULL)"
        ),
        format!(
            "UPDATE game_item_instances SET last_transaction_id = '{tx}' \
               WHERE item_instance_id = '{source}' AND lifecycle = 1 AND quantity = 1"
        ),
        removal.to_owned(),
        format!(
            "INSERT INTO game_item_container_entries VALUES \
             ('{source}', '{world}', '{character}', '{parent}', {ordinal}, '{tx}')"
        ),
        format!(
            "INSERT INTO game_item_transfer_receipts \
               (game_session_id, command_id, character_id, intent_binding, transaction_id, \
                event_id, shape, source_item_instance_id, source_quantity_before, \
                source_quantity_after, destination_parent_item_instance_id, \
                destination_ordinal, occurred_at, envelope_sha256, committed_at) \
             VALUES \
             ('{character}', {command_id}, '{character}', decode(repeat('ab',33),'hex'), '{tx}', \
               '{ev}', 2, '{source}', 1, 1, '{parent}', {ordinal}, 1000, \
               sha256(decode(repeat('ab',16),'hex')), 1000)"
        ),
    ]
}

fn remove_entry(item: [u8; 16]) -> String {
    format!(
        "DELETE FROM game_item_corpse_container_entries WHERE item_instance_id = '{}'",
        uuid_text(item)
    )
}

/// The database gate, independent of the Rust admission: a raw-SQL TRANSFER of
/// a corpse entry by a non-top-damage Character inside the window is rejected
/// at commit with nothing changed; the identical forgery commits once the
/// window has closed on the database clock, and inside the window for the
/// top-damage Character. A corpse entry also cannot be deleted without a
/// TRANSFER receipt.
#[test]
fn database_enforces_the_exclusive_window_and_the_removal_proof() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let harness = Harness::create(admin, "corpsedbgate").await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let backpack = equip_backpack(&harness, &authority, fence()?, SESSION).await?;

        let foreign = mint_corpse(&harness, &authority, 1000, id(OTHER_TOP)).await?;
        let owned = mint_corpse(&harness, &authority, 1001, id(CHARACTER)).await?;
        let foreign_a = put_loot(&harness, foreign, 1000, 1, 100).await?;
        let foreign_b = put_loot(&harness, foreign, 1000, 2, 104).await?;
        let owned_a = put_loot(&harness, owned, 1001, 1, 108).await?;
        set_materialized_ago(&harness, foreign, 1_000).await?;
        set_materialized_ago(&harness, owned, 1_000).await?;
        let receipts = harness.count("game_item_transfer_receipts").await?;

        // A bare DELETE of a corpse entry has no TRANSFER receipt behind it.
        assert!(
            run_statements(&harness.pool, vec![remove_entry(foreign_a)])
                .await
                .is_err()
        );
        assert!(in_corpse(&harness, foreign_a).await?);

        // Non-top-damage Character, inside the window: rejected at commit.
        let inside = forge_transfer(
            id(CHARACTER),
            backpack,
            foreign_a,
            &remove_entry(foreign_a),
            1,
            301,
            (id(240), id(241)),
        );
        assert!(run_statements(&harness.pool, inside).await.is_err());
        assert!(in_corpse(&harness, foreign_a).await?);
        assert_eq!(locations(&harness, foreign_a).await?, 1);
        assert_eq!(
            harness.count("game_item_transfer_receipts").await?,
            receipts
        );

        // The top-damage Character, inside its own window: admitted.
        let owner = forge_transfer(
            id(CHARACTER),
            backpack,
            owned_a,
            &remove_entry(owned_a),
            1,
            302,
            (id(242), id(243)),
        );
        run_statements(&harness.pool, owner).await?;
        assert!(!in_corpse(&harness, owned_a).await?);
        assert_eq!(locations(&harness, owned_a).await?, 1);

        // Once the window has closed on the database clock, the very forgery
        // that was rejected above is admitted.
        set_materialized_ago(&harness, foreign, 11_000).await?;
        let after = forge_transfer(
            id(CHARACTER),
            backpack,
            foreign_a,
            &remove_entry(foreign_a),
            2,
            303,
            (id(244), id(245)),
        );
        run_statements(&harness.pool, after).await?;
        assert!(!in_corpse(&harness, foreign_a).await?);
        assert_eq!(locations(&harness, foreign_a).await?, 1);
        assert!(in_corpse(&harness, foreign_b).await?);
        assert_eq!(
            harness.count("game_item_transfer_receipts").await?,
            receipts + 2
        );

        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

/// Replay and conflict: the same CommandRef replays its original result and
/// never transfers twice; a changed intent conflicts; a stale duplicate under a
/// new CommandRef finds the entry gone and writes nothing.
#[test]
fn corpse_pickup_replays_conflicts_and_transfers_at_most_once() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let harness = Harness::create(admin, "corpsereplay").await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        equip_backpack(&harness, &authority, fence()?, SESSION).await?;
        let corpse = mint_corpse(&harness, &authority, 1000, id(CHARACTER)).await?;
        let item = put_loot(&harness, corpse, 1000, 1, 100).await?;
        let other = put_loot(&harness, corpse, 1000, 2, 104).await?;

        let first = match harness
            .transfer(&authority, fence()?, take(SESSION, 2, item)?)
            .await
            .map_err(debug)?
        {
            ItemTransferOutcome::Committed(result) => result,
            other => return Err(format!("expected a fresh commit, got {other:?}").into()),
        };
        assert_eq!(first.shape, TransferShape::NewEntry);
        let after_first = harness.footprint().await?;

        // Replay: the original result, nothing written.
        match harness
            .transfer(&authority, fence()?, take(SESSION, 2, item)?)
            .await
            .map_err(debug)?
        {
            ItemTransferOutcome::AlreadyCommitted(result) => assert_eq!(result, first),
            other => return Err(format!("expected a replay, got {other:?}").into()),
        }
        assert_eq!(harness.footprint().await?, after_first);

        // Conflict: the same CommandRef with a changed intent (another
        // destination) is rejected, still nothing written.
        match harness
            .transfer(
                &authority,
                fence()?,
                to_slot(command_of(SESSION, 2)?, item, loot_facts()),
            )
            .await
        {
            Err(ItemTransferError::ConflictingCause) => {}
            other => return Err(format!("expected ConflictingCause, got {other:?}").into()),
        }
        assert_eq!(harness.footprint().await?, after_first);

        // Stale duplicate under a NEW CommandRef: the entry is gone.
        refused(
            harness
                .transfer(&authority, fence()?, take(SESSION, 3, item)?)
                .await,
            ItemTransferRefusal::SourceNotOnGround,
        )?;
        assert_eq!(harness.footprint().await?, after_first);
        assert_eq!(locations(&harness, item).await?, 1);
        assert!(backpack_entry_of(&harness, item).await?.is_some());
        assert!(in_corpse(&harness, other).await?);

        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

/// Two players at the window boundary: the top-damage Character and another
/// player race for the same corpse entry across two runtime roots, the other
/// player's freeze succeeding only once the window has closed on the database
/// clock. Exactly one commits, the loser finds the entry gone, and the item
/// ends with exactly one location -- never duplicated, never lost.
#[test]
fn two_players_at_the_window_boundary_transfer_the_entry_once() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let harness = Harness::create(admin, "corpseboundary").await?;
        seed_second_character(&harness.pool).await?;
        let second_root = DurabilityRoot::connect_test_runtime(&harness.database.url)?;
        assert!(second_root.maintain_ready_once().await?);
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let first_authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let second_authority = second_root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let owner_fence = fence_of(CHARACTER, SESSION)?;
        let other_fence = fence_of(SECOND_CHARACTER, SECOND_SESSION)?;
        let owner_backpack =
            equip_backpack(&harness, &first_authority, owner_fence, SESSION).await?;
        let other_backpack =
            equip_backpack(&harness, &first_authority, other_fence, SECOND_SESSION).await?;

        let corpse = mint_corpse(&harness, &first_authority, 1000, id(CHARACTER)).await?;
        let item = put_loot(&harness, corpse, 1000, 1, 100).await?;
        // The window closes about 3 s from now.
        set_materialized_ago(&harness, corpse, 7_000).await?;

        // The owner freezes inside the window; the other player is refused.
        let mut owner_candidate = harness
            .root
            .freeze_item_transfer(
                &first_authority,
                &harness.node,
                owner_fence,
                take(SESSION, 2, item)?,
            )
            .await
            .map_err(debug)?;
        refused(
            second_root
                .freeze_item_transfer(
                    &second_authority,
                    &harness.node,
                    other_fence,
                    take(SECOND_SESSION, 2, item)?,
                )
                .await,
            ItemTransferRefusal::CorpseExclusiveWindow,
        )?;
        // The other player's freeze succeeds only from the boundary onward.
        let mut other_candidate = None;
        for _ in 0..200 {
            match second_root
                .freeze_item_transfer(
                    &second_authority,
                    &harness.node,
                    other_fence,
                    take(SECOND_SESSION, 2, item)?,
                )
                .await
            {
                Ok(candidate) => {
                    other_candidate = Some(candidate);
                    break;
                }
                Err(ItemTransferError::Refused(ItemTransferRefusal::CorpseExclusiveWindow)) => {
                    tokio::time::sleep(Duration::from_millis(50)).await;
                }
                Err(other) => return Err(format!("unexpected freeze error {other:?}").into()),
            }
        }
        let mut other_candidate = other_candidate.ok_or("the window never closed")?;

        // Both are now admissible; commit them concurrently.
        let (owner_result, other_result) = join_two(
            harness.root.commit_item_transfer(
                &first_authority,
                &harness.node,
                owner_fence,
                &mut owner_candidate,
            ),
            second_root.commit_item_transfer(
                &second_authority,
                &harness.node,
                other_fence,
                &mut other_candidate,
            ),
        )
        .await;
        let owner_won = match (&owner_result, &other_result) {
            (Ok(ItemTransferOutcome::Committed(_)), Err(loser))
            | (Err(loser), Ok(ItemTransferOutcome::Committed(_))) => {
                assert!(
                    matches!(
                        loser,
                        ItemTransferError::Refused(ItemTransferRefusal::SourceNotOnGround)
                    ),
                    "the loser must find the entry gone: {loser:?}"
                );
                owner_result.is_ok()
            }
            other => return Err(format!("expected exactly one winner, got {other:?}").into()),
        };
        assert!(!in_corpse(&harness, item).await?);
        assert_eq!(locations(&harness, item).await?, 1);
        let (character, parent) = backpack_entry_of(&harness, item)
            .await?
            .ok_or("the winner's backpack entry is missing")?;
        let (expected_character, expected_backpack) = if owner_won {
            (id(CHARACTER), owner_backpack)
        } else {
            (id(SECOND_CHARACTER), other_backpack)
        };
        assert_eq!(character, uuid_text(expected_character));
        assert_eq!(parent, uuid_text(expected_backpack));
        assert_eq!(
            harness
                .count("game_item_corpse_entry_removal_evidence")
                .await?,
            1
        );

        drop(first_authority);
        drop(second_authority);
        drop(seal);
        drop(second_root);
        harness.cleanup().await
    })
}
