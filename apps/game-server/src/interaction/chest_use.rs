//! D39: server-side `USE` on a placed plain `once` reward chest, wired to the CHEST-1
//! reward-claim MINT (`durability::reward_claim_mint`).
//!
//! Accepted contract text: GAME-INTERACTION-01 chest `USE` slice (§4.1, §4.3, §5.1, §5.3-§5.7,
//! §17, §19.1) in `OTERYN_GAME_D39_CHEST_USE_GAME_INTERACTION_AMENDMENT_DECISION_2026-09-29.md`.
//!
//! - **Child identity (§5.1, §5.3-§5.7).** The chest `USE` is a first-level child of the `USE`
//!   command's root occurrence. Its definition is the placed chest's resolved definition
//!   (`family:key@revision`) plus the claimed RewardClaim, its target the placed chest (resolved
//!   from the current Content, never trusted), its edge [`USE_EDGE`], no ordinal, and the
//!   revisions in force at the `USE`. The occurrence key is therefore the placed chest plus the
//!   claim identity (D40).
//! - **Retry (§17).** DUR-03 keys the MINT by the `USE` `CommandRef` and binds the placed chest
//!   into its intent: the same `CommandRef` returns its first outcome, a changed intent under it
//!   (another chest, claim, reward or backpack) conflicts, a second command on a claimed `once`
//!   claim is refused, and a second command while the first is still pending is refused with
//!   `ClaimPending` (§17.2). An ambiguous commit stays pending on the same DUR-03 reservation;
//!   calling [`settle_chest_use`] again with the same request reconciles it.
//! - **Replay precondition.** A replay recomputes the intent from the current Content and the
//!   equipped backpack; nothing is read back from the stored candidate. It returns the first
//!   outcome only while the chest placement, the reward item and the backpack definitions are
//!   unchanged and the same backpack is equipped. Otherwise it is refused or conflicts before any
//!   DUR-03 write, and the committed first outcome stays durable. The FND-02 command-result
//!   replay (§17.1) is the path that must answer a duplicate `CommandRef` after such a change.
//! - **DUR-03 (§19.1).** Durable value, refusal and ambiguity stay in
//!   `durability::reward_claim_mint`. This module only resolves facts from Content, builds the
//!   correlation identity and calls freeze and commit.
//!
//! Facts: everything comes from the current [`CanonicalReferencePlayableContent`], never from
//! the caller, who names only the chest. The claim is the RewardClaim definition that lists the
//! chest placement (a placement belongs to at most one claim), and the reward item and its count
//! are that placement's entry (CHEST-CONTENT-1, architect ruling on #162 5905746509). The reward
//! item's and the equipped backpack's `ItemDefinitionFacts` come through B3-2's
//! [`resolve_item_definition_facts`]; the MINT's own admission still enforces every D40-D42 and
//! D92 rule, so an item without a known stack class fails closed (D82).
//!
//! Like B3-2, this has no production caller yet: the client `USE` command (control-wire lane)
//! and its network dispatch are a later stage. Note for that dispatch: a commit that did not
//! finish (ambiguous, or rejected by a stale fence) leaves its reservation pending, and every
//! other `CommandRef` for the claim is refused with `ClaimPending` until it is terminal. The
//! dispatch must therefore replay the same `CommandRef` with the same request to reconcile it,
//! never allocate a new one. This is a top-level module
//! (`crate::interaction_chest_use`), not `interaction::chest_use`, because
//! `tests/interaction_workflow.rs` recompiles `interaction/mod.rs` without Content or
//! durability.

use crate::combat::DurabilitySession;
use crate::combat_pickup::{PickupContentError, resolve_item_definition_facts};
use crate::content::{
    CanonicalReferencePlayableContent, DefinitionFamily, PlacementKey, PlacementRef,
    ReferenceDefinitionKind, TypedDefinitionRef as ContentDefinitionRef,
};
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::item_transfer::{CurrentCharacterItemFence, ItemTransferError};
use crate::durability::reward_claim_mint::{
    RewardClaimMintError, RewardClaimMintOutcome, RewardClaimMintRequest, RewardClaimRefusal,
};
use crate::foundation::CommandRef;
use crate::interaction::{
    ChildOccurrenceRef, InteractionError, RootSourceOccurrenceRef, SemanticRevisionContext,
};

/// §5.5 typed edge of a player `USE` on a placed object.
pub(crate) const USE_EDGE: &str = "USE";

/// A player's `USE` of one placed reward chest, as GAME-INTERACTION resolves it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ChestUseRequest {
    /// The FND-02 `CommandRef` of the player's `USE`; the DUR-03 cause key.
    pub(crate) command: CommandRef,
    /// The placed chest the player used. It must be an Item placement in the current Content
    /// and be listed under a RewardClaim; the claim and the reward are resolved from Content.
    pub(crate) chest: PlacementKey,
    pub(crate) content_revision: String,
    pub(crate) ruleset_revision: String,
    pub(crate) sim_revision: String,
}

/// Committed or replayed result of one chest `USE`, with its GAME-INTERACTION correlation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ChestUseOutcome {
    pub(crate) child: ChildOccurrenceRef,
    pub(crate) mint: RewardClaimMintOutcome,
}

#[derive(Debug)]
pub(crate) enum ChestUseError {
    /// No placement with the named key exists in the current Content.
    ChestNotPlaced,
    /// The placement is not an Item placement, so it cannot be a chest.
    ChestNotAnItem,
    /// No RewardClaim in the current Content lists the placement, so it rewards nothing.
    ChestHasNoClaim,
    /// The reward item or the equipped backpack has no admissible Content definition.
    Content(PickupContentError),
    /// The child occurrence identity could not be built (empty key).
    Identity(InteractionError),
    /// The equipped backpack could not be read.
    Backpack(ItemTransferError),
    /// DUR-03 refused, conflicted or could not commit; nothing was written on a refusal.
    Mint(RewardClaimMintError),
}

impl From<RewardClaimMintError> for ChestUseError {
    fn from(error: RewardClaimMintError) -> Self {
        Self::Mint(error)
    }
}

impl From<InteractionError> for ChestUseError {
    fn from(error: InteractionError) -> Self {
        Self::Identity(error)
    }
}

impl std::fmt::Display for ChestUseError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ChestNotPlaced => formatter.write_str("chest use: no such placement in Content"),
            Self::ChestNotAnItem => {
                formatter.write_str("chest use: the placement is not an Item placement")
            }
            Self::ChestHasNoClaim => {
                formatter.write_str("chest use: no reward claim lists the placement")
            }
            Self::Content(error) => write!(formatter, "chest use content resolution: {error}"),
            Self::Identity(error) => write!(formatter, "chest use occurrence identity: {error}"),
            Self::Backpack(error) => write!(formatter, "chest use backpack read: {error}"),
            Self::Mint(error) => write!(formatter, "chest use reward claim: {error}"),
        }
    }
}

impl std::error::Error for ChestUseError {}

/// §4.1 `RootSourceOccurrenceRef`: the player's `USE` command occurrence.
fn root_occurrence(command: CommandRef) -> Result<RootSourceOccurrenceRef, InteractionError> {
    let session: String = command
        .game_session_id()
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    RootSourceOccurrenceRef::new(&format!(
        "use-command:{session}:{}",
        command.command_id().get()
    ))
}

/// §5.3 definition component: the resolved chest placement definition plus the claim.
fn chest_definition(placement: &str, claim: &TypedDefinitionRef) -> String {
    format!(
        "{placement}+{}:{}@{}",
        claim.family, claim.production_key, claim.revision_ref
    )
}

/// §5.1 child identity on any typed edge. Pure; the target must already be resolved.
fn chest_occurrence(
    request: &ChestUseRequest,
    placement_definition: &str,
    claim: &TypedDefinitionRef,
    edge: &str,
) -> Result<ChildOccurrenceRef, InteractionError> {
    let revisions = SemanticRevisionContext::new(
        &request.content_revision,
        &request.ruleset_revision,
        &request.sim_revision,
    )?;
    ChildOccurrenceRef::for_root(
        &root_occurrence(request.command)?,
        &chest_definition(placement_definition, claim),
        request.chest.as_str(),
        edge,
        None,
        &revisions,
    )
}

/// §5.1 child identity of the chest `USE`, from the resolved chest ([`resolve_chest`]).
pub(crate) fn chest_use_occurrence(
    request: &ChestUseRequest,
    chest: &ResolvedChest,
) -> Result<ChildOccurrenceRef, InteractionError> {
    chest_occurrence(request, &chest.definition, &chest.claim, USE_EDGE)
}

/// A placed chest resolved from the current Content: its placement definition, the claim that
/// lists it and that placement's reward.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ResolvedChest {
    /// The placement definition as `Item:key@revision` (§5.3).
    pub(crate) definition: String,
    pub(crate) claim: TypedDefinitionRef,
    pub(crate) reward_item: TypedDefinitionRef,
    pub(crate) quantity: u32,
}

fn durable_ref(family: &str, definition: &ContentDefinitionRef) -> TypedDefinitionRef {
    TypedDefinitionRef {
        family: family.to_owned(),
        production_key: definition.key().as_str().to_owned(),
        revision_ref: definition.revision().as_str().to_owned(),
    }
}

/// §5.4: the server resolves the target. The named chest must be an Item placement of the
/// current Content generation, listed under a RewardClaim; its reward is that placement's entry.
pub(crate) fn resolve_chest(
    content: &CanonicalReferencePlayableContent,
    chest: &PlacementKey,
) -> Result<ResolvedChest, ChestUseError> {
    let placement: &PlacementRef = content
        .placements
        .iter()
        .find(|placement| &placement.key == chest)
        .ok_or(ChestUseError::ChestNotPlaced)?;
    if placement.definition.family() != DefinitionFamily::Item {
        return Err(ChestUseError::ChestNotAnItem);
    }
    let (claim, entry) = content
        .definitions
        .iter()
        .find_map(|definition| match &definition.kind {
            ReferenceDefinitionKind::RewardClaim(claim) => claim
                .placement(chest)
                .map(|entry| (&definition.definition, entry)),
            _ => None,
        })
        .ok_or(ChestUseError::ChestHasNoClaim)?;
    // Link validation guarantees exactly one reward item per placement; fail closed otherwise.
    let [reward] = entry.items.as_slice() else {
        return Err(ChestUseError::ChestHasNoClaim);
    };
    if reward.item.family() != DefinitionFamily::Item {
        return Err(ChestUseError::Content(PickupContentError::NotAnItem));
    }
    Ok(ResolvedChest {
        definition: format!(
            "Item:{}@{}",
            placement.definition.key().as_str(),
            placement.definition.revision().as_str()
        ),
        claim: durable_ref("RewardClaim", claim),
        reward_item: durable_ref("Item", &reward.item),
        quantity: reward.count,
    })
}

/// Everything before DUR-03: resolve the chest and the facts from `content` and the equipped
/// backpack, and build the child occurrence and the MINT intent. Reads only; writes nothing.
pub(crate) async fn prepare_chest_use(
    session: &DurabilitySession<'_, '_, '_>,
    content: &CanonicalReferencePlayableContent,
    fence: CurrentCharacterItemFence,
    request: ChestUseRequest,
) -> Result<(ChildOccurrenceRef, RewardClaimMintRequest), ChestUseError> {
    let chest = resolve_chest(content, &request.chest)?;
    let child = chest_use_occurrence(&request, &chest)?;
    let item = resolve_item_definition_facts(content, &chest.reward_item)
        .map_err(ChestUseError::Content)?;
    let Some(current) = session
        .root
        .read_character_backpack(session.authority, fence.character_id)
        .await
        .map_err(ChestUseError::Backpack)?
    else {
        return Err(RewardClaimMintError::Refused(RewardClaimRefusal::NoMainBackpack).into());
    };
    let backpack = resolve_item_definition_facts(content, &current.backpack.definition)
        .map_err(ChestUseError::Content)?;
    let mint_request = RewardClaimMintRequest {
        command: request.command,
        claim: chest.claim,
        source_placement: request.chest.as_str().to_owned(),
        item,
        quantity: chest.quantity,
        backpack,
        content_revision: request.content_revision,
        ruleset_revision: request.ruleset_revision,
        sim_revision: request.sim_revision,
    };
    Ok((child, mint_request))
}

/// D39 entry point: [`prepare_chest_use`], then claim through CHEST-1's freeze and commit.
/// Every failure before freeze writes nothing.
pub(crate) async fn settle_chest_use(
    session: &DurabilitySession<'_, '_, '_>,
    content: &CanonicalReferencePlayableContent,
    fence: CurrentCharacterItemFence,
    request: ChestUseRequest,
) -> Result<ChestUseOutcome, ChestUseError> {
    let (child, mint_request) = prepare_chest_use(session, content, fence, request).await?;
    let mut candidate = session
        .root
        .freeze_reward_claim_mint(session.authority, session.node, fence, mint_request)
        .await?;
    let mint = session
        .root
        .commit_reward_claim_mint(session.authority, session.node, fence, &mut candidate)
        .await?;
    Ok(ChestUseOutcome { child, mint })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::foundation::{CommandId, GameSessionId};

    type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

    const CHEST_DEFINITION: &str = "Item:oteryn:item.test.chest@definition-r1";

    fn request(command_id: u64, chest: &str) -> TestResult<ChestUseRequest> {
        let session =
            GameSessionId::decode(&[7, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, 7])
                .map_err(|error| format!("{error:?}"))?;
        let command_id = CommandId::new(command_id).map_err(|error| format!("{error:?}"))?;
        Ok(ChestUseRequest {
            command: CommandRef::new(session, command_id),
            chest: PlacementKey::new(chest)?,
            content_revision: "content-1".into(),
            ruleset_revision: "ruleset-1".into(),
            sim_revision: "sim-1".into(),
        })
    }

    fn resolved(claim_key: &str, definition: &str) -> ResolvedChest {
        ResolvedChest {
            definition: definition.into(),
            claim: TypedDefinitionRef {
                family: "RewardClaim".into(),
                production_key: claim_key.into(),
                revision_ref: "claim-r1".into(),
            },
            reward_item: TypedDefinitionRef {
                family: "Item".into(),
                production_key: "oteryn:item.coin".into(),
                revision_ref: "definition-r1".into(),
            },
            quantity: 1,
        }
    }

    #[test]
    fn same_use_builds_the_same_child_occurrence() -> TestResult {
        let chest = resolved("oteryn:claim.a", CHEST_DEFINITION);
        let first = chest_use_occurrence(&request(1, "oteryn:chest.a")?, &chest)?;
        let again = chest_use_occurrence(&request(1, "oteryn:chest.a")?, &chest)?;
        assert_eq!(first, again);
        assert_eq!(first.ancestry_depth(), 1);
        Ok(())
    }

    #[test]
    fn chest_claim_and_command_each_change_the_child_occurrence() -> TestResult {
        let claim_a = resolved("oteryn:claim.a", CHEST_DEFINITION);
        let base = chest_use_occurrence(&request(1, "oteryn:chest.a")?, &claim_a)?;
        for (other, chest) in [
            (request(1, "oteryn:chest.b")?, claim_a.clone()),
            (
                request(1, "oteryn:chest.a")?,
                resolved("oteryn:claim.b", CHEST_DEFINITION),
            ),
            (request(2, "oteryn:chest.a")?, claim_a.clone()),
        ] {
            assert_ne!(base, chest_use_occurrence(&other, &chest)?);
        }
        Ok(())
    }

    #[test]
    fn the_placement_definition_is_part_of_the_child_occurrence() -> TestResult {
        let use_request = request(1, "oteryn:chest.a")?;
        assert_ne!(
            chest_use_occurrence(&use_request, &resolved("oteryn:claim.a", CHEST_DEFINITION))?,
            chest_use_occurrence(
                &use_request,
                &resolved(
                    "oteryn:claim.a",
                    "Item:oteryn:item.test.chest@definition-r2"
                )
            )?
        );
        Ok(())
    }

    #[test]
    fn another_edge_on_the_same_chest_is_another_occurrence() -> TestResult {
        // §5.5: same root, definition and target; only the typed edge differs.
        let use_request = request(1, "oteryn:chest.a")?;
        let chest = resolved("oteryn:claim.a", CHEST_DEFINITION);
        let used = chest_use_occurrence(&use_request, &chest)?;
        let stepped = chest_occurrence(&use_request, &chest.definition, &chest.claim, "STEP_IN")?;
        assert_ne!(used, stepped);
        assert_eq!(
            used,
            chest_occurrence(&use_request, &chest.definition, &chest.claim, USE_EDGE)?
        );
        Ok(())
    }

    #[test]
    fn revisions_are_part_of_the_child_and_must_be_present() -> TestResult {
        let chest = resolved("oteryn:claim.a", CHEST_DEFINITION);
        let base = request(1, "oteryn:chest.a")?;
        let mut later = base.clone();
        later.content_revision = "content-2".into();
        assert_ne!(
            chest_use_occurrence(&base, &chest)?,
            chest_use_occurrence(&later, &chest)?
        );
        let mut empty = base;
        empty.sim_revision = " ".into();
        assert_eq!(
            chest_use_occurrence(&empty, &chest),
            Err(InteractionError::EmptySemanticKey)
        );
        Ok(())
    }
}
