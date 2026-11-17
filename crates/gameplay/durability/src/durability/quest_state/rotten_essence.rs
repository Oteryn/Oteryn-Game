//! The fixed Rotten essence USE composes Quest and the existing Item writer.
use super::super::item_use::{ItemUseError, ItemUseKind, ItemUseOutcome, ItemUseRequest};
use super::*;

#[derive(Debug, Clone)]
pub struct RottenEssenceUse {
    pub item: ItemUseRequest,
    already_held: Arc<std::sync::OnceLock<()>>,
}
impl RottenEssenceUse {
    pub fn new(item: ItemUseRequest) -> Self {
        Self {
            item,
            already_held: Arc::new(std::sync::OnceLock::new()),
        }
    }
    pub fn already_held(&self) -> bool {
        self.already_held.get().is_some()
    }

    pub(super) fn request(
        &self,
        fence: &CurrentCharacterGameplayFence,
    ) -> Result<QuestTransitionRequest> {
        if self.item.kind != ItemUseKind::RottenEssence {
            return Err(CharacterProgressionError::InvalidInput);
        }
        self.item
            .validate()
            .map_err(|_| CharacterProgressionError::InvalidInput)?;
        let source = oteryn_gameplay_quests::quest::rotten_blood::essence(
            &self.item.source_definition.production_key,
        )
        .ok_or(CharacterProgressionError::InvalidInput)?;
        Ok(QuestTransitionRequest {
            transition_key: source.transition.into(),
            cause: QuestCause::Use(CommandRef::new(
                fence.game_session_id,
                CommandId::new(self.item.command_id)
                    .map_err(|_| CharacterProgressionError::InvalidInput)?,
            )),
        })
    }
    pub(super) fn binding(
        &self,
        character: CharacterId,
        request: &QuestTransitionRequest,
    ) -> [u8; 33] {
        let mut binding = request_binding(character, request);
        let digest = Sha256::new()
            .chain_update(binding)
            .chain_update(b"rotten-essence-use/v1")
            .chain_update(self.item.binding())
            .chain_update(self.item.transaction_id)
            .chain_update(self.item.event_id)
            .finalize();
        binding[1..].copy_from_slice(&digest);
        binding
    }
}

pub(super) async fn consume(
    tx: &mut super::super::item_mint_audit::Type2Transaction<'_>,
    fence: CurrentCharacterGameplayFence,
    input: &RottenEssenceUse,
) -> StoredResult<bool> {
    let source = oteryn_gameplay_quests::quest::rotten_blood::essence(
        &input.item.source_definition.production_key,
    )
    .ok_or(DurabilityError::InvalidStoredState)?;
    let held:Option<i64>=sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2 FOR UPDATE")
        .bind(fence.character_id.as_bytes().as_slice()).bind(source.track).fetch_optional(&mut ***tx).await?;
    if held.unwrap_or(0) != 0 {
        if held.unwrap_or(0) >= 1 {
            match super::super::item_use::soul_war_source::assert_retained_source(
                tx,
                &fence.item_fence(),
                &input.item,
            )
            .await
            {
                Ok(()) => {
                    let _ = input.already_held.set(());
                }
                Err(ItemUseError::Unavailable(error)) => return Err(error),
                Err(_) => {}
            }
        }
        return Ok(false);
    }
    let tuple = tx.tuple();
    match super::super::item_use::write(tx, &fence.item_fence(), &input.item, tuple).await {
        Ok(ItemUseOutcome::Written { .. }) => Ok(true),
        Ok(ItemUseOutcome::AlreadyCommitted { .. }) => Err(DurabilityError::InvalidStoredState),
        Err(ItemUseError::Unavailable(error)) => Err(error),
        Err(_) => Ok(false),
    }
}

/// Source essence USE is valid in a Channel, an Activity, or a real named House.
/// The House's origin remains provenance and never replaces its custody authority.
pub(super) async fn assert_fence(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    fence: &CurrentCharacterGameplayFence,
    node: &NodeIncarnationProof,
    allow_named_house: bool,
) -> StoredResult<
    std::result::Result<
        super::super::character_progression::FencedCharacterRoot,
        CharacterProgressionError,
    >,
> {
    if !allow_named_house || !matches!(fence.runtime_scope, RuntimeScopeRefV1::Instance { .. }) {
        return assert_gameplay_fence(tx, fence, node).await;
    }
    let origin:Option<String>=sqlx::query_scalar("SELECT origin_channel_id::text FROM game_durability_reconnect_sessions WHERE game_session_id=encode($1,'hex')::uuid AND character_id=encode($2,'hex')::uuid AND world_id=encode($3,'hex')::uuid AND runtime_scope_kind=2 AND runtime_scope_house_key IS NOT NULL FOR SHARE")
        .bind(fence.game_session_id.as_bytes().as_slice()).bind(fence.character_id.as_bytes().as_slice())
        .bind(fence.runtime_scope.world_id().as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
    let Some(origin) = origin else {
        return assert_gameplay_fence(tx, fence, node).await;
    };
    let origin = uuid_text(&origin)?;
    if super::super::house_hireling::fence::assert_house_custody_fence(
        tx,
        node,
        &fence.item_fence(),
        origin,
    )
    .await?
    .is_none()
    {
        return Ok(Err(CharacterProgressionError::AuthorityRejected));
    }
    let row=sqlx::query("SELECT character_revision::text,profile_revision,ruleset_revision,content_revision FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid AND lifecycle=1 FOR UPDATE")
        .bind(fence.character_id.as_bytes().as_slice()).fetch_optional(&mut **tx).await?
        .ok_or(DurabilityError::InvalidStoredState)?;
    let revision = numeric_u64(&row, "character_revision")?;
    if revision != fence.expected_character_revision.get() {
        return Ok(Err(CharacterProgressionError::CharacterRevisionMismatch));
    }
    Ok(Ok(
        super::super::character_progression::FencedCharacterRoot {
            revision,
            profile_revision: row.try_get("profile_revision")?,
            ruleset_revision: row.try_get("ruleset_revision")?,
            content_revision: row.try_get("content_revision")?,
        },
    ))
}

impl DurabilityRoot {
    pub async fn commit_rotten_blood_essence_use(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        input: RottenEssenceUse,
        catalogue: Arc<QuestStateCatalogue>,
    ) -> Result<QuestTransitionOutcome> {
        let request = input.request(&fence)?;
        self.commit_quest_inner::<_, 0>(
            authority,
            node,
            fence,
            request,
            catalogue,
            None,
            None,
            None,
            None,
            &grizzly::NoGrizzlyPremium,
            None,
            None,
            None,
            None,
            None,
            Some(input),
            None,
            None,
        )
        .await
    }
}
