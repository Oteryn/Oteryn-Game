// Shared ITEM-USE-1b cases (migration 0081). Any wrapper that provides the same path-loaded crate
// root as `item_use_postgres.rs`, with the Bestiary harness (one bootstrapped Character 41 on a
// live Channel session in World 42 / Channel 43, held by node 1 at scope ownership generation 1),
// can include this file.
//
// The backpack and its stacks are seeded by the migration owner with the guards off, as other
// item cases do. Every use goes through `DurabilityRoot::commit_item_use` under the Character's
// current gameplay fence; the raw guard cases run as a login in the runtime group.

use crate::bestiary_postgres_harness::{
    CHARACTER, Harness, SESSION, TestResult, WORLD, configured_admin, debug, fence, id, runtime,
};
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::item_use::{
    ItemUseError, ItemUseFlaskPlan, ItemUseKind, ItemUseOutcome, ItemUseRequest,
};
use crate::durability::item_use_audit::{
    ITEM_USE_CAUSE_FOOD, ITEM_USE_CAUSE_POTION, ItemUseShape, decode_item_use_envelope,
};
use sqlx::PgPool;

const BACKPACK: u8 = 70;
const FOOD: u8 = 80;
const POTION: u8 = 90;
const LAST_POTION: u8 = 100;
const FLASK_STACK: u8 = 110;
const NEW_FLASK: u8 = 120;
const FULL_FLASK_STACK: u8 = 130;
const FOOD_KEY: &str = "oteryn:item.tibia.i3577";
const POTION_KEY: &str = "oteryn:item.tibia.i7618";
const FLASK_KEY: &str = "oteryn:item.tibia.i7636";
const SERVER_BUILD: &str = "oteryn-game-server-test";

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn uuid(seed: u8) -> String {
    format!("'{}'::uuid", hex(&id(seed)))
}

fn definition(key: &str) -> TypedDefinitionRef {
    TypedDefinitionRef {
        family: "Item".into(),
        production_key: key.into(),
        revision_ref: "rev-1".into(),
    }
}

/// The Bestiary harness plus the backpack fixture and a login in the runtime group.
struct Use {
    harness: Harness,
    runtime: PgPool,
}

impl Use {
    async fn create(admin: String, tag: &str) -> TestResult<Self> {
        let harness = Harness::create(admin, tag, false).await?;
        let mut script = String::new();
        script.push_str(&item(BACKPACK, "oteryn:item.tibia.i2854", 1));
        script.push_str(&format!(
            "INSERT INTO game_item_container_slots(character_id, item_instance_id, world_id, \
               placed_transaction_id) VALUES ({}, {}, {}, {});",
            uuid(CHARACTER),
            uuid(BACKPACK),
            uuid(WORLD),
            uuid(BACKPACK + 2),
        ));
        for (seed, key, quantity, ordinal) in [
            (FOOD, FOOD_KEY, 3, 1),
            (POTION, POTION_KEY, 2, 2),
            (LAST_POTION, POTION_KEY, 1, 3),
            (FLASK_STACK, FLASK_KEY, 5, 4),
            (FULL_FLASK_STACK, FLASK_KEY, 100, 5),
        ] {
            script.push_str(&item(seed, key, quantity));
            script.push_str(&entry(seed, BACKPACK, ordinal));
        }
        let mut tx = harness.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role = replica")
            .execute(&mut *tx)
            .await?;
        sqlx::raw_sql(sqlx::AssertSqlSafe(script))
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;

        let suffix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos();
        let role = format!("use_runtime_{tag}_{suffix}");
        let password = format!("{role}-secret");
        let database: String = sqlx::query_scalar("SELECT current_database()")
            .fetch_one(&harness.pool)
            .await?;
        for statement in [
            format!("CREATE ROLE {role} LOGIN PASSWORD '{password}' IN ROLE oteryn_game_runtime"),
            format!("GRANT CONNECT ON DATABASE {database} TO {role}"),
        ] {
            sqlx::query(sqlx::AssertSqlSafe(statement))
                .execute(&harness.pool)
                .await?;
        }
        let (_, address) = harness
            .database
            .url
            .split_once('@')
            .ok_or("no authority separator")?;
        let runtime = PgPool::connect(&format!("postgresql://{role}:{password}@{address}")).await?;
        Ok(Self { harness, runtime })
    }

    /// Raw statements as the runtime role, committed; the SQLSTATE of a refusal.
    async fn runtime_sql(&self, script: String) -> TestResult<Option<String>> {
        let mut tx = self.runtime.begin().await?;
        if let Err(error) = sqlx::raw_sql(sqlx::AssertSqlSafe(script))
            .execute(&mut *tx)
            .await
        {
            tx.rollback().await?;
            return Ok(Some(sqlstate(&error)));
        }
        match tx.commit().await {
            Ok(()) => Ok(None),
            Err(error) => Ok(Some(sqlstate(&error))),
        }
    }

    /// One use under the Character's fence at `revision`.
    async fn use_item(
        &self,
        revision: u64,
        request: &ItemUseRequest,
    ) -> Result<ItemUseOutcome, ItemUseError> {
        let seal = self
            .harness
            .recovery
            .seal_current()
            .map_err(|_| ItemUseError::AuthorityRejected)?;
        let authority = self
            .harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(|_| ItemUseError::AuthorityRejected)?;
        let fence = fence(revision).map_err(|_| ItemUseError::InvalidInput)?;
        self.harness
            .root
            .commit_item_use(&authority, &self.harness.node, fence, request.clone())
            .await
    }

    /// The item's quantity and lifecycle, and its entry ordinal in the backpack.
    async fn item(&self, seed: u8) -> TestResult<(i64, i16, Option<i64>)> {
        Ok(sqlx::query_as(
            "SELECT i.quantity, i.lifecycle, \
                    (SELECT e.placement_ordinal::bigint FROM game_item_container_entries e \
                      WHERE e.item_instance_id = i.item_instance_id) \
               FROM game_item_instances i WHERE i.item_instance_id = encode($1,'hex')::uuid",
        )
        .bind(id(seed).as_slice())
        .fetch_one(&self.harness.pool)
        .await?)
    }

    async fn item_exists(&self, seed: u8) -> TestResult<bool> {
        Ok(sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM game_item_instances \
                             WHERE item_instance_id = encode($1,'hex')::uuid)",
        )
        .bind(id(seed).as_slice())
        .fetch_one(&self.harness.pool)
        .await?)
    }

    /// The envelopes of the audit outbox for `event`.
    async fn envelopes(&self, event: u8) -> TestResult<Vec<Vec<u8>>> {
        Ok(sqlx::query_scalar(
            "SELECT envelope FROM game_item_audit_outbox \
              WHERE event_id = encode($1,'hex')::uuid",
        )
        .bind(id(event).as_slice())
        .fetch_all(&self.harness.pool)
        .await?)
    }

    async fn records(&self) -> TestResult<i64> {
        self.harness.count("game_item_use_records").await
    }

    async fn cleanup(self) -> TestResult {
        self.runtime.close().await;
        self.harness.cleanup().await
    }
}

/// A live stack of `quantity` of `key`. Item `seed` uses the ids `seed` (item), `seed + 1`
/// (minted) and `seed + 2` (placed).
fn item(seed: u8, key: &str, quantity: u32) -> String {
    format!(
        "INSERT INTO game_item_instances(item_instance_id, world_id, definition_family, \
           definition_production_key, definition_revision_ref, quantity, lifecycle, \
           minted_transaction_id) \
         VALUES ({}, {}, 'Item', '{key}', 'rev-1', {quantity}, 1, {});",
        uuid(seed),
        uuid(WORLD),
        uuid(seed + 1),
    )
}

fn entry(seed: u8, parent: u8, ordinal: u64) -> String {
    format!(
        "INSERT INTO game_item_container_entries(item_instance_id, world_id, character_id, \
           parent_item_instance_id, placement_ordinal, placed_transaction_id) \
         VALUES ({}, {}, {}, {}, {ordinal}, {});",
        uuid(seed),
        uuid(WORLD),
        uuid(CHARACTER),
        uuid(parent),
        uuid(seed + 2),
    )
}

/// A use of `source` (quantity `before`, key `key`) by command `command`, TransactionId
/// `id(tx)` and event id `id(tx + 1)`.
fn request(
    kind: ItemUseKind,
    command: u64,
    tx: u8,
    source: u8,
    key: &str,
    before: u32,
    flask: ItemUseFlaskPlan,
) -> ItemUseRequest {
    ItemUseRequest {
        kind,
        command_id: command,
        content_revision: "content-1".into(),
        ruleset_revision: "ruleset-1".into(),
        sim_revision: "sim-1".into(),
        transaction_id: id(tx),
        event_id: id(tx + 1),
        occurred_at_unix_ms: 1_790_000_000_000,
        server_build_id: SERVER_BUILD.into(),
        source_item_instance_id: id(source),
        source_definition: definition(key),
        source_quantity_before: before,
        flask,
    }
}

fn food(command: u64, tx: u8, before: u32) -> ItemUseRequest {
    request(
        ItemUseKind::Food,
        command,
        tx,
        FOOD,
        FOOD_KEY,
        before,
        ItemUseFlaskPlan::None,
    )
}

fn potion(
    command: u64,
    tx: u8,
    source: u8,
    before: u32,
    flask: ItemUseFlaskPlan,
) -> ItemUseRequest {
    request(
        ItemUseKind::Potion,
        command,
        tx,
        source,
        POTION_KEY,
        before,
        flask,
    )
}

fn flask_stack(seed: u8, before: u32) -> ItemUseFlaskPlan {
    ItemUseFlaskPlan::Stack {
        item_instance_id: id(seed),
        definition: definition(FLASK_KEY),
        quantity_before: before,
    }
}

fn flask_new(ordinal: u64) -> ItemUseFlaskPlan {
    ItemUseFlaskPlan::New {
        item_instance_id: id(NEW_FLASK),
        definition: definition(FLASK_KEY),
        placement_ordinal: ordinal,
    }
}

fn sqlstate(error: &sqlx::Error) -> String {
    error
        .as_database_error()
        .and_then(|error| error.code())
        .map_or_else(|| format!("{error}"), |code| code.into_owned())
}

fn run<F>(body: F) -> TestResult
where
    F: AsyncFnOnce(String) -> TestResult,
{
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(body(admin))
}

#[test]
fn food_burns_one_unit_keeps_the_stack_and_replays_without_burning_twice() -> TestResult {
    run(async |admin| {
        let used = Use::create(admin, "food").await?;
        let first = food(7, 150, 3);
        assert_eq!(
            used.use_item(1, &first).await.map_err(debug)?,
            ItemUseOutcome::Written {
                shape: ItemUseShape::Burn
            }
        );
        assert_eq!(used.item(FOOD).await?, (2, 1, Some(1)));
        // An item-only transaction never advances the CharacterRevision.
        assert_eq!(used.harness.root_revision().await?, "1");
        assert_eq!(used.records().await?, 1);

        let envelopes = used.envelopes(151).await?;
        let [envelope] = envelopes.as_slice() else {
            return Err("expected exactly one audit event".into());
        };
        let (_, decoded) = decode_item_use_envelope(envelope).map_err(debug)?;
        let cause = decoded.cause.ok_or("no cause")?;
        assert_eq!(cause.kind, ITEM_USE_CAUSE_FOOD);
        let command = cause.command_ref.ok_or("no command")?;
        assert_eq!(command.command_id, 7);
        assert_eq!(command.game_session_id, id(SESSION).to_vec());
        assert_eq!(decoded.expected_session_generation, 1);
        assert_eq!(decoded.source_before.ok_or("no before")?.quantity, 3);
        assert_eq!(decoded.source_after.ok_or("no after")?.quantity, 2);

        // The exact replay answers from the record, even under a stale fence, and burns nothing.
        assert_eq!(
            used.use_item(9, &first).await.map_err(debug)?,
            ItemUseOutcome::AlreadyCommitted {
                shape: ItemUseShape::Burn
            }
        );
        assert_eq!(used.item(FOOD).await?, (2, 1, Some(1)));
        // The same CommandRef with another request, or another command with the same
        // TransactionId, conflicts and writes nothing.
        assert!(matches!(
            used.use_item(1, &food(7, 152, 2)).await,
            Err(ItemUseError::ConflictingCommand)
        ));
        assert!(matches!(
            used.use_item(1, &food(8, 150, 2)).await,
            Err(ItemUseError::ConflictingCommand)
        ));
        // The same CommandRef and fields under another EventId is not the committed event.
        let mut other_event = food(7, 150, 3);
        other_event.event_id = id(199);
        assert!(matches!(
            used.use_item(1, &other_event).await,
            Err(ItemUseError::ConflictingCommand)
        ));
        // Revisions that are not the fenced root's interpretation are stale, writing nothing.
        let mut stale = food(8, 152, 2);
        stale.content_revision = "content-0".into();
        assert!(matches!(
            used.use_item(1, &stale).await,
            Err(ItemUseError::StaleState)
        ));
        let mut stale = food(8, 152, 2);
        stale.ruleset_revision = "rules-0".into();
        assert!(matches!(
            used.use_item(1, &stale).await,
            Err(ItemUseError::StaleState)
        ));
        assert_eq!(used.item(FOOD).await?, (2, 1, Some(1)));
        assert_eq!(used.records().await?, 1);

        // The next command burns the next unit.
        assert_eq!(
            used.use_item(1, &food(8, 152, 2)).await.map_err(debug)?,
            ItemUseOutcome::Written {
                shape: ItemUseShape::Burn
            }
        );
        assert_eq!(used.item(FOOD).await?, (1, 1, Some(1)));
        used.cleanup().await
    })
}

#[test]
fn the_last_unit_retires_the_stack_and_removes_its_entry() -> TestResult {
    run(async |admin| {
        let used = Use::create(admin, "last").await?;
        assert_eq!(
            used.use_item(1, &potion(1, 150, LAST_POTION, 1, ItemUseFlaskPlan::None))
                .await
                .map_err(debug)?,
            ItemUseOutcome::Written {
                shape: ItemUseShape::Burn
            }
        );
        assert_eq!(used.item(LAST_POTION).await?, (0, 2, None));
        let envelopes = used.envelopes(151).await?;
        let [envelope] = envelopes.as_slice() else {
            return Err("expected exactly one audit event".into());
        };
        let (_, decoded) = decode_item_use_envelope(envelope).map_err(debug)?;
        assert_eq!(decoded.cause.ok_or("no cause")?.kind, ITEM_USE_CAUSE_POTION);
        used.cleanup().await
    })
}

#[test]
fn a_flask_unit_grows_a_compatible_stack_of_the_backpack() -> TestResult {
    run(async |admin| {
        let used = Use::create(admin, "stack").await?;
        assert_eq!(
            used.use_item(1, &potion(1, 150, POTION, 2, flask_stack(FLASK_STACK, 5)))
                .await
                .map_err(debug)?,
            ItemUseOutcome::Written {
                shape: ItemUseShape::FlaskStack
            }
        );
        assert_eq!(used.item(POTION).await?, (1, 1, Some(2)));
        assert_eq!(used.item(FLASK_STACK).await?, (6, 1, Some(4)));
        let envelopes = used.envelopes(151).await?;
        let [envelope] = envelopes.as_slice() else {
            return Err("expected exactly one audit event".into());
        };
        let (_, decoded) = decode_item_use_envelope(envelope).map_err(debug)?;
        let flask = decoded.flask.ok_or("no flask")?;
        assert_eq!(flask.before.ok_or("no flask before")?.quantity, 5);
        assert_eq!(flask.after.ok_or("no flask after")?.quantity, 6);
        used.cleanup().await
    })
}

#[test]
fn a_flask_unit_becomes_a_fresh_item_in_a_new_entry_and_nothing_is_minted_twice() -> TestResult {
    run(async |admin| {
        let used = Use::create(admin, "fresh").await?;
        assert!(!used.item_exists(NEW_FLASK).await?);
        assert_eq!(
            used.use_item(1, &potion(1, 150, LAST_POTION, 1, flask_new(6)))
                .await
                .map_err(debug)?,
            ItemUseOutcome::Written {
                shape: ItemUseShape::FlaskNew
            }
        );
        assert_eq!(used.item(LAST_POTION).await?, (0, 2, None));
        assert_eq!(used.item(NEW_FLASK).await?, (1, 1, Some(6)));
        // The unit is conserved: one potion unit left, one flask unit arrived.
        assert_eq!(
            used.use_item(1, &potion(1, 150, LAST_POTION, 1, flask_new(6)))
                .await
                .map_err(debug)?,
            ItemUseOutcome::AlreadyCommitted {
                shape: ItemUseShape::FlaskNew
            }
        );
        assert_eq!(used.item(NEW_FLASK).await?, (1, 1, Some(6)));
        assert_eq!(used.records().await?, 1);
        used.cleanup().await
    })
}

#[test]
fn a_missing_flask_is_recorded_and_the_unit_still_burns() -> TestResult {
    run(async |admin| {
        let used = Use::create(admin, "missing").await?;
        assert_eq!(
            used.use_item(1, &potion(1, 150, POTION, 2, ItemUseFlaskPlan::Missing))
                .await
                .map_err(debug)?,
            ItemUseOutcome::Written {
                shape: ItemUseShape::BurnFlaskMissing
            }
        );
        assert_eq!(used.item(POTION).await?, (1, 1, Some(2)));
        let envelopes = used.envelopes(151).await?;
        let [envelope] = envelopes.as_slice() else {
            return Err("expected exactly one audit event".into());
        };
        let (_, decoded) = decode_item_use_envelope(envelope).map_err(debug)?;
        assert!(decoded.flask_missing);
        assert!(decoded.flask.is_none());
        used.cleanup().await
    })
}

#[test]
fn a_moved_plan_row_answers_stale_state_and_writes_nothing() -> TestResult {
    run(async |admin| {
        let used = Use::create(admin, "stale").await?;
        // The used stack is not at the planned quantity.
        assert!(matches!(
            used.use_item(1, &food(1, 150, 2)).await,
            Err(ItemUseError::StaleState)
        ));
        // The flask stack is not at the planned quantity.
        assert!(matches!(
            used.use_item(1, &potion(2, 152, POTION, 2, flask_stack(FLASK_STACK, 4)))
                .await,
            Err(ItemUseError::StaleState)
        ));
        // A full flask stack cannot be planned.
        assert!(matches!(
            used.use_item(
                1,
                &potion(3, 154, POTION, 2, flask_stack(FULL_FLASK_STACK, 100))
            )
            .await,
            Err(ItemUseError::InvalidInput)
        ));
        // A fresh flask below the highest ordinal, and one that already exists.
        assert!(matches!(
            used.use_item(1, &potion(4, 156, LAST_POTION, 1, flask_new(5)))
                .await,
            Err(ItemUseError::StaleState)
        ));
        assert!(matches!(
            used.use_item(1, &potion(5, 158, POTION, 2, flask_stack(NEW_FLASK, 1)))
                .await,
            Err(ItemUseError::StaleState)
        ));
        // A stack that is not in the backpack.
        assert!(matches!(
            used.use_item(
                1,
                &request(
                    ItemUseKind::Food,
                    6,
                    160,
                    NEW_FLASK,
                    FOOD_KEY,
                    1,
                    ItemUseFlaskPlan::None
                )
            )
            .await,
            Err(ItemUseError::StaleState)
        ));
        assert_eq!(used.item(FOOD).await?, (3, 1, Some(1)));
        assert_eq!(used.item(POTION).await?, (2, 1, Some(2)));
        assert_eq!(used.item(FLASK_STACK).await?, (5, 1, Some(4)));
        assert_eq!(used.records().await?, 0);
        assert_eq!(used.harness.count("game_item_audit_outbox").await?, 0);
        used.cleanup().await
    })
}

#[test]
fn a_stale_fence_writes_nothing() -> TestResult {
    run(async |admin| {
        let used = Use::create(admin, "fence").await?;
        assert!(matches!(
            used.use_item(2, &food(1, 150, 3)).await,
            Err(ItemUseError::CharacterRevisionMismatch)
        ));
        assert_eq!(used.item(FOOD).await?, (3, 1, Some(1)));
        assert_eq!(used.records().await?, 0);
        used.cleanup().await
    })
}

#[test]
fn the_runtime_role_cannot_change_items_without_a_use_record() -> TestResult {
    run(async |admin| {
        let used = Use::create(admin, "raw").await?;
        // A quantity change and a retire of a stack, with no record.
        assert_eq!(
            used.runtime_sql(format!(
                "UPDATE game_item_instances SET quantity = 2, last_transaction_id = {} \
                  WHERE item_instance_id = {};",
                uuid(150),
                uuid(FOOD)
            ))
            .await?
            .as_deref(),
            Some("23514")
        );
        assert_eq!(
            used.runtime_sql(format!(
                "DELETE FROM game_item_container_entries WHERE item_instance_id = {};",
                uuid(FOOD)
            ))
            .await?
            .as_deref(),
            Some("23514")
        );
        // A fresh flask item with no record.
        assert_eq!(
            used.runtime_sql(format!(
                "INSERT INTO game_item_instances(item_instance_id, world_id, definition_family, \
                   definition_production_key, definition_revision_ref, quantity, lifecycle, \
                   minted_transaction_id) \
                 VALUES ({}, {}, 'Item', '{FLASK_KEY}', 'rev-1', 1, 1, {});",
                uuid(NEW_FLASK),
                uuid(WORLD),
                uuid(150)
            ))
            .await?
            .as_deref(),
            Some("23514")
        );
        // A record with no item change, no entry change and no audit event.
        assert_eq!(
            used.runtime_sql(format!(
                "INSERT INTO game_item_use_records (transaction_id, event_id, cause_kind, \
                   game_session_id, command_id, content_revision, ruleset_revision, \
                   sim_revision, request_binding, character_id, world_id, channel_id, \
                   runtime_scope_ownership_generation, expected_session_generation, shape, \
                   source_item_instance_id, source_definition_family, \
                   source_definition_production_key, source_definition_revision_ref, \
                   source_parent_item_instance_id, source_placement_ordinal, \
                   source_quantity_before, source_quantity_after, occurred_at, \
                   envelope_sha256) \
                 VALUES ({}, {}, 1, {}, 1, 'c', 'r', 's', '\\x{}'::bytea, {}, {}, {}, 1, 1, 1, \
                   {}, 'Item', '{FOOD_KEY}', 'rev-1', {}, 1, 3, 2, 1790000000000, \
                   '\\x{}'::bytea);",
                uuid(150),
                uuid(151),
                uuid(SESSION),
                "00".repeat(32),
                uuid(CHARACTER),
                uuid(WORLD),
                uuid(43),
                uuid(FOOD),
                uuid(BACKPACK),
                "00".repeat(32),
            ))
            .await?
            .as_deref(),
            Some("23514")
        );
        assert_eq!(used.item(FOOD).await?, (3, 1, Some(1)));
        assert_eq!(used.records().await?, 0);
        used.cleanup().await
    })
}
