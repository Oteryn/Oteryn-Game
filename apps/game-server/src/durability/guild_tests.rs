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
    sqlx::query("INSERT INTO game_character_account_guards VALUES(encode($1,'hex')::uuid)")
        .bind(account(n).as_slice())
        .execute(&h.pool)
        .await?;
    sqlx::query("INSERT INTO game_character_roots(character_id,account_id,world_id,lifecycle,character_revision,profile_revision,ruleset_revision,content_revision,starter_template_revision,name) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,1,1,'profile-1','ruleset-1','content-1','starter-1',$4)")
        .bind(character(n).as_bytes().as_slice()).bind(account(n).as_slice()).bind(world.as_slice()).bind(name(n)).execute(&h.pool).await?;
    let operation = sid(n, 30);
    let decision = sid(n, 31);
    let raw = serde_json::to_vec(
        &serde_json::json!({"contract_version":2,"variant":"OPERATOR_CONTROL_PLANE_BOOTSTRAP",
        "issuer_authority":"OTERYN_PLATFORM_CHARACTER_AUTHORITY","issuer_decision_id":uuid(decision),
        "source_revision":(u64::from(n)+100).to_string(),"operation_id":uuid(operation),"operation":"INITIAL_CHARACTER_BOOTSTRAP",
        "account_id":uuid(account(n)),"target_world_id":uuid(world),"requested_name":name(n),
        "interpretation_context":{"profile_revision":"profile-1","ruleset_revision":"ruleset-1","content_revision":"content-1","starter_template_revision":"starter-1"},
        "issued_at_source":"1","expires_at_source":"120","audience":"OTERYN_GAME_CHARACTER_AUTHORITY"}),
    )?;
    let intent = crate::character_bootstrap_intent::decode_producer_response(&raw, operation)
        .map_err(debug)?;
    sqlx::query("INSERT INTO game_character_operation_receipts(operation_id,command_binding,account_id,character_id,world_id,character_revision,event_id,occurred_at,server_build_id,transaction_id,issuer_decision_id,intent_source_revision,issued_at_source,expires_at_source) VALUES(encode($1,'hex')::uuid,$2,encode($3,'hex')::uuid,encode($4,'hex')::uuid,encode($5,'hex')::uuid,1,encode($6,'hex')::uuid,1,'test/1',encode($7,'hex')::uuid,encode($8,'hex')::uuid,$9,1,120)")
        .bind(operation.as_slice()).bind(intent.binding()).bind(account(n).as_slice()).bind(character(n).as_bytes().as_slice()).bind(world.as_slice())
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
