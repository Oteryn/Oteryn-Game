//! N009: the control-only Platform sanction projection (2800) and the fenced own record (6240).
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{FirstEntryOutcome, FreshAdmissionAuthority};
use crate::gameplay_transport::item_fence_of;
use oteryn_protocol_oteryn::CommandStatus;
use oteryn_protocol_oteryn::sanction_record::*;
use sqlx::Executor;

const PROJECT: &str = "SELECT game_account_sanction_project($1::text::uuid,$2,$3::text::uuid,$4::text::uuid,$5,$6,$7,$8,$9,$10)";

/// One `game_account_sanction_project` call.
struct Decision<'a> {
    stream: &'a str,
    revision: i64,
    account: &'a str,
    character: Option<&'a str>,
    kind: i16,
    reason: &'a str,
    starts: i64,
    ends: Option<i64>,
    revoked: bool,
}

async fn project(
    connection: &mut sqlx::PgConnection,
    decision: &Decision<'_>,
) -> Result<i16, sqlx::Error> {
    sqlx::query_scalar(PROJECT)
        .bind(decision.stream)
        .bind(decision.revision)
        .bind(decision.account)
        .bind(decision.character)
        .bind(decision.kind)
        .bind(9_i32)
        .bind(decision.reason)
        .bind(decision.starts)
        .bind(decision.ends)
        .bind(decision.revoked)
        .fetch_one(connection)
        .await
}

fn sqlstate(result: Result<i16, sqlx::Error>) -> Option<String> {
    result
        .err()?
        .as_database_error()
        .and_then(|e| e.code())
        .map(|c| c.into_owned())
}

#[test]
fn sanction_record_postgres_projection_is_idempotent_and_the_read_is_own_and_fenced() -> TestResult
{
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new().stack_size(256 << 20).spawn(|| run(async |admin| {
        use super::cyclopedia::support::exchange;
        reset();
        let h = Harness::create_with_admission_seed(admin, "sanction_record", true, false).await.map_err(debug)?;
        prepare_current_session(&h).await?;
        let deps = Dependencies::new()?;
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        let current = crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone()).current_session_at(admitted.game_session_id).await?.0;
        admitted.item_fence = item_fence_of(current);
        let fence = admitted.item_fence.ok_or("fence")?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
        let owner = deps.owner(&h, &authority);
        deps.initialize(&owner).await;
        owner.runtime.lock().await.initialize_first_entry_position(deps.actor).map_err(debug)?;
        let (account, character): (String, String) = sqlx::query_as("SELECT account_id::text,character_id::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
            .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
        sqlx::query("INSERT INTO game_character_account_guards(account_id) VALUES(encode($1,'hex')::uuid)")
            .bind(id(90).as_slice()).execute(&h.pool).await?;
        let foreign: String = sqlx::query_scalar("SELECT encode($1,'hex')::uuid::text").bind(id(90).as_slice()).fetch_one(&h.pool).await?;

        let kind = COMMAND_TYPE_SANCTION_RECORD_QUERY;
        // No selected capability: real dispatch refuses.
        assert_eq!(exchange(&owner, &mut admitted, 1, kind, &[]).await?.0, CommandStatus::Rejected);
        admitted.continuity.selected_capabilities = SelectedCapabilities::select(owner.offered_capabilities(), &[CAPABILITY_SANCTION_RECORD_V1]).ok_or("selection")?;
        let (status, empty) = exchange(&owner, &mut admitted, 2, kind, &[]).await?;
        assert_eq!(status, CommandStatus::Accepted);
        assert_eq!(decode_reply(&empty), Some(SanctionRecord { more: false, entries: Vec::new() }));
        assert_eq!(exchange(&owner, &mut admitted, 3, kind, &[0]).await?.0, CommandStatus::Rejected);

        // Platform's control-plane writer: idempotent by decision revision.
        let mute_stream = "01900000-0000-7000-8000-0000000000a1";
        let ban_stream = "01900000-0000-7000-8000-0000000000a2";
        let mute = Decision { stream: mute_stream, revision: 1, account: &account, character: Some(&character), kind: 1, reason: "Insulting", starts: 1_000, ends: Some(2_000), revoked: false };
        let mut runtime = h.pool.acquire().await?;
        runtime.execute("SET ROLE oteryn_game_runtime").await?;
        assert_eq!(sqlstate(project(&mut runtime, &mute).await).as_deref(), Some("42501"));
        runtime.execute("RESET ROLE").await?;
        drop(runtime);
        let mut control = h.pool.acquire().await?;
        control.execute("SET ROLE oteryn_game_control").await?;
        assert_eq!(project(&mut control, &mute).await?, 1);
        assert_eq!(project(&mut control, &mute).await?, 2);
        assert_eq!(sqlstate(project(&mut control, &Decision { reason: "Spamming", ..mute }).await).as_deref(), Some("23000"));
        let lifted = Decision { revision: 2, revoked: true, ..mute };
        assert_eq!(project(&mut control, &lifted).await?, 1);
        assert_eq!(project(&mut control, &mute).await?, 3);
        assert_eq!(sqlstate(project(&mut control, &Decision { revision: 3, kind: 2, ..lifted }).await).as_deref(), Some("23000"));
        assert_eq!(sqlstate(project(&mut control, &Decision { stream: ban_stream, account: &foreign, ..mute }).await).as_deref(), Some("23503"));
        assert_eq!(sqlstate(project(&mut control, &Decision { stream: ban_stream, ends: None, ..mute }).await).as_deref(), Some("23514"));
        let ban = Decision { stream: ban_stream, revision: 1, account: &account, character: None, kind: 4, reason: "Account trading", starts: 3_000, ends: None, revoked: false };
        assert_eq!(project(&mut control, &ban).await?, 1);
        let foreign_notation = Decision { stream: "01900000-0000-7000-8000-0000000000a3", account: &foreign, character: None, kind: 6, starts: 4_000, ..ban };
        assert_eq!(project(&mut control, &foreign_notation).await?, 1);
        control.execute("RESET ROLE").await?;
        drop(control);

        // Only the account-wide rows and this Character's rows, newest first.
        let (status, bytes) = exchange(&owner, &mut admitted, 4, kind, &[]).await?;
        assert_eq!(status, CommandStatus::Accepted);
        let record = decode_reply(&bytes).ok_or("record")?;
        assert_eq!(record, SanctionRecord { more: false, entries: vec![
            SanctionEntry { kind: SanctionKind::AccountBan, character_scope: false, revoked: false, reason_code: 9, starts_at_ms: 3_000, ends_at_ms: None, reason: "Account trading".into() },
            SanctionEntry { kind: SanctionKind::Mute, character_scope: true, revoked: true, reason_code: 9, starts_at_ms: 1_000, ends_at_ms: Some(2_000), reason: "Insulting".into() },
        ] });

        // A stale lease never reads.
        let mut stale = admitted;
        if let Some(ref mut fence) = stale.item_fence { fence.character_lease_generation += 1; }
        assert!(owner.sanction_record_query(stale, 5, &[]).await.is_none());
        let writable: bool = sqlx::query_scalar("SELECT has_table_privilege('oteryn_game_runtime','game_account_sanction_projections','INSERT') OR has_table_privilege('oteryn_game_runtime','game_account_sanction_projections','UPDATE') OR has_table_privilege('oteryn_game_runtime','game_account_sanction_projections','DELETE')").fetch_one(&h.pool).await?;
        assert!(!writable);
        drop(owner); drop(authority); drop(seal);
        h.cleanup().await
    }).map_err(|e|e.to_string()))?.join().map_err(|_|"sanction record panicked")??;
    Ok(())
}
