// ACHIEVEMENT step 3 account facts (migration 0021,
// `durability::account_achievement`). Every wrapper provides the same
// path-loaded crate root and the `bestiary_postgres_harness` module, whose
// Character 41 of account 40 is live on session 50, and the path-loaded
// `achievement_catalogue` for the display read.

use crate::achievement_catalogue::AchievementCatalogue;
use crate::bestiary_postgres_harness::{
    CHANNEL, CHARACTER, Harness, TestResult, WORLD, configured_admin, debug, fence, id, runtime,
};
use crate::domain::CharacterId;
use crate::durability::DurabilityRoot;
use crate::durability::account_achievement::{
    AccountAchievement, AchievementCatalogueLookup, AchievementGrantError, AchievementGrantOutcome,
    AchievementGrantRequest, AchievementSourceEvent, EarnedAchievement,
};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::foundation::{ConnectionGeneration, GameSessionId};
use oteryn_protocol_oteryn::account_achievements::AccountAchievementsResult;

const ACCOUNT: u8 = 40;
const SECOND_CHARACTER: u8 = 44;
const SECOND_SESSION: u8 = 51;
const COOKIES: &str = "oteryn:achievement/allow_cookies";
const NUT: &str = "oteryn:achievement/the_professor_s_nut";
const MERRIER: &str = "oteryn:achievement/the_more_the_merrier";

fn source(tag: u8) -> AchievementSourceEvent {
    AchievementSourceEvent {
        kind: "oteryn:test-granter".into(),
        event_id: id(tag).to_vec(),
    }
}

fn grant(key: &str, tag: u8, catalogue: AchievementCatalogueLookup) -> AchievementGrantRequest {
    AchievementGrantRequest {
        achievement_key: key.into(),
        catalogue,
        source: source(tag),
    }
}

fn earnable(key: &str, tag: u8) -> AchievementGrantRequest {
    grant(
        key,
        tag,
        AchievementCatalogueLookup::Earnable {
            revision: "r1".into(),
        },
    )
}

fn granted(outcomes: Vec<AchievementGrantOutcome>) -> TestResult<AccountAchievement> {
    match <[AchievementGrantOutcome; 1]>::try_from(outcomes) {
        Ok([AchievementGrantOutcome::Granted(fact)]) => Ok(fact),
        other => Err(format!("expected one new fact, got {other:?}").into()),
    }
}

async fn counts(harness: &Harness) -> TestResult<(i64, i64)> {
    Ok((
        harness
            .count("game_account_achievement_grant_requests")
            .await?,
        harness.count("game_account_achievements").await?,
    ))
}

fn run(
    tag: &'static str,
    case: impl AsyncFnOnce(&Harness, &DurabilityRoot) -> TestResult,
) -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let harness = Harness::create(admin, tag, false).await?;
        let root = harness.root.clone();
        case(&harness, &root).await?;
        harness.cleanup().await
    })
}

/// Character 44 of the same account takes the account's presence on session
/// 51; session 50 of Character 41 ends.
async fn switch_to_second_character(harness: &Harness) -> TestResult {
    let pool = &harness.pool;
    sqlx::query(
        "INSERT INTO game_character_roots VALUES \
         (encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,\
          1,1,'profile-1','ruleset-1','content-1','starter-1','Second Hero')",
    )
    .bind(id(SECOND_CHARACTER).as_slice())
    .bind(id(ACCOUNT).as_slice())
    .bind(id(WORLD).as_slice())
    .execute(pool)
    .await?;
    sqlx::query("UPDATE game_durability_reconnect_sessions SET session_state = 3")
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
    .bind(id(ACCOUNT).as_slice())
    .bind(id(SECOND_CHARACTER).as_slice())
    .bind(id(WORLD).as_slice())
    .bind(id(CHANNEL).as_slice())
    .bind([8_u8; 16].as_slice())
    .execute(pool)
    .await?;
    sqlx::query(
        "INSERT INTO game_durability_admission_character_guards VALUES \
         (encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,\
          true,1,encode($4,'hex')::uuid,1,'test',1,'character-current',1,0,'{}')",
    )
    .bind(id(SECOND_CHARACTER).as_slice())
    .bind(id(ACCOUNT).as_slice())
    .bind(id(WORLD).as_slice())
    .bind(id(SECOND_SESSION).as_slice())
    .execute(pool)
    .await?;
    sqlx::query(
        "UPDATE game_durability_admission_account_guards \
            SET presence_character_id = encode($1,'hex')::uuid, \
                holder_game_session_id = encode($2,'hex')::uuid",
    )
    .bind(id(SECOND_CHARACTER).as_slice())
    .bind(id(SECOND_SESSION).as_slice())
    .execute(pool)
    .await?;
    Ok(())
}

fn second_fence()
-> TestResult<crate::durability::character_progression::CurrentCharacterGameplayFence> {
    let mut fence = fence(1)?;
    fence.character_id = CharacterId::from_bytes(id(SECOND_CHARACTER)).map_err(debug)?;
    fence.game_session_id = GameSessionId::decode(&id(SECOND_SESSION)).map_err(debug)?;
    Ok(fence)
}

#[test]
fn a_fact_is_derived_only_from_a_request_committed_in_a_fenced_grant() -> TestResult {
    run("derived", async |harness, root| {
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = root.open_character_authority(&seal).await.map_err(debug)?;
        let fact = granted(
            root.commit_test_achievement_grants(
                &authority,
                &harness.node,
                fence(1)?,
                vec![earnable(COOKIES, 60)],
            )
            .await
            .map_err(debug)?,
        )?;
        assert_eq!(fact.account_id, id(ACCOUNT));
        assert_eq!(fact.character_id.as_bytes(), &id(CHARACTER));
        assert_eq!(fact.achievement_revision, "r1");
        assert_eq!(fact.source, source(60));
        assert!(fact.earned_at_unix_ms > 0);
        assert_eq!(counts(harness).await?, (1, 1));
        // A grant is not a Character semantic transaction of its own.
        assert_eq!(harness.root_revision().await?, "1");

        let pool = &harness.pool;
        // A fact without its request.
        assert!(
            sqlx::query(
                "INSERT INTO game_account_achievements VALUES \
                 (encode($1,'hex')::uuid, $2, 'oteryn:test-granter', $3)",
            )
            .bind(id(ACCOUNT).as_slice())
            .bind(NUT)
            .bind(id(61).as_slice())
            .execute(pool)
            .await
            .is_err()
        );
        // A request that is never consumed, and a request bound to another
        // account than its Character's, fail at commit.
        for (account, with_fact) in [(ACCOUNT, false), (45, true)] {
            let mut tx = pool.begin().await?;
            sqlx::query(
                "INSERT INTO game_account_achievement_grant_requests VALUES \
                 ('oteryn:test-granter', $1, $2, 'r1', encode($3,'hex')::uuid, \
                  encode($4,'hex')::uuid, 1)",
            )
            .bind(id(62).as_slice())
            .bind(NUT)
            .bind(id(account).as_slice())
            .bind(id(CHARACTER).as_slice())
            .execute(&mut *tx)
            .await?;
            if with_fact {
                sqlx::query(
                    "INSERT INTO game_account_achievements VALUES \
                     (encode($1,'hex')::uuid, $2, 'oteryn:test-granter', $3)",
                )
                .bind(id(account).as_slice())
                .bind(NUT)
                .bind(id(62).as_slice())
                .execute(&mut *tx)
                .await?;
            }
            assert!(tx.commit().await.is_err(), "account {account}");
        }
        for statement in [
            "UPDATE game_account_achievements SET source_kind = source_kind",
            "DELETE FROM game_account_achievements",
            "UPDATE game_account_achievement_grant_requests SET earned_at = 0",
            "DELETE FROM game_account_achievement_grant_requests",
            "TRUNCATE game_account_achievements, game_account_achievement_grant_requests",
        ] {
            assert!(
                sqlx::query(statement).execute(pool).await.is_err(),
                "{statement}"
            );
        }
        assert_eq!(counts(harness).await?, (1, 1));
        Ok(())
    })
}

#[test]
fn a_duplicate_grant_keeps_one_fact_and_its_first_committed_provenance() -> TestResult {
    run("duplicate", async |harness, root| {
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = root.open_character_authority(&seal).await.map_err(debug)?;
        let commit = async |requests| {
            root.commit_test_achievement_grants(&authority, &harness.node, fence(1)?, requests)
                .await
                .map_err(|error| -> Box<dyn std::error::Error> { debug(error).into() })
        };
        let first = granted(commit(vec![earnable(COOKIES, 60)]).await?)?;
        // Another source event, a replay of the first, and another source
        // event under a newer catalogue revision.
        let mut newer = earnable(COOKIES, 62);
        newer.catalogue = AchievementCatalogueLookup::Earnable {
            revision: "r2".into(),
        };
        for request in [earnable(COOKIES, 61), earnable(COOKIES, 60), newer] {
            assert_eq!(
                commit(vec![request]).await?,
                vec![AchievementGrantOutcome::AlreadyHeld(first.clone())]
            );
        }
        assert_eq!(counts(harness).await?, (3, 1));
        // The first source event reused with other facts conflicts and
        // writes nothing.
        let mut reused = earnable(COOKIES, 60);
        reused.catalogue = AchievementCatalogueLookup::Earnable {
            revision: "r2".into(),
        };
        assert!(matches!(
            root.commit_test_achievement_grants(
                &authority,
                &harness.node,
                fence(1)?,
                vec![earnable(NUT, 63), reused]
            )
            .await,
            Err(AchievementGrantError::ConflictingSourceEvent)
        ));
        assert_eq!(counts(harness).await?, (3, 1));
        Ok(())
    })
}

#[test]
fn a_stale_character_fence_writes_neither_request_nor_fact() -> TestResult {
    run("stale", async |harness, root| {
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = root.open_character_authority(&seal).await.map_err(debug)?;
        // The session moves to connection generation 2; the old fence is stale.
        sqlx::query("UPDATE game_durability_reconnect_sessions SET current_generation = 2")
            .execute(&harness.pool)
            .await?;
        assert!(matches!(
            root.commit_test_achievement_grants(
                &authority,
                &harness.node,
                fence(1)?,
                vec![earnable(COOKIES, 60)]
            )
            .await,
            Err(AchievementGrantError::AuthorityRejected)
        ));
        assert_eq!(counts(harness).await?, (0, 0));
        let mut current = fence(1)?;
        current.connection_generation = ConnectionGeneration::new(2).map_err(debug)?;
        granted(
            root.commit_test_achievement_grants(
                &authority,
                &harness.node,
                current,
                vec![earnable(COOKIES, 60)],
            )
            .await
            .map_err(debug)?,
        )?;
        assert_eq!(counts(harness).await?, (1, 1));
        Ok(())
    })
}

/// Only the fenced token's own transaction is changed: the same complete
/// current fence holds in both transactions, the request is valid and new.
#[test]
fn a_token_minted_in_another_transaction_writes_neither_request_nor_fact() -> TestResult {
    run("foreign", async |harness, root| {
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = root.open_character_authority(&seal).await.map_err(debug)?;
        let outcome = root
            .commit_test_achievement_grant_with_foreign_token(
                &authority,
                &harness.node,
                fence(1)?,
                earnable(COOKIES, 60),
            )
            .await;
        assert!(
            matches!(outcome, Err(AchievementGrantError::AuthorityRejected)),
            "{outcome:?}"
        );
        assert_eq!(counts(harness).await?, (0, 0));
        assert_eq!(harness.root_revision().await?, "1");
        // The same request with a token of its own transaction grants.
        granted(
            root.commit_test_achievement_grants(
                &authority,
                &harness.node,
                fence(1)?,
                vec![earnable(COOKIES, 60)],
            )
            .await
            .map_err(debug)?,
        )?;
        assert_eq!(counts(harness).await?, (1, 1));
        Ok(())
    })
}

#[test]
fn an_unknown_key_fails_closed_and_a_retired_key_is_a_no_op() -> TestResult {
    run("catalogue", async |harness, root| {
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = root.open_character_authority(&seal).await.map_err(debug)?;
        assert!(matches!(
            root.commit_test_achievement_grants(
                &authority,
                &harness.node,
                fence(1)?,
                vec![
                    earnable(COOKIES, 60),
                    grant(NUT, 61, AchievementCatalogueLookup::Absent),
                ],
            )
            .await,
            Err(AchievementGrantError::UnknownAchievement)
        ));
        assert_eq!(counts(harness).await?, (0, 0));

        let outcomes = root
            .commit_test_achievement_grants(
                &authority,
                &harness.node,
                fence(1)?,
                vec![
                    grant(MERRIER, 62, AchievementCatalogueLookup::Retired),
                    earnable(COOKIES, 63),
                ],
            )
            .await
            .map_err(debug)?;
        let [
            AchievementGrantOutcome::Retired,
            AchievementGrantOutcome::Granted(fact),
        ] = outcomes.as_slice()
        else {
            return Err(format!("unexpected outcomes: {outcomes:?}").into());
        };
        assert_eq!(fact.source, source(63));
        assert_eq!(counts(harness).await?, (1, 1));
        let retired: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_account_achievement_grant_requests \
              WHERE achievement_key = $1",
        )
        .bind(MERRIER)
        .fetch_one(&harness.pool)
        .await?;
        assert_eq!(retired, 0);
        Ok(())
    })
}

#[test]
fn two_characters_of_one_account_share_one_fact() -> TestResult {
    run("account", async |harness, root| {
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = root.open_character_authority(&seal).await.map_err(debug)?;
        let first = granted(
            root.commit_test_achievement_grants(
                &authority,
                &harness.node,
                fence(1)?,
                vec![earnable(COOKIES, 60)],
            )
            .await
            .map_err(debug)?,
        )?;
        switch_to_second_character(harness).await?;
        // Character 41's session ended: its fence is stale.
        assert!(matches!(
            root.commit_test_achievement_grants(
                &authority,
                &harness.node,
                fence(1)?,
                vec![earnable(NUT, 61)]
            )
            .await,
            Err(AchievementGrantError::AuthorityRejected)
        ));
        let outcomes = root
            .commit_test_achievement_grants(
                &authority,
                &harness.node,
                second_fence()?,
                vec![earnable(COOKIES, 62), earnable(NUT, 63)],
            )
            .await
            .map_err(debug)?;
        let [
            AchievementGrantOutcome::AlreadyHeld(held),
            AchievementGrantOutcome::Granted(nut),
        ] = outcomes.as_slice()
        else {
            return Err(format!("unexpected outcomes: {outcomes:?}").into());
        };
        assert_eq!(held, &first);
        assert_eq!(nut.character_id.as_bytes(), &id(SECOND_CHARACTER));
        assert_eq!(nut.account_id, id(ACCOUNT));
        assert_eq!(counts(harness).await?, (3, 2));

        // Two concurrent transactions for one new key: the second waits on
        // the unique key and inserts nothing once the first commits.
        let pool = &harness.pool;
        let request = "INSERT INTO game_account_achievement_grant_requests VALUES \
             ('oteryn:test-granter', $1, $2, 'r1', encode($3,'hex')::uuid, \
              encode($4,'hex')::uuid, 1)";
        let fact = "INSERT INTO game_account_achievements VALUES \
             (encode($1,'hex')::uuid, $2, 'oteryn:test-granter', $3) \
             ON CONFLICT (account_id, achievement_key) DO NOTHING";
        let key = "oteryn:achievement/sculptor_apprentice";
        let mut earlier = pool.begin().await?;
        let mut later = pool.begin().await?;
        for (tx, tag, character) in [
            (&mut earlier, 70, CHARACTER),
            (&mut later, 71, SECOND_CHARACTER),
        ] {
            sqlx::query(request)
                .bind(id(tag).as_slice())
                .bind(key)
                .bind(id(ACCOUNT).as_slice())
                .bind(id(character).as_slice())
                .execute(&mut **tx)
                .await?;
        }
        sqlx::query(fact)
            .bind(id(ACCOUNT).as_slice())
            .bind(key)
            .bind(id(70).as_slice())
            .execute(&mut *earlier)
            .await?;
        let commit_earlier = tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(300)).await;
            earlier.commit().await
        });
        let inserted = sqlx::query(fact)
            .bind(id(ACCOUNT).as_slice())
            .bind(key)
            .bind(id(71).as_slice())
            .execute(&mut *later)
            .await?
            .rows_affected();
        commit_earlier.await??;
        assert_eq!(inserted, 0);
        later.commit().await?;
        let provenance: Vec<u8> = sqlx::query_scalar(
            "SELECT source_event_id FROM game_account_achievements WHERE achievement_key = $1",
        )
        .bind(key)
        .fetch_one(pool)
        .await?;
        assert_eq!(provenance, id(70).to_vec());
        assert_eq!(counts(harness).await?, (5, 3));
        Ok(())
    })
}

/// Every statement of a grant, a duplicate and a retired no-op runs as the
/// runtime group, never as the migration owner: the 0021 grants suffice.
#[test]
fn grants_commit_under_the_runtime_role_grants() -> TestResult {
    run("runtime", async |harness, _root| {
        let database: String = sqlx::query_scalar("SELECT current_database()")
            .fetch_one(&harness.pool)
            .await?;
        let suffix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos();
        let role = format!("achievement_runtime_{suffix}");
        let password = format!("{role}-secret");
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
            .ok_or("database URL has no authority separator")?;
        let outcome = async {
            let runtime = DurabilityRoot::connect_test_runtime(&format!(
                "postgresql://{role}:{password}@{address}"
            ))?;
            assert!(runtime.maintain_ready_once().await?);
            let seal = harness.recovery.seal_current().map_err(debug)?;
            let authority = runtime
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let outcomes = runtime
                .commit_test_achievement_grants(
                    &authority,
                    &harness.node,
                    fence(1)?,
                    vec![
                        earnable(COOKIES, 60),
                        earnable(COOKIES, 61),
                        grant(MERRIER, 62, AchievementCatalogueLookup::Retired),
                    ],
                )
                .await
                .map_err(|error| format!("runtime grant: {error:?}"))?;
            assert!(matches!(
                outcomes.as_slice(),
                [
                    AchievementGrantOutcome::Granted(_),
                    AchievementGrantOutcome::AlreadyHeld(_),
                    AchievementGrantOutcome::Retired,
                ]
            ));
            // The display read needs only the runtime role's SELECT (display contract §4.4).
            let facts = runtime
                .read_account_achievements(&authority, id(ACCOUNT), 2)
                .await
                .map_err(|error| format!("runtime read: {error:?}"))?;
            assert_eq!(
                facts
                    .iter()
                    .map(|fact| fact.achievement_key.as_str())
                    .collect::<Vec<_>>(),
                [COOKIES]
            );
            TestResult::Ok(())
        }
        .await;
        sqlx::query(sqlx::AssertSqlSafe(format!("DROP OWNED BY {role}")))
            .execute(&harness.pool)
            .await?;
        sqlx::query(sqlx::AssertSqlSafe(format!("DROP ROLE {role}")))
            .execute(&harness.pool)
            .await?;
        outcome?;
        assert_eq!(counts(harness).await?, (2, 1));
        Ok(())
    })
}

// ACCOUNT_ACHIEVEMENTS_QUERY display read (`OTERYN_ACHIEVEMENT_DISPLAY_CONTRACT_V1`
// §3-§4): `DurabilityRoot::read_account_achievements` and the runtime
// catalogue's page, over real facts.

const OTHER_ACCOUNT: u8 = 45;
const OTHER_CHARACTER: u8 = 46;
const EMPTY_ACCOUNT: u8 = 47;

fn catalogue() -> TestResult<AchievementCatalogue> {
    AchievementCatalogue::embedded().map_err(|error| debug(error).into())
}

/// The session account's facts as the query reads them.
async fn read_facts(
    root: &DurabilityRoot,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    account: u8,
) -> TestResult<Vec<EarnedAchievement>> {
    root.read_account_achievements(authority, id(account), catalogue()?.len() + 1)
        .await
        .map_err(|error| debug(error).into())
}

async fn read_page(
    root: &DurabilityRoot,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    account: u8,
    page: u32,
) -> TestResult<AccountAchievementsResult> {
    let facts = read_facts(root, authority, account).await?;
    catalogue()?
        .account_achievements_page(&facts, page)
        .map_err(|error| debug(error).into())
}

/// A fact recorded outside the fenced grant path, as a historical grant: its
/// request and the fact in one transaction, under a live Character of
/// `account`. `tag` names the source event.
async fn insert_fact(
    harness: &Harness,
    account: u8,
    character: u8,
    key: &str,
    tag: u16,
    earned_at: i64,
) -> TestResult {
    let mut tx = harness.pool.begin().await?;
    sqlx::query(
        "INSERT INTO game_account_achievement_grant_requests VALUES \
         ('oteryn:test-history', $1, $2, 'r1', encode($3,'hex')::uuid, \
          encode($4,'hex')::uuid, $5)",
    )
    .bind(tag.to_be_bytes().as_slice())
    .bind(key)
    .bind(id(account).as_slice())
    .bind(id(character).as_slice())
    .bind(earned_at)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "INSERT INTO game_account_achievements VALUES \
         (encode($1,'hex')::uuid, $2, 'oteryn:test-history', $3)",
    )
    .bind(id(account).as_slice())
    .bind(key)
    .bind(tag.to_be_bytes().as_slice())
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(())
}

/// A live Character 46 of a second account 45.
async fn seed_other_account(harness: &Harness) -> TestResult {
    sqlx::query("INSERT INTO game_character_account_guards VALUES (encode($1,'hex')::uuid)")
        .bind(id(OTHER_ACCOUNT).as_slice())
        .execute(&harness.pool)
        .await?;
    sqlx::query(
        "INSERT INTO game_character_roots VALUES \
         (encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,\
          1,1,'profile-1','ruleset-1','content-1','starter-1','Other Hero')",
    )
    .bind(id(OTHER_CHARACTER).as_slice())
    .bind(id(OTHER_ACCOUNT).as_slice())
    .bind(id(WORLD).as_slice())
    .execute(&harness.pool)
    .await?;
    Ok(())
}

fn row_keys(page: &AccountAchievementsResult) -> Vec<&str> {
    page.rows.iter().map(|row| row.key.as_str()).collect()
}

/// Display contract §3.2, §4.1-§4.2: only the account's own facts, one row per
/// fact with the earning time of its first committed request, and no record
/// the account has not earned.
#[test]
fn the_query_returns_only_the_accounts_own_earned_facts() -> TestResult {
    run("query_own", async |harness, root| {
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = root.open_character_authority(&seal).await.map_err(debug)?;
        let first = granted(
            root.commit_test_achievement_grants(
                &authority,
                &harness.node,
                fence(1)?,
                vec![earnable(COOKIES, 60)],
            )
            .await
            .map_err(debug)?,
        )?;
        // A later request for the same key keeps the first provenance.
        root.commit_test_achievement_grants(
            &authority,
            &harness.node,
            fence(1)?,
            vec![earnable(COOKIES, 61)],
        )
        .await
        .map_err(debug)?;
        seed_other_account(harness).await?;
        insert_fact(harness, OTHER_ACCOUNT, OTHER_CHARACTER, NUT, 1, 5).await?;
        assert_eq!(counts(harness).await?, (3, 2));

        let own = read_page(root, &authority, ACCOUNT, 0).await?;
        assert_eq!(row_keys(&own), [COOKIES]);
        assert_eq!(own.rows[0].earned_at, first.earned_at_unix_ms);
        assert_eq!(own.rows[0].name, "Allow Cookies?");
        assert_eq!((own.total_points, own.fact_count), (2, 1));
        assert!(!own.has_more);

        let other = read_page(root, &authority, OTHER_ACCOUNT, 0).await?;
        assert_eq!(row_keys(&other), [NUT]);
        assert_eq!(other.rows[0].earned_at, 5);
        assert_eq!((other.total_points, other.fact_count), (3, 1));

        // An account with no fact reads an empty page.
        assert_eq!(
            read_page(root, &authority, EMPTY_ACCOUNT, 0).await?,
            AccountAchievementsResult::default()
        );
        Ok(())
    })
}

/// Display contract §2.1, §3.3: a retired fact is shown with 0 points, and a
/// zero-point grant moves `fact_count` but not `total_points`.
#[test]
fn a_retired_fact_shows_with_zero_points_and_moves_only_the_watermark() -> TestResult {
    run("query_retired", async |harness, root| {
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = root.open_character_authority(&seal).await.map_err(debug)?;
        granted(
            root.commit_test_achievement_grants(
                &authority,
                &harness.node,
                fence(1)?,
                vec![earnable(COOKIES, 60)],
            )
            .await
            .map_err(debug)?,
        )?;
        let before = read_page(root, &authority, ACCOUNT, 0).await?;
        assert_eq!((before.total_points, before.fact_count), (2, 1));
        // Granted before the record was retired.
        insert_fact(harness, ACCOUNT, CHARACTER, MERRIER, 1, 9).await?;
        let after = read_page(root, &authority, ACCOUNT, 0).await?;
        assert_eq!((after.total_points, after.fact_count), (2, 2));
        let retired = after
            .rows
            .iter()
            .find(|row| row.key == MERRIER)
            .ok_or("the retired fact is shown")?;
        assert_eq!(
            (retired.points, retired.secret, retired.grade),
            (0, true, 1)
        );
        Ok(())
    })
}

/// Display contract §3.3-§3.4: pages of 64 rows in grade, name, key order,
/// `has_more` until the last page, no rows past the end.
#[test]
fn the_query_pages_by_64_rows() -> TestResult {
    run("query_pages", async |harness, root| {
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = root.open_character_authority(&seal).await.map_err(debug)?;
        let catalogue = catalogue()?;
        let shard: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../content/achievements/achievements-00000-00499.json"
        ))?;
        let keys: Vec<String> = shard["records"]
            .as_array()
            .ok_or("records")?
            .iter()
            .filter_map(|record| record["identity"]["key"].as_str().map(str::to_owned))
            .take(130)
            .collect();
        assert_eq!(keys.len(), 130);
        for (tag, key) in (1_u16..).zip(&keys) {
            insert_fact(harness, ACCOUNT, CHARACTER, key, tag, i64::from(tag)).await?;
        }
        let facts = read_facts(root, &authority, ACCOUNT).await?;
        let all = catalogue
            .account_achievements_page(&facts, 0)
            .map_err(debug)?;
        let mut seen = Vec::new();
        for (page, rows, has_more) in [(0, 64, true), (1, 64, true), (2, 2, false), (3, 0, false)] {
            let result = read_page(root, &authority, ACCOUNT, page).await?;
            assert_eq!(
                (result.rows.len(), result.has_more),
                (rows, has_more),
                "{page}"
            );
            assert_eq!(
                (result.total_points, result.fact_count),
                (all.total_points, 130)
            );
            seen.extend(result.rows);
        }
        let ordered = seen.windows(2).all(|pair| {
            (pair[0].grade, &pair[0].name, &pair[0].key)
                < (pair[1].grade, &pair[1].name, &pair[1].key)
        });
        assert!(ordered, "grade, name, key order across pages");
        let mut shown: Vec<String> = seen.into_iter().map(|row| row.key).collect();
        shown.sort();
        let mut expected = keys;
        expected.sort();
        assert_eq!(shown, expected);
        Ok(())
    })
}

/// Owner decision 2026-09-30 (display contract §4.3): a fact under a key the
/// catalogue lacks, seeded by the admin role as a historical fact because the
/// fenced grant path refuses an absent key, is an integrity fault. The answer
/// is `ACCOUNT_DATA_INTEGRITY` (the code and nothing else on the wire), with
/// one error-level operator event naming the account, the key and the
/// request; the account's valid fact is not shown in its place.
#[test]
fn a_fact_under_a_key_absent_from_the_catalogue_is_an_integrity_fault() -> TestResult {
    use crate::achievement_catalogue::{
        AccountAchievementsPageError, AccountAchievementsRequest, integrity_fault_event,
    };
    use oteryn_protocol_oteryn::{
        CommandResultView, CommandStatus, FoundationProtocolError, decode_command_result,
        decode_wire_envelope, encode_command_error_result,
    };
    const ABSENT: &str = "oteryn:achievement/not_in_the_catalogue";
    run("query_integrity", async |harness, root| {
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = root.open_character_authority(&seal).await.map_err(debug)?;
        granted(
            root.commit_test_achievement_grants(
                &authority,
                &harness.node,
                fence(1)?,
                vec![earnable(COOKIES, 60)],
            )
            .await
            .map_err(debug)?,
        )?;
        insert_fact(harness, ACCOUNT, CHARACTER, ABSENT, 1, 9).await?;
        let catalogue = catalogue()?;
        let facts = read_facts(root, &authority, ACCOUNT).await?;
        assert_eq!(facts.len(), 2);
        let request = AccountAchievementsRequest {
            account_id: id(ACCOUNT),
            page: 0,
            game_session_id: id(50),
            command_id: 12,
        };
        let mut events = Vec::new();
        let answer = catalogue.answer_account_achievements(&facts, &request, &mut |line| {
            events.push(line.to_owned());
        });
        assert_eq!(
            answer,
            Err(AccountAchievementsPageError::UnknownKey(ABSENT.into()))
        );
        let Err(error) = answer else {
            return Err("the integrity fault fails closed".into());
        };
        let code = error.protocol_error().ok_or("a registered code")?;
        assert_eq!(code, FoundationProtocolError::AccountDataIntegrity);
        assert_eq!(code.code(), 1050);
        assert_eq!(events, [integrity_fault_event(&request, ABSENT)]);
        assert!(events[0].starts_with("event=account_achievement_integrity_fault level=error "));
        assert!(events[0].contains(&format!("achievement_key=\"{ABSENT}\"")));
        assert!(events[0].ends_with(" command_id=12"));
        // The client receives REJECTED with the code only: no row, no key.
        let wire = encode_command_error_result(4, 9, request.command_id, code).map_err(debug)?;
        let envelope = decode_wire_envelope(&wire).map_err(debug)?;
        assert_eq!(
            decode_command_result(envelope.payload()).map_err(debug)?,
            CommandResultView {
                command_id: 12,
                status: CommandStatus::Rejected,
                error_code: 1050,
                payload: &[],
            }
        );
        Ok(())
    })
}
