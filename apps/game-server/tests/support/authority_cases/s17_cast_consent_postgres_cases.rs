//! S17 P1: the installed migration, least privilege and default-off consent.
use crate::bestiary_postgres_harness::{Harness, TestResult, configured_admin, runtime};

#[test]
fn s17_cast_consent_postgres_schema_defaults_and_grants() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            runtime()
                .map_err(|error| error.to_string())?
                .block_on(async {
                    let started = std::time::Instant::now();
                    let h = Harness::create(admin, "s17_consent_schema", true).await?;
                    let result = async {
                        let (allowed, revision): (bool, i64) = sqlx::query_as(
                            "INSERT INTO game_character_cast_consent(character_id) \
                             SELECT character_id FROM game_character_roots \
                             RETURNING allowed, revision",
                        )
                        .fetch_one(&h.pool)
                        .await?;
                        assert!(!allowed);
                        assert_eq!(revision, 1);
                        let grants: (bool, bool, bool, bool) = sqlx::query_as(
                            "SELECT has_table_privilege('oteryn_game_runtime', \
                             'game_character_cast_consent', 'SELECT') AND \
                             has_table_privilege('oteryn_game_runtime', \
                             'game_character_cast_consent', 'INSERT') AND \
                             has_table_privilege('oteryn_game_runtime', \
                             'game_character_cast_consent', 'UPDATE'), \
                             has_table_privilege('oteryn_game_runtime', \
                             'game_character_cast_consent', 'DELETE'), \
                             has_table_privilege('oteryn_game_control', \
                             'game_character_cast_consent', 'SELECT'), \
                             has_table_privilege('oteryn_game_control', \
                             'game_character_cast_consent', 'UPDATE')",
                        )
                        .fetch_one(&h.pool)
                        .await?;
                        assert_eq!(grants, (true, false, true, false));
                        Ok::<(), Box<dyn std::error::Error>>(())
                    }
                    .await;
                    h.cleanup().await?;
                    eprintln!("S17 real PostgreSQL consent schema: {:.3}s", started.elapsed().as_secs_f64());
                    result
                })
                .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "S17 consent test thread panicked")?
        .map_err(Into::into)
}

#[test]
fn s17_cast_consent_postgres_owner_cas_reload_and_session_fences() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            runtime()
                .map_err(|error| error.to_string())?
                .block_on(consent_owner_case(admin))
                .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "S17 consent test thread panicked")?
        .map_err(Into::into)
}

async fn consent_owner_case(admin: String) -> TestResult {
    let started = std::time::Instant::now();
    use crate::bestiary_postgres_harness::{debug, fence, id};
    use crate::durability::cast_consent::{CastConsent, CastConsentOutcome as Outcome};

    let h = Harness::create(admin, "s17_consent_owner", true).await?;
    let result = async {
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
        let off = CastConsent::default();
        assert_eq!(
            h.root.character_cast_consent(&authority, &h.node, fence(1)?, None).await?,
            Outcome::Read(off)
        );
        let on = CastConsent { allowed: true, revision: 1 };
        assert_eq!(
            h.root.character_cast_consent(&authority, &h.node, fence(1)?, Some((0, true))).await?,
            Outcome::Written(on)
        );
        // CAS precedes equal-value replay detection.
        assert_eq!(
            h.root.character_cast_consent(&authority, &h.node, fence(1)?, Some((0, true))).await?,
            Outcome::Conflict(on)
        );
        assert_eq!(
            h.root.character_cast_consent(&authority, &h.node, fence(1)?, Some((1, true))).await?,
            Outcome::Unchanged(on)
        );
        // A separate runtime adapter sees committed state; no consent cache is involved.
        let reopened = crate::durability::DurabilityRoot::connect_test_runtime(&h.database.url)?;
        assert!(reopened.maintain_ready_once().await?);
        let reopened_authority = reopened.open_character_authority(&seal).await.map_err(debug)?;
        assert_eq!(
            reopened.character_cast_consent(&reopened_authority, &h.node, fence(1)?, None).await?,
            Outcome::Read(on)
        );
        // A new alternate on the same account does not inherit this opt-in.
        sqlx::query(
            "INSERT INTO game_character_roots(character_id,account_id,world_id,lifecycle,\
             character_revision,profile_revision,ruleset_revision,content_revision,\
             starter_template_revision,name) \
             SELECT encode($1,'hex')::uuid,account_id,world_id,lifecycle,character_revision,\
             profile_revision,ruleset_revision,content_revision,starter_template_revision,\
             'Cast Consent Alt' FROM game_character_roots WHERE character_id=encode($2,'hex')::uuid",
        )
        .bind(id(98).as_slice())
        .bind(id(crate::bestiary_postgres_harness::CHARACTER).as_slice())
        .execute(&h.pool)
        .await?;
        let alternate: (bool, i64) = sqlx::query_as(
            "SELECT COALESCE(c.allowed,FALSE), COALESCE(c.revision,0::bigint) \
             FROM game_character_roots r LEFT JOIN game_character_cast_consent c \
             ON c.character_id=r.character_id WHERE r.character_id=encode($1,'hex')::uuid",
        )
        .bind(id(98).as_slice())
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(alternate, (false, 0));
        let primary=crate::domain::CharacterId::from_bytes(id(crate::bestiary_postgres_harness::CHARACTER)).map_err(debug)?;
        let alt=crate::domain::CharacterId::from_bytes(id(98)).map_err(debug)?;
        assert_eq!(h.root.read_cast_consents(&h.node,&[primary,alt]).await?,vec![(primary,on),(alt,off)]);
        assert!(h.root.read_cast_consents(&h.node,&vec![primary;129]).await.is_err());
        // Mutate one current-authority dimension per case, for both reads and writes.
        let valid = fence(1)?;
        let mut rejected = Vec::new();
        let mut invalid = valid;
        invalid.connection_generation = crate::foundation::ConnectionGeneration::new(2).map_err(debug)?;
        rejected.push(invalid);
        invalid = valid;
        invalid.character_lease_generation += 1;
        rejected.push(invalid);
        invalid = valid;
        invalid.scope_ownership_generation = crate::foundation::ScopeOwnershipGeneration::new(2).map_err(debug)?;
        rejected.push(invalid);
        invalid = valid;
        invalid.game_session_id = crate::foundation::GameSessionId::decode(&id(99)).map_err(debug)?;
        rejected.push(invalid);
        invalid = valid;
        invalid.character_id = crate::domain::CharacterId::from_bytes(id(98)).map_err(debug)?;
        rejected.push(invalid);
        invalid = valid;
        invalid.runtime_scope = crate::foundation::RuntimeScopeRefV1::channel(
            valid.runtime_scope.world_id(),
            crate::foundation::ChannelId::decode(&id(99)).map_err(debug)?,
        );
        rejected.push(invalid);
        invalid = valid;
        invalid.expected_character_revision = crate::domain::CharacterRevision::new(2).map_err(debug)?;
        rejected.push(invalid);
        for invalid in rejected {
            for change in [None, Some((1, false))] {
                assert!(h.root.character_cast_consent(&authority, &h.node, invalid, change).await.is_err());
            }
        }
        for revision in [-1, i64::MAX] {
            assert!(h.root.character_cast_consent(&authority, &h.node, valid, Some((revision, false))).await.is_err());
        }
        let withdrawn = CastConsent { allowed: false, revision: 2 };
        assert_eq!(
            h.root.character_cast_consent(&authority, &h.node, valid, Some((1, false))).await?,
            Outcome::Written(withdrawn)
        );
        assert_eq!(
            reopened.character_cast_consent(&reopened_authority, &h.node, valid, None).await?,
            Outcome::Read(withdrawn)
        );
        assert_eq!(h.root_revision().await?, "1");
        let audited:i64=sqlx::query_scalar("SELECT count(*) FROM game_cast_audit_outbox").fetch_one(&h.pool).await?;
        assert_eq!(audited,2,"both consent mutations must commit their audit atomically");
        Ok(())
    }.await;
    h.cleanup().await?;
    eprintln!("S17 real PostgreSQL consent owner: {:.3}s", started.elapsed().as_secs_f64());
    result
}
