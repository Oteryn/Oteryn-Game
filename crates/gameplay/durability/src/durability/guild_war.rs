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
    pub async fn commit_guild_war(&self, authority: &ReconciledCharacterAuthority<'_, '_>, node: &NodeIncarnationProof, fence: CurrentCharacterItemFence, command: CommandRef, digest: [u8; 32], intent: GuildWarIntent, war_id: [u8; 16], _now: i64) -> Result<GuildWarResult, GuildError> {
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
                let now: i64 = sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp())*1000)::bigint").fetch_one(&mut *tx).await?;
                let actor = *fence.character_id.as_bytes();
                let world_id = fence.runtime_scope.world_id();
                let world = world_id.as_bytes();
                let actual = match intent {
                    GuildWarIntent::Declare { opponent, frag_limit, duration_days } => {
                        validate_guild_war_terms(frag_limit, duration_days).map_err(|_| GuildError::InvalidInput)?;
                        let actor_guild = leader_guild(&mut tx, &actor, world).await?;
                        if actor_guild == opponent { return Err(GuildError::InvalidInput); }
                        require_war_guilds(&mut tx, [actor_guild, opponent], world).await?;
                        require_war_capacity(&mut tx, [actor_guild, opponent], world, false).await?;
                        let inserted = sqlx::query("INSERT INTO game_guild_wars(war_id,world_id,declaring_guild,opposing_guild,state,frag_limit,duration_days,declared_at) SELECT encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,guild_id,1,$5,$6,$7 FROM game_guilds WHERE guild_id=encode($4,'hex')::uuid AND world_id=encode($2,'hex')::uuid AND state=2").bind(war_id.as_slice()).bind(world.as_slice()).bind(actor_guild.as_slice()).bind(opponent.as_slice()).bind(i32::from(frag_limit)).bind(i32::from(duration_days)).bind(now).execute(&mut *tx).await?.rows_affected();
                        if inserted != 1 { return Err(GuildError::InvalidInput); }
                        war_id
                    }
                    GuildWarIntent::Accept { war } => {
                        let actor_guild = leader_guild(&mut tx, &actor, world).await?;
                        let declaring: String = sqlx::query_scalar("SELECT declaring_guild::text FROM game_guild_wars WHERE war_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid AND opposing_guild=encode($3,'hex')::uuid AND state=1 FOR UPDATE")
                            .bind(war.as_slice()).bind(world.as_slice()).bind(actor_guild.as_slice()).fetch_optional(&mut *tx).await?.ok_or(GuildError::InvalidInput)?;
                        let declaring = uuid_text(&declaring).map_err(|_| GuildError::InvalidInput)?;
                        require_war_guilds(&mut tx, [declaring, actor_guild], world).await?;
                        require_war_capacity(&mut tx, [declaring, actor_guild], world, true).await?;
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

    /// End one bounded page of due wars or unanswered declarations. Active wars use the
    /// locked score and database clock; recovery and World assignment authorize writes.
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
            let rows = sqlx::query("SELECT war_id::text,state,frag_limit,declaring_frags,opposing_frags,ends_at,floor(extract(epoch FROM transaction_timestamp())*1000)::bigint AS now FROM game_guild_wars WHERE world_id=encode($1,'hex')::uuid AND state<3 AND ((state=2 AND (declaring_frags>=frag_limit OR opposing_frags>=frag_limit OR ends_at<=floor(extract(epoch FROM transaction_timestamp())*1000)::bigint)) OR (state=1 AND declared_at<=floor(extract(epoch FROM transaction_timestamp())*1000)::bigint-604800000)) ORDER BY war_id LIMIT $2 FOR UPDATE")
                .bind(world.as_bytes().as_slice()).bind(GUILD_WAR_PASS_MAX).fetch_all(&mut *tx).await?;
            let mut ended = 0usize;
            for row in rows {
                let war: String = row.try_get("war_id")?;
                if row.try_get::<i16,_>("state")? == 1 {
                    sqlx::query("UPDATE game_guild_wars SET state=3,ended_reason=NULL WHERE war_id=$1::uuid AND state=1")
                        .bind(war).execute(&mut *tx).await?;
                    ended += 1;
                    continue;
                }
                let limit = u16::try_from(row.try_get::<i32, _>("frag_limit")?).map_err(|_| crate::durability::DurabilityError::InvalidStoredState)?;
                // Clamp only the bounded rule input; retain the full INTEGER scores in storage.
                let declaring = u16::try_from(row.try_get::<i32, _>("declaring_frags")?.min(i32::from(limit))).map_err(|_| crate::durability::DurabilityError::InvalidStoredState)?;
                let opposing = u16::try_from(row.try_get::<i32, _>("opposing_frags")?.min(i32::from(limit))).map_err(|_| crate::durability::DurabilityError::InvalidStoredState)?;
                let Some(reason) = guild_war_end(limit, [declaring, opposing], row.try_get("ends_at")?, row.try_get("now")?) else { continue };
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
    let row = sqlx::query("SELECT w.war_id::text,w.frag_limit,w.declaring_frags,w.opposing_frags,w.ends_at,w.declaring_guild::text,kg.guild_id::text AS killer_guild,vg.guild_id::text AS victim_guild,h.attacker::text AS killer FROM game_character_pvp_harm h JOIN game_guild_members km ON km.character_id=h.attacker JOIN game_guilds kg ON kg.guild_id=km.guild_id AND kg.world_id=encode($2,'hex')::uuid JOIN game_guild_members vm ON vm.character_id=encode($1,'hex')::uuid JOIN game_guilds vg ON vg.guild_id=vm.guild_id AND vg.world_id=kg.world_id JOIN game_guild_wars w ON w.world_id=kg.world_id AND w.state=2 AND ((w.declaring_guild=kg.guild_id AND w.opposing_guild=vg.guild_id) OR (w.opposing_guild=kg.guild_id AND w.declaring_guild=vg.guild_id)) WHERE h.victim=encode($1,'hex')::uuid AND h.attacker=ANY(SELECT encode(x,'hex')::uuid FROM unnest($3::bytea[]) x) AND h.damage>0 AND h.created_at>=clock_timestamp()-interval '5 minutes' ORDER BY h.created_at DESC LIMIT 1 FOR UPDATE OF w")
        .bind(victim.as_bytes().as_slice()).bind(world.as_bytes().as_slice()).bind(contributor_ids).fetch_optional(&mut **tx).await?;
    let Some(row) = row else { return Ok(()) };
    // The locked row decides whether this death still belongs to live combat.
    let now: i64 = sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp())*1000)::bigint")
        .fetch_one(&mut **tx).await?;
    let limit: i32 = row.try_get("frag_limit")?;
    if row.try_get::<i32,_>("declaring_frags")? >= limit
        || row.try_get::<i32,_>("opposing_frags")? >= limit
        || row.try_get::<i64,_>("ends_at")? <= now {
        return Ok(());
    }
    let war: String = row.try_get("war_id")?;
    let recent: i64 = sqlx::query_scalar("SELECT count(*) FROM (SELECT 1 FROM game_guild_war_frags WHERE war_id=$1::uuid AND victim_character_id=encode($2,'hex')::uuid AND occurred_at>$3::bigint-86400000 LIMIT 5) recent")
        .bind(&war).bind(victim.as_bytes().as_slice()).bind(now).fetch_one(&mut **tx).await?;
    if recent >= 5 {
        return Ok(());
    }
    let declaring: String = row.try_get("declaring_guild")?;
    let killer_guild: String = row.try_get("killer_guild")?;
    let victim_guild: String = row.try_get("victim_guild")?;
    let killer: String = row.try_get("killer")?;
    let inserted = sqlx::query("INSERT INTO game_guild_war_frags(death_occurrence_id,war_id,killer_character_id,victim_character_id,killer_guild,victim_guild,occurred_at) VALUES(encode($1,'hex')::uuid,$2::uuid,$3::uuid,encode($4,'hex')::uuid,$5::uuid,$6::uuid,$7) ON CONFLICT(death_occurrence_id) DO NOTHING")
        .bind(occurrence.as_slice()).bind(&war).bind(killer).bind(victim.as_bytes().as_slice()).bind(&killer_guild).bind(victim_guild).bind(now).execute(&mut **tx).await?.rows_affected();
    if inserted == 1 {
        let declaring_kill = killer_guild == declaring;
        if sqlx::query("UPDATE game_guild_wars SET declaring_frags=declaring_frags+CASE WHEN $2 THEN 1 ELSE 0 END,opposing_frags=opposing_frags+CASE WHEN $2 THEN 0 ELSE 1 END WHERE war_id=$1::uuid AND state=2 AND declaring_frags<frag_limit AND opposing_frags<frag_limit AND ends_at>$3")
            .bind(war).bind(declaring_kill).bind(now).execute(&mut **tx).await?.rows_affected() != 1 {
            return Err(crate::durability::DurabilityError::InvalidStoredState);
        }
    }
    Ok(())
}

async fn leader_guild(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>, actor: &[u8;16], world: &[u8;16],
) -> Result<[u8;16], GuildError> {
    let row=sqlx::query("SELECT g.guild_id::text FROM game_guild_members m JOIN game_guilds g USING(guild_id) WHERE m.character_id=encode($1,'hex')::uuid AND m.rank_level=1 AND g.world_id=encode($2,'hex')::uuid AND g.state=2")
        .bind(actor.as_slice()).bind(world.as_slice()).fetch_optional(&mut **tx).await?.ok_or(GuildError::InvalidInput)?;
    let text: String=row.try_get(0)?;
    let guild=uuid_text(&text).map_err(|_| GuildError::InvalidInput)?;
    Ok(guild)
}

async fn require_war_guilds(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>, guilds: [[u8;16];2], world: &[u8;16],
) -> Result<(), GuildError> {
    let [first,second]=guilds;
    // Resolve under admission serialization, then acquire both Guild rows in key order.
    let eligible=sqlx::query("SELECT g.guild_id FROM game_guilds g WHERE g.guild_id IN(encode($1,'hex')::uuid,encode($2,'hex')::uuid) AND g.world_id=encode($3,'hex')::uuid AND g.state=2 AND (SELECT count(*) FROM (SELECT 1 FROM game_guild_members m WHERE m.guild_id=g.guild_id AND m.rank_level=2 LIMIT 4) v)=4 ORDER BY g.guild_id LIMIT 2 FOR UPDATE OF g")
        .bind(first.as_slice()).bind(second.as_slice()).bind(world.as_slice()).fetch_all(&mut **tx).await?;
    if eligible.len()!=2 { return Err(GuildError::InvalidInput); }
    Ok(())
}

async fn require_war_capacity(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>, guilds: [[u8;16];2], world: &[u8;16], accepting: bool,
) -> Result<(), GuildError> {
    // The admission relations and both Guild rows stay locked through the deciding write.
    let maximum = if accepting {5} else {4};
    for guild in guilds {
        let count: i64=sqlx::query_scalar("SELECT count(*) FROM (SELECT 1 FROM game_guild_wars WHERE world_id=encode($1,'hex')::uuid AND state<3 AND (declaring_guild=encode($2,'hex')::uuid OR opposing_guild=encode($2,'hex')::uuid) LIMIT 6) live")
            .bind(world.as_slice()).bind(guild.as_slice()).fetch_one(&mut **tx).await?;
        if count > maximum { return Err(GuildError::InvalidInput); }
    }
    Ok(())
}

#[cfg(test)]
use crate::durability::{guild, item_transfer};
#[cfg(test)]
#[allow(dead_code, unused_imports, reason = "reuse the historical Guild fixture; these cases need only its seed helpers")]
#[path = "guild_chat_fixture.rs"]
mod test_fixture;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bestiary_postgres_harness::{self as harness, Harness, TestResult, debug, id};
    use super::test_fixture as seed;
    use crate::domain::{CharacterId, guild::GuildId};
    use crate::foundation::{CommandId, WorldId};

    fn guild(n: u32) -> TestResult<GuildId> {
        GuildId::from_bytes(seed::sid(n, 3)).map_err(|e| debug(e).into())
    }

    fn run<F>(tag: &'static str, body: F) -> TestResult
    where F: AsyncFnOnce(&Harness, CurrentCharacterItemFence) -> TestResult + Send + 'static {
        let Some(admin) = harness::configured_admin() else { return Ok(()) };
        std::thread::Builder::new().stack_size(64 << 20).spawn(move || {
            harness::runtime().map_err(|e| e.to_string())?.block_on(async move {
                let h = Harness::create(admin, tag, true).await?;
                let result = async {
                    let opponent = fixture(&h).await?;
                    body(&h, opponent).await
                }.await;
                h.cleanup().await?;
                result
            }).map_err(|e| e.to_string())
        })?.join().map_err(|_| "guild war regression panicked")??;
        Ok(())
    }

    async fn fixture(h: &Harness) -> TestResult<CurrentCharacterItemFence> {
        sqlx::raw_sql("DROP TABLE game_guild_war_frags,game_guild_war_commands,game_guild_wars; DROP FUNCTION game_guild_war_rows_immutable()")
            .execute(&h.pool).await?;
        seed::prepare(&h.pool).await?;
        let opponent = CharacterId::from_bytes(seed::sid(90, 1)).map_err(debug)?;
        seed::historical_character(&h.pool, 90, opponent, seed::sid(90, 2), id(harness::WORLD), "Opposing Leader").await?;
        seed::initial_guild(&h.pool, guild(80)?, id(harness::WORLD), "Declaring Guild", CharacterId::from_bytes(id(harness::CHARACTER)).map_err(debug)?, id(40)).await?;
        seed::initial_guild(&h.pool, guild(90)?, id(harness::WORLD), "Opposing Guild", opponent, seed::sid(90, 2)).await?;
        for (g, first) in [(80, 100), (90, 110)] {
            for n in first..first+4 {
                let character = CharacterId::from_bytes(seed::sid(n, 1)).map_err(debug)?;
                let name = format!("Vice {}{}", char::from(b'A'+u8::try_from(n / 26)?), char::from(b'A'+u8::try_from(n % 26)?));
                seed::historical_character(&h.pool, n, character, seed::sid(n, 2), id(harness::WORLD), &name).await?;
                seed::initial_member(&h.pool, guild(g)?, character, seed::sid(n, 2), 2).await?;
            }
        }
        for (n,name) in [(130,"Earlier Recruit"),(131,"New Recruit")] {
            seed::historical_character(&h.pool,n,CharacterId::from_bytes(seed::sid(n,1)).map_err(debug)?,seed::sid(n,2),id(harness::WORLD),name).await?;
        }
        sqlx::query("UPDATE game_guilds SET vice_deficit_since=NULL").execute(&h.pool).await?;
        let opponent = seed::historical_session(&h.pool, 90, harness::item_fence()?).await?;
        seed::guard(&h.pool).await?;
        sqlx::raw_sql(include_str!("../../../../../apps/game-server/migrations/0232_guild_wars.sql")).execute(&h.pool).await?;
        Ok(opponent)
    }

    async fn command(h: &Harness, fence: CurrentCharacterItemFence, ordinal: u8, intent: GuildWarIntent, now: i64) -> Result<GuildWarResult, GuildError> {
        let seal = h.recovery.seal_current().map_err(|_| GuildError::NotCurrent)?;
        let authority = h.root.open_character_authority(&seal).await.map_err(|_| GuildError::NotCurrent)?;
        let command = CommandRef::new(fence.game_session_id, CommandId::new(u64::from(ordinal)).map_err(|_| GuildError::InvalidInput)?);
        h.root.commit_guild_war(&authority, &h.node, fence, command, [ordinal;32], intent, seed::sid(u32::from(ordinal),40), now).await
    }

    async fn now(h: &Harness) -> TestResult<i64> {
        Ok(sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp())*1000)::bigint").fetch_one(&h.pool).await?)
    }

    async fn active(h: &Harness, opponent: CurrentCharacterItemFence) -> TestResult<[u8;16]> {
        let declared = command(h, harness::item_fence()?, 200, GuildWarIntent::Declare { opponent: *guild(90)?.as_bytes(), frag_limit:10, duration_days:7 }, now(h).await?).await.map_err(debug)?;
        command(h, opponent, 201, GuildWarIntent::Accept { war:declared.war }, now(h).await?).await.map_err(debug)?;
        Ok(declared.war)
    }

    async fn finish(h: &Harness) -> TestResult<usize> {
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
        h.root.finish_due_guild_wars(&authority, &h.node, WorldId::decode(&id(harness::WORLD)).map_err(debug)?).await.map_err(|e| debug(e).into())
    }

    async fn score(h: &Harness, war: [u8;16]) -> TestResult<(i16,i32,i32)> {
        Ok(sqlx::query_as("SELECT state,declaring_frags,opposing_frags FROM game_guild_wars WHERE war_id=encode($1,'hex')::uuid").bind(war.as_slice()).fetch_one(&h.pool).await?)
    }

    async fn seed_wars(h: &Harness, count: u32) -> TestResult {
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
        for n in 1..=count {
            let opponent = seed::sid(1000+n,3);
            let name = format!("Scan Guild {}{}", char::from(b'A'+u8::try_from(n / 26)?), char::from(b'A'+u8::try_from(n % 26)?));
            sqlx::query("INSERT INTO game_guilds(guild_id,world_id,name,state,formation_deadline,revision,created_at) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,2,9999999999999,1,1)")
                .bind(opponent.as_slice()).bind(id(harness::WORLD).as_slice()).bind(name).execute(&mut *tx).await?;
            sqlx::query("INSERT INTO game_guild_wars(war_id,world_id,declaring_guild,opposing_guild,state,frag_limit,duration_days,declared_at,started_at,ends_at) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,2,10,7,1,1,9999999999999)")
                .bind(seed::sid(n,40).as_slice()).bind(id(harness::WORLD).as_slice()).bind(guild(80)?.as_bytes().as_slice()).bind(opponent.as_slice()).execute(&mut *tx).await?;
        }
        tx.commit().await?;
        Ok(())
    }

    #[test]
    fn guild_war_due_scan_skips_a_full_page_of_unfinished_wars() -> TestResult {
        run("war_due_scan", async |h, opponent| {
            let war = active(h, opponent).await?;
            seed_wars(h, 100).await?;
            sqlx::query("UPDATE game_guild_wars SET ends_at=1 WHERE war_id=encode($1,'hex')::uuid").bind(war.as_slice()).execute(&h.pool).await?;
            assert_eq!(finish(h).await?, 1);
            assert_eq!(score(h, war).await?, (3,0,0));
            assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_guild_wars WHERE state=2").fetch_one(&h.pool).await?, 100);
            Ok(())
        })
    }
    async fn harm(h: &Harness) -> TestResult {
        sqlx::query("INSERT INTO game_character_pvp_harm(game_session_id,command_id,ordinal,victim,attacker,binding,damage,unjustified) VALUES(encode($1,'hex')::uuid,1,0,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4,100,false)")
            .bind(id(harness::SESSION).as_slice()).bind(id(harness::CHARACTER).as_slice()).bind(seed::sid(90,1).as_slice()).bind([7u8;32].as_slice()).execute(&h.pool).await?;
        Ok(())
    }

    async fn attribute(h: &Harness, occurrence: u32) -> TestResult {
        use crate::durability::pvp_death_request::{PvpDeathContributor, PvpContributorKind};
        let mut tx = h.pool.begin().await?;
        attribute_guild_war_frag_in_transaction(&mut tx,
            CharacterId::from_bytes(id(harness::CHARACTER)).map_err(debug)?,
            WorldId::decode(&id(harness::WORLD)).map_err(debug)?,
            &seed::sid(occurrence,41),
            &[PvpDeathContributor { character_id:CharacterId::from_bytes(seed::sid(90,1)).map_err(debug)?, kind:PvpContributorKind::Damage, unjustified:false }],
        ).await?;
        tx.commit().await?;
        Ok(())
    }

    #[test]
    fn guild_war_attribution_refuses_frags_after_score_or_duration_ends() -> TestResult {
        run("war_terminal_frags", async |h, opponent| {
            let war = active(h, opponent).await?;
            harm(h).await?;
            sqlx::query("UPDATE game_guild_wars SET opposing_frags=10 WHERE war_id=encode($1,'hex')::uuid").bind(war.as_slice()).execute(&h.pool).await?;
            attribute(h,1).await?;
            assert_eq!(score(h,war).await?, (2,0,10));
            assert_eq!(h.count("game_guild_war_frags").await?,0);
            sqlx::query("UPDATE game_guild_wars SET opposing_frags=0,ends_at=1 WHERE war_id=encode($1,'hex')::uuid").bind(war.as_slice()).execute(&h.pool).await?;
            attribute(h,2).await?;
            assert_eq!(score(h,war).await?, (2,0,0));
            assert_eq!(h.count("game_guild_war_frags").await?,0);
            Ok(())
        })
    }

    #[test]
    fn guild_war_victim_window_caps_concurrent_occurrences_and_allows_old_frags() -> TestResult {
        run("war_victim_window", async |h, opponent| {
            let war = active(h, opponent).await?;
            harm(h).await?;
            use std::task::Poll;
            let mut pending: Vec<_> = (1..=6).map(|n| Some(Box::pin(attribute(h,n)))).collect();
            std::future::poll_fn(|cx| {
                let mut complete = true;
                for task in &mut pending {
                    if let Some(future) = task {
                        match std::future::Future::poll(future.as_mut(),cx) {
                            Poll::Ready(Ok(())) => *task = None,
                            Poll::Ready(Err(error)) => return Poll::Ready(Err(error)),
                            Poll::Pending => complete = false,
                        }
                    }
                }
                if complete { Poll::Ready(Ok(())) } else { Poll::Pending }
            }).await?;
            assert_eq!(score(h,war).await?, (2,0,5));
            assert_eq!(h.count("game_guild_war_frags").await?,5);
            // Retain one earlier frag outside the rolling window; the next occurrence may score.
            let mut fixture = h.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *fixture).await?;
            sqlx::query("UPDATE game_guild_war_frags SET occurred_at=floor(extract(epoch FROM clock_timestamp())*1000)::bigint-86400001 WHERE death_occurrence_id=(SELECT death_occurrence_id FROM game_guild_war_frags ORDER BY death_occurrence_id LIMIT 1)").execute(&mut *fixture).await?;
            fixture.commit().await?;
            attribute(h,7).await?;
            attribute(h,7).await?;
            assert_eq!(score(h,war).await?, (2,0,6));
            assert_eq!(h.count("game_guild_war_frags").await?,6);
            Ok(())
        })
    }

    async fn eligibility(h: &Harness, g: u32, state: i16, four_vices: bool) -> TestResult {
        let vice = if g==80 {103} else {113};
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
        sqlx::query("UPDATE game_guilds SET state=$2 WHERE guild_id=encode($1,'hex')::uuid").bind(guild(g)?.as_bytes().as_slice()).bind(state).execute(&mut *tx).await?;
        if four_vices {
            sqlx::query("INSERT INTO game_guild_members(character_id,guild_id,rank_level,joined_at) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,2,1) ON CONFLICT DO NOTHING")
                .bind(seed::sid(vice,1).as_slice()).bind(guild(g)?.as_bytes().as_slice()).execute(&mut *tx).await?;
            sqlx::query("INSERT INTO game_account_guild_leadership VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid) ON CONFLICT DO NOTHING")
                .bind(seed::sid(vice,2).as_slice()).bind(guild(g)?.as_bytes().as_slice()).bind(seed::sid(vice,1).as_slice()).execute(&mut *tx).await?;
        } else {
            sqlx::query("DELETE FROM game_account_guild_leadership WHERE account_id=encode($1,'hex')::uuid").bind(seed::sid(vice,2).as_slice()).execute(&mut *tx).await?;
            sqlx::query("DELETE FROM game_guild_members WHERE character_id=encode($1,'hex')::uuid").bind(seed::sid(vice,1).as_slice()).execute(&mut *tx).await?;
        }
        tx.commit().await?;
        Ok(())
    }

    #[test]
    fn guild_war_declaration_and_acceptance_require_both_active_rosters() -> TestResult {
        run("war_eligibility", async |h, opponent| {
            for (ordinal,(g,state,four)) in (220..).zip([(80,1,false),(90,1,false),(80,2,false),(90,2,false),(80,3,true),(90,3,true)]) {
                eligibility(h,g,state,four).await?;
                let outcome=command(h,harness::item_fence()?,ordinal,GuildWarIntent::Declare { opponent:*guild(90)?.as_bytes(),frag_limit:10,duration_days:7 },now(h).await?).await;
                assert!(matches!(&outcome,Err(GuildError::InvalidInput)), "unexpected declaration: {outcome:?}");
                assert_eq!(h.count("game_guild_wars").await?,0);
                eligibility(h,g,2,true).await?;
            }
            let declared=command(h,harness::item_fence()?,226,GuildWarIntent::Declare { opponent:*guild(90)?.as_bytes(),frag_limit:10,duration_days:7 },now(h).await?).await.map_err(debug)?;
            for (ordinal,(g,state,four)) in (227..).zip([(80,1,false),(90,1,false),(80,2,false),(90,2,false),(80,3,true),(90,3,true)]) {
                eligibility(h,g,state,four).await?;
                let outcome=command(h,opponent,ordinal,GuildWarIntent::Accept {war:declared.war},now(h).await?).await;
                assert!(matches!(&outcome,Err(GuildError::InvalidInput)), "unexpected acceptance: {outcome:?}");
                assert_eq!(score(h,declared.war).await?,(1,0,0));
                eligibility(h,g,2,true).await?;
            }
            command(h,opponent,233,GuildWarIntent::Accept {war:declared.war},now(h).await?).await.map_err(debug)?;
            assert_eq!(score(h,declared.war).await?,(2,0,0));
            Ok(())
        })
    }

    #[test]
    fn guild_war_five_opponent_bound_covers_both_guilds_and_acceptance() -> TestResult {
        run("war_opponent_bound", async |h, opponent| {
            seed_wars(h,5).await?;
            let declare=GuildWarIntent::Declare {opponent:*guild(90)?.as_bytes(),frag_limit:10,duration_days:7};
            let outcome=command(h,harness::item_fence()?,210,declare,now(h).await?).await;
            assert!(matches!(&outcome,Err(GuildError::InvalidInput)), "sixth opponent: {outcome:?}");
            sqlx::query("UPDATE game_guild_wars SET declaring_guild=encode($1,'hex')::uuid").bind(guild(90)?.as_bytes().as_slice()).execute(&h.pool).await?;
            assert!(matches!(command(h,harness::item_fence()?,211,declare,now(h).await?).await,Err(GuildError::InvalidInput)));
            assert_eq!(h.count("game_guild_war_commands").await?,0);
            sqlx::query("UPDATE game_guild_wars SET state=3").execute(&h.pool).await?;
            let declared=command(h,harness::item_fence()?,212,declare,now(h).await?).await.map_err(debug)?;
            sqlx::query("UPDATE game_guild_wars SET state=2,declaring_guild=encode($1,'hex')::uuid WHERE war_id<>encode($2,'hex')::uuid").bind(guild(80)?.as_bytes().as_slice()).bind(declared.war.as_slice()).execute(&h.pool).await?;
            assert!(matches!(command(h,opponent,213,GuildWarIntent::Accept {war:declared.war},now(h).await?).await,Err(GuildError::InvalidInput)));
            sqlx::query("UPDATE game_guild_wars SET declaring_guild=encode($1,'hex')::uuid WHERE war_id<>encode($2,'hex')::uuid").bind(guild(90)?.as_bytes().as_slice()).bind(declared.war.as_slice()).execute(&h.pool).await?;
            assert!(matches!(command(h,opponent,214,GuildWarIntent::Accept {war:declared.war},now(h).await?).await,Err(GuildError::InvalidInput)));
            assert_eq!(score(h,declared.war).await?,(1,0,0));
            sqlx::query("UPDATE game_guild_wars SET state=3 WHERE war_id=encode($1,'hex')::uuid").bind(seed::sid(1,40).as_slice()).execute(&h.pool).await?;
            command(h,opponent,215,GuildWarIntent::Accept {war:declared.war},now(h).await?).await.map_err(debug)?;
            assert_eq!(score(h,declared.war).await?,(2,0,0));
            Ok(())
        })
    }

    #[test]
    fn guild_war_blocks_new_invites_but_preserves_prior_acceptance() -> TestResult {
        run("war_invites", async |h, opponent| {
            use crate::durability::guild::{GuildCommand,GuildOutcome,GuildResult};
            let recruit=seed::historical_session(&h.pool,130,harness::item_fence()?).await?;
            let seal=h.recovery.seal_current().map_err(debug)?;
            let authority=h.root.open_character_authority(&seal).await.map_err(debug)?;
            let apply=async |fence:CurrentCharacterItemFence,n:u32,g:u32,op:GuildCommand| -> TestResult<GuildResult> {
                let request=seed::request(n,guild(g)?,fence,op)?;
                let outcome=h.root.commit_guild_membership(&authority,&h.node,fence,request).await.map_err(debug)?;
                Ok(match outcome { GuildOutcome::Committed(receipt)|GuildOutcome::Replayed(receipt)=>receipt.result })
            };
            let earlier=CharacterId::from_bytes(seed::sid(130,1)).map_err(debug)?;
            let later=CharacterId::from_bytes(seed::sid(131,1)).map_err(debug)?;
            assert_eq!(apply(harness::item_fence()?,10,80,GuildCommand::Invite(earlier)).await?,GuildResult::Ok);
            let war=command(h,harness::item_fence()?,11,GuildWarIntent::Declare {opponent:*guild(90)?.as_bytes(),frag_limit:10,duration_days:7},now(h).await?).await.map_err(debug)?.war;
            assert_eq!(apply(harness::item_fence()?,12,80,GuildCommand::Invite(later)).await?,GuildResult::NotAllowed);
            assert_eq!(h.count("game_guild_invitations").await?,1);
            assert_eq!(apply(recruit,14,80,GuildCommand::Accept).await?,GuildResult::Ok);
            command(h,opponent,13,GuildWarIntent::Accept {war},now(h).await?).await.map_err(debug)?;
            assert_eq!(apply(opponent,15,90,GuildCommand::Invite(later)).await?,GuildResult::NotAllowed);
            assert_eq!(apply(recruit,14,80,GuildCommand::Accept).await?,GuildResult::Ok);
            assert_eq!(h.count("game_guild_invitations").await?,0);
            assert_eq!(h.count("game_guild_members").await?,11);
            Ok(())
        })
    }

    #[test]
    fn guild_war_unanswered_declaration_expires_and_releases_live_pair() -> TestResult {
        run("war_pending_expiry", async |h, _opponent| {
            let intent=GuildWarIntent::Declare {opponent:*guild(90)?.as_bytes(),frag_limit:10,duration_days:7};
            let old=command(h,harness::item_fence()?,200,intent,now(h).await?).await.map_err(debug)?.war;
            seed_wars(h,1).await?;
            sqlx::query("UPDATE game_guild_wars SET state=1,started_at=NULL,ends_at=NULL,declared_at=floor(extract(epoch FROM clock_timestamp())*1000)::bigint WHERE war_id=encode($1,'hex')::uuid").bind(seed::sid(1,40).as_slice()).execute(&h.pool).await?;
            sqlx::query("UPDATE game_guild_wars SET declared_at=floor(extract(epoch FROM clock_timestamp())*1000)::bigint-604800001 WHERE war_id=encode($1,'hex')::uuid").bind(old.as_slice()).execute(&h.pool).await?;
            assert_eq!(finish(h).await?,1);
            assert_eq!(score(h,old).await?,(3,0,0));
            assert_eq!(score(h,seed::sid(1,40)).await?,(1,0,0));
            let replacement=command(h,harness::item_fence()?,201,intent,now(h).await?).await.map_err(debug)?.war;
            assert_ne!(replacement,old);
            assert_eq!(score(h,replacement).await?,(1,0,0));
            assert_eq!(command(h,harness::item_fence()?,200,intent,now(h).await?).await.map_err(debug)?.war,old);
            assert_eq!(finish(h).await?,0);
            Ok(())
        })
    }

    #[test]
    fn guild_war_timestamps_use_database_clock_and_replay_original_values() -> TestResult {
        run("war_db_clock", async |h, opponent| {
            let before=now(h).await?;
            let war=command(h,harness::item_fence()?,200,GuildWarIntent::Declare {opponent:*guild(90)?.as_bytes(),frag_limit:10,duration_days:7},1_000_000_000_000).await.map_err(debug)?.war;
            let after=now(h).await?;
            let declared:i64=sqlx::query_scalar("SELECT declared_at FROM game_guild_wars WHERE war_id=encode($1,'hex')::uuid").bind(war.as_slice()).fetch_one(&h.pool).await?;
            assert!(before<=declared && declared<=after,"declaration used caller clock: {declared}");
            let before=now(h).await?;
            command(h,opponent,201,GuildWarIntent::Accept {war},3_000_000_000_000).await.map_err(debug)?;
            let after=now(h).await?;
            let times:(i64,i64)=sqlx::query_as("SELECT started_at,ends_at FROM game_guild_wars WHERE war_id=encode($1,'hex')::uuid").bind(war.as_slice()).fetch_one(&h.pool).await?;
            assert!(before<=times.0 && times.0<=after,"activation used caller clock: {}",times.0);
            assert_eq!(times.1-times.0,604800000);
            command(h,opponent,201,GuildWarIntent::Accept {war},4_000_000_000_000).await.map_err(debug)?;
            let replay:(i64,i64)=sqlx::query_as("SELECT started_at,ends_at FROM game_guild_wars WHERE war_id=encode($1,'hex')::uuid").bind(war.as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(replay,times);
            assert_eq!(h.count("game_guild_war_commands").await?,2);
            Ok(())
        })
    }


    #[test]
    fn guild_war_maintenance_retains_integer_scores_above_u16() -> TestResult {
        run("war_wide_score", async |h, opponent| {
            let war=active(h,opponent).await?;
            seed_wars(h,1).await?;
            sqlx::query("UPDATE game_guild_wars SET declaring_frags=65536 WHERE war_id=encode($1,'hex')::uuid").bind(war.as_slice()).execute(&h.pool).await?;
            assert_eq!(finish(h).await?,1);
            assert_eq!(score(h,war).await?,(3,65536,0));
            let reason:Option<i16>=sqlx::query_scalar("SELECT ended_reason FROM game_guild_wars WHERE war_id=encode($1,'hex')::uuid").bind(war.as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(reason,Some(1));
            assert_eq!(score(h,seed::sid(1,40)).await?,(2,0,0));
            assert_eq!(finish(h).await?,0);
            Ok(())
        })
    }

}
