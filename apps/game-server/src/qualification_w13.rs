//! Disposable W13 hunter build and one-handed weapon fixture.
#[cfg(feature = "qualification-spawn")]
use crate::domain::CharacterId;
#[cfg(feature = "qualification-spawn")]
use crate::durability::DurabilityRoot;
#[cfg(feature = "qualification-spawn")]
use crate::durability::character_authority::ReconciledCharacterAuthority;
#[cfg(feature = "qualification-spawn")]
use crate::durability::character_build::{
    BuildCause, BuildChangeRequest, BuildOccurrence, DurableBuildState,
};
#[cfg(feature = "qualification-spawn")]
use crate::durability::character_progression::CurrentCharacterGameplayFence;
#[cfg(feature = "qualification-spawn")]
use crate::durability::character_revision_sequencer::CharacterRevisionSequencer;
#[cfg(feature = "qualification-spawn")]
use crate::durability::runtime_scope_assignment::NodeIncarnationProof;
#[cfg(feature = "qualification-spawn")]
use crate::foundation::{AuthenticatedTransportRefV1, GameSessionAuthoritySnapshot};
#[cfg(feature = "qualification-spawn")]
use crate::qualification_spell_character::{
    TestResult, authority_busy, debug, progression_busy, retry_holder,
};

pub(crate) const W13_HUNTER_SWORD_ID: u32 = 7383;

pub(crate) const W13_SWORD_SKILL: u16 = 110;

/// Seed the W13 Knight through the same fenced build writer that maintains the receipt chain.
#[cfg(feature = "qualification-spawn")]
pub(super) async fn prepare_w13_sword_skill(
    root: &DurabilityRoot,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    node: &NodeIncarnationProof,
    current: &GameSessionAuthoritySnapshot<AuthenticatedTransportRefV1>,
    content_revision: &str,
    occurrence: [u8; 16],
    deadline: tokio::time::Instant,
) -> TestResult {
    let character =
        CharacterId::from_bytes(*current.current_character_lease().character_id().as_bytes())
            .map_err(debug)?;
    let sequencer = CharacterRevisionSequencer::new();
    let mut slot = sequencer.acquire(character).await;
    let revision = retry_holder(Some(root), deadline, authority_busy, async || {
        root.read_current_character(authority, character).await
    })
    .await
    .map_err(debug)?
    .revision;
    let before = retry_holder(Some(root), deadline, progression_busy, async || {
        root.read_character_build_state(authority, character).await
    })
    .await
    .map_err(debug)?;
    const SWORD: usize = 2;
    let mut skills = before.skills();
    let occurrence = BuildOccurrence::from_bytes(occurrence).map_err(debug)?;
    if before.vocation() == "knight" && skills[SWORD] == (W13_SWORD_SKILL, 0) {
        let receipt = retry_holder(Some(root), deadline, progression_busy, async || {
            root.reconcile_character_build(authority, occurrence).await
        })
        .await
        .map_err(debug)?;
        if receipt.is_some_and(|receipt| {
            receipt.character_id == character
                && receipt.cause == BuildCause::Training
                && receipt.after == before
        }) {
            return Ok(());
        }
    }
    if before.vocation() != "knight" || skills[SWORD] != (10, 0) {
        return Err("W13 sword fixture requires an untrained Knight".into());
    }
    skills[SWORD] = (W13_SWORD_SKILL, 0);
    let after = DurableBuildState::new(before.vocation(), before.magic(), skills).map_err(debug)?;
    let request = BuildChangeRequest {
        occurrence,
        cause: BuildCause::Training,
        before,
        after,
        pruned_stance: None,
    };
    let fence = CurrentCharacterGameplayFence {
        character_id: character,
        game_session_id: current.current_game_session_id(),
        connection_generation: current.current_connection_generation(),
        character_lease_generation: current.current_character_lease().generation(),
        runtime_scope: current.current_runtime_scope(),
        scope_ownership_generation: current.current_scope_generation(),
        expected_character_revision: revision,
    };
    let formula = crate::spell::mana_training::CompiledTrainingFormula::from_profile(
        include_bytes!("../../../tools/content-schema/native-gameplay/build-training.json"),
        content_revision,
    )
    .map_err(debug)?;
    retry_holder(Some(root), deadline, progression_busy, async || {
        slot.commit_build(
            root,
            authority,
            node,
            fence,
            request.clone(),
            &formula,
            None,
        )
        .await
    })
    .await
    .map_err(debug)?;
    Ok(())
}
