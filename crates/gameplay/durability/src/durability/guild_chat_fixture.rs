//! Shared historical Guild seed used by the existing Guild tests and real chat dispatch tests.
//! Moved from the Guild owner's fixture; no schema, authority or harness is duplicated.
#![allow(clippy::unwrap_used)]
use crate::domain::{CharacterId, guild::GuildId};
type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
pub fn sid(number: u32, kind: u8) -> [u8; 16] {
    let mut bytes = [0_u8; 16];
    bytes[0] = 0xa1;
    bytes[1] = kind;
    bytes[6] = 0x70;
    bytes[8] = 0x80;
    bytes[12..].copy_from_slice(&number.to_be_bytes());
    bytes
}
pub fn uuid(bytes: [u8; 16]) -> String {
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
pub async fn historical_character(
    pool: &sqlx::PgPool,
    n: u32,
    character: CharacterId,
    owner: [u8; 16],
    world: [u8; 16],
    name: &str,
) -> TestResult {
    sqlx::query("INSERT INTO game_character_account_guards VALUES(encode($1,'hex')::uuid) ON CONFLICT DO NOTHING")
        .bind(owner.as_slice())
        .execute(pool)
        .await?;
    // Match the v3 Male intent below: NULL sex belongs only to historical tag-2 receipts.
    sqlx::query("INSERT INTO game_character_roots(character_id,account_id,world_id,lifecycle,character_revision,profile_revision,ruleset_revision,content_revision,starter_template_revision,name,sex) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,1,1,'profile-1','ruleset-1','content-1','starter-1',$4,2)")
        .bind(character.as_bytes().as_slice()).bind(owner.as_slice()).bind(world.as_slice()).bind(name).execute(pool).await?;
    let operation = sid(n, 30);
    let decision = sid(n, 31);
    let raw = serde_json::to_vec(
        &serde_json::json!({"contract_version":3,"variant":"OPERATOR_CONTROL_PLANE_BOOTSTRAP",
        "issuer_authority":"OTERYN_PLATFORM_CHARACTER_AUTHORITY","issuer_decision_id":uuid(decision),
        "source_revision":(u64::from(n)+100).to_string(),"operation_id":uuid(operation),"operation":"INITIAL_CHARACTER_BOOTSTRAP",
        "account_id":uuid(owner),"target_world_id":uuid(world),"requested_sex":"Male","requested_name":name,
        "interpretation_context":{"profile_revision":"profile-1","ruleset_revision":"ruleset-1","content_revision":"content-1","starter_template_revision":"starter-1"},
        "issued_at_source":"1","expires_at_source":"120","audience":"OTERYN_GAME_CHARACTER_AUTHORITY"}),
    )?;
    let intent = crate::character_bootstrap_intent::decode_producer_response(&raw, operation)
        .map_err(|error| format!("{error:?}"))?;
    sqlx::query("INSERT INTO game_character_operation_receipts(operation_id,command_binding,account_id,character_id,world_id,character_revision,event_id,occurred_at,server_build_id,transaction_id,issuer_decision_id,intent_source_revision,issued_at_source,expires_at_source) VALUES(encode($1,'hex')::uuid,$2,encode($3,'hex')::uuid,encode($4,'hex')::uuid,encode($5,'hex')::uuid,1,encode($6,'hex')::uuid,1,'test/1',encode($7,'hex')::uuid,encode($8,'hex')::uuid,$9,1,120)")
        .bind(operation.as_slice()).bind(intent.binding()).bind(owner.as_slice()).bind(character.as_bytes().as_slice()).bind(world.as_slice())
        .bind(sid(n,32).as_slice()).bind(sid(n,33).as_slice()).bind(decision.as_slice()).bind(i64::from(n)+100).execute(pool).await?;
    sqlx::query("INSERT INTO game_character_bootstrap_intent_floors(issuer_scope,source_revision,issuer_decision_id,intent_binding) SELECT 1,intent_source_revision,issuer_decision_id,command_binding FROM game_character_operation_receipts ORDER BY intent_source_revision DESC LIMIT 1 ON CONFLICT(issuer_scope) DO UPDATE SET source_revision=EXCLUDED.source_revision,issuer_decision_id=EXCLUDED.issuer_decision_id,intent_binding=EXCLUDED.intent_binding")
        .execute(pool).await?;
    Ok(())
}
pub async fn initial_guild(
    pool: &sqlx::PgPool,
    guild: GuildId,
    world: [u8; 16],
    name: &str,
    leader: CharacterId,
    account: [u8; 16],
) -> TestResult {
    sqlx::query("INSERT INTO game_guilds(guild_id,world_id,name,state,formation_deadline,vice_deficit_since,revision,created_at) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,2,9999999999999,1,1,1)")
        .bind(guild.as_bytes().as_slice()).bind(world.as_slice()).bind(name).execute(pool).await?;
    for (rank, name) in [
        (1_i16, "Leader"),
        (2, "Vice Leader"),
        (3, "Member"),
        (4, "Visitor"),
    ] {
        sqlx::query("INSERT INTO game_guild_ranks(guild_id,level,name) VALUES(encode($1,'hex')::uuid,$2,$3)")
            .bind(guild.as_bytes().as_slice()).bind(rank).bind(name).execute(pool).await?;
    }
    initial_member(pool, guild, leader, account, 1).await
}
pub async fn initial_member(
    pool: &sqlx::PgPool,
    guild: GuildId,
    character: CharacterId,
    account: [u8; 16],
    rank: i16,
) -> TestResult {
    sqlx::query("INSERT INTO game_guild_members(character_id,guild_id,rank_level,title,joined_at) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,'',1)")
        .bind(character.as_bytes().as_slice()).bind(guild.as_bytes().as_slice()).bind(rank).execute(pool).await?;
    if rank <= 2 {
        sqlx::query("INSERT INTO game_account_guild_leadership VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid)")
        .bind(account.as_slice()).bind(guild.as_bytes().as_slice()).bind(character.as_bytes().as_slice()).execute(pool).await?;
    }
    Ok(())
}

mod schema;
pub use schema::{GUILD_GUARDS_PROPOSAL, GUILD_TABLES_PROPOSAL, guard, prepare};

pub async fn historical_session(
    pool: &sqlx::PgPool,
    n: u32,
    base: super::item_transfer::CurrentCharacterItemFence,
) -> TestResult<super::item_transfer::CurrentCharacterItemFence> {
    let crate::foundation::RuntimeScopeRefV1::Channel { channel_id, .. } = base.runtime_scope
    else {
        return Err("expected Channel".into());
    };
    let session = sid(n, 20);
    sqlx::query("INSERT INTO game_durability_reconnect_sessions(game_session_id,account_id,character_id,world_id,runtime_scope_kind,runtime_scope_world_id,runtime_scope_channel_id,control_loss_epoch,original_grace_deadline,predecessor_generation,character_lease_generation,scope_ownership_generation,current_generation,current_transport_ref,session_state) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,1,encode($4,'hex')::uuid,encode($5,'hex')::uuid,1,999999,1,1,1,1,$6,1)")
        .bind(session.as_slice()).bind(sid(n,2).as_slice()).bind(CharacterId::from_bytes(sid(n,1))?.as_bytes().as_slice()).bind(base.runtime_scope.world_id().as_bytes().as_slice()).bind(channel_id.as_bytes().as_slice()).bind(sid(n,21).as_slice()).execute(pool).await?;
    sqlx::query("INSERT INTO game_durability_admission_account_guards VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,1,'test',1,'account-current',1,0,'{}')")
        .bind(sid(n,2).as_slice()).bind(CharacterId::from_bytes(sid(n,1))?.as_bytes().as_slice()).bind(session.as_slice()).execute(pool).await?;
    sqlx::query("INSERT INTO game_durability_admission_character_guards VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,true,1,encode($4,'hex')::uuid,1,'test',1,'character-current',1,0,'{}')")
        .bind(CharacterId::from_bytes(sid(n,1))?.as_bytes().as_slice()).bind(sid(n,2).as_slice()).bind(base.runtime_scope.world_id().as_bytes().as_slice()).bind(session.as_slice()).execute(pool).await?;
    Ok(super::item_transfer::CurrentCharacterItemFence {
        character_id: CharacterId::from_bytes(sid(n, 1))?,
        game_session_id: crate::foundation::GameSessionId::decode(&session)?,
        ..base
    })
}

use super::guild::{GuildCommand, GuildOperationIdentity, GuildRequest};
use crate::foundation::{CommandId, CommandRef};
pub fn request(
    n: u32,
    guild: GuildId,
    fence: super::item_transfer::CurrentCharacterItemFence,
    op: GuildCommand,
) -> TestResult<GuildRequest> {
    Ok(GuildRequest {
        identity: GuildOperationIdentity {
            wire_digest: None,
            occurrence: sid(n, 10),
            transaction_id: sid(n, 11),
            event_id: sid(n, 12),
            occurred_at_unix_ms: 1_790_000_000_000,
        },
        command: CommandRef::new(fence.game_session_id, CommandId::new(u64::from(n))?),
        guild_id: guild,
        operation: op,
    })
}
