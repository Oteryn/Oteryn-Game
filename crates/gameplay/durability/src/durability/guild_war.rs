use crate::durability::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use crate::durability::character_progression::{assert_session_fence, uuid_text};
use crate::durability::db::{begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations};
use crate::durability::guild::GuildError;
use crate::durability::item_transfer::CurrentCharacterItemFence;
use crate::durability::runtime_scope_assignment::NodeIncarnationProof;
use crate::durability::DurabilityRoot;
use crate::foundation::CommandRef;
use oteryn_gameplay_social_rules::guild::{guild_war_end, validate_guild_war_terms, GuildWarEnd};
use oteryn_protocol_oteryn::guild_war::{GuildWarIntent, GuildWarResult};
use sqlx::Row;

const GUILD_WAR_PASS_MAX: i64 = 100;

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

    /// End at most one bounded page of active wars. The database clock and locked score are the
    /// facts supplied to the shared rule; current recovery and World assignment authorize writes.
    pub async fn finish_due_guild_wars(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        world: crate::foundation::WorldId,
    ) -> Result<usize, GuildError> {
        let recovery = authority.record_for(self).map_err(|_| GuildError::NotCurrent)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            lock_admission_relations(&mut tx).await?;
            if !crate::durability::runtime_scope_assignment::prove_current_incarnation(&mut tx, &node).await? {
                return Err(crate::durability::DurabilityError::Unavailable);
            }
            super::guild::assert_guild_job_world(&mut tx, &node, world)
                .await
                .map_err(|_| crate::durability::DurabilityError::Unavailable)?;
            let rows = sqlx::query("SELECT war_id::text,frag_limit,declaring_frags,opposing_frags,ends_at,floor(extract(epoch FROM transaction_timestamp())*1000)::bigint AS now FROM game_guild_wars WHERE world_id=encode($1,'hex')::uuid AND state=2 ORDER BY war_id LIMIT $2 FOR UPDATE")
                .bind(world.as_bytes().as_slice()).bind(GUILD_WAR_PASS_MAX).fetch_all(&mut *tx).await?;
            let mut ended = 0usize;
            for row in rows {
                let limit = u16::try_from(row.try_get::<i32, _>("frag_limit")?).map_err(|_| crate::durability::DurabilityError::InvalidStoredState)?;
                let declaring = u16::try_from(row.try_get::<i32, _>("declaring_frags")?).map_err(|_| crate::durability::DurabilityError::InvalidStoredState)?;
                let opposing = u16::try_from(row.try_get::<i32, _>("opposing_frags")?).map_err(|_| crate::durability::DurabilityError::InvalidStoredState)?;
                let Some(reason) = guild_war_end(limit, [declaring, opposing], row.try_get("ends_at")?, row.try_get("now")?) else { continue };
                let war: String = row.try_get("war_id")?;
                sqlx::query("UPDATE game_guild_wars SET state=3,ended_reason=$2 WHERE war_id=$1::uuid AND state=2")
                    .bind(war).bind(match reason { GuildWarEnd::FragLimit => 1i16, GuildWarEnd::Duration => 2i16 }).execute(&mut *tx).await?;
                ended += 1;
            }
            commit_semantic_transaction(tx, deadline).await?;
            Ok(ended)
        })).await.map_err(GuildError::from)
    }
}

/// Attribute the last damaging player from the fenced PvP ledger to the one active war between
/// the killer's and victim's current guilds. The death occurrence makes replay exactly-once.
pub(crate) async fn attribute_guild_war_frag_in_transaction(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    victim: crate::domain::CharacterId,
    world: crate::foundation::WorldId,
    occurrence: &[u8; 16],
    contributors: &[crate::durability::pvp_death_request::PvpDeathContributor],
) -> Result<(), crate::durability::DurabilityError> {
    if contributors.is_empty() {
        return Ok(());
    }
    let contributor_ids: Vec<Vec<u8>> = contributors.iter().map(|c| c.character_id.as_bytes().to_vec()).collect();
    let row = sqlx::query("SELECT w.war_id::text,w.declaring_guild::text,kg.guild_id::text AS killer_guild,vg.guild_id::text AS victim_guild,h.attacker::text AS killer FROM game_character_pvp_harm h JOIN game_guild_members km ON km.character_id=h.attacker JOIN game_guilds kg ON kg.guild_id=km.guild_id AND kg.world_id=encode($2,'hex')::uuid JOIN game_guild_members vm ON vm.character_id=encode($1,'hex')::uuid JOIN game_guilds vg ON vg.guild_id=vm.guild_id AND vg.world_id=kg.world_id JOIN game_guild_wars w ON w.world_id=kg.world_id AND w.state=2 AND ((w.declaring_guild=kg.guild_id AND w.opposing_guild=vg.guild_id) OR (w.opposing_guild=kg.guild_id AND w.declaring_guild=vg.guild_id)) WHERE h.victim=encode($1,'hex')::uuid AND h.attacker=ANY(SELECT encode(x,'hex')::uuid FROM unnest($3::bytea[]) x) AND h.damage>0 AND h.created_at>=clock_timestamp()-interval '5 minutes' ORDER BY h.created_at DESC LIMIT 1 FOR UPDATE OF w")
        .bind(victim.as_bytes().as_slice()).bind(world.as_bytes().as_slice()).bind(contributor_ids).fetch_optional(&mut **tx).await?;
    let Some(row) = row else { return Ok(()) };
    let war: String = row.try_get("war_id")?;
    let declaring: String = row.try_get("declaring_guild")?;
    let killer_guild: String = row.try_get("killer_guild")?;
    let victim_guild: String = row.try_get("victim_guild")?;
    let killer: String = row.try_get("killer")?;
    let inserted = sqlx::query("INSERT INTO game_guild_war_frags(death_occurrence_id,war_id,killer_character_id,victim_character_id,killer_guild,victim_guild,occurred_at) VALUES(encode($1,'hex')::uuid,$2::uuid,$3::uuid,encode($4,'hex')::uuid,$5::uuid,$6::uuid,floor(extract(epoch FROM transaction_timestamp())*1000)::bigint) ON CONFLICT(death_occurrence_id) DO NOTHING")
        .bind(occurrence.as_slice()).bind(&war).bind(killer).bind(victim.as_bytes().as_slice()).bind(&killer_guild).bind(victim_guild).execute(&mut **tx).await?.rows_affected();
    if inserted == 1 {
        let declaring_kill = killer_guild == declaring;
        if sqlx::query("UPDATE game_guild_wars SET declaring_frags=declaring_frags+CASE WHEN $2 THEN 1 ELSE 0 END,opposing_frags=opposing_frags+CASE WHEN $2 THEN 0 ELSE 1 END WHERE war_id=$1::uuid AND state=2")
            .bind(war).bind(declaring_kill).execute(&mut **tx).await?.rows_affected() != 1 {
            return Err(crate::durability::DurabilityError::InvalidStoredState);
        }
    }
    Ok(())
}

async fn leader_guild(tx:&mut sqlx::Transaction<'_,sqlx::Postgres>,actor:&[u8;16],world:&[u8;16])->Result<[u8;16],GuildError>{let row=sqlx::query("SELECT g.guild_id::text FROM game_guild_members m JOIN game_guilds g USING(guild_id) WHERE m.character_id=encode($1,'hex')::uuid AND m.rank_level=1 AND g.world_id=encode($2,'hex')::uuid AND g.state IN(1,2) FOR UPDATE OF g").bind(actor.as_slice()).bind(world.as_slice()).fetch_optional(&mut **tx).await?.ok_or(GuildError::InvalidInput)?;let text:String=row.try_get(0)?;uuid_text(&text).map_err(|_|GuildError::InvalidInput)}
