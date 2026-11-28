//! Prove native terrain actions from their receipts, including uncommitted
//! receipts in the transaction that just changed the terrain.
use super::super::{item_transfer::CurrentCharacterItemFence, terrain_tool::TerrainMutation};
use crate::durability::db::commit_semantic_read;
use super::*;
use oteryn_gameplay_quests::quest::{
    authored_bindings::{Trigger, terrain_use::TerrainUse},
    npc_rules::legacy_defaults,
};

const DRAGON_CORPSE: &str = "oteryn:quest.dragon_corpse_quest";

pub(in crate::durability) fn dragon_corpse_binding<'a>(
    catalogue: &'a QuestStateCatalogue,
    mutation: &TerrainMutation,
) -> Option<&'a TerrainUse> {
    catalogue
        .authored_bindings()?
        .quests
        .iter()
        .find(|quest| quest.quest == DRAGON_CORPSE)?
        .stages
        .iter()
        .find_map(|stage| {
            let Trigger::TerrainUse { usage } = &stage.trigger else {
                return None;
            };
            (usage.result == mutation.after
                && usage.matches_placement(&mutation.placement)
                && usage.tools.iter().any(|tool| {
                    tool.key == mutation.tool_key && tool.revision == mutation.tool_revision
                }))
            .then_some(usage.as_ref())
        })
}

/// The caller holds the complete item/command fence and the Character root lock.
/// The same transaction owns the terrain CAS and any gathering outputs.
pub(in crate::durability) async fn credit_dragon_corpse(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    fence: CurrentCharacterItemFence,
    command: CommandRef,
    catalogue: &QuestStateCatalogue,
    mutation: &TerrainMutation,
) -> StoredResult<()> {
    let usage =
        dragon_corpse_binding(catalogue, mutation).ok_or(DurabilityError::InvalidStoredState)?;
    let transition = catalogue
        .transition(&usage.key)
        .ok_or(DurabilityError::InvalidStoredState)?;
    if transition.completes
        || transition.experience.is_some()
        || transition.world_change.is_some()
        || super::authored_admission::requires_premium(catalogue, transition, false)
    {
        return Err(DurabilityError::InvalidStoredState);
    }
    let character = fence.character_id;
    let id = character.as_bytes().as_slice();
    let (revision, level): (String, i64) = sqlx::query_as(
        "SELECT r.character_revision::text, p.level
         FROM game_character_roots r JOIN game_character_progression_state p USING(character_id)
         WHERE r.character_id=encode($1,'hex')::uuid
           AND r.character_revision=p.character_revision
           AND r.content_revision=$2 AND p.content_revision=$2",
    )
    .bind(id)
    .bind(catalogue.content_revision())
    .fetch_one(&mut **tx)
    .await?;
    let root_revision = revision
        .parse()
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let request = QuestTransitionRequest {
        transition_key: usage.key.clone(),
        cause: QuestCause::Command(command),
    };
    if !super::authored_admission::permits_level(catalogue, DRAGON_CORPSE, level)
        || !super::authored_admission::prerequisites_hold(tx, id, catalogue, DRAGON_CORPSE).await?
    {
        return Ok(());
    }
    let mut keys: Vec<_> = transition
        .effects
        .iter()
        .map(|effect| effect.track.as_str())
        .collect();
    keys.sort_unstable();
    let rows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT track_key, value FROM game_character_quest_tracks
         WHERE character_id=encode($1,'hex')::uuid AND quest_key=$2 AND track_key=ANY($3)
         ORDER BY track_key FOR UPDATE",
    )
    .bind(id)
    .bind(DRAGON_CORPSE)
    .bind(&keys)
    .fetch_all(&mut **tx)
    .await?;
    let stored: BTreeMap<_, _> = rows.into_iter().collect();
    // Both Dragon Corpse actions have only fixed storage comparisons and assignments.
    let changes = match catalogue.evaluate(transition, &stored, 0) {
        Ok(changes) => changes,
        Err(QuestRefusal::StageMismatch) => return Ok(()),
        Err(_) => return Err(DurabilityError::InvalidStoredState),
    };
    let state: Option<(String, Vec<u8>, bool)> = sqlx::query_as(
        "SELECT pinned_content_revision, definition_hash, completed_character_revision IS NOT NULL
         FROM game_character_quest_states
         WHERE character_id=encode($1,'hex')::uuid AND quest_key=$2 FOR UPDATE",
    )
    .bind(id)
    .bind(DRAGON_CORPSE)
    .fetch_optional(&mut **tx)
    .await?;
    let current_hash = catalogue
        .definition_hash(DRAGON_CORPSE)
        .ok_or(DurabilityError::InvalidStoredState)?;
    let (pin, hash) = match &state {
        Some((pin, hash, completed)) => {
            let hash = <[u8; 32]>::try_from(hash.as_slice())
                .map_err(|_| DurabilityError::InvalidStoredState)?;
            if hash != current_hash
                && !completed
                && legacy_defaults(catalogue, DRAGON_CORPSE, &hash).is_none()
            {
                return Ok(());
            }
            (pin.as_str(), hash)
        }
        None => {
            if !super::capacity::new_state_fits(tx, character).await? {
                return Ok(());
            }
            (catalogue.content_revision(), current_hash)
        }
    };
    if !super::capacity::tracks_fit(tx, character, &keys, stored.len()).await? {
        return Ok(());
    }
    if !matches_receipt(tx, fence, &request, DRAGON_CORPSE, usage).await? {
        return Err(DurabilityError::InvalidStoredState);
    }
    super::persist::persist(
        tx,
        super::persist::QuestWrite {
            character,
            root_revision,
            request: &request,
            transition,
            pin,
            hash: &hash,
            changes: &changes,
            binding: &super::request_binding(character, &request),
            state_exists: state.is_some(),
        },
    )
    .await?;
    Ok(())
}

pub(super) async fn admit(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    fence: CurrentCharacterGameplayFence,
    request: &QuestTransitionRequest,
    quest: &str,
    usage: &TerrainUse,
) -> StoredResult<bool> {
    if quest == DRAGON_CORPSE {
        return Ok(false);
    }
    matches_receipt(tx, fence.item_fence(), request, quest, usage).await
}

async fn matches_receipt(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    fence: CurrentCharacterItemFence,
    request: &QuestTransitionRequest,
    quest: &str,
    usage: &TerrainUse,
) -> StoredResult<bool> {
    let QuestCause::Command(command) = request.cause else {
        return Ok(false);
    };
    if command.game_session_id() != fence.game_session_id {
        return Ok(false);
    }
    let Some(channel) = fence.runtime_scope.channel_id() else {
        return Ok(false);
    };
    let world = fence.runtime_scope.world_id();
    let Some(placement) = usage.placement_bytes() else {
        return Ok(false);
    };
    // Definition alternatives are exact pairs, never a cross product of key/revision lists.
    // A native action can advance at most one stage in this quest. Exact replay of the
    // same stage has already returned its immutable quest receipt before this predicate.
    Ok(sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM game_terrain_tool_receipts r \
         JOIN game_terrain_tool_transforms t USING(world_id,channel_id,placement,reset_epoch) \
         WHERE r.character_id=encode($1,'hex')::uuid \
           AND r.game_session_id=encode($2,'hex')::uuid AND r.command_id=$3::text::numeric \
           AND r.world_id=encode($4,'hex')::uuid AND r.channel_id=encode($5,'hex')::uuid \
           AND ((octet_length($6)=40 AND r.placement=$6) OR (octet_length($6)=8 AND substring(r.placement FROM 33 FOR 8)=$6)) AND r.transformed_item_id=$7 \
           AND r.tool_definition_family='Item' \
           AND EXISTS(SELECT 1 FROM jsonb_to_recordset($8) AS tool(key TEXT,revision TEXT) \
                      WHERE tool.key=r.tool_definition_production_key \
                        AND tool.revision=r.tool_definition_revision_ref) \
           AND t.content_generation=(SELECT server_artifact_digest FROM game_content_activations a \
               WHERE a.world_id=r.world_id AND a.channel_id=r.channel_id \
               ORDER BY activation_sequence DESC LIMIT 1) \
           AND NOT EXISTS(SELECT 1 FROM game_character_quest_receipts q \
               WHERE q.character_id=r.character_id AND q.quest_key=$9 AND q.cause_kind='command' \
                 AND q.cause_id=r.game_session_id AND q.cause_ordinal=r.command_id))",
    )
    .bind(fence.character_id.as_bytes().as_slice())
    .bind(command.game_session_id().as_bytes().as_slice())
    .bind(command.command_id().get().to_string())
    .bind(world.as_bytes().as_slice())
    .bind(channel.as_bytes().as_slice())
    .bind(placement)
    .bind(i64::from(usage.result))
    .bind(serde_json::to_value(&usage.tools).map_err(|_| DurabilityError::InvalidStoredState)?)
    .bind(quest)
    .fetch_one(&mut **tx)
    .await?)
}

/// Candidate-selection facts, not admission authority. The quest writer rechecks the receipt.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TerrainUseObservation {
    pub tool: oteryn_gameplay_quests::quest::authored_bindings::terrain_use::ToolDefinition,
    pub placement: Vec<u8>,
    pub result: u32,
}

impl DurabilityRoot {
    /// Read the exact committed action after fresh USE or receipt replay. Legacy untyped
    /// receipts cannot select a new typed observer. No current tool handle is required.
    pub async fn read_quest_terrain_use(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: super::super::item_transfer::CurrentCharacterItemFence,
        command: crate::foundation::CommandRef,
    ) -> StoredResult<Option<TerrainUseObservation>> {
        if command.game_session_id() != fence.game_session_id {
            return Ok(None);
        }
        let (world, channel) = super::super::item_transfer::scope_of(&fence)
            .map_err(|_| DurabilityError::Unavailable)?;
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            super::super::character_authority::assert_recovery_fence(&mut tx, &recovery).await?;
            if super::super::character_progression::assert_item_only_fence(&mut tx, &fence, &node).await?.is_err() {
                return Err(DurabilityError::Unavailable);
            }
            let row = sqlx::query("SELECT tool_definition_family,tool_definition_production_key,tool_definition_revision_ref,placement,transformed_item_id FROM game_terrain_tool_receipts WHERE character_id=encode($1,'hex')::uuid AND game_session_id=encode($2,'hex')::uuid AND command_id=$3::text::numeric AND world_id=encode($4,'hex')::uuid AND channel_id=encode($5,'hex')::uuid")
                .bind(fence.character_id.as_bytes().as_slice()).bind(fence.game_session_id.as_bytes().as_slice())
                .bind(command.command_id().get().to_string()).bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice())
                .fetch_optional(&mut *tx).await?;
            let observation = row.map(|row| -> StoredResult<Option<TerrainUseObservation>> {
                let family: Option<String> = row.try_get("tool_definition_family")?;
                let key: Option<String> = row.try_get("tool_definition_production_key")?;
                let revision: Option<String> = row.try_get("tool_definition_revision_ref")?;
                let (Some(family),Some(key),Some(revision)) = (family,key,revision) else { return Ok(None); };
                if family != "Item" { return Ok(None); }
                Ok(Some(TerrainUseObservation {
                    tool:oteryn_gameplay_quests::quest::authored_bindings::terrain_use::ToolDefinition { key,revision },
                    placement:row.try_get("placement")?,
                    result:u32::try_from(row.try_get::<i64,_>("transformed_item_id")?).map_err(|_| DurabilityError::InvalidStoredState)?,
                }))
            }).transpose()?.flatten();
            commit_semantic_read(tx, deadline).await?;
            Ok(observation)
        })).await
    }
}
