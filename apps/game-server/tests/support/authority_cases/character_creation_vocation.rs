//! Platform v2 create and v4 bootstrap exercise the fenced durable caller.
use super::*;
use native_admission_source::character_commands::{CREATE_COMMAND, Decision, Input};

#[test]
fn character_creation_vocation_and_sex_persist_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            (|| -> TestResult {
                tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()?
                    .block_on(async {
                        let db = Database::create(admin, "creation_vocation").await?;
                        let result = exercise(&db).await;
                        db.cleanup().await?;
                        if result.is_ok() {
                            eprintln!("CREATION_VOCATION_PG_EXECUTED create=5 bootstrap=1");
                        }
                        result
                    })
            })()
            .map_err(|e| e.to_string())
        })?
        .join()
        .map_err(|_| "creation vocation worker panicked")?
        .map_err(Into::into)
}

async fn exercise(db: &Database) -> TestResult {
    let root = DurabilityRoot::connect_test_runtime(&db.url)?;
    assert!(root.maintain_ready_once().await?);
    let pool = sqlx::PgPool::connect(&db.url).await?;
    let retained = fence_parent().join(format!("oteryn-creation-vocation-{}", db.name));
    std::fs::create_dir(&retained)?;
    let recovery = CharacterRecoveryStore::open(&retained, "character-primary", "game-ops")
        .map_err(|e| format!("{e:?}"))?;
    let fresh = recovery
        .authorize_fresh_store(id(11), 100)
        .map_err(|e| format!("{e:?}"))?;
    root.admit_fresh_character_recovery(&fresh)
        .await
        .map_err(|e| format!("{e:?}"))?;
    drop(fresh);
    let sealed = recovery.seal_current().map_err(|e| format!("{e:?}"))?;
    let authority = root
        .open_character_authority(&sealed)
        .await
        .map_err(|e| format!("{e:?}"))?;
    let node = register(&root, 1, None).await?;
    initialize_s2(&root, &node).await?;
    assign_world(&pool, &root, &node, 90, 95).await?;
    configure(
        &pool,
        ["profile-1", "ruleset-1", "content-1", "newhaven-starter-r1"],
    )
    .await?;
    allow(&root, &node, 41).await?;
    let now = now_secs()?;
    for (i, vocation) in ["Knight", "Paladin", "Sorcerer", "Druid", "Monk"]
        .into_iter()
        .enumerate()
    {
        let operation = 61 + u8::try_from(i)?;
        let raw = create_intent(
            operation,
            41,
            90,
            5 + u64::try_from(i)?,
            &name_for(operation),
            (now - 1, now + 120),
        )
        .replace("\"Male\"", "\"Female\"")
        .replace("\"Knight\"", &format!("\"{vocation}\""));
        let decision = root
            .decide_character_command(
                &authority,
                &node,
                "oteryn:character-authority:primary",
                CREATE_COMMAND,
                &uuid(id(operation)),
                Input::Intent(raw.as_bytes()),
            )
            .await;
        let Decision::Publish {
            receipt,
            result_code,
        } = &decision
        else {
            return Err(format!("create failed: {decision:?}").into());
        };
        assert_eq!(result_code, "CHAR_CMD_OK");
        let receipt: serde_json::Value = serde_json::from_slice(receipt)?;
        assert_eq!(receipt["contract_version"], 1);
        let character = receipt["character_id"]
            .as_str()
            .ok_or("missing character")?;
        assert_created(&pool, character, vocation, 1).await?;
        assert_eq!(
            root.decide_character_command(
                &authority,
                &node,
                "oteryn:character-authority:primary",
                CREATE_COMMAND,
                &uuid(id(operation)),
                Input::Intent(raw.as_bytes())
            )
            .await,
            decision
        );
    }
    let raw = Wire::new(81, 41, 1)?
        .json()
        .replace("starter-1", "newhaven-starter-r1")
        .replace("\"Knight\"", "\"Monk\"");
    let intent = decode_producer_response(raw.as_bytes(), id(81)).map_err(|e| format!("{e:?}"))?;
    let created = root
        .bootstrap_character(&authority, &node, &intent)
        .await
        .map_err(|e| format!("{e:?}"))?;
    assert_created(&pool, &uuid(*created.character_id.as_bytes()), "Monk", 2).await?;
    drop(authority);
    root.open_character_authority(&sealed)
        .await
        .map_err(|e| format!("recovery failed: {e:?}"))?;
    pool.close().await;
    drop(root);
    std::fs::remove_dir_all(retained)?;
    Ok(())
}

async fn assert_created(
    pool: &sqlx::PgPool,
    character: &str,
    vocation: &str,
    sex: i16,
) -> TestResult {
    let row: (String, i16, String, String, String) = sqlx::query_as(
        "SELECT r.creation_vocation,r.sex,r.starter_template_revision,b.vocation,r.character_revision::text FROM game_character_roots r JOIN game_character_build_state b USING(character_id) WHERE r.character_id=$1::text::uuid")
        .bind(character).fetch_one(pool).await?;
    assert_eq!(
        row,
        (
            vocation.to_ascii_lowercase(),
            sex,
            "newhaven-starter-r1".into(),
            vocation.to_ascii_lowercase(),
            "1".into()
        )
    );
    assert!(
        sqlx::query(
            "UPDATE game_character_build_state SET magic_level=1 WHERE character_id=$1::text::uuid"
        )
        .bind(character)
        .execute(pool)
        .await
        .is_err()
    );
    assert!(sqlx::query("UPDATE game_character_roots SET creation_vocation=NULL WHERE character_id=$1::text::uuid")
        .bind(character).execute(pool).await.is_err());
    Ok(())
}
