//! MAIL-1 leaf qualification under the real runtime role and native item guards.
//! Source items are native MINT/TRANSFERs. Content/handle admission remains gated.
use crate::bestiary_postgres_harness::{TestResult, configured_admin, runtime};
use crate::durability::item_transfer::ItemTransferOutcome;
use crate::durability::mail::{
    MAIL_INBOX_ITEM_CEILING, MailPostOutcome, MailPostRefusal, post_mail_item,
};
use crate::item_transfer_postgres_cases as native;
use oteryn_gameplay_domain::domain::{CharacterId, WorldId};

const STAMP: &str = "fixture:mail.stamped";

fn run(
    tag: &'static str,
    case: impl AsyncFnOnce(&native::Harness, [u8; 16]) -> TestResult,
) -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async {
        let h = native::Harness::create(admin, tag).await?;
        let result = async {
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
                    native::to_slot(native::command(11)?, backpack, native::backpack_facts())
                )
                .await
                .map_err(native::debug)?,
                ItemTransferOutcome::Committed(_)
            ));
            let item = h.mint(&authority, native::COIN, 1).await?;
            assert!(matches!(
                h.transfer(
                    &authority,
                    native::fence()?,
                    native::to_backpack(
                        native::command(12)?,
                        item,
                        native::facts(native::COIN, native::STACKABLE)
                    )
                )
                .await
                .map_err(native::debug)?,
                ItemTransferOutcome::Committed(_)
            ));
            case(&h, item).await
        }
        .await;
        h.cleanup().await?;
        result
    })
}

async fn post(
    h: &native::Harness,
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    item: [u8; 16],
    recipient: [u8; 16],
    command: u64,
    hash: [u8; 32],
) -> TestResult<MailPostOutcome> {
    sqlx::query("RESET ROLE").execute(&mut **tx).await?;
    let fence = native::fence()?;
    assert!(
        crate::durability::item_transfer::character_item_fence_is_current(
            tx,
            &h.node,
            &fence,
            native::command(command)?,
            fence.character_id,
            native::id(native::WORLD),
            native::id(native::CHANNEL),
        )
        .await?
    );
    sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
        .execute(&mut **tx)
        .await?;
    Ok(post_mail_item(
        tx,
        native::id(210),
        native::id(native::SESSION),
        command,
        WorldId::from_bytes(native::id(native::WORLD))?,
        native::id(native::CHANNEL),
        fence.character_id,
        CharacterId::from_bytes(recipient)?,
        item,
        STAMP,
        b"fixture:mailbox",
        hash,
        native::id(211),
        b"text-free-event",
    )
    .await?)
}

async fn runtime_tx(h: &native::Harness) -> TestResult<sqlx::Transaction<'static, sqlx::Postgres>> {
    Ok(h.pool.begin().await?)
}

async fn footprint(h: &native::Harness, item: [u8; 16]) -> TestResult<serde_json::Value> {
    // Each subquery is bounded by an item/Character key; only MAIL's two fresh rows exist here.
    Ok(sqlx::query_scalar("SELECT jsonb_build_array(
        (SELECT to_jsonb(i) FROM game_item_instances i WHERE item_instance_id=encode($1,'hex')::uuid),
        (SELECT to_jsonb(e) FROM game_item_container_entries e WHERE item_instance_id=encode($1,'hex')::uuid),
        (SELECT to_jsonb(c) FROM game_character_inbox_counters c WHERE character_id=encode($2,'hex')::uuid),
        (SELECT count(*) FROM game_mail_operations), (SELECT count(*) FROM game_mail_outbox),
        (SELECT count(*) FROM game_character_inbox_deliveries WHERE item_instance_id=encode($1,'hex')::uuid),
        (SELECT count(*) FROM game_item_character_inbox_locations WHERE item_instance_id=encode($1,'hex')::uuid))")
        .bind(item.as_slice()).bind(native::id(native::CHARACTER).as_slice()).fetch_one(&h.pool).await?)
}

#[test]
fn mail_posting_commits_identity_stamp_counter_and_replays_once() -> TestResult {
    run("mail_post_once", async |h, item| {
        let mut tx = runtime_tx(h).await?;
        assert_eq!(
            post(h, &mut tx, item, native::id(native::CHARACTER), 31, [1; 32]).await?,
            MailPostOutcome::Posted
        );
        tx.commit().await?; // Runs all deferred native guards; no trigger bypass.
        let state: (String, String, String) = sqlx::query_as("SELECT i.definition_production_key,c.committed::text,c.next_ordinal::text FROM game_item_instances i JOIN game_item_character_inbox_locations l USING(item_instance_id) JOIN game_character_inbox_counters c USING(character_id) WHERE i.item_instance_id=encode($1,'hex')::uuid")
            .bind(item.as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(state, (STAMP.into(), "1".into(), "2".into()));
        let evidence: (i64,i64,i64) = sqlx::query_as("SELECT (SELECT count(*) FROM game_mail_operations WHERE root_item_instance_id=encode($1,'hex')::uuid),(SELECT count(*) FROM game_mail_outbox WHERE mail_id=encode($2,'hex')::uuid AND event_payload=$3),(SELECT count(*) FROM game_item_container_entries WHERE item_instance_id=encode($1,'hex')::uuid)")
            .bind(item.as_slice()).bind(native::id(210).as_slice()).bind(b"text-free-event".as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(evidence, (1, 1, 0));
        let before = footprint(h, item).await?;
        for (hash, expected) in [
            ([1; 32], MailPostOutcome::AlreadyPosted),
            (
                [2; 32],
                MailPostOutcome::Refused(MailPostRefusal::ReplayConflict),
            ),
        ] {
            let mut tx = runtime_tx(h).await?;
            assert_eq!(
                post(h, &mut tx, item, native::id(native::CHARACTER), 31, hash).await?,
                expected
            );
            tx.commit().await?;
            assert_eq!(footprint(h, item).await?, before);
        }
        Ok(())
    })
}

#[test]
fn mail_posting_second_post_within_one_second_is_exhausted() -> TestResult {
    run("mail_post_rate", async |h, item| {
        // Same transaction makes the interval deterministic even under shared-PC load.
        let mut tx = runtime_tx(h).await?;
        assert_eq!(
            post(h, &mut tx, item, native::id(native::CHARACTER), 31, [1; 32]).await?,
            MailPostOutcome::Posted
        );
        let elapsed: bool = sqlx::query_scalar("SELECT clock_timestamp()-occurred_at < interval '1 second' FROM game_mail_operations WHERE mail_id=encode($1,'hex')::uuid")
            .bind(native::id(210).as_slice()).fetch_one(&mut *tx).await?;
        assert!(elapsed, "shared host took over a second before rate check");
        assert_eq!(
            post(h, &mut tx, item, native::id(native::CHARACTER), 32, [2; 32]).await?,
            MailPostOutcome::Refused(MailPostRefusal::Exhausted)
        );
        tx.commit().await?;
        assert_eq!(h.count("game_mail_operations").await?, 1);
        Ok(())
    })
}

#[test]
fn mail_posting_unknown_and_other_world_recipients_preserve_sender() -> TestResult {
    run("mail_post_unknown", async |h, item| {
        // Recipient root only is a disclosed fixture; production reads/locks and item guards run normally.
        let mut seed = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *seed)
            .await?;
        sqlx::query("INSERT INTO game_character_roots(character_id,account_id,world_id,lifecycle,character_revision,profile_revision,ruleset_revision,content_revision,starter_template_revision,name) SELECT encode($1,'hex')::uuid,account_id,encode($2,'hex')::uuid,lifecycle,character_revision,profile_revision,ruleset_revision,content_revision,starter_template_revision,'Foreign Hero' FROM game_character_roots WHERE character_id=encode($3,'hex')::uuid")
            .bind(native::id(220).as_slice()).bind(native::id(221).as_slice()).bind(native::id(native::CHARACTER).as_slice()).execute(&mut *seed).await?;
        seed.commit().await?;
        let before = footprint(h, item).await?;
        for recipient in [native::id(219), native::id(220)] {
            let mut tx = runtime_tx(h).await?;
            assert_eq!(
                post(h, &mut tx, item, recipient, 31, [1; 32]).await?,
                MailPostOutcome::Refused(MailPostRefusal::UnknownRecipient)
            );
            tx.commit().await?;
            assert_eq!(footprint(h, item).await?, before);
        }
        Ok(())
    })
}

#[test]
fn mail_posting_not_in_sender_and_full_inbox_preserve_everything() -> TestResult {
    run("mail_post_refuse", async |h, item| {
        let before = footprint(h, item).await?;
        let backpack: String=sqlx::query_scalar("SELECT item_instance_id::text FROM game_item_container_slots WHERE character_id=encode($1,'hex')::uuid")
            .bind(native::id(native::CHARACTER).as_slice()).fetch_one(&h.pool).await?;
        let backpack = uuid_bytes(&backpack)?;
        for wrong_item in [native::id(219), backpack] {
            let mut tx = runtime_tx(h).await?;
            assert_eq!(
                post(
                    h,
                    &mut tx,
                    wrong_item,
                    native::id(native::CHARACTER),
                    31,
                    [1; 32]
                )
                .await?,
                MailPostOutcome::Refused(MailPostRefusal::NotMailable)
            );
            tx.commit().await?;
            assert_eq!(footprint(h, item).await?, before);
        }
        // Boundary fixture: no need to mint 50,000 unrelated values to test the counter ceiling.
        sqlx::query("INSERT INTO game_character_inbox_counters(character_id,committed,next_ordinal) VALUES(encode($1,'hex')::uuid,$2::text::numeric,$3::text::numeric)")
            .bind(native::id(native::CHARACTER).as_slice())
            .bind(MAIL_INBOX_ITEM_CEILING.to_string())
            .bind((MAIL_INBOX_ITEM_CEILING + 1).to_string())
            .execute(&h.pool).await?;
        let before = footprint(h, item).await?;
        let mut tx = runtime_tx(h).await?;
        assert_eq!(
            post(h, &mut tx, item, native::id(native::CHARACTER), 31, [1; 32]).await?,
            MailPostOutcome::Refused(MailPostRefusal::RecipientInboxFull)
        );
        tx.commit().await?;
        assert_eq!(footprint(h, item).await?, before);
        Ok(())
    })
}

fn uuid_bytes(text: &str) -> TestResult<[u8; 16]> {
    let hex = text.replace('-', "");
    let mut bytes = [0; 16];
    for (index, byte) in bytes.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&hex[index * 2..index * 2 + 2], 16)?;
    }
    Ok(bytes)
}

#[test]
fn mail_posting_nonempty_parcel_is_refused_without_removal() -> TestResult {
    run("mail_post_children", async |h, item| {
        // Disclosed child-location fixture isolates the emptiness check. Only
        // fixture setup suppresses guards; the posting transaction runs normally.
        let mut seed = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *seed)
            .await?;
        sqlx::query("INSERT INTO game_item_nested_entries(item_instance_id,world_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,1,encode($4,'hex')::uuid)")
            .bind(native::id(230).as_slice()).bind(native::id(native::WORLD).as_slice()).bind(item.as_slice()).bind(native::id(231).as_slice()).execute(&mut *seed).await?;
        seed.commit().await?;
        let before = footprint(h, item).await?;
        let mut tx = runtime_tx(h).await?;
        assert_eq!(
            post(h, &mut tx, item, native::id(native::CHARACTER), 31, [1; 32]).await?,
            MailPostOutcome::Refused(MailPostRefusal::NotMailable)
        );
        tx.commit().await?;
        assert_eq!(footprint(h, item).await?, before);
        let count:i64=sqlx::query_scalar("SELECT count(*) FROM game_item_nested_entries WHERE parent_item_instance_id=encode($1,'hex')::uuid").bind(item.as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(count, 1);
        Ok(())
    })
}

#[test]
fn mail_posting_zero_row_stamp_fails_closed_and_rollback_restores_sender() -> TestResult {
    run("mail_post_stamp", async |h, item| {
        // Simulate a stamp suppressed by a BEFORE trigger; never disable a guard.
        sqlx::raw_sql("CREATE FUNCTION fixture_mail_skip_stamp() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RETURN NULL; END $$;
            CREATE TRIGGER aaa_fixture_mail_skip_stamp BEFORE UPDATE ON game_item_instances FOR EACH ROW EXECUTE FUNCTION fixture_mail_skip_stamp()")
            .execute(&h.pool).await?;
        let before = footprint(h, item).await?;
        let mut tx = runtime_tx(h).await?;
        let result = post(h, &mut tx, item, native::id(native::CHARACTER), 31, [1; 32]).await;
        assert!(
            matches!(
                result
                    .as_ref()
                    .err()
                    .and_then(|e| e.downcast_ref::<crate::durability::DurabilityError>()),
                Some(crate::durability::DurabilityError::InvalidStoredState)
            ),
            "expected InvalidStoredState, got {result:?}"
        );
        tx.rollback().await?;
        assert_eq!(footprint(h, item).await?, before);
        Ok(())
    })
}

#[test]
fn mail_posting_runtime_control_acl_and_receipt_immutability() -> TestResult {
    run("mail_post_acl", async |h, item| {
        for table in ["game_mail_operations", "game_mail_outbox"] {
            for (role, privilege, expected) in [
                ("oteryn_game_runtime", "SELECT", true),
                ("oteryn_game_runtime", "INSERT", true),
                ("oteryn_game_runtime", "UPDATE", false),
                ("oteryn_game_runtime", "DELETE", false),
                ("oteryn_game_runtime", "TRUNCATE", false),
                ("oteryn_game_control", "SELECT", true),
                ("oteryn_game_control", "INSERT", false),
                ("oteryn_game_control", "UPDATE", false),
                ("oteryn_game_control", "DELETE", false),
                ("oteryn_game_control", "TRUNCATE", false),
            ] {
                let actual: bool = sqlx::query_scalar("SELECT has_table_privilege($1,$2,$3)")
                    .bind(role)
                    .bind(table)
                    .bind(privilege)
                    .fetch_one(&h.pool)
                    .await?;
                assert_eq!(actual, expected, "{role} {privilege} {table}");
            }
        }
        for table in [
            "game_character_inbox_counters",
            "game_character_inbox_deliveries",
            "game_item_character_inbox_locations",
        ] {
            for privilege in ["INSERT", "UPDATE", "DELETE", "TRUNCATE"] {
                let granted: bool =
                    sqlx::query_scalar("SELECT has_table_privilege('oteryn_game_runtime',$1,$2)")
                        .bind(table)
                        .bind(privilege)
                        .fetch_one(&h.pool)
                        .await?;
                assert!(!granted, "Inbox remains closed: {privilege} {table}");
            }
        }
        let apply = "game_mail_posting_apply(uuid,uuid,numeric,uuid,uuid,uuid,uuid,uuid,text,bytea,bytea,uuid,bytea,jsonb,jsonb)";
        for (role, expected) in [
            ("oteryn_game_runtime", true),
            ("oteryn_game_control", false),
        ] {
            let granted: bool =
                sqlx::query_scalar("SELECT has_function_privilege($1,$2,'EXECUTE')")
                    .bind(role)
                    .bind(apply)
                    .fetch_one(&h.pool)
                    .await?;
            assert_eq!(granted, expected);
        }
        let definer: (bool, Vec<String>) =
            sqlx::query_as("SELECT prosecdef,proconfig FROM pg_proc WHERE oid=$1::regprocedure")
                .bind(apply)
                .fetch_one(&h.pool)
                .await?;
        assert!(definer.0);
        assert!(
            definer
                .1
                .iter()
                .any(|setting| setting == "search_path=pg_catalog, public, pg_temp")
        );
        let mut tx = runtime_tx(h).await?;
        assert_eq!(
            post(h, &mut tx, item, native::id(native::CHARACTER), 31, [1; 32]).await?,
            MailPostOutcome::Posted
        );
        tx.commit().await?;
        // Even privileged callers cannot rewrite the durable replay binding.
        assert!(
            sqlx::query("UPDATE game_mail_operations SET request_sha256=$1")
                .bind([2u8; 32].as_slice())
                .execute(&h.pool)
                .await
                .is_err()
        );
        Ok(())
    })
}
