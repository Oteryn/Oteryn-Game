use crate::bestiary_postgres_harness::{
    self as fixture, CHARACTER, Harness, TestResult, debug, id,
};
use crate::domain::CharacterId;
use crate::durability::DurabilityRoot;
use crate::durability::spell_item_transaction::{
    assert_spell_item_authority_in_transaction, begin_spell_owner_transaction,
};
use crate::durability::spell_owner_commit::SpellLanePermit;
use crate::durability::spell_privacy::{
    ExivaDecision, ExivaPreferences, read_exiva_decision_in_transaction,
    write_exiva_preferences_in_transaction,
};
use crate::foundation::{CommandId, CommandRef};
use oteryn_protocol_oteryn::character_info::PageRequest;

#[test]
fn runtime_reads_character_info_inbox_and_find_person_access_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            fixture::runtime()
                .map_err(debug)?
                .block_on(run_case(admin))
                .map_err(debug)
        })?
        .join()
        .map_err(|_| "runtime read grants PostgreSQL worker failed")??;
    Ok(())
}

async fn run_case(admin: String) -> TestResult {
    let h = Harness::create(admin, "inbox_spell_grants", true).await?;
    let inbox = read_inbox_summary(&h).await.map_err(debug);
    let exiva = read_exiva(&h).await.map_err(debug);
    let identity_update = try_update_spell_access_identity(&h).await.map_err(debug);
    h.cleanup().await?;

    // Evaluate both reads before asserting, so the base run reports both failures.
    assert_eq!(
        (inbox, exiva),
        (Ok(1), Ok(ExivaDecision::Allowed)),
        "G10 Character Info inbox count and C05 Find Person privacy decision"
    );
    assert_eq!(
        identity_update?,
        (
            Some("42501".to_owned()),
            "runtime cannot update spell access identity".to_owned()
        )
    );
    eprintln!("RUNTIME_READ_GRANTS_PG_EXECUTED character_info_inbox find_person_access");
    Ok(())
}

async fn read_inbox_summary(h: &Harness) -> TestResult<i64> {
    let character = CharacterId::from_bytes(id(CHARACTER)).map_err(debug)?;
    sqlx::query(
        "INSERT INTO game_character_inbox_counters(character_id, committed, next_ordinal) \
         VALUES(encode($1,'hex')::uuid, 1, 2)",
    )
    .bind(character.as_bytes().as_slice())
    .execute(&h.pool)
    .await?;
    let runtime = DurabilityRoot::connect_test_runtime(&format!(
        "{}?options=-c%20role%3Doteryn_game_runtime",
        h.database.url
    ))?;
    assert!(runtime.maintain_ready_once().await?);
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = runtime
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let bytes = runtime
        .read_character_info(
            &authority,
            character,
            character,
            PageRequest {
                offset: 0,
                limit: 1,
            },
            1000,
        )
        .await
        .map_err(debug)?;
    // The inbox count is the last i64 before the two u16 history counts.
    let count = bytes
        .get(57..65)
        .ok_or("missing Character Info inbox count")?;
    Ok(i64::from_le_bytes(count.try_into()?))
}

async fn read_exiva(h: &Harness) -> TestResult<ExivaDecision> {
    let character = id(CHARACTER);
    sqlx::query(
        "INSERT INTO game_character_spell_access(character_id, staff_access, source_policy, source_policy_digest) \
         VALUES(encode($1,'hex')::uuid, false, 'fixture', decode(repeat('01', 32), 'hex'))",
    )
    .bind(character.as_slice())
    .execute(&h.pool)
    .await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let recovery = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let fence = fixture::item_fence()?;
    let permit = SpellLanePermit::of_fresh_scope(fence.runtime_scope).await?;
    let command = CommandRef::new(fence.game_session_id, CommandId::new(1).map_err(debug)?);
    let mut context = (h, recovery, permit);
    h.root
        .try_issue_semantic_pass()?
        .run_with_context(
            &mut context,
            move |holder, deadline, (h, recovery, permit)| {
                Box::pin(async move {
                    let mut tx = begin_spell_owner_transaction(holder, deadline).await?;
                    sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
                        .execute(&mut *tx)
                        .await?;
                    let decision = async {
                        let authority = assert_spell_item_authority_in_transaction(
                            &mut tx, permit, &h.root, recovery, &h.node, &fence, command, [1; 32],
                        )
                        .await?;
                        let mut preferences = ExivaPreferences::source_initial_preferences();
                        preferences.allow_all = true;
                        write_exiva_preferences_in_transaction(
                            &mut tx,
                            &authority,
                            None,
                            &preferences,
                        )
                        .await?;
                        read_exiva_decision_in_transaction(&mut tx, &authority, character).await
                    }
                    .await;
                    tx.rollback().await?;
                    Ok(decision)
                })
            },
        )
        .await?
        .map_err(|error| debug(error).into())
}

async fn try_update_spell_access_identity(h: &Harness) -> TestResult<(Option<String>, String)> {
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
        .execute(&mut *tx)
        .await?;
    let result = sqlx::query("UPDATE game_character_spell_access SET character_id = character_id")
        .execute(&mut *tx)
        .await;
    tx.rollback().await?;
    match result {
        Err(sqlx::Error::Database(error)) => Ok((
            error.code().map(|code| code.into_owned()),
            error.message().to_owned(),
        )),
        Err(error) => Err(error.into()),
        Ok(_) => Err("runtime updated spell access identity".into()),
    }
}
