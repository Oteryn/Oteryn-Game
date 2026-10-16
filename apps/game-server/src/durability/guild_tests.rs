//! Actual native Root qualification with privileged historical fixtures. Guild snapshot rows
//! below are NOT a formation/Premium writer or an external membership authority. Every native
//! Character guard remains enabled, and current authority is independently reconciled afterwards.
#![allow(clippy::expect_used, clippy::unwrap_used)]
use super::*;
use crate::foundation::{CommandId, ConnectionGeneration, GameSessionId, ScopeOwnershipGeneration};
use crate::item_transfer_postgres_cases::{
    self as fixture, CHARACTER, Harness, TestResult, WORLD, configured_admin, debug, fence, id,
    runtime,
};
use sqlx::Executor;

fn sid(number: u32, kind: u8) -> [u8; 16] {
    let mut bytes = [0_u8; 16];
    bytes[0] = 0xa1;
    bytes[1] = kind;
    bytes[6] = 0x70;
    bytes[8] = 0x80;
    bytes[12..].copy_from_slice(&number.to_be_bytes());
    bytes
}
fn character(n: u32) -> CharacterId {
    CharacterId::from_bytes(if n == 0 { id(CHARACTER) } else { sid(n, 1) }).unwrap()
}
fn account(n: u32) -> [u8; 16] {
    if n == 0 { id(40) } else { sid(n, 2) }
}
fn gid(n: u32) -> GuildId {
    GuildId::from_bytes(sid(n, 3)).unwrap()
}
fn request(n: u32, guild: u32, op: GuildCommand) -> GuildRequest {
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
fn result(value: GuildOutcome) -> GuildReceipt {
    match value {
        GuildOutcome::Committed(r) | GuildOutcome::Replayed(r) => r,
    }
}
fn name(mut n: u32) -> String {
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
fn uuid(bytes: [u8; 16]) -> String {
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
async fn historical_character(h: &Harness, n: u32, world: [u8; 16]) -> TestResult {
    historical_character_account(h, n, world, account(n)).await
}

async fn historical_character_account(
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
async fn initial_guild(h: &Harness, g: u32, leader: u32) -> TestResult {
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
async fn initial_member(h: &Harness, g: u32, c: u32, rank: i16) -> TestResult {
    sqlx::query("INSERT INTO game_guild_members(character_id,guild_id,rank_level,title,joined_at) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,'',1)")
        .bind(character(c).as_bytes().as_slice()).bind(gid(g).as_bytes().as_slice()).bind(rank).execute(&h.pool).await?;
    if rank <= 2 {
        sqlx::query("INSERT INTO game_account_guild_leadership VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid)")
        .bind(account(c).as_slice()).bind(gid(g).as_bytes().as_slice()).bind(character(c).as_bytes().as_slice()).execute(&h.pool).await?;
    }
    Ok(())
}
async fn initial_invitation(
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
async fn historical_session(h: &Harness, n: u32) -> TestResult<CurrentCharacterItemFence> {
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
async fn prepare(h: &Harness) -> TestResult {
    let version: String = sqlx::query_scalar("SHOW server_version_num")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(version, "170006");
    eprintln!("GUILD-NATIVE-PG server_version_num={version}");
    h.pool.execute(GUILD_TABLES_PROPOSAL).await?;
    Ok(())
}
async fn guard(h: &Harness) -> TestResult {
    h.pool.execute(GUILD_GUARDS_PROPOSAL).await?;
    Ok(())
}
async fn apply(
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
async fn footprint(h: &Harness) -> TestResult<Vec<i64>> {
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
async fn revision(h: &Harness, g: u32) -> TestResult<String> {
    Ok(sqlx::query_scalar(
        "SELECT revision::text FROM game_guilds WHERE guild_id=encode($1,'hex')::uuid",
    )
    .bind(gid(g).as_bytes().as_slice())
    .fetch_one(&h.pool)
    .await?)
}

#[test]
fn native_binding_changes_with_each_semantic_identity_and_target() -> TestResult {
    assert!(GuildId::from_bytes([0; 16]).is_err());
    let mut wrong_version = sid(1, 3);
    wrong_version[6] = 0x40;
    assert!(GuildId::from_bytes(wrong_version).is_err());
    let f = fence()?;
    let request = request(1, 1, GuildCommand::Invite(character(90)));
    let original = binding(&f, &request)?;
    let mut changed = request.clone();
    changed.operation = GuildCommand::Invite(character(91));
    assert_ne!(original, binding(&f, &changed)?);
    changed = request.clone();
    changed.guild_id = gid(2);
    assert_ne!(original, binding(&f, &changed)?);
    changed = request.clone();
    changed.identity.event_id = sid(2, 12);
    assert_ne!(original, binding(&f, &changed)?);
    let mut mismatch = f;
    mismatch.game_session_id = GameSessionId::decode(&sid(90, 20))?;
    assert!(binding(&mismatch, &request).is_err());
    assert_eq!(crate::domain::guild::INVITATION_LIFETIME_MS, 2_592_000_000);
    assert_eq!(
        (MEMBERS_MAX, GUILD_INVITATIONS_MAX, TARGET_INVITATIONS_MAX),
        (2000, 500, 50)
    );
    Ok(())
}

#[test]
fn native_invite_revoke_receipt_first_and_command_binding() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let h=Harness::create(admin,"guild_invite").await?;prepare(&h).await?;
        historical_character(&h,90,id(WORLD)).await?;initial_guild(&h,1,0).await?;guard(&h).await?;
        let character_revision:String=sqlx::query_scalar("SELECT character_revision::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid").bind(id(CHARACTER).as_slice()).fetch_one(&h.pool).await?;
        let req=request(1,1,GuildCommand::Invite(character(90)));assert_eq!(result(apply(&h,req.clone(),fence()?).await?).result,GuildResult::Ok);
        assert_eq!(revision(&h,1).await?,"2");assert_eq!(h.count("game_guild_invitations").await?,1);
        let saved=footprint(&h).await?;assert!(matches!(apply(&h,req.clone(),fence()?).await?,GuildOutcome::Replayed(_)));assert_eq!(saved,footprint(&h).await?);
        let seal=h.recovery.seal_current().map_err(debug)?;let a=h.root.open_character_authority(&seal).await.map_err(debug)?;
        let mut changed=req.clone();changed.operation=GuildCommand::Revoke(character(90));
        assert!(matches!(h.root.commit_guild_membership(&a,&h.node,fence()?,changed).await,Err(GuildError::ConflictingOccurrence)));
        let mut reused=request(2,1,GuildCommand::Revoke(character(90)));reused.command=req.command;
        assert!(matches!(h.root.commit_guild_membership(&a,&h.node,fence()?,reused).await,Err(GuildError::ConflictingOccurrence)));
        drop(a);drop(seal);
        assert_eq!(result(apply(&h,request(2,1,GuildCommand::Revoke(character(90))),fence()?).await?).result,GuildResult::Ok);
        assert_eq!(h.count("game_guild_invitations").await?,0);assert_eq!(h.count("game_guild_event_outbox").await?,2);
        let after:String=sqlx::query_scalar("SELECT character_revision::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid").bind(id(CHARACTER).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(after,character_revision);h.cleanup().await
    })
}

#[test]
fn native_accept_removes_all_target_invitations_and_joins_lowest_rank() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let h = Harness::create(admin, "guild_accept").await?;
        prepare(&h).await?;
        for n in 100..150 {
            historical_character(&h, n, id(WORLD)).await?;
            initial_guild(&h, n, n).await?;
            initial_invitation(&h, n, 0, n, false).await?;
        }
        guard(&h).await?;
        let req = request(1, 100, GuildCommand::Accept);
        let receipt = result(apply(&h, req.clone(), fence()?).await?);
        assert_eq!(receipt.result, GuildResult::Ok);
        let rank: i16 = sqlx::query_scalar(
            "SELECT rank_level FROM game_guild_members WHERE character_id=encode($1,'hex')::uuid",
        )
        .bind(id(CHARACTER).as_slice())
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(rank, 4);
        assert_eq!(h.count("game_guild_invitations").await?, 0);
        let wire: Vec<u8> = sqlx::query_scalar("SELECT payload FROM game_guild_event_outbox")
            .fetch_one(&h.pool)
            .await?;
        let audit: GuildAuditProposal = serde_json::from_slice(&wire)?;
        assert_eq!(audit.removed_invitations, 50);
        assert_eq!(audit.rank_after, Some(4));
        let mut old = fence()?;
        old.connection_generation = ConnectionGeneration::new(2)?;
        assert!(
            matches!(apply(&h, req, old).await?, GuildOutcome::Replayed(_)),
            "historical outcome is not a new grant"
        );
        assert_eq!(
            result(apply(&h, request(2, 101, GuildCommand::Accept), fence()?).await?).result,
            GuildResult::AlreadyInGuild
        );
        h.cleanup().await
    })
}

#[test]
fn native_refusal_replays_after_membership_changes_and_never_invites_again() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let h = Harness::create(admin, "guild_refusal").await?;
        prepare(&h).await?;
        historical_character(&h, 90, id(WORLD)).await?;
        initial_guild(&h, 1, 0).await?;
        initial_member(&h, 1, 90, 3).await?;
        guard(&h).await?;
        let rejected = request(1, 1, GuildCommand::Invite(character(90)));
        assert_eq!(
            result(apply(&h, rejected.clone(), fence()?).await?).result,
            GuildResult::NotAllowed
        );
        assert_eq!(h.count("game_guild_event_outbox").await?, 0);
        assert_eq!(revision(&h, 1).await?, "1");
        assert_eq!(
            result(
                apply(
                    &h,
                    request(2, 1, GuildCommand::Exclude(character(90))),
                    fence()?
                )
                .await?
            )
            .result,
            GuildResult::Ok
        );
        let fp = footprint(&h).await?;
        let replay = apply(&h, rejected, fence()?).await?;
        assert!(matches!(
            replay,
            GuildOutcome::Replayed(GuildReceipt {
                result: GuildResult::NotAllowed,
                ..
            })
        ));
        assert_eq!(fp, footprint(&h).await?);
        assert_eq!(h.count("game_guild_invitations").await?, 0);
        h.cleanup().await
    })
}

#[test]
fn native_vice_exit_cleans_account_position_and_preserves_first_deficit() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        for leave in [false,true] {
            let h=Harness::create(admin.clone(),"guild_vice").await?;prepare(&h).await?;
            for n in 90..95 {historical_character(&h,n,id(WORLD)).await?;}
            initial_guild(&h,1,if leave{94}else{0}).await?;
            for n in 90..94 {initial_member(&h,1,if leave&&n==90{0}else{n},2).await?;}
            sqlx::query("UPDATE game_guilds SET vice_deficit_since=NULL").execute(&h.pool).await?;guard(&h).await?;
            if leave {
                assert_eq!(result(apply(&h,request(1,1,GuildCommand::Exclude(character(91))),fence()?).await?).result,GuildResult::NotAllowed);
            }
            let op=if leave{GuildCommand::Leave}else{GuildCommand::Exclude(character(90))};
            assert_eq!(result(apply(&h,request(if leave{2}else{1},1,op),fence()?).await?).result,GuildResult::Ok);
            let exited=if leave{0}else{90};let position:i64=sqlx::query_scalar("SELECT count(*) FROM game_account_guild_leadership WHERE character_id=encode($1,'hex')::uuid")
                .bind(character(exited).as_bytes().as_slice()).fetch_one(&h.pool).await?;assert_eq!(position,0);
            let first:Option<i64>=sqlx::query_scalar("SELECT vice_deficit_since FROM game_guilds").fetch_one(&h.pool).await?;assert!(first.is_some());
            if !leave {
                assert_eq!(result(apply(&h,request(2,1,GuildCommand::Exclude(character(91))),fence()?).await?).result,GuildResult::Ok);
                let next:Option<i64>=sqlx::query_scalar("SELECT vice_deficit_since FROM game_guilds").fetch_one(&h.pool).await?;assert_eq!(next,first);
                assert_eq!(result(apply(&h,request(3,1,GuildCommand::Leave),fence()?).await?).result,GuildResult::NotAllowed);
                assert_eq!(result(apply(&h,request(4,1,GuildCommand::Exclude(character(0))),fence()?).await?).result,GuildResult::NotAllowed);
            }
            h.cleanup().await?;
        }Ok(())
    })
}

#[test]
fn native_invite_allows_other_guild_but_rejects_other_world_and_stale_authority() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let h = Harness::create(admin, "guild_fences").await?;
        prepare(&h).await?;
        historical_character(&h, 90, id(WORLD)).await?;
        historical_character(&h, 91, id(WORLD)).await?;
        historical_character(&h, 92, sid(2, 50)).await?;
        initial_guild(&h, 1, 0).await?;
        initial_guild(&h, 2, 90).await?;
        initial_member(&h, 2, 91, 3).await?;
        guard(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let a = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        for field in 0..3 {
            let mut stale = fence()?;
            match field {
                0 => stale.connection_generation = ConnectionGeneration::new(2)?,
                1 => stale.character_lease_generation = 2,
                _ => stale.scope_ownership_generation = ScopeOwnershipGeneration::new(2)?,
            };
            assert!(matches!(
                h.root
                    .commit_guild_membership(
                        &a,
                        &h.node,
                        stale,
                        request(field + 1, 1, GuildCommand::Invite(character(91)))
                    )
                    .await,
                Err(GuildError::NotCurrent)
            ));
            assert_eq!(h.count("game_guild_operations").await?, 0);
        }
        let other = DurabilityRoot::connect_test_runtime(&h.database.url)?;
        assert!(other.maintain_ready_once().await?);
        assert!(
            matches!(
                other
                    .commit_guild_membership(
                        &a,
                        &h.node,
                        fence()?,
                        request(4, 1, GuildCommand::Invite(character(91)))
                    )
                    .await,
                Err(GuildError::NotCurrent)
            ),
            "Root capability cannot be transplanted"
        );
        let wrong = fixture::register(&h.root, 2).await?;
        assert!(matches!(
            h.root
                .commit_guild_membership(
                    &a,
                    &wrong,
                    fence()?,
                    request(5, 1, GuildCommand::Invite(character(91)))
                )
                .await,
            Err(GuildError::NotCurrent)
        ));
        drop(other);
        drop(a);
        drop(seal);
        assert_eq!(
            result(
                apply(
                    &h,
                    request(6, 1, GuildCommand::Invite(character(91))),
                    fence()?
                )
                .await?
            )
            .result,
            GuildResult::Ok
        );
        assert_eq!(
            result(
                apply(
                    &h,
                    request(7, 1, GuildCommand::Invite(character(92))),
                    fence()?
                )
                .await?
            )
            .result,
            GuildResult::NotAllowed
        );
        h.cleanup().await
    })
}

#[test]
fn native_invitation_limits_include_retained_rows_and_cleanup_is_bounded() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        for (count, expired, expected) in [
            (49_u32, false, GuildResult::Ok),
            (50, false, GuildResult::InvitationLimit),
            (50, true, GuildResult::Ok),
        ] {
            let h = Harness::create(admin.clone(), "guild_target_cap").await?;
            prepare(&h).await?;
            initial_guild(&h, 1, 0).await?;
            historical_character(&h, 90, id(WORLD)).await?;
            for n in 100..100 + count {
                historical_character(&h, n, id(WORLD)).await?;
                initial_guild(&h, n, n).await?;
                initial_invitation(&h, n, 90, n, expired).await?;
            }
            guard(&h).await?;
            assert_eq!(
                result(
                    apply(
                        &h,
                        request(1, 1, GuildCommand::Invite(character(90))),
                        fence()?
                    )
                    .await?
                )
                .result,
                expected
            );
            assert_eq!(
                h.count("game_guild_invitations").await?,
                if expired { 1 } else { 50 }
            );
            h.cleanup().await?;
        }
        let h = Harness::create(admin, "guild_cap").await?;
        prepare(&h).await?;
        initial_guild(&h, 1, 0).await?;
        for n in 100..601 {
            historical_character(&h, n, id(WORLD)).await?;
            if n < 599 {
                initial_invitation(&h, 1, n, 0, false).await?;
            }
        }
        guard(&h).await?;
        assert_eq!(
            result(
                apply(
                    &h,
                    request(1, 1, GuildCommand::Invite(character(599))),
                    fence()?
                )
                .await?
            )
            .result,
            GuildResult::Ok
        );
        assert_eq!(
            result(
                apply(
                    &h,
                    request(2, 1, GuildCommand::Invite(character(600))),
                    fence()?
                )
                .await?
            )
            .result,
            GuildResult::InvitationLimit
        );
        assert_eq!(h.count("game_guild_invitations").await?, 500);
        h.cleanup().await
    })
}

#[test]
fn native_member_limit_accepts_2000_and_refuses_2001() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        for count in [1999_u32, 2000] {
            let h = Harness::create(admin.clone(), "guild_member_cap").await?;
            prepare(&h).await?;
            historical_character(&h, 90, id(WORLD)).await?;
            initial_guild(&h, 1, 90).await?;
            for n in 100..100 + count - 1 {
                historical_character(&h, n, id(WORLD)).await?;
                initial_member(&h, 1, n, 3).await?;
            }
            initial_invitation(&h, 1, 0, 90, false).await?;
            guard(&h).await?;
            let r = result(apply(&h, request(1, 1, GuildCommand::Accept), fence()?).await?);
            assert_eq!(
                r.result,
                if count == 1999 {
                    GuildResult::Ok
                } else {
                    GuildResult::GuildFull
                }
            );
            assert_eq!(h.count("game_guild_members").await?, 2000);
            h.cleanup().await?;
        }
        Ok(())
    })
}

#[test]
fn native_state_and_expiry_refusals_leave_revision_and_event_untouched() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        for (state, expired, expected) in [
            (1_i16, true, GuildResult::NotInvited),
            (3, false, GuildResult::GuildDisbanding),
            (4, false, GuildResult::GuildDisbanding),
        ] {
            let h = Harness::create(admin.clone(), "guild_state").await?;
            prepare(&h).await?;
            historical_character(&h, 90, id(WORLD)).await?;
            initial_guild(&h, 1, 90).await?;
            sqlx::query("UPDATE game_guilds SET state=$1")
                .bind(state)
                .execute(&h.pool)
                .await?;
            if state == 4 {
                sqlx::query("DELETE FROM game_guild_members")
                    .execute(&h.pool)
                    .await?;
                sqlx::query("DELETE FROM game_account_guild_leadership")
                    .execute(&h.pool)
                    .await?;
            } else {
                initial_invitation(&h, 1, 0, 90, expired).await?;
            }
            guard(&h).await?;
            let response = result(apply(&h, request(1, 1, GuildCommand::Accept), fence()?).await?);
            assert_eq!(response.result, expected);
            assert_eq!(revision(&h, 1).await?, "1");
            assert_eq!(h.count("game_guild_event_outbox").await?, 0);
            h.cleanup().await?;
        }
        Ok(())
    })
}

#[test]
fn native_atomic_event_failure_rolls_back_then_exact_retry_commits_once() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let h=Harness::create(admin,"guild_rollback").await?;prepare(&h).await?;historical_character(&h,90,id(WORLD)).await?;initial_guild(&h,1,0).await?;guard(&h).await?;
        h.pool.execute("CREATE FUNCTION guild_test_failure() RETURNS TRIGGER LANGUAGE plpgsql AS $$BEGIN RAISE EXCEPTION 'fixture failure' USING ERRCODE='23514';END$$;CREATE CONSTRAINT TRIGGER guild_test_failure AFTER INSERT ON game_guild_event_outbox DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION guild_test_failure();").await?;
        let before=footprint(&h).await?;let seal=h.recovery.seal_current().map_err(debug)?;let a=h.root.open_character_authority(&seal).await.map_err(debug)?;
        let req=request(1,1,GuildCommand::Invite(character(90)));
        assert!(matches!(h.root.commit_guild_membership(&a,&h.node,fence()?,req.clone()).await,Err(GuildError::Unavailable(_))));
        assert_eq!(before,footprint(&h).await?);assert_eq!(revision(&h,1).await?,"1");
        h.pool.execute("DROP TRIGGER guild_test_failure ON game_guild_event_outbox;DROP FUNCTION guild_test_failure();").await?;
        let other=DurabilityRoot::connect_test_runtime(&h.database.url)?;assert!(other.maintain_ready_once().await?);let b=other.open_character_authority(&seal).await.map_err(debug)?;
        assert_eq!(result(other.commit_guild_membership(&b,&h.node,fence()?,req.clone()).await.map_err(debug)?).result,GuildResult::Ok);
        assert!(matches!(other.commit_guild_membership(&b,&h.node,fence()?,req).await.map_err(debug)?,GuildOutcome::Replayed(_)));
        assert_eq!(h.count("game_guild_event_outbox").await?,1);
        let denied=sqlx::query("UPDATE game_guild_operations SET result=1").execute(&h.pool).await.expect_err("receipt immutable");assert_eq!(denied.as_database_error().and_then(|e|e.code()).as_deref(),Some("23514"));
        // Excluding only the generated key must still reject mutation of its actual source.
        // The extra fixture trigger runs before the real immutable-field guard; no guard is
        // disabled. This actual current Root operation must roll back its receipt/event/delete.
        let guarded_before=footprint(&h).await?;
        let original_name:String=sqlx::query_scalar("SELECT name FROM game_guilds").fetch_one(&h.pool).await?;
        h.pool.execute("CREATE FUNCTION guild_test_immutable_name() RETURNS TRIGGER LANGUAGE plpgsql AS $$BEGIN NEW.name='Guild Changed';RETURN NEW;END$$;CREATE TRIGGER aaa_guild_test_immutable_name BEFORE UPDATE ON game_guilds FOR EACH ROW EXECUTE FUNCTION guild_test_immutable_name();").await?;
        assert!(matches!(other.commit_guild_membership(&b,&h.node,fence()?,request(2,1,GuildCommand::Revoke(character(90)))).await,Err(GuildError::Unavailable(_))));
        assert_eq!(footprint(&h).await?,guarded_before);assert_eq!(revision(&h,1).await?,"2");
        let still_name:String=sqlx::query_scalar("SELECT name FROM game_guilds").fetch_one(&h.pool).await?;assert_eq!(still_name,original_name);
        h.pool.execute("DROP TRIGGER aaa_guild_test_immutable_name ON game_guilds;DROP FUNCTION guild_test_immutable_name();").await?;
        drop(b);drop(other);drop(a);drop(seal);h.cleanup().await
    })
}

#[test]
fn native_two_roots_replay_one_occurrence_without_double_revision_or_event() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let h = Harness::create(admin, "guild_race").await?;
        prepare(&h).await?;
        historical_character(&h, 90, id(WORLD)).await?;
        initial_guild(&h, 1, 0).await?;
        guard(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let a = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let other = DurabilityRoot::connect_test_runtime(&h.database.url)?;
        assert!(other.maintain_ready_once().await?);
        let b = other.open_character_authority(&seal).await.map_err(debug)?;
        let req = request(1, 1, GuildCommand::Invite(character(90)));
        let f = fence()?;
        let (one, two) = fixture::join_two(
            h.root.commit_guild_membership(&a, &h.node, f, req.clone()),
            other.commit_guild_membership(&b, &h.node, f, req),
        )
        .await;
        let one = one.map_err(debug)?;
        let two = two.map_err(debug)?;
        assert_eq!(
            usize::from(matches!(one, GuildOutcome::Committed(_)))
                + usize::from(matches!(two, GuildOutcome::Committed(_))),
            1
        );
        assert_eq!(revision(&h, 1).await?, "2");
        assert_eq!(h.count("game_guild_operations").await?, 1);
        assert_eq!(h.count("game_guild_event_outbox").await?, 1);
        drop(b);
        drop(other);
        drop(a);
        drop(seal);
        h.cleanup().await
    })
}

#[test]
fn native_two_current_roots_race_at_both_invitation_caps() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        for target_cap in [false, true] {
            let h = Harness::create(admin.clone(), "guild_cap_race").await?;
            prepare(&h).await?;
            for n in 90..93 {
                historical_character(&h, n, id(WORLD)).await?;
            }
            initial_guild(&h, 1, 0).await?;
            if target_cap {
                initial_guild(&h, 2, 90).await?;
                for n in 100..149 {
                    historical_character(&h, n, id(WORLD)).await?;
                    initial_guild(&h, n, n).await?;
                    initial_invitation(&h, n, 91, n, false).await?;
                }
            } else {
                initial_member(&h, 1, 90, 2).await?;
                for n in 100..599 {
                    historical_character(&h, n, id(WORLD)).await?;
                    initial_invitation(&h, 1, n, 0, false).await?;
                }
            }
            let other_fence = historical_session(&h, 90).await?;
            guard(&h).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let a = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let other = DurabilityRoot::connect_test_runtime(&h.database.url)?;
            assert!(other.maintain_ready_once().await?);
            let b = other.open_character_authority(&seal).await.map_err(debug)?;
            let first = request(1, 1, GuildCommand::Invite(character(91)));
            let mut second = request(
                2,
                if target_cap { 2 } else { 1 },
                GuildCommand::Invite(character(if target_cap { 91 } else { 92 })),
            );
            second.command = CommandRef::new(other_fence.game_session_id, CommandId::new(2)?);
            let (one, two) = fixture::join_two(
                h.root.commit_guild_membership(&a, &h.node, fence()?, first),
                other.commit_guild_membership(&b, &h.node, other_fence, second),
            )
            .await;
            let mut outcomes = [
                result(one.map_err(debug)?).result.code(),
                result(two.map_err(debug)?).result.code(),
            ];
            outcomes.sort();
            assert_eq!(
                outcomes,
                [GuildResult::Ok.code(), GuildResult::InvitationLimit.code()]
            );
            assert_eq!(
                h.count("game_guild_invitations").await?,
                if target_cap { 50 } else { 500 }
            );
            assert_eq!(h.count("game_guild_event_outbox").await?, 1);
            // Actual native admission relation locks serialize these passes before row locks;
            // this proves whole-native outcomes/caps, not isolated row-lock stress.
            drop(b);
            drop(other);
            drop(a);
            drop(seal);
            h.cleanup().await?;
        }
        Ok(())
    })
}

#[test]
fn audit_is_closed_and_schema_keeps_native_bank_prerequisite_columns() -> TestResult {
    let audit = GuildAuditProposal {
        version: 1,
        occurrence: sid(1, 10),
        transaction_id: sid(1, 11),
        event_id: sid(1, 12),
        guild_id: *gid(1).as_bytes(),
        world_id: id(WORLD),
        actor: id(CHARACTER),
        target: id(CHARACTER),
        action: 3,
        previous_revision: 1,
        revision: 2,
        rank_before: None,
        rank_after: Some(4),
        removed_invitations: 50,
        occurred_at_unix_ms: 1,
    };
    assert!(audit.checked_bytes().is_some());
    let mut wrong = audit.clone();
    wrong.rank_after = Some(1);
    assert!(wrong.checked_bytes().is_none());
    wrong = audit.clone();
    wrong.removed_invitations = 51;
    assert!(wrong.checked_bytes().is_none());
    wrong = audit;
    wrong.revision = 3;
    assert!(wrong.checked_bytes().is_none());
    // Shape only, not a claim to execute or admit the separately published Bank writer.
    for name in [
        "game_guilds",
        "guild_id",
        "world_id",
        "state SMALLINT",
        "revision NUMERIC(20,0)",
        "game_guild_ranks",
        "level SMALLINT",
        "game_guild_members",
        "rank_level SMALLINT",
        "game_guild_one_leader",
    ] {
        assert!(GUILD_TABLES_PROPOSAL.contains(name));
    }
    Ok(())
}

// These tests run the original private native Premium coordinator and its original consumer,
// durable fence and latch. The controlled registered clock is a test owner, never production
// time inferred from a receipt or an unavailable clock. No status/bool is passed to Found.
use crate::gameplay_transport::spell_premium_coordinator::{
    PremiumActivation, RegisteredPremiumClock, clock_registration,
};
use crate::premium::{PremiumConsumer, TrustedNow};
use std::sync::Arc;

struct FoundTestClock(TrustedNow);
impl clock_registration::Registered for FoundTestClock {}
impl RegisteredPremiumClock for FoundTestClock {
    fn read_current_window(&self) -> Option<TrustedNow> {
        Some(self.0)
    }
}
fn found_request(n: u32, g: u32, value: &str) -> GuildFoundRequest {
    let common = request(n, g, GuildCommand::Found);
    GuildFoundRequest {
        identity: common.identity,
        command: common.command,
        guild_id: common.guild_id,
        name: GuildName::parse(value).unwrap(),
    }
}
fn pre_delivery(root: &DurabilityRoot) -> SpellPremiumCoordinator {
    SpellPremiumCoordinator::new(Arc::default(), None, None, Some(root.clone()))
}
async fn found(
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
async fn current_coordinator(
    h: &Harness,
    who: [u8; 16],
    clock: bool,
) -> TestResult<(SpellPremiumCoordinator, Arc<PremiumConsumer>)> {
    use crate::premium::test_producer::{ProducerRequest, snapshot};
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
async fn found_footprint(h: &Harness) -> TestResult<Vec<i64>> {
    let mut counts = footprint(h).await?;
    counts.push(h.count("game_guilds").await?);
    counts.push(h.count("game_guild_ranks").await?);
    Ok(counts)
}

#[test]
fn found_defaults_and_creation_audit_are_closed() -> TestResult {
    assert_eq!(FORMATION_LIFETIME_MS, 259_200_000);
    assert_eq!(
        DEFAULT_RANKS,
        [(1, "Leader"), (2, "Vice Leader"), (3, "Member")]
    );
    assert!(GuildName::parse(&"a".repeat(29)).is_ok());
    assert!(GuildName::parse(&"a".repeat(30)).is_err());
    let audit = GuildAuditProposal {
        version: 1,
        occurrence: sid(1, 10),
        transaction_id: sid(1, 11),
        event_id: sid(1, 12),
        guild_id: *gid(1).as_bytes(),
        world_id: id(WORLD),
        actor: id(CHARACTER),
        target: id(CHARACTER),
        action: 6,
        previous_revision: 0,
        revision: 1,
        rank_before: None,
        rank_after: Some(1),
        removed_invitations: 0,
        occurred_at_unix_ms: 1,
    };
    assert!(audit.checked_bytes().is_some());
    let mut wrong = audit.clone();
    wrong.previous_revision = 1;
    wrong.revision = 2;
    assert!(wrong.checked_bytes().is_none());
    wrong = audit.clone();
    wrong.rank_after = Some(2);
    assert!(wrong.checked_bytes().is_none());
    wrong = audit.clone();
    wrong.target = sid(99, 1);
    assert!(wrong.checked_bytes().is_none());
    wrong = audit;
    wrong.action = 3;
    assert!(wrong.checked_bytes().is_none());
    assert!(!super::super::bank::fee_payer_is_junior(character(0)));
    assert!(super::super::bank::is_junior(
        super::super::bank::StarterIslandFact::OnIsland
    ));
    Ok(())
}

#[test]
fn native_found_creates_exact_snapshot_then_runs_all_five_membership_commands() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let h=Harness::create(admin,"guild_found_flow").await?;prepare(&h).await?;
        historical_character(&h,90,id(WORLD)).await?;historical_character(&h,91,id(WORLD)).await?;
        let target_fence=historical_session(&h,90).await?;guard(&h).await?;
        let before_revision:String=sqlx::query_scalar("SELECT character_revision::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid").bind(id(CHARACTER).as_slice()).fetch_one(&h.pool).await?;
        // Character names and Guild names intentionally occupy separate namespaces.
        let character_name:String=sqlx::query_scalar("SELECT name FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid").bind(id(CHARACTER).as_slice()).fetch_one(&h.pool).await?;
        let coordinator=pre_delivery(&h.root);let req=found_request(1,1,&character_name);
        let receipt=result(found(&h,req.clone(),fence()?,&coordinator).await?);assert_eq!(receipt.result,GuildResult::Ok);assert_eq!((receipt.previous_revision,receipt.revision),(Some(0),Some(1)));
        let initial:Vec<(i16,String)>=sqlx::query_as("SELECT level,name FROM game_guild_ranks ORDER BY level").fetch_all(&h.pool).await?;
        assert_eq!(initial,vec![(1,"Leader".into()),(2,"Vice Leader".into()),(3,"Member".into())]);
        let row=sqlx::query("SELECT state,revision::text,formation_deadline-created_at AS duration,vice_deficit_since,premium_deficit_since,motd FROM game_guilds").fetch_one(&h.pool).await?;
        assert_eq!(row.try_get::<i16,_>("state")?,1);assert_eq!(row.try_get::<String,_>("revision")?,"1");assert_eq!(row.try_get::<i64,_>("duration")?,259_200_000);assert_eq!(row.try_get::<Option<i64>,_>("vice_deficit_since")?,None);assert_eq!(row.try_get::<Option<i64>,_>("premium_deficit_since")?,None);assert_eq!(row.try_get::<String,_>("motd")?,"");
        assert_eq!(h.count("game_account_guild_leadership").await?,1);assert_eq!(h.count("game_guild_members").await?,1);
        assert_eq!(result(apply(&h,request(2,1,GuildCommand::Invite(character(90))),fence()?).await?).result,GuildResult::Ok);
        let mut accept=request(3,1,GuildCommand::Accept);accept.command=CommandRef::new(target_fence.game_session_id,CommandId::new(3)?);
        assert_eq!(result(apply(&h,accept,target_fence).await?).result,GuildResult::Ok);
        let joined:i16=sqlx::query_scalar("SELECT rank_level FROM game_guild_members WHERE character_id=encode($1,'hex')::uuid").bind(character(90).as_bytes().as_slice()).fetch_one(&h.pool).await?;assert_eq!(joined,3);
        let mut leave=request(4,1,GuildCommand::Leave);leave.command=CommandRef::new(target_fence.game_session_id,CommandId::new(4)?);
        assert_eq!(result(apply(&h,leave,target_fence).await?).result,GuildResult::Ok);
        assert_eq!(result(apply(&h,request(5,1,GuildCommand::Invite(character(90))),fence()?).await?).result,GuildResult::Ok);
        let mut accept=request(6,1,GuildCommand::Accept);accept.command=CommandRef::new(target_fence.game_session_id,CommandId::new(6)?);
        assert_eq!(result(apply(&h,accept,target_fence).await?).result,GuildResult::Ok);
        assert_eq!(result(apply(&h,request(7,1,GuildCommand::Exclude(character(90))),fence()?).await?).result,GuildResult::Ok);
        assert_eq!(result(apply(&h,request(8,1,GuildCommand::Invite(character(91))),fence()?).await?).result,GuildResult::Ok);
        assert_eq!(result(apply(&h,request(9,1,GuildCommand::Revoke(character(91))),fence()?).await?).result,GuildResult::Ok);
        let after_revision:String=sqlx::query_scalar("SELECT character_revision::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid").bind(id(CHARACTER).as_slice()).fetch_one(&h.pool).await?;assert_eq!(before_revision,after_revision);
        assert_eq!(h.count("game_guild_event_outbox").await?,9);assert_eq!(h.count("game_guild_operations").await?,9);
        h.cleanup().await
    })
}

#[test]
fn native_found_real_current_and_closed_premium_accounts() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        for (case, expected) in [
            (0, GuildResult::Ok),
            (1, GuildResult::NotPremium),
            (2, GuildResult::NotPremium),
            (3, GuildResult::NotPremium),
        ] {
            let h = Harness::create(admin.clone(), "guild_found_premium").await?;
            prepare(&h).await?;
            guard(&h).await?;
            let (coordinator, consumer) = current_coordinator(
                &h,
                if case == 2 { account(90) } else { account(0) },
                case != 1,
            )
            .await?;
            if case == 3 {
                let ticket = consumer.ticket();
                consumer.pull_failed(account(0), ticket);
            }
            assert_eq!(
                result(
                    found(
                        &h,
                        found_request(1, 1, "Founded Premium"),
                        fence()?,
                        &coordinator
                    )
                    .await?
                )
                .result,
                expected
            );
            assert_eq!(
                h.count("game_guilds").await?,
                i64::from(expected == GuildResult::Ok)
            );
            assert_eq!(
                h.count("game_guild_event_outbox").await?,
                i64::from(expected == GuildResult::Ok)
            );
            assert_eq!(
                h.count("game_premium_activation_latch").await?,
                if case == 1 { 0 } else { 1 }
            );
            h.cleanup().await?;
        }
        Ok(())
    })
}

#[test]
fn native_found_late_pre_delivery_refusal_rolls_back_savepoint_then_exact_retry() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let h=Harness::create(admin,"guild_found_late").await?;prepare(&h).await?;guard(&h).await?;
        h.pool.execute("CREATE FUNCTION guild_found_test_wrong_member() RETURNS TRIGGER LANGUAGE plpgsql AS $$BEGIN NEW.rank_level=2;RETURN NEW;END$$;CREATE TRIGGER aaa_guild_found_test_wrong_member BEFORE INSERT ON game_guild_members FOR EACH ROW EXECUTE FUNCTION guild_found_test_wrong_member();").await?;
        let before=found_footprint(&h).await?;let req=found_request(1,1,"Founded Rollback");let coordinator=pre_delivery(&h.root);
        let seal=h.recovery.seal_current().map_err(debug)?;let authority=h.root.open_character_authority(&seal).await.map_err(debug)?;
        assert!(matches!(h.root.commit_guild_found(&authority,&h.node,fence()?,req.clone(),&coordinator).await,Err(GuildError::Unavailable(_))));
        assert_eq!(found_footprint(&h).await?,before);assert_eq!(h.count("game_premium_activation_latch").await?,0);
        h.pool.execute("DROP TRIGGER aaa_guild_found_test_wrong_member ON game_guild_members;DROP FUNCTION guild_found_test_wrong_member();").await?;
        // A second single-field mutation remains a valid rank name under the generic schema,
        // so collect the real immediate Found-specific rank-guard message from server logs.
        // Neither this fixture trigger nor the generic rank-name CHECK raises that message.
        h.pool.execute("CREATE FUNCTION guild_found_test_wrong_rank_name() RETURNS TRIGGER LANGUAGE plpgsql AS $$BEGIN IF NEW.level=2 THEN NEW.name='Deputy';END IF;RETURN NEW;END$$;CREATE TRIGGER aaa_guild_found_test_wrong_rank_name BEFORE INSERT ON game_guild_ranks FOR EACH ROW EXECUTE FUNCTION guild_found_test_wrong_rank_name();").await?;
        assert!(matches!(h.root.commit_guild_found(&authority,&h.node,fence()?,req.clone(),&coordinator).await,Err(GuildError::Unavailable(_))));
        assert_eq!(found_footprint(&h).await?,before);
        h.pool.execute("DROP TRIGGER aaa_guild_found_test_wrong_rank_name ON game_guild_ranks;DROP FUNCTION guild_found_test_wrong_rank_name();").await?;
        // No operation survived: this exact fresh command really writes, rather than fast-replays.
        assert!(matches!(h.root.commit_guild_found(&authority,&h.node,fence()?,req.clone(),&coordinator).await.map_err(debug)?,GuildOutcome::Committed(_)));
        assert_eq!(h.count("game_guilds").await?,1);assert_eq!(h.count("game_guild_ranks").await?,3);assert_eq!(h.count("game_guild_event_outbox").await?,1);
        let closed=SpellPremiumCoordinator::new(Arc::default(),None,None,None);
        let saved=found_footprint(&h).await?;let mut stale=fence()?;stale.connection_generation=ConnectionGeneration::new(999)?;
        assert!(matches!(h.root.commit_guild_found(&authority,&h.node,stale,req.clone(),&closed).await.map_err(debug)?,GuildOutcome::Replayed(_)));assert_eq!(found_footprint(&h).await?,saved);
        let mut changed=req.clone();changed.name=GuildName::parse("Changed Request").unwrap();assert!(matches!(h.root.commit_guild_found(&authority,&h.node,fence()?,changed,&closed).await,Err(GuildError::ConflictingOccurrence)));
        let mut reused=found_request(2,2,"Reused Command");reused.command=req.command;assert!(matches!(h.root.commit_guild_found(&authority,&h.node,fence()?,reused,&closed).await,Err(GuildError::ConflictingOccurrence)));
        drop(authority);drop(seal);h.cleanup().await
    })
}

async fn wait_advisory(pool: &sqlx::PgPool, key: i64, granted: bool) -> TestResult {
    for _ in 0..100 {
        let present:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_locks WHERE locktype='advisory' AND database=(SELECT oid FROM pg_database WHERE datname=current_database()) AND objid::bigint=$1 AND granted=$2)")
            .bind(key & 0xffff_ffff).bind(granted).fetch_one(pool).await?;
        if present {
            return Ok(());
        }
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    }
    Err("expected genuine native advisory lock was not observed".into())
}

#[test]
fn native_found_shared_gate_transaction_commits_before_activation_latch() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let h=Harness::create(admin,"guild_found_latch").await?;prepare(&h).await?;guard(&h).await?;
        h.pool.execute("CREATE FUNCTION guild_found_test_barrier() RETURNS TRIGGER LANGUAGE plpgsql AS $$BEGIN PERFORM pg_advisory_xact_lock(901122);RETURN NEW;END$$;CREATE TRIGGER aaa_guild_found_test_barrier BEFORE INSERT ON game_guilds FOR EACH ROW EXECUTE FUNCTION guild_found_test_barrier();").await?;
        let mut barrier=h.pool.begin().await?;sqlx::query("SELECT pg_advisory_xact_lock(901122)").execute(&mut *barrier).await?;
        let other=DurabilityRoot::connect_test_runtime(&h.database.url)?;assert!(other.maintain_ready_once().await?);
        let seal=h.recovery.seal_current().map_err(debug)?;let authority=h.root.open_character_authority(&seal).await.map_err(debug)?;
        let coordinator=pre_delivery(&h.root);let req=found_request(1,1,"Founded Before Latch");
        let (created,latched)=fixture::join_two(h.root.commit_guild_found(&authority,&h.node,fence()?,req.clone(),&coordinator),async {
            wait_advisory(&h.pool,901122,false).await?;
            let latch_key=super::super::premium_activation_latch::PREMIUM_ACTIVATION_LOCK_KEY;
            wait_advisory(&h.pool,latch_key,true).await?;
            let (latched,released)=fixture::join_two(other.latch_premium_activation("guild-found-racing-activation",1,2),async {
                wait_advisory(&h.pool,latch_key,false).await?;barrier.rollback().await?;Ok::<_,Box<dyn std::error::Error>>(())
            }).await;released?;latched.map_err(|e| -> Box<dyn std::error::Error> { debug(e).into() })
        }).await;
        assert_eq!(result(created.map_err(debug)?).result,GuildResult::Ok);assert_eq!(latched?.activation_id,"guild-found-racing-activation");
        assert_eq!(h.count("game_guilds").await?,1);assert_eq!(h.count("game_premium_activation_latch").await?,1);
        // A new owner after the persisted switch cannot apply NotActivated, even with no clock.
        historical_character(&h,90,id(WORLD)).await?;let f=historical_session(&h,90).await?;
        let mut second=found_request(2,2,"Founded After Latch");second.command=CommandRef::new(f.game_session_id,CommandId::new(2)?);
        drop(authority);drop(seal);
        assert_eq!(result(found(&h,second,f,&pre_delivery(&h.root)).await?).result,GuildResult::NotPremium);
        assert!(matches!(found(&h,req,fence()?,&pre_delivery(&h.root)).await?,GuildOutcome::Replayed(_)));
        h.pool.execute("DROP TRIGGER aaa_guild_found_test_barrier ON game_guilds;DROP FUNCTION guild_found_test_barrier();").await?;
        drop(other);h.cleanup().await
    })
}

#[test]
fn native_found_name_membership_and_global_account_eligibility() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        for case in 0..5 {
            let h=Harness::create(admin.clone(),"guild_found_eligibility").await?;prepare(&h).await?;
            let other_world=if case==2||case==4 {sid(900,40)}else{id(WORLD)};
            historical_character_account(&h,90,other_world,if case==2 {account(0)}else{account(90)}).await?;
            let old_name="Al Dric";
            sqlx::query("INSERT INTO game_guilds(guild_id,world_id,name,state,formation_deadline,revision,created_at) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,$4,9999999999999,1,1)")
                .bind(gid(90).as_bytes().as_slice()).bind(other_world.as_slice()).bind(old_name).bind(if case==3 {4_i16}else{2}).execute(&h.pool).await?;
            for (level,value) in DEFAULT_RANKS {sqlx::query("INSERT INTO game_guild_ranks(guild_id,level,name) VALUES(encode($1,'hex')::uuid,$2,$3)").bind(gid(90).as_bytes().as_slice()).bind(i16::from(level)).bind(value).execute(&h.pool).await?;}
            if case!=3 {
                sqlx::query("INSERT INTO game_guild_members(character_id,guild_id,rank_level,title,joined_at) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,'',1)").bind(character(90).as_bytes().as_slice()).bind(gid(90).as_bytes().as_slice()).execute(&h.pool).await?;
                sqlx::query("INSERT INTO game_account_guild_leadership VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid)").bind(if case==2{account(0)}else{account(90)}.as_slice()).bind(gid(90).as_bytes().as_slice()).bind(character(90).as_bytes().as_slice()).execute(&h.pool).await?;
            }
            if case==1 {initial_member(&h,90,0,3).await?;}
            guard(&h).await?;
            let req=found_request(1,1,"ALDRIC");let expected=match case {0=>GuildResult::NameTaken,1=>GuildResult::AlreadyInGuild,2=>GuildResult::AccountHasPosition,_=>GuildResult::Ok};
            assert_eq!(result(found(&h,req,fence()?,&pre_delivery(&h.root)).await?).result,expected);
            assert_eq!(h.count("game_guild_operations").await?,1);assert_eq!(h.count("game_guild_event_outbox").await?,i64::from(expected==GuildResult::Ok));
            assert_eq!(h.count("game_guilds").await?,1+i64::from(expected==GuildResult::Ok));
            h.cleanup().await?;
        }Ok(())
    })
}

#[test]
fn native_found_current_authority_negatives_do_not_create_rows() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let h = Harness::create(admin, "guild_found_fences").await?;
        prepare(&h).await?;
        guard(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let coordinator = pre_delivery(&h.root);
        let before = found_footprint(&h).await?;
        for field in 0..3 {
            let mut f = fence()?;
            match field {
                0 => f.connection_generation = ConnectionGeneration::new(99)?,
                1 => f.character_lease_generation = 99,
                _ => f.scope_ownership_generation = ScopeOwnershipGeneration::new(99)?,
            }
            assert!(matches!(
                h.root
                    .commit_guild_found(
                        &authority,
                        &h.node,
                        f,
                        found_request(field + 1, 1, "Fenced Found"),
                        &coordinator
                    )
                    .await,
                Err(GuildError::NotCurrent)
            ));
            assert_eq!(found_footprint(&h).await?, before);
        }
        let other = DurabilityRoot::connect_test_runtime(&h.database.url)?;
        assert!(other.maintain_ready_once().await?);
        assert!(matches!(
            other
                .commit_guild_found(
                    &authority,
                    &h.node,
                    fence()?,
                    found_request(4, 1, "Wrong Root"),
                    &pre_delivery(&other)
                )
                .await,
            Err(GuildError::NotCurrent)
        ));
        let wrong = fixture::register(&h.root, 2).await?;
        assert!(matches!(
            h.root
                .commit_guild_found(
                    &authority,
                    &wrong,
                    fence()?,
                    found_request(5, 1, "Wrong Node"),
                    &coordinator
                )
                .await,
            Err(GuildError::NotCurrent)
        ));
        assert_eq!(found_footprint(&h).await?, before);
        assert_eq!(
            result(
                h.root
                    .commit_guild_found(
                        &authority,
                        &h.node,
                        fence()?,
                        found_request(6, 1, "Healthy Found"),
                        &coordinator
                    )
                    .await
                    .map_err(debug)?
            )
            .result,
            GuildResult::Ok
        );
        drop(other);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

#[test]
fn native_found_two_root_name_account_and_occurrence_races() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        for case in 0..3 {
            let h = Harness::create(admin.clone(), "guild_found_race").await?;
            prepare(&h).await?;
            let second_fence = if case == 0 {
                historical_character(&h, 90, id(WORLD)).await?;
                historical_session(&h, 90).await?
            } else {
                fence()?
            };
            guard(&h).await?;
            let other = DurabilityRoot::connect_test_runtime(&h.database.url)?;
            assert!(other.maintain_ready_once().await?);
            let seal = h.recovery.seal_current().map_err(debug)?;
            let a = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let b = other.open_character_authority(&seal).await.map_err(debug)?;
            let one = found_request(1, 1, "Racing Found");
            let mut two = if case == 2 {
                one.clone()
            } else {
                found_request(
                    2,
                    2,
                    if case == 0 {
                        "Racing Found"
                    } else {
                        "Other Found"
                    },
                )
            };
            if case == 0 {
                two.command = CommandRef::new(second_fence.game_session_id, CommandId::new(2)?);
            }
            let one_gate = pre_delivery(&h.root);
            let two_gate = pre_delivery(&other);
            let (one, two) = fixture::join_two(
                h.root
                    .commit_guild_found(&a, &h.node, fence()?, one, &one_gate),
                other.commit_guild_found(&b, &h.node, second_fence, two, &two_gate),
            )
            .await;
            let one = one.map_err(debug)?;
            let two = two.map_err(debug)?;
            if case == 2 {
                assert_eq!(
                    usize::from(matches!(&one, GuildOutcome::Committed(_)))
                        + usize::from(matches!(&two, GuildOutcome::Committed(_))),
                    1
                );
                assert_eq!(result(one), result(two));
            } else {
                let mut results = [result(one).result.code(), result(two).result.code()];
                results.sort();
                assert_eq!(
                    results,
                    [
                        0,
                        if case == 0 {
                            GuildResult::NameTaken.code()
                        } else {
                            GuildResult::AlreadyInGuild.code()
                        }
                    ]
                );
            }
            assert_eq!(h.count("game_guilds").await?, 1);
            assert_eq!(h.count("game_account_guild_leadership").await?, 1);
            assert_eq!(h.count("game_guild_event_outbox").await?, 1);
            // Native EXCLUSIVE admission relations serialize complete passes. Two simultaneous
            // current Characters of one Account are not fabricated: its native presence guard
            // has one Account PK. Cross-World Account eligibility is covered separately above.
            drop(b);
            drop(a);
            drop(seal);
            drop(other);
            h.cleanup().await?;
        }
        Ok(())
    })
}

#[test]
fn native_found_deferred_event_failure_is_atomic_then_fresh_root_retries_original() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let h=Harness::create(admin,"guild_found_event").await?;prepare(&h).await?;guard(&h).await?;
        h.pool.execute("CREATE FUNCTION guild_found_test_event_failure() RETURNS TRIGGER LANGUAGE plpgsql AS $$BEGIN RAISE EXCEPTION 'Found fixture event failure' USING ERRCODE='23514';END$$;CREATE CONSTRAINT TRIGGER guild_found_test_event_failure AFTER INSERT ON game_guild_event_outbox DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION guild_found_test_event_failure();").await?;
        let before=found_footprint(&h).await?;let req=found_request(1,1,"Atomic Found");let seal=h.recovery.seal_current().map_err(debug)?;let a=h.root.open_character_authority(&seal).await.map_err(debug)?;
        assert!(matches!(h.root.commit_guild_found(&a,&h.node,fence()?,req.clone(),&pre_delivery(&h.root)).await,Err(GuildError::Unavailable(_))));assert_eq!(found_footprint(&h).await?,before);
        h.pool.execute("DROP TRIGGER guild_found_test_event_failure ON game_guild_event_outbox;DROP FUNCTION guild_found_test_event_failure();").await?;
        let other=DurabilityRoot::connect_test_runtime(&h.database.url)?;assert!(other.maintain_ready_once().await?);let b=other.open_character_authority(&seal).await.map_err(debug)?;let gate=pre_delivery(&other);
        assert!(matches!(other.commit_guild_found(&b,&h.node,fence()?,req.clone(),&gate).await.map_err(debug)?,GuildOutcome::Committed(_)));
        assert!(matches!(other.commit_guild_found(&b,&h.node,fence()?,req,&gate).await.map_err(debug)?,GuildOutcome::Replayed(_)));
        assert_eq!(h.count("game_guilds").await?,1);assert_eq!(h.count("game_guild_ranks").await?,3);assert_eq!(h.count("game_guild_operations").await?,1);assert_eq!(h.count("game_guild_event_outbox").await?,1);
        let denied=sqlx::query("UPDATE game_guild_operations SET found_name='Mutated Found'").execute(&h.pool).await.expect_err("retained binding immutable");assert_eq!(denied.as_database_error().and_then(|e|e.code()).as_deref(),Some("23514"));
        drop(b);drop(other);drop(a);drop(seal);h.cleanup().await
    })
}

// Text operations append after the complete inherited 29 bodies/helpers. These fixtures
// install historical current rows independently; they do not construct authority capabilities.
async fn text_character_revision(h: &Harness) -> TestResult<String> {
    Ok(sqlx::query_scalar(
        "SELECT character_revision::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(id(CHARACTER).as_slice())
    .fetch_one(&h.pool)
    .await?)
}
fn text_request(n: u32, op: GuildCommand, expected: u64, text: &str) -> GuildTextRequest {
    GuildTextRequest::new(request(n, 1, op), expected, text).unwrap()
}
fn text_run<F>(tag: &'static str, body: F) -> TestResult
where
    F: AsyncFnOnce(&Harness) -> TestResult,
{
    let admin = configured_admin().ok_or("native Guild text PG route required")?;
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
async fn text_apply(h: &Harness, req: GuildTextRequest) -> TestResult<GuildOutcome> {
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
async fn text_snapshot(h: &Harness) -> TestResult<(String, String)> {
    let rows: String = sqlx::query_scalar("SELECT COALESCE(jsonb_agg(x ORDER BY x::text),'[]'::jsonb)::text FROM (SELECT jsonb_build_array('guild',to_jsonb(t)) x FROM game_guilds t UNION ALL SELECT jsonb_build_array('member',to_jsonb(t)) FROM game_guild_members t UNION ALL SELECT jsonb_build_array('rank',to_jsonb(t)) FROM game_guild_ranks t UNION ALL SELECT jsonb_build_array('invite',to_jsonb(t)) FROM game_guild_invitations t UNION ALL SELECT jsonb_build_array('position',to_jsonb(t)) FROM game_account_guild_leadership t UNION ALL SELECT jsonb_build_array('receipt',to_jsonb(t)) FROM game_guild_operations t UNION ALL SELECT jsonb_build_array('event',to_jsonb(t)) FROM game_guild_event_outbox t) r")
        .fetch_one(&h.pool).await?;
    Ok((rows, text_character_revision(h).await?))
}

#[test]
fn guild_text_native_parser_bounds_empty_title_and_closed_audit() -> TestResult {
    let title = "🦀".repeat(29);
    assert_eq!(
        text_request(1, GuildCommand::SetTitle(character(0)), 1, &title).text,
        title
    );
    assert_eq!(
        text_request(1, GuildCommand::SetTitle(character(0)), 1, "").text,
        ""
    );
    assert!(
        GuildTextRequest::new(
            request(1, 1, GuildCommand::SetTitle(character(0))),
            1,
            &"x".repeat(30)
        )
        .is_err()
    );
    assert!(
        GuildTextRequest::new(
            request(1, 1, GuildCommand::SetTitle(character(0))),
            1,
            "a\0b"
        )
        .is_err()
    );
    for input in ["", "   ", "control\nline", "\u{0085}"] {
        assert!(GuildTextRequest::new(request(1, 1, GuildCommand::SetMessage), 1, input).is_err());
    }
    assert!(
        GuildTextRequest::new(request(1, 1, GuildCommand::SetMessage), 1, &"x".repeat(256))
            .is_err()
    );
    assert!(
        GuildTextRequest::new(
            request(1, 1, GuildCommand::SetMessage),
            1,
            &format!("{}x", " ".repeat(1020))
        )
        .is_err()
    );
    assert!(GuildTextRequest::new(request(1, 1, GuildCommand::SetMessage), 0, "hello").is_err());
    assert!(GuildTextRequest::new(request(1, 1, GuildCommand::Leave), 1, "hello").is_err());
    assert_eq!(
        text_request(1, GuildCommand::SetMessage, 1, "  hello  ").text,
        "hello"
    );
    let text = "🦀".repeat(255);
    let req = text_request(1, GuildCommand::SetMessage, 1, &text);
    let mut audit = GuildTextAuditProposal {
        operation: GuildAuditProposal {
            version: 1,
            occurrence: req.common.identity.occurrence,
            transaction_id: req.common.identity.transaction_id,
            event_id: req.common.identity.event_id,
            guild_id: *gid(1).as_bytes(),
            world_id: id(WORLD),
            actor: *character(0).as_bytes(),
            target: *character(0).as_bytes(),
            action: 8,
            previous_revision: 1,
            revision: 2,
            rank_before: None,
            rank_after: None,
            removed_invitations: 0,
            occurred_at_unix_ms: 1,
        },
        expected_revision: 1,
        text,
    };
    let bytes = audit
        .checked_bytes()
        .ok_or("maximal valid message audit rejected")?;
    let value: serde_json::Value = serde_json::from_slice(&bytes)?;
    assert_eq!(value["text"].as_str(), Some(req.text.as_str()));
    assert_eq!(value["expected_revision"], 1);
    assert!(
        audit.operation.checked_bytes().is_none(),
        "old codec still refuses text-less new arms"
    );
    audit.expected_revision = 2;
    assert!(audit.checked_bytes().is_none());
    audit.expected_revision = 1;
    audit.operation.target = *character(90).as_bytes();
    assert!(audit.checked_bytes().is_none());
    audit.operation.target = *character(0).as_bytes();
    audit.text = " padded ".into();
    assert!(audit.checked_bytes().is_none());
    Ok(())
}

#[test]
fn guild_text_native_found_source_exact_title_message_and_clear_title() -> TestResult {
    text_run("guild_text_found", async |h| {
        guard(h).await?;
        assert_eq!(
            result(
                found(
                    h,
                    found_request(100, 1, "Text Guild"),
                    fence()?,
                    &pre_delivery(&h.root)
                )
                .await?
            )
            .result,
            GuildResult::Ok
        );
        let title = "🦀".repeat(29);
        let message = "🦀".repeat(255);
        assert_eq!(
            result(
                text_apply(
                    h,
                    text_request(101, GuildCommand::SetTitle(character(0)), 1, &title)
                )
                .await?
            )
            .revision,
            Some(2)
        );
        assert_eq!(
            result(text_apply(h, text_request(102, GuildCommand::SetMessage, 2, &message)).await?)
                .revision,
            Some(3)
        );
        let actual:(String,String,String,Option<i64>)=sqlx::query_as("SELECT motd,revision::text,title,vice_deficit_since FROM game_guilds JOIN game_guild_members USING(guild_id)").fetch_one(&h.pool).await?;
        assert_eq!(actual, (message, "3".into(), title, None));
        let op:(String,String,String)=sqlx::query_as("SELECT expected_revision::text,edit_text,revision::text FROM game_guild_operations WHERE action=8").fetch_one(&h.pool).await?;
        assert_eq!(op, ("2".into(), "🦀".repeat(255), "3".into()));
        let payload:Vec<u8>=sqlx::query_scalar("SELECT payload FROM game_guild_event_outbox e JOIN game_guild_operations o USING(occurrence) WHERE o.action=8").fetch_one(&h.pool).await?;
        let audit: serde_json::Value = serde_json::from_slice(&payload)?;
        assert_eq!(audit["text"].as_str(), Some("🦀".repeat(255).as_str()));
        assert_eq!(
            result(
                text_apply(
                    h,
                    text_request(103, GuildCommand::SetTitle(character(0)), 3, "")
                )
                .await?
            )
            .result,
            GuildResult::Ok
        );
        let title: String = sqlx::query_scalar("SELECT title FROM game_guild_members")
            .fetch_one(&h.pool)
            .await?;
        assert!(title.is_empty());
        assert_eq!(h.count("game_guild_operations").await?, 4);
        assert_eq!(h.count("game_guild_event_outbox").await?, 4);
        assert_eq!(text_character_revision(h).await?, "1");
        Ok(())
    })
}

#[test]
fn guild_text_native_current_role_target_and_world_are_read_not_supplied() -> TestResult {
    let admin = configured_admin().ok_or("native Guild text PG route required")?;
    runtime()?.block_on(async move {
        for role in 0..5 {
            let h=Harness::create(admin.clone(),"guild_text_roles").await?;
            let outcome: TestResult<()> = async {
                prepare(&h).await?;historical_character(&h,90,id(WORLD)).await?;historical_character(&h,91,id(WORLD)).await?;
                initial_guild(&h,1,if role==0 {0}else{90}).await?;
                initial_member(&h,1,91,3).await?;
                if role==1 || role==2 {initial_member(&h,1,0,if role==1 {2}else{3}).await?;}
                if role==4 {initial_guild(&h,2,0).await?;}
                guard(&h).await?;
                let title=result(text_apply(&h,text_request(1,GuildCommand::SetTitle(character(91)),1,"Target Title")).await?);
                assert_eq!(title.result,if role==0{GuildResult::Ok}else{GuildResult::NotAllowed});
                let revision=if role==0{2}else{1};
                let message=result(text_apply(&h,text_request(2,GuildCommand::SetMessage,revision,"Guild Message")).await?);
                assert_eq!(message.result,if role<=1{GuildResult::Ok}else{GuildResult::NotAllowed});
                let before=text_snapshot(&h).await?;
                let seal=h.recovery.seal_current().map_err(debug)?;let authority=h.root.open_character_authority(&seal).await.map_err(debug)?;
                let mut missing=text_request(3,GuildCommand::SetTitle(character(999)),revision,"Unknown Member");
                if role==0 {missing.expected_revision=3;}
                assert_eq!(result(h.root.commit_guild_text(&authority,&h.node,fence()?,missing).await.map_err(debug)?).result,GuildResult::NotAllowed);
                let title:String=sqlx::query_scalar("SELECT title FROM game_guild_members WHERE character_id=encode($1,'hex')::uuid").bind(character(91).as_bytes().as_slice()).fetch_one(&h.pool).await?;
                assert_eq!(title,if role==0{"Target Title"}else{""});
                // Same immutable requested Guild/actor under a foreign expected World never grants current authority.
                let mut foreign=fence()?;foreign.runtime_scope=crate::foundation::RuntimeScopeRefV1::channel(crate::foundation::WorldId::decode(&sid(7,91))?,crate::foundation::ChannelId::decode(&id(fixture::CHANNEL))?);
                assert!(matches!(h.root.commit_guild_text(&authority,&h.node,foreign,text_request(4,GuildCommand::SetMessage,revision,"No Foreign World")).await,Err(GuildError::NotCurrent)));
                assert_eq!(text_character_revision(&h).await?,"1");
                // Ordinary refusal receipts are expected; no foreign/current-authority error writes another receipt.
                assert_eq!(h.count("game_guild_event_outbox").await?,if role==0{2}else if role==1{1}else{0});
                assert_ne!(text_snapshot(&h).await?,before,"typed missing-target refusal is receipted");
                Ok(())
            }.await;
            h.cleanup().await?;outcome?;
        }
        for state in [3_i16,4] {
            let h=Harness::create(admin.clone(),"guild_text_closed_state").await?;
            let outcome: TestResult<()> = async {
                prepare(&h).await?;initial_guild(&h,1,0).await?;
                // A historical DISBANDED snapshot has no remaining members/positions.
                if state==4 {h.pool.execute("DELETE FROM game_account_guild_leadership;DELETE FROM game_guild_members").await?;}
                sqlx::query("UPDATE game_guilds SET state=$1").bind(state).execute(&h.pool).await?;
                guard(&h).await?;
                let before:String=sqlx::query_scalar("SELECT to_jsonb(g)::text FROM game_guilds g").fetch_one(&h.pool).await?;
                for (n,op) in [(1,GuildCommand::SetTitle(character(0))),(2,GuildCommand::SetMessage)] {
                    assert_eq!(result(text_apply(&h,text_request(n,op,1,"Closed Guild")).await?).result,GuildResult::GuildDisbanding);
                    let after:String=sqlx::query_scalar("SELECT to_jsonb(g)::text FROM game_guilds g").fetch_one(&h.pool).await?;
                    assert_eq!(after,before);
                }
                assert_eq!(h.count("game_guild_operations").await?,2);assert_eq!(h.count("game_guild_event_outbox").await?,0);assert_eq!(text_character_revision(&h).await?,"1");
                Ok(())
            }.await;
            h.cleanup().await?;outcome?;
        }
        Ok(())
    })
}

#[test]
fn guild_text_native_revision_cas_receipt_binding_and_restart_replay() -> TestResult {
    text_run("guild_text_receipt", async |h| {
        initial_guild(h, 1, 0).await?;
        guard(h).await?;
        let original = text_request(1, GuildCommand::SetMessage, 1, "Original Message");
        let committed = result(text_apply(h, original.clone()).await?);
        assert_eq!(committed.result, GuildResult::Ok);
        assert_eq!(
            result(
                text_apply(
                    h,
                    text_request(2, GuildCommand::SetTitle(character(0)), 1, "Stale")
                )
                .await?
            )
            .result,
            GuildResult::StaleRevision
        );
        assert_eq!(
            result(
                text_apply(
                    h,
                    text_request(3, GuildCommand::SetMessage, 2, "Newer Message")
                )
                .await?
            )
            .result,
            GuildResult::Ok
        );
        let title = text_request(4, GuildCommand::SetTitle(character(0)), 3, "Bound Target");
        assert_eq!(
            result(text_apply(h, title.clone()).await?).result,
            GuildResult::Ok
        );
        let before = text_snapshot(h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut changed_target = title.clone();
        changed_target.common.operation = GuildCommand::SetTitle(character(90));
        assert!(matches!(
            h.root
                .commit_guild_text(&authority, &h.node, fence()?, changed_target)
                .await,
            Err(GuildError::ConflictingOccurrence)
        ));
        let mut changed_command = title.clone();
        changed_command.common.identity =
            request(5, 1, GuildCommand::SetTitle(character(0))).identity;
        assert!(matches!(
            h.root
                .commit_guild_text(&authority, &h.node, fence()?, changed_command)
                .await,
            Err(GuildError::ConflictingOccurrence)
        ));
        assert_eq!(text_snapshot(h).await?, before);
        for mutation in 0..4 {
            let mut changed = original.clone();
            match mutation {
                0 => changed.text = "Changed".into(),
                1 => changed.expected_revision = 2,
                2 => changed.common.operation = GuildCommand::SetTitle(character(0)),
                _ => changed.common.guild_id = gid(2),
            }
            assert!(matches!(
                h.root
                    .commit_guild_text(&authority, &h.node, fence()?, changed)
                    .await,
                Err(GuildError::ConflictingOccurrence)
            ));
            assert_eq!(text_snapshot(h).await?, before);
        }
        assert!(matches!(
            h.root
                .commit_guild_membership(
                    &authority,
                    &h.node,
                    fence()?,
                    request(4, 1, GuildCommand::SetMessage)
                )
                .await,
            Err(GuildError::InvalidInput)
        ));
        let other = DurabilityRoot::connect_test_runtime(&h.database.url)?;
        assert!(other.maintain_ready_once().await?);
        let recovered = other.open_character_authority(&seal).await.map_err(debug)?;
        let replay = other
            .commit_guild_text(&recovered, &h.node, fence()?, original)
            .await
            .map_err(debug)?;
        assert!(matches!(&replay, GuildOutcome::Replayed(_)));
        assert_eq!(result(replay), committed);
        assert_eq!(text_snapshot(h).await?, before);
        drop(recovered);
        drop(other);
        Ok(())
    })
}

#[test]
fn guild_text_native_both_boundaries_current_fences_revoke_and_recovery_first() -> TestResult {
    text_run("guild_text_fences", async |h| {
        initial_guild(h, 1, 0).await?;
        guard(h).await?;
        let first = text_request(1, GuildCommand::SetTitle(character(0)), 1, "Current Title");
        text_apply(h, first.clone()).await?;
        let second = text_request(2, GuildCommand::SetMessage, 2, "Current Message");
        text_apply(h, second.clone()).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let before = text_snapshot(h).await?;
        // Bootstrap authorization is one-launch/one-use. Register the independently
        // current foreign Node once, then reuse that genuine proof for both boundaries.
        let wrong = fixture::register(&h.root, 2).await?;
        for op in [
            GuildCommand::SetTitle(character(0)),
            GuildCommand::SetMessage,
        ] {
            let req = text_request(3, op, 3, "Fresh Edit");
            for mutation in 0..4 {
                let mut f = fence()?;
                match mutation {
                    0 => f.connection_generation = ConnectionGeneration::new(2)?,
                    1 => f.character_lease_generation = 2,
                    2 => f.scope_ownership_generation = ScopeOwnershipGeneration::new(2)?,
                    _ => f.character_id = character(999),
                }
                assert!(matches!(
                    h.root
                        .commit_guild_text(&authority, &h.node, f, req.clone())
                        .await,
                    Err(GuildError::NotCurrent)
                ));
                assert_eq!(text_snapshot(h).await?, before);
            }
            assert!(matches!(
                h.root
                    .commit_guild_text(&authority, &wrong, fence()?, req.clone())
                    .await,
                Err(GuildError::NotCurrent)
            ));
            assert_eq!(text_snapshot(h).await?, before);
        }
        use crate::durability::runtime_scope_assignment::{
            AssignmentCommand, AssignmentOutcome, AssignmentRequest, AssignmentState, ControlActor,
            OperationKey, RuntimeScopeAssignmentWriter,
        };
        // The native fixture grants only Assign (operation1). Revoke must have its
        // own exact-scope operation3 grant for the same authenticated DB principal.
        let actor: String = sqlx::query_scalar("SELECT session_user")
            .fetch_one(&h.pool)
            .await?;
        assert_eq!(
            actor, "oteryn_test_admin",
            "native fixture's writer session principal"
        );
        sqlx::query("INSERT INTO game_control_scope_grants(control_role,world_id,channel_id,operation) VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,3)")
            .bind(id(WORLD).as_slice()).bind(id(fixture::CHANNEL).as_slice()).execute(&h.pool).await?;
        // Root lifetime is already bound to this native fixture writer registration.
        let writer = RuntimeScopeAssignmentWriter::open(h.root.clone(), "item-transfer-writer")
            .await
            .map_err(debug)?;
        let predecessor = h
            .root
            .read_runtime_scope_predecessor(fence()?.runtime_scope)
            .await
            .map_err(debug)?
            .ok_or("missing assignment")?;
        let revoked = writer
            .submit(&AssignmentRequest {
                operation_key: OperationKey::from_bytes([87; 32]),
                actor: ControlActor::new(&actor).map_err(debug)?,
                command: AssignmentCommand::Revoke {
                    scope: fence()?.runtime_scope,
                    predecessor,
                },
            })
            .await
            .map_err(debug)?;
        assert!(matches!(revoked, AssignmentOutcome::Committed(_)));
        let current = h
            .root
            .read_runtime_scope_assignment(fence()?.runtime_scope)
            .await
            .map_err(debug)?
            .ok_or("missing revoked assignment")?;
        assert_eq!(current.state, AssignmentState::Revoked);
        assert_eq!(current.ownership_generation, 2);
        for op in [
            GuildCommand::SetTitle(character(0)),
            GuildCommand::SetMessage,
        ] {
            assert!(matches!(
                h.root
                    .commit_guild_text(
                        &authority,
                        &h.node,
                        fence()?,
                        text_request(3, op, 3, "Revoked")
                    )
                    .await,
                Err(GuildError::NotCurrent)
            ));
        }
        for req in [first, second] {
            assert!(matches!(
                h.root
                    .commit_guild_text(&authority, &h.node, fence()?, req)
                    .await
                    .map_err(debug)?,
                GuildOutcome::Replayed(_)
            ));
        }
        assert_eq!(text_snapshot(h).await?, before);
        let wrong_root = DurabilityRoot::connect_test_runtime(&h.database.url)?;
        assert!(wrong_root.maintain_ready_once().await?);
        assert!(matches!(
            wrong_root
                .commit_guild_text(
                    &authority,
                    &h.node,
                    fence()?,
                    text_request(1, GuildCommand::SetTitle(character(0)), 1, "Current Title")
                )
                .await,
            Err(GuildError::NotCurrent)
        ));
        // Sealed-store generation locks deliberately prevent manufacturing a stale opaque
        // Root capability. Negative expected recovery DATA exercises the SAME production
        // transaction prelude before its historical receipt, without changing native rows.
        let mut wrong_recovery = authority.record_for(&h.root).map_err(debug)?;
        wrong_recovery.recovery_generation += 1;
        let req = text_request(1, GuildCommand::SetTitle(character(0)), 1, "Current Title");
        let mut digest = Sha256::new();
        digest.update(b"oteryn:guild-text-proposal:v1");
        digest.update(binding(&fence()?, &req.common).map_err(debug)?);
        digest.update(req.expected_revision.to_be_bytes());
        digest.update(req.text.as_bytes());
        let binding = digest.finalize().to_vec();
        let mut tx = h.pool.begin().await?;
        assert!(matches!(
            execute_fenced_operation(
                &mut tx,
                &wrong_recovery,
                &h.node,
                &fence()?,
                &req.common,
                &binding,
                Some(&req)
            )
            .await,
            Err(GuildError::Unavailable(_))
        ));
        tx.rollback().await?;
        assert_eq!(text_snapshot(h).await?, before);
        drop(wrong_root);
        Ok(())
    })
}

#[test]
fn guild_text_native_closed_field_proofs_deferred_failure_and_overflow_rollback() -> TestResult {
    text_run("guild_text_rollback", async |h| {
        initial_guild(h, 1, 0).await?;
        guard(h).await?;
        h.pool.execute("CREATE FUNCTION guild_text_fault() RETURNS TRIGGER LANGUAGE plpgsql AS $$BEGIN RAISE EXCEPTION 'injected text event failure' USING ERRCODE='23514';END$$;CREATE CONSTRAINT TRIGGER guild_text_fault AFTER INSERT ON game_guild_event_outbox DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION guild_text_fault()").await?;
        for op in [
            GuildCommand::SetTitle(character(0)),
            GuildCommand::SetMessage,
        ] {
            let before = text_snapshot(h).await?;
            assert!(
                text_apply(h, text_request(1, op, 1, "Atomic Text"))
                    .await
                    .is_err()
            );
            assert_eq!(text_snapshot(h).await?, before);
        }
        h.pool.execute("DROP TRIGGER guild_text_fault ON game_guild_event_outbox;DROP FUNCTION guild_text_fault()").await?;
        h.pool.execute("CREATE FUNCTION guild_text_wrong_field() RETURNS trigger LANGUAGE plpgsql AS $$BEGIN NEW.vice_deficit_since=999;RETURN NEW;END$$;CREATE TRIGGER aaa_guild_text_wrong_field BEFORE UPDATE ON game_guilds FOR EACH ROW EXECUTE FUNCTION guild_text_wrong_field()").await?;
        for op in [
            GuildCommand::SetTitle(character(0)),
            GuildCommand::SetMessage,
        ] {
            let before = text_snapshot(h).await?;
            assert!(
                text_apply(h, text_request(1, op, 1, "No Unrelated Field"))
                    .await
                    .is_err()
            );
            assert_eq!(text_snapshot(h).await?, before);
        }
        h.pool.execute("DROP TRIGGER aaa_guild_text_wrong_field ON game_guilds;DROP FUNCTION guild_text_wrong_field()").await?;
        let denied = sqlx::query("UPDATE game_guild_members SET title='unproved'")
            .execute(&h.pool)
            .await
            .expect_err("no current-xid operation");
        assert_eq!(
            denied.as_database_error().and_then(|e| e.code()).as_deref(),
            Some("23514")
        );
        assert_eq!(
            result(
                text_apply(
                    h,
                    text_request(1, GuildCommand::SetTitle(character(0)), 1, "Atomic Title")
                )
                .await?
            )
            .result,
            GuildResult::Ok
        );
        assert_eq!(
            result(
                text_apply(
                    h,
                    text_request(2, GuildCommand::SetMessage, 2, "Atomic Message")
                )
                .await?
            )
            .result,
            GuildResult::Ok
        );
        assert_eq!(h.count("game_guild_operations").await?, 2);
        assert_eq!(h.count("game_guild_event_outbox").await?, 2);
        assert_eq!(text_character_revision(h).await?, "1");
        Ok(())
    })?;
    text_run("guild_text_overflow", async |h| {
        initial_guild(h, 1, 0).await?;
        sqlx::query("UPDATE game_guilds SET revision=18446744073709551615")
            .execute(&h.pool)
            .await?;
        guard(h).await?;
        for op in [
            GuildCommand::SetTitle(character(0)),
            GuildCommand::SetMessage,
        ] {
            let before = text_snapshot(h).await?;
            assert!(
                text_apply(h, text_request(1, op, u64::MAX, "Overflow"))
                    .await
                    .is_err()
            );
            assert_eq!(text_snapshot(h).await?, before);
        }
        Ok(())
    })
}

#[test]
fn guild_text_native_two_root_revision_and_occurrence_serialization() -> TestResult {
    text_run("guild_text_race", async |h| {
        initial_guild(h, 1, 0).await?;
        guard(h).await?;
        let other = DurabilityRoot::connect_test_runtime(&h.database.url)?;
        assert!(other.maintain_ready_once().await?);
        let seal = h.recovery.seal_current().map_err(debug)?;
        let first = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let second = other.open_character_authority(&seal).await.map_err(debug)?;
        let a = text_request(1, GuildCommand::SetTitle(character(0)), 1, "Racing Title");
        let b = text_request(2, GuildCommand::SetMessage, 1, "Racing Message");
        let (a, b) = fixture::join_two(
            h.root.commit_guild_text(&first, &h.node, fence()?, a),
            other.commit_guild_text(&second, &h.node, fence()?, b),
        )
        .await;
        let results = [result(a.map_err(debug)?), result(b.map_err(debug)?)];
        assert_eq!(
            results
                .iter()
                .filter(|r| r.result == GuildResult::Ok)
                .count(),
            1
        );
        assert_eq!(
            results
                .iter()
                .filter(|r| r.result == GuildResult::StaleRevision)
                .count(),
            1
        );
        assert_eq!(h.count("game_guild_event_outbox").await?, 1);
        assert_eq!(h.count("game_guild_operations").await?, 2);
        let same = text_request(3, GuildCommand::SetMessage, 2, "One Exact Edit");
        let (a, b) = fixture::join_two(
            h.root
                .commit_guild_text(&first, &h.node, fence()?, same.clone()),
            other.commit_guild_text(&second, &h.node, fence()?, same),
        )
        .await;
        let a = a.map_err(debug)?;
        let b = b.map_err(debug)?;
        assert_eq!(
            usize::from(matches!(&a, GuildOutcome::Committed(_)))
                + usize::from(matches!(&b, GuildOutcome::Committed(_))),
            1
        );
        assert_eq!(result(a), result(b));
        assert_eq!(h.count("game_guild_event_outbox").await?, 2);
        assert_eq!(h.count("game_guild_operations").await?, 3);
        assert_eq!(text_character_revision(h).await?, "1");
        // The unchanged native admission relation locks serialize full passes too; this
        // proves two genuine Root results, not isolated row-lock/load performance.
        drop(second);
        drop(first);
        drop(other);
        Ok(())
    })
}
