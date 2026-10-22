use super::*;
const MIGRATION: &str = include_str!("../../../migrations/0132_guild_core.sql");
// Retained source-qualified fixtures use the exact installed SQL, never a copied schema.
use std::sync::LazyLock;
pub(super) static GUILD_TABLES_PROPOSAL: LazyLock<&str> = LazyLock::new(|| {
    MIGRATION
        .split("-- GUILD TABLES BEGIN")
        .nth(1)
        .unwrap()
        .split("-- GUILD GUARDS BEGIN")
        .next()
        .unwrap()
});
pub(super) static GUILD_GUARDS_PROPOSAL: LazyLock<&str> = LazyLock::new(|| {
    MIGRATION
        .split("-- GUILD GUARDS BEGIN")
        .nth(1)
        .unwrap()
        .split("-- GUILD GRANTS BEGIN")
        .next()
        .unwrap()
});
pub(super) fn sid(number: u32, kind: u8) -> [u8; 16] {
    let mut bytes = [0_u8; 16];
    bytes[0] = 0xa1;
    bytes[1] = kind;
    bytes[6] = 0x70;
    bytes[8] = 0x80;
    bytes[12..].copy_from_slice(&number.to_be_bytes());
    bytes
}
pub(super) fn character(n: u32) -> CharacterId {
    CharacterId::from_bytes(if n == 0 { id(CHARACTER) } else { sid(n, 1) }).unwrap()
}
pub(super) fn account(n: u32) -> [u8; 16] {
    if n == 0 { id(40) } else { sid(n, 2) }
}
pub(super) fn gid(n: u32) -> GuildId {
    GuildId::from_bytes(sid(n, 3)).unwrap()
}
pub(super) fn request(n: u32, guild: u32, op: GuildCommand) -> GuildRequest {
    GuildRequest {
        identity: GuildOperationIdentity {
            occurrence: sid(n, 10),
            transaction_id: sid(n, 11),
            event_id: sid(n, 12),
            occurred_at_unix_ms: 1_790_000_000_000,
        },
        command: CommandRef::new(
            GameSessionId::decode(&id(fixture::SESSION)).unwrap(),
            CommandId::new(u64::from(n)).unwrap(),
        ),
        guild_id: gid(guild),
        operation: op,
    }
}
pub(super) fn result(value: GuildOutcome) -> GuildReceipt {
    match value {
        GuildOutcome::Committed(r) | GuildOutcome::Replayed(r) => r,
    }
}
pub(super) fn name(mut n: u32) -> String {
    let mut letters = String::new();
    loop {
        letters.push(char::from(b'A' + (n % 26) as u8));
        n /= 26;
        if n == 0 {
            break;
        }
    }
    format!("Guild Fixture {letters}")
}
pub(super) fn uuid(bytes: [u8; 16]) -> String {
    let h = bytes.iter().map(|b| format!("{b:02x}")).collect::<String>();
    format!(
        "{}-{}-{}-{}-{}",
        &h[..8],
        &h[8..12],
        &h[12..16],
        &h[16..20],
        &h[20..]
    )
}

/// Complete historical Character creation fixture, using the native decoder/binding, not a
/// manufactured authority capability. The real recovery seal/full integrity check must pass.
pub(super) async fn historical_character(h: &Harness, n: u32, world: [u8; 16]) -> TestResult {
    historical_character_account(h, n, world, account(n)).await
}

pub(super) async fn historical_character_account(
    h: &Harness,
    n: u32,
    world: [u8; 16],
    owner: [u8; 16],
) -> TestResult {
    sqlx::query("INSERT INTO game_character_account_guards VALUES(encode($1,'hex')::uuid) ON CONFLICT DO NOTHING")
        .bind(owner.as_slice())
        .execute(&h.pool)
        .await?;
    sqlx::query("INSERT INTO game_character_roots(character_id,account_id,world_id,lifecycle,character_revision,profile_revision,ruleset_revision,content_revision,starter_template_revision,name) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,1,1,'profile-1','ruleset-1','content-1','starter-1',$4)")
        .bind(character(n).as_bytes().as_slice()).bind(owner.as_slice()).bind(world.as_slice()).bind(name(n)).execute(&h.pool).await?;
    let operation = sid(n, 30);
    let decision = sid(n, 31);
    let raw = serde_json::to_vec(
        &serde_json::json!({"contract_version":2,"variant":"OPERATOR_CONTROL_PLANE_BOOTSTRAP",
        "issuer_authority":"OTERYN_PLATFORM_CHARACTER_AUTHORITY","issuer_decision_id":uuid(decision),
        "source_revision":(u64::from(n)+100).to_string(),"operation_id":uuid(operation),"operation":"INITIAL_CHARACTER_BOOTSTRAP",
        "account_id":uuid(owner),"target_world_id":uuid(world),"requested_name":name(n),
        "interpretation_context":{"profile_revision":"profile-1","ruleset_revision":"ruleset-1","content_revision":"content-1","starter_template_revision":"starter-1"},
        "issued_at_source":"1","expires_at_source":"120","audience":"OTERYN_GAME_CHARACTER_AUTHORITY"}),
    )?;
    let intent = crate::character_bootstrap_intent::decode_producer_response(&raw, operation)
        .map_err(debug)?;
    sqlx::query("INSERT INTO game_character_operation_receipts(operation_id,command_binding,account_id,character_id,world_id,character_revision,event_id,occurred_at,server_build_id,transaction_id,issuer_decision_id,intent_source_revision,issued_at_source,expires_at_source) VALUES(encode($1,'hex')::uuid,$2,encode($3,'hex')::uuid,encode($4,'hex')::uuid,encode($5,'hex')::uuid,1,encode($6,'hex')::uuid,1,'test/1',encode($7,'hex')::uuid,encode($8,'hex')::uuid,$9,1,120)")
        .bind(operation.as_slice()).bind(intent.binding()).bind(owner.as_slice()).bind(character(n).as_bytes().as_slice()).bind(world.as_slice())
        .bind(sid(n,32).as_slice()).bind(sid(n,33).as_slice()).bind(decision.as_slice()).bind(i64::from(n)+100).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_character_bootstrap_intent_floors(issuer_scope,source_revision,issuer_decision_id,intent_binding) SELECT 1,intent_source_revision,issuer_decision_id,command_binding FROM game_character_operation_receipts ORDER BY intent_source_revision DESC LIMIT 1 ON CONFLICT(issuer_scope) DO UPDATE SET source_revision=EXCLUDED.source_revision,issuer_decision_id=EXCLUDED.issuer_decision_id,intent_binding=EXCLUDED.intent_binding")
        .execute(&h.pool).await?;
    Ok(())
}
pub(super) async fn initial_guild(h: &Harness, g: u32, leader: u32) -> TestResult {
    sqlx::query("INSERT INTO game_guilds(guild_id,world_id,name,state,formation_deadline,vice_deficit_since,revision,created_at) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,2,9999999999999,1,1,1)")
        .bind(gid(g).as_bytes().as_slice()).bind(id(WORLD).as_slice()).bind(name(g)).execute(&h.pool).await?;
    for (rank, name) in [
        (1_i16, "Leader"),
        (2, "Vice Leader"),
        (3, "Member"),
        (4, "Visitor"),
    ] {
        sqlx::query("INSERT INTO game_guild_ranks(guild_id,level,name) VALUES(encode($1,'hex')::uuid,$2,$3)")
            .bind(gid(g).as_bytes().as_slice()).bind(rank).bind(name).execute(&h.pool).await?;
    }
    initial_member(h, g, leader, 1).await
}
pub(super) async fn initial_member(h: &Harness, g: u32, c: u32, rank: i16) -> TestResult {
    sqlx::query("INSERT INTO game_guild_members(character_id,guild_id,rank_level,title,joined_at) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,'',1)")
        .bind(character(c).as_bytes().as_slice()).bind(gid(g).as_bytes().as_slice()).bind(rank).execute(&h.pool).await?;
    if rank <= 2 {
        sqlx::query("INSERT INTO game_account_guild_leadership VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid)")
        .bind(account(c).as_slice()).bind(gid(g).as_bytes().as_slice()).bind(character(c).as_bytes().as_slice()).execute(&h.pool).await?;
    }
    Ok(())
}
pub(super) async fn initial_invitation(
    h: &Harness,
    g: u32,
    target: u32,
    inviter: u32,
    expired: bool,
) -> TestResult {
    sqlx::query("INSERT INTO game_guild_invitations VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,CASE WHEN $4 THEN 1 ELSE floor(extract(epoch FROM statement_timestamp())*1000)::bigint+2592000000 END)")
        .bind(gid(g).as_bytes().as_slice()).bind(character(target).as_bytes().as_slice()).bind(character(inviter).as_bytes().as_slice()).bind(expired).execute(&h.pool).await?;
    Ok(())
}

/// A second actual native session/guard fixture in the same admitted Channel, without changing
/// the first actor. These are private DB setup rows, not an authority constructor: every call
/// below still obtains a genuine independently reconciled Root capability and current node.
pub(super) async fn historical_session(
    h: &Harness,
    n: u32,
) -> TestResult<CurrentCharacterItemFence> {
    let session = sid(n, 20);
    sqlx::query("INSERT INTO game_durability_reconnect_sessions(game_session_id,account_id,character_id,world_id,runtime_scope_kind,runtime_scope_world_id,runtime_scope_channel_id,control_loss_epoch,original_grace_deadline,predecessor_generation,character_lease_generation,scope_ownership_generation,current_generation,current_transport_ref,session_state) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,1,encode($4,'hex')::uuid,encode($5,'hex')::uuid,1,999999,1,1,1,1,$6,1)")
        .bind(session.as_slice()).bind(account(n).as_slice()).bind(character(n).as_bytes().as_slice()).bind(id(WORLD).as_slice()).bind(id(fixture::CHANNEL).as_slice()).bind(sid(n,21).as_slice()).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_durability_admission_account_guards VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,1,'test',1,'account-current',1,0,'{}')")
        .bind(account(n).as_slice()).bind(character(n).as_bytes().as_slice()).bind(session.as_slice()).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_durability_admission_character_guards VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,true,1,encode($4,'hex')::uuid,1,'test',1,'character-current',1,0,'{}')")
        .bind(character(n).as_bytes().as_slice()).bind(account(n).as_slice()).bind(id(WORLD).as_slice()).bind(session.as_slice()).execute(&h.pool).await?;
    Ok(CurrentCharacterItemFence {
        character_id: character(n),
        game_session_id: GameSessionId::decode(&session)?,
        ..fence()?
    })
}
pub(super) async fn prepare(h: &Harness) -> TestResult {
    let version: String = sqlx::query_scalar("SHOW server_version_num")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(version, "170006");
    eprintln!("GUILD-NATIVE-PG server_version_num={version}");
    // Only this isolated historical snapshot fixture is rebuilt; new real-path tests below
    // use the installed migration without this setup. No production guards are relaxed.
    h.pool.execute("DROP TABLE game_guild_disband_step_events,game_guild_disband_steps,game_guild_job_event_outbox,game_guild_job_checkpoints,game_guild_event_outbox,game_guild_operations,game_account_guild_leadership,game_guild_invitations,game_guild_members,game_guild_ranks,game_guilds CASCADE").await?;
    h.pool.execute("DO $$ DECLARE f RECORD;BEGIN FOR f IN SELECT p.oid::regprocedure::text AS signature FROM pg_proc p JOIN pg_namespace n ON n.oid=p.pronamespace WHERE n.nspname=current_schema() AND p.proname LIKE 'game_guild_%' LOOP EXECUTE format('DROP FUNCTION IF EXISTS %s CASCADE',f.signature);END LOOP;END $$").await?;
    h.pool.execute(*GUILD_TABLES_PROPOSAL).await?;
    Ok(())
}
pub(super) async fn guard(h: &Harness) -> TestResult {
    h.pool.execute(*GUILD_GUARDS_PROPOSAL).await?;
    Ok(())
}
pub(super) async fn apply(
    h: &Harness,
    request: GuildRequest,
    f: CurrentCharacterItemFence,
) -> TestResult<GuildOutcome> {
    let seal = h.recovery.seal_current().map_err(debug)?;
    let a = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    h.root
        .commit_guild_membership(&a, &h.node, f, request)
        .await
        .map_err(|e| debug(e).into())
}
pub(super) async fn footprint(h: &Harness) -> TestResult<Vec<i64>> {
    let mut result = Vec::new();
    for table in [
        "game_guild_operations",
        "game_guild_event_outbox",
        "game_guild_members",
        "game_guild_invitations",
        "game_account_guild_leadership",
    ] {
        result.push(h.count(table).await?);
    }
    Ok(result)
}
pub(super) async fn revision(h: &Harness, g: u32) -> TestResult<String> {
    Ok(sqlx::query_scalar(
        "SELECT revision::text FROM game_guilds WHERE guild_id=encode($1,'hex')::uuid",
    )
    .bind(gid(g).as_bytes().as_slice())
    .fetch_one(&h.pool)
    .await?)
}

// These tests run the original private native Premium coordinator and its original consumer,
// durable fence and latch. The controlled registered clock is a test owner, never production
// time inferred from a receipt or an unavailable clock. No status/bool is passed to Found.
pub(super) struct FoundTestClock(pub(super) TrustedNow);
impl clock_registration::Registered for FoundTestClock {}
impl RegisteredPremiumClock for FoundTestClock {
    fn read_current_window(&self) -> Option<TrustedNow> {
        Some(self.0)
    }
}
pub(super) fn found_request(n: u32, g: u32, value: &str) -> GuildFoundRequest {
    let common = request(n, g, GuildCommand::Found);
    GuildFoundRequest {
        identity: common.identity,
        command: common.command,
        guild_id: common.guild_id,
        name: GuildName::parse(value).unwrap(),
    }
}
pub(super) fn pre_delivery(root: &DurabilityRoot) -> SpellPremiumCoordinator {
    SpellPremiumCoordinator::new(Arc::default(), None, None, Some(root.clone()))
}
pub(super) async fn found(
    h: &Harness,
    req: GuildFoundRequest,
    f: CurrentCharacterItemFence,
    coordinator: &SpellPremiumCoordinator,
) -> TestResult<GuildOutcome> {
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    h.root
        .commit_guild_found(&authority, &h.node, f, req, coordinator)
        .await
        .map_err(|e| debug(e).into())
}
pub(super) async fn current_coordinator(
    h: &Harness,
    who: [u8; 16],
    clock: bool,
) -> TestResult<(SpellPremiumCoordinator, Arc<PremiumConsumer>)> {
    use crate::premium_test_producer::{ProducerRequest, snapshot};
    let consumer = Arc::new(PremiumConsumer::default());
    let nonce = "0123456789abcdef0123456789abcdef";
    let body = snapshot(
        &ProducerRequest {
            account_id: uuid(who),
            nonce: nonce.into(),
        },
        1,
        &[],
    );
    assert!(matches!(
        consumer.ingest(&h.root, who, nonce, &body).await,
        crate::premium::IngestOutcome::Accepted
    ));
    let now = crate::premium::snapshot::rfc3339_utc_micros("2026-09-30T12:05:00Z").unwrap();
    let clock = if clock {
        Some(
            Arc::new(FoundTestClock(TrustedNow::new(now, 1_000_000).unwrap()))
                as Arc<dyn RegisteredPremiumClock>,
        )
    } else {
        None
    };
    Ok((
        SpellPremiumCoordinator::new(
            Arc::clone(&consumer),
            clock,
            Some(PremiumActivation {
                activation_id: "guild-found-fixture-activation".into(),
                switch_over_us: now - 60_000_000,
            }),
            Some(h.root.clone()),
        ),
        consumer,
    ))
}
pub(super) async fn found_footprint(h: &Harness) -> TestResult<Vec<i64>> {
    let mut counts = footprint(h).await?;
    counts.push(h.count("game_guilds").await?);
    counts.push(h.count("game_guild_ranks").await?);
    Ok(counts)
}

// Match the existing native lock observer's two-second budget. This observes a barrier;
// it never extends the production write deadline or relaxes the required lock ordering.
pub(super) async fn wait_advisory(pool: &sqlx::PgPool, key: i64, granted: bool) -> TestResult {
    tokio::time::timeout(crate::durability::DB_PASS_DEADLINE, async {
        loop {
            let present: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_locks WHERE locktype='advisory' AND database=(SELECT oid FROM pg_database WHERE datname=current_database()) AND classid::bigint=$1 AND objid::bigint=$2 AND objsubid=1 AND granted=$3)")
                .bind(((key as u64) >> 32) as i64).bind(key & 0xffff_ffff).bind(granted).fetch_one(pool).await?;
            if present { return Ok::<_, Box<dyn std::error::Error>>(()); }
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
    }).await.map_err(|_| format!("expected genuine native advisory lock was not observed: key={key}, granted={granted}"))?
}

// Text operations append after the complete inherited 29 bodies/helpers. These fixtures
// install historical current rows independently; they do not construct authority capabilities.
pub(super) async fn text_character_revision(h: &Harness) -> TestResult<String> {
    Ok(sqlx::query_scalar(
        "SELECT character_revision::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(id(CHARACTER).as_slice())
    .fetch_one(&h.pool)
    .await?)
}
pub(super) fn text_request(
    n: u32,
    op: GuildCommand,
    expected: u64,
    text: &str,
) -> GuildTextRequest {
    GuildTextRequest::new(request(n, 1, op), expected, text).unwrap()
}
pub(super) fn text_run<F>(tag: &'static str, body: F) -> TestResult
where
    F: AsyncFnOnce(&Harness) -> TestResult,
{
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let h = Harness::create(admin, tag).await?;
        let result = async {
            prepare(&h).await?;
            body(&h).await
        }
        .await;
        h.cleanup().await?;
        result
    })
}
pub(super) async fn text_apply(h: &Harness, req: GuildTextRequest) -> TestResult<GuildOutcome> {
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    h.root
        .commit_guild_text(&authority, &h.node, fence()?, req)
        .await
        .map_err(|e| debug(e).into())
}
pub(super) async fn text_snapshot(h: &Harness) -> TestResult<(String, String)> {
    let rows: String = sqlx::query_scalar("SELECT COALESCE(jsonb_agg(x ORDER BY x::text),'[]'::jsonb)::text FROM (SELECT jsonb_build_array('guild',to_jsonb(t)) x FROM game_guilds t UNION ALL SELECT jsonb_build_array('member',to_jsonb(t)) FROM game_guild_members t UNION ALL SELECT jsonb_build_array('rank',to_jsonb(t)) FROM game_guild_ranks t UNION ALL SELECT jsonb_build_array('invite',to_jsonb(t)) FROM game_guild_invitations t UNION ALL SELECT jsonb_build_array('position',to_jsonb(t)) FROM game_account_guild_leadership t UNION ALL SELECT jsonb_build_array('receipt',to_jsonb(t)) FROM game_guild_operations t UNION ALL SELECT jsonb_build_array('event',to_jsonb(t)) FROM game_guild_event_outbox t) r")
        .fetch_one(&h.pool).await?;
    Ok((rows, text_character_revision(h).await?))
}

// SetRank appends after the complete 96,638-byte inherited owner/helper prefix.
// Historical rows below are fixtures, never Premium/session/role authority supplied to a command.
pub(super) fn rank_request(n: u32, target: u32, revision: u64, level: u8) -> GuildRankRequest {
    GuildRankRequest::new(
        request(n, 1, GuildCommand::SetRank(character(target))),
        revision,
        level,
    )
    .unwrap()
}
pub(super) async fn rank_apply(
    h: &Harness,
    req: GuildRankRequest,
    premium: &SpellPremiumCoordinator,
) -> TestResult<GuildOutcome> {
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    h.root
        .commit_guild_rank(&authority, &h.node, fence()?, req, premium)
        .await
        .map_err(|e| debug(e).into())
}
/// Refusals legitimately persist their immutable receipt. Compare only mutation/event state.
pub(super) async fn rank_mutation_snapshot(h: &Harness) -> TestResult<String> {
    Ok(sqlx::query_scalar("SELECT COALESCE(jsonb_agg(x ORDER BY x::text),'[]'::jsonb)::text FROM (SELECT jsonb_build_array('guild',to_jsonb(t)) x FROM game_guilds t UNION ALL SELECT jsonb_build_array('member',to_jsonb(t)) FROM game_guild_members t UNION ALL SELECT jsonb_build_array('rank',to_jsonb(t)) FROM game_guild_ranks t UNION ALL SELECT jsonb_build_array('invite',to_jsonb(t)) FROM game_guild_invitations t UNION ALL SELECT jsonb_build_array('position',to_jsonb(t)) FROM game_account_guild_leadership t UNION ALL SELECT jsonb_build_array('event',to_jsonb(t)) FROM game_guild_event_outbox t) r")
        .fetch_one(&h.pool).await?)
}
pub(super) async fn rank_level(h: &Harness, target: u32) -> TestResult<i16> {
    Ok(sqlx::query_scalar(
        "SELECT rank_level FROM game_guild_members WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(character(target).as_bytes().as_slice())
    .fetch_one(&h.pool)
    .await?)
}
pub(super) async fn rank_fixture(h: &Harness, vice_count: u32) -> TestResult {
    initial_guild(h, 1, 0).await?;
    historical_character(h, 91, id(WORLD)).await?;
    initial_member(h, 1, 91, 3).await?;
    for n in 100..100 + vice_count {
        historical_character(h, n, id(WORLD)).await?;
        initial_member(h, 1, n, 2).await?;
    }
    sqlx::query("UPDATE game_guilds SET vice_deficit_since=NULL,premium_deficit_since=7")
        .execute(&h.pool)
        .await?;
    Ok(())
}

// Proposed append only; author integrates after the COMPLETE old48 owner/helper prefix.
// Uses actual native Root/Premium consumer/PG fixture. No runtime authority is constructed here.
pub(super) fn resign_request(n: u32, target: u32, expected: u64) -> GuildResignRequest {
    GuildResignRequest::new(
        request(n, 1, GuildCommand::Resign(character(target))),
        expected,
    )
    .expect("bounded native request fixture")
}
pub(super) async fn resign_apply(
    h: &Harness,
    req: GuildResignRequest,
    premium: &SpellPremiumCoordinator,
) -> TestResult<GuildOutcome> {
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    h.root
        .commit_guild_resign(&authority, &h.node, fence()?, req, premium)
        .await
        .map_err(|e| debug(e).into())
}
/// Historical setup happens before guard; the returned session is the original native fixture.
pub(super) async fn resign_fixture(h: &Harness) -> TestResult<CurrentCharacterItemFence> {
    initial_guild(h, 1, 0).await?;
    historical_character(h, 91, id(WORLD)).await?;
    initial_member(h, 1, 91, 2).await?;
    historical_session(h, 91).await
}
pub(super) async fn resign_resource_seed_current(
    h: &Harness,
    consumer: &Arc<PremiumConsumer>,
    who: [u8; 16],
) -> TestResult {
    use crate::premium_test_producer::{ProducerRequest, snapshot};
    let nonce = "0123456789abcdef0123456789abcdef";
    let response = snapshot(
        &ProducerRequest {
            account_id: uuid(who),
            nonce: nonce.into(),
        },
        1,
        &[],
    );
    assert!(matches!(
        consumer.ingest(&h.root, who, nonce, &response).await,
        crate::premium::IngestOutcome::Accepted
    ));
    Ok(())
}
/// Both real Account snapshots enter the SAME actual consumer/registered-clock coordinator.
pub(super) async fn resign_current(
    h: &Harness,
) -> TestResult<(SpellPremiumCoordinator, Arc<PremiumConsumer>)> {
    let (coordinator, consumer) = current_coordinator(h, account(0), true).await?;
    resign_resource_seed_current(h, &consumer, account(91)).await?;
    Ok((coordinator, consumer))
}
pub(super) async fn resign_resource_character_revisions(
    h: &Harness,
) -> TestResult<Vec<(String, String)>> {
    Ok(sqlx::query_as(
        "SELECT character_id::text,character_revision::text FROM game_character_roots ORDER BY character_id",
    )
    .fetch_all(&h.pool)
    .await?)
}
/// A successful role swap owns only rank_level and one Guild revision; all other rows stay exact.
pub(super) async fn resign_resource_unchanged_fields(h: &Harness) -> TestResult<String> {
    Ok(sqlx::query_scalar("SELECT COALESCE(jsonb_agg(x ORDER BY x::text),'[]'::jsonb)::text FROM (SELECT jsonb_build_array('guild',to_jsonb(t)-'revision') x FROM game_guilds t UNION ALL SELECT jsonb_build_array('member',to_jsonb(t)-'rank_level') FROM game_guild_members t UNION ALL SELECT jsonb_build_array('rank',to_jsonb(t)) FROM game_guild_ranks t UNION ALL SELECT jsonb_build_array('invite',to_jsonb(t)) FROM game_guild_invitations t UNION ALL SELECT jsonb_build_array('position',to_jsonb(t)) FROM game_account_guild_leadership t) r")
        .fetch_one(&h.pool).await?)
}

// Native writer/SQL/race families authored independently of the Premium/role proposal.
// Audit DATA below is only the expected producer payload, never current authority evidence.
pub(super) fn resign_test_audit(req: &GuildResignRequest) -> TestResult<serde_json::Value> {
    let data = GuildResignAuditProposal {
        operation: GuildAuditProposal {
            version: 1,
            occurrence: req.common.identity.occurrence,
            transaction_id: req.common.identity.transaction_id,
            event_id: req.common.identity.event_id,
            guild_id: *req.common.guild_id.as_bytes(),
            world_id: id(WORLD),
            actor: *character(0).as_bytes(),
            target: *req.common.operation.target(character(0)).as_bytes(),
            action: 10,
            previous_revision: req.expected_revision,
            revision: req
                .expected_revision
                .checked_add(1)
                .ok_or("audit overflow")?,
            rank_before: Some(2),
            rank_after: Some(1),
            removed_invitations: 0,
            occurred_at_unix_ms: req.common.identity.occurred_at_unix_ms,
        },
        expected_revision: req.expected_revision,
    };
    Ok(serde_json::from_slice(
        &data.checked_bytes().ok_or("invalid fixture audit DATA")?,
    )?)
}

// PROPOSAL ONLY: append to the eventual published Resign guild_tests.rs after exact-base refresh.
// Existing helpers above remain literal; every native case requires configured_admin via text_run.
// Rank-name lexical/length policy remains unallocated; these examples use ordinary ASCII names.

pub(super) fn edit_ranks_request(n: u32, expected: u64, names: &[&str]) -> GuildEditRanksRequest {
    GuildEditRanksRequest::new(
        request(n, 1, GuildCommand::EditRanks),
        expected,
        names.iter().map(|name| (*name).to_owned()).collect(),
    )
    .expect("bounded native rank-set DATA")
}

pub(super) async fn edit_ranks_apply(
    h: &Harness,
    req: GuildEditRanksRequest,
) -> TestResult<GuildOutcome> {
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    h.root
        .commit_guild_edit_ranks(&authority, &h.node, fence()?, req)
        .await
        .map_err(|error| debug(error).into())
}

pub(super) async fn edit_ranks_rows(h: &Harness, guild: u32) -> TestResult<Vec<(i16, String)>> {
    Ok(sqlx::query_as(
        "SELECT level,name FROM game_guild_ranks WHERE guild_id=encode($1,'hex')::uuid ORDER BY level",
    )
    .bind(gid(guild).as_bytes().as_slice())
    .fetch_all(&h.pool)
    .await?)
}

pub(super) async fn edit_ranks_expect(h: &Harness, names: &[String]) -> TestResult {
    let expected = names
        .iter()
        .enumerate()
        .map(|(index, name)| (i16::try_from(index + 1).unwrap(), name.clone()))
        .collect::<Vec<_>>();
    assert_eq!(edit_ranks_rows(h, 1).await?, expected);
    Ok(())
}

/// Historical setup precedes all Guild guards. Offline lower ranks have no fabricated sessions.
pub(super) async fn edit_ranks_fixture(h: &Harness) -> TestResult {
    initial_guild(h, 1, 0).await?;
    for (who, rank) in [(91, 2), (92, 3), (93, 4)] {
        historical_character(h, who, id(WORLD)).await?;
        initial_member(h, 1, who, rank).await?;
    }
    historical_character(h, 94, id(WORLD)).await?;
    initial_guild(h, 2, 94).await?;
    historical_character(h, 95, id(WORLD)).await?;
    initial_member(h, 2, 95, 4).await?;
    Ok(())
}

/// Rank definitions and remapped rank_level are the only mutable snapshot fields owned here.
pub(super) async fn edit_ranks_other_fields(h: &Harness) -> TestResult<String> {
    Ok(sqlx::query_scalar("SELECT COALESCE(jsonb_agg(x ORDER BY x::text),'[]'::jsonb)::text FROM (SELECT jsonb_build_array('guild',to_jsonb(t)-'revision') x FROM game_guilds t UNION ALL SELECT jsonb_build_array('member',to_jsonb(t)-'rank_level') FROM game_guild_members t UNION ALL SELECT jsonb_build_array('invite',to_jsonb(t)) FROM game_guild_invitations t UNION ALL SELECT jsonb_build_array('position',to_jsonb(t)) FROM game_account_guild_leadership t) r")
        .fetch_one(&h.pool).await?)
}

pub(super) fn edit_ranks_large_names() -> Vec<String> {
    (b'A'..=b'T')
        .map(|letter| format!("Rank {}", char::from(letter)))
        .collect()
}

pub(super) fn edit_ranks_test_audit(req: &GuildEditRanksRequest) -> TestResult<serde_json::Value> {
    let audit = GuildEditRanksAuditProposal {
        operation: GuildAuditProposal {
            version: 1,
            occurrence: req.common.identity.occurrence,
            transaction_id: req.common.identity.transaction_id,
            event_id: req.common.identity.event_id,
            guild_id: *req.common.guild_id.as_bytes(),
            world_id: id(WORLD),
            actor: *character(0).as_bytes(),
            target: *character(0).as_bytes(),
            action: 11,
            previous_revision: req.expected_revision,
            revision: req
                .expected_revision
                .checked_add(1)
                .ok_or("audit overflow")?,
            rank_before: None,
            rank_after: None,
            removed_invitations: 0,
            occurred_at_unix_ms: req.common.identity.occurred_at_unix_ms,
        },
        expected_revision: req.expected_revision,
        names: req.names.clone(),
    };
    Ok(serde_json::from_slice(
        &audit.checked_bytes().ok_or("invalid native audit DATA")?,
    )?)
}

// Proposal only. Append after the eventual unchanged Resign60 + accepted EditRanks12.
// Decline uses the real membership owner, not a new authority constructor or Hall/Bank port.

pub(super) async fn decline_fixture(h: &Harness, expired: bool) -> TestResult {
    historical_character(h, 91, id(WORLD)).await?;
    historical_character(h, 92, id(WORLD)).await?;
    initial_guild(h, 1, 91).await?;
    initial_guild(h, 2, 0).await?;
    historical_character(h, 94, id(WORLD)).await?;
    initial_guild(h, 3, 94).await?;
    historical_character(h, 96, id(WORLD)).await?;
    initial_guild(h, 4, 96).await?;
    initial_invitation(h, 1, 0, 91, expired).await?;
    initial_invitation(h, 1, 92, 91, false).await?;
    initial_invitation(h, 3, 0, 94, false).await?;
    initial_invitation(h, 4, 0, 96, false).await?;
    Ok(())
}

pub(super) async fn decline_invitation_rows(
    h: &Harness,
) -> TestResult<Vec<(String, String, String, i64)>> {
    Ok(sqlx::query_as("SELECT guild_id::text,character_id::text,inviter::text,expires_at FROM game_guild_invitations ORDER BY guild_id,character_id")
        .fetch_all(&h.pool).await?)
}

pub(super) async fn decline_non_invitation_fields(h: &Harness) -> TestResult<String> {
    Ok(sqlx::query_scalar("SELECT COALESCE(jsonb_agg(x ORDER BY x::text),'[]'::jsonb)::text FROM (SELECT jsonb_build_array('guild',to_jsonb(t)-'revision') x FROM game_guilds t UNION ALL SELECT jsonb_build_array('member',to_jsonb(t)) FROM game_guild_members t UNION ALL SELECT jsonb_build_array('rank',to_jsonb(t)) FROM game_guild_ranks t UNION ALL SELECT jsonb_build_array('position',to_jsonb(t)) FROM game_account_guild_leadership t) r")
        .fetch_one(&h.pool).await?)
}

pub(super) fn decline_test_audit(req: &GuildRequest) -> GuildDeclineAuditProposal {
    GuildDeclineAuditProposal {
        operation: GuildAuditProposal {
            version: 1,
            occurrence: req.identity.occurrence,
            transaction_id: req.identity.transaction_id,
            event_id: req.identity.event_id,
            guild_id: *req.guild_id.as_bytes(),
            world_id: id(WORLD),
            actor: *character(0).as_bytes(),
            target: *character(0).as_bytes(),
            action: 12,
            previous_revision: 1,
            revision: 2,
            rank_before: None,
            rank_after: None,
            removed_invitations: 1,
            occurred_at_unix_ms: req.identity.occurred_at_unix_ms,
        },
    }
}

pub(super) async fn guild_job_start(h: &Harness, guild: u32) -> TestResult<GuildDeadlineOutcome> {
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    h.root
        .start_guild_deadline_disband(
            &authority,
            &h.node,
            fence()?.runtime_scope.world_id(),
            gid(guild),
        )
        .await
        .map_err(|error| debug(error).into())
}

pub(super) fn disband_request(
    n: u32,
    f: CurrentCharacterItemFence,
    confirmation: &str,
) -> GuildDisbandRequest {
    let mut common = request(n, 1, GuildCommand::Disband);
    common.command = CommandRef::new(f.game_session_id, CommandId::new(u64::from(n)).unwrap());
    GuildDisbandRequest::new(common, confirmation).unwrap()
}
pub(super) async fn disband_apply(
    h: &Harness,
    disband: GuildDisbandRequest,
    f: CurrentCharacterItemFence,
) -> TestResult<GuildOutcome> {
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    h.root
        .commit_guild_disband(&authority, &h.node, f, disband)
        .await
        .map_err(|error| debug(error).into())
}

// CHAT owner-read qualification. Historical rows below establish initial DB state only;
// all observations reconcile genuine Root authority and all changes use native Guild writers.
pub(super) async fn chat_read(
    h: &Harness,
    payload: Option<GuildId>,
) -> TestResult<Option<GuildChatMembership>> {
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let observed = match payload {
        Some(guild) => {
            h.root
                .read_guild_chat_recipient(&authority, &h.node, fence()?, guild)
                .await
        }
        None => {
            h.root
                .read_guild_chat_membership(&authority, &h.node, fence()?)
                .await
        }
    };
    observed.map_err(|error| debug(error).into())
}
