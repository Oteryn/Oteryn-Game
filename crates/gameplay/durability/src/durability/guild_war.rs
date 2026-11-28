use crate::durability::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use crate::durability::character_progression::{assert_session_fence, uuid_text};
use crate::durability::db::{
    begin_semantic_transaction, commit_semantic_read, commit_semantic_transaction,
    lock_admission_relations,
};
use crate::durability::guild::GuildError;
use crate::durability::item_transfer::CurrentCharacterItemFence;
use crate::durability::runtime_scope_assignment::NodeIncarnationProof;
use crate::durability::{DurabilityError, DurabilityRoot};
use crate::foundation::CommandRef;
use oteryn_gameplay_social_rules::guild::{GuildWarEnd, guild_war_end, validate_guild_war_terms};
use oteryn_protocol_oteryn::guild_war::{GuildWarIntent, GuildWarResult};
use sqlx::{Acquire, Row};

const GUILD_WAR_PASS_MAX: i64 = 100;

// Use the transaction's decision time and the locked score wherever a war confers privileges.
macro_rules! active_war_predicate {
    () => {
        "w.state=2 AND w.ends_at>floor(extract(epoch FROM transaction_timestamp())*1000)::bigint \
         AND w.declaring_frags<w.frag_limit AND w.opposing_frags<w.frag_limit"
    };
}
pub(super) use active_war_predicate;

/// A connection-local delivery candidate. Only the fenced recipient may mark it written.
pub struct GuildWarNotice {
    pub id: i64,
    pub guild: [u8; 16],
    pub message: String,
}

const NOTICE_RECIPIENTS: &str = "WITH memberships AS (
    SELECT guild_id, joined_at FROM game_guild_members WHERE character_id=encode($1,'hex')::uuid
    UNION ALL
    SELECT guild_id, joined_at FROM game_guild_war_disband_members WHERE character_id=encode($1,'hex')::uuid
), eligible AS (
    SELECT n.notice_id,n.guild_id,n.message FROM memberships m
    JOIN game_guilds g USING(guild_id)
    JOIN game_guild_war_notices n ON n.guild_id=g.guild_id AND n.occurred_at>=m.joined_at
    LEFT JOIN game_guild_war_notice_cursors c ON c.guild_id=g.guild_id AND c.character_id=encode($1,'hex')::uuid
    WHERE g.world_id=encode($2,'hex')::uuid AND n.notice_id>coalesce(c.notice_id,0)
    ORDER BY n.notice_id LIMIT 1
) ";

impl DurabilityRoot {
    #[allow(
        clippy::too_many_arguments,
        reason = "independent authority, replay, wire, identity and database-time trust inputs"
    )]
    pub async fn commit_guild_war(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        command: CommandRef,
        digest: [u8; 32],
        intent: GuildWarIntent,
        war_id: [u8; 16],
        now: i64,
    ) -> Result<GuildWarResult, GuildError> {
        if command.game_session_id() != fence.game_session_id {
            return Err(GuildError::NotCurrent);
        }
        let recovery = authority
            .record_for(self)
            .map_err(|_| GuildError::NotCurrent)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            let result: Result<GuildWarResult, GuildError> = async {
                assert_recovery_fence(&mut tx, &recovery).await?;
                lock_admission_relations(&mut tx).await?;
                if assert_session_fence(&mut tx, &fence, &node).await?.is_err() { return Err(GuildError::NotCurrent); }
                let world_id = fence.runtime_scope.world_id();
                // Both concurrent requests and a later replay observe the winning receipt.
                lock_war_world(&mut tx, world_id).await?;
                let session_id = command.game_session_id();
                let session = session_id.as_bytes().as_slice();
                let cid = i64::try_from(command.command_id().get()).map_err(|_| GuildError::InvalidInput)?;
                if let Some(row) = sqlx::query("SELECT war_id::text AS war,result,wire_sha256 FROM game_guild_war_commands WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2").bind(session).bind(cid).fetch_optional(&mut *tx).await? {
                    if row.try_get::<Vec<u8>, _>("wire_sha256")?.as_slice() != digest { return Err(GuildError::ConflictingOccurrence); }
                    let text: String = row.try_get("war")?;
                    return Ok(GuildWarResult { code: u8::try_from(row.try_get::<i16, _>("result")?).map_err(|_| GuildError::InvalidInput)?, war: uuid_text(&text).map_err(|_| GuildError::InvalidInput)? });
                }
                let actor = *fence.character_id.as_bytes();
                let world = world_id.as_bytes();
                let stake = intent.stake().map(i64::from);
                let actual = match intent {
                    GuildWarIntent::Declare { opponent, frag_limit, duration_days }
                    | GuildWarIntent::DeclareWithStake { opponent, frag_limit, duration_days, .. } => {
                        validate_guild_war_terms(frag_limit, duration_days).map_err(|_| GuildError::InvalidInput)?;
                        let actor_guild = leader_guild(&mut tx, &actor, world).await?;
                        if actor_guild == opponent { return Err(GuildError::InvalidInput); }
                        let inserted = sqlx::query("INSERT INTO game_guild_wars(war_id,world_id,declaring_guild,opposing_guild,state,frag_limit,duration_days,declared_at,stake_gold,stake_declarer) SELECT encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,guild_id,1,$5,$6,$7,$8,encode($9,'hex')::uuid FROM game_guilds WHERE guild_id=encode($4,'hex')::uuid AND world_id=encode($2,'hex')::uuid AND state IN(1,2)").bind(war_id.as_slice()).bind(world.as_slice()).bind(actor_guild.as_slice()).bind(opponent.as_slice()).bind(i32::from(frag_limit)).bind(i32::from(duration_days)).bind(now).bind(stake).bind(stake.map(|_| actor.as_slice())).execute(&mut *tx).await?.rows_affected();
                        if inserted != 1 { return Err(GuildError::InvalidInput); }
                        war_id
                    }
                    GuildWarIntent::Accept { war } | GuildWarIntent::AcceptWithStake { war, .. } => {
                        let actor_guild = leader_guild(&mut tx, &actor, world).await?;
                        let matched: Option<i32> = sqlx::query_scalar(
                            "SELECT 1 FROM game_guild_wars WHERE war_id=encode($1,'hex')::uuid
                             AND world_id=encode($2,'hex')::uuid AND opposing_guild=encode($3,'hex')::uuid
                             AND state=1 AND stake_gold IS NOT DISTINCT FROM $4 FOR UPDATE"
                        ).bind(war.as_slice()).bind(world.as_slice()).bind(actor_guild.as_slice())
                            .bind(stake).fetch_optional(&mut *tx).await?;
                        if matched.is_none() { return Err(GuildError::InvalidInput); }
                        if stake.is_some() {
                            let funded: bool = sqlx::query_scalar("SELECT game_guild_war_fund(
                                encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4)")
                                .bind(war.as_slice()).bind(actor.as_slice()).bind(session).bind(cid)
                                .fetch_one(&mut *tx).await?;
                            if !funded { return Ok(GuildWarResult { code: 2, war }); }
                        }
                        sqlx::query("UPDATE game_guild_wars SET state=2,started_at=$1,ends_at=$1+duration_days::bigint*86400000 WHERE war_id=encode($2,'hex')::uuid")
                            .bind(now).bind(war.as_slice()).execute(&mut *tx).await?;
                        war
                    }
                };
                sqlx::query("INSERT INTO game_guild_war_commands(game_session_id,command_id,wire_sha256,war_id,result) VALUES(encode($1,'hex')::uuid,$2,$3,encode($4,'hex')::uuid,1)").bind(session).bind(cid).bind(digest.as_slice()).bind(actual.as_slice()).execute(&mut *tx).await?;
                Ok(GuildWarResult { code: 1, war: actual })
            }.await;
            match result { Ok(value) => { commit_semantic_transaction(tx, deadline).await?; Ok(Ok(value)) }, Err(GuildError::Unavailable(error)) => Err(error), Err(error) => Ok(Err(error)) }
        })).await?
    }

    /// Read the next notice or acknowledge that exact notice after a successful frame write.
    pub async fn guild_war_notice(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        written: Option<i64>,
    ) -> Result<Option<GuildWarNotice>, GuildError> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| GuildError::NotCurrent)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            lock_admission_relations(&mut tx).await?;
            if assert_session_fence(&mut tx, &fence, &node).await?.is_err() {
                return Ok(Err(GuildError::NotCurrent));
            }
            let character = fence.character_id.as_bytes();
            let world = fence.runtime_scope.world_id();
            let world = world.as_bytes();
            if let Some(notice) = written {
                let query = format!("{NOTICE_RECIPIENTS}
                    INSERT INTO game_guild_war_notice_cursors(character_id,guild_id,notice_id)
                    SELECT encode($1,'hex')::uuid,guild_id,notice_id FROM eligible WHERE notice_id=$3
                    ON CONFLICT (character_id,guild_id) DO UPDATE SET notice_id=excluded.notice_id
                    WHERE game_guild_war_notice_cursors.notice_id<excluded.notice_id");
                sqlx::query(sqlx::AssertSqlSafe(query))
                    .bind(character.as_slice()).bind(world.as_slice()).bind(notice)
                    .execute(&mut *tx).await?;
                commit_semantic_transaction(tx, deadline).await?;
                return Ok(Ok(None));
            }
            let query = format!("{NOTICE_RECIPIENTS} SELECT notice_id,guild_id::text,message FROM eligible");
            let row: Option<(i64, String, String)> = sqlx::query_as(sqlx::AssertSqlSafe(query))
                .bind(character.as_slice()).bind(world.as_slice())
                .fetch_optional(&mut *tx).await?;
            commit_semantic_read(tx, deadline).await?;
            Ok(Ok(row.map(|(id, guild, message)| {
                Ok::<_, DurabilityError>(GuildWarNotice { id, guild: uuid_text(&guild)?, message })
            }).transpose()?))
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
        let recovery = authority
            .record_for(self)
            .map_err(|_| GuildError::NotCurrent)?;
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
            lock_war_world(&mut tx, world).await?;
            let rows = sqlx::query(
                "SELECT w.war_id::text,w.frag_limit,w.declaring_frags,w.opposing_frags,w.ends_at,w.stake_gold,
                        w.declaring_guild::text,w.opposing_guild::text,d.name AS declaring_name,o.name AS opposing_name,
                        d.state>=3 OR o.state>=3 OR w.state=1 OR closure.pending AS cancelled,
                        floor(extract(epoch FROM transaction_timestamp())*1000)::bigint AS now
                   FROM game_guild_wars w
                   JOIN game_guilds d ON d.guild_id=w.declaring_guild
                   JOIN game_guilds o ON o.guild_id=w.opposing_guild
                   CROSS JOIN LATERAL (SELECT EXISTS(
                     SELECT 1 FROM game_guild_war_escrow e WHERE e.war_id=w.war_id
                     AND e.settled_xact IS NULL AND game_account_closure_closed(e.account_id)
                   ) AS pending) closure
                  WHERE w.world_id=encode($1,'hex')::uuid AND (
                      (w.state=2 AND (w.ends_at<=floor(extract(epoch FROM transaction_timestamp())*1000)::bigint
                       OR greatest(w.declaring_frags,w.opposing_frags)>=w.frag_limit OR d.state>=3 OR o.state>=3 OR closure.pending))
                      OR (w.state=1 AND w.stake_gold IS NOT NULL AND (d.state>=3 OR o.state>=3
                          OR NOT EXISTS (SELECT 1 FROM game_guild_members m WHERE m.character_id=w.stake_declarer
                              AND m.guild_id=w.declaring_guild AND m.rank_level=1))))
                  ORDER BY w.war_id LIMIT $2 FOR UPDATE OF w"
            ).bind(world.as_bytes().as_slice()).bind(GUILD_WAR_PASS_MAX).fetch_all(&mut *tx).await?;
            if !rows.is_empty() {
                let mut guilds = Vec::with_capacity(rows.len() * 2);
                for row in &rows {
                    guilds.push(row.try_get::<String, _>("declaring_guild")?);
                    guilds.push(row.try_get::<String, _>("opposing_guild")?);
                }
                // Guild disband takes its guild before touching any bank account.
                sqlx::query("SELECT guild_id FROM game_guilds
                    WHERE guild_id IN (SELECT g::uuid FROM unnest($1::text[]) g)
                    ORDER BY guild_id FOR KEY SHARE")
                    .bind(guilds).fetch_all(&mut *tx).await?;
            }
            let mut finished = 0;
            for row in &rows {
                let mut war = tx.begin().await?;
                // Check deferred bank and custody proofs inside this war's boundary.
                sqlx::query("SET CONSTRAINTS ALL DEFERRED").execute(&mut *war).await?;
                let result = async {
                    finish_one(&mut war, row).await?;
                    sqlx::query("SET CONSTRAINTS ALL IMMEDIATE").execute(&mut *war).await?;
                    Ok::<_, DurabilityError>(())
                }.await;
                match result {
                    Ok(()) => {
                        war.commit().await?;
                        finished += 1;
                    }
                    Err(DurabilityError::Database(_) | DurabilityError::InvalidStoredState) => war.rollback().await?,
                    Err(error) => return Err(error),
                }
            }
            commit_semantic_transaction(tx, deadline).await?;
            Ok(finished)
        })).await.map_err(GuildError::from)
    }
}

// Serialize World war transitions. The order also keeps admission, war and notice writes
// stable across Channels when the World authority moves.
async fn lock_war_world(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    world: crate::foundation::WorldId,
) -> Result<(), DurabilityError> {
    sqlx::query(
        "SELECT pg_advisory_xact_lock(hashtextextended('guild-war:' || encode($1,'hex'), 0))",
    )
    .bind(world.as_bytes().as_slice())
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn finish_one(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    row: &sqlx::postgres::PgRow,
) -> Result<(), DurabilityError> {
    let score = |column| {
        u16::try_from(row.try_get::<i32, _>(column)?)
            .map_err(|_| DurabilityError::InvalidStoredState)
    };
    let limit = score("frag_limit")?;
    let declaring = score("declaring_frags")?;
    let opposing = score("opposing_frags")?;
    let now: i64 = row.try_get("now")?;
    let ends_at: Option<i64> = row.try_get("ends_at")?;
    let scored_end = ends_at.and_then(|end| guild_war_end(limit, [declaring, opposing], end, now));
    let reason: i16 = match scored_end {
        Some(GuildWarEnd::FragLimit) => 1,
        Some(GuildWarEnd::Duration) => 2,
        None if row.try_get::<bool, _>("cancelled")? => 3,
        None => return Err(DurabilityError::InvalidStoredState),
    };
    let war: String = row.try_get("war_id")?;
    sqlx::query("UPDATE game_guild_wars SET state=3,ended_reason=$2 WHERE war_id=$1::text::uuid")
        .bind(&war)
        .bind(reason)
        .execute(&mut **tx)
        .await?;
    if ends_at.is_some() && row.try_get::<Option<i64>, _>("stake_gold")?.is_some() {
        sqlx::query("SELECT game_guild_war_settle($1::text::uuid)")
            .bind(&war)
            .execute(&mut **tx)
            .await?;
    }

    for (guild_column, opponent_column, score, other_score) in [
        ("declaring_guild", "opposing_name", declaring, opposing),
        ("opposing_guild", "declaring_name", opposing, declaring),
    ] {
        use std::cmp::Ordering;
        let opponent: String = row.try_get(opponent_column)?;
        let outcome = match (reason, score.cmp(&other_score)) {
            (3, _) => "was cancelled",
            (_, Ordering::Greater) => "was won",
            (_, Ordering::Less) => "was lost",
            (_, Ordering::Equal) => "ended in a draw",
        };
        let line =
            format!("Your war against {opponent} {outcome}. Final score: {score}-{other_score}.");
        sqlx::query(
            "INSERT INTO game_guild_war_notices(war_id,guild_id,occurred_at,message)
             VALUES($1::text::uuid,$2::text::uuid,$3,$4)",
        )
        .bind(&war)
        .bind(row.try_get::<String, _>(guild_column)?)
        .bind(now)
        .bind(line)
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
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
    let contributor_ids: Vec<Vec<u8>> = contributors
        .iter()
        .map(|c| c.character_id.as_bytes().to_vec())
        .collect();
    let row = sqlx::query(concat!("SELECT w.war_id::text,w.declaring_guild::text,kg.guild_id::text AS killer_guild,vg.guild_id::text AS victim_guild,h.attacker::text AS killer FROM game_character_pvp_harm h JOIN game_guild_members km ON km.character_id=h.attacker JOIN game_guilds kg ON kg.guild_id=km.guild_id AND kg.world_id=encode($2,'hex')::uuid JOIN game_guild_members vm ON vm.character_id=encode($1,'hex')::uuid JOIN game_guilds vg ON vg.guild_id=vm.guild_id AND vg.world_id=kg.world_id JOIN game_guild_wars w ON w.world_id=kg.world_id AND ", active_war_predicate!(), " AND ((w.declaring_guild=kg.guild_id AND w.opposing_guild=vg.guild_id) OR (w.opposing_guild=kg.guild_id AND w.declaring_guild=vg.guild_id)) WHERE h.victim=encode($1,'hex')::uuid AND h.attacker=ANY(SELECT encode(x,'hex')::uuid FROM unnest($3::bytea[]) x) AND h.damage>0 AND h.created_at>=clock_timestamp()-interval '5 minutes' ORDER BY h.created_at DESC LIMIT 1 FOR UPDATE OF w"))
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

async fn leader_guild(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    actor: &[u8; 16],
    world: &[u8; 16],
) -> Result<[u8; 16], GuildError> {
    let row=sqlx::query("SELECT g.guild_id::text FROM game_guild_members m JOIN game_guilds g USING(guild_id) WHERE m.character_id=encode($1,'hex')::uuid AND m.rank_level=1 AND g.world_id=encode($2,'hex')::uuid AND g.state IN(1,2) FOR UPDATE OF g").bind(actor.as_slice()).bind(world.as_slice()).fetch_optional(&mut **tx).await?.ok_or(GuildError::InvalidInput)?;
    let text: String = row.try_get(0)?;
    uuid_text(&text).map_err(|_| GuildError::InvalidInput)
}
