//! Bounded DATA snapshot for actual source combat callbacks, inside their existing scope read.
use super::{DurabilityError, DurabilityRoot, RottenBloodState};
use crate::durability::spell_item_transaction::{SpellItemScopeAuthority, check_scope_transaction};
use crate::foundation::{ChannelRuntimeV1, ExactActorRef, GameSessionId};
use sqlx::{Postgres, Row, Transaction};

/// A current read result, not admission, damage or Quest-transition authority.
#[derive(Debug, Clone, Copy)]
pub struct CurrentRottenBloodPartyFacts {
    pub leader: [u8; 16],
    leader_login_incarnation: Option<[u8; 32]>,
    levels: [u8; 50],
    len: u8,
}
impl CurrentRottenBloodPartyFacts {
    pub fn leader_login_incarnation(&self) -> Option<[u8; 32]> {
        self.leader_login_incarnation
    }

    pub fn taint_levels(&self) -> &[u8] {
        &self.levels[..usize::from(self.len)]
    }
}
#[derive(Debug, Clone, Copy)]
pub struct CurrentRottenBloodTaintFacts {
    pub actor: ExactActorRef,
    pub session: GameSessionId,
    pub character: [u8; 16],
    pub state: RottenBloodState,
    pub observed_at_seconds: i64,
    pub party: Option<CurrentRottenBloodPartyFacts>,
}

/// Read the exact current source census through bounded 64-actor work pages.
/// The enclosing scope transaction and one database clock sample span every
/// page; paging never clips a physically owned player from the result.
pub async fn read_current_rotten_blood_taints_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    root: &DurabilityRoot,
    scope: &SpellItemScopeAuthority,
    runtime: &ChannelRuntimeV1,
    actors: &[(ExactActorRef, GameSessionId)],
) -> Result<Vec<CurrentRottenBloodTaintFacts>, DurabilityError> {
    if scope.runtime_scope() != runtime.binding().runtime_scope()
        || scope.generation() != runtime.binding().scope_generation().get()
        || runtime.owner_fence().is_err()
    {
        return Err(DurabilityError::Unavailable);
    }
    let current = runtime
        .positioned_melee_census()
        .map_err(|_| DurabilityError::Unavailable)?;
    let owned_count = current.len();
    let current = current
        .into_iter()
        .map(|(actor, _, session)| (actor.placement_identity(), (actor, session)))
        .collect::<std::collections::BTreeMap<_, _>>();
    let mut identities = std::collections::BTreeSet::new();
    if current.len() != owned_count
        || actors.len() > current.len()
        || actors.iter().any(|(actor, session)| {
            !identities.insert(actor.placement_identity())
                || current.get(&actor.placement_identity()) != Some(&(*actor, Some(*session)))
        })
    {
        return Err(DurabilityError::Unavailable);
    }
    check_scope_transaction(tx, scope)
        .await
        .map_err(|_| DurabilityError::Unavailable)?;
    if actors.is_empty() {
        return Ok(Vec::new());
    }
    let observed: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
            .fetch_one(&mut **tx)
            .await?;
    let mut facts = Vec::with_capacity(actors.len());
    let mut characters = std::collections::BTreeSet::new();
    for page in actors.chunks(64) {
        let rows = read_work_page(tx, root, scope, runtime, page, observed).await?;
        if rows.iter().any(|row| !characters.insert(row.character)) {
            return Err(DurabilityError::Unavailable);
        }
        facts.extend(rows);
    }
    read_parties(tx, root, scope, runtime, &mut facts).await?;
    check_scope_transaction(tx, scope)
        .await
        .map_err(|_| DurabilityError::Unavailable)?;
    runtime
        .owner_fence()
        .map_err(|_| DurabilityError::Unavailable)?;
    Ok(facts)
}

async fn read_work_page(
    tx: &mut Transaction<'_, Postgres>,
    root: &DurabilityRoot,
    scope: &SpellItemScopeAuthority,
    runtime: &ChannelRuntimeV1,
    actors: &[(ExactActorRef, GameSessionId)],
    observed_at_seconds: i64,
) -> Result<Vec<CurrentRottenBloodTaintFacts>, DurabilityError> {
    let refuse = || DurabilityError::Unavailable;
    if actors.len() > 64
        || scope.runtime_scope() != runtime.binding().runtime_scope()
        || scope.generation() != runtime.binding().scope_generation().get()
        || runtime.owner_fence().is_err()
    {
        return Err(refuse());
    }
    check_scope_transaction(tx, scope)
        .await
        .map_err(|_| refuse())?;
    let mut characters = Vec::with_capacity(actors.len());
    for (index, &(actor, session)) in actors.iter().enumerate() {
        if actors[..index].iter().any(|(before, _)| *before == actor) {
            return Err(refuse());
        }
        // The canonical admission journal independently joins the actual physical
        // actor/session to its live Character lease and current scope membership.
        use crate::durability::admission_journal::party_target_binding::{
            current_visible_character_for_source_read_in_transaction,
            observe_current_visible_actor_for_source_read,
        };
        let observed = observe_current_visible_actor_for_source_read(runtime, actor, session)
            .ok_or_else(refuse)?;
        let character =
            current_visible_character_for_source_read_in_transaction(tx, root, scope, &observed)
                .await?
                .ok_or_else(refuse)?;
        if !observed.matches(runtime) {
            return Err(refuse());
        }
        if characters.contains(&character) {
            return Err(refuse());
        }
        characters.push(character);
    }
    if characters.is_empty() {
        return Ok(Vec::new());
    }
    let facts = read_taint_rows(tx, &characters, actors, observed_at_seconds).await?;
    check_scope_transaction(tx, scope)
        .await
        .map_err(|_| refuse())?;
    runtime.owner_fence().map_err(|_| refuse())?;
    Ok(facts)
}

/// Membership remains with the existing strongly ordered World Party owner.
/// This read never turns an unavailable member into a solo actor or zero taint.
async fn read_parties(
    tx: &mut Transaction<'_, Postgres>,
    root: &DurabilityRoot,
    scope: &SpellItemScopeAuthority,
    runtime: &ChannelRuntimeV1,
    facts: &mut [CurrentRottenBloodTaintFacts],
) -> Result<(), DurabilityError> {
    let snapshots =
        crate::durability::world_party::read_world_party_views_for_scope_in_transaction(
            tx, root, scope, runtime,
        )
        .await
        .map_err(|_| DurabilityError::Unavailable)?;
    apply_party_snapshots(tx, scope.world_bytes(), facts, &snapshots).await
}

/// DATA identity of the same logical Player object across activity/House moves.
/// Raw admission replay bytes never leave SQL and grant no gameplay authority.
pub(in crate::durability::quest_state) async fn read_player_login_incarnation(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    character: [u8; 16],
    world: [u8; 16],
    session: Option<(GameSessionId, u64)>,
    party: Option<([u8; 16], u64, u64)>,
) -> Result<Option<[u8; 32]>, DurabilityError> {
    let session_id = session.map(|(id, _)| *id.as_bytes());
    let value:Option<Option<Vec<u8>>>=sqlx::query_scalar("SELECT sha256($5::bytea || uuid_send(c.character_id) || COALESCE(s.fresh_replay_key,s.initial_fresh_replay_key)) FROM game_durability_admission_character_guards c JOIN game_durability_admission_account_guards a ON a.account_id=c.account_id JOIN game_durability_reconnect_sessions s ON s.game_session_id=c.holder_game_session_id AND s.character_id=c.character_id AND s.account_id=c.account_id AND s.world_id=c.world_id AND s.character_lease_generation=c.lease_generation WHERE c.character_id=encode($1,'hex')::uuid AND c.world_id=encode($2,'hex')::uuid AND c.eligible AND a.presence_character_id=c.character_id AND a.holder_game_session_id=c.holder_game_session_id AND s.session_state IN(1,2) AND ($3::bytea IS NULL OR s.game_session_id=encode($3,'hex')::uuid) AND ($4::text IS NULL OR c.lease_generation=$4::text::numeric(20,0)) AND ($6::bytea IS NULL OR EXISTS(SELECT 1 FROM game_parties p JOIN game_party_members m ON m.party_id=p.party_id WHERE p.party_id=encode($6,'hex')::uuid AND p.world_id=c.world_id AND p.leader_character_id=c.character_id AND p.revision=$7::text::numeric(20,0) AND m.character_id=c.character_id AND m.presence_revision=$8::text::numeric(20,0) AND m.presence_state IN(1,2)))")
        .bind(character.as_slice()).bind(world.as_slice()).bind(session_id.as_ref().map(|id|id.as_slice()))
        .bind(session.map(|(_,lease)|lease.to_string())).bind(b"oteryn:source-player-login-incarnation:v1\0".as_slice())
        .bind(party.as_ref().map(|(id,_,_)|id.as_slice())).bind(party.map(|(_,rev,_)|rev.to_string())).bind(party.map(|(_,_,presence)|presence.to_string()))
        .fetch_optional(&mut **tx).await?;
    value
        .flatten()
        .map(|bytes| {
            bytes
                .try_into()
                .map_err(|_| DurabilityError::InvalidStoredState)
        })
        .transpose()
}

/// Shared bounded row decoder; callers already prove the current visible observations.
pub(in crate::durability::quest_state) async fn read_taint_rows(
    tx: &mut Transaction<'_, Postgres>,
    characters: &[[u8; 16]],
    actors: &[(ExactActorRef, GameSessionId)],
    observed_at_seconds: i64,
) -> Result<Vec<CurrentRottenBloodTaintFacts>, DurabilityError> {
    let refuse = || DurabilityError::Unavailable;
    if actors.len() != characters.len() || actors.len() > 64 {
        return Err(refuse());
    }
    let keys = characters
        .iter()
        .map(|id| {
            id.iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        })
        .collect::<Vec<_>>();
    // Every bounded statement uses the enclosing owner pass's one database clock sample.
    let rows=sqlx::query(
        "WITH requested AS (SELECT character,ordinal FROM unnest($1::text[]) WITH ORDINALITY AS p(character,ordinal)) \
         SELECT requested.ordinal,$2::bigint AS seconds,coalesce(q.value,0)::bigint AS access_stage, \
         coalesce(s.regular_taints,0)::smallint AS regular_taints,coalesce(s.child_flags,0)::smallint AS child_flags, \
         coalesce(s.final_taint,false) AS final_taint,coalesce(s.completed,false) AS completed,s.last_cleanse_at \
         FROM requested LEFT JOIN game_character_rotten_blood s ON s.character_id=requested.character::uuid \
         LEFT JOIN game_character_quest_tracks q ON q.character_id=requested.character::uuid \
            AND q.track_key='oteryn:quest-progress/rotten-blood/access' ORDER BY requested.ordinal"
    ).bind(keys).bind(observed_at_seconds).fetch_all(&mut **tx).await?;
    if rows.len() != actors.len() {
        return Err(DurabilityError::InvalidStoredState);
    }
    let mut facts = Vec::with_capacity(rows.len());
    for (index, row) in rows.into_iter().enumerate() {
        if row.try_get::<i64, _>("ordinal")? != i64::try_from(index + 1).map_err(|_| refuse())? {
            return Err(DurabilityError::InvalidStoredState);
        }
        let state = RottenBloodState::from_stored(
            u8::try_from(row.try_get::<i64, _>("access_stage")?)
                .map_err(|_| DurabilityError::InvalidStoredState)?,
            u8::try_from(row.try_get::<i16, _>("regular_taints")?)
                .map_err(|_| DurabilityError::InvalidStoredState)?,
            u8::try_from(row.try_get::<i16, _>("child_flags")?)
                .map_err(|_| DurabilityError::InvalidStoredState)?,
            row.try_get("final_taint")?,
            row.try_get("completed")?,
            row.try_get::<Option<i64>, _>("last_cleanse_at")?
                .map(u64::try_from)
                .transpose()
                .map_err(|_| DurabilityError::InvalidStoredState)?,
        )
        .map_err(|_| DurabilityError::InvalidStoredState)?;
        let observed = row.try_get("seconds")?;
        facts.push(CurrentRottenBloodTaintFacts {
            actor: actors[index].0,
            session: actors[index].1,
            character: characters[index],
            state,
            observed_at_seconds: observed,
            party: None,
        });
    }
    Ok(facts)
}

pub(in crate::durability::quest_state) async fn apply_party_snapshots(
    tx: &mut Transaction<'_, Postgres>,
    world: [u8; 16],
    facts: &mut [CurrentRottenBloodTaintFacts],
    snapshots: &[crate::foundation::PartySnapshot],
) -> Result<(), DurabilityError> {
    let mut needed = std::collections::BTreeSet::new();
    for snapshot in snapshots {
        if snapshot.party_id.is_some() {
            for member in &snapshot.members {
                if member.presence != crate::foundation::PartyPresence::Offline {
                    needed.insert(member.character);
                }
            }
        }
    }
    let mut levels = std::collections::BTreeMap::new();
    let requested = needed.into_iter().collect::<Vec<_>>();
    for page in requested.chunks(64) {
        let keys = page
            .iter()
            .map(|id| id.iter().map(|b| format!("{b:02x}")).collect::<String>())
            .collect::<Vec<_>>();
        let rows=sqlx::query("WITH requested AS (SELECT unnest($1::text[]) AS character) SELECT requested.character,coalesce(s.regular_taints,0)::smallint AS regular_taints,coalesce(s.child_flags,0)::smallint AS child_flags,coalesce(s.final_taint,false) AS final_taint,coalesce(s.completed,false) AS completed,s.last_cleanse_at,coalesce(q.value,0)::bigint AS access_stage FROM requested JOIN game_character_roots r ON r.character_id=requested.character::uuid AND r.world_id=encode($2,'hex')::uuid LEFT JOIN game_character_rotten_blood s ON s.character_id=r.character_id LEFT JOIN game_character_quest_tracks q ON q.character_id=r.character_id AND q.track_key=$3")
            .bind(keys).bind(world.as_slice()).bind(super::ACCESS_TRACK).fetch_all(&mut **tx).await?;
        if rows.len() != page.len() {
            return Err(DurabilityError::Unavailable);
        }
        for row in rows {
            let text: String = row.try_get("character")?;
            let mut character = [0; 16];
            if text.len() != 32 || !text.is_ascii() {
                return Err(DurabilityError::InvalidStoredState);
            }
            for (i, b) in character.iter_mut().enumerate() {
                *b = u8::from_str_radix(&text[i * 2..i * 2 + 2], 16)
                    .map_err(|_| DurabilityError::InvalidStoredState)?;
            }
            if !page.contains(&character) {
                return Err(DurabilityError::InvalidStoredState);
            }
            let state = RottenBloodState::from_stored(
                u8::try_from(row.try_get::<i64, _>("access_stage")?)
                    .map_err(|_| DurabilityError::InvalidStoredState)?,
                u8::try_from(row.try_get::<i16, _>("regular_taints")?)
                    .map_err(|_| DurabilityError::InvalidStoredState)?,
                u8::try_from(row.try_get::<i16, _>("child_flags")?)
                    .map_err(|_| DurabilityError::InvalidStoredState)?,
                row.try_get("final_taint")?,
                row.try_get("completed")?,
                row.try_get::<Option<i64>, _>("last_cleanse_at")?
                    .map(u64::try_from)
                    .transpose()
                    .map_err(|_| DurabilityError::InvalidStoredState)?,
            )
            .map_err(|_| DurabilityError::InvalidStoredState)?;
            let level = state.regular_taints() + 5 * u8::from(state.final_taint());
            if levels.insert(character, level).is_some() {
                return Err(DurabilityError::InvalidStoredState);
            }
        }
    }
    for fact in facts {
        let snapshot = snapshots
            .iter()
            .find(|s| s.character == fact.character)
            .ok_or(DurabilityError::Unavailable)?;
        if snapshot.party_id.is_none() {
            if !snapshot.members.is_empty() || snapshot.leader.is_some() {
                return Err(DurabilityError::InvalidStoredState);
            }
            fact.party = None;
            continue;
        }
        let leader = snapshot.leader.ok_or(DurabilityError::InvalidStoredState)?;
        let leader_member = snapshot
            .members
            .iter()
            .find(|member| {
                member.character == leader
                    && member.presence != crate::foundation::PartyPresence::Offline
            })
            .ok_or(DurabilityError::Unavailable)?;
        let login = read_player_login_incarnation(
            tx,
            leader,
            world,
            None,
            Some((
                snapshot
                    .party_id
                    .ok_or(DurabilityError::InvalidStoredState)?,
                snapshot
                    .revision
                    .ok_or(DurabilityError::InvalidStoredState)?,
                leader_member.presence_revision,
            )),
        )
        .await?;
        let mut party = CurrentRottenBloodPartyFacts {
            leader,
            leader_login_incarnation: login,
            levels: [0; 50],
            len: 0,
        };
        let mut saw_leader = false;
        for member in &snapshot.members {
            if member.presence == crate::foundation::PartyPresence::Offline {
                continue;
            }
            let level = *levels
                .get(&member.character)
                .ok_or(DurabilityError::Unavailable)?;
            if member.character == leader {
                saw_leader = true;
            }
            let index = usize::from(party.len);
            if index >= party.levels.len() {
                return Err(DurabilityError::InvalidStoredState);
            }
            party.levels[index] = level;
            party.len += 1;
        }
        if !saw_leader || party.len == 0 {
            return Err(DurabilityError::Unavailable);
        }
        fact.party = Some(party);
    }
    Ok(())
}
