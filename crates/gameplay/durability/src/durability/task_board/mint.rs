//! Task Board participates in the existing reserved RewardClaim -> Inbox MINT.
//! The Item owner supplies the fresh instances, destination journal and DUR-03
//! audit; this module supplies only the source receipt and its exact output plan.
use super::super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::super::character_inbox::{
    RewardClaimInboxError, RewardClaimInboxMintCandidate, RewardClaimInboxMintOutput,
    RewardClaimInboxMintRequest, RewardClaimInboxPlan,
};
use super::super::character_progression::{
    CharacterProgressionError, CurrentCharacterGameplayFence, assert_gameplay_fence,
};
use super::super::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use super::super::item_mint::TypedDefinitionRef;
use super::super::item_mint_audit::Type2Transaction;
use super::super::runtime_scope_assignment::NodeIncarnationProof;
use super::super::{DurabilityError, DurabilityRoot};
use super::*;
use oteryn_gameplay_quests::quest::task_board::full::{
    SHOP_OFFERS, TaskBoardEffects, TaskBoardItemQuantity, TaskBoardShopKind,
};
use sha2::{Digest, Sha256};

pub(super) struct Preparation {
    pub replay: Option<TaskBoardCommit>,
    pub candidate: Option<RewardClaimInboxMintCandidate>,
}

fn outputs(action: &TaskBoardAction) -> Option<Vec<TaskBoardItemQuantity>> {
    match action {
        TaskBoardAction::BuyTalisman => Some(vec![TaskBoardItemQuantity {
            item_id: 51_978,
            amount: 1,
        }]),
        TaskBoardAction::BuyShop { offer } => SHOP_OFFERS
            .iter()
            .find(|item| item.id == *offer)
            .filter(|item| {
                matches!(
                    item.kind,
                    TaskBoardShopKind::Item | TaskBoardShopKind::ItemDouble
                )
            })
            .filter(|item| !item.requires_decoration_kit())
            .map(|item| {
                vec![TaskBoardItemQuantity {
                    item_id: item.appearance_or_item,
                    amount: 1,
                }]
            }),
        _ => None,
    }
}

fn identity(binding: &[u8; 32], purpose: &[u8], ordinal: u32) -> [u8; 16] {
    let hash = Sha256::new()
        .chain_update(b"oteryn:taskboard:inbox-mint:v1")
        .chain_update(binding)
        .chain_update(purpose)
        .chain_update(ordinal.to_be_bytes())
        .finalize();
    let mut id = [0; 16];
    id.copy_from_slice(&hash[..16]);
    id[6] = (id[6] & 15) | 0x70;
    id[8] = (id[8] & 63) | 0x80;
    id
}

fn unavailable(error: RewardClaimInboxError) -> CharacterProgressionError {
    match error {
        RewardClaimInboxError::Unavailable(error) => error.into(),
        RewardClaimInboxError::AuthorityRejected => CharacterProgressionError::AuthorityRejected,
        RewardClaimInboxError::ConflictingDelivery => {
            CharacterProgressionError::ConflictingOccurrence
        }
        _ => DurabilityError::InvalidStoredState.into(),
    }
}

/// Preparation holds no value and is retained across failed attempts. Exact task
/// command replay is checked before content resolution, so a later content change
/// cannot make an already committed purchase ambiguous.
#[allow(clippy::too_many_arguments)]
pub(super) async fn prepare(
    root: &DurabilityRoot,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    node: &NodeIncarnationProof,
    fence: CurrentCharacterGameplayFence,
    request: &TaskBoardCommand,
    binding: [u8; 32],
    catalogue: &TaskBoardCatalogue,
) -> Result<Preparation, CharacterProgressionError> {
    let Some(requested) = outputs(&request.action) else {
        return Ok(Preparation {
            replay: None,
            candidate: None,
        });
    };
    let recovery = authority
        .record_for(root)
        .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
    let node_copy = node.clone();
    let request_copy = request.clone();
    let expected_content = catalogue.gameplay_content_revision.clone();
    let (replay,revisions)=root.try_issue_semantic_pass()?.run(move |holder,deadline|Box::pin(async move {
        let mut tx=begin_semantic_transaction(holder,deadline).await?;
        assert_recovery_fence(&mut tx,&recovery).await?;
        lock_admission_relations(&mut tx).await?;
        match state::replay(&mut tx,fence,&request_copy,&binding).await {
            Ok(Some(replay))=>{commit_semantic_transaction(tx,deadline).await?;return Ok(Ok((Some(replay),None)));},
            Ok(None)=>{},Err(error)=>return Ok(Err(error)),
        }
        if let Err(error)=assert_gameplay_fence(&mut tx,&fence,&node_copy).await? {return Ok(Err(error));}
        let revisions:(String,String,String)=sqlx::query_as("SELECT content_revision,ruleset_revision,simulation_revision FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid")
            .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&mut *tx).await?;
        if revisions.0 != expected_content { return Ok(Err(CharacterProgressionError::ProgressionContextMismatch)); }
        commit_semantic_transaction(tx,deadline).await?;
        Ok(Ok((None,Some(revisions))))
    })).await??;
    if replay.is_some() {
        return Ok(Preparation {
            replay,
            candidate: None,
        });
    }
    let Some((content_revision, ruleset_revision, sim_revision)) = revisions else {
        return Err(DurabilityError::InvalidStoredState.into());
    };
    let QuestCause::Command(command) = request.cause else {
        return Err(CharacterProgressionError::InvalidInput);
    };
    let crate::foundation::RuntimeScopeRefV1::Channel { channel_id, .. } = fence.runtime_scope
    else {
        return Err(CharacterProgressionError::AuthorityRejected);
    };
    let mut resolved = Vec::new();
    for output in requested {
        let Some(item) = catalogue
            .items
            .get(&output.item_id)
            .and_then(|item| item.mint.clone())
        else {
            // The actual value pass persists the stable UnavailableItem refusal.
            return Ok(Preparation {
                replay: None,
                candidate: None,
            });
        };
        resolved.push(RewardClaimInboxMintOutput {
            source_line: u32::from(output.item_id),
            item,
            quantity: output.amount,
            key_number: None,
        });
    }
    let mint_identity: [u8; 32] = Sha256::new()
        .chain_update(binding)
        .chain_update(command.game_session_id().as_bytes())
        .chain_update(command.command_id().get().to_be_bytes())
        .finalize()
        .into();
    let plan = RewardClaimInboxPlan {
        command,
        character_id: fence.character_id,
        world_id: fence.runtime_scope.world_id(),
        channel_id,
        transaction_id: identity(&mint_identity, b"transaction", 0),
        source_binding: binding,
        item_instance_ids: (0..resolved.len())
            .map(|ordinal| identity(&mint_identity, b"item", ordinal as u32))
            .collect(),
    };
    let claim_key = match request.action {
        TaskBoardAction::BuyShop { offer } => format!("oteryn:taskboard/shop/{offer}"),
        _ => "oteryn:taskboard/talisman".to_owned(),
    };
    let mint = RewardClaimInboxMintRequest {
        claim: TypedDefinitionRef {
            family: "RewardClaim".to_owned(),
            production_key: claim_key,
            revision_ref: content_revision.clone(),
        },
        content_revision,
        ruleset_revision,
        sim_revision,
        outputs: resolved,
    };
    let mut candidate = root
        .prepare_reward_claim_inbox_mint(authority, node, fence.item_fence(), &plan, mint)
        .await
        .map_err(unavailable)?;
    root.charge_reward_claim_inbox_mint(authority, &mut candidate)
        .await
        .map_err(unavailable)?;
    Ok(Preparation {
        replay: None,
        candidate: Some(candidate),
    })
}

pub(super) async fn apply(
    tx: &mut Type2Transaction<'_>,
    fence: CurrentCharacterGameplayFence,
    request: &TaskBoardCommand,
    effects: &TaskBoardEffects,
    candidate: Option<&RewardClaimInboxMintCandidate>,
) -> Result<Option<TaskBoardRefusal>, DurabilityError> {
    if effects.grant_items.is_empty() {
        return Ok(None);
    }
    let Some(candidate) = candidate else {
        return Ok(Some(TaskBoardRefusal::UnavailableItem));
    };
    let mut actual = std::collections::BTreeMap::new();
    for item in candidate.outputs() {
        *actual.entry(item.source_line).or_insert(0_u32) += item.quantity;
    }
    let expected: std::collections::BTreeMap<_, _> = effects
        .grant_items
        .iter()
        .map(|item| (u32::from(item.item_id), item.amount))
        .collect();
    if actual != expected {
        return Err(DurabilityError::InvalidStoredState);
    }
    candidate
        .mint_in_transaction(tx, fence.item_fence())
        .await
        .map_err(|error| match error {
            RewardClaimInboxError::Unavailable(error) => error,
            _ => DurabilityError::InvalidStoredState,
        })?;
    let (_, cause, ordinal) = state::cause_key(request.cause);
    for (instance, item) in candidate
        .plan()
        .item_instance_ids
        .iter()
        .zip(candidate.outputs())
    {
        sqlx::query("INSERT INTO game_task_board_item_grants(item_instance_id,transaction_id,character_id,world_id,cause_id,cause_ordinal,source_item_id,definition_key,definition_revision,quantity) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,encode($5,'hex')::uuid,$6::text::numeric,$7,$8,$9,$10)")
            .bind(instance.as_slice()).bind(candidate.plan().transaction_id.as_slice()).bind(fence.character_id.as_bytes().as_slice())
            .bind(fence.runtime_scope.world_id().as_bytes().as_slice()).bind(cause.as_slice()).bind(ordinal.to_string())
            .bind(i32::try_from(item.source_line).map_err(|_|DurabilityError::InvalidStoredState)?)
            .bind(&item.item.definition.production_key).bind(&item.item.definition.revision_ref).bind(i64::from(item.quantity))
            .execute(&mut ***tx).await?;
    }
    Ok(None)
}
