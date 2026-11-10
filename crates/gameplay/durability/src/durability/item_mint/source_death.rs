//! Closed completion-to-ItemMINT port; same physical writer as credited corpses.
use super::*;
use oteryn_gameplay_content::content::native_gameplay::NativeGameplayState;
use oteryn_gameplay_entities::carrier::native_source_death::SourceCreatureDeathCompletion;

impl DurabilityRoot {
    /// Caller retains the actual completion before releasing the current owner.
    /// SQL re-proves global recovery and current NodeScope in the existing core.
    #[allow(
        clippy::too_many_arguments,
        reason = "the closed source port retains the existing physical lane, recovery, node, content, death and candidate fences"
    )]
    pub async fn commit_uncredited_source_corpse_mint(
        &self,
        permit: &SpellLanePermit,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        native: &NativeGameplayState,
        completed: &SourceCreatureDeathCompletion,
        candidate: &mut ItemMintCandidate,
    ) -> Result<ItemMintOutcome> {
        let begin = completed.begin();
        let corpse_key = begin
            .uncredited_corpse_item_key()
            .ok_or(ItemMintError::InvalidInput)?;
        if native.source_digest() != begin.content_digest() {
            return Err(ItemMintError::InvalidInput);
        }
        let row = native
            .reward_table()
            .row(begin.definition_key())
            .map_err(|_| ItemMintError::InvalidInput)?;
        let corpse = row
            .corpse_item
            .as_ref()
            .ok_or(ItemMintError::InvalidInput)?;
        let expected = ItemMintCause::from_creature_death(
            completed.death_key(),
            TypedDefinitionRef {
                family: corpse.family.clone(),
                production_key: corpse.production_key.clone(),
                revision_ref: corpse.revision_ref.clone(),
            },
            CORPSE_MATERIALIZATION_PURPOSE_KEY.to_owned(),
            0,
        )?;
        let request = &candidate.request;
        let mut position = Vec::with_capacity(10);
        position.extend_from_slice(&begin.position().x.to_be_bytes());
        position.extend_from_slice(&begin.position().y.to_be_bytes());
        position.extend_from_slice(&begin.position().floor.to_be_bytes());
        let digest_text = |value: [u8; 32]| {
            format!(
                "sha256:{}",
                value
                    .iter()
                    .map(|byte| format!("{byte:02x}"))
                    .collect::<String>()
            )
        };
        if request.cause != expected
            || request.quantity != 1
            || request.item.family != corpse.family
            || request.item.production_key != corpse.production_key
            || request.item.revision_ref != corpse.revision_ref
            || corpse.production_key != corpse_key
            || request.ground.map_revision != digest_text(begin.map_digest())
            || request.ground.content_revision != digest_text(begin.content_digest())
            || request.content_revision != digest_text(begin.content_digest())
            || candidate.corpse_entry.is_some()
            || request.ground.spatial_position != position
            || request.ground.corpse_ref
                != super::super::party_kill_xp::encode_death_key(completed.death_key())
            || request.ground.native_room_placement_context != begin.frame_digest()
        {
            return Err(ItemMintError::InvalidInput);
        }
        self.commit_uncredited_corpse_mint_inner(permit, authority, node, candidate)
            .await
    }
}
