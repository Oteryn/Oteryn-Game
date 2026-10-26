//! WRITE-1 rewrite prerequisites, not qualification of the pending TEXT ingress.
use super::*;

fn rewrite_request(command: u64, text: &str) -> TestResult<ItemTextWriteRequest> {
    let mut request = request(command, text)?;
    request.definition.production_key = "oteryn:item.tibia.i2815".into();
    Ok(request)
}

fn policy(request: &ItemTextWriteRequest, maximum: u32) -> MailTextPolicy {
    MailTextPolicy::from_qualified_content(request.definition.clone(), [1; 32], maximum)
}

#[test]
fn write_content_scalar_limits_controls_and_mail_limits_are_exact() -> TestResult {
    for maximum in [1, 79, 511, 1023, 1500, 1999, 3997, 4000] {
        for scalar in ['a', '🦀'] {
            let request = rewrite_request(1, &scalar.to_string().repeat(maximum))?;
            let policy = policy(&request, maximum as u32);
            assert_eq!(qualified_text(&policy, &request)?, request.text);
            let mut longer = rewrite_request(2, &request.text)?;
            longer.text.push(scalar);
            assert!(matches!(
                qualified_text(&policy, &longer),
                Err(ItemTextWriteError::TooLong)
            ));
        }
    }
    let request = rewrite_request(1, "")?;
    for maximum in [0, 4001, u32::MAX] {
        assert!(matches!(
            qualified_text(&policy(&request, maximum), &request),
            Err(ItemTextWriteError::NotWritable)
        ));
    }
    let limit = policy(&request, 511);
    assert_eq!(qualified_text(&limit, &request)?, "");
    assert_eq!(
        qualified_text(&limit, &rewrite_request(2, "a\nb")?)?,
        "a\nb"
    );
    for control in ['\0', '\t', '\r', '\u{7f}', '\u{85}', '\u{9f}'] {
        assert!(matches!(
            qualified_text(&limit, &rewrite_request(2, &format!("a{control}b"))?),
            Err(ItemTextWriteError::Rejected)
        ));
    }
    for (key, maximum) in [
        ("oteryn:item.tibia.i3505", 1999),
        ("oteryn:item.tibia.i3507", 79),
    ] {
        let mut mail = rewrite_request(1, &"🦀".repeat(maximum))?;
        mail.definition.production_key = key.into();
        let correct = policy(&mail, maximum as u32);
        assert_eq!(qualified_text(&correct, &mail)?, mail.text);
        mail.text.push('a');
        assert!(matches!(
            qualified_text(&correct, &mail),
            Err(ItemTextWriteError::TooLong)
        ));
        assert!(matches!(
            qualified_text(&policy(&mail, 4000), &mail),
            Err(ItemTextWriteError::NotWritable)
        ));
    }
    let mut different = rewrite_request(1, "")?;
    different.definition.revision_ref = "definition-r2".into();
    assert!(matches!(
        qualified_text(&limit, &different),
        Err(ItemTextWriteError::NotWritable)
    ));
    different.definition.production_key = "oteryn:item.tibia.i3506".into();
    assert!(matches!(
        qualified_text(&policy(&different, 1999), &different),
        Err(ItemTextWriteError::NotWritable)
    ));
    Ok(())
}

#[test]
fn write_postgres_scalar_ceiling_and_rewrite_metadata_survive_reload() -> TestResult {
    use crate::bestiary_postgres_harness::{Harness, configured_admin, runtime};
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let harness = Harness::create(admin, "write_limits", false).await?;
        let result = async {
            let pool = &harness.pool;
            let character = storage_fixture(pool).await?;
            // Native migrator installs the successor, retaining standing 0101.
            let installed: bool = sqlx::query_scalar("SELECT success FROM _sqlx_migrations WHERE version=263")
                .fetch_one(pool).await?;
            assert!(installed);
            let original = rewrite_request(1, &"🦀".repeat(4000))?;
            let limit = policy(&original, 4000);
            let mut tx = pool.begin().await?;
            sqlx::query("SET LOCAL ROLE oteryn_game_runtime").execute(&mut *tx).await?;
            lock_item_text_command(&mut tx, original.command).await?;
            sqlx::query("SELECT character_id FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid FOR UPDATE")
                .bind(character.as_slice()).execute(&mut *tx).await?;
            sqlx::query("SELECT item_instance_id FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid FOR UPDATE")
                .bind(original.item_instance_id.as_slice()).execute(&mut *tx).await?;
            let written = persist_rewrite(&mut tx, &character, &original, qualified_text(&limit, &original)?).await?;
            assert_eq!(written.text_revision, 1);
            tx.commit().await?;
            let state: (i32, i32, bool, bool, String) = sqlx::query_as("SELECT char_length(text),octet_length(text),writer_character_id=encode($1,'hex')::uuid,isfinite(written_at),text_revision::text FROM game_item_texts WHERE item_instance_id=encode($2,'hex')::uuid")
                .bind(character.as_slice()).bind(original.item_instance_id.as_slice()).fetch_one(pool).await?;
            assert_eq!(state, (4000, 16000, true, true, "1".into()));
            // Database protection independently rejects scalar max+1, including
            // ASCII which stays below the byte ceiling, without losing the text.
            for body in ["a".repeat(4001), "🦀".repeat(4001), String::new()] {
                let mut tx = pool.begin().await?;
                sqlx::query("SET LOCAL ROLE oteryn_game_runtime").execute(&mut *tx).await?;
                let error = sqlx::query("UPDATE game_item_texts SET text=$1 WHERE item_instance_id=encode($2,'hex')::uuid")
                    .bind(body).bind(original.item_instance_id.as_slice()).execute(&mut *tx).await
                    .err().ok_or("database admitted text outside WRITE-0 ceiling")?;
                assert_eq!(error.as_database_error().and_then(|error| error.code()).as_deref(), Some("23514"));
                tx.rollback().await?;
            }
            let mut tx = pool.begin().await?;
            let frozen = original.freeze_receipt_binding(CharacterId::from_bytes(character)?);
            lock_item_text_command(&mut tx, original.command).await?;
            assert_eq!(load_item_text_receipt(&mut tx, &frozen).await?, Some(ItemTextWriteOutcome { replayed: true, ..written }));
            tx.rollback().await?;
            let retained: String = sqlx::query_scalar("SELECT text FROM game_item_texts WHERE item_instance_id=encode($1,'hex')::uuid")
                .bind(original.item_instance_id.as_slice()).fetch_one(pool).await?;
            assert_eq!(retained, original.text);
            // Re-running this migration is safe and retains existing text/ACLs.
            sqlx::raw_sql(include_str!("../../../../../../../apps/game-server/migrations/0263_item_text_write_limits.sql"))
                .execute(pool).await?;
            text_storage_delete_retains_tombstone_and_rewrite_advances_on_existing_fixture(pool, &character).await?;
            Ok::<(), Box<dyn std::error::Error>>(())
        }.await;
        harness.cleanup().await?;
        if result.is_ok() {
            eprintln!("WRITE-NATIVE-PG migration0263 scalar/byte ceiling, metadata, replay, rollback and rerun PASS");
        }
        result
    })
}

async fn text_storage_delete_retains_tombstone_and_rewrite_advances_on_existing_fixture(
    pool: &sqlx::PgPool,
    character: &[u8; 16],
) -> TestResult {
    // Original max-size write used one token. Empty write removes writer/date
    // together with the body, while preserving the revision tombstone.
    assert_eq!(store(pool, character, 2, "").await?.text_revision, 2);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_item_texts")
            .fetch_one(pool)
            .await?,
        0
    );
    sqlx::query("UPDATE game_item_text_write_receipts SET written_at=clock_timestamp()-interval '2 seconds'")
        .execute(pool).await?;
    assert_eq!(store(pool, character, 3, "new").await?.text_revision, 3);
    Ok(())
}
