//! Current readonly Scope/Map facts for the native area callback.
use super::*;
use crate::durability::admission_journal::party_target_binding::{
    CurrentVisibleActorObservation, current_visible_character_for_source_read_in_transaction,
    observe_current_visible_actor_for_source_read,
};
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::durability::spell_item_transaction::{SpellItemScopeAuthority, check_scope_transaction};
use crate::foundation::{ChannelRuntimeV1, ExactActorRef, GameSessionId, MovementPositionSnapshot};

#[derive(Debug, Clone)]
pub struct CurrentRottenAreaPlayerFacts {
    pub(super) fence: CurrentCharacterGameplayFence,
    pub(super) clocks: [i64; 3],
    pub(super) taint: super::super::rotten_blood::CurrentRottenBloodTaintFacts,
    pub(super) pose: MovementPositionSnapshot,
    observation: Arc<CurrentVisibleActorObservation>,
    pins: [[u8; 32]; 4],
    activation: u64,
    login_incarnation: Option<[u8; 32]>,
}
impl CurrentRottenAreaPlayerFacts {
    pub fn login_incarnation(&self) -> Option<[u8; 32]> {
        self.login_incarnation
    }

    pub fn fence(&self) -> CurrentCharacterGameplayFence {
        self.fence
    }
    pub fn clocks(&self) -> [i64; 3] {
        self.clocks
    }
    pub fn taint(&self) -> super::super::rotten_blood::CurrentRottenBloodTaintFacts {
        self.taint
    }
    pub fn current_for(&self, runtime: &ChannelRuntimeV1) -> bool {
        self.observation.matches(runtime)
            && runtime.read_actor_position(self.taint.actor) == Ok(self.pose)
            && runtime.binding().runtime_scope() == self.fence.runtime_scope
            && runtime.binding().scope_generation() == self.fence.scope_ownership_generation
            && runtime.content_pin().activation_sequence() == self.activation
            && self.pins
                == [
                    runtime.content_pin().server_artifact_digest(),
                    runtime.content_pin().client_artifact_digest(),
                    runtime.content_pin().frame_binding_digest(),
                    runtime.map_revision_digest(),
                ]
    }
}

#[derive(Debug, Clone)]
struct PreparedPlayer {
    actor: ExactActorRef,
    session: GameSessionId,
    pose: MovementPositionSnapshot,
    observation: Arc<CurrentVisibleActorObservation>,
}
/// Owned pre-await DATA; neither an actor cache nor a gameplay grant.
#[derive(Debug, Clone)]
pub struct PreparedRottenAreaRead {
    scope: crate::foundation::RuntimeScopeRefV1,
    generation: u64,
    pins: [[u8; 32]; 4],
    activation: u64,
    entries: Vec<PreparedPlayer>,
}
impl PreparedRottenAreaRead {
    pub fn current_for(&self, runtime: &ChannelRuntimeV1) -> bool {
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
            && self.entries.iter().all(|e| {
                e.observation.matches(runtime) && runtime.read_actor_position(e.actor) == Ok(e.pose)
            })
    }
}
pub fn prepare_current_rotten_area_read(
    runtime: &ChannelRuntimeV1,
    actors: &[(ExactActorRef, GameSessionId)],
) -> Option<PreparedRottenAreaRead> {
    runtime.owner_fence().ok()?;
    let mut seen = std::collections::BTreeSet::new();
    let mut entries = Vec::with_capacity(actors.len());
    for &(actor, session) in actors {
        if !seen.insert(actor.placement_identity()) {
            return None;
        }
        entries.push(PreparedPlayer {
            actor,
            session,
            pose: runtime.read_actor_position(actor).ok()?,
            observation: Arc::new(observe_current_visible_actor_for_source_read(
                runtime, actor, session,
            )?),
        });
    }
    Some(PreparedRottenAreaRead {
        scope: runtime.binding().runtime_scope(),
        generation: runtime.binding().scope_generation().get(),
        pins: [
            runtime.content_pin().server_artifact_digest(),
            runtime.content_pin().client_artifact_digest(),
            runtime.content_pin().frame_binding_digest(),
            runtime.map_revision_digest(),
        ],
        activation: runtime.content_pin().activation_sequence(),
        entries,
    })
}
/// Legacy wrapper; new paid stages use owned plan + SQL without a Runtime borrow.
pub async fn read_current_rotten_area_players_in_transaction(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    root: &DurabilityRoot,
    scope: &SpellItemScopeAuthority,
    runtime: &ChannelRuntimeV1,
    actors: &[(ExactActorRef, GameSessionId)],
) -> StoredResult<Vec<CurrentRottenAreaPlayerFacts>> {
    let plan =
        prepare_current_rotten_area_read(runtime, actors).ok_or(DurabilityError::Unavailable)?;
    let result = read_prepared_rotten_area_players_in_transaction(tx, root, scope, &plan).await?;
    if !plan.current_for(runtime) {
        return Err(DurabilityError::Unavailable);
    }
    Ok(result)
}
pub async fn read_prepared_rotten_area_players_in_transaction(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    root: &DurabilityRoot,
    scope: &SpellItemScopeAuthority,
    plan: &PreparedRottenAreaRead,
) -> StoredResult<Vec<CurrentRottenAreaPlayerFacts>> {
    check_scope_transaction(tx, scope)
        .await
        .map_err(|_| DurabilityError::Unavailable)?;
    if plan.scope != scope.runtime_scope() || plan.generation != scope.generation() {
        return Err(DurabilityError::Unavailable);
    }
    let mut characters = Vec::with_capacity(plan.entries.len());
    let mut unique = std::collections::BTreeSet::new();
    for entry in &plan.entries {
        let character = current_visible_character_for_source_read_in_transaction(
            tx,
            root,
            scope,
            &entry.observation,
        )
        .await?
        .ok_or(DurabilityError::Unavailable)?;
        if !unique.insert(character) {
            return Err(DurabilityError::Unavailable);
        }
        characters.push(character);
    }
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
            .fetch_one(&mut **tx)
            .await?;
    let actors = plan
        .entries
        .iter()
        .map(|e| (e.actor, e.session))
        .collect::<Vec<_>>();
    let mut taints = Vec::with_capacity(actors.len());
    for (chars, actors) in characters.chunks(64).zip(actors.chunks(64)) {
        taints.extend(
            super::super::rotten_blood::current::read_taint_rows(tx, chars, actors, now).await?,
        );
    }
    let observations = plan
        .entries
        .iter()
        .map(|e| Arc::clone(&e.observation))
        .collect::<Vec<_>>();
    let parties =
        crate::durability::world_party::read_world_party_views_for_observations_in_transaction(
            tx,
            root,
            scope,
            &observations,
        )
        .await
        .map_err(|_| DurabilityError::Unavailable)?;
    super::super::rotten_blood::current::apply_party_snapshots(
        tx,
        scope.world_bytes(),
        &mut taints,
        &parties,
    )
    .await?;
    let mut result = Vec::with_capacity(taints.len());
    for (index, taint) in taints.into_iter().enumerate() {
        let entry = plan
            .entries
            .get(index)
            .ok_or(DurabilityError::InvalidStoredState)?;
        let observed = Arc::clone(&entry.observation);
        let character = characters[index];
        let current = FreshAdmissionStore::from_root(root.clone())
            .current_session_in_transaction(tx, taint.session)
            .await?;
        if current.commit().character_id().as_bytes() != &character
            || current.current_runtime_scope() != scope.runtime_scope()
            || current.current_scope_generation().get() != scope.generation()
        {
            return Err(DurabilityError::Unavailable);
        }
        let row=sqlx::query("SELECT character_revision::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid AND lifecycle=1")
            .bind(character.as_slice()).fetch_optional(&mut **tx).await?.ok_or(DurabilityError::InvalidStoredState)?;
        let revision =
            super::super::super::character_progression::numeric_u64(&row, "character_revision")?;
        let fence = CurrentCharacterGameplayFence {
            character_id: CharacterId::from_bytes(character)
                .map_err(|_| DurabilityError::InvalidStoredState)?,
            game_session_id: taint.session,
            connection_generation: current.current_connection_generation(),
            character_lease_generation: current.current_character_lease().generation(),
            runtime_scope: scope.runtime_scope(),
            scope_ownership_generation: current.current_scope_generation(),
            expected_character_revision: CharacterRevision::new(revision)
                .map_err(|_| DurabilityError::InvalidStoredState)?,
        };
        let keys = oteryn_gameplay_quests::quest::rotten_blood::area_icons::TRACKS;
        let rows=sqlx::query("SELECT track_key,value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=ANY($2::text[])")
            .bind(character.as_slice()).bind(keys.to_vec()).fetch_all(&mut **tx).await?;
        let mut clocks = [0_i64; 3];
        for row in rows {
            let key: String = row.try_get("track_key")?;
            let index = keys
                .iter()
                .position(|v| *v == key)
                .ok_or(DurabilityError::InvalidStoredState)?;
            clocks[index] = row.try_get("value")?;
        }
        if clocks.iter().any(|v| *v < 0) || taint.observed_at_seconds < 0 {
            return Err(DurabilityError::InvalidStoredState);
        }
        let login_incarnation = super::super::rotten_blood::current::read_player_login_incarnation(
            tx,
            character,
            scope.world_bytes(),
            Some((
                taint.session,
                current.current_character_lease().generation(),
            )),
            None,
        )
        .await?;
        result.push(CurrentRottenAreaPlayerFacts {
            fence,
            login_incarnation,
            clocks,
            taint,
            pose: entry.pose,
            observation: observed,
            pins: plan.pins,
            activation: plan.activation,
        });
    }
    check_scope_transaction(tx, scope)
        .await
        .map_err(|_| DurabilityError::Unavailable)?;
    Ok(result)
}
