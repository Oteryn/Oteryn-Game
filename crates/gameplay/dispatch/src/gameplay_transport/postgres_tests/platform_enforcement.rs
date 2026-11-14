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
    deps.admitted.continuity.selected_capabilities = SelectedCapabilities::select(
        BUNDLE_WORLD_OFFERED_CAPABILITIES,
        &[6, 7, 36, 42, 80, 240, 259],
    )
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
    use crate::durability::guild_chat_fixture as gf;
    gf::prepare(&h.pool).await?;
    let guild = crate::domain::guild::GuildId::from_bytes(gf::sid(990, 3)).map_err(debug)?;
    gf::initial_guild(
        &h.pool,
        guild,
        id(42),
        "Enforcement Guild",
        crate::domain::CharacterId::from_bytes(id(41)).map_err(debug)?,
        id(40),
    )
    .await?;
    gf::guard(&h.pool).await?;
    let mut party_tx = h.pool.begin().await?;
    sqlx::query("INSERT INTO game_parties(party_id,world_id,leader_character_id,revision,next_seq) VALUES($1::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,1,2)").bind(uuid(990)).bind(id(42).as_slice()).bind(id(41).as_slice()).execute(&mut *party_tx).await?;
    sqlx::query("INSERT INTO game_party_members(character_id,party_id,seq,presence_state,presence_channel_id) VALUES(encode($1,'hex')::uuid,$2::uuid,1,1,encode($3,'hex')::uuid)").bind(id(41).as_slice()).bind(uuid(990)).bind(id(43).as_slice()).execute(&mut *party_tx).await?;
    party_tx.commit().await?;
    use crate::gameplay_transport::spell_premium_coordinator::{
        SpellPremiumCoordinator, purchase_fixture,
    };
    let (consumer, premium_clock, _, _) = purchase_fixture::current(id(40));
    let coordinator =
        SpellPremiumCoordinator::new(consumer, Some(premium_clock), None, Some(h.root.clone()));
    let mut owner = deps.owner(h, a);
    owner.premium_coordinator = Some(&coordinator);
    deps.initialize(&owner).await;
    owner.chat_open(deps.admitted.game_session_id).await;
    assert!(owner.states.chat.knows(deps.admitted.game_session_id));
    owner.chat_party_admit(deps.admitted).await;
    owner
        .public_chat_admit(deps.admitted, ChatRoomSet::default())
        .await;
    owner.private_channel_admit(deps.admitted, true).await;
    owner.guild_chat_admit(deps.admitted).await;
    let ring = crate::chat::relay::RelayKeyRing::new(vec![crate::chat::relay::RelayKey::new(
        1, 0, [1; 32],
    )])
    .map_err(debug)?;
    let relay = crate::gameplay_transport::chat_relay::ChatRelay::new(
        ring,
        id(42),
        h.root.chat_relay_connect_options().ok_or("relay options")?,
    )
    .map_err(debug)?;
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp())*1000000)::bigint")
            .fetch_one(&h.pool)
            .await?;
    relay.force_ready(u64::try_from(now)?);
    owner.states.chat.install_relay(std::sync::Arc::new(relay));
    assert_eq!(
        owner
            .private_channel_command(deps.admitted, 50, &PrivateChannelIntent::Create)
            .await
            .ok_or("create")?
            .disposition,
        ChatDisposition::Ok
    );
    let channel = owner
        .private_channel_list(deps.admitted)
        .await
        .ok_or("private list")?
        .own_channel
        .ok_or("own channel")?;
    assert_eq!(
        owner
            .public_chat_preflight(deps.admitted, 51, &ChatIntent::OpenRoom(ChatRoom::World))
            .await
            .flatten()
            .ok_or("open World")?
            .disposition,
        ChatDisposition::Ok
    );
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
        channel_id: channel,
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
    // A character-scoped delivery must not apply a previous owner's account binding.
    sqlx::query("UPDATE game_platform_enforcement_streams SET account_id=$1::uuid WHERE sanction_stream_id=$2::uuid")
        .bind(uuid(999)).bind(&c.sanction_stream_id).execute(&h.pool).await?;
    assert_eq!(
        owner.staff_mute_wait(deps.admitted.game_session_id).await,
        None
    );
    sqlx::query("UPDATE game_platform_enforcement_streams SET account_id=$1::uuid WHERE sanction_stream_id=$2::uuid")
        .bind(&c.account_id).bind(&c.sanction_stream_id).execute(&h.pool).await?;
    clock(h, END).await?;
    assert_eq!(
        owner
            .chat_speak(deps.actor, deps.admitted.game_session_id, 3, &say)
            .await
            .disposition,
        ChatDisposition::Ok
    );
    owner
        .states
        .test_owner_time
        .store(100000001, std::sync::atomic::Ordering::Relaxed);
    assert_eq!(
        owner
            .guild_chat_send(deps.admitted, "guild restored".into())
            .await
            .disposition,
        ChatDisposition::Ok
    );
    owner
        .states
        .test_owner_time
        .store(110000001, std::sync::atomic::Ordering::Relaxed);
    assert_eq!(
        owner
            .party_chat_send(deps.admitted, "party restored".into())
            .await
            .disposition,
        ChatDisposition::Ok
    );
    owner
        .states
        .test_owner_time
        .store(120000001, std::sync::atomic::Ordering::Relaxed);
    assert_eq!(
        owner
            .public_chat_preflight(deps.admitted, 103, &public)
            .await
            .flatten()
            .ok_or("public restored")?
            .disposition,
        ChatDisposition::Ok
    );
    owner
        .states
        .test_owner_time
        .store(130000001, std::sync::atomic::Ordering::Relaxed);
    assert_eq!(
        owner
            .private_channel_command(deps.admitted, 203, &private)
            .await
            .ok_or("private restored")?
            .disposition,
        ChatDisposition::Ok
    );
    assert_eq!(
        owner.staff_mute_wait(deps.admitted.game_session_id).await,
        None
    );
    owner
        .states
        .test_owner_time
        .store(190_000_001, std::sync::atomic::Ordering::Relaxed);
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
    owner
        .states
        .test_owner_time
        .store(200000001, std::sync::atomic::Ordering::Relaxed);
    assert_eq!(
        owner
            .guild_chat_send(deps.admitted, "guild restored".into())
            .await
            .disposition,
        ChatDisposition::Ok
    );
    owner
        .states
        .test_owner_time
        .store(210000001, std::sync::atomic::Ordering::Relaxed);
    assert_eq!(
        owner
            .party_chat_send(deps.admitted, "party restored".into())
            .await
            .disposition,
        ChatDisposition::Ok
    );
    owner
        .states
        .test_owner_time
        .store(220000001, std::sync::atomic::Ordering::Relaxed);
    assert_eq!(
        owner
            .public_chat_preflight(deps.admitted, 104, &public)
            .await
            .flatten()
            .ok_or("public restored")?
            .disposition,
        ChatDisposition::Ok
    );
    owner
        .states
        .test_owner_time
        .store(230000001, std::sync::atomic::Ordering::Relaxed);
    assert_eq!(
        owner
            .private_channel_command(deps.admitted, 204, &private)
            .await
            .ok_or("private restored")?
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
                },5=>pull_to_ack(&h,&a).await,6=>pending_expiry_restart(&h,&a).await,7=>pending_runtime_restart(&h,&a).await,_=>Err("unknown scenario".into())}
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

struct PgEnforcementStore<'a, 'f, 's> {
    root: &'a crate::durability::DurabilityRoot,
    node: &'a crate::durability::runtime_scope_assignment::NodeIncarnationProof,
    authority: &'a ReconciledCharacterAuthority<'f, 's>,
}
impl crate::platform_streams::enforcement::worker::Store for PgEnforcementStore<'_, '_, '_> {
    async fn transact(
        &self,
        action: crate::platform_streams::enforcement::worker::Action,
    ) -> Result<crate::platform_streams::enforcement::worker::Reply, ()> {
        super::maintain_ready(self.root).await.map_err(|_| ())?;
        self.root
            .enforcement_transport(self.authority, action)
            .await
            .map_err(|_| ())
    }
    async fn decide(&self, raw: Vec<u8>) -> Result<Vec<u8>, ()> {
        super::maintain_ready(self.root).await.map_err(|_| ())?;
        self.root
            .decide_platform_enforcement(self.authority, self.node, &raw)
            .await
            .map_err(|_| ())
    }
}
async fn pull_to_ack(h: &Harness, a: &ReconciledCharacterAuthority<'_, '_>) -> TestResult {
    use crate::platform_streams::enforcement::{
        CommandPage, Message,
        transport::Clients,
        worker::{self, Step},
    };
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let apply = command(h, 101).await?;
    let apply_raw = apply.encode().ok_or("encode apply")?;
    let mut revoke = apply.clone();
    revoke.operation_id = uuid(102);
    revoke.intent = Intent::Revoke;
    revoke.decision_revision = 2;
    revoke.prior_decision_revision = Some(1);
    revoke.effective_at_ms = None;
    revoke.expires_at_ms = None;
    let page = |commands, next: Option<&str>| {
        Message::Page(CommandPage {
            schema: "oteryn.enforcement_command_stream_page.v1".into(),
            items: commands,
            next_cursor: next.map(str::to_owned),
        })
        .encode()
        .ok_or("page encode")
    };
    let ack = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../docs/contracts/fixtures/game-enforcement-wire-v1/result-ack.json"
    ))
    .to_vec();
    let replies = vec![
        page(vec![apply], Some("v1:1"))?,
        ack.clone(),
        page(vec![], None)?,
        page(vec![revoke], Some("v1:2"))?,
        ack,
    ];
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let clients = Clients::local(format!("http://{}", listener.local_addr()?)).map_err(debug)?;
    let peer = tokio::spawn(async move {
        let mut requests = Vec::new();
        for body in replies {
            let (mut socket, _) = listener.accept().await.map_err(|e| e.to_string())?;
            let mut raw = Vec::new();
            let mut buffer = [0u8; 4096];
            let body_start = loop {
                let n = socket.read(&mut buffer).await.map_err(|e| e.to_string())?;
                if n == 0 {
                    return Err("request disconnected".to_string());
                }
                raw.extend_from_slice(&buffer[..n]);
                if let Some(end) = raw.windows(4).position(|bytes| bytes == b"\r\n\r\n") {
                    let head = String::from_utf8_lossy(&raw[..end]);
                    let length = head
                        .lines()
                        .find_map(|line| {
                            line.to_ascii_lowercase()
                                .strip_prefix("content-length:")
                                .and_then(|s| s.trim().parse::<usize>().ok())
                        })
                        .ok_or("request content length")?;
                    if raw.len() >= end + 4 + length {
                        break end + 4;
                    }
                }
            };
            requests.push(raw[body_start..].to_vec());
            let head = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            socket
                .write_all(head.as_bytes())
                .await
                .map_err(|e| e.to_string())?;
            socket.write_all(&body).await.map_err(|e| e.to_string())?;
        }
        Ok::<_, String>(requests)
    });
    let first = PgEnforcementStore { root: &h.root, node: &h.node, authority: a };
    assert_eq!(
        worker::pull(&first, &clients).await.map_err(debug)?,
        Step::More
    );
    let count = |query: &'static str| sqlx::query_scalar::<_, i64>(query).fetch_one(&h.pool);
    assert_eq!(
        count("SELECT count(*) FROM game_platform_enforcement_receipts").await?,
        1
    );
    assert_eq!(
        count(
            "SELECT count(*) FROM game_platform_enforcement_result_outbox WHERE NOT acknowledged"
        )
        .await?,
        1
    );
    let stored: Vec<u8> =
        sqlx::query_scalar("SELECT result_bytes FROM game_platform_enforcement_receipts")
            .fetch_one(&h.pool)
            .await?;
    let mut expected = EnforcementResult::parse(include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../docs/contracts/fixtures/game-enforcement-wire-v1/result-applied.json"
    )))
    .ok_or("result fixture")?;
    expected.command_digest = crate::platform_streams::enforcement::digest(&apply_raw);
    assert_eq!(stored, expected.encode().ok_or("result encode")?);
    assert_eq!(
        worker::publish(&first, &clients, START + 500)
            .await
            .map_err(debug)?,
        Step::More
    );
    assert_eq!(
        count("SELECT count(*) FROM game_platform_enforcement_result_outbox WHERE acknowledged")
            .await?,
        1
    );
    let mut immutable = h.pool.begin().await?;
    immutable
        .execute("SET LOCAL ROLE oteryn_game_runtime")
        .await?;
    let mut changed = stored.clone();
    changed.push(b' ');
    let error = sqlx::query("UPDATE game_platform_enforcement_result_outbox SET result_bytes=$1")
        .bind(changed)
        .execute(&mut *immutable)
        .await
        .err()
        .ok_or("result history changed")?;
    assert!(matches!(error,sqlx::Error::Database(ref e) if e.code().as_deref()==Some("23514")));
    immutable.rollback().await?;
    // Restart the task over the same durable cursor and acknowledgements.
    let restarted = PgEnforcementStore { root: &h.root, node: &h.node, authority: a };
    assert_eq!(
        worker::pull(&restarted, &clients).await.map_err(debug)?,
        Step::Idle
    );
    assert_eq!(
        worker::publish(&restarted, &clients, START + 500)
            .await
            .map_err(debug)?,
        Step::Idle
    );
    assert_eq!(
        count("SELECT count(*) FROM game_platform_enforcement_receipts").await?,
        1
    );
    // Crash after the next decision/cursor commits but before any result publication.
    assert_eq!(
        worker::pull(&restarted, &clients).await.map_err(debug)?,
        Step::More
    );
    assert_eq!(
        count(
            "SELECT count(*) FROM game_platform_enforcement_result_outbox WHERE NOT acknowledged"
        )
        .await?,
        1
    );
    let after_crash = PgEnforcementStore { root: &h.root, node: &h.node, authority: a };
    assert_eq!(
        worker::publish(&after_crash, &clients, START + 500)
            .await
            .map_err(debug)?,
        Step::More
    );
    assert_eq!(
        count("SELECT count(*) FROM game_platform_enforcement_result_outbox WHERE acknowledged")
            .await?,
        2
    );
    let requests = peer.await.map_err(debug)?.map_err(debug)?;
    assert_eq!(requests.len(), 5);
    assert_eq!(requests[1], stored);
    let Some(Message::Request(request)) = Message::parse(&requests[2]) else {
        return Err("restart did not use stream request".into());
    };
    assert_eq!(request.after.as_deref(), Some("v1:1"));
    let latest: Vec<u8> = sqlx::query_scalar(
        "SELECT result_bytes FROM game_platform_enforcement_receipts WHERE operation_id=$1::uuid",
    )
    .bind(uuid(102))
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(requests[4], latest);
    Ok(())
}
#[test]
fn pull_to_ack_end_to_end() -> TestResult {
    scenario(5)
}


#[derive(Default)]
struct ResultPeer(std::sync::Mutex<Vec<Vec<u8>>>);
impl crate::platform_streams::enforcement::worker::Peer for ResultPeer {
    async fn request(&self, route: crate::platform_streams::enforcement::transport::Route, raw: Vec<u8>) -> Result<Vec<u8>, crate::platform_streams::enforcement::transport::Failure> {
        use crate::platform_streams::enforcement::transport::{Route,Failure};
        if !matches!(route,Route::Results) { return Err(Failure::Defect); }
        self.0.lock().map_err(|_|Failure::Defect)?.push(raw);
        Ok(include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../docs/contracts/fixtures/game-enforcement-wire-v1/result-ack.json")).to_vec())
    }
}
async fn pending_expiry_restart(h: &Harness, a: &ReconciledCharacterAuthority<'_, '_>) -> TestResult {
    use crate::platform_streams::enforcement::worker::{Publisher,Step};
    let mut c=command(h,501).await?;
    decide(h,a,&c).await?;
    c.operation_id=uuid(502); c.intent=Intent::Expire; c.effective_at_ms=None; c.expires_at_ms=None; c.reason_code=None;
    let pending=decide(h,a,&c).await?;
    assert_eq!(pending.state,State::AcceptedPending);
    assert_eq!(pending.result_code,Code::PendingExpiry);
    let peer=ResultPeer::default();
    let store=PgEnforcementStore {root:&h.root,node:&h.node,authority:a};
    let mut publisher=Publisher::default();
    // Acknowledge both APPLY and the early EXPIRE. Neither A1 nor A4 supplies it again.
    assert_eq!(publisher.step(&store,&peer,START+500).await.map_err(debug)?,Step::More);
    assert_eq!(publisher.step(&store,&peer,START+500).await.map_err(debug)?,Step::More);
    assert_eq!(publisher.step(&store,&peer,START+500).await.map_err(debug)?,Step::Idle);
    clock(h,END-1).await?;
    assert_eq!(Publisher::default().step(&store,&peer,END-1).await.map_err(debug)?,Step::Idle);
    clock(h,END).await?;
    let options=h.root.chat_relay_connect_options().ok_or("options")?;
    let restarted=crate::durability::DurabilityRoot::connect_test_runtime(options.to_url_lossy().as_str())?;
    super::maintain_ready(&restarted).await?;
    let seal=h.recovery.seal_current().map_err(debug)?;
    let authority=restarted.open_character_authority(&seal).await.map_err(debug)?;
    let store=PgEnforcementStore {root:&restarted,node:&h.node,authority:&authority};
    assert_eq!(Publisher::default().step(&store,&peer,END).await.map_err(debug)?,Step::More);
    assert_eq!(Publisher::default().step(&store,&peer,END).await.map_err(debug)?,Step::Idle);
    let terminal = {
        let posted=peer.0.lock().map_err(|_|"peer")?;
        assert_eq!(posted.len(),3);
        EnforcementResult::parse(&posted[2]).ok_or("terminal")?
    };
    assert_eq!((terminal.state,terminal.result_code,terminal.receipt_revision),(State::Expired,Code::Ok,2));
    assert_eq!(terminal.command_digest,pending.command_digest);
    let rows:(i64,i64)=sqlx::query_as("SELECT count(*),count(*) FILTER(WHERE acknowledged) FROM game_platform_enforcement_result_outbox WHERE operation_id=$1::uuid").bind(&c.operation_id).fetch_one(&h.pool).await?;
    assert_eq!(rows,(2,2));
    let due:Option<i64>=sqlx::query_scalar("SELECT pending_retry_at_ms FROM game_platform_enforcement_receipts WHERE operation_id=$1::uuid").bind(&c.operation_id).fetch_one(&h.pool).await?;
    assert_eq!(due,None);
    Ok(())
}
async fn pending_runtime_restart(h: &Harness, a: &ReconciledCharacterAuthority<'_, '_>) -> TestResult {
    use crate::platform_streams::enforcement::worker::{Publisher,Step};
    // EXPIRE naturally emits PENDING_EXPIRY. Seed the other contract-permitted
    // pending reason against real accepted streams to exercise the whole class.
    for (n,intent) in [(610,Intent::Apply),(620,Intent::Replace),(630,Intent::Revoke),(640,Intent::Expire)] {
        clock(h,START+500).await?;
        let mut c=command(h,n).await?; c.sanction_stream_id=uuid(n+1);
        c.effective_at_ms=Some(START+10_000);
        let mut accepted=decide(h,a,&c).await?;
        if intent!=Intent::Apply {
            c.operation_id=uuid(n+2); c.intent=intent;
            if intent==Intent::Expire { c.prior_decision_revision=None; c.reason_code=None; } else { c.decision_revision=2; c.prior_decision_revision=Some(1); }
            if matches!(intent,Intent::Revoke|Intent::Expire) {c.effective_at_ms=None;c.expires_at_ms=None;}
            accepted=decide(h,a,&c).await?;
        }
        accepted.state=State::AcceptedPending; accepted.result_code=Code::PendingRuntime; accepted.runtime_effect=RuntimeEffect::None;
        let raw=accepted.encode().ok_or("pending runtime")?;
        sqlx::query("UPDATE game_platform_enforcement_receipts SET result_bytes=$2,pending_retry_at_ms=$3 WHERE operation_id=$1::uuid").bind(&c.operation_id).bind(&raw).bind(START+10_000).execute(&h.pool).await?;
        sqlx::query("DELETE FROM game_platform_enforcement_result_outbox WHERE operation_id=$1::uuid").bind(&c.operation_id).execute(&h.pool).await?;
        sqlx::query("INSERT INTO game_platform_enforcement_result_outbox(operation_id,command_digest,receipt_revision,result_bytes,acknowledged) VALUES($1::uuid,$2,1,$3,true)").bind(&c.operation_id).bind(&accepted.command_digest).bind(&raw).execute(&h.pool).await?;
        let due=if intent==Intent::Expire {END} else {START+10_000};
        clock(h,due).await?;
        let peer=ResultPeer::default();
        let store=PgEnforcementStore {root:&h.root,node:&h.node,authority:a};
        let mut publisher=Publisher::default();
        for _ in 0..2 { publisher.step(&store,&peer,due).await.map_err(debug)?; }
        assert_eq!(publisher.step(&store,&peer,due).await.map_err(debug)?,Step::Idle);
        let result = {
            let posted=peer.0.lock().map_err(|_|"peer")?;
            let parsed=posted.iter().map(|b|EnforcementResult::parse(b).ok_or("invalid publication")).collect::<Result<Vec<_>,_>>()?;
            parsed.into_iter().find(|r|r.operation_id==c.operation_id).ok_or("pending result not published")?
        };
        let expected=match intent {Intent::Apply=>State::Applied,Intent::Replace=>State::Replaced,Intent::Revoke=>State::Revoked,Intent::Expire=>State::Expired};
        assert_eq!((result.state,result.receipt_revision),(expected,2));
        assert_eq!(decide(h,a,&c).await?.receipt_revision,2,"terminal replay stays immutable");
    }
    Ok(())
}
#[test]
fn platform_enforcement_pending_expiry_finishes_after_restart() -> TestResult { scenario(6) }
#[test]
fn platform_enforcement_pending_runtime_finishes_all_intents() -> TestResult { scenario(7) }
