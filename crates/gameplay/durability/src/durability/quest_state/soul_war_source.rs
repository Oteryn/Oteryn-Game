//! Closed Source Step/USE inputs for the existing sole Quest writer.
use super::super::item_use::soul_war_source::{QualifiedSoulWarUseTarget, SourceUse, source_use};
use super::super::item_use::{ItemUseKind, ItemUseRequest};
use super::*;
pub mod step;

#[derive(Debug, Clone)]
pub struct SoulWarSourceUse {
    pub item: ItemUseRequest,
    pub target: QualifiedSoulWarUseTarget,
}
impl SoulWarSourceUse {
    pub fn request(&self, fence: &CurrentCharacterGameplayFence) -> Result<QuestTransitionRequest> {
        self.item
            .validate()
            .map_err(|_| CharacterProgressionError::InvalidInput)?;
        let ItemUseKind::SoulWarSourceUse { wire_binding } = self.item.kind else {
            return Err(CharacterProgressionError::InvalidInput);
        };
        let kind = source_use(&self.item.source_definition)
            .ok_or(CharacterProgressionError::InvalidInput)?;
        if kind != self.target.source_use() || wire_binding != self.target.wire_binding() {
            return Err(CharacterProgressionError::InvalidInput);
        }
        if self.target.game_session_id() != fence.game_session_id
            || self.target.runtime_scope() != fence.runtime_scope
            || self.target.scope_generation() != fence.scope_ownership_generation
        {
            return Err(CharacterProgressionError::AuthorityRejected);
        }
        let suffix = match kind {
            SourceUse::MortalEssence => "greedy-maw",
            SourceUse::CleansedSanity => "cleansed-sanity",
            // Sorrow has no durable Quest state. Its separate closed Item caller
            // must not fabricate a persistent transition for the volatile effect.
            SourceUse::Sorrow => return Err(CharacterProgressionError::InvalidInput),
        };
        Ok(QuestTransitionRequest {
            transition_key: format!(
                "{}{suffix}",
                oteryn_gameplay_quests::quest::soul_war::source_items::TRANSITION_PREFIX
            ),
            cause: QuestCause::Use(CommandRef::new(
                fence.game_session_id,
                CommandId::new(self.item.command_id)
                    .map_err(|_| CharacterProgressionError::InvalidInput)?,
            )),
        })
    }
    pub fn binding(&self, character: CharacterId, request: &QuestTransitionRequest) -> [u8; 33] {
        let mut result = request_binding(character, request);
        let mut hash = Sha256::new();
        hash.update(result);
        hash.update(b"oteryn:soul-war-source-use-binding:v1");
        hash.update(self.item.binding());
        hash.update(self.item.transaction_id);
        hash.update(self.item.event_id);
        hash.update(self.target.actor().placement_identity());
        hash.update(self.target.activation_sequence().to_be_bytes());
        hash.update(self.target.server_artifact_digest());
        hash.update(self.target.client_artifact_digest());
        hash.update(self.target.frame_binding_digest());
        hash.update(self.target.map_revision_digest());
        hash.update([u8::from(self.target.target_accepted())]);
        result[1..].copy_from_slice(&hash.finalize());
        result
    }
}

#[derive(Debug, Clone)]
pub enum SoulWarSourceAction {
    Use(SoulWarSourceUse),
    Step(step::SoulWarSourceStep),
}
impl SoulWarSourceAction {
    pub fn request(&self, fence: &CurrentCharacterGameplayFence) -> Result<QuestTransitionRequest> {
        match self {
            Self::Use(input) => input.request(fence),
            Self::Step(input) => input.request(fence),
        }
    }
    pub fn binding(&self, character: CharacterId, request: &QuestTransitionRequest) -> [u8; 33] {
        match self {
            Self::Use(input) => input.binding(character, request),
            Self::Step(input) => input.binding(character, request),
        }
    }
}

impl DurabilityRoot {
    #[allow(
        clippy::too_many_arguments,
        reason = "the same Quest owner holds its canonical authority and optional physical Step context"
    )]
    pub async fn commit_soul_war_source_action(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        input: SoulWarSourceAction,
        catalogue: Arc<QuestStateCatalogue>,
        context: Option<(
            &super::super::spell_owner_commit::SpellLanePermit,
            super::super::quest_world_callback::QuestWorldContext,
        )>,
    ) -> Result<QuestTransitionOutcome> {
        let request = input.request(&fence)?;
        let context = match (&input, context) {
            (SoulWarSourceAction::Use(_), None) => None,
            (SoulWarSourceAction::Step(step), Some((permit, context))) => {
                let context: super::super::quest_world_callback::QuestWorldToolContext =
                    context.into();
                if !step.matches_context(&context) {
                    return Err(CharacterProgressionError::InvalidInput);
                }
                Some((permit, context, None))
            }
            _ => return Err(CharacterProgressionError::InvalidInput),
        };

        self.commit_quest_inner::<_, 0>(
            authority,
            node,
            fence,
            request,
            catalogue,
            context,
            None,
            None,
            None,
            &grizzly::NoGrizzlyPremium,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some(super::QuestSourceAction::SoulWar(Box::new(input))),
        )
        .await
    }
}

/// Item custody and the canonical cooldown share the Quest Type-2 tuple.
/// Wrong Sanity still records the cooldown, while retaining the source unit.
pub(super) async fn consume(
    tx: &mut super::super::item_mint_audit::Type2Transaction<'_>,
    fence: CurrentCharacterGameplayFence,
    node: &NodeIncarnationProof,
    action: &SoulWarSourceAction,
) -> StoredResult<bool> {
    use super::super::item_use::{ItemUseError, ItemUseOutcome};
    let SoulWarSourceAction::Use(input) = action else {
        return Ok(false);
    };
    if !input.target.target_accepted() && input.target.source_use() != SourceUse::CleansedSanity {
        return Ok(false);
    }
    let request = input
        .request(&fence)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let binding = input.binding(fence.character_id, &request);
    match super::super::item_use::soul_war_source::record_source_use_proof(
        tx,
        &fence.item_fence(),
        node,
        &input.item,
        &input.target,
        Some(&binding),
    )
    .await
    {
        Ok(()) => {}
        Err(ItemUseError::Unavailable(error)) => return Err(error),
        Err(_) => return Ok(false),
    }
    if !input.target.target_accepted() {
        return Ok(true);
    }
    let tuple = tx.tuple();
    match super::super::item_use::write(tx, &fence.item_fence(), &input.item, tuple).await {
        Ok(ItemUseOutcome::Written { .. }) => Ok(true),
        Ok(ItemUseOutcome::AlreadyCommitted { .. }) => Err(DurabilityError::InvalidStoredState),
        Err(ItemUseError::Unavailable(error)) => Err(error),
        Err(_) => Ok(false),
    }
}
