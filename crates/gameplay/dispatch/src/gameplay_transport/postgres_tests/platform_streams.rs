//! N009/N014: pulled Platform stream pages project with their cursor in one transaction
//! (2800, 2802, 2803). The cursor fences concurrent pullers; records are idempotent by revision.
use super::fixture::{Harness, debug, id, run};
use super::{TestResult, prepare_current_session, reset};
use crate::durability::platform_streams::{
    LoyaltySnapshot, PageCounts, PageOutcome, PlatformStream, SanctionDecision, StreamPage,
};

/// `01900000-0000-7000-8000-0000000000b1`.
const MUTE_STREAM: [u8; 16] = [
    0x01, 0x90, 0, 0, 0, 0, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 0xb1,
];

fn uuid16(bytes: Vec<u8>) -> Result<[u8; 16], &'static str> {
    bytes.try_into().map_err(|_| "uuid")
}

fn counts(applied: u32, replayed: u32, stale: u32, skipped: u32) -> PageOutcome {
    PageOutcome::Advanced(PageCounts {
        applied,
        replayed,
        stale,
        skipped,
    })
}

#[test]
fn platform_stream_pages_advance_their_cursor_once_and_project_idempotently() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new().stack_size(64 << 20).spawn(|| run(async |admin| {
        reset();
        let h = Harness::create_with_admission_seed(admin, "platform_streams", true, false).await.map_err(debug)?;
        prepare_current_session(&h).await?;
        let (account, character): (Vec<u8>, Vec<u8>) = sqlx::query_as("SELECT uuid_send(account_id),uuid_send(character_id) FROM game_character_roots ORDER BY character_id LIMIT 1")
            .fetch_one(&h.pool).await?;
        let (account, character) = (uuid16(account)?, uuid16(character)?);
        let foreign = id(91);
        let root = &h.root;
        assert_eq!(root.platform_stream_cursor(PlatformStream::Sanctions).await.map_err(debug)?, None);
        assert_eq!(root.platform_stream_cursor(PlatformStream::Loyalty).await.map_err(debug)?, None);

        let mute = SanctionDecision {
            sanction_stream_id: MUTE_STREAM,
            decision_revision: 1,
            account_id: account,
            character_id: Some(character),
            kind: 1,
            reason_code: 9,
            reason_text: "Insulting".into(),
            starts_at_ms: 1_000,
            ends_at_ms: Some(2_000),
            revoked: false,
        };
        let unknown = SanctionDecision { account_id: foreign, character_id: None, kind: 6, ends_at_ms: None, ..mute.clone() };
        let first = StreamPage::Sanctions(vec![mute.clone(), unknown]);
        let s1 = Some("s:1".to_owned());
        assert_eq!(root.apply_platform_stream_page(None, first.clone(), s1.clone()).await.map_err(debug)?, counts(1, 0, 0, 1));
        // A second puller that read the old cursor loses: nothing of its page is kept.
        assert_eq!(root.apply_platform_stream_page(None, first, Some("s:x".into())).await.map_err(debug)?, PageOutcome::Superseded);
        assert_eq!(root.platform_stream_cursor(PlatformStream::Sanctions).await.map_err(debug)?, s1);
        // A replayed record, then a higher revision with a stale one behind it.
        let s2 = Some("s:2".to_owned());
        assert_eq!(root.apply_platform_stream_page(s1, StreamPage::Sanctions(vec![mute.clone()]), s2.clone()).await.map_err(debug)?, counts(0, 1, 0, 0));
        let lifted = SanctionDecision { decision_revision: 2, revoked: true, ..mute.clone() };
        let s3 = Some("s:3".to_owned());
        assert_eq!(root.apply_platform_stream_page(s2, StreamPage::Sanctions(vec![lifted.clone(), mute.clone()]), s3.clone()).await.map_err(debug)?, counts(1, 0, 1, 0));
        // A conflicting revision refuses the whole page and keeps the cursor and the row.
        let conflict = SanctionDecision { reason_text: "Spamming".into(), ..lifted };
        assert!(root.apply_platform_stream_page(s3.clone(), StreamPage::Sanctions(vec![conflict]), Some("s:4".into())).await.is_err());
        assert_eq!(root.platform_stream_cursor(PlatformStream::Sanctions).await.map_err(debug)?, s3.clone());
        // An empty page without a cursor keeps it.
        assert_eq!(root.apply_platform_stream_page(s3.clone(), StreamPage::Sanctions(Vec::new()), None).await.map_err(debug)?, counts(0, 0, 0, 0));
        assert_eq!(root.platform_stream_cursor(PlatformStream::Sanctions).await.map_err(debug)?, s3);
        let stored: (i64, bool, String) = sqlx::query_as("SELECT decision_revision,revoked,reason_text FROM game_account_sanction_projections WHERE sanction_stream_id='01900000-0000-7000-8000-0000000000b1'")
            .fetch_one(&h.pool).await?;
        assert_eq!(stored, (2, true, "Insulting".to_owned()));
        let foreign_rows: i64 = sqlx::query_scalar("SELECT count(*) FROM game_account_sanction_projections WHERE account_id=encode($1,'hex')::uuid")
            .bind(foreign.as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(foreign_rows, 0);

        // Loyalty ranks by Platform's points; a higher revision may lower them.
        let snapshot = LoyaltySnapshot { account_id: account, spent_premium_days: 400, loyalty_points: 410, platform_revision: 3 };
        let l1 = Some("l:1".to_owned());
        let page = StreamPage::Loyalty(vec![snapshot, LoyaltySnapshot { account_id: foreign, ..snapshot }]);
        assert_eq!(root.apply_platform_stream_page(None, page, l1.clone()).await.map_err(debug)?, counts(1, 0, 0, 1));
        let lowered = LoyaltySnapshot { loyalty_points: 380, platform_revision: 4, ..snapshot };
        let l2 = Some("l:2".to_owned());
        assert_eq!(root.apply_platform_stream_page(l1, StreamPage::Loyalty(vec![lowered, lowered, snapshot]), l2.clone()).await.map_err(debug)?, counts(1, 1, 1, 0));
        assert!(root.apply_platform_stream_page(l2.clone(), StreamPage::Loyalty(vec![LoyaltySnapshot { loyalty_points: 1, ..lowered }]), Some("l:3".into())).await.is_err());
        assert_eq!(root.platform_stream_cursor(PlatformStream::Loyalty).await.map_err(debug)?, l2);
        let points: (i64, i64) = sqlx::query_as("SELECT loyalty_points,platform_revision FROM game_account_loyalty_projections WHERE account_id=encode($1,'hex')::uuid")
            .bind(account.as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(points, (380, 4));

        // Only the Game runtime projects; Platform's control role has no route into Game SQL.
        let control_execute: bool = sqlx::query_scalar("SELECT has_function_privilege('oteryn_game_control','game_account_loyalty_project_points(uuid,bigint,bigint,bigint)','EXECUTE') OR has_function_privilege('oteryn_game_control','game_account_sanction_project(uuid,bigint,uuid,uuid,smallint,integer,text,bigint,bigint,boolean)','EXECUTE')")
            .fetch_one(&h.pool).await?;
        assert!(!control_execute);
        h.cleanup().await
    }).map_err(|e|e.to_string()))?.join().map_err(|_|"platform streams panicked")??;
    Ok(())
}
