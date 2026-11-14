//! Frozen enforcement wire through the durable owner and every player-chat gate.
use super::fixture::{Harness, debug, id, join_two, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::durability::platform_streams::enforcement_wire::{
    Code, Command, EnforcementResult, Intent, Profile, RuntimeEffect, State, StreamState,
};
use oteryn_protocol_oteryn::chat::{
    ChatDisposition, ChatIntent, ChatRoom, ChatRoomSet, ChatSpeechMode,
};
use sqlx::{ConnectOptions, Executor};
const START: i64 = 1_700_000_000_000;
const END: i64 = 1_700_003_600_000;
async fn clock(h: &Harness, now: i64) -> TestResult {
    sqlx::query(sqlx::AssertSqlSafe(format!("CREATE OR REPLACE FUNCTION game_platform_enforcement_now_ms() RETURNS BIGINT LANGUAGE sql SET search_path=pg_catalog,public AS $$ SELECT {now}::bigint $$"))).execute(&h.pool).await?;
    Ok(())
}
async fn command(h: &Harness, n: u16) -> TestResult<Command> {
    let mut c = Command::parse(include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../docs/contracts/fixtures/game-enforcement-wire-v1/command-apply.json"
    )))
    .ok_or("command fixture")?;
    let (account, character): (String, String) = sqlx::query_as(
        "SELECT account_id::text,character_id::text FROM game_character_roots LIMIT 1",
    )
    .fetch_one(&h.pool)
    .await?;
    c.account_id = account;
    c.character_id = character;
    c.operation_id = uuid(n);
    Ok(c)
}
fn uuid(n: u16) -> String {
    format!("019a0000-0000-7000-8000-{n:012}")
}
async fn decide_raw(
    h: &Harness,
    a: &ReconciledCharacterAuthority<'_, '_>,
    raw: &[u8],
) -> TestResult<Vec<u8>> {
    super::maintain_ready(&h.root).await?;
    let bytes = h
        .root
        .decide_platform_enforcement(a, &h.node, raw)
        .await
        .map_err(debug)?;
    let result = EnforcementResult::parse(&bytes).ok_or("result codec")?;
    // Assert the actual published key after EVERY decision and replay, not just totals.
    let rows:Vec<Vec<u8>>=sqlx::query_scalar("SELECT result_bytes FROM game_platform_enforcement_result_outbox WHERE operation_id=$1::uuid AND receipt_revision=$2")
        .bind(&result.operation_id).bind(i32::from(result.receipt_revision)).fetch_all(&h.pool).await?;
    assert_eq!(
        rows,
        vec![bytes.clone()],
        "exactly one outbox row for this decision key"
    );
    Ok(bytes)
}
async fn decide(
    h: &Harness,
    a: &ReconciledCharacterAuthority<'_, '_>,
    c: &Command,
) -> TestResult<EnforcementResult> {
    let bytes = decide_raw(h, a, &c.encode().ok_or("encode")?).await?;
    EnforcementResult::parse(&bytes).ok_or_else(|| "result".into())
}
fn stale(r: &EnforcementResult, revision: i64, state: StreamState) {
    assert_eq!(
        (
            r.state,
            r.result_code,
            r.accepted_decision_revision,
            r.stream_state
        ),
        (State::Rejected, Code::StaleRevision, revision, state)
    );
}
async fn decisions(h: &Harness, a: &ReconciledCharacterAuthority<'_, '_>) -> TestResult {
    let mut c = command(h, 101).await?;
    let first = decide_raw(h, a, &c.encode().ok_or("encode")?).await?;
    let applied = EnforcementResult::parse(&first).ok_or("applied")?;
    assert_eq!(
        (
            applied.state,
            applied.result_code,
            applied.runtime_effect,
            applied.recovery_generation
        ),
        (State::Applied, Code::Ok, RuntimeEffect::OnlineApplied, 1)
    );
    assert_eq!(decide_raw(h, a, &c.encode().ok_or("encode")?).await?, first);
    let mut conflict = c.clone();
    conflict.reason_code = Some(8);
    assert_eq!(
        decide_raw(h, a, &conflict.encode().ok_or("encode")?).await?,
        first,
        "a contradictory command may only republish the original receipt"
    );
    sqlx::query("DELETE FROM game_platform_enforcement_result_outbox WHERE operation_id=$1::uuid")
        .bind(&c.operation_id)
        .execute(&h.pool)
        .await?;
    assert_eq!(
        decide_raw(h, a, &conflict.encode().ok_or("encode")?).await?,
        first,
        "conflict survives loss/delivery of the outbox row"
    );
    let codes: Vec<String> = sqlx::query_scalar(
        "SELECT result_code FROM game_platform_enforcement_conflicts WHERE operation_id=$1::uuid",
    )
    .bind(&c.operation_id)
    .fetch_all(&h.pool)
    .await?;
    assert_eq!(codes, vec!["OPERATION_CONFLICT"]);
    assert_eq!(decide_raw(h, a, &c.encode().ok_or("encode")?).await?, first);
    let guards: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_account_guards")
        .fetch_one(&h.pool)
        .await?;
    let mut alien = conflict.clone();
    alien.account_id = uuid(999);
    assert_eq!(
        decide_raw(h, a, &alien.encode().ok_or("encode")?).await?,
        first
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_character_account_guards")
            .fetch_one(&h.pool)
            .await?,
        guards
    );
    c.operation_id = uuid(102);
    c.intent = Intent::Replace;
    c.decision_revision = 3;
    c.prior_decision_revision = Some(1);
    c.expires_at_ms = Some(END + 3_600_000);
    assert_eq!(decide(h, a, &c).await?.state, State::Replaced);
    c.operation_id = uuid(103);
    c.decision_revision = 4;
    stale(&decide(h, a, &c).await?, 3, StreamState::Active);
    for intent in [Intent::Apply, Intent::Revoke, Intent::Expire] {
        let mut old = command(h, 110 + intent as u16).await?;
        old.intent = intent;
        old.decision_revision = 2;
        if intent != Intent::Apply {
            old.effective_at_ms = None;
            old.expires_at_ms = None;
            if intent == Intent::Revoke {
                old.prior_decision_revision = Some(1);
            } else {
                old.reason_code = None;
            }
        }
        stale(&decide(h, a, &old).await?, 3, StreamState::Active);
    }
    c.operation_id = uuid(104);
    c.intent = Intent::Revoke;
    c.decision_revision = 5;
    c.prior_decision_revision = Some(3);
    c.effective_at_ms = None;
    c.expires_at_ms = None;
    assert_eq!(decide(h, a, &c).await?.state, State::Revoked);
    c.operation_id = uuid(105);
    c.intent = Intent::Replace;
    c.decision_revision = 6;
    c.prior_decision_revision = Some(5);
    c.effective_at_ms = Some(START);
    c.expires_at_ms = Some(END);
    assert_eq!(decide(h, a, &c).await?.state, State::Replaced);
    c.operation_id = uuid(106);
    c.intent = Intent::Expire;
    c.prior_decision_revision = None;
    c.effective_at_ms = None;
    c.expires_at_ms = None;
    c.reason_code = None;
    let pending = decide_raw(h, a, &c.encode().ok_or("encode")?).await?;
    let p = EnforcementResult::parse(&pending).ok_or("pending")?;
    assert_eq!(
        (p.state, p.result_code),
        (State::AcceptedPending, Code::PendingExpiry)
    );
    assert_eq!(
        decide_raw(h, a, &c.encode().ok_or("encode")?).await?,
        pending
    );
    clock(h, END).await?;
    let expired = decide_raw(h, a, &c.encode().ok_or("encode")?).await?;
    let e = EnforcementResult::parse(&expired).ok_or("expiry")?;
    assert_eq!((e.state, e.receipt_revision), (State::Expired, 2));
    assert_eq!(
        decide_raw(h, a, &c.encode().ok_or("encode")?).await?,
        expired
    );
    clock(h, START + 500).await?;
    let mut jail = command(h, 107).await?;
    jail.profile = Profile::Jail;
    assert_eq!(
        decide(h, a, &jail).await?.result_code,
        Code::Unsupported,
        "disabled JAIL never executes, including a profile substitution"
    );
    assert_eq!(sqlx::query_scalar::<_,String>("SELECT profile FROM game_platform_enforcement_streams WHERE sanction_stream_id=$1::uuid").bind(&jail.sanction_stream_id).fetch_one(&h.pool).await?,"MUTE");
    let mut foreign = command(h, 108).await?;
    foreign.sanction_stream_id = uuid(208);
    foreign.account_id = uuid(999);
    assert_eq!(
        decide(h, a, &foreign).await?.result_code,
        Code::TargetMismatch
    );
    let mut unknown = command(h, 109).await?;
    unknown.sanction_stream_id = uuid(209);
    unknown.character_id = uuid(999);
    unknown.account_id = uuid(998);
    assert_eq!(
        decide(h, a, &unknown).await?.result_code,
        Code::TargetNotApplicable
    );
    assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_character_account_guards WHERE account_id IN($1::uuid,$2::uuid)").bind(uuid(998)).bind(uuid(999)).fetch_one(&h.pool).await?,0);
    let mut policy = command(h, 120).await?;
    policy.sanction_stream_id = uuid(220);
    policy.policy_revision = 2;
    assert_eq!(
        decide(h, a, &policy).await?.result_code,
        Code::CapabilityConflict
    );
    sqlx::query("UPDATE game_platform_enforcement_configuration SET supported_policy_revision=2")
        .execute(&h.pool)
        .await?;
    policy.operation_id = uuid(121);
    policy.policy_revision = 1;
    assert_eq!(
        decide(h, a, &policy).await?.state,
        State::Applied,
        "older compatible policy remains supported"
    );
    // Malformed but uniquely attributable immutable input is a terminal no-effect decision.
    let malformed = command(h, 130).await?.encode().ok_or("encode")?;
    let malformed = String::from_utf8(malformed)?.replace("\"reason_code\":7", "\"reason_code\":0");
    let refused = decide_raw(h, a, malformed.as_bytes()).await?;
    assert_eq!(
        EnforcementResult::parse(&refused)
            .ok_or("refused")?
            .result_code,
        Code::InternalNonRetryable
    );
    assert_eq!(decide_raw(h, a, malformed.as_bytes()).await?, refused);
    // A now-current new owner cannot retarget a restriction that bound the old account.
    let mut transferred = command(h, 131).await?;
    transferred.sanction_stream_id = policy.sanction_stream_id.clone();
    transferred.intent = Intent::Revoke;
    transferred.decision_revision = 2;
    transferred.prior_decision_revision = Some(1);
    transferred.effective_at_ms = None;
    transferred.expires_at_ms = None;
    let mut tx = h.pool.begin().await?;
    tx.execute("SET LOCAL session_replication_role=replica")
        .await?;
    sqlx::query("UPDATE game_character_roots SET account_id=$2::uuid WHERE character_id=$1::uuid")
        .bind(&transferred.character_id)
        .bind(uuid(997))
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    transferred.account_id = uuid(997);
    assert_eq!(
        decide(h, a, &transferred).await?.result_code,
        Code::TargetMismatch
    );
    assert_eq!(sqlx::query_scalar::<_,i64>("SELECT decision_revision FROM game_platform_enforcement_streams WHERE sanction_stream_id=$1::uuid").bind(&transferred.sanction_stream_id).fetch_one(&h.pool).await?,1);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_character_sanctions")
            .fetch_one(&h.pool)
            .await?,
        0
    );
    Ok(())
}
async fn chat(h: &Harness, a: &ReconciledCharacterAuthority<'_, '_>) -> TestResult {
    use crate::gameplay_transport::{
        capabilities::{BUNDLE_WORLD_OFFERED_CAPABILITIES, SelectedCapabilities},
        connection::FreshAdmissionAuthority,
    };
    use oteryn_protocol_oteryn::chat::private_channel::PrivateChannelIntent;
    let mut deps = Dependencies::new()?;
    let mut item = super::fixture::fence(1)?.item_fence();
    item.character_lease_generation = 2;
    deps.admitted.item_fence = Some(item);
    deps.admitted.continuity.selected_capabilities =
        SelectedCapabilities::select(BUNDLE_WORLD_OFFERED_CAPABILITIES, &[6, 7, 42, 80, 240, 259])
            .ok_or("chat capabilities")?;
    {
        let mut runtime = deps.runtime.lock().await;
        runtime
            .initialize_first_entry_position(deps.actor)
            .map_err(debug)?;
        runtime
            .bind_attacker_lease(
                deps.actor,
                deps.admitted.game_session_id,
                crate::foundation::CharacterLease::new(
                    crate::foundation::CharacterId::decode(&id(41)).map_err(debug)?,
                    2,
                )
                .map_err(debug)?,
            )
            .map_err(debug)?;
    }
    let owner = deps.owner(h, a);
    deps.initialize(&owner).await;
    owner.chat_open(deps.admitted.game_session_id).await;
    assert!(owner.states.chat.knows(deps.admitted.game_session_id));
    owner.chat_party_admit(deps.admitted).await;
    owner
        .public_chat_admit(deps.admitted, ChatRoomSet::default())
        .await;
    owner.private_channel_admit(deps.admitted, true).await;
    // OFF never asks Platform storage, even when the sole semantic holder is busy.
    crate::gameplay_transport::staff::test_clock::set_enforcement(false);
    let busy = h.root.try_issue_semantic_pass().map_err(debug)?;
    assert_eq!(
        owner.staff_mute_wait(deps.admitted.game_session_id).await,
        None
    );
    drop(busy);
    let mut c = command(h, 101).await?;
    decide(h, a, &c).await?;
    // Captured data still must not change main's behavior while the flag is OFF.
    let say = ChatIntent::Say {
        mode: ChatSpeechMode::Say,
        text: "enforcement chat probe".into(),
    };
    assert_eq!(
        owner
            .chat_speak(deps.actor, deps.admitted.game_session_id, 1, &say)
            .await
            .disposition,
        ChatDisposition::Ok
    );
    crate::gameplay_transport::staff::test_clock::set_enforcement(true);
    // The enabled gate retries a busy holder instead of treating contention as mute truth.
    let busy = h.root.try_issue_semantic_pass().map_err(debug)?;
    let pending = owner.staff_mute_wait(deps.admitted.game_session_id);
    let mut pending = std::pin::pin!(pending);
    assert!(
        std::future::poll_fn(|cx| std::task::Poll::Ready(pending.as_mut().poll(cx).is_pending()))
            .await
    );
    drop(busy);
    assert!(pending.await.is_some());
    assert_eq!(
        owner
            .chat_speak(deps.actor, deps.admitted.game_session_id, 2, &say)
            .await
            .disposition,
        ChatDisposition::Muted
    );
    assert_eq!(
        owner
            .guild_chat_send(deps.admitted, "guild probe".into())
            .await
            .disposition,
        ChatDisposition::Muted
    );
    assert_eq!(
        owner
            .party_chat_send(deps.admitted, "party probe".into())
            .await
            .disposition,
        ChatDisposition::Muted
    );
    let public = ChatIntent::Room {
        room: ChatRoom::World,
        text: "public probe".into(),
    };
    assert_eq!(
        owner
            .public_chat_preflight(deps.admitted, 10, &public)
            .await
            .flatten()
            .ok_or("public result")?
            .disposition,
        ChatDisposition::Muted
    );
    let private = PrivateChannelIntent::Say {
        channel_id: 65535,
        text: "private probe".into(),
    };
    assert_eq!(
        owner
            .private_channel_command(deps.admitted, 11, &private)
            .await
            .ok_or("private result")?
            .disposition,
        ChatDisposition::Muted
    );
    assert!(
        !owner.staff_muted(deps.admitted.game_session_id).await,
        "Platform MUTE must not become a staff spell restriction"
    );
    clock(h, END).await?;
    assert_eq!(
        owner
            .chat_speak(deps.actor, deps.admitted.game_session_id, 3, &say)
            .await
            .disposition,
        ChatDisposition::Ok
    );
    assert_eq!(
        owner.staff_mute_wait(deps.admitted.game_session_id).await,
        None
    );
    clock(h, START + 500).await?;
    c.operation_id = uuid(102);
    c.intent = Intent::Revoke;
    c.decision_revision = 2;
    c.prior_decision_revision = Some(1);
    c.effective_at_ms = None;
    c.expires_at_ms = None;
    decide(h, a, &c).await?;
    assert_eq!(
        owner
            .chat_speak(deps.actor, deps.admitted.game_session_id, 4, &say)
            .await
            .disposition,
        ChatDisposition::Ok
    );
    assert_eq!(
        owner.staff_mute_wait(deps.admitted.game_session_id).await,
        None
    );
    Ok(())
}
async fn permissions(h: &Harness, a: &ReconciledCharacterAuthority<'_, '_>) -> TestResult {
    let c = command(h, 101).await?;
    decide(h, a, &c).await?;
    for table in [
        "configuration",
        "cursor",
        "streams",
        "receipts",
        "result_outbox",
        "conflicts",
    ] {
        let denied: bool = sqlx::query_scalar(
            "SELECT has_table_privilege('oteryn_game_control',$1,'INSERT,UPDATE,DELETE')",
        )
        .bind(format!("game_platform_enforcement_{table}"))
        .fetch_one(&h.pool)
        .await?;
        assert!(!denied, "control is read-only: {table}");
    }
    let (runtime,control,public):(bool,bool,bool)=sqlx::query_as("SELECT has_function_privilege('oteryn_game_runtime','game_platform_enforcement_now_ms()','EXECUTE'),has_function_privilege('oteryn_game_control','game_platform_enforcement_now_ms()','EXECUTE'),EXISTS(SELECT 1 FROM pg_proc p CROSS JOIN LATERAL aclexplode(coalesce(p.proacl,acldefault('f',p.proowner))) x WHERE p.oid='game_platform_enforcement_now_ms()'::regprocedure AND x.grantee=0 AND x.privilege_type='EXECUTE')").fetch_one(&h.pool).await?;
    assert_eq!((runtime, control, public), (true, false, false));
    for sql in [
        "UPDATE game_platform_enforcement_receipts SET command_bytes=decode('01','hex') WHERE operation_id=$1::uuid",
        "UPDATE game_platform_enforcement_receipts SET command_digest=repeat('0',64) WHERE operation_id=$1::uuid",
    ] {
        let mut tx = h.pool.begin().await?;
        tx.execute("SET LOCAL ROLE oteryn_game_runtime").await?;
        let error = sqlx::query(sql)
            .bind(&c.operation_id)
            .execute(&mut *tx)
            .await
            .err()
            .ok_or("binding mutation succeeded")?;
        assert!(matches!(error,sqlx::Error::Database(ref e) if e.code().as_deref()==Some("23514")));
        tx.rollback().await?;
    }
    Ok(())
}
async fn race(h: &Harness, a: &ReconciledCharacterAuthority<'_, '_>) -> TestResult {
    let mut c = command(h, 101).await?;
    decide(h, a, &c).await?;
    let options = h
        .root
        .chat_relay_connect_options()
        .ok_or("database options")?;
    let second =
        crate::durability::DurabilityRoot::connect_test_runtime(options.to_url_lossy().as_str())?;
    super::maintain_ready(&second).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let other = second
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    c.intent = Intent::Replace;
    c.prior_decision_revision = Some(1);
    c.decision_revision = 2;
    c.operation_id = uuid(102);
    let mut d = c.clone();
    d.operation_id = uuid(103);
    d.expires_at_ms = Some(END + 1000);
    let left = c.encode().ok_or("encode")?;
    let right = d.encode().ok_or("encode")?;
    let (left, right) = join_two(
        h.root.decide_platform_enforcement(a, &h.node, &left),
        second.decide_platform_enforcement(&other, &h.node, &right),
    )
    .await;
    let left = EnforcementResult::parse(&left.map_err(debug)?).ok_or("left")?;
    let right = EnforcementResult::parse(&right.map_err(debug)?).ok_or("right")?;
    assert_eq!(
        [left.state, right.state]
            .iter()
            .filter(|&&s| s == State::Replaced)
            .count(),
        1
    );
    let loser = if left.state == State::Rejected {
        &left
    } else {
        &right
    };
    stale(loser, 2, StreamState::Active);
    for result in [left, right] {
        assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_platform_enforcement_result_outbox WHERE operation_id=$1::uuid AND receipt_revision=$2").bind(result.operation_id).bind(i32::from(result.receipt_revision)).fetch_one(&h.pool).await?,1);
    }
    Ok(())
}
fn scenario(which: u8) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new().stack_size(64<<20).spawn(move ||->Result<(),String> {
        run(async |admin| {
            let h=Harness::create_with_admission_seed(admin,"platform_enforcement",true,false).await?;
            let result:TestResult=async {
                if which!=4 { prepare_current_session(&h).await?; }
                let seal=h.recovery.seal_current().map_err(debug)?;let a=h.root.open_character_authority(&seal).await.map_err(debug)?;
                clock(&h,START+500).await?;
                let c=command(&h,101).await?;
                assert!(h.root.decide_platform_enforcement(&a,&h.node,&c.encode().ok_or("encode")?).await.is_err(),"default-off deciding configuration");
                sqlx::query("UPDATE game_platform_enforcement_configuration SET enabled=true,supported_policy_revision=1 WHERE singleton").execute(&h.pool).await?;
                match which {0=>decisions(&h,&a).await,1=>chat(&h,&a).await,2=>race(&h,&a).await,3=>permissions(&h,&a).await,4=>{
                    let sessions:i64=sqlx::query_scalar("SELECT count(*) FROM game_durability_reconnect_sessions WHERE session_state IN(1,2)").fetch_one(&h.pool).await?;
                    assert_eq!(sessions,0);
                    assert_eq!(decide(&h,&a,&c).await?.runtime_effect,RuntimeEffect::OnNextAdmission);
                    Ok(())
                },_=>Err("unknown scenario".into())}
            }.await;
            h.cleanup().await?;result
        }).map_err(|e|e.to_string())
    })?.join().map_err(|_|"enforcement test thread failed")?.map_err(Into::into)
}
#[test]
fn platform_enforcement_apply_replace_revoke_expire() -> TestResult {
    scenario(0)
}
#[test]
fn platform_mute_blocks_chat_until_expiry_or_revoke() -> TestResult {
    scenario(1)
}
#[test]
fn platform_enforcement_concurrent_commands_are_serialized() -> TestResult {
    scenario(2)
}
#[test]
fn platform_enforcement_storage_privileges_and_immutable_binding() -> TestResult {
    scenario(3)
}

#[test]
fn platform_enforcement_without_session_is_on_next_admission() -> TestResult {
    scenario(4)
}
