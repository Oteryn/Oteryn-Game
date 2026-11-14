//! Frozen enforcement wire through the durable owner and real chat gate.
use super::fixture::{Harness, debug, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::durability::platform_streams::enforcement_wire::{
    Code, Command, EnforcementResult, Intent, Profile, State, StreamState,
};
use oteryn_protocol_oteryn::chat::{ChatDisposition, ChatIntent, ChatSpeechMode};

async fn clock(h: &Harness, now: i64) -> TestResult {
    sqlx::query(sqlx::AssertSqlSafe(format!("CREATE OR REPLACE FUNCTION game_platform_enforcement_now_ms() RETURNS BIGINT LANGUAGE sql AS $$ SELECT {now}::bigint $$"))).execute(&h.pool).await?;
    Ok(())
}
async fn command(h: &Harness, n: u16) -> TestResult<Command> {
    let raw = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../docs/contracts/fixtures/game-enforcement-wire-v1/command-apply.json"
    ));
    let mut c = Command::parse(raw).ok_or("command fixture")?;
    let (account, character): (String, String) = sqlx::query_as(
        "SELECT account_id::text,character_id::text FROM game_character_roots LIMIT 1",
    )
    .fetch_one(&h.pool)
    .await?;
    c.account_id = account;
    c.character_id = character;
    c.operation_id = format!("019a0000-0000-7000-8000-{n:012}");
    Ok(c)
}
fn threaded(chat: bool) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let result=std::thread::Builder::new().stack_size(64<<20).spawn(move ||->Result<(),String>{
        run(async |admin| {
            let h=Harness::create_with_admission_seed(admin,"platform_enforcement",true,false).await?;
            let result:TestResult=async {
                prepare_current_session(&h).await?;
                let seal=h.recovery.seal_current().map_err(debug)?;
                let authority=h.root.open_character_authority(&seal).await.map_err(debug)?;
                let mut c=command(&h,101).await?;
                clock(&h,1_700_000_000_500).await?;
                let raw=c.encode().ok_or("command encode")?;
                assert!(h.root.decide_platform_enforcement(&authority,&h.node,&raw).await.is_err(),"flag defaults off");
                sqlx::query("UPDATE game_platform_enforcement_configuration SET enabled=true,supported_policy_revision=1 WHERE singleton").execute(&h.pool).await?;
                let decide=async |c:&Command|->TestResult<Vec<u8>> {
                    super::maintain_ready(&h.root).await?;
                    Ok(h.root.decide_platform_enforcement(&authority,&h.node,&c.encode().ok_or("encode")?).await.map_err(debug)?)
                };
                let first=decide(&c).await?;
                let applied=EnforcementResult::parse(&first).ok_or("result")?;
                assert_eq!((applied.state,applied.result_code),(State::Applied,Code::Ok));
                assert_eq!(applied.recovery_generation,1);
                assert_eq!(decide(&c).await?,first);
                if chat {
                    let deps=Dependencies::new()?;let owner=deps.owner(&h,&authority);
                    owner.chat_open(deps.admitted.game_session_id).await;
                    assert!(owner.states.chat.knows(deps.admitted.game_session_id),"real chat session is admitted");
                    let say=ChatIntent::Say{mode:ChatSpeechMode::Say,text:"enforcement chat probe".into()};
                    assert_eq!(owner.chat_speak(deps.actor,deps.admitted.game_session_id,1,&say).await.disposition,ChatDisposition::Muted);
                    clock(&h,1_700_003_600_000).await?;
                    assert_eq!(owner.chat_speak(deps.actor,deps.admitted.game_session_id,2,&say).await.disposition,ChatDisposition::Ok);
                    clock(&h,1_700_000_000_500).await?;
                    c.operation_id=command(&h,102).await?.operation_id;c.intent=Intent::Revoke;c.decision_revision=2;c.prior_decision_revision=Some(1);c.effective_at_ms=None;c.expires_at_ms=None;
                    decide(&c).await?;
                    assert_eq!(owner.chat_speak(deps.actor,deps.admitted.game_session_id,3,&say).await.disposition,ChatDisposition::Ok);
                } else {
                    let mut conflict=c.clone();conflict.reason_code=Some(8);
                    let conflict_bytes=decide(&conflict).await?;
                    assert_eq!(EnforcementResult::parse(&conflict_bytes).ok_or("conflict")?.result_code,Code::OperationConflict);
                    assert_eq!(decide(&conflict).await?,conflict_bytes);
                    assert_eq!(decide(&c).await?,first,"conflict cannot overwrite original receipt");
                    c.operation_id=command(&h,102).await?.operation_id;c.intent=Intent::Replace;c.decision_revision=3;c.prior_decision_revision=Some(1);c.expires_at_ms=Some(1_700_007_200_000);
                    assert_eq!(EnforcementResult::parse(&decide(&c).await?).ok_or("replace")?.state,State::Replaced);
                    c.operation_id=command(&h,103).await?.operation_id;c.decision_revision=4;
                    let stale=EnforcementResult::parse(&decide(&c).await?).ok_or("stale")?;
                    assert_eq!((stale.result_code,stale.accepted_decision_revision,stale.stream_state),(Code::StaleRevision,3,StreamState::Active));
                    c.operation_id=command(&h,104).await?.operation_id;c.intent=Intent::Revoke;c.decision_revision=5;c.prior_decision_revision=Some(3);c.effective_at_ms=None;c.expires_at_ms=None;
                    assert_eq!(EnforcementResult::parse(&decide(&c).await?).ok_or("revoke")?.state,State::Revoked);
                    c.operation_id=command(&h,105).await?.operation_id;c.intent=Intent::Replace;c.decision_revision=6;c.prior_decision_revision=Some(5);c.effective_at_ms=Some(1_700_000_000_000);c.expires_at_ms=Some(1_700_003_600_000);
                    decide(&c).await?;
                    c.operation_id=command(&h,106).await?.operation_id;c.intent=Intent::Expire;c.prior_decision_revision=None;c.effective_at_ms=None;c.expires_at_ms=None;c.reason_code=None;
                    let pending=decide(&c).await?;let p=EnforcementResult::parse(&pending).ok_or("pending")?;
                    assert_eq!((p.state,p.result_code),(State::AcceptedPending,Code::PendingExpiry));
                    assert_eq!(decide(&c).await?,pending);
                    clock(&h,1_700_003_600_000).await?;
                    let expired=decide(&c).await?;let e=EnforcementResult::parse(&expired).ok_or("expiry")?;
                    assert_eq!((e.state,e.receipt_revision),(State::Expired,2));assert_eq!(decide(&c).await?,expired);
                    let mut jail=command(&h,107).await?;jail.profile=Profile::Jail;jail.sanction_stream_id="019a0000-0000-7000-8000-000000000207".into();
                    assert_eq!(EnforcementResult::parse(&decide(&jail).await?).ok_or("jail")?.result_code,Code::Unsupported);
                    let mut foreign=command(&h,108).await?;foreign.sanction_stream_id="019a0000-0000-7000-8000-000000000208".into();foreign.account_id="019a0000-0000-7000-8000-000000000999".into();
                    assert_eq!(EnforcementResult::parse(&decide(&foreign).await?).ok_or("foreign")?.result_code,Code::TargetMismatch);
                    let unknown=Command {operation_id:command(&h,109).await?.operation_id,character_id:"019a0000-0000-7000-8000-000000000999".into(),sanction_stream_id:"019a0000-0000-7000-8000-000000000209".into(),..command(&h,109).await?};
                    assert_eq!(EnforcementResult::parse(&decide(&unknown).await?).ok_or("unknown")?.result_code,Code::TargetNotApplicable);
                    assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_platform_enforcement_result_outbox").fetch_one(&h.pool).await?,11);
                    assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_platform_enforcement_receipts").fetch_one(&h.pool).await?,9);
                    assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_character_sanctions").fetch_one(&h.pool).await?,0);
                    let orphan:i64=sqlx::query_scalar("SELECT count(*) FROM game_platform_enforcement_receipts r WHERE NOT EXISTS(SELECT 1 FROM game_platform_enforcement_result_outbox o WHERE o.operation_id=r.operation_id AND o.command_digest=r.command_digest AND o.receipt_revision=r.receipt_revision AND o.result_bytes=r.result_bytes)").fetch_one(&h.pool).await?;
                    assert_eq!(orphan,0);
                    let duplicates:i64=sqlx::query_scalar("SELECT count(*) FROM (SELECT operation_id,command_digest,receipt_revision FROM game_platform_enforcement_result_outbox GROUP BY 1,2,3 HAVING count(*)<>1) q").fetch_one(&h.pool).await?;
                    assert_eq!(duplicates,0);
                }
                Ok(())
            }.await;
            h.cleanup().await?;result
        }).map_err(|error|error.to_string())
    })?.join().map_err(|_|"enforcement test thread failed")?;
    result.map_err(Into::into)
}
#[test]
fn platform_enforcement_apply_replace_revoke_expire() -> TestResult {
    threaded(false)
}
#[test]
fn platform_mute_blocks_chat_until_expiry_or_revoke() -> TestResult {
    threaded(true)
}
