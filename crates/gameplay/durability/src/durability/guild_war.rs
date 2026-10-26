use crate::durability::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use crate::durability::character_progression::{assert_session_fence, uuid_text};
use crate::durability::db::{begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations};
use crate::durability::guild::GuildError;
use crate::durability::item_transfer::CurrentCharacterItemFence;
use crate::durability::runtime_scope_assignment::NodeIncarnationProof;
use crate::durability::DurabilityRoot;
use crate::foundation::CommandRef;
use oteryn_gameplay_social_rules::guild::validate_guild_war_terms;
use oteryn_protocol_oteryn::guild_war::{GuildWarIntent, GuildWarResult};
use sqlx::Row;

impl DurabilityRoot {
    #[allow(
        clippy::too_many_arguments,
        reason = "independent authority, replay, wire, identity and database-time trust inputs"
    )]
    pub async fn commit_guild_war(&self, authority: &ReconciledCharacterAuthority<'_, '_>, node: &NodeIncarnationProof, fence: CurrentCharacterItemFence, command: CommandRef, digest: [u8; 32], intent: GuildWarIntent, war_id: [u8; 16], now: i64) -> Result<GuildWarResult, GuildError> {
        let recovery = authority.record_for(self).map_err(|_| GuildError::NotCurrent)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            let result: Result<GuildWarResult, GuildError> = async {
                assert_recovery_fence(&mut tx, &recovery).await?;
                lock_admission_relations(&mut tx).await?;
                if assert_session_fence(&mut tx, &fence, &node).await?.is_err() { return Err(GuildError::NotCurrent); }
                let session_id = command.game_session_id();
                let session = session_id.as_bytes().as_slice();
                let cid = i64::try_from(command.command_id().get()).map_err(|_| GuildError::InvalidInput)?;
                if let Some(row) = sqlx::query("SELECT war_id::text AS war,result,wire_sha256 FROM game_guild_war_commands WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2").bind(session).bind(cid).fetch_optional(&mut *tx).await? {
                    if row.try_get::<Vec<u8>, _>("wire_sha256")?.as_slice() != digest { return Err(GuildError::ConflictingOccurrence); }
                    let text: String = row.try_get("war")?;
                    return Ok(GuildWarResult { code: u8::try_from(row.try_get::<i16, _>("result")?).map_err(|_| GuildError::InvalidInput)?, war: uuid_text(&text).map_err(|_| GuildError::InvalidInput)? });
                }
                let actor = *fence.character_id.as_bytes();
                let world_id = fence.runtime_scope.world_id();
                let world = world_id.as_bytes();
                let actual = match intent {
                    GuildWarIntent::Declare { opponent, frag_limit, duration_days } => {
                        validate_guild_war_terms(frag_limit, duration_days).map_err(|_| GuildError::InvalidInput)?;
                        let actor_guild = leader_guild(&mut tx, &actor, world).await?;
                        if actor_guild == opponent { return Err(GuildError::InvalidInput); }
                        let inserted = sqlx::query("INSERT INTO game_guild_wars(war_id,world_id,declaring_guild,opposing_guild,state,frag_limit,duration_days,declared_at) SELECT encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,guild_id,1,$5,$6,$7 FROM game_guilds WHERE guild_id=encode($4,'hex')::uuid AND world_id=encode($2,'hex')::uuid AND state IN(1,2)").bind(war_id.as_slice()).bind(world.as_slice()).bind(actor_guild.as_slice()).bind(opponent.as_slice()).bind(i32::from(frag_limit)).bind(i32::from(duration_days)).bind(now).execute(&mut *tx).await?.rows_affected();
                        if inserted != 1 { return Err(GuildError::InvalidInput); }
                        war_id
                    }
                    GuildWarIntent::Accept { war } => {
                        let actor_guild = leader_guild(&mut tx, &actor, world).await?;
                        let changed = sqlx::query("UPDATE game_guild_wars SET state=2,started_at=$1,ends_at=$1+duration_days::bigint*86400000 WHERE war_id=encode($2,'hex')::uuid AND world_id=encode($3,'hex')::uuid AND opposing_guild=encode($4,'hex')::uuid AND state=1").bind(now).bind(war.as_slice()).bind(world.as_slice()).bind(actor_guild.as_slice()).execute(&mut *tx).await?.rows_affected();
                        if changed != 1 { return Err(GuildError::InvalidInput); }
                        war
                    }
                };
                sqlx::query("INSERT INTO game_guild_war_commands VALUES(encode($1,'hex')::uuid,$2,$3,encode($4,'hex')::uuid,1)").bind(session).bind(cid).bind(digest.as_slice()).bind(actual.as_slice()).execute(&mut *tx).await?;
                Ok(GuildWarResult { code: 1, war: actual })
            }.await;
            match result { Ok(value) => { commit_semantic_transaction(tx, deadline).await?; Ok(Ok(value)) }, Err(GuildError::Unavailable(error)) => Err(error), Err(error) => Ok(Err(error)) }
        })).await?
    }
}

async fn leader_guild(tx:&mut sqlx::Transaction<'_,sqlx::Postgres>,actor:&[u8;16],world:&[u8;16])->Result<[u8;16],GuildError>{let row=sqlx::query("SELECT g.guild_id::text FROM game_guild_members m JOIN game_guilds g USING(guild_id) WHERE m.character_id=encode($1,'hex')::uuid AND m.rank_level=1 AND g.world_id=encode($2,'hex')::uuid AND g.state IN(1,2) FOR UPDATE OF g").bind(actor.as_slice()).bind(world.as_slice()).fetch_optional(&mut **tx).await?.ok_or(GuildError::InvalidInput)?;let text:String=row.try_get(0)?;uuid_text(&text).map_err(|_|GuildError::InvalidInput)}
