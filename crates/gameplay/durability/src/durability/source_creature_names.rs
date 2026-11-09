//! Fixed-name source lookup DATA in the existing current Map transaction.
//! A matching name never grants input, creature mutation or Character authority.
use super::admission_journal::party_target_binding::{
    CurrentVisibleActorObservation, current_visible_character_for_source_read_in_transaction,
    observe_current_visible_actor_for_source_read,
};
use super::spell_item_transaction::{SpellItemScopeAuthority, check_scope_transaction};
use super::{DurabilityError, DurabilityRoot};
use crate::foundation::{
    ChannelRuntimeV1, ExactActorRef, GameSessionId, MovementPositionSnapshot, RuntimeScopeRefV1,
};
use sqlx::{Postgres, Row, Transaction};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct CurrentSourcePlayerName {
    pub actor: ExactActorRef,
    pub session: GameSessionId,
    pub character: [u8; 16],
    pub name: String,
    pub character_revision: String,
}

#[derive(Debug, Clone)]
struct PreparedPlayer {
    actor: ExactActorRef,
    session: GameSessionId,
    character: [u8; 16],
    position: MovementPositionSnapshot,
    observed: Arc<CurrentVisibleActorObservation>,
}

/// Owned observations for one fixed-name lookup. This is DATA, not a name cache
/// or mutation authority; callers must reprove it under the current Runtime.
#[derive(Debug, Clone)]
pub struct PreparedSourcePlayerNames {
    scope: RuntimeScopeRefV1,
    generation: u64,
    pins: [[u8; 32]; 4],
    activation: u64,
    names: Vec<String>,
    entries: Vec<PreparedPlayer>,
}
impl PreparedSourcePlayerNames {
    /// Identity/name DATA can be rechecked after known movement in one held
    /// owner turn. The original full seal below still requires exact poses.
    pub fn identities_current_for(&self, runtime: &ChannelRuntimeV1) -> bool {
        runtime.owner_fence().is_ok()
            && runtime.binding().runtime_scope() == self.scope
            && runtime.binding().scope_generation().get() == self.generation
            && runtime.content_pin().activation_sequence() == self.activation
            && self.pins
                == [
                    runtime.content_pin().server_artifact_digest(),
                    runtime.content_pin().client_artifact_digest(),
                    runtime.content_pin().frame_binding_digest(),
                    runtime.map_revision_digest(),
                ]
            && self.entries.iter().all(|entry| {
                entry.observed.matches(runtime)
                    && runtime
                        .bound_attacker_lease(entry.actor, entry.session)
                        .is_ok_and(|lease| lease.character_id().as_bytes() == &entry.character)
            })
    }
    pub fn current_for(&self, runtime: &ChannelRuntimeV1) -> bool {
        self.identities_current_for(runtime)
            && self
                .entries
                .iter()
                .all(|entry| runtime.read_actor_position(entry.actor) == Ok(entry.position))
    }
}

/// Capture only synchronous owner observations before any SQL await. The
/// selected vector remains bounded by the actual owner's allocated slot table.
pub fn prepare_source_player_names(
    runtime: &ChannelRuntimeV1,
    names: &[&str],
    actors: &[(ExactActorRef, GameSessionId)],
) -> Result<PreparedSourcePlayerNames, DurabilityError> {
    if names.is_empty()
        || names.len() > 16
        || actors.len() > runtime.actor_slot_capacity()
        || names
            .iter()
            .any(|name| name.is_empty() || name.len() > 128 || !name.is_ascii())
        || runtime.owner_fence().is_err()
    {
        return Err(DurabilityError::Unavailable);
    }
    let mut identities = std::collections::BTreeSet::new();
    let mut characters = std::collections::BTreeSet::new();
    let mut entries = Vec::with_capacity(actors.len());
    for &(actor, session) in actors {
        if !identities.insert(actor.placement_identity()) {
            return Err(DurabilityError::Unavailable);
        }
        let lease = runtime
            .bound_attacker_lease(actor, session)
            .map_err(|_| DurabilityError::Unavailable)?;
        let character = *lease.character_id().as_bytes();
        if !characters.insert(character) {
            return Err(DurabilityError::Unavailable);
        }
        entries.push(PreparedPlayer {
            actor,
            session,
            character,
            position: runtime
                .read_actor_position(actor)
                .map_err(|_| DurabilityError::Unavailable)?,
            observed: Arc::new(
                observe_current_visible_actor_for_source_read(runtime, actor, session)
                    .ok_or(DurabilityError::Unavailable)?,
            ),
        });
    }
    Ok(PreparedSourcePlayerNames {
        scope: runtime.binding().runtime_scope(),
        generation: runtime.binding().scope_generation().get(),
        pins: [
            runtime.content_pin().server_artifact_digest(),
            runtime.content_pin().client_artifact_digest(),
            runtime.content_pin().frame_binding_digest(),
            runtime.map_revision_digest(),
        ],
        activation: runtime.content_pin().activation_sequence(),
        names: names.iter().map(|name| name.to_ascii_lowercase()).collect(),
        entries,
    })
}

/// Same fixed-name query and canonical journal proof, detached from Runtime.
/// No Runtime reference can be retained in this SQL future.
pub async fn read_prepared_source_player_names_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    root: &DurabilityRoot,
    scope: &SpellItemScopeAuthority,
    plan: &PreparedSourcePlayerNames,
) -> Result<Vec<CurrentSourcePlayerName>, DurabilityError> {
    let refuse = || DurabilityError::Unavailable;
    if scope.runtime_scope() != plan.scope || scope.generation() != plan.generation {
        return Err(refuse());
    }
    check_scope_transaction(tx, scope)
        .await
        .map_err(|_| refuse())?;
    let requested = plan
        .entries
        .iter()
        .map(|entry| {
            let id = &entry.character;
            id.iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        })
        .collect::<Vec<_>>();
    let names = &plan.names;
    let rows = sqlx::query(
        "SELECT uuid_send(r.character_id) AS character, \
         CASE WHEN lower(r.name)=ANY($2::text[]) THEN r.name ELSE NULL END AS name, \
         r.character_revision::text AS revision \
         FROM game_character_roots r JOIN unnest($1::text[]) AS requested(id) \
         ON r.character_id=requested.id::uuid \
         WHERE r.lifecycle=1 FOR SHARE OF r",
    )
    .bind(requested)
    .bind(names)
    .fetch_all(&mut **tx)
    .await?;

    let mut result = Vec::with_capacity(rows.len());
    for entry in &plan.entries {
        let (actor, session, character) = (entry.actor, entry.session, entry.character);
        let matching = rows
            .iter()
            .filter(|row| {
                row.try_get::<Vec<u8>, _>("character")
                    .is_ok_and(|id| id == character)
            })
            .collect::<Vec<_>>();
        if matching.len() > 1 {
            return Err(DurabilityError::InvalidStoredState);
        }
        let row = matching.first().ok_or_else(refuse)?;
        if current_visible_character_for_source_read_in_transaction(
            tx,
            root,
            scope,
            &entry.observed,
        )
        .await?
            != Some(character)
        {
            return Err(refuse());
        }
        let Some(name) = row.try_get::<Option<String>, _>("name")? else {
            continue; // Active Root and current canonical admission proved no match.
        };
        // Unicode SQL lower() supplies candidates; source literal comparisons
        // must not acquire aliases through a different Unicode case fold.
        if !names
            .iter()
            .any(|literal| name.eq_ignore_ascii_case(literal))
        {
            continue;
        }
        result.push(CurrentSourcePlayerName {
            actor,
            session,
            character,
            name,
            character_revision: row.try_get("revision")?,
        });
    }
    check_scope_transaction(tx, scope)
        .await
        .map_err(|_| refuse())?;
    Ok(result)
}

impl DurabilityRoot {
    /// Readonly canonical scope transaction over an already captured plan.
    pub async fn read_prepared_source_player_names(
        &self,
        permit: &super::spell_owner_commit::SpellLanePermit,
        recovery: &crate::durability::character_authority::ReconciledCharacterAuthority<'_, '_>,
        node: &super::runtime_scope_assignment::NodeIncarnationProof,
        plan: &PreparedSourcePlayerNames,
    ) -> Result<Vec<CurrentSourcePlayerName>, DurabilityError> {
        let record = recovery
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let root = self.clone();
        let node = node.clone();
        let plan = plan.clone();
        let mut context = permit;
        self.try_issue_semantic_pass()?
            .run_with_context(&mut context, move |holder, deadline, permit| {
                Box::pin(async move {
                    let permit: &super::spell_owner_commit::SpellLanePermit = permit;
                    let mut tx = super::spell_item_transaction::begin_spell_owner_transaction(
                        holder, deadline,
                    )
                    .await?;
                    let scope =
                        super::spell_item_transaction::assert_spell_item_scope_with_recovery(
                            &mut tx,
                            permit,
                            &record,
                            &node,
                            plan.scope,
                            plan.generation,
                        )
                        .await
                        .map_err(|_| DurabilityError::Unavailable)?;
                    let result = read_prepared_source_player_names_in_transaction(
                        &mut tx, &root, &scope, &plan,
                    )
                    .await?;
                    super::db::commit_semantic_transaction(tx, deadline).await?;
                    Ok(result)
                })
            })
            .await
    }
}
