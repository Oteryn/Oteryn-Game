//! Bounded DATA snapshot for actual source combat callbacks, inside their existing scope read.
use super::{DurabilityError, DurabilityRoot, SoulWarState};
use crate::durability::spell_item_transaction::{SpellItemScopeAuthority, check_scope_transaction};
use crate::foundation::{ChannelRuntimeV1, ExactActorRef, GameSessionId};
use sqlx::{Postgres, Row, Transaction};

/// A current read result, not admission, damage or Quest-transition authority.
#[derive(Debug, Clone, Copy)]
pub struct CurrentSoulWarTaintFacts {
    pub actor: ExactActorRef,
    pub session: GameSessionId,
    pub character: [u8; 16],
    pub count: u8,
    pub teleport_access: bool,
    pub defeated_bosses: u8,
    pub observed_at_seconds: i64,
    pub first_taint_at: Option<i64>,
}

/// Read the exact current source census through bounded 64-actor work pages.
/// The enclosing scope transaction and one database clock sample span every
/// page; paging never clips a physically owned player from the result.
pub async fn read_current_soul_war_taints_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    root: &DurabilityRoot,
    scope: &SpellItemScopeAuthority,
    runtime: &ChannelRuntimeV1,
    actors: &[(ExactActorRef, GameSessionId)],
) -> Result<Vec<CurrentSoulWarTaintFacts>, DurabilityError> {
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
) -> Result<Vec<CurrentSoulWarTaintFacts>, DurabilityError> {
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
    let keys = characters
        .iter()
        .map(|id| {
            id.iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        })
        .collect::<Vec<_>>();
    // Every bounded statement uses the enclosing owner pass's one database clock sample.
    let rows = sqlx::query(
        "WITH requested AS (SELECT character,ordinal FROM unnest($1::text[]) WITH ORDINALITY AS p(character,ordinal)), \
         observed AS MATERIALIZED (SELECT $2::bigint AS seconds) \
         SELECT requested.ordinal,observed.seconds,coalesce(s.teleport_access,false) AS teleport_access, \
         coalesce(s.defeated_bosses,0)::smallint AS defeated_bosses,s.first_taint_at, \
         coalesce(s.megalomania_completed,false) AS megalomania_completed,coalesce(s.final_reward_claimed,false) AS final_reward_claimed \
         FROM requested CROSS JOIN observed LEFT JOIN game_character_soul_war s ON s.character_id=requested.character::uuid ORDER BY requested.ordinal"
    ).bind(keys).bind(observed_at_seconds).fetch_all(&mut **tx).await?;
    if rows.len() != actors.len() {
        return Err(DurabilityError::InvalidStoredState);
    }
    let mut facts = Vec::with_capacity(rows.len());
    for (index, row) in rows.into_iter().enumerate() {
        if row.try_get::<i64, _>("ordinal")? != i64::try_from(index + 1).map_err(|_| refuse())? {
            return Err(DurabilityError::InvalidStoredState);
        }
        let state = SoulWarState {
            teleport_access: row.try_get("teleport_access")?,
            defeated_bosses: u8::try_from(row.try_get::<i16, _>("defeated_bosses")?)
                .map_err(|_| DurabilityError::InvalidStoredState)?,
            first_taint_at: row.try_get("first_taint_at")?,
            megalomania_completed: row.try_get("megalomania_completed")?,
            final_reward_claimed: row.try_get("final_reward_claimed")?,
        };
        state
            .validate()
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        let observed = row.try_get("seconds")?;
        facts.push(CurrentSoulWarTaintFacts {
            actor: actors[index].0,
            session: actors[index].1,
            character: characters[index],
            count: state.taint_count(observed),
            teleport_access: state.teleport_access,
            defeated_bosses: state.defeated_bosses,
            observed_at_seconds: observed,
            first_taint_at: state.first_taint_at,
        });
    }
    check_scope_transaction(tx, scope)
        .await
        .map_err(|_| refuse())?;
    runtime.owner_fence().map_err(|_| refuse())?;
    Ok(facts)
}
