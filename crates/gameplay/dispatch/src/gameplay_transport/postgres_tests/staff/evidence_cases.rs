use super::*;

pub(super) async fn schema_and_retained_evidence(
    h: &Harness,
    signing: &Signing,
    runtime_url: &str,
) -> TestResult {
    // Real LOGIN membership is checked even through a NOINHERIT intermediary.
    let suffix = format!(
        "{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos()
    );
    let login = format!("gm_test_login_{suffix}");
    let group = format!("gm_test_group_{suffix}");
    let mut tx = h.pool.begin().await?;
    sqlx::raw_sql(sqlx::AssertSqlSafe(format!(
        "CREATE ROLE {login} LOGIN NOINHERIT; CREATE ROLE {group} NOLOGIN NOINHERIT; GRANT oteryn_game_runtime TO {group}; GRANT {group} TO {login};"
    ))).execute(&mut *tx).await?;
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "SET LOCAL SESSION AUTHORIZATION {login}"
    )))
    .execute(&mut *tx)
    .await?;
    sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
        .execute(&mut *tx)
        .await?;
    assert!(
        sqlx::query_scalar::<_, bool>("SELECT game_staff_is_executor_member()")
            .fetch_one(&mut *tx)
            .await?
    );
    let denied = sqlx::query("INSERT INTO game_staff_control_links SELECT environment,100,'ACTIVATION',repeat('e',64),link_sha256,activation_sha256,roster_sha256,principal,key_fingerprint,canonical,signature,NULL,pg_current_xact_id() FROM game_staff_control_links WHERE control_seq=1")
        .execute(&mut *tx).await;
    assert_eq!(
        denied
            .err()
            .and_then(|e| e
                .as_database_error()
                .and_then(|d| d.code().map(|c| c.into_owned())))
            .as_deref(),
        Some("42501")
    );
    tx.rollback().await?;

    // A runtime writer cannot insert a replacement for expired audit evidence.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_staff_action_outbox(environment,operation_id,payload,starts_at_ms,expires_at_ms) SELECT environment,operation_id,$1,starts_at_ms,starts_at_ms+31536000000 FROM game_staff_operations WHERE operation_id=$2::uuid")
        .bind(b"forged".as_slice()).bind(hex(&id(95))).execute(&mut *tx).await?;
    let error = tx.commit().await.err().ok_or("detached audit committed")?;
    assert_eq!(
        error.as_database_error().and_then(|e| e.code()).as_deref(),
        Some("23514")
    );

    // Independently retained bytes must match every committed signed link.
    let retained = signing.directory.join(format!(
        "staff-link-{}-{:020}",
        signing.config.environment, 2
    ));
    let bytes = std::fs::read(&retained)?;
    std::fs::write(&retained, b"torn or changed retained evidence")?;
    let root = crate::durability::DurabilityRoot::connect_test_runtime(runtime_url)?;
    assert!(root.maintain_ready_once().await?);
    assert!(
        root.configure_staff(Some(signing.config.clone()))
            .await
            .is_err()
    );
    std::fs::write(&retained, bytes)?;
    drop(root);
    // A retained acknowledged tip ahead of a restored DB stops boot; no automatic replay.
    let ahead = signing.directory.join(format!(
        "staff-link-{}-{:020}",
        signing.config.environment, 100
    ));
    std::fs::write(&ahead, b"unreconciled retained tail")?;
    let root = crate::durability::DurabilityRoot::connect_test_runtime(runtime_url)?;
    assert!(root.maintain_ready_once().await?);
    assert!(
        root.configure_staff(Some(signing.config.clone()))
            .await
            .is_err()
    );
    std::fs::remove_file(ahead)?;
    drop(root);
    Ok(())
}

pub(super) fn history_boundaries(signing: &Signing) -> TestResult {
    let original = std::fs::read(&signing.config.activation_history_file)?;
    let mut maximum = original.clone();
    maximum.resize(262_144, b' ');
    std::fs::write(&signing.config.activation_history_file, &maximum)?;
    StaffTrust::load(signing.config.clone())?;
    maximum.push(b' ');
    std::fs::write(&signing.config.activation_history_file, &maximum)?;
    assert!(StaffTrust::load(signing.config.clone()).is_err());
    std::fs::write(&signing.config.activation_history_file, original)?;
    Ok(())
}

pub(super) async fn retained_key_across_revisions(
    h: &Harness,
    signing: &mut Signing,
    deps: &Dependencies,
    admitted: AdmittedSession,
    old_request: &wire::SignedEnvelope,
    now: u64,
    runtime_url: &str,
) -> TestResult {
    let mut history: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&signing.config.activation_history_file)?)?;
    let roster = history["revisions"][0]["allowed_signers"]
        .as_str()
        .ok_or("roster")?
        .replace("operator GM", "operator ADMIN");
    let previous: String =
        sqlx::query_scalar("SELECT link_sha256 FROM game_staff_control_heads WHERE environment=$1")
            .bind(&signing.config.environment)
            .fetch_one(&h.pool)
            .await?;
    let activation = rules::Activation {
        control_seq: "11".into(),
        environment: signing.config.environment.clone(),
        previous_link_sha256: previous,
        roster_sha256: sha(roster.as_bytes()),
    };
    let signed = signing.sign(&canonical(&activation)?, false)?;
    let digest = sha(&signed.canonical);
    history["revisions"]
        .as_array_mut()
        .ok_or("history")?
        .push(serde_json::json!({
            "allowed_signers": roster, "activation": String::from_utf8(signed.canonical.clone())?,
            "signature": String::from_utf8(signed.signature.clone())?, "principal": "operator",
        }));
    // Only an owner can install this activation, then restart the node. No runtime
    // command reloads configuration or grants the new role.
    let mut tx = h.pool.begin().await?;
    sqlx::query("INSERT INTO game_staff_rosters(environment,activation_sha256,roster_sha256,principal,key_fingerprint,role) VALUES($1,$2,$3,'operator',$4,'ADMIN')")
        .bind(&activation.environment).bind(&digest).bind(&activation.roster_sha256)
        .bind(&signing.fingerprint).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_staff_control_links(environment,control_seq,kind,link_sha256,previous_link_sha256,activation_sha256,roster_sha256,principal,key_fingerprint,canonical,signature) VALUES($1,11,'ACTIVATION',$2,$3,$2,$4,'operator',$5,$6,$7)")
        .bind(&activation.environment).bind(&digest).bind(&activation.previous_link_sha256)
        .bind(&activation.roster_sha256).bind(&signing.fingerprint)
        .bind(&signed.canonical).bind(&signed.signature).execute(&mut *tx).await?;
    sqlx::query("UPDATE game_staff_control_heads SET control_seq=11,link_sha256=$2,activation_sha256=$2 WHERE environment=$1")
        .bind(&activation.environment).bind(&digest).execute(&mut *tx).await?;
    tx.commit().await?;
    signing.config.allowed_signers_sha256 = activation.roster_sha256;
    signing.config.newest_activation_sha256 = digest;
    std::fs::write(
        &signing.config.activation_history_file,
        serde_json::to_vec(&history)?,
    )?;
    let root = crate::durability::DurabilityRoot::connect_test_runtime(runtime_url)?;
    assert!(root.maintain_ready_once().await?);
    root.configure_staff(Some(signing.config.clone())).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = root.open_character_authority(&seal).await.map_err(debug)?;
    let mut owner = deps.owner(h, &authority);
    owner.root = &root;
    let old_outcome: Vec<u8> =
        sqlx::query_scalar("SELECT outcome FROM game_staff_operations WHERE operation_id=$1::uuid")
            .bind(hex(&id(90)))
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(
        submit(&owner, admitted, old_request, false).await?,
        Some(old_outcome)
    );
    let next = signing.request(
        12,
        &signing.config.newest_activation_sha256,
        97,
        now,
        rules::Action::Inspect {
            character_id: hex(&id(41)),
        },
    );
    let signed_next = signing.sign(&canonical(&next)?, false)?;
    assert!(
        submit(&owner, admitted, &signed_next, false)
            .await?
            .is_some()
    );
    let unmute = signing.request(
        13,
        &sha(&signed_next.canonical),
        98,
        now,
        rules::Action::Unmute {
            character_id: hex(&id(41)),
            sanction_id: hex(&id(90)),
        },
    );
    let refused = submit(
        &owner,
        admitted,
        &signing.sign(&canonical(&unmute)?, false)?,
        false,
    )
    .await?
    .ok_or("already lifted sanction")?;
    assert_eq!(
        wire::decode_result(&refused)?.code,
        wire::StaffCode::AlreadyReleased
    );
    Ok(())
}
