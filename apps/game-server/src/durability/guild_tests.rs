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

// SetRank appends after the complete 96,638-byte inherited owner/helper prefix.
// Historical rows below are fixtures, never Premium/session/role authority supplied to a command.
fn rank_request(n: u32, target: u32, revision: u64, level: u8) -> GuildRankRequest {
    GuildRankRequest::new(
        request(n, 1, GuildCommand::SetRank(character(target))),
        revision,
        level,
    )
    .unwrap()
}
async fn rank_apply(
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
async fn rank_mutation_snapshot(h: &Harness) -> TestResult<String> {
    Ok(sqlx::query_scalar("SELECT COALESCE(jsonb_agg(x ORDER BY x::text),'[]'::jsonb)::text FROM (SELECT jsonb_build_array('guild',to_jsonb(t)) x FROM game_guilds t UNION ALL SELECT jsonb_build_array('member',to_jsonb(t)) FROM game_guild_members t UNION ALL SELECT jsonb_build_array('rank',to_jsonb(t)) FROM game_guild_ranks t UNION ALL SELECT jsonb_build_array('invite',to_jsonb(t)) FROM game_guild_invitations t UNION ALL SELECT jsonb_build_array('position',to_jsonb(t)) FROM game_account_guild_leadership t UNION ALL SELECT jsonb_build_array('event',to_jsonb(t)) FROM game_guild_event_outbox t) r")
        .fetch_one(&h.pool).await?)
}
async fn rank_level(h: &Harness, target: u32) -> TestResult<i16> {
    Ok(sqlx::query_scalar(
        "SELECT rank_level FROM game_guild_members WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(character(target).as_bytes().as_slice())
    .fetch_one(&h.pool)
    .await?)
}
async fn rank_fixture(h: &Harness, vice_count: u32) -> TestResult {
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

#[test]
fn guild_rank_native_request_binding_and_closed_audit() -> TestResult {
    for level in [0, 21, u8::MAX] {
        assert!(
            GuildRankRequest::new(
                request(1, 1, GuildCommand::SetRank(character(91))),
                1,
                level
            )
            .is_err()
        );
    }
    for level in [1, 20] {
        assert_eq!(rank_request(1, 91, 1, level).level.level(), level);
    }
    assert!(
        GuildRankRequest::new(request(1, 1, GuildCommand::SetRank(character(91))), 0, 2).is_err()
    );
    assert!(
        GuildRankRequest::new(request(1, 1, GuildCommand::SetTitle(character(91))), 1, 2).is_err()
    );
    let original = rank_request(1, 91, 1, 2);
    let binding = rank_binding(&fence()?, &original).map_err(debug)?;
    let mut alternatives = vec![
        rank_request(1, 92, 1, 2),
        rank_request(1, 91, 2, 2),
        rank_request(1, 91, 1, 3),
    ];
    let mut other = original.clone();
    other.common.guild_id = gid(2);
    alternatives.push(other);
    let mut other = original.clone();
    other.common.operation = GuildCommand::SetTitle(character(91));
    alternatives.push(other);
    let mut other = original.clone();
    other.common.command = request(2, 1, GuildCommand::SetRank(character(91))).command;
    alternatives.push(other);
    for changed in alternatives {
        assert_ne!(rank_binding(&fence()?, &changed).map_err(debug)?, binding);
    }
    let audit = GuildRankAuditProposal {
        operation: GuildAuditProposal {
            version: 1,
            occurrence: sid(1, 10),
            transaction_id: sid(1, 11),
            event_id: sid(1, 12),
            guild_id: *gid(1).as_bytes(),
            world_id: id(WORLD),
            actor: id(CHARACTER),
            target: *character(91).as_bytes(),
            action: 9,
            previous_revision: 1,
            revision: 2,
            rank_before: Some(3),
            rank_after: Some(2),
            removed_invitations: 0,
            occurred_at_unix_ms: 1,
        },
        expected_revision: 1,
    };
    let bytes = audit.checked_bytes().ok_or("valid rank audit rejected")?;
    assert_eq!(
        serde_json::from_slice::<GuildRankAuditProposal>(&bytes)?,
        audit
    );
    let mut wrong = audit.clone();
    wrong.expected_revision = 2;
    assert!(wrong.checked_bytes().is_none());
    let mut wrong = audit.clone();
    wrong.operation.action = 7;
    assert!(wrong.checked_bytes().is_none());
    let mut wrong = audit.clone();
    wrong.operation.rank_after = Some(1);
    assert!(wrong.checked_bytes().is_none());
    let mut wrong = audit.clone();
    wrong.operation.rank_before = None;
    assert!(wrong.checked_bytes().is_none());
    let mut wrong = audit.clone();
    wrong.operation.actor = wrong.operation.target;
    assert!(wrong.checked_bytes().is_none());
    let mut json: serde_json::Value = serde_json::from_slice(&bytes)?;
    json["caller_premium"] = true.into();
    assert!(serde_json::from_value::<GuildRankAuditProposal>(json).is_err());
    Ok(())
}

#[test]
fn guild_rank_native_target_premium_and_position_transitions() -> TestResult {
    // GUILD0 §3.2: only the newly promoted TARGET Account consumes Current/NotActivated.
    for case in 0..5 {
        text_run("guild_rank_premium", async |h| {
            rank_fixture(h, 0).await?;
            guard(h).await?;
            let who = if case == 1 { account(0) } else { account(91) };
            let (coordinator, consumer) = current_coordinator(h, who, case != 2).await?;
            if case == 3 {
                consumer.pull_failed(account(91), consumer.ticket());
            }
            let pre = pre_delivery(&h.root);
            let premium = if case == 4 { &pre } else { &coordinator };
            let before = rank_mutation_snapshot(h).await?;
            let outcome = result(rank_apply(h, rank_request(1, 91, 1, 2), premium).await?);
            let success = case == 0 || case == 4;
            assert_eq!(
                outcome.result,
                if success {
                    GuildResult::Ok
                } else {
                    GuildResult::NotPremium
                }
            );
            if !success {
                assert_eq!(rank_mutation_snapshot(h).await?, before);
                assert_eq!(h.count("game_guild_operations").await?, 1);
            } else {
                assert_eq!(rank_level(h, 91).await?, 2);
                assert_eq!(h.count("game_account_guild_leadership").await?, 2);
                // Once the role exists, unavailable Premium cannot revoke it or block demotion.
                let closed =
                    SpellPremiumCoordinator::new(Arc::default(), None, None, Some(h.root.clone()));
                assert_eq!(
                    result(rank_apply(h, rank_request(2, 91, 2, 2), &closed).await?).result,
                    GuildResult::Ok
                );
                assert_eq!(h.count("game_account_guild_leadership").await?, 2);
                assert_eq!(
                    result(rank_apply(h, rank_request(3, 91, 3, 3), &closed).await?).result,
                    GuildResult::Ok
                );
                assert_eq!(rank_level(h, 91).await?, 3);
                assert_eq!(h.count("game_account_guild_leadership").await?, 1);
                assert_eq!(revision(h, 1).await?, "4");
                assert_eq!(h.count("game_guild_event_outbox").await?, 3);
                let audit:Vec<u8>=sqlx::query_scalar("SELECT payload FROM game_guild_event_outbox ORDER BY occurred_at,event_id LIMIT 1").fetch_one(&h.pool).await?;
                let audit: GuildRankAuditProposal = serde_json::from_slice(&audit)?;
                assert_eq!(audit.operation.action, 9);
                assert_eq!(audit.operation.rank_before, Some(3));
                assert_eq!(audit.operation.rank_after, Some(2));
                assert_eq!(audit.expected_revision, 1);
                let unchanged:bool=sqlx::query_scalar("SELECT title='' AND joined_at=1 FROM game_guild_members WHERE character_id=encode($1,'hex')::uuid")
                    .bind(character(91).as_bytes().as_slice()).fetch_one(&h.pool).await?;
                assert!(unchanged);
            }
            assert_eq!(text_character_revision(h).await?, "1");
            let deficit: Option<i64> =
                sqlx::query_scalar("SELECT premium_deficit_since FROM game_guilds")
                    .fetch_one(&h.pool)
                    .await?;
            assert_eq!(deficit, Some(7));
            Ok(())
        })?;
    }
    Ok(())
}

#[test]
fn guild_rank_native_both_strict_role_inequalities_and_existing_rank() -> TestResult {
    // Numeric role authority is resolved from current locked rows, including ordinary rank3.
    for (actor, target, desired, expected) in [
        (1, 3, 2, GuildResult::Ok),
        (2, 3, 2, GuildResult::NotAllowed),
        (2, 3, 4, GuildResult::Ok),
        (3, 4, 4, GuildResult::Ok),
        (3, 3, 4, GuildResult::NotAllowed),
        (3, 4, 3, GuildResult::NotAllowed),
        (1, 3, 1, GuildResult::NotAllowed),
        (1, 3, 20, GuildResult::NotAllowed),
    ] {
        text_run("guild_rank_roles", async |h| {
            historical_character(h, 90, id(WORLD)).await?;
            historical_character(h, 91, id(WORLD)).await?;
            // Actor0 is the leader only in the actor1 case; all other ranks use a real other leader.
            initial_guild(h, 1, if actor == 1 { 0 } else { 90 }).await?;
            if actor != 1 {
                initial_member(h, 1, 0, actor).await?;
            }
            initial_member(h, 1, 91, target).await?;
            guard(h).await?;
            let before = rank_mutation_snapshot(h).await?;
            assert_eq!(
                result(
                    rank_apply(h, rank_request(1, 91, 1, desired), &pre_delivery(&h.root)).await?
                )
                .result,
                expected
            );
            if expected == GuildResult::Ok {
                assert_eq!(rank_level(h, 91).await?, i16::from(desired));
                assert_eq!(revision(h, 1).await?, "2");
            } else {
                assert_eq!(rank_mutation_snapshot(h).await?, before);
            }
            assert_eq!(text_character_revision(h).await?, "1");
            Ok(())
        })?;
    }
    text_run("guild_rank_scope", async |h| {
        initial_guild(h, 1, 0).await?;
        historical_character(h, 90, id(WORLD)).await?;
        initial_guild(h, 2, 90).await?;
        historical_character(h, 91, id(WORLD)).await?;
        initial_member(h, 2, 91, 3).await?;
        historical_character(h, 92, sid(900, 40)).await?;
        guard(h).await?;
        for (n, target) in [(1, 0), (2, 91), (3, 92), (4, 999)] {
            let before = rank_mutation_snapshot(h).await?;
            assert_eq!(
                result(rank_apply(h, rank_request(n, target, 1, 2), &pre_delivery(&h.root)).await?)
                    .result,
                GuildResult::NotAllowed
            );
            assert_eq!(rank_mutation_snapshot(h).await?, before);
        }
        Ok(())
    })
}

#[test]
fn guild_rank_native_formation_and_first_deficit_cure() -> TestResult {
    for forming in [false, true] {
        text_run("guild_rank_lifecycle", async |h| {
            rank_fixture(h, 3).await?;
            sqlx::query("UPDATE game_guilds SET state=$1")
                .bind(if forming { 1_i16 } else { 2 })
                .execute(&h.pool)
                .await?;
            guard(h).await?;
            // The three retained vices have no Current Premium snapshot; all three still count.
            let (premium, _) = current_coordinator(h, account(91), true).await?;
            assert_eq!(
                result(rank_apply(h, rank_request(1, 91, 1, 2), &premium).await?).result,
                GuildResult::Ok
            );
            let row: (i16, Option<i64>, Option<i64>) = sqlx::query_as(
                "SELECT state,vice_deficit_since,premium_deficit_since FROM game_guilds",
            )
            .fetch_one(&h.pool)
            .await?;
            assert_eq!(row, (2, None, Some(7)));
            assert_eq!(
                result(rank_apply(h, rank_request(2, 91, 2, 3), &premium).await?).result,
                GuildResult::Ok
            );
            let first: Option<i64> =
                sqlx::query_scalar("SELECT vice_deficit_since FROM game_guilds")
                    .fetch_one(&h.pool)
                    .await?;
            assert!(first.is_some_and(|t| t > 0));
            assert_eq!(
                result(rank_apply(h, rank_request(3, 100, 3, 3), &premium).await?).result,
                GuildResult::Ok
            );
            let second: Option<i64> =
                sqlx::query_scalar("SELECT vice_deficit_since FROM game_guilds")
                    .fetch_one(&h.pool)
                    .await?;
            assert_eq!(second, first);
            assert_eq!(
                result(rank_apply(h, rank_request(4, 91, 4, 2), &premium).await?).result,
                GuildResult::Ok
            );
            let still: Option<i64> =
                sqlx::query_scalar("SELECT vice_deficit_since FROM game_guilds")
                    .fetch_one(&h.pool)
                    .await?;
            assert_eq!(still, first); // three is still a deficit.
            let (last, _) = current_coordinator(h, account(100), true).await?;
            assert_eq!(
                result(rank_apply(h, rank_request(5, 100, 5, 2), &last).await?).result,
                GuildResult::Ok
            );
            let cleared: Option<i64> =
                sqlx::query_scalar("SELECT vice_deficit_since FROM game_guilds")
                    .fetch_one(&h.pool)
                    .await?;
            assert_eq!(cleared, None);
            assert_eq!(revision(h, 1).await?, "6");
            assert_eq!(text_character_revision(h).await?, "1");
            Ok(())
        })?;
    }
    // Curative commands cannot outrun an expired formation/14-day deadline; refusals retain receipts only.
    for state in [1_i16, 2] {
        text_run("guild_rank_late_cure", async |h| {
            rank_fixture(h, 3).await?;
            sqlx::query(
                "UPDATE game_guilds SET state=$1,formation_deadline=1,vice_deficit_since=1",
            )
            .bind(state)
            .execute(&h.pool)
            .await?;
            guard(h).await?;
            let before = rank_mutation_snapshot(h).await?;
            assert_eq!(
                result(rank_apply(h, rank_request(1, 91, 1, 2), &pre_delivery(&h.root)).await?)
                    .result,
                GuildResult::GuildDeadlinePassed
            );
            assert_eq!(rank_mutation_snapshot(h).await?, before);
            assert_eq!(
                result(rank_apply(h, rank_request(2, 100, 1, 3), &pre_delivery(&h.root)).await?)
                    .result,
                GuildResult::Ok
            );
            let deficit: Option<i64> =
                sqlx::query_scalar("SELECT vice_deficit_since FROM game_guilds")
                    .fetch_one(&h.pool)
                    .await?;
            assert_eq!(deficit, Some(1));
            Ok(())
        })?;
    }
    Ok(())
}

#[test]
fn guild_rank_native_replay_conflicts_cas_and_restart() -> TestResult {
    text_run("guild_rank_replay", async |h| {
        rank_fixture(h, 0).await?;
        guard(h).await?;
        let req = rank_request(1, 91, 1, 2);
        let first = result(rank_apply(h, req.clone(), &pre_delivery(&h.root)).await?);
        assert_eq!(first.result, GuildResult::Ok);
        // A later demotion changes target rank/current Premium and revision before historical replay.
        assert_eq!(
            result(rank_apply(h, rank_request(2, 91, 2, 3), &pre_delivery(&h.root)).await?).result,
            GuildResult::Ok
        );
        let before = text_snapshot(h).await?;
        let other = DurabilityRoot::connect_test_runtime(&h.database.url)?;
        assert!(other.maintain_ready_once().await?);
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = other.open_character_authority(&seal).await.map_err(debug)?;
        let closed = SpellPremiumCoordinator::new(Arc::default(), None, None, None);
        let mut stale = fence()?;
        stale.connection_generation = ConnectionGeneration::new(999)?;
        let replay = other
            .commit_guild_rank(&authority, &h.node, stale, req.clone(), &closed)
            .await
            .map_err(debug)?;
        assert!(matches!(&replay, GuildOutcome::Replayed(_)));
        assert_eq!(result(replay), first);
        assert_eq!(text_snapshot(h).await?, before);
        for field in 0..6 {
            let mut changed = req.clone();
            match field {
                0 => changed.level = GuildRank::new(3).unwrap(),
                1 => changed.expected_revision = 2,
                2 => changed.common.operation = GuildCommand::SetRank(character(100)),
                3 => changed.common.guild_id = gid(2),
                4 => changed.common.operation = GuildCommand::SetTitle(character(91)),
                _ => {
                    changed.common.command =
                        request(10, 1, GuildCommand::SetRank(character(91))).command
                }
            }
            assert!(matches!(
                other
                    .commit_guild_rank(&authority, &h.node, fence()?, changed, &closed)
                    .await,
                Err(GuildError::ConflictingOccurrence)
            ));
            assert_eq!(text_snapshot(h).await?, before);
        }
        let mut reused = rank_request(3, 91, 3, 2);
        reused.common.command = req.common.command;
        assert!(matches!(
            other
                .commit_guild_rank(&authority, &h.node, fence()?, reused, &closed)
                .await,
            Err(GuildError::ConflictingOccurrence)
        ));
        drop(authority);
        drop(other);
        let before = rank_mutation_snapshot(h).await?;
        assert_eq!(
            result(rank_apply(h, rank_request(4, 91, 1, 2), &pre_delivery(&h.root)).await?).result,
            GuildResult::StaleRevision
        );
        assert_eq!(rank_mutation_snapshot(h).await?, before);
        assert_eq!(h.count("game_guild_operations").await?, 3);
        Ok(())
    })?;
    text_run("guild_rank_overflow", async |h| {
        rank_fixture(h, 0).await?;
        sqlx::query("UPDATE game_guilds SET revision=18446744073709551615")
            .execute(&h.pool)
            .await?;
        guard(h).await?;
        let before = text_snapshot(h).await?;
        assert!(
            rank_apply(h, rank_request(1, 91, u64::MAX, 2), &pre_delivery(&h.root))
                .await
                .is_err()
        );
        assert_eq!(text_snapshot(h).await?, before);
        Ok(())
    })
}

#[test]
fn guild_rank_native_two_roots_revision_and_occurrence_serialization() -> TestResult {
    text_run("guild_rank_revision_race", async |h| {
        rank_fixture(h, 0).await?;
        guard(h).await?;
        let other = DurabilityRoot::connect_test_runtime(&h.database.url)?;
        assert!(other.maintain_ready_once().await?);
        let seal = h.recovery.seal_current().map_err(debug)?;
        let a = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let b = other.open_character_authority(&seal).await.map_err(debug)?;
        let premium = pre_delivery(&h.root);
        let second = pre_delivery(&other);
        let (x, y) = fixture::join_two(
            h.root
                .commit_guild_rank(&a, &h.node, fence()?, rank_request(1, 91, 1, 4), &premium),
            other.commit_guild_rank(&b, &h.node, fence()?, rank_request(2, 91, 1, 4), &second),
        )
        .await;
        let results = [
            result(x.map_err(debug)?).result,
            result(y.map_err(debug)?).result,
        ];
        assert_eq!(results.iter().filter(|r| **r == GuildResult::Ok).count(), 1);
        assert_eq!(
            results
                .iter()
                .filter(|r| **r == GuildResult::StaleRevision)
                .count(),
            1
        );
        let same = rank_request(3, 91, 2, 3);
        let (x, y) = fixture::join_two(
            h.root
                .commit_guild_rank(&a, &h.node, fence()?, same.clone(), &premium),
            other.commit_guild_rank(&b, &h.node, fence()?, same, &second),
        )
        .await;
        let x = x.map_err(debug)?;
        let y = y.map_err(debug)?;
        assert_eq!(
            usize::from(matches!(&x, GuildOutcome::Committed(_)))
                + usize::from(matches!(&y, GuildOutcome::Committed(_))),
            1
        );
        assert_eq!(result(x), result(y));
        assert_eq!(revision(h, 1).await?, "3");
        assert_eq!(h.count("game_guild_operations").await?, 3);
        assert_eq!(h.count("game_guild_event_outbox").await?, 2);
        // Native admission relation locking also participates; no isolated throughput claim.
        Ok(())
    })
}

#[test]
fn guild_rank_native_global_target_account_serialization() -> TestResult {
    text_run("guild_rank_account_race", async |h| {
        historical_character(h, 90, id(WORLD)).await?;
        for c in [91, 92] {
            historical_character_account(h, c, id(WORLD), account(777)).await?;
        }
        initial_guild(h, 1, 0).await?;
        initial_guild(h, 2, 90).await?;
        initial_member(h, 1, 91, 3).await?;
        initial_member(h, 2, 92, 3).await?;
        let actor_b = historical_session(h, 90).await?;
        guard(h).await?;
        let other = DurabilityRoot::connect_test_runtime(&h.database.url)?;
        assert!(other.maintain_ready_once().await?);
        let seal = h.recovery.seal_current().map_err(debug)?;
        let a = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let b = other.open_character_authority(&seal).await.map_err(debug)?;
        let first = rank_request(1, 91, 1, 2);
        let mut next =
            GuildRankRequest::new(request(2, 2, GuildCommand::SetRank(character(92))), 1, 2)
                .map_err(debug)?;
        next.common.command = CommandRef::new(actor_b.game_session_id, CommandId::new(2)?);
        let pa = pre_delivery(&h.root);
        let pb = pre_delivery(&other);
        let (x, y) = fixture::join_two(
            h.root.commit_guild_rank(&a, &h.node, fence()?, first, &pa),
            other.commit_guild_rank(&b, &h.node, actor_b, next, &pb),
        )
        .await;
        let results = [
            result(x.map_err(debug)?).result,
            result(y.map_err(debug)?).result,
        ];
        assert_eq!(results.iter().filter(|r| **r == GuildResult::Ok).count(), 1);
        assert_eq!(
            results
                .iter()
                .filter(|r| **r == GuildResult::AccountHasPosition)
                .count(),
            1
        );
        let count:i64=sqlx::query_scalar("SELECT count(*) FROM game_account_guild_leadership WHERE account_id=encode($1,'hex')::uuid")
            .bind(account(777).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(count, 1);
        assert_eq!(h.count("game_guild_event_outbox").await?, 1);
        assert_eq!(h.count("game_guild_operations").await?, 2);
        Ok(())
    })
}

#[test]
fn guild_rank_native_savepoint_atomicity_and_closed_sql() -> TestResult {
    text_run("guild_rank_rollback", async |h| {
        rank_fixture(h, 0).await?;
        guard(h).await?;
        let before = text_snapshot(h).await?;
        let premium = pre_delivery(&h.root);
        let req = rank_request(1, 91, 1, 2);
        // Faults happen after receipt/event creation and must roll back that entire savepoint,
        // even though the genuine NotActivated gate commits its enclosing physical transaction.
        for (trigger, cleanup) in [
            (
                "CREATE FUNCTION guild_rank_test_corrupt() RETURNS TRIGGER LANGUAGE plpgsql AS $$BEGIN NEW.title='tampered';RETURN NEW;END$$;CREATE TRIGGER aaa_guild_rank_test_corrupt BEFORE UPDATE ON game_guild_members FOR EACH ROW EXECUTE FUNCTION guild_rank_test_corrupt();",
                "DROP TRIGGER aaa_guild_rank_test_corrupt ON game_guild_members;DROP FUNCTION guild_rank_test_corrupt();",
            ),
            (
                "CREATE FUNCTION guild_rank_test_corrupt() RETURNS TRIGGER LANGUAGE plpgsql AS $$BEGIN NEW.premium_deficit_since=99;RETURN NEW;END$$;CREATE TRIGGER aaa_guild_rank_test_corrupt BEFORE UPDATE ON game_guilds FOR EACH ROW EXECUTE FUNCTION guild_rank_test_corrupt();",
                "DROP TRIGGER aaa_guild_rank_test_corrupt ON game_guilds;DROP FUNCTION guild_rank_test_corrupt();",
            ),
            (
                "CREATE FUNCTION guild_rank_test_corrupt() RETURNS TRIGGER LANGUAGE plpgsql AS $$BEGIN RETURN NULL;END$$;CREATE TRIGGER aaa_guild_rank_test_corrupt BEFORE INSERT ON game_account_guild_leadership FOR EACH ROW EXECUTE FUNCTION guild_rank_test_corrupt();",
                "DROP TRIGGER aaa_guild_rank_test_corrupt ON game_account_guild_leadership;DROP FUNCTION guild_rank_test_corrupt();",
            ),
        ] {
            h.pool.execute(trigger).await?;
            assert!(rank_apply(h, req.clone(), &premium).await.is_err());
            assert_eq!(text_snapshot(h).await?, before);
            assert_eq!(h.count("game_premium_activation_latch").await?, 0);
            h.pool.execute(cleanup).await?;
        }
        // No receipt survived; the exact command now commits rather than replaying.
        let outcome = rank_apply(h, req.clone(), &premium).await?;
        assert!(matches!(&outcome, GuildOutcome::Committed(_)));
        assert_eq!(result(outcome).result, GuildResult::Ok);
        let before = text_snapshot(h).await?;
        assert!(sqlx::query("UPDATE game_guild_members SET rank_level=4 WHERE character_id=encode($1,'hex')::uuid")
            .bind(character(91).as_bytes().as_slice()).execute(&h.pool).await.is_err());
        assert!(
            sqlx::query(
                "DELETE FROM game_account_guild_leadership WHERE account_id=encode($1,'hex')::uuid"
            )
            .bind(account(91).as_slice())
            .execute(&h.pool)
            .await
            .is_err()
        );
        assert!(
            sqlx::query("UPDATE game_guild_operations SET edit_rank=4")
                .execute(&h.pool)
                .await
                .is_err()
        );
        assert!(sqlx::query("UPDATE game_guild_event_outbox SET payload=convert_to('{}','UTF8'),payload_sha256=sha256(convert_to('{}','UTF8'))")
            .execute(&h.pool).await.is_err());
        assert_eq!(text_snapshot(h).await?, before);
        Ok(())
    })?;
    text_run("guild_rank_current_rollback", async |h| {
        rank_fixture(h, 0).await?;
        guard(h).await?;
        let (current, _) = current_coordinator(h, account(91), true).await?;
        let before = text_snapshot(h).await?;
        h.pool.execute("CREATE FUNCTION guild_rank_test_current_fault() RETURNS TRIGGER LANGUAGE plpgsql AS $$BEGIN NEW.title='tampered';RETURN NEW;END$$;CREATE TRIGGER aaa_guild_rank_test_current_fault BEFORE UPDATE ON game_guild_members FOR EACH ROW EXECUTE FUNCTION guild_rank_test_current_fault();").await?;
        let req = rank_request(1, 91, 1, 2);
        assert!(rank_apply(h, req.clone(), &current).await.is_err());
        assert_eq!(text_snapshot(h).await?, before);
        h.pool.execute("DROP TRIGGER aaa_guild_rank_test_current_fault ON game_guild_members;DROP FUNCTION guild_rank_test_current_fault();").await?;
        assert!(matches!(
            rank_apply(h, req, &current).await?,
            GuildOutcome::Committed(_)
        ));
        assert_eq!(rank_level(h, 91).await?, 2);
        assert_eq!(h.count("game_guild_operations").await?, 1);
        assert_eq!(h.count("game_guild_event_outbox").await?, 1);
        Ok(())
    })
}

#[test]
fn guild_rank_native_fresh_session_node_recovery_and_writer_recheck() -> TestResult {
    text_run("guild_rank_currentness", async |h| {
        rank_fixture(h, 0).await?;
        guard(h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let premium = pre_delivery(&h.root);
        let req = rank_request(1, 91, 1, 2);
        let before = text_snapshot(h).await?;
        let wrong_node = fixture::register(&h.root, 2).await?;
        for field in 0..5 {
            let mut stale = fence()?;
            match field {
                0 => stale.connection_generation = ConnectionGeneration::new(2)?,
                1 => stale.character_lease_generation = 2,
                2 => stale.scope_ownership_generation = ScopeOwnershipGeneration::new(2)?,
                3 => stale.character_id = character(999),
                _ => {}
            }
            let node = if field == 4 { &wrong_node } else { &h.node };
            assert!(matches!(
                h.root
                    .commit_guild_rank(&authority, node, stale, req.clone(), &premium)
                    .await,
                Err(GuildError::NotCurrent)
            ));
            assert_eq!(text_snapshot(h).await?, before);
        }
        let recovery = authority.record_for(&h.root).map_err(debug)?;
        let mut stale_recovery = recovery.clone();
        stale_recovery.recovery_generation += 1;
        let write = RankWrite {
            node: h.node.clone(),
            fence: fence()?,
            binding: rank_binding(&fence()?, &req).map_err(debug)?,
            rank: req.clone(),
            account: Some(account(91)),
        };
        let mut tx = h.pool.begin().await?;
        assert!(write.execute(&mut tx, &stale_recovery, None).await.is_err());
        tx.rollback().await?;
        // Revalidation refuses a changed native target Account selector in the real writer too.
        let wrong_selector = RankWrite {
            account: Some(account(92)),
            ..write
        };
        let mut tx = h.pool.begin().await?;
        assert!(matches!(
            wrong_selector.execute(&mut tx, &recovery, None).await,
            Err(GuildError::NotCurrent)
        ));
        tx.rollback().await?;
        assert_eq!(text_snapshot(h).await?, before);
        assert_eq!(
            result(
                h.root
                    .commit_guild_rank(&authority, &h.node, fence()?, req.clone(), &premium)
                    .await
                    .map_err(debug)?
            )
            .result,
            GuildResult::Ok
        );
        // Recovery is checked before even a historical exact replay.
        let replay_write = RankWrite {
            node: h.node.clone(),
            fence: fence()?,
            binding: rank_binding(&fence()?, &req).map_err(debug)?,
            rank: req.clone(),
            account: None,
        };
        let before = text_snapshot(h).await?;
        let mut tx = h.pool.begin().await?;
        assert!(
            replay_write
                .execute(&mut tx, &stale_recovery, None)
                .await
                .is_err()
        );
        tx.rollback().await?;
        assert_eq!(text_snapshot(h).await?, before);
        use crate::durability::runtime_scope_assignment::{
            AssignmentCommand, AssignmentOutcome, AssignmentRequest, AssignmentState, ControlActor,
            OperationKey, RuntimeScopeAssignmentWriter,
        };
        let actor: String = sqlx::query_scalar("SELECT session_user")
            .fetch_one(&h.pool)
            .await?;
        assert_eq!(actor, "oteryn_test_admin");
        // Exact operation3 grant belongs only to this native test principal/scope; no fake caller.
        sqlx::query("INSERT INTO game_control_scope_grants(control_role,world_id,channel_id,operation) VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,3)")
            .bind(id(WORLD).as_slice()).bind(id(fixture::CHANNEL).as_slice()).execute(&h.pool).await?;
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
                operation_key: OperationKey::from_bytes([88; 32]),
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
        let fresh = rank_request(2, 91, 2, 3);
        assert!(matches!(
            h.root
                .commit_guild_rank(&authority, &h.node, fence()?, fresh.clone(), &premium)
                .await,
            Err(GuildError::NotCurrent)
        ));
        let revoked_write = RankWrite {
            node: h.node.clone(),
            fence: fence()?,
            binding: rank_binding(&fence()?, &fresh).map_err(debug)?,
            rank: fresh,
            account: Some(account(91)),
        };
        let mut tx = h.pool.begin().await?;
        assert!(matches!(
            revoked_write.execute(&mut tx, &recovery, None).await,
            Err(GuildError::NotCurrent)
        ));
        tx.rollback().await?;
        assert_eq!(text_snapshot(h).await?, before);
        assert!(matches!(
            h.root
                .commit_guild_rank(&authority, &h.node, fence()?, req, &premium)
                .await
                .map_err(debug)?,
            GuildOutcome::Replayed(_)
        ));
        assert_eq!(text_snapshot(h).await?, before);
        Ok(())
    })
}

#[test]
fn guild_rank_native_real_found_invite_accept_feed_through() -> TestResult {
    text_run("guild_rank_real_formation", async |h| {
        historical_character(h, 91, id(WORLD)).await?;
        let target_fence = historical_session(h, 91).await?;
        guard(h).await?;
        let premium = pre_delivery(&h.root);
        assert_eq!(
            result(
                found(
                    h,
                    found_request(1, 1, "Rank Native Guild"),
                    fence()?,
                    &premium
                )
                .await?
            )
            .result,
            GuildResult::Ok
        );
        assert_eq!(
            result(
                apply(
                    h,
                    request(2, 1, GuildCommand::Invite(character(91))),
                    fence()?
                )
                .await?
            )
            .result,
            GuildResult::Ok
        );
        let mut accept = request(3, 1, GuildCommand::Accept);
        accept.command = CommandRef::new(target_fence.game_session_id, CommandId::new(3)?);
        assert_eq!(
            result(apply(h, accept, target_fence).await?).result,
            GuildResult::Ok
        );
        let (target_premium, _) = current_coordinator(h, account(91), true).await?;
        assert_eq!(
            result(rank_apply(h, rank_request(4, 91, 3, 2), &target_premium).await?).result,
            GuildResult::Ok
        );
        assert_eq!(rank_level(h, 91).await?, 2);
        assert_eq!(revision(h, 1).await?, "4");
        assert_eq!(h.count("game_guild_event_outbox").await?, 4);
        assert_eq!(h.count("game_account_guild_leadership").await?, 2);
        assert_eq!(h.count("game_guild_invitations").await?, 0);
        let state: i16 = sqlx::query_scalar("SELECT state FROM game_guilds")
            .fetch_one(&h.pool)
            .await?;
        assert_eq!(state, 1);
        assert_eq!(text_character_revision(h).await?, "1");
        Ok(())
    })
}

#[test]
fn guild_rank_native_exact_database_transaction_deadline_boundaries() -> TestResult {
    // The only exact clock here is PostgreSQL's physical transaction clock. Historical deadline
    // setup happens before installing the private guards, inside the real NotActivated gate TX.
    // RankWrite then reuses that genuine borrowed TX and genuine recovery/node/session records.
    // The public entry's real selector/Premium binding is separately covered above.
    for state in [1_i16, 2] {
        for delta in [0_i64, -1] {
            text_run("guild_rank_exact_deadline", async |h| {
                rank_fixture(h, 3).await?;
                let seal = h.recovery.seal_current().map_err(debug)?;
                let authority = h
                    .root
                    .open_character_authority(&seal)
                    .await
                    .map_err(debug)?;
                let recovery = authority.record_for(&h.root).map_err(debug)?;
                let req = rank_request(1, 91, 1, 2);
                let write = RankWrite {
                    node: h.node.clone(),
                    fence: fence()?,
                    binding: rank_binding(&fence()?, &req).map_err(debug)?,
                    rank: req,
                    account: Some(account(91)),
                };
                let applied=pre_delivery(&h.root).with_premium_gate(account(91),move |status,tx| {
                    Box::pin(async move {
                        if !matches!(&status,PremiumStatus::NotActivated(_)) {return Err(invalid());}
                        let tx=tx.ok_or_else(invalid)?;
                        let now:i64=sqlx::query_scalar("SELECT floor(extract(epoch FROM transaction_timestamp())*1000)::bigint").fetch_one(&mut **tx).await?;
                        sqlx::query("UPDATE game_guilds SET state=$1,formation_deadline=$2,vice_deficit_since=$3")
                            .bind(state).bind(now+delta).bind(if state==2 {Some(now-1_209_600_000+delta)}else{None}).execute(&mut **tx).await?;
                        (&mut **tx).execute(GUILD_GUARDS_PROPOSAL).await?;
                        let outcome=write.execute(tx,&recovery,Some(&status)).await?;
                        Ok::<_,GuildError>((outcome,now))
                    })
                }).await.map_err(debug)?.map_err(debug)?;
                assert_eq!(
                    result(applied.0).result,
                    if delta == 0 {
                        GuildResult::Ok
                    } else {
                        GuildResult::GuildDeadlinePassed
                    }
                );
                assert_eq!(revision(h, 1).await?, if delta == 0 { "2" } else { "1" });
                assert_eq!(rank_level(h, 91).await?, if delta == 0 { 2 } else { 3 });
                assert_eq!(h.count("game_guild_operations").await?, 1);
                assert_eq!(
                    h.count("game_guild_event_outbox").await?,
                    i64::from(delta == 0)
                );
                let row: (i16, Option<i64>, Option<i64>) = sqlx::query_as(
                    "SELECT state,vice_deficit_since,premium_deficit_since FROM game_guilds",
                )
                .fetch_one(&h.pool)
                .await?;
                assert_eq!(row.0, if delta == 0 { 2 } else { state });
                assert_eq!(
                    row.1,
                    if delta == 0 || state == 1 {
                        None
                    } else {
                        Some(applied.1 - 1_209_600_000 - 1)
                    }
                );
                assert_eq!(row.2, Some(7));
                assert_eq!(text_character_revision(h).await?, "1");
                Ok(())
            })?;
        }
    }
    Ok(())
}

#[test]
fn guild_rank_native_shared_gate_commits_before_activation_latch() -> TestResult {
    text_run("guild_rank_gate_latch", async |h| {
        rank_fixture(h, 0).await?;
        guard(h).await?;
        h.pool.execute("CREATE FUNCTION guild_rank_test_barrier() RETURNS TRIGGER LANGUAGE plpgsql AS $$BEGIN PERFORM pg_advisory_xact_lock(901123);RETURN NEW;END$$;CREATE TRIGGER aaa_guild_rank_test_barrier BEFORE UPDATE ON game_guild_members FOR EACH ROW EXECUTE FUNCTION guild_rank_test_barrier();").await?;
        let mut barrier = h.pool.begin().await?;
        sqlx::query("SELECT pg_advisory_xact_lock(901123)")
            .execute(&mut *barrier)
            .await?;
        let other = DurabilityRoot::connect_test_runtime(&h.database.url)?;
        assert!(other.maintain_ready_once().await?);
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let premium = pre_delivery(&h.root);
        let req = rank_request(1, 91, 1, 2);
        let (promoted, latched) = fixture::join_two(
            h.root
                .commit_guild_rank(&authority, &h.node, fence()?, req.clone(), &premium),
            async {
                wait_advisory(&h.pool, 901123, false).await?;
                let latch_key = super::super::premium_activation_latch::PREMIUM_ACTIVATION_LOCK_KEY;
                wait_advisory(&h.pool, latch_key, true).await?;
                let (latched, released) = fixture::join_two(
                    other.latch_premium_activation("guild-rank-racing-activation", 1, 2),
                    async {
                        wait_advisory(&h.pool, latch_key, false).await?;
                        barrier.rollback().await?;
                        Ok::<_, Box<dyn std::error::Error>>(())
                    },
                )
                .await;
                released?;
                latched.map_err(|e| -> Box<dyn std::error::Error> { debug(e).into() })
            },
        )
        .await;
        assert_eq!(result(promoted.map_err(debug)?).result, GuildResult::Ok);
        assert_eq!(latched?.activation_id, "guild-rank-racing-activation");
        h.pool.execute("DROP TRIGGER aaa_guild_rank_test_barrier ON game_guild_members;DROP FUNCTION guild_rank_test_barrier();").await?;
        // After delivery, demotion still requires no fresh Premium grant. Re-promotion does.
        assert_eq!(
            result(
                h.root
                    .commit_guild_rank(
                        &authority,
                        &h.node,
                        fence()?,
                        rank_request(2, 91, 2, 3),
                        &pre_delivery(&h.root)
                    )
                    .await
                    .map_err(debug)?
            )
            .result,
            GuildResult::Ok
        );
        let before = rank_mutation_snapshot(h).await?;
        assert_eq!(
            result(
                h.root
                    .commit_guild_rank(
                        &authority,
                        &h.node,
                        fence()?,
                        rank_request(3, 91, 3, 2),
                        &pre_delivery(&h.root)
                    )
                    .await
                    .map_err(debug)?
            )
            .result,
            GuildResult::NotPremium
        );
        assert_eq!(rank_mutation_snapshot(h).await?, before);
        assert!(matches!(
            h.root
                .commit_guild_rank(&authority, &h.node, fence()?, req, &pre_delivery(&h.root))
                .await
                .map_err(debug)?,
            GuildOutcome::Replayed(_)
        ));
        assert_eq!(h.count("game_guild_event_outbox").await?, 2);
        assert_eq!(h.count("game_guild_operations").await?, 3);
        Ok(())
    })
}

// Proposed append only; author integrates after the COMPLETE old48 owner/helper prefix.
// Uses actual native Root/Premium consumer/PG fixture. No runtime authority is constructed here.
fn resign_request(n: u32, target: u32, expected: u64) -> GuildResignRequest {
    GuildResignRequest::new(
        request(n, 1, GuildCommand::Resign(character(target))),
        expected,
    )
    .expect("bounded native request fixture")
}
async fn resign_apply(
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
async fn resign_fixture(h: &Harness) -> TestResult<CurrentCharacterItemFence> {
    initial_guild(h, 1, 0).await?;
    historical_character(h, 91, id(WORLD)).await?;
    initial_member(h, 1, 91, 2).await?;
    historical_session(h, 91).await
}
async fn resign_resource_seed_current(
    h: &Harness,
    consumer: &Arc<PremiumConsumer>,
    who: [u8; 16],
) -> TestResult {
    use crate::premium::test_producer::{ProducerRequest, snapshot};
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
async fn resign_current(
    h: &Harness,
) -> TestResult<(SpellPremiumCoordinator, Arc<PremiumConsumer>)> {
    let (coordinator, consumer) = current_coordinator(h, account(0), true).await?;
    resign_resource_seed_current(h, &consumer, account(91)).await?;
    Ok((coordinator, consumer))
}
async fn resign_resource_character_revisions(h: &Harness) -> TestResult<Vec<(String, String)>> {
    Ok(sqlx::query_as(
        "SELECT character_id::text,character_revision::text FROM game_character_roots ORDER BY character_id",
    )
    .fetch_all(&h.pool)
    .await?)
}
/// A successful role swap owns only rank_level and one Guild revision; all other rows stay exact.
async fn resign_resource_unchanged_fields(h: &Harness) -> TestResult<String> {
    Ok(sqlx::query_scalar("SELECT COALESCE(jsonb_agg(x ORDER BY x::text),'[]'::jsonb)::text FROM (SELECT jsonb_build_array('guild',to_jsonb(t)-'revision') x FROM game_guilds t UNION ALL SELECT jsonb_build_array('member',to_jsonb(t)-'rank_level') FROM game_guild_members t UNION ALL SELECT jsonb_build_array('rank',to_jsonb(t)) FROM game_guild_ranks t UNION ALL SELECT jsonb_build_array('invite',to_jsonb(t)) FROM game_guild_invitations t UNION ALL SELECT jsonb_build_array('position',to_jsonb(t)) FROM game_account_guild_leadership t) r")
        .fetch_one(&h.pool).await?)
}

#[test]
fn guild_resign_native_request_binding_and_closed_audit() -> TestResult {
    assert!(
        GuildResignRequest::new(request(1, 1, GuildCommand::Resign(character(91))), 0).is_err()
    );
    assert!(
        GuildResignRequest::new(request(1, 1, GuildCommand::SetRank(character(91))), 1).is_err()
    );
    let original = resign_request(1, 91, 1);
    let binding = resign_binding(&fence()?, &original).map_err(debug)?;
    let mut changes = vec![resign_request(1, 92, 1), resign_request(1, 91, 2)];
    let mut changed = original.clone();
    changed.common.guild_id = gid(2);
    changes.push(changed);
    let mut changed = original.clone();
    changed.common.operation = GuildCommand::SetRank(character(91));
    changes.push(changed);
    let mut changed = original.clone();
    changed.common.command = request(2, 1, GuildCommand::Resign(character(91))).command;
    changes.push(changed);
    let mut changed = original.clone();
    changed.common.identity.transaction_id = sid(9, 11);
    changes.push(changed);
    for changed in changes {
        assert_ne!(resign_binding(&fence()?, &changed).map_err(debug)?, binding);
    }
    let mut actor_data = fence()?;
    actor_data.character_id = character(92);
    assert_ne!(
        resign_binding(&actor_data, &original).map_err(debug)?,
        binding
    );
    let audit = GuildResignAuditProposal {
        operation: GuildAuditProposal {
            version: 1,
            occurrence: sid(1, 10),
            transaction_id: sid(1, 11),
            event_id: sid(1, 12),
            guild_id: *gid(1).as_bytes(),
            world_id: id(WORLD),
            actor: id(CHARACTER),
            target: *character(91).as_bytes(),
            action: 10,
            previous_revision: 1,
            revision: 2,
            rank_before: Some(2),
            rank_after: Some(1),
            removed_invitations: 0,
            occurred_at_unix_ms: 1,
        },
        expected_revision: 1,
    };
    let bytes = audit.checked_bytes().ok_or("valid Resign audit refused")?;
    assert_eq!(
        serde_json::from_slice::<GuildResignAuditProposal>(&bytes)?,
        audit
    );
    for field in 0..6 {
        let mut wrong = audit.clone();
        match field {
            0 => wrong.expected_revision = 2,
            1 => wrong.operation.rank_before = Some(1),
            2 => wrong.operation.rank_after = Some(2),
            3 => wrong.operation.actor = wrong.operation.target,
            4 => wrong.operation.action = 9,
            _ => wrong.operation.revision = 3,
        }
        assert!(wrong.checked_bytes().is_none());
    }
    let mut json: serde_json::Value = serde_json::from_slice(&bytes)?;
    json["caller_target_premium"] = true.into();
    assert!(serde_json::from_value::<GuildResignAuditProposal>(json).is_err());
    Ok(())
}

#[test]
fn guild_resign_native_legacy_payloadless_entry_refuses_before_guild_lookup() -> TestResult {
    text_run("guild_resign_legacy", async |h| {
        guard(h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let before = text_snapshot(h).await?;
        assert!(matches!(
            h.root
                .commit_guild_membership(
                    &authority,
                    &h.node,
                    fence()?,
                    request(1, 999, GuildCommand::Resign(character(91)))
                )
                .await,
            Err(GuildError::InvalidInput)
        ));
        assert_eq!(text_snapshot(h).await?, before);
        assert_eq!(h.count("game_guild_operations").await?, 0);
        assert_eq!(h.count("game_guild_event_outbox").await?, 0);
        Ok(())
    })
}

#[test]
fn guild_resign_native_both_actual_account_premium_matrix() -> TestResult {
    // 0 bothCurrent; 1 onlyactor; 2 onlytarget; 3 bothNotCurrent; 4 noClock;
    // 5 actorPullFailed; 6 targetPullFailed; 7 missinggate; 8 genuineNotActivated.
    for case in 0..9 {
        text_run("guild_resign_premium", async |h| {
            resign_fixture(h).await?;
            guard(h).await?;
            let (premium, consumer) = match case {
                0 | 5 | 6 => resign_current(h).await?,
                1 | 3 => current_coordinator(h, account(0), true).await?,
                2 => current_coordinator(h, account(91), true).await?,
                4 => {
                    let pair = current_coordinator(h, account(0), false).await?;
                    resign_resource_seed_current(h, &pair.1, account(91)).await?;
                    pair
                }
                7 => {
                    let coordinator =
                        SpellPremiumCoordinator::new(Arc::default(), None, None, None);
                    let consumer = coordinator.consumer();
                    (coordinator, consumer)
                }
                _ => {
                    let coordinator = pre_delivery(&h.root);
                    let consumer = coordinator.consumer();
                    (coordinator, consumer)
                }
            };
            if case == 3 || case == 5 {
                consumer.pull_failed(account(0), consumer.ticket());
            }
            if case == 6 {
                consumer.pull_failed(account(91), consumer.ticket());
            }
            let before = rank_mutation_snapshot(h).await?;
            let unchanged = resign_resource_unchanged_fields(h).await?;
            let revisions = resign_resource_character_revisions(h).await?;
            let outcome = result(resign_apply(h, resign_request(1, 91, 1), &premium).await?);
            let success = case == 0 || case == 8;
            assert_eq!(
                outcome.result,
                if success {
                    GuildResult::Ok
                } else {
                    GuildResult::NotPremium
                }
            );
            assert_eq!(h.count("game_guild_operations").await?, 1);
            assert_eq!(
                h.count("game_guild_event_outbox").await?,
                i64::from(success)
            );
            assert_eq!(h.count("game_account_guild_leadership").await?, 2);
            assert_eq!(resign_resource_character_revisions(h).await?, revisions);
            if success {
                assert_eq!(rank_level(h, 0).await?, 2);
                assert_eq!(rank_level(h, 91).await?, 1);
                assert_eq!(revision(h, 1).await?, "2");
                assert_eq!(resign_resource_unchanged_fields(h).await?, unchanged);
                if case == 8 {
                    assert_eq!(h.count("game_premium_activation_latch").await?, 0);
                }
            } else {
                assert_eq!(rank_mutation_snapshot(h).await?, before);
                assert_eq!(revision(h, 1).await?, "1");
            }
            Ok(())
        })?;
    }
    Ok(())
}

#[test]
fn guild_resign_native_current_roles_target_and_world_refusals() -> TestResult {
    for (actor_rank, target_rank, expected_revision) in
        [(2, 2, 1), (3, 2, 1), (1, 3, 1), (2, 2, 99)]
    {
        text_run("guild_resign_roles", async |h| {
            historical_character(h, 90, id(WORLD)).await?;
            historical_character(h, 91, id(WORLD)).await?;
            initial_guild(h, 1, if actor_rank == 1 { 0 } else { 90 }).await?;
            if actor_rank != 1 {
                initial_member(h, 1, 0, actor_rank).await?;
            }
            initial_member(h, 1, 91, target_rank).await?;
            guard(h).await?;
            let before = rank_mutation_snapshot(h).await?;
            assert_eq!(
                result(
                    resign_apply(
                        h,
                        resign_request(1, 91, expected_revision),
                        &pre_delivery(&h.root)
                    )
                    .await?
                )
                .result,
                GuildResult::NotAllowed
            );
            assert_eq!(rank_mutation_snapshot(h).await?, before);
            assert_eq!(h.count("game_guild_operations").await?, 1);
            assert_eq!(h.count("game_guild_event_outbox").await?, 0);
            Ok(())
        })?;
    }
    text_run("guild_resign_target_scope", async |h| {
        historical_character(h, 90, id(WORLD)).await?;
        resign_fixture(h).await?;
        initial_guild(h, 2, 90).await?;
        historical_character(h, 92, id(WORLD)).await?;
        initial_member(h, 2, 92, 2).await?;
        historical_character(h, 93, sid(900, 40)).await?;
        guard(h).await?;
        let before = rank_mutation_snapshot(h).await?;
        for (n, target) in [(1, 0), (2, 92), (3, 93), (4, 999)] {
            assert_eq!(
                result(
                    resign_apply(h, resign_request(n, target, 1), &pre_delivery(&h.root)).await?
                )
                .result,
                GuildResult::NotAllowed
            );
            assert_eq!(rank_mutation_snapshot(h).await?, before);
            assert_eq!(h.count("game_guild_operations").await?, i64::from(n));
            assert_eq!(h.count("game_guild_event_outbox").await?, 0);
        }
        Ok(())
    })
}

#[test]
fn guild_resign_native_keeps_lifecycle_and_refuses_terminal_states() -> TestResult {
    for state in [1_i16, 2, 3, 4] {
        text_run("guild_resign_lifecycle", async |h| {
            resign_fixture(h).await?;
            sqlx::query("UPDATE game_guilds SET state=$1,formation_deadline=1,vice_deficit_since=$2,premium_deficit_since=$3")
                .bind(state).bind(if state == 1 { None } else { Some(1_i64) })
                .bind(if state == 1 { None } else { Some(7_i64) }).execute(&h.pool).await?;
            if state == 4 {
                // A terminal historical fixture has no remaining members/leadership rows.
                h.pool
                    .execute(
                        "DELETE FROM game_account_guild_leadership;DELETE FROM game_guild_members;",
                    )
                    .await?;
            }
            guard(h).await?;
            let before = rank_mutation_snapshot(h).await?;
            let unchanged = resign_resource_unchanged_fields(h).await?;
            let revisions = resign_resource_character_revisions(h).await?;
            let outcome =
                result(resign_apply(h, resign_request(1, 91, 1), &pre_delivery(&h.root)).await?);
            assert_eq!(
                outcome.result,
                if state <= 2 {
                    GuildResult::Ok
                } else {
                    GuildResult::GuildDisbanding
                }
            );
            assert_eq!(h.count("game_guild_operations").await?, 1);
            assert_eq!(
                h.count("game_guild_event_outbox").await?,
                i64::from(state <= 2)
            );
            assert_eq!(resign_resource_character_revisions(h).await?, revisions);
            if state <= 2 {
                assert_eq!(resign_resource_unchanged_fields(h).await?, unchanged);
                assert_eq!(rank_level(h, 0).await?, 2);
                assert_eq!(rank_level(h, 91).await?, 1);
            } else {
                assert_eq!(rank_mutation_snapshot(h).await?, before);
            }
            Ok(())
        })?;
    }
    Ok(())
}

#[test]
fn guild_resign_native_cas_and_overflow_leave_no_partial_swap() -> TestResult {
    text_run("guild_resign_stale", async |h| {
        resign_fixture(h).await?;
        guard(h).await?;
        let before = rank_mutation_snapshot(h).await?;
        assert_eq!(
            result(resign_apply(h, resign_request(1, 91, 2), &pre_delivery(&h.root)).await?).result,
            GuildResult::StaleRevision
        );
        assert_eq!(rank_mutation_snapshot(h).await?, before);
        assert_eq!(h.count("game_guild_operations").await?, 1);
        assert_eq!(h.count("game_guild_event_outbox").await?, 0);
        Ok(())
    })?;
    text_run("guild_resign_overflow", async |h| {
        resign_fixture(h).await?;
        h.pool
            .execute("UPDATE game_guilds SET revision=18446744073709551615")
            .await?;
        guard(h).await?;
        let before = text_snapshot(h).await?;
        assert!(
            resign_apply(h, resign_request(1, 91, u64::MAX), &pre_delivery(&h.root))
                .await
                .is_err()
        );
        assert_eq!(text_snapshot(h).await?, before);
        Ok(())
    })
}

#[test]
fn guild_resign_native_committed_and_refusal_replay_survive_role_changes() -> TestResult {
    text_run("guild_resign_replay", async |h| {
        let target_fence = resign_fixture(h).await?;
        guard(h).await?;
        let premium = pre_delivery(&h.root);
        let first_req = resign_request(1, 91, 1);
        let first = result(resign_apply(h, first_req.clone(), &premium).await?);
        assert_eq!(first.result, GuildResult::Ok);
        let refused_req = resign_request(2, 91, 2);
        let refused = result(resign_apply(h, refused_req.clone(), &premium).await?);
        assert_eq!(refused.result, GuildResult::NotAllowed);
        let mut reverse =
            GuildResignRequest::new(request(3, 1, GuildCommand::Resign(character(0))), 2)
                .map_err(debug)?;
        reverse.common.command = CommandRef::new(target_fence.game_session_id, CommandId::new(3)?);
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        assert_eq!(
            result(
                h.root
                    .commit_guild_resign(&authority, &h.node, target_fence, reverse, &premium)
                    .await
                    .map_err(debug)?
            )
            .result,
            GuildResult::Ok
        );
        drop(authority);
        assert_eq!(rank_level(h, 0).await?, 1);
        assert_eq!(rank_level(h, 91).await?, 2);
        let other = DurabilityRoot::connect_test_runtime(&h.database.url)?;
        assert!(other.maintain_ready_once().await?);
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = other.open_character_authority(&seal).await.map_err(debug)?;
        let closed = SpellPremiumCoordinator::new(Arc::default(), None, None, None);
        let mut stale = fence()?;
        stale.connection_generation = ConnectionGeneration::new(999)?;
        let before = text_snapshot(h).await?;
        for (req, receipt) in [(first_req.clone(), first), (refused_req.clone(), refused)] {
            let replay = other
                .commit_guild_resign(&authority, &h.node, stale, req, &closed)
                .await
                .map_err(debug)?;
            assert!(matches!(&replay, GuildOutcome::Replayed(_)));
            assert_eq!(result(replay), receipt);
            assert_eq!(text_snapshot(h).await?, before);
        }
        for field in 0..6 {
            let mut changed = first_req.clone();
            match field {
                0 => changed.expected_revision = 2,
                1 => changed.common.operation = GuildCommand::Resign(character(92)),
                2 => changed.common.guild_id = gid(2),
                3 => changed.common.operation = GuildCommand::SetRank(character(91)),
                4 => changed.common.identity.transaction_id = sid(99, 11),
                _ => {
                    changed.common.command =
                        request(99, 1, GuildCommand::Resign(character(91))).command
                }
            }
            assert!(matches!(
                other
                    .commit_guild_resign(&authority, &h.node, fence()?, changed, &closed)
                    .await,
                Err(GuildError::ConflictingOccurrence)
            ));
            assert_eq!(text_snapshot(h).await?, before);
        }
        let mut reused = resign_request(4, 91, 3);
        reused.common.command = first_req.common.command;
        assert!(matches!(
            other
                .commit_guild_resign(&authority, &h.node, fence()?, reused, &closed)
                .await,
            Err(GuildError::ConflictingOccurrence)
        ));
        assert_eq!(text_snapshot(h).await?, before);
        assert_eq!(h.count("game_guild_operations").await?, 3);
        assert_eq!(h.count("game_guild_event_outbox").await?, 2);
        Ok(())
    })
}

#[test]
fn guild_resign_native_real_formation_and_follow_on_leader_authority() -> TestResult {
    text_run("guild_resign_real_formation", async |h| {
        historical_character(h, 91, id(WORLD)).await?;
        let target_fence = historical_session(h, 91).await?;
        guard(h).await?;
        let pre = pre_delivery(&h.root);
        assert_eq!(
            result(
                found(
                    h,
                    found_request(1, 1, "Resign Native Guild"),
                    fence()?,
                    &pre
                )
                .await?
            )
            .result,
            GuildResult::Ok
        );
        assert_eq!(
            result(
                apply(
                    h,
                    request(2, 1, GuildCommand::Invite(character(91))),
                    fence()?
                )
                .await?
            )
            .result,
            GuildResult::Ok
        );
        let mut accept = request(3, 1, GuildCommand::Accept);
        accept.command = CommandRef::new(target_fence.game_session_id, CommandId::new(3)?);
        assert_eq!(
            result(apply(h, accept, target_fence).await?).result,
            GuildResult::Ok
        );
        let (premium, _) = resign_current(h).await?;
        assert_eq!(
            result(rank_apply(h, rank_request(4, 91, 3, 2), &premium).await?).result,
            GuildResult::Ok
        );
        assert_eq!(
            result(resign_apply(h, resign_request(5, 91, 4), &premium).await?).result,
            GuildResult::Ok
        );
        assert_eq!(rank_level(h, 0).await?, 2);
        assert_eq!(rank_level(h, 91).await?, 1);
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut title = text_request(6, GuildCommand::SetTitle(character(0)), 5, "Resigned");
        title.common.command = CommandRef::new(target_fence.game_session_id, CommandId::new(6)?);
        assert_eq!(
            result(
                h.root
                    .commit_guild_text(&authority, &h.node, target_fence, title)
                    .await
                    .map_err(debug)?
            )
            .result,
            GuildResult::Ok
        );
        assert_eq!(
            result(
                text_apply(
                    h,
                    text_request(7, GuildCommand::SetTitle(character(91)), 6, "Blocked")
                )
                .await?
            )
            .result,
            GuildResult::NotAllowed
        );
        assert_eq!(
            result(
                text_apply(
                    h,
                    text_request(8, GuildCommand::SetMessage, 6, "New leader; retained vice")
                )
                .await?
            )
            .result,
            GuildResult::Ok
        );
        assert_eq!(revision(h, 1).await?, "7");
        assert_eq!(h.count("game_guild_operations").await?, 8);
        assert_eq!(h.count("game_guild_event_outbox").await?, 7);
        assert_eq!(h.count("game_account_guild_leadership").await?, 2);
        assert_eq!(h.count("game_guild_invitations").await?, 0);
        let leaders: i64 =
            sqlx::query_scalar("SELECT count(*) FROM game_guild_members WHERE rank_level=1")
                .fetch_one(&h.pool)
                .await?;
        assert_eq!(leaders, 1);
        let unchanged_revisions: bool =
            sqlx::query_scalar("SELECT bool_and(character_revision=1) FROM game_character_roots")
                .fetch_one(&h.pool)
                .await?;
        assert!(unchanged_revisions);
        Ok(())
    })
}

// Native writer/SQL/race families authored independently of the Premium/role proposal.
// Audit DATA below is only the expected producer payload, never current authority evidence.
fn resign_test_audit(req: &GuildResignRequest) -> TestResult<serde_json::Value> {
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

#[test]
fn guild_resign_native_ordered_swap_sql_binding_and_both_transaction_rollbacks() -> TestResult {
    for delivered in [false, true] {
        text_run("guild_resign_rollback", async |h| {
            let _ = resign_fixture(h).await?;
            guard(h).await?;
            let (current, _) = resign_current(h).await?;
            let before = text_snapshot(h).await?;
            // delivered=false still needs a genuinely empty activation latch. Creating Current
            // snapshots alone does not decide the gate until that coordinator is actually used.
            let pre = pre_delivery(&h.root);
            let premium = if delivered { &current } else { &pre };
            let req = resign_request(1, 91, 1);
            for (setup, cleanup) in [
                (
                    "CREATE FUNCTION guild_resign_test_fault() RETURNS TRIGGER LANGUAGE plpgsql AS $$BEGIN IF OLD.rank_level=1 THEN NEW.title='corrupt';END IF;RETURN NEW;END$$;CREATE TRIGGER aaa_guild_resign_test_fault BEFORE UPDATE ON game_guild_members FOR EACH ROW EXECUTE FUNCTION guild_resign_test_fault();",
                    "DROP TRIGGER aaa_guild_resign_test_fault ON game_guild_members;DROP FUNCTION guild_resign_test_fault();",
                ),
                (
                    "CREATE FUNCTION guild_resign_test_fault() RETURNS TRIGGER LANGUAGE plpgsql AS $$BEGIN IF OLD.rank_level=2 AND NEW.rank_level=1 THEN RETURN NULL;END IF;RETURN NEW;END$$;CREATE TRIGGER aaa_guild_resign_test_fault BEFORE UPDATE ON game_guild_members FOR EACH ROW EXECUTE FUNCTION guild_resign_test_fault();",
                    "DROP TRIGGER aaa_guild_resign_test_fault ON game_guild_members;DROP FUNCTION guild_resign_test_fault();",
                ),
                (
                    "CREATE FUNCTION guild_resign_test_fault() RETURNS TRIGGER LANGUAGE plpgsql AS $$BEGIN NEW.vice_deficit_since=NULL;RETURN NEW;END$$;CREATE TRIGGER aaa_guild_resign_test_fault BEFORE UPDATE ON game_guilds FOR EACH ROW EXECUTE FUNCTION guild_resign_test_fault();",
                    "DROP TRIGGER aaa_guild_resign_test_fault ON game_guilds;DROP FUNCTION guild_resign_test_fault();",
                ),
            ] {
                h.pool.execute(setup).await?;
                assert!(resign_apply(h, req.clone(), premium).await.is_err());
                assert_eq!(text_snapshot(h).await?, before);
                assert_eq!(rank_level(h, 0).await?, 1);
                assert_eq!(rank_level(h, 91).await?, 2);
                h.pool.execute(cleanup).await?;
            }
            if !delivered {
                // BEFORE INSERT fixtures corrupt NEW rows before retention. They NEVER mutate
                // immutable receipts/events. Both hashes agree, and all roles/positions swap
                // correctly, so only exact closed action10 payload binding can refuse these.
                h.pool.execute("CREATE TABLE guild_resign_test_payload(payload BYTEA NOT NULL);CREATE FUNCTION guild_resign_test_payload_op() RETURNS TRIGGER LANGUAGE plpgsql AS $$BEGIN SELECT sha256(payload) INTO NEW.audit_sha256 FROM guild_resign_test_payload;RETURN NEW;END$$;CREATE TRIGGER aaa_guild_resign_test_payload_op BEFORE INSERT ON game_guild_operations FOR EACH ROW EXECUTE FUNCTION guild_resign_test_payload_op();CREATE FUNCTION guild_resign_test_payload_event() RETURNS TRIGGER LANGUAGE plpgsql AS $$BEGIN SELECT payload INTO NEW.payload FROM guild_resign_test_payload;NEW.payload_sha256=sha256(NEW.payload);RETURN NEW;END$$;CREATE TRIGGER aaa_guild_resign_test_payload_event BEFORE INSERT ON game_guild_event_outbox FOR EACH ROW EXECUTE FUNCTION guild_resign_test_payload_event();").await?;
                let valid = resign_test_audit(&req)?;
                let keys = valid
                    .as_object()
                    .ok_or("audit not object")?
                    .keys()
                    .cloned()
                    .collect::<Vec<_>>();
                assert_eq!(keys.len(), 16);
                for key in &keys {
                    for remove in [false, true] {
                        let mut altered = valid.clone();
                        if remove {
                            altered
                                .as_object_mut()
                                .ok_or("audit not object")?
                                .remove(key);
                        } else {
                            // Change exactly one field; all IDs retain genuine UUIDv7 byte shape.
                            altered[key] = match key.as_str() {
                                "actor" | "target" => {
                                    serde_json::to_value(character(999).as_bytes())?
                                }
                                "guild_id" => serde_json::to_value(gid(999).as_bytes())?,
                                "world_id" | "occurrence" | "transaction_id" | "event_id" => {
                                    serde_json::to_value(sid(999, 90))?
                                }
                                _ => serde_json::json!(99),
                            };
                        }
                        sqlx::query("DELETE FROM guild_resign_test_payload")
                            .execute(&h.pool)
                            .await?;
                        sqlx::query("INSERT INTO guild_resign_test_payload VALUES($1)")
                            .bind(serde_json::to_vec(&altered)?)
                            .execute(&h.pool)
                            .await?;
                        assert!(
                            resign_apply(h, req.clone(), premium).await.is_err(),
                            "unbound audit field {key}, remove={remove}"
                        );
                        assert_eq!(text_snapshot(h).await?, before);
                    }
                }
                let mut extra = valid;
                extra["caller_premium"] = true.into();
                sqlx::query("DELETE FROM guild_resign_test_payload")
                    .execute(&h.pool)
                    .await?;
                sqlx::query("INSERT INTO guild_resign_test_payload VALUES($1)")
                    .bind(serde_json::to_vec(&extra)?)
                    .execute(&h.pool)
                    .await?;
                assert!(resign_apply(h, req.clone(), premium).await.is_err());
                assert_eq!(text_snapshot(h).await?, before);
                h.pool.execute("DROP TRIGGER aaa_guild_resign_test_payload_op ON game_guild_operations;DROP TRIGGER aaa_guild_resign_test_payload_event ON game_guild_event_outbox;DROP FUNCTION guild_resign_test_payload_op();DROP FUNCTION guild_resign_test_payload_event();DROP TABLE guild_resign_test_payload;").await?;
            }
            let outcome = resign_apply(h, req, premium).await?;
            assert!(matches!(&outcome, GuildOutcome::Committed(_)));
            assert_eq!(result(outcome).result, GuildResult::Ok);
            assert_eq!(rank_level(h, 0).await?, 2);
            assert_eq!(rank_level(h, 91).await?, 1);
            assert_eq!(h.count("game_guild_operations").await?, 1);
            assert_eq!(h.count("game_guild_event_outbox").await?, 1);
            let before = text_snapshot(h).await?;
            for sql in [
                "UPDATE game_guild_members SET rank_level=1 WHERE rank_level=2",
                "DELETE FROM game_account_guild_leadership",
                "UPDATE game_guild_operations SET edit_rank=2",
                "UPDATE game_guild_event_outbox SET occurred_at=occurred_at+1",
            ] {
                assert!(h.pool.execute(sql).await.is_err());
                assert_eq!(text_snapshot(h).await?, before);
            }
            Ok(())
        })?;
    }
    Ok(())
}

#[test]
fn guild_resign_native_two_root_occurrence_successor_and_demotion_races() -> TestResult {
    for case in 0..3 {
        text_run("guild_resign_race", async |h| {
            let _ = resign_fixture(h).await?;
            historical_character(h, 92, id(WORLD)).await?;
            initial_member(h, 1, 92, 2).await?;
            guard(h).await?;
            let other = DurabilityRoot::connect_test_runtime(&h.database.url)?;
            assert!(other.maintain_ready_once().await?);
            let seal = h.recovery.seal_current().map_err(debug)?;
            let a = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let b = other.open_character_authority(&seal).await.map_err(debug)?;
            let pa = pre_delivery(&h.root);
            let pb = pre_delivery(&other);
            let req = resign_request(1, 91, 1);
            let (x, y) = if case < 2 {
                fixture::join_two(
                    h.root
                        .commit_guild_resign(&a, &h.node, fence()?, req.clone(), &pa),
                    other.commit_guild_resign(
                        &b,
                        &h.node,
                        fence()?,
                        if case == 0 {
                            req
                        } else {
                            resign_request(2, 92, 1)
                        },
                        &pb,
                    ),
                )
                .await
            } else {
                fixture::join_two(
                    h.root.commit_guild_resign(&a, &h.node, fence()?, req, &pa),
                    other.commit_guild_rank(&b, &h.node, fence()?, rank_request(2, 91, 1, 3), &pb),
                )
                .await
            };
            let x = x.map_err(debug)?;
            let y = y.map_err(debug)?;
            if case == 0 {
                assert_eq!(
                    usize::from(matches!(&x, GuildOutcome::Committed(_)))
                        + usize::from(matches!(&y, GuildOutcome::Committed(_))),
                    1
                );
                assert_eq!(result(x), result(y));
                assert_eq!(h.count("game_guild_operations").await?, 1);
            } else {
                let outcomes = [result(x).result, result(y).result];
                assert_eq!(
                    outcomes.iter().filter(|x| **x == GuildResult::Ok).count(),
                    1
                );
                assert_eq!(
                    outcomes
                        .iter()
                        .filter(|x| **x == GuildResult::NotAllowed)
                        .count(),
                    1
                );
                assert_eq!(h.count("game_guild_operations").await?, 2);
            }
            let leaders: i64 =
                sqlx::query_scalar("SELECT count(*) FROM game_guild_members WHERE rank_level=1")
                    .fetch_one(&h.pool)
                    .await?;
            assert_eq!(leaders, 1);
            assert_eq!(revision(h, 1).await?, "2");
            assert_eq!(h.count("game_guild_event_outbox").await?, 1);
            let incoherent:i64=sqlx::query_scalar("SELECT count(*) FROM game_guild_members m JOIN game_character_roots c USING(character_id) WHERE (m.rank_level<=2) <> EXISTS(SELECT 1 FROM game_account_guild_leadership l WHERE l.account_id=c.account_id AND l.character_id=m.character_id AND l.guild_id=m.guild_id)").fetch_one(&h.pool).await?;
            assert_eq!(incoherent, 0);
            assert_eq!(text_character_revision(h).await?, "1");
            // Native admission locking also serializes whole passes; no isolated throughput claim.
            Ok(())
        })?;
    }
    Ok(())
}

#[test]
fn guild_resign_native_both_fence_boundaries_actual_revoke_and_recovery_first() -> TestResult {
    text_run("guild_resign_fences", async |h| {
        let _ = resign_fixture(h).await?;
        guard(h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let recovery = authority.record_for(&h.root).map_err(debug)?;
        let req = resign_request(1, 91, 1);
        let premium = pre_delivery(&h.root);
        let before = text_snapshot(h).await?;
        let wrong = fixture::register(&h.root, 2).await?;
        for field in 0..5 {
            let mut f = fence()?;
            match field {
                0 => f.connection_generation = ConnectionGeneration::new(2)?,
                1 => f.character_lease_generation = 2,
                2 => f.scope_ownership_generation = ScopeOwnershipGeneration::new(2)?,
                3 => f.character_id = character(999),
                _ => {}
            }
            let node = if field == 4 { &wrong } else { &h.node };
            assert!(matches!(
                h.root
                    .commit_guild_resign(&authority, node, f, req.clone(), &premium)
                    .await,
                Err(GuildError::NotCurrent)
            ));
            let write = ResignWrite {
                node: node.clone(),
                fence: f,
                resign: req.clone(),
                binding: resign_binding(&f, &req).map_err(debug)?,
                accounts: Some([account(0), account(91)]),
            };
            let mut tx = h.pool.begin().await?;
            assert!(matches!(
                write.execute(&mut tx, &recovery, None).await,
                Err(GuildError::NotCurrent)
            ));
            tx.rollback().await?;
            assert_eq!(text_snapshot(h).await?, before);
        }
        let write = ResignWrite {
            node: h.node.clone(),
            fence: fence()?,
            resign: req.clone(),
            binding: resign_binding(&fence()?, &req).map_err(debug)?,
            accounts: Some([account(0), account(92)]),
        };
        let mut tx = h.pool.begin().await?;
        assert!(matches!(
            write.execute(&mut tx, &recovery, None).await,
            Err(GuildError::NotCurrent)
        ));
        tx.rollback().await?;
        assert_eq!(text_snapshot(h).await?, before);
        let other = DurabilityRoot::connect_test_runtime(&h.database.url)?;
        assert!(other.maintain_ready_once().await?);
        assert!(matches!(
            other
                .commit_guild_resign(&authority, &h.node, fence()?, req.clone(), &premium)
                .await,
            Err(GuildError::NotCurrent)
        ));
        drop(other);
        use crate::durability::runtime_scope_assignment::{
            AssignmentCommand, AssignmentOutcome, AssignmentRequest, AssignmentState, ControlActor,
            OperationKey, RuntimeScopeAssignmentWriter,
        };
        let actor: String = sqlx::query_scalar("SELECT session_user")
            .fetch_one(&h.pool)
            .await?;
        assert_eq!(actor, "oteryn_test_admin");
        sqlx::query("INSERT INTO game_control_scope_grants(control_role,world_id,channel_id,operation) VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,3)").bind(id(WORLD).as_slice()).bind(id(fixture::CHANNEL).as_slice()).execute(&h.pool).await?;
        let writer = RuntimeScopeAssignmentWriter::open(h.root.clone(), "item-transfer-writer")
            .await
            .map_err(debug)?;
        let predecessor = h
            .root
            .read_runtime_scope_predecessor(fence()?.runtime_scope)
            .await
            .map_err(debug)?
            .ok_or("missing assignment")?;
        assert!(matches!(
            writer
                .submit(&AssignmentRequest {
                    operation_key: OperationKey::from_bytes([89; 32]),
                    actor: ControlActor::new(&actor).map_err(debug)?,
                    command: AssignmentCommand::Revoke {
                        scope: fence()?.runtime_scope,
                        predecessor
                    }
                })
                .await
                .map_err(debug)?,
            AssignmentOutcome::Committed(_)
        ));
        let actual = h
            .root
            .read_runtime_scope_assignment(fence()?.runtime_scope)
            .await
            .map_err(debug)?
            .ok_or("missing revoked assignment")?;
        assert_eq!(actual.state, AssignmentState::Revoked);
        assert_eq!(actual.ownership_generation, 2);
        assert!(matches!(
            h.root
                .commit_guild_resign(&authority, &h.node, fence()?, req.clone(), &premium)
                .await,
            Err(GuildError::NotCurrent)
        ));
        let write = ResignWrite {
            node: h.node.clone(),
            fence: fence()?,
            resign: req.clone(),
            binding: resign_binding(&fence()?, &req).map_err(debug)?,
            accounts: Some([account(0), account(91)]),
        };
        let mut tx = h.pool.begin().await?;
        assert!(matches!(
            write.execute(&mut tx, &recovery, None).await,
            Err(GuildError::NotCurrent)
        ));
        tx.rollback().await?;
        assert_eq!(text_snapshot(h).await?, before);
        assert_eq!(rank_level(h, 0).await?, 1);
        assert_eq!(rank_level(h, 91).await?, 2);
        Ok(())
    })?;
    text_run("guild_resign_recovery", async |h| {
        let _ = resign_fixture(h).await?;
        guard(h).await?;
        let req = resign_request(1, 91, 1);
        assert_eq!(
            result(resign_apply(h, req.clone(), &pre_delivery(&h.root)).await?).result,
            GuildResult::Ok
        );
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut recovery = authority.record_for(&h.root).map_err(debug)?;
        recovery.recovery_generation += 1;
        let write = ResignWrite {
            node: h.node.clone(),
            fence: fence()?,
            resign: req.clone(),
            binding: resign_binding(&fence()?, &req).map_err(debug)?,
            accounts: None,
        };
        let before = text_snapshot(h).await?;
        let mut tx = h.pool.begin().await?;
        assert!(write.execute(&mut tx, &recovery, None).await.is_err());
        tx.rollback().await?;
        assert_eq!(text_snapshot(h).await?, before);
        Ok(())
    })
}

#[test]
fn guild_resign_native_batch_gate_commit_precedes_activation_latch() -> TestResult {
    text_run("guild_resign_batch_latch", async |h| {
        let target_fence = resign_fixture(h).await?;
        guard(h).await?;
        h.pool.execute("CREATE FUNCTION guild_resign_test_barrier() RETURNS TRIGGER LANGUAGE plpgsql AS $$BEGIN IF OLD.rank_level=1 THEN PERFORM pg_advisory_xact_lock(901124);END IF;RETURN NEW;END$$;CREATE TRIGGER aaa_guild_resign_test_barrier BEFORE UPDATE ON game_guild_members FOR EACH ROW EXECUTE FUNCTION guild_resign_test_barrier();").await?;
        let mut barrier = h.pool.begin().await?;
        sqlx::query("SELECT pg_advisory_xact_lock(901124)")
            .execute(&mut *barrier)
            .await?;
        let other = DurabilityRoot::connect_test_runtime(&h.database.url)?;
        assert!(other.maintain_ready_once().await?);
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let premium = pre_delivery(&h.root);
        let req = resign_request(1, 91, 1);
        let (swapped, latched) = fixture::join_two(
            h.root
                .commit_guild_resign(&authority, &h.node, fence()?, req.clone(), &premium),
            async {
                wait_advisory(&h.pool, 901124, false).await?;
                let latch_key = super::super::premium_activation_latch::PREMIUM_ACTIVATION_LOCK_KEY;
                wait_advisory(&h.pool, latch_key, true).await?;
                let (latched, released) = fixture::join_two(
                    other.latch_premium_activation("guild-resign-racing-activation", 1, 2),
                    async {
                        wait_advisory(&h.pool, latch_key, false).await?;
                        barrier.rollback().await?;
                        Ok::<_, Box<dyn std::error::Error>>(())
                    },
                )
                .await;
                released?;
                latched.map_err(|e| -> Box<dyn std::error::Error> { debug(e).into() })
            },
        )
        .await;
        assert_eq!(result(swapped.map_err(debug)?).result, GuildResult::Ok);
        assert_eq!(latched?.activation_id, "guild-resign-racing-activation");
        assert_eq!(rank_level(h, 0).await?, 2);
        assert_eq!(rank_level(h, 91).await?, 1);
        h.pool.execute("DROP TRIGGER aaa_guild_resign_test_barrier ON game_guild_members;DROP FUNCTION guild_resign_test_barrier();").await?;
        // Current newly elected leader attempts the reverse swap after delivery, with no Current
        // snapshot for either Account. Both real decisions are required; persisted positions grant none.
        let mut reverse =
            GuildResignRequest::new(request(2, 1, GuildCommand::Resign(character(0))), 2)
                .map_err(debug)?;
        reverse.common.command = CommandRef::new(target_fence.game_session_id, CommandId::new(2)?);
        let before = rank_mutation_snapshot(h).await?;
        assert_eq!(
            result(
                h.root
                    .commit_guild_resign(
                        &authority,
                        &h.node,
                        target_fence,
                        reverse,
                        &pre_delivery(&h.root)
                    )
                    .await
                    .map_err(debug)?
            )
            .result,
            GuildResult::NotPremium
        );
        assert_eq!(rank_mutation_snapshot(h).await?, before);
        assert!(matches!(
            h.root
                .commit_guild_resign(&authority, &h.node, fence()?, req, &pre_delivery(&h.root))
                .await
                .map_err(debug)?,
            GuildOutcome::Replayed(_)
        ));
        assert_eq!(h.count("game_guild_operations").await?, 2);
        assert_eq!(h.count("game_guild_event_outbox").await?, 1);
        Ok(())
    })
}

// PROPOSAL ONLY: append to the eventual published Resign guild_tests.rs after exact-base refresh.
// Existing helpers above remain literal; every native case requires configured_admin via text_run.
// Rank-name lexical/length policy remains unallocated; these examples use ordinary ASCII names.

fn edit_ranks_request(n: u32, expected: u64, names: &[&str]) -> GuildEditRanksRequest {
    GuildEditRanksRequest::new(
        request(n, 1, GuildCommand::EditRanks),
        expected,
        names.iter().map(|name| (*name).to_owned()).collect(),
    )
    .expect("bounded native rank-set DATA")
}

async fn edit_ranks_apply(h: &Harness, req: GuildEditRanksRequest) -> TestResult<GuildOutcome> {
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

async fn edit_ranks_rows(h: &Harness, guild: u32) -> TestResult<Vec<(i16, String)>> {
    Ok(sqlx::query_as(
        "SELECT level,name FROM game_guild_ranks WHERE guild_id=encode($1,'hex')::uuid ORDER BY level",
    )
    .bind(gid(guild).as_bytes().as_slice())
    .fetch_all(&h.pool)
    .await?)
}

async fn edit_ranks_expect(h: &Harness, names: &[String]) -> TestResult {
    let expected = names
        .iter()
        .enumerate()
        .map(|(index, name)| (i16::try_from(index + 1).unwrap(), name.clone()))
        .collect::<Vec<_>>();
    assert_eq!(edit_ranks_rows(h, 1).await?, expected);
    Ok(())
}

/// Historical setup precedes all Guild guards. Offline lower ranks have no fabricated sessions.
async fn edit_ranks_fixture(h: &Harness) -> TestResult {
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
async fn edit_ranks_other_fields(h: &Harness) -> TestResult<String> {
    Ok(sqlx::query_scalar("SELECT COALESCE(jsonb_agg(x ORDER BY x::text),'[]'::jsonb)::text FROM (SELECT jsonb_build_array('guild',to_jsonb(t)-'revision') x FROM game_guilds t UNION ALL SELECT jsonb_build_array('member',to_jsonb(t)-'rank_level') FROM game_guild_members t UNION ALL SELECT jsonb_build_array('invite',to_jsonb(t)) FROM game_guild_invitations t UNION ALL SELECT jsonb_build_array('position',to_jsonb(t)) FROM game_account_guild_leadership t) r")
        .fetch_one(&h.pool).await?)
}

fn edit_ranks_large_names() -> Vec<String> {
    (b'A'..=b'T')
        .map(|letter| format!("Rank {}", char::from(letter)))
        .collect()
}

#[test]
fn guild_edit_ranks_native_data_bounds_and_ordered_binding() -> TestResult {
    let common = request(1, 1, GuildCommand::EditRanks);
    for names in [vec!["Alpha".into(); 2], vec!["Alpha".into(); 21]] {
        assert!(matches!(
            GuildEditRanksRequest::new(common.clone(), 1, names),
            Err(GuildError::InvalidInput)
        ));
    }
    for invalid in ["", "Al\0pha"] {
        assert!(matches!(
            GuildEditRanksRequest::new(
                common.clone(),
                1,
                vec![invalid.into(), "Beta".into(), "Gamma".into()]
            ),
            Err(GuildError::InvalidInput)
        ));
    }
    assert!(
        GuildEditRanksRequest::new(
            common.clone(),
            0,
            vec!["Alpha".into(), "Beta".into(), "Gamma".into()]
        )
        .is_err()
    );
    assert!(
        GuildEditRanksRequest::new(
            request(1, 1, GuildCommand::Leave),
            1,
            vec!["Alpha".into(), "Beta".into(), "Gamma".into()]
        )
        .is_err()
    );
    let req = edit_ranks_request(1, 1, &["Alpha", "Beta", "Gamma"]);
    let first = edit_ranks_binding(&fence()?, &req).map_err(debug)?;
    for field in 0..5 {
        let mut changed = req.clone();
        match field {
            0 => changed.names.swap(0, 1),
            1 => changed.names.push("Delta".into()),
            2 => changed.names[2] = "Delta".into(),
            3 => changed.expected_revision = 2,
            _ => changed.common.guild_id = gid(2),
        }
        assert_ne!(
            edit_ranks_binding(&fence()?, &changed).map_err(debug)?,
            first
        );
    }
    let max = GuildEditRanksRequest::new(common, 1, edit_ranks_large_names()).map_err(debug)?;
    assert_eq!(max.names.len(), 20);
    // Canonical uniqueness is an actual DB refusal, not an alternate Rust name normalizer.
    assert!(
        GuildEditRanksRequest::new(
            request(2, 1, GuildCommand::EditRanks),
            1,
            vec!["Al Dric".into(), "ALDRIC".into(), "Gamma".into()]
        )
        .is_ok()
    );
    Ok(())
}

#[test]
fn guild_edit_ranks_native_rename_cycles_and_exact_contiguous_set() -> TestResult {
    text_run("guild_edit_cycle", async |h| {
        edit_ranks_fixture(h).await?;
        guard(h).await?;
        let untouched = edit_ranks_other_fields(h).await?;
        let foreign = edit_ranks_rows(h, 2).await?;
        let characters = resign_resource_character_revisions(h).await?;
        let req = edit_ranks_request(1, 1, &["Vice Leader", "Member", "Visitor", "Leader"]);
        assert_eq!(
            result(edit_ranks_apply(h, req.clone()).await?).result,
            GuildResult::Ok
        );
        edit_ranks_expect(h, &req.names).await?;
        assert_eq!(revision(h, 1).await?, "2");
        assert_eq!(revision(h, 2).await?, "1");
        assert_eq!(edit_ranks_rows(h, 2).await?, foreign);
        for (who, rank) in [(0, 1), (91, 2), (92, 3), (93, 4), (94, 1), (95, 4)] {
            assert_eq!(rank_level(h, who).await?, rank);
        }
        assert_eq!(edit_ranks_other_fields(h).await?, untouched);
        assert_eq!(resign_resource_character_revisions(h).await?, characters);
        assert_eq!(h.count("game_guild_operations").await?, 1);
        assert_eq!(h.count("game_guild_event_outbox").await?, 1);
        Ok(())
    })
}

#[test]
fn guild_edit_ranks_native_twenty_to_three_remaps_only_removed_levels() -> TestResult {
    text_run("guild_edit_remap", async |h| {
        edit_ranks_fixture(h).await?;
        guard(h).await?;
        let grow = GuildEditRanksRequest::new(
            request(1, 1, GuildCommand::EditRanks),
            1,
            edit_ranks_large_names(),
        )
        .map_err(debug)?;
        assert_eq!(
            result(edit_ranks_apply(h, grow.clone()).await?).result,
            GuildResult::Ok
        );
        edit_ranks_expect(h, &grow.names).await?;
        assert_eq!(rank_level(h, 93).await?, 4);
        // Feed the created level20 through the existing genuine SetRank writer, not raw SQL.
        assert_eq!(
            result(rank_apply(h, rank_request(2, 93, 2, 20), &pre_delivery(&h.root)).await?).result,
            GuildResult::Ok
        );
        let untouched = edit_ranks_other_fields(h).await?;
        let characters = resign_resource_character_revisions(h).await?;
        let foreign = edit_ranks_rows(h, 2).await?;
        let shrink = edit_ranks_request(3, 3, &["Rank T", "Rank A", "Rank B"]);
        assert_eq!(
            result(edit_ranks_apply(h, shrink.clone()).await?).result,
            GuildResult::Ok
        );
        edit_ranks_expect(h, &shrink.names).await?;
        for (who, rank) in [(0, 1), (91, 2), (92, 3), (93, 3), (94, 1), (95, 4)] {
            assert_eq!(rank_level(h, who).await?, rank);
        }
        assert_eq!(edit_ranks_other_fields(h).await?, untouched);
        assert_eq!(resign_resource_character_revisions(h).await?, characters);
        assert_eq!(edit_ranks_rows(h, 2).await?, foreign);
        assert_eq!(revision(h, 1).await?, "4");
        // Grow the genuine minimum3 snapshot again: removed historical level20 is not restored.
        let regrow = GuildEditRanksRequest::new(
            request(4, 1, GuildCommand::EditRanks),
            4,
            edit_ranks_large_names(),
        )
        .map_err(debug)?;
        assert_eq!(
            result(edit_ranks_apply(h, regrow.clone()).await?).result,
            GuildResult::Ok
        );
        edit_ranks_expect(h, &regrow.names).await?;
        assert_eq!(rank_level(h, 93).await?, 3);
        assert_eq!(rank_level(h, 0).await?, 1);
        assert_eq!(rank_level(h, 91).await?, 2);
        assert_eq!(edit_ranks_other_fields(h).await?, untouched);
        assert_eq!(resign_resource_character_revisions(h).await?, characters);
        assert_eq!(edit_ranks_rows(h, 2).await?, foreign);
        assert_eq!(revision(h, 1).await?, "5");
        assert_eq!(h.count("game_guild_operations").await?, 4);
        assert_eq!(h.count("game_guild_event_outbox").await?, 4);
        Ok(())
    })
}

#[test]
fn guild_edit_ranks_native_canonical_duplicates_and_stale_cas_are_atomic() -> TestResult {
    text_run("guild_edit_refusals", async |h| {
        edit_ranks_fixture(h).await?;
        guard(h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let no_payload_before = text_snapshot(h).await?;
        assert!(matches!(
            h.root
                .commit_guild_membership(
                    &authority,
                    &h.node,
                    fence()?,
                    request(99, 1, GuildCommand::EditRanks)
                )
                .await,
            Err(GuildError::InvalidInput)
        ));
        assert_eq!(text_snapshot(h).await?, no_payload_before);
        for sql in [
            "SELECT game_guild_rank_names_valid(ARRAY['Alpha','Beta']::text[])",
            "SELECT game_guild_rank_names_valid(ARRAY(SELECT 'Rank '||chr(65+g) FROM generate_series(0,20) g))",
            "SELECT game_guild_rank_names_valid(ARRAY['Alpha',NULL,'Gamma']::text[])",
            "SELECT game_guild_rank_names_valid(ARRAY['Alpha','','Gamma']::text[])",
            "SELECT game_guild_rank_names_valid(ARRAY['Al Dric','ALDRIC','Gamma']::text[])",
            "SELECT game_guild_rank_names_valid(ARRAY[['Alpha'],['Beta'],['Gamma']]::text[])",
            "SELECT game_guild_rank_names_valid('[0:2]={Alpha,Beta,Gamma}'::text[])",
        ] {
            let valid: bool = sqlx::query_scalar(sql).fetch_one(&h.pool).await?;
            assert!(!valid, "accepted malformed rank vector: {sql}");
        }
        let unchanged = rank_mutation_snapshot(h).await?;
        for (n, names) in [
            (1, ["Al Dric", "ALDRIC", "Gamma"]),
            (2, ["Alpha", "Alpha", "Gamma"]),
        ] {
            assert_eq!(
                result(edit_ranks_apply(h, edit_ranks_request(n, 1, &names)).await?).result,
                GuildResult::NameTaken
            );
            assert_eq!(rank_mutation_snapshot(h).await?, unchanged);
        }
        assert_eq!(
            result(
                edit_ranks_apply(h, edit_ranks_request(3, 2, &["Alpha", "Beta", "Gamma"])).await?
            )
            .result,
            GuildResult::StaleRevision
        );
        assert_eq!(rank_mutation_snapshot(h).await?, unchanged);
        assert_eq!(h.count("game_guild_operations").await?, 3);
        assert_eq!(h.count("game_guild_event_outbox").await?, 0);
        assert_eq!(revision(h, 1).await?, "1");
        Ok(())
    })
}

#[test]
fn guild_edit_ranks_native_current_guild_leader_and_actual_resign_roles() -> TestResult {
    text_run("guild_edit_roles", async |h| {
        edit_ranks_fixture(h).await?;
        let actors = [
            historical_session(h, 91).await?,
            historical_session(h, 92).await?,
            historical_session(h, 94).await?,
        ];
        guard(h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let unchanged = rank_mutation_snapshot(h).await?;
        for (index, actor) in actors.iter().enumerate() {
            let mut req =
                edit_ranks_request(u32::try_from(index + 1)?, 1, &["Alpha", "Beta", "Gamma"]);
            req.common.command = CommandRef::new(
                actor.game_session_id,
                CommandId::new(u64::try_from(index + 1)?)?,
            );
            assert_eq!(
                result(
                    h.root
                        .commit_guild_edit_ranks(&authority, &h.node, *actor, req)
                        .await
                        .map_err(debug)?
                )
                .result,
                GuildResult::NotAllowed
            );
            assert_eq!(rank_mutation_snapshot(h).await?, unchanged);
        }
        assert_eq!(
            result(resign_apply(h, resign_request(4, 91, 1), &pre_delivery(&h.root)).await?).result,
            GuildResult::Ok
        );
        let unchanged = rank_mutation_snapshot(h).await?;
        assert_eq!(
            result(
                edit_ranks_apply(h, edit_ranks_request(5, 2, &["Alpha", "Beta", "Gamma"])).await?
            )
            .result,
            GuildResult::NotAllowed
        );
        assert_eq!(rank_mutation_snapshot(h).await?, unchanged);
        let mut new_leader = edit_ranks_request(6, 2, &["Alpha", "Beta", "Gamma"]);
        new_leader.common.command = CommandRef::new(actors[0].game_session_id, CommandId::new(6)?);
        assert_eq!(
            result(
                h.root
                    .commit_guild_edit_ranks(&authority, &h.node, actors[0], new_leader)
                    .await
                    .map_err(debug)?
            )
            .result,
            GuildResult::Ok
        );
        assert_eq!(rank_level(h, 91).await?, 1);
        assert_eq!(rank_level(h, 0).await?, 2);
        assert_eq!(revision(h, 1).await?, "3");
        Ok(())
    })
}

#[test]
fn guild_edit_ranks_native_history_binds_name_order_count_and_revision() -> TestResult {
    text_run("guild_edit_replay", async |h| {
        edit_ranks_fixture(h).await?;
        guard(h).await?;
        let req = edit_ranks_request(1, 1, &["Alpha", "Beta", "Gamma", "Delta"]);
        let first = result(edit_ranks_apply(h, req.clone()).await?);
        assert_eq!(first.result, GuildResult::Ok);
        assert_eq!(
            result(
                edit_ranks_apply(h, edit_ranks_request(2, 2, &["Gamma", "Beta", "Alpha"])).await?
            )
            .result,
            GuildResult::Ok
        );
        let before = text_snapshot(h).await?;
        let other = DurabilityRoot::connect_test_runtime(&h.database.url)?;
        assert!(other.maintain_ready_once().await?);
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = other.open_character_authority(&seal).await.map_err(debug)?;
        let mut stale = fence()?;
        stale.connection_generation = ConnectionGeneration::new(999)?;
        let replay = other
            .commit_guild_edit_ranks(&authority, &h.node, stale, req.clone())
            .await
            .map_err(debug)?;
        assert!(matches!(&replay, GuildOutcome::Replayed(_)));
        assert_eq!(result(replay), first);
        assert_eq!(text_snapshot(h).await?, before);
        for field in 0..6 {
            let mut changed = req.clone();
            match field {
                0 => changed.names.swap(0, 1),
                1 => {
                    changed.names.pop();
                }
                2 => changed.names[0] = "Epsilon".into(),
                3 => changed.expected_revision = 2,
                4 => changed.common.guild_id = gid(2),
                _ => changed.common.operation = GuildCommand::SetMessage,
            }
            assert!(matches!(
                other
                    .commit_guild_edit_ranks(&authority, &h.node, fence()?, changed)
                    .await,
                Err(GuildError::ConflictingOccurrence)
            ));
            assert_eq!(text_snapshot(h).await?, before);
        }
        assert_eq!(revision(h, 1).await?, "3");
        assert_eq!(h.count("game_guild_operations").await?, 2);
        assert_eq!(h.count("game_guild_event_outbox").await?, 2);
        Ok(())
    })
}

#[test]
fn guild_edit_ranks_native_lifecycle_does_not_cure_or_extend_deadlines() -> TestResult {
    for state in [1_i16, 2, 3, 4] {
        text_run("guild_edit_lifecycle", async |h| {
            initial_guild(h, 1, 0).await?;
            sqlx::query("UPDATE game_guilds SET state=$1,formation_deadline=1,vice_deficit_since=$2,premium_deficit_since=$3")
                .bind(state).bind(if state == 1 { None } else { Some(1_i64) })
                .bind(if state == 1 { None } else { Some(7_i64) }).execute(&h.pool).await?;
            if state == 4 {
                h.pool
                    .execute(
                        "DELETE FROM game_account_guild_leadership;DELETE FROM game_guild_members;",
                    )
                    .await?;
            }
            guard(h).await?;
            let unchanged = edit_ranks_other_fields(h).await?;
            let before = rank_mutation_snapshot(h).await?;
            let req = edit_ranks_request(1, 1, &["Alpha", "Beta", "Gamma"]);
            assert_eq!(
                result(edit_ranks_apply(h, req.clone()).await?).result,
                if state <= 2 {
                    GuildResult::Ok
                } else {
                    GuildResult::GuildDisbanding
                }
            );
            assert_eq!(edit_ranks_other_fields(h).await?, unchanged);
            if state <= 2 {
                edit_ranks_expect(h, &req.names).await?;
            } else {
                assert_eq!(rank_mutation_snapshot(h).await?, before);
            }
            assert_eq!(h.count("game_guild_operations").await?, 1);
            assert_eq!(
                h.count("game_guild_event_outbox").await?,
                i64::from(state <= 2)
            );
            Ok(())
        })?;
    }
    Ok(())
}

#[test]
fn guild_edit_ranks_native_two_root_cas_and_occurrence_serialization() -> TestResult {
    text_run("guild_edit_races", async |h| {
        edit_ranks_fixture(h).await?;
        guard(h).await?;
        let other = DurabilityRoot::connect_test_runtime(&h.database.url)?;
        assert!(other.maintain_ready_once().await?);
        let seal = h.recovery.seal_current().map_err(debug)?;
        let a = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let b = other.open_character_authority(&seal).await.map_err(debug)?;
        let left = edit_ranks_request(1, 1, &["Alpha", "Beta", "Gamma"]);
        let right = edit_ranks_request(2, 1, &["Delta", "Epsilon", "Zeta"]);
        let (x, y) = fixture::join_two(
            h.root
                .commit_guild_edit_ranks(&a, &h.node, fence()?, left.clone()),
            other.commit_guild_edit_ranks(&b, &h.node, fence()?, right.clone()),
        )
        .await;
        let (x, y) = (result(x.map_err(debug)?), result(y.map_err(debug)?));
        assert_eq!(
            usize::from(x.result == GuildResult::Ok) + usize::from(y.result == GuildResult::Ok),
            1
        );
        assert_eq!(
            usize::from(x.result == GuildResult::StaleRevision)
                + usize::from(y.result == GuildResult::StaleRevision),
            1
        );
        edit_ranks_expect(
            h,
            if x.result == GuildResult::Ok {
                &left.names
            } else {
                &right.names
            },
        )
        .await?;
        let same = edit_ranks_request(3, 2, &["Eta", "Theta", "Iota", "Kappa"]);
        let (x, y) = fixture::join_two(
            h.root
                .commit_guild_edit_ranks(&a, &h.node, fence()?, same.clone()),
            other.commit_guild_edit_ranks(&b, &h.node, fence()?, same.clone()),
        )
        .await;
        let (x, y) = (x.map_err(debug)?, y.map_err(debug)?);
        assert_eq!(
            usize::from(matches!(&x, GuildOutcome::Committed(_)))
                + usize::from(matches!(&y, GuildOutcome::Committed(_))),
            1
        );
        assert_eq!(result(x), result(y));
        edit_ranks_expect(h, &same.names).await?;
        assert_eq!(revision(h, 1).await?, "3");
        assert_eq!(h.count("game_guild_operations").await?, 3);
        assert_eq!(h.count("game_guild_event_outbox").await?, 2);
        Ok(())
    })
}

#[test]
fn guild_edit_ranks_native_closed_sql_and_after_remap_rollback() -> TestResult {
    text_run("guild_edit_atomic", async |h| {
        edit_ranks_fixture(h).await?;
        guard(h).await?;
        let req = edit_ranks_request(1, 1, &["Alpha", "Beta", "Gamma"]);
        let before = text_snapshot(h).await?;
        for sql in [
            "UPDATE game_guild_ranks SET name='Forged' WHERE level=3",
            "DELETE FROM game_guild_ranks WHERE level=4",
            "INSERT INTO game_guild_ranks(guild_id,level,name) SELECT guild_id,5,'Forged' FROM game_guilds",
            "UPDATE game_guild_members SET rank_level=3 WHERE rank_level=4",
        ] {
            let error = h
                .pool
                .execute(sql)
                .await
                .expect_err("unbound raw mutation accepted");
            assert_eq!(
                error
                    .as_database_error()
                    .and_then(|error| error.code())
                    .as_deref(),
                Some("23514")
            );
            assert_eq!(text_snapshot(h).await?, before);
        }
        for (setup, cleanup) in [
            (
                "CREATE FUNCTION guild_edit_test_fault() RETURNS TRIGGER LANGUAGE plpgsql AS $$BEGIN NEW.title='forged';RETURN NEW;END$$;CREATE TRIGGER aaa_guild_edit_test_fault BEFORE UPDATE ON game_guild_members FOR EACH ROW EXECUTE FUNCTION guild_edit_test_fault();",
                "DROP TRIGGER aaa_guild_edit_test_fault ON game_guild_members;DROP FUNCTION guild_edit_test_fault();",
            ),
            (
                "CREATE FUNCTION guild_edit_test_fault() RETURNS TRIGGER LANGUAGE plpgsql AS $$BEGIN IF NEW.level=2 THEN NEW.name='Unbound';END IF;RETURN NEW;END$$;CREATE TRIGGER aaa_guild_edit_test_fault BEFORE UPDATE ON game_guild_ranks FOR EACH ROW EXECUTE FUNCTION guild_edit_test_fault();",
                "DROP TRIGGER aaa_guild_edit_test_fault ON game_guild_ranks;DROP FUNCTION guild_edit_test_fault();",
            ),
            (
                "CREATE FUNCTION guild_edit_test_fault() RETURNS TRIGGER LANGUAGE plpgsql AS $$BEGIN NEW.level=5;RETURN NEW;END$$;CREATE TRIGGER aaa_guild_edit_test_fault BEFORE UPDATE ON game_guild_ranks FOR EACH ROW EXECUTE FUNCTION guild_edit_test_fault();",
                "DROP TRIGGER aaa_guild_edit_test_fault ON game_guild_ranks;DROP FUNCTION guild_edit_test_fault();",
            ),
            (
                "CREATE FUNCTION guild_edit_test_fault() RETURNS TRIGGER LANGUAGE plpgsql AS $$BEGIN RAISE EXCEPTION 'native post-remap event failure' USING ERRCODE='23514';END$$;CREATE CONSTRAINT TRIGGER guild_edit_test_fault AFTER INSERT ON game_guild_event_outbox DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION guild_edit_test_fault();",
                "DROP TRIGGER guild_edit_test_fault ON game_guild_event_outbox;DROP FUNCTION guild_edit_test_fault();",
            ),
        ] {
            h.pool.execute(setup).await?;
            assert!(edit_ranks_apply(h, req.clone()).await.is_err());
            assert_eq!(text_snapshot(h).await?, before);
            assert_eq!(rank_level(h, 93).await?, 4);
            h.pool.execute(cleanup).await?;
        }
        let outcome = edit_ranks_apply(h, req.clone()).await?;
        assert!(matches!(&outcome, GuildOutcome::Committed(_)));
        assert_eq!(result(outcome).result, GuildResult::Ok);
        edit_ranks_expect(h, &req.names).await?;
        assert_eq!(rank_level(h, 93).await?, 3);
        assert_eq!(h.count("game_guild_operations").await?, 1);
        assert_eq!(h.count("game_guild_event_outbox").await?, 1);
        Ok(())
    })?;
    text_run("guild_edit_overflow", async |h| {
        edit_ranks_fixture(h).await?;
        sqlx::query("UPDATE game_guilds SET revision=18446744073709551615 WHERE guild_id=encode($1,'hex')::uuid").bind(gid(1).as_bytes().as_slice()).execute(&h.pool).await?;
        guard(h).await?;
        let before = text_snapshot(h).await?;
        assert!(
            edit_ranks_apply(
                h,
                edit_ranks_request(1, u64::MAX, &["Alpha", "Beta", "Gamma"])
            )
            .await
            .is_err()
        );
        assert_eq!(text_snapshot(h).await?, before);
        Ok(())
    })
}

#[test]
fn guild_edit_ranks_native_real_actor_fences_and_cross_root_authority() -> TestResult {
    text_run("guild_edit_fences", async |h| {
        edit_ranks_fixture(h).await?;
        guard(h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let req = edit_ranks_request(1, 1, &["Alpha", "Beta", "Gamma"]);
        let before = text_snapshot(h).await?;
        let wrong = fixture::register(&h.root, 2).await?;
        let recovery = authority.record_for(&h.root).map_err(debug)?;
        for field in 0..8 {
            let mut stale = fence()?;
            match field {
                0 => stale.connection_generation = ConnectionGeneration::new(2)?,
                1 => stale.character_lease_generation = 2,
                2 => stale.scope_ownership_generation = ScopeOwnershipGeneration::new(2)?,
                3 => stale.character_id = character(999),
                4 => stale.game_session_id = GameSessionId::decode(&sid(999, 20))?,
                5 => {
                    if let crate::foundation::RuntimeScopeRefV1::Channel { channel_id, .. } =
                        stale.runtime_scope
                    {
                        stale.runtime_scope = crate::foundation::RuntimeScopeRefV1::channel(
                            crate::foundation::WorldId::decode(&sid(999, 70))?,
                            channel_id,
                        );
                    }
                }
                6 => {
                    if let crate::foundation::RuntimeScopeRefV1::Channel { world_id, .. } =
                        stale.runtime_scope
                    {
                        stale.runtime_scope = crate::foundation::RuntimeScopeRefV1::channel(
                            world_id,
                            crate::foundation::ChannelId::decode(&sid(999, 71))?,
                        );
                    }
                }
                _ => {}
            }
            let node = if field == 7 { &wrong } else { &h.node };
            let mut case_req = req.clone();
            if field == 4 {
                case_req.common.command =
                    CommandRef::new(stale.game_session_id, req.common.command.command_id());
            }
            assert!(matches!(
                h.root
                    .commit_guild_edit_ranks(&authority, node, stale, case_req.clone())
                    .await,
                Err(GuildError::NotCurrent)
            ));
            let write = EditRanksWrite {
                node: node.clone(),
                fence: stale,
                edit_ranks: case_req.clone(),
                binding: edit_ranks_binding(&stale, &case_req).map_err(debug)?,
            };
            let mut tx = h.pool.begin().await?;
            assert!(matches!(
                write.execute(&mut tx, &recovery).await,
                Err(GuildError::NotCurrent)
            ));
            tx.rollback().await?;
            assert_eq!(text_snapshot(h).await?, before);
        }
        let other = DurabilityRoot::connect_test_runtime(&h.database.url)?;
        assert!(other.maintain_ready_once().await?);
        assert!(matches!(
            other
                .commit_guild_edit_ranks(&authority, &h.node, fence()?, req.clone())
                .await,
            Err(GuildError::NotCurrent)
        ));
        assert_eq!(text_snapshot(h).await?, before);
        // The same real capability works before actual revocation, ruling out a broken fixture.
        assert_eq!(
            result(
                h.root
                    .commit_guild_edit_ranks(&authority, &h.node, fence()?, req)
                    .await
                    .map_err(debug)?
            )
            .result,
            GuildResult::Ok
        );
        let before = text_snapshot(h).await?;
        let req = edit_ranks_request(2, 2, &["Alpha", "Beta", "Gamma", "Delta"]);
        use crate::durability::runtime_scope_assignment::{
            AssignmentCommand, AssignmentOutcome, AssignmentRequest, AssignmentState, ControlActor,
            OperationKey, RuntimeScopeAssignmentWriter,
        };
        let actor: String = sqlx::query_scalar("SELECT session_user")
            .fetch_one(&h.pool)
            .await?;
        assert_eq!(actor, "oteryn_test_admin");
        sqlx::query("INSERT INTO game_control_scope_grants(control_role,world_id,channel_id,operation) VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,3)").bind(id(WORLD).as_slice()).bind(id(fixture::CHANNEL).as_slice()).execute(&h.pool).await?;
        let writer = RuntimeScopeAssignmentWriter::open(h.root.clone(), "item-transfer-writer")
            .await
            .map_err(debug)?;
        let predecessor = h
            .root
            .read_runtime_scope_predecessor(fence()?.runtime_scope)
            .await
            .map_err(debug)?
            .ok_or("missing assignment")?;
        assert!(matches!(
            writer
                .submit(&AssignmentRequest {
                    operation_key: OperationKey::from_bytes([90; 32]),
                    actor: ControlActor::new(&actor).map_err(debug)?,
                    command: AssignmentCommand::Revoke {
                        scope: fence()?.runtime_scope,
                        predecessor
                    },
                })
                .await
                .map_err(debug)?,
            AssignmentOutcome::Committed(_)
        ));
        let actual = h
            .root
            .read_runtime_scope_assignment(fence()?.runtime_scope)
            .await
            .map_err(debug)?
            .ok_or("missing revoked assignment")?;
        assert_eq!(actual.state, AssignmentState::Revoked);
        assert_eq!(actual.ownership_generation, 2);
        assert!(matches!(
            h.root
                .commit_guild_edit_ranks(&authority, &h.node, fence()?, req.clone())
                .await,
            Err(GuildError::NotCurrent)
        ));
        let write = EditRanksWrite {
            node: h.node.clone(),
            fence: fence()?,
            edit_ranks: req.clone(),
            binding: edit_ranks_binding(&fence()?, &req).map_err(debug)?,
        };
        let mut tx = h.pool.begin().await?;
        assert!(matches!(
            write.execute(&mut tx, &recovery).await,
            Err(GuildError::NotCurrent)
        ));
        tx.rollback().await?;
        assert_eq!(text_snapshot(h).await?, before);
        Ok(())
    })
}

#[test]
fn guild_edit_ranks_native_recovery_precedes_fresh_write_and_historical_replay() -> TestResult {
    text_run("guild_edit_recovery", async |h| {
        edit_ranks_fixture(h).await?;
        guard(h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut stale = authority.record_for(&h.root).map_err(debug)?;
        stale.recovery_generation += 1;
        let req = edit_ranks_request(1, 1, &["Alpha", "Beta", "Gamma"]);
        let write = EditRanksWrite {
            node: h.node.clone(),
            fence: fence()?,
            edit_ranks: req.clone(),
            binding: edit_ranks_binding(&fence()?, &req).map_err(debug)?,
        };
        let before = text_snapshot(h).await?;
        let mut tx = h.pool.begin().await?;
        assert!(write.execute(&mut tx, &stale).await.is_err());
        tx.rollback().await?;
        assert_eq!(text_snapshot(h).await?, before);
        assert_eq!(
            result(edit_ranks_apply(h, req).await?).result,
            GuildResult::Ok
        );
        let before = text_snapshot(h).await?;
        let mut tx = h.pool.begin().await?;
        assert!(write.execute(&mut tx, &stale).await.is_err());
        tx.rollback().await?;
        assert_eq!(text_snapshot(h).await?, before);
        Ok(())
    })
}

fn edit_ranks_test_audit(req: &GuildEditRanksRequest) -> TestResult<serde_json::Value> {
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

#[test]
fn guild_edit_ranks_native_forged_rank_set_and_closed_event_proofs_rollback() -> TestResult {
    text_run("guild_edit_proofs", async |h| {
        edit_ranks_fixture(h).await?;
        guard(h).await?;
        let req = edit_ranks_request(1, 1, &["Alpha", "Beta", "Gamma"]);
        let before = text_snapshot(h).await?;
        // Corrupt only newly proposed rows, never overwrite an immutable committed receipt.
        h.pool.execute("CREATE TABLE guild_edit_test_payload(payload BYTEA NOT NULL);CREATE FUNCTION guild_edit_test_payload_op() RETURNS TRIGGER LANGUAGE plpgsql AS $$BEGIN SELECT sha256(payload) INTO NEW.audit_sha256 FROM guild_edit_test_payload;RETURN NEW;END$$;CREATE TRIGGER aaa_guild_edit_test_payload_op BEFORE INSERT ON game_guild_operations FOR EACH ROW EXECUTE FUNCTION guild_edit_test_payload_op();CREATE FUNCTION guild_edit_test_payload_event() RETURNS TRIGGER LANGUAGE plpgsql AS $$BEGIN SELECT payload INTO NEW.payload FROM guild_edit_test_payload;NEW.payload_sha256=sha256(NEW.payload);RETURN NEW;END$$;CREATE TRIGGER aaa_guild_edit_test_payload_event BEFORE INSERT ON game_guild_event_outbox FOR EACH ROW EXECUTE FUNCTION guild_edit_test_payload_event();").await?;
        let valid = edit_ranks_test_audit(&req)?;
        let mut altered_values = Vec::new();
        let mut reordered = valid.clone();
        reordered["names"] = serde_json::json!(["Beta", "Alpha", "Gamma"]);
        altered_values.push(reordered);
        let mut truncated = valid.clone();
        truncated["names"] = serde_json::json!(["Alpha", "Beta"]);
        altered_values.push(truncated);
        let mut replaced = valid.clone();
        replaced["names"] = serde_json::json!(["Alpha", "Beta", "Delta"]);
        altered_values.push(replaced);
        let mut wrong_cas = valid.clone();
        wrong_cas["expected_revision"] = serde_json::json!(2);
        altered_values.push(wrong_cas);
        let mut missing = valid.clone();
        missing
            .as_object_mut()
            .ok_or("audit not object")?
            .remove("names");
        altered_values.push(missing);
        let mut extra = valid;
        extra["caller_is_leader"] = serde_json::json!(true);
        altered_values.push(extra);
        for altered in altered_values {
            sqlx::query("DELETE FROM guild_edit_test_payload")
                .execute(&h.pool)
                .await?;
            sqlx::query("INSERT INTO guild_edit_test_payload VALUES($1)")
                .bind(serde_json::to_vec(&altered)?)
                .execute(&h.pool)
                .await?;
            assert!(edit_ranks_apply(h, req.clone()).await.is_err());
            assert_eq!(text_snapshot(h).await?, before);
        }
        h.pool.execute("DROP TRIGGER aaa_guild_edit_test_payload_op ON game_guild_operations;DROP TRIGGER aaa_guild_edit_test_payload_event ON game_guild_event_outbox;DROP FUNCTION guild_edit_test_payload_op();DROP FUNCTION guild_edit_test_payload_event();DROP TABLE guild_edit_test_payload;").await?;
        // Same expected revision and actual final rows must still bind the exact ordered vector.
        h.pool.execute("CREATE FUNCTION guild_edit_test_names() RETURNS TRIGGER LANGUAGE plpgsql AS $$BEGIN NEW.edit_rank_names=ARRAY['Beta','Alpha','Gamma'];RETURN NEW;END$$;CREATE TRIGGER aaa_guild_edit_test_names BEFORE INSERT ON game_guild_operations FOR EACH ROW EXECUTE FUNCTION guild_edit_test_names();").await?;
        assert!(edit_ranks_apply(h, req.clone()).await.is_err());
        assert_eq!(text_snapshot(h).await?, before);
        h.pool.execute("DROP TRIGGER aaa_guild_edit_test_names ON game_guild_operations;DROP FUNCTION guild_edit_test_names();").await?;
        assert_eq!(
            result(edit_ranks_apply(h, req).await?).result,
            GuildResult::Ok
        );
        assert_eq!(h.count("game_guild_operations").await?, 1);
        assert_eq!(h.count("game_guild_event_outbox").await?, 1);
        Ok(())
    })
}

// Proposal only. Append after the eventual unchanged Resign60 + accepted EditRanks12.
// Decline uses the real membership owner, not a new authority constructor or Hall/Bank port.

async fn decline_fixture(h: &Harness, expired: bool) -> TestResult {
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

async fn decline_invitation_rows(h: &Harness) -> TestResult<Vec<(String, String, String, i64)>> {
    Ok(sqlx::query_as("SELECT guild_id::text,character_id::text,inviter::text,expires_at FROM game_guild_invitations ORDER BY guild_id,character_id")
        .fetch_all(&h.pool).await?)
}

async fn decline_non_invitation_fields(h: &Harness) -> TestResult<String> {
    Ok(sqlx::query_scalar("SELECT COALESCE(jsonb_agg(x ORDER BY x::text),'[]'::jsonb)::text FROM (SELECT jsonb_build_array('guild',to_jsonb(t)-'revision') x FROM game_guilds t UNION ALL SELECT jsonb_build_array('member',to_jsonb(t)) FROM game_guild_members t UNION ALL SELECT jsonb_build_array('rank',to_jsonb(t)) FROM game_guild_ranks t UNION ALL SELECT jsonb_build_array('position',to_jsonb(t)) FROM game_account_guild_leadership t) r")
        .fetch_one(&h.pool).await?)
}

fn decline_test_audit(req: &GuildRequest) -> GuildDeclineAuditProposal {
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

#[test]
fn guild_decline_native_actor_target_binding_and_closed_codec() -> TestResult {
    let req = request(1, 1, GuildCommand::Decline);
    assert_eq!(req.operation.target(character(0)), character(0));
    for op in [
        GuildCommand::Accept,
        GuildCommand::Revoke(character(0)),
        GuildCommand::Leave,
    ] {
        assert_ne!(
            binding(&fence()?, &req).map_err(debug)?,
            binding(&fence()?, &request(1, 1, op)).map_err(debug)?
        );
    }
    let valid = decline_test_audit(&req);
    let bytes = valid
        .checked_bytes()
        .ok_or("valid Decline audit rejected")?;
    let value: serde_json::Value = serde_json::from_slice(&bytes)?;
    assert_eq!(value["removed_invitations"], 1);
    assert_eq!(value["action"], 12);
    assert!(
        valid.operation.checked_bytes().is_none(),
        "old codec must remain closed"
    );
    for field in 0..5 {
        let mut changed = valid.clone();
        match field {
            0 => changed.operation.target = *character(92).as_bytes(),
            1 => changed.operation.removed_invitations = 0,
            2 => changed.operation.rank_before = Some(1),
            3 => changed.operation.revision = 3,
            _ => changed.operation.action = 2,
        }
        assert!(changed.checked_bytes().is_none());
    }
    Ok(())
}

#[test]
fn guild_decline_native_only_own_selected_invitation_even_expired_and_other_guild_member()
-> TestResult {
    for expired in [false, true] {
        text_run("guild_decline_own", async |h| {
            decline_fixture(h, expired).await?;
            guard(h).await?;
            let original = decline_non_invitation_fields(h).await?;
            let characters = resign_resource_character_revisions(h).await?;
            let mut expected = decline_invitation_rows(h).await?;
            expected.retain(|(g, c, _, _)| {
                *g != uuid(*gid(1).as_bytes()) || *c != uuid(*character(0).as_bytes())
            });
            // Actor leads a DIFFERENT Guild: that never grants this Guild's Revoke permission.
            assert_eq!(
                result(
                    apply(
                        h,
                        request(1, 1, GuildCommand::Revoke(character(0))),
                        fence()?
                    )
                    .await?
                )
                .result,
                GuildResult::NotAllowed
            );
            assert_eq!(
                result(apply(h, request(2, 1, GuildCommand::Decline), fence()?).await?).result,
                GuildResult::Ok
            );
            assert_eq!(decline_invitation_rows(h).await?, expected);
            assert_eq!(decline_non_invitation_fields(h).await?, original);
            assert_eq!(resign_resource_character_revisions(h).await?, characters);
            assert_eq!(revision(h, 1).await?, "2");
            for guild in [2, 3, 4] {
                assert_eq!(revision(h, guild).await?, "1");
            }
            let unchanged = rank_mutation_snapshot(h).await?;
            assert_eq!(
                result(apply(h, request(3, 1, GuildCommand::Decline), fence()?).await?).result,
                GuildResult::NotInvited
            );
            assert_eq!(rank_mutation_snapshot(h).await?, unchanged);
            assert_eq!(h.count("game_guild_operations").await?, 3);
            assert_eq!(h.count("game_guild_event_outbox").await?, 1);
            Ok(())
        })?;
    }
    Ok(())
}

#[test]
fn guild_decline_native_replay_binding_and_two_root_single_effect() -> TestResult {
    text_run("guild_decline_history", async |h| {
        decline_fixture(h, false).await?;
        guard(h).await?;
        let req = request(1, 1, GuildCommand::Decline);
        let first = result(apply(h, req.clone(), fence()?).await?);
        assert_eq!(first.result, GuildResult::Ok);
        let before = text_snapshot(h).await?;
        let other = DurabilityRoot::connect_test_runtime(&h.database.url)?;
        assert!(other.maintain_ready_once().await?);
        let seal = h.recovery.seal_current().map_err(debug)?;
        let a = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let b = other.open_character_authority(&seal).await.map_err(debug)?;
        let mut stale_fence = fence()?;
        stale_fence.connection_generation = ConnectionGeneration::new(999)?;
        let replay = other
            .commit_guild_membership(&b, &h.node, stale_fence, req.clone())
            .await
            .map_err(debug)?;
        assert!(matches!(&replay, GuildOutcome::Replayed(_)));
        assert_eq!(result(replay), first);
        for field in 0..3 {
            let mut changed = req.clone();
            match field {
                0 => changed.guild_id = gid(3),
                1 => changed.operation = GuildCommand::Revoke(character(0)),
                _ => changed.command = request(10, 1, GuildCommand::Decline).command,
            }
            assert!(matches!(
                other
                    .commit_guild_membership(&b, &h.node, fence()?, changed)
                    .await,
                Err(GuildError::ConflictingOccurrence)
            ));
            assert_eq!(text_snapshot(h).await?, before);
        }
        let mut recovery = a.record_for(&h.root).map_err(debug)?;
        recovery.recovery_generation += 1;
        let mut tx = h.pool.begin().await?;
        assert!(
            lock_fenced_guild_operation(&mut tx, &recovery, &req)
                .await
                .is_err()
        );
        tx.rollback().await?;
        assert_eq!(text_snapshot(h).await?, before);
        let same = request(2, 3, GuildCommand::Decline);
        let (x, y) = fixture::join_two(
            h.root
                .commit_guild_membership(&a, &h.node, fence()?, same.clone()),
            other.commit_guild_membership(&b, &h.node, fence()?, same),
        )
        .await;
        let (x, y) = (x.map_err(debug)?, y.map_err(debug)?);
        assert_eq!(
            usize::from(matches!(&x, GuildOutcome::Committed(_)))
                + usize::from(matches!(&y, GuildOutcome::Committed(_))),
            1
        );
        assert_eq!(result(x), result(y));
        let (x, y) = fixture::join_two(
            h.root.commit_guild_membership(
                &a,
                &h.node,
                fence()?,
                request(3, 4, GuildCommand::Decline),
            ),
            other.commit_guild_membership(
                &b,
                &h.node,
                fence()?,
                request(4, 4, GuildCommand::Decline),
            ),
        )
        .await;
        let answers = [
            result(x.map_err(debug)?).result,
            result(y.map_err(debug)?).result,
        ];
        assert_eq!(
            answers
                .iter()
                .filter(|value| **value == GuildResult::Ok)
                .count(),
            1
        );
        assert_eq!(
            answers
                .iter()
                .filter(|value| **value == GuildResult::NotInvited)
                .count(),
            1
        );
        assert_eq!(h.count("game_guild_operations").await?, 4);
        assert_eq!(h.count("game_guild_event_outbox").await?, 3);
        Ok(())
    })
}

#[test]
fn guild_decline_native_current_session_node_and_physical_tx_guards() -> TestResult {
    text_run("guild_decline_fences", async |h| {
        decline_fixture(h, false).await?;
        guard(h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let recovery = authority.record_for(&h.root).map_err(debug)?;
        let req = request(1, 1, GuildCommand::Decline);
        let before = text_snapshot(h).await?;
        let wrong = fixture::register(&h.root, 2).await?;
        for field in 0..8 {
            let mut stale = fence()?;
            match field {
                0 => stale.connection_generation = ConnectionGeneration::new(2)?,
                1 => stale.character_lease_generation = 2,
                2 => stale.scope_ownership_generation = ScopeOwnershipGeneration::new(2)?,
                3 => stale.character_id = character(999),
                4 => stale.game_session_id = GameSessionId::decode(&sid(999, 20))?,
                5 => {
                    if let crate::foundation::RuntimeScopeRefV1::Channel { channel_id, .. } =
                        stale.runtime_scope
                    {
                        stale.runtime_scope = crate::foundation::RuntimeScopeRefV1::channel(
                            crate::foundation::WorldId::decode(&sid(999, 70))?,
                            channel_id,
                        );
                    }
                }
                6 => {
                    if let crate::foundation::RuntimeScopeRefV1::Channel { world_id, .. } =
                        stale.runtime_scope
                    {
                        stale.runtime_scope = crate::foundation::RuntimeScopeRefV1::channel(
                            world_id,
                            crate::foundation::ChannelId::decode(&sid(999, 71))?,
                        );
                    }
                }
                _ => {}
            }
            let node = if field == 7 { &wrong } else { &h.node };
            let mut case_req = req.clone();
            if field == 4 {
                // Keep DATA binding internally consistent to reach the real current-session guard.
                case_req.command = CommandRef::new(stale.game_session_id, req.command.command_id());
            }
            assert!(matches!(
                h.root
                    .commit_guild_membership(&authority, node, stale, case_req.clone())
                    .await,
                Err(GuildError::NotCurrent)
            ));
            let mut tx = h.pool.begin().await?;
            lock_fenced_guild_operation(&mut tx, &recovery, &case_req)
                .await
                .map_err(debug)?;
            assert!(matches!(
                execute(
                    &mut tx,
                    node,
                    &stale,
                    &case_req,
                    &binding(&stale, &case_req).map_err(debug)?,
                    None,
                    None
                )
                .await,
                Err(GuildError::NotCurrent)
            ));
            tx.rollback().await?;
            assert_eq!(text_snapshot(h).await?, before);
        }
        let other = DurabilityRoot::connect_test_runtime(&h.database.url)?;
        assert!(other.maintain_ready_once().await?);
        assert!(matches!(
            other
                .commit_guild_membership(&authority, &h.node, fence()?, req.clone())
                .await,
            Err(GuildError::NotCurrent)
        ));
        assert_eq!(text_snapshot(h).await?, before);
        assert_eq!(
            result(apply(h, req, fence()?).await?).result,
            GuildResult::Ok
        );
        let before = text_snapshot(h).await?;
        let fresh = request(2, 3, GuildCommand::Decline);
        use crate::durability::runtime_scope_assignment::{
            AssignmentCommand, AssignmentOutcome, AssignmentRequest, AssignmentState, ControlActor,
            OperationKey, RuntimeScopeAssignmentWriter,
        };
        let actor: String = sqlx::query_scalar("SELECT session_user")
            .fetch_one(&h.pool)
            .await?;
        assert_eq!(actor, "oteryn_test_admin");
        sqlx::query("INSERT INTO game_control_scope_grants(control_role,world_id,channel_id,operation) VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,3)").bind(id(WORLD).as_slice()).bind(id(fixture::CHANNEL).as_slice()).execute(&h.pool).await?;
        let writer = RuntimeScopeAssignmentWriter::open(h.root.clone(), "item-transfer-writer")
            .await
            .map_err(debug)?;
        let predecessor = h
            .root
            .read_runtime_scope_predecessor(fence()?.runtime_scope)
            .await
            .map_err(debug)?
            .ok_or("missing assignment")?;
        assert!(matches!(
            writer
                .submit(&AssignmentRequest {
                    operation_key: OperationKey::from_bytes([91; 32]),
                    actor: ControlActor::new(&actor).map_err(debug)?,
                    command: AssignmentCommand::Revoke {
                        scope: fence()?.runtime_scope,
                        predecessor
                    },
                })
                .await
                .map_err(debug)?,
            AssignmentOutcome::Committed(_)
        ));
        let actual = h
            .root
            .read_runtime_scope_assignment(fence()?.runtime_scope)
            .await
            .map_err(debug)?
            .ok_or("missing revoked assignment")?;
        assert_eq!(actual.state, AssignmentState::Revoked);
        assert_eq!(actual.ownership_generation, 2);
        assert!(matches!(
            h.root
                .commit_guild_membership(&authority, &h.node, fence()?, fresh.clone())
                .await,
            Err(GuildError::NotCurrent)
        ));
        let mut tx = h.pool.begin().await?;
        lock_fenced_guild_operation(&mut tx, &recovery, &fresh)
            .await
            .map_err(debug)?;
        assert!(matches!(
            execute(
                &mut tx,
                &h.node,
                &fence()?,
                &fresh,
                &binding(&fence()?, &fresh).map_err(debug)?,
                None,
                None
            )
            .await,
            Err(GuildError::NotCurrent)
        ));
        tx.rollback().await?;
        assert_eq!(text_snapshot(h).await?, before);
        Ok(())
    })
}

#[test]
fn guild_decline_native_terminal_guild_state_never_cleans_invitation() -> TestResult {
    for state in [3_i16, 4] {
        text_run("guild_decline_state", async |h| {
            decline_fixture(h, false).await?;
            sqlx::query("UPDATE game_guilds SET state=$1 WHERE guild_id=encode($2,'hex')::uuid")
                .bind(state)
                .bind(gid(1).as_bytes().as_slice())
                .execute(&h.pool)
                .await?;
            if state == 4 {
                // Valid terminal Guild has no membership or invitation left; no fake Hall/Bank fact.
                sqlx::query("DELETE FROM game_account_guild_leadership WHERE guild_id=encode($1,'hex')::uuid").bind(gid(1).as_bytes().as_slice()).execute(&h.pool).await?;
                sqlx::query("DELETE FROM game_guild_members WHERE guild_id=encode($1,'hex')::uuid")
                    .bind(gid(1).as_bytes().as_slice())
                    .execute(&h.pool)
                    .await?;
                sqlx::query(
                    "DELETE FROM game_guild_invitations WHERE guild_id=encode($1,'hex')::uuid",
                )
                .bind(gid(1).as_bytes().as_slice())
                .execute(&h.pool)
                .await?;
            }
            guard(h).await?;
            let before = rank_mutation_snapshot(h).await?;
            assert_eq!(
                result(apply(h, request(1, 1, GuildCommand::Decline), fence()?).await?).result,
                GuildResult::GuildDisbanding
            );
            assert_eq!(rank_mutation_snapshot(h).await?, before);
            assert_eq!(h.count("game_guild_event_outbox").await?, 0);
            Ok(())
        })?;
    }
    Ok(())
}

#[test]
fn guild_decline_native_sql_delete_and_event_failure_are_atomic() -> TestResult {
    text_run("guild_decline_atomic", async |h| {
        decline_fixture(h, false).await?;
        guard(h).await?;
        let before = text_snapshot(h).await?;
        let raw = sqlx::query("DELETE FROM game_guild_invitations WHERE guild_id=encode($1,'hex')::uuid AND character_id=encode($2,'hex')::uuid")
            .bind(gid(1).as_bytes().as_slice()).bind(character(0).as_bytes().as_slice()).execute(&h.pool).await.expect_err("unbound invitation delete accepted");
        assert_eq!(
            raw.as_database_error()
                .and_then(|error| error.code())
                .as_deref(),
            Some("23514")
        );
        for (setup, cleanup) in [
            (
                "CREATE FUNCTION guild_decline_test_fault() RETURNS TRIGGER LANGUAGE plpgsql AS $$BEGIN RETURN NULL;END$$;CREATE TRIGGER aaa_guild_decline_test_fault BEFORE DELETE ON game_guild_invitations FOR EACH ROW EXECUTE FUNCTION guild_decline_test_fault();",
                "DROP TRIGGER aaa_guild_decline_test_fault ON game_guild_invitations;DROP FUNCTION guild_decline_test_fault();",
            ),
            (
                "CREATE FUNCTION guild_decline_test_fault() RETURNS TRIGGER LANGUAGE plpgsql AS $$BEGIN RAISE EXCEPTION 'native deferred event failure' USING ERRCODE='23514';END$$;CREATE CONSTRAINT TRIGGER guild_decline_test_fault AFTER INSERT ON game_guild_event_outbox DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION guild_decline_test_fault();",
                "DROP TRIGGER guild_decline_test_fault ON game_guild_event_outbox;DROP FUNCTION guild_decline_test_fault();",
            ),
            (
                "CREATE FUNCTION guild_decline_test_fault() RETURNS TRIGGER LANGUAGE plpgsql AS $$BEGIN NEW.vice_deficit_since=99;RETURN NEW;END$$;CREATE TRIGGER aaa_guild_decline_test_fault BEFORE UPDATE ON game_guilds FOR EACH ROW EXECUTE FUNCTION guild_decline_test_fault();",
                "DROP TRIGGER aaa_guild_decline_test_fault ON game_guilds;DROP FUNCTION guild_decline_test_fault();",
            ),
        ] {
            h.pool.execute(setup).await?;
            assert!(
                apply(h, request(1, 1, GuildCommand::Decline), fence()?)
                    .await
                    .is_err()
            );
            assert_eq!(text_snapshot(h).await?, before);
            h.pool.execute(cleanup).await?;
        }
        h.pool.execute("CREATE TABLE guild_decline_test_payload(payload BYTEA NOT NULL);CREATE FUNCTION guild_decline_test_op() RETURNS TRIGGER LANGUAGE plpgsql AS $$BEGIN SELECT sha256(payload) INTO NEW.audit_sha256 FROM guild_decline_test_payload;RETURN NEW;END$$;CREATE TRIGGER aaa_guild_decline_test_op BEFORE INSERT ON game_guild_operations FOR EACH ROW EXECUTE FUNCTION guild_decline_test_op();CREATE FUNCTION guild_decline_test_event() RETURNS TRIGGER LANGUAGE plpgsql AS $$BEGIN SELECT payload INTO NEW.payload FROM guild_decline_test_payload;NEW.payload_sha256=sha256(NEW.payload);RETURN NEW;END$$;CREATE TRIGGER aaa_guild_decline_test_event BEFORE INSERT ON game_guild_event_outbox FOR EACH ROW EXECUTE FUNCTION guild_decline_test_event();").await?;
        let valid =
            serde_json::to_value(decline_test_audit(&request(1, 1, GuildCommand::Decline)))?;
        for field in 0..5 {
            let mut changed = valid.clone();
            match field {
                0 => changed["target"] = serde_json::to_value(character(92).as_bytes())?,
                1 => changed["removed_invitations"] = serde_json::json!(0),
                2 => changed["action"] = serde_json::json!(2),
                3 => changed["occurred_at_unix_ms"] = serde_json::json!(1),
                _ => changed["caller_is_invited"] = serde_json::json!(true),
            }
            sqlx::query("DELETE FROM guild_decline_test_payload")
                .execute(&h.pool)
                .await?;
            sqlx::query("INSERT INTO guild_decline_test_payload VALUES($1)")
                .bind(serde_json::to_vec(&changed)?)
                .execute(&h.pool)
                .await?;
            assert!(
                apply(h, request(1, 1, GuildCommand::Decline), fence()?)
                    .await
                    .is_err()
            );
            assert_eq!(text_snapshot(h).await?, before);
        }
        h.pool.execute("DROP TRIGGER aaa_guild_decline_test_op ON game_guild_operations;DROP TRIGGER aaa_guild_decline_test_event ON game_guild_event_outbox;DROP FUNCTION guild_decline_test_op();DROP FUNCTION guild_decline_test_event();DROP TABLE guild_decline_test_payload;").await?;
        assert_eq!(
            result(apply(h, request(1, 1, GuildCommand::Decline), fence()?).await?).result,
            GuildResult::Ok
        );
        let committed = text_snapshot(h).await?;
        assert!(
            sqlx::query("UPDATE game_guild_operations SET target=actor WHERE action=12")
                .execute(&h.pool)
                .await
                .is_err()
        );
        assert!(sqlx::query("UPDATE game_guild_event_outbox SET payload=convert_to('{}','UTF8'),payload_sha256=sha256(convert_to('{}','UTF8'))").execute(&h.pool).await.is_err());
        assert_eq!(text_snapshot(h).await?, committed);
        assert_eq!(h.count("game_guild_operations").await?, 1);
        assert_eq!(h.count("game_guild_event_outbox").await?, 1);
        Ok(())
    })
}
