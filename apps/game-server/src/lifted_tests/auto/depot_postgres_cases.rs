//! DEPOT-1 storage qualification. Auto-mounted once, never in dispatch.
//! Fixtures seed native identities only; assertions run with all guards enabled.

use crate::bestiary_postgres_harness::{self as fixture, Harness, TestResult, id};
use oteryn_gameplay_durability::durability::character_depot::CharacterDepotEntry;
use oteryn_gameplay_items::depot::{DEPOT_BOXES, DEPOT_HARD_ENTRIES};
use sqlx::{Connection, Executor, PgConnection};

#[path = "depot_postgres_cases/fixture.rs"]
mod storage;
use storage::{insert, run, seed_items, sqlstate};

#[test]
fn depot_towns_boxes_and_characters_are_durable_and_isolated() -> TestResult {
    run("depot_scope", async |harness| {
        let items = seed_items(&harness.pool, 6, 100).await?;
        for (item, town, box_number) in [
            (&items[0], 0, 1),
            (&items[1], 1, 1),
            (&items[2], 0, i16::from(DEPOT_BOXES)),
            (&items[3], 0, 1),
            (&items[4], i32::from(u16::MAX), i16::from(DEPOT_BOXES)),
        ] {
            insert(&harness.pool, item, town, box_number).await?;
        }
        let rows: Vec<(i32, i16, String)> = sqlx::query_as(
            "SELECT town_id, box_number, ordinal::text \
             FROM game_item_character_depot_locations \
             WHERE character_id=encode($1,'hex')::uuid \
             ORDER BY town_id, box_number, ordinal LIMIT 6",
        )
        .bind(id(fixture::CHARACTER).as_slice())
        .fetch_all(&harness.pool)
        .await?;
        assert_eq!(
            rows,
            [
                (0, 1, "1".into()),
                (0, 1, "2".into()),
                (0, 17, "1".into()),
                (1, 1, "1".into()),
                (65535, 17, "1".into())
            ]
        );

        // No Channel column exists: any channel of the Character's World
        // addresses the same rows. A new connection reloads native identity.
        let mut reloaded = PgConnection::connect(&harness.database.url).await?;
        let (town, box_number, ordinal): (i32, i16, String) = sqlx::query_as(
            "SELECT town_id, box_number, ordinal::text \
             FROM game_item_character_depot_locations WHERE item_instance_id=$1::uuid",
        )
        .bind(&items[0])
        .fetch_one(&mut reloaded)
        .await?;
        let entry = CharacterDepotEntry {
            item: storage::item(&items[0], 100)?,
            character_id: id(fixture::CHARACTER),
            world_id: id(fixture::WORLD),
            town_id: town.try_into()?,
            box_number: box_number.try_into()?,
            ordinal: ordinal.parse()?,
        };
        assert_eq!(entry.ordinal, 1);
        assert_eq!(entry.item.quantity, 100);
        assert_eq!(entry.town_id, 0);
        assert_eq!(entry.box_number, 1);
        assert_eq!(entry.world_id, id(fixture::WORLD));
        assert_eq!(entry.character_id, id(fixture::CHARACTER));
        reloaded.close().await?;

        storage::seed_character(&harness.pool, 61, fixture::WORLD).await?;
        sqlx::query(
            "INSERT INTO game_item_character_depot_locations \
             (item_instance_id,world_id,character_id,town_id,box_number) \
             VALUES($1::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,0,1)",
        )
        .bind(&items[5])
        .bind(id(fixture::WORLD).as_slice())
        .bind(id(61).as_slice())
        .execute(&harness.pool)
        .await?;
        let own: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_item_character_depot_locations \
             WHERE character_id=encode($1,'hex')::uuid AND town_id=0 AND box_number=1",
        )
        .bind(id(61).as_slice())
        .fetch_one(&harness.pool)
        .await?;
        assert_eq!(own, 1);
        Ok(())
    })
}

#[test]
fn depot_capacity_counts_stacks_once_across_boxes_and_does_not_cap_inbox() -> TestResult {
    run("depot_capacity", async |harness| {
        let items = seed_items(&harness.pool, i64::from(DEPOT_HARD_ENTRIES) + 2, 100).await?;
        let last_slot = DEPOT_HARD_ENTRIES as usize - 1;
        sqlx::query(
            "INSERT INTO game_item_character_depot_locations \
             (item_instance_id,world_id,character_id,town_id,box_number) \
             SELECT p.item::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,0, \
                    (1 + (p.n-1) % 17)::smallint \
             FROM unnest($1::text[]) WITH ORDINALITY AS p(item,n) ORDER BY p.n",
        )
        .bind(&items[..last_slot])
        .bind(id(fixture::WORLD).as_slice())
        .bind(id(fixture::CHARACTER).as_slice())
        .execute(&harness.pool)
        .await?;
        let overflow = &items[DEPOT_HARD_ENTRIES as usize];
        let mut last = harness.pool.begin().await?;
        storage::insert_on(&mut last, &items[last_slot], 0, 1).await?;
        let mut competing = PgConnection::connect(&harness.database.url).await?;
        competing.execute("SET statement_timeout='5s'").await?;
        let pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
            .fetch_one(&mut competing)
            .await?;
        let competing_item = overflow.clone();
        let later =
            tokio::spawn(
                async move { storage::insert_on(&mut competing, &competing_item, 0, 1).await },
            );
        storage::wait_for_lock(&harness.pool, pid).await?;
        last.commit().await?;
        assert_eq!(sqlstate(later.await?)?, "23514");
        let count: i64 = sqlx::query_scalar(
            "SELECT sum(committed)::bigint FROM game_character_depot_box_counters \
             WHERE character_id=encode($1,'hex')::uuid AND town_id=0",
        )
        .bind(id(fixture::CHARACTER).as_slice())
        .fetch_one(&harness.pool)
        .await?;
        assert_eq!(count, i64::from(DEPOT_HARD_ENTRIES));
        insert(&harness.pool, overflow, 1, 1).await?;

        // The existing sink is the MAIL / MARKET delivery target. A full
        // town depot neither consumes Inbox capacity nor creates another Inbox.
        sqlx::query("INSERT INTO game_character_inbox_cause_kinds VALUES ('DEPOT_TEST_DELIVERY')")
            .execute(&harness.pool)
            .await?;
        let delivered: (String, String) = sqlx::query_as(
            "SELECT character_id::text,ordinal::text FROM game_character_inbox_deliver(\
             $1::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,'DEPOT_TEST_DELIVERY','test')",
        )
        .bind(&items[DEPOT_HARD_ENTRIES as usize + 1])
        .bind(id(fixture::CHARACTER).as_slice())
        .bind(id(fixture::WORLD).as_slice())
        .fetch_one(&harness.pool)
        .await?;
        assert_eq!(delivered.1, "1");
        let inbox = crate::durability::character_inbox::read_character_inbox_counter(
            &mut PgConnection::connect(&harness.database.url).await?,
            &id(fixture::CHARACTER),
        )
        .await?
        .ok_or("missing Inbox counter")?;
        assert_eq!(inbox.committed, 1);
        Ok(())
    })
}

#[test]
fn depot_rejects_bad_scope_boxes_ordinals_and_duplicate_custody_atomically() -> TestResult {
    run("depot_reject", async |harness| {
        let items = seed_items(&harness.pool, 2, 1).await?;
        for (town, box_number) in [(-1, 1), (65536, 1), (0, 0), (0, 18)] {
            assert_eq!(
                sqlstate(insert(&harness.pool, &items[0], town, box_number).await)?,
                "23514"
            );
        }
        storage::seed_character(&harness.pool, 62, 63).await?;
        let mismatch = sqlx::query(
            "INSERT INTO game_item_character_depot_locations \
             (item_instance_id,world_id,character_id,town_id,box_number) \
             VALUES($1::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,0,1)",
        )
        .bind(&items[0])
        .bind(id(fixture::WORLD).as_slice())
        .bind(id(62).as_slice())
        .execute(&harness.pool)
        .await;
        assert_eq!(sqlstate(mismatch)?, "23514");
        for ordinal in ["0", "2", "18446744073709551616"] {
            let result = sqlx::query(
                "INSERT INTO game_item_character_depot_locations \
                 (item_instance_id,world_id,character_id,town_id,box_number,ordinal) \
                 VALUES($1::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,0,1,$4::text::numeric)",
            )
            .bind(&items[0]).bind(id(fixture::WORLD).as_slice())
            .bind(id(fixture::CHARACTER).as_slice()).bind(ordinal)
            .execute(&harness.pool).await;
            assert_eq!(sqlstate(result)?, "23514");
        }
        insert(&harness.pool, &items[0], 0, 1).await?;
        assert_eq!(
            sqlstate(insert(&harness.pool, &items[0], 1, 1).await)?,
            "23505"
        );
        let counters: i64 =
            sqlx::query_scalar("SELECT count(*) FROM game_character_depot_box_counters")
                .fetch_one(&harness.pool)
                .await?;
        assert_eq!(counters, 1, "failed placements must not leak counters");
        let skipped = sqlx::query(
            "INSERT INTO game_item_character_depot_locations \
             (item_instance_id,world_id,character_id,town_id,box_number) \
             VALUES($1::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,0,1) \
             ON CONFLICT DO NOTHING",
        )
        .bind(&items[0])
        .bind(id(fixture::WORLD).as_slice())
        .bind(id(fixture::CHARACTER).as_slice())
        .execute(&harness.pool)
        .await;
        assert_eq!(
            sqlstate(skipped)?,
            "23514",
            "a skipped row cannot consume capacity"
        );

        // The existing Inbox also participates in the one native custody union.
        sqlx::query("INSERT INTO game_character_inbox_cause_kinds VALUES ('DEPOT_TEST_DELIVERY')")
            .execute(&harness.pool)
            .await?;
        sqlx::query(
            "SELECT * FROM game_character_inbox_deliver($1::uuid,encode($2,'hex')::uuid,\
             encode($3,'hex')::uuid,'DEPOT_TEST_DELIVERY','already-in-inbox')",
        )
        .bind(&items[1])
        .bind(id(fixture::CHARACTER).as_slice())
        .bind(id(fixture::WORLD).as_slice())
        .execute(&harness.pool)
        .await?;
        assert_eq!(
            sqlstate(insert(&harness.pool, &items[1], 0, 1).await)?,
            "23514"
        );
        let next: String =
            sqlx::query_scalar("SELECT next_ordinal::text FROM game_character_depot_box_counters")
                .fetch_one(&harness.pool)
                .await?;
        assert_eq!(next, "2");
        Ok(())
    })
}

#[test]
fn depot_rejects_contents_in_every_native_container_family() -> TestResult {
    run("depot_contents", async |harness| {
        // Only baseline fixtures bypass triggers, as in the existing Inbox tests.
        // The attempted depot placements run with every constraint enabled.
        for relation in [
            "game_item_container_entries",
            "game_item_corpse_container_entries",
            "game_item_nested_entries",
        ] {
            let items = seed_items(&harness.pool, 2, 1).await?;
            let mut tx = harness.pool.begin().await?;
            tx.execute("SET LOCAL session_replication_role = replica")
                .await?;
            let character_column = if relation == "game_item_container_entries" {
                ",character_id"
            } else {
                ""
            };
            let character_value = if relation == "game_item_container_entries" {
                ",encode($4,'hex')::uuid"
            } else {
                ""
            };
            let query = sqlx::query(sqlx::AssertSqlSafe(format!(
                "INSERT INTO {relation}(item_instance_id,parent_item_instance_id,world_id,placement_ordinal,placed_transaction_id{character_column}) \
                 VALUES($1::uuid,$2::uuid,encode($3,'hex')::uuid,1,game_character_uuid_v7(){character_value})"
            )))
            .bind(&items[1]).bind(&items[0]).bind(id(fixture::WORLD).to_vec());
            let query = if relation == "game_item_container_entries" {
                query.bind(id(fixture::CHARACTER).to_vec())
            } else {
                query
            };
            query.execute(&mut *tx).await?;
            tx.commit().await?;
            assert_eq!(
                sqlstate(insert(&harness.pool, &items[0], 0, 1).await)?,
                "23514",
                "{relation}"
            );
        }
        Ok(())
    })
}

#[test]
fn depot_runtime_cannot_bypass_missing_locker_writer_or_remove_storage() -> TestResult {
    run("depot_roles", async |harness| {
        let items = seed_items(&harness.pool, 1, 1).await?;
        insert(&harness.pool, &items[0], 0, 1).await?;
        let mut connection = PgConnection::connect(&harness.database.url).await?;
        connection.execute("SET ROLE oteryn_game_runtime").await?;
        for command in [
            "DELETE FROM game_item_character_depot_locations",
            "UPDATE game_item_character_depot_locations SET box_number=2",
            "TRUNCATE game_item_character_depot_locations",
            "DELETE FROM game_character_depot_box_counters",
            "UPDATE game_character_depot_box_counters SET committed=2",
            "INSERT INTO game_item_character_depot_locations SELECT * FROM game_item_character_depot_locations",
        ] {
            assert_eq!(
                sqlstate(connection.execute(command).await)?,
                "42501",
                "{command}"
            );
        }
        let count: i64 =
            sqlx::query_scalar("SELECT count(*) FROM game_item_character_depot_locations")
                .fetch_one(&mut connection)
                .await?;
        assert_eq!(count, 1);
        connection.execute("RESET ROLE").await?;
        for command in [
            "DELETE FROM game_item_character_depot_locations",
            "UPDATE game_item_character_depot_locations SET town_id=2",
            "TRUNCATE game_item_character_depot_locations",
            "UPDATE game_character_depot_box_counters SET committed=2",
            "TRUNCATE game_character_depot_box_counters",
        ] {
            assert_eq!(
                sqlstate(connection.execute(command).await)?,
                "23514",
                "{command}"
            );
        }
        connection.close().await?;
        Ok(())
    })
}

#[test]
fn depot_concurrent_placements_serialize_on_character_root() -> TestResult {
    run("depot_concurrent", async |harness| {
        let items = seed_items(&harness.pool, 2, 1).await?;
        let mut first = harness.pool.begin().await?;
        storage::insert_on(&mut first, &items[0], 0, 1).await?;
        let mut second = PgConnection::connect(&harness.database.url).await?;
        second.execute("SET statement_timeout='5s'").await?;
        let pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
            .fetch_one(&mut second)
            .await?;
        let later_item = items[1].clone();
        let later =
            tokio::spawn(async move { storage::insert_on(&mut second, &later_item, 0, 1).await });
        storage::wait_for_lock(&harness.pool, pid).await?;
        first.commit().await?;
        later.await??;
        let ordinals: Vec<String> = sqlx::query_scalar(
            "SELECT ordinal::text FROM game_item_character_depot_locations \
             ORDER BY ordinal LIMIT 3",
        )
        .fetch_all(&harness.pool)
        .await?;
        assert_eq!(ordinals, ["1", "2"]);
        Ok(())
    })
}
