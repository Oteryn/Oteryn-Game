//! Closed completion-to-ItemMINT port; same physical writer as credited corpses.
use super::*;
use oteryn_gameplay_content::content::native_gameplay::NativeGameplayState;
use oteryn_gameplay_entities::carrier::native_source_death::{
    SourceCreatureDeathCompletion, SourceCreatureDeathSeal,
};
use oteryn_simulation_determinism::{
    DecisionOccurrenceId, GameplayDecisionRoot, deterministic_decision_u64,
};
use sha2::{Digest, Sha256};

/// Immutable result of the actual copied Bakragore handler. The dispatch owner
/// supplies its retained storage88888 value; neither a caller-selected Item nor
/// a later read of public essence flags can become this physical capability.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BakragoreDeathLoot {
    begin: SourceCreatureDeathSeal,
    essences: u8,
    chance_roll: Option<u8>,
    selection_roll: Option<u8>,
    source_item: Option<u32>,
    item: Option<TypedDefinitionRef>,
}

impl BakragoreDeathLoot {
    pub fn from_current_death(
        runtime: &crate::foundation::ChannelRuntimeV1,
        native: &NativeGameplayState,
        begin: &SourceCreatureDeathSeal,
        captured_essences: u8,
    ) -> Result<Self> {
        if !begin.current_for(runtime)
            || captured_essences > 4
            || !native.qualifies_rotten_blood_callbacks(begin.content_digest())
            || begin.definition_key() != "oteryn:creature.bakragore"
            || begin.events().len() != 1
            || begin.events()[0].name() != "BakragoreMechanicsDeath"
            || begin.events()[0].registration_index() != 1
            || runtime
                .source_creature_death_event_index(begin)
                .ok()
                .flatten()
                != Some(0)
        {
            return Err(ItemMintError::InvalidInput);
        }
        let row = native
            .reward_table()
            .row("oteryn:creature.bakragore")
            .map_err(|_| ItemMintError::InvalidInput)?;
        if row.corpse_item.as_ref().is_none_or(|item| {
            item.family != "Item" || item.production_key != "oteryn:item.tibia.i44012"
        }) || row.loot_table_ref.is_none()
        {
            return Err(ItemMintError::InvalidInput);
        }
        let mut result = Self {
            begin: begin.clone(),
            essences: captured_essences,
            chance_roll: None,
            selection_roll: None,
            source_item: None,
            item: None,
        };
        if captured_essences == 0 {
            return Ok(result);
        }
        let root = GameplayDecisionRoot::from_bytes(
            Sha256::new()
                .chain_update(b"srx3:bakragore:copied-death-loot:v1")
                .chain_update(native.source_digest())
                .chain_update(begin.actor().placement_identity())
                .finalize()
                .into(),
        );
        let hash = Sha256::new()
            .chain_update(super::super::party_kill_xp::encode_death_key(
                begin.death_key(),
            ))
            .chain_update(begin.events()[0].registration_index().to_be_bytes())
            .finalize();
        let occurrence = DecisionOccurrenceId::from_bytes(
            hash[..16]
                .try_into()
                .map_err(|_| ItemMintError::InvalidInput)?,
        );
        let (chance_roll, selection_roll, id) =
            roll_source_loot(&root, occurrence, captured_essences)?;
        result.chance_roll = chance_roll;
        result.selection_roll = selection_roll;
        let Some(id) = id else {
            return Ok(result);
        };
        let item = native
            .reward_table()
            .admitted_source_item(id)
            .ok_or(ItemMintError::InvalidInput)?;
        if item.family != "Item" || item.production_key != format!("oteryn:item.tibia.i{id}") {
            return Err(ItemMintError::InvalidInput);
        }
        result.source_item = Some(id);
        result.item = Some(TypedDefinitionRef {
            family: item.family.clone(),
            production_key: item.production_key.clone(),
            revision_ref: item.revision_ref.clone(),
        });
        Ok(result)
    }

    pub fn matches_source(&self, begin: &SourceCreatureDeathSeal) -> bool {
        &self.begin == begin
    }
    pub fn source_item_id(&self) -> Option<u32> {
        self.source_item
    }
    pub fn selected_item(&self) -> Option<&TypedDefinitionRef> {
        self.item.as_ref()
    }
    pub fn captured_essences(&self) -> u8 {
        self.essences
    }
    pub fn chance_roll(&self) -> Option<u8> {
        self.chance_roll
    }
    pub fn selection_roll(&self) -> Option<u8> {
        self.selection_roll
    }

    pub fn current_for(
        &self,
        runtime: &crate::foundation::ChannelRuntimeV1,
        native: &NativeGameplayState,
    ) -> bool {
        self.begin.current_for(runtime)
            && runtime
                .source_creature_death_event_index(&self.begin)
                .ok()
                .flatten()
                == Some(0)
            && self.matches_native(native)
    }

    fn matches_native(&self, native: &NativeGameplayState) -> bool {
        if !native.qualifies_rotten_blood_callbacks(self.begin.content_digest()) {
            return false;
        }
        match (&self.item, self.source_item) {
            (None, None) => true,
            (Some(item), Some(id)) => {
                native
                    .reward_table()
                    .admitted_source_item(id)
                    .is_some_and(|current| {
                        current.family == item.family
                            && current.production_key == item.production_key
                            && current.revision_ref == item.revision_ref
                    })
            }
            _ => false,
        }
    }

    /// Reserve the optional one-unit child alongside its actual corpse
    /// candidate. The final writer rechecks this complete immutable request.
    pub fn uncredited_child_request(
        &self,
        native: &NativeGameplayState,
        completed: &SourceCreatureDeathCompletion,
        corpse: &ItemMintCandidate,
    ) -> Result<Option<CorpseLootMintRequest>> {
        if !self.matches_source(completed.begin()) || !self.matches_native(native) {
            return Err(ItemMintError::InvalidInput);
        }
        validate_source_corpse_candidate(native, completed, corpse)?;
        let Some(item) = self.item.as_ref() else {
            return Ok(None);
        };
        let row = native
            .reward_table()
            .row("oteryn:creature.bakragore")
            .map_err(|_| ItemMintError::InvalidInput)?;
        let table = row
            .loot_table_ref
            .as_ref()
            .ok_or(ItemMintError::InvalidInput)?;
        Ok(Some(CorpseLootMintRequest {
            cause: ItemMintCause::from_creature_death(
                completed.death_key(),
                TypedDefinitionRef {
                    family: table.family.clone(),
                    production_key: table.production_key.clone(),
                    revision_ref: table.revision_ref.clone(),
                },
                item.production_key.clone(),
                0,
            )?,
            item: item.clone(),
            quantity: 1,
            placement: CorpseContainerPlacement {
                corpse_item_instance_id: *corpse.item_instance_id(),
                placement_ordinal: 1,
                fluid_subtype: None,
            },
            content_revision: corpse.request.content_revision.clone(),
            ruleset_revision: corpse.request.ruleset_revision.clone(),
            sim_revision: corpse.request.sim_revision.clone(),
        }))
    }
}

type SourceLootRoll = (Option<u8>, Option<u8>, Option<u32>);

fn roll_source_loot(
    root: &GameplayDecisionRoot,
    occurrence: DecisionOccurrenceId,
    essences: u8,
) -> Result<SourceLootRoll> {
    if essences > 4 {
        return Err(ItemMintError::InvalidInput);
    }
    if essences == 0 {
        return Ok((None, None, None));
    }
    let chance_roll = source_draw(root, occurrence, 0, 100)?
        .checked_add(1)
        .ok_or(ItemMintError::InvalidInput)?;
    if chance_roll > [0, 5, 8, 12, 15][usize::from(essences)] {
        return Ok((Some(chance_roll), None, None));
    }
    let selection = source_draw(root, occurrence, 1, 2)?;
    Ok((
        Some(chance_roll),
        Some(selection + 1),
        Some([43_895, 43_860][usize::from(selection)]),
    ))
}

fn source_draw(
    root: &GameplayDecisionRoot,
    occurrence: DecisionOccurrenceId,
    ordinal: u64,
    bound: u64,
) -> Result<u8> {
    let limit = u64::MAX - u64::MAX % bound;
    for attempt in 0..64 {
        let value = deterministic_decision_u64(
            root,
            occurrence,
            "bakragore_source_death",
            ordinal * 64 + attempt,
        )
        .map_err(|_| ItemMintError::InvalidInput)?;
        if value < limit {
            return u8::try_from(value % bound).map_err(|_| ItemMintError::InvalidInput);
        }
    }
    Err(ItemMintError::InvalidInput)
}

/// Only the exact source facades below can authorize a physical corpse without
/// a damage principal. A selected child additionally needs the retained Bak draw.
pub(super) struct SourceCorpseMintProof<'a> {
    native: &'a NativeGameplayState,
    completed: &'a SourceCreatureDeathCompletion,
    loot: Option<&'a BakragoreDeathLoot>,
}

impl SourceCorpseMintProof<'_> {
    pub(super) fn freeze(
        &self,
        corpse: &ItemMintCandidate,
        loot: &[ItemMintCandidate],
    ) -> Result<FrozenSourceCorpseProof> {
        validate_source_corpse_candidate(self.native, self.completed, corpse)?;
        let begin = self.completed.begin();
        if self.loot.is_some() != (begin.definition_key() == "oteryn:creature.bakragore") {
            return Err(ItemMintError::InvalidInput);
        }
        let expected = self
            .loot
            .map(|source| source.uncredited_child_request(self.native, self.completed, corpse))
            .transpose()?
            .flatten();
        if loot.len() != usize::from(expected.is_some()) {
            return Err(ItemMintError::InvalidInput);
        }
        if let Some(expected) = expected {
            let child = &loot[0];
            let request = &child.request;
            if child.corpse_entry != Some(expected.placement)
                || request.cause != expected.cause
                || request.item != expected.item
                || request.quantity != 1
                || request.content_revision != expected.content_revision
                || request.ruleset_revision != expected.ruleset_revision
                || request.sim_revision != expected.sim_revision
                || request.ground
                    != (GroundPlacement {
                        spatial_position: Vec::new(),
                        corpse_ref: Vec::new(),
                        map_revision: String::new(),
                        content_revision: String::new(),
                        native_room_placement_context: Vec::new(),
                    })
            {
                return Err(ItemMintError::InvalidInput);
            }
        }
        let hex = |bytes: &[u8]| {
            bytes
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        };
        let events = begin
            .events()
            .iter()
            .map(|event| {
                serde_json::json!({
                    "registration_index": event.registration_index(),
                    "name": event.name(),
                    "source_path": event.source_path(),
                    "source_sha256": hex(&event.source_sha256()),
                })
            })
            .collect::<Vec<_>>();
        let selected = self.loot.map(|source| {
            serde_json::json!({
                "essences": source.captured_essences(),
                "chance_roll": source.chance_roll(),
                "selection_roll": source.selection_roll(),
                "source_item": source.source_item_id(),
            })
        });
        let proof = serde_json::json!({
            "schema": "OTERYN_SOURCE_CORPSE_MINT/v1",
            "creature_key": begin.definition_key(),
            "creature_revision": begin.definition_revision(),
            "events": events,
            "has_reward_damage_credit": begin.has_reward_damage_credit(),
            "death": hex(&super::super::party_kill_xp::encode_death_key(begin.death_key())),
            "content": hex(&begin.content_digest()),
            "client": hex(&begin.client_digest()),
            "map": hex(&begin.map_digest()),
            "frame": hex(&begin.frame_digest()),
            "position": hex(&corpse.request.ground.spatial_position),
            "bakragore_loot": selected,
        });
        Ok(FrozenSourceCorpseProof {
            proof: serde_json::to_vec(&proof).map_err(|_| ItemMintError::InvalidInput)?,
        })
    }
}

/// Immutable DATA attached only by the sealed completion facade. SQL proves
/// the current scope and pins independently, in the physical MINT transaction.
pub(super) struct FrozenSourceCorpseProof {
    proof: Vec<u8>,
}

impl FrozenSourceCorpseProof {
    pub(super) async fn matches_committed(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        corpse: &FrozenMint,
    ) -> std::result::Result<bool, DurabilityError> {
        Ok(sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM game_source_corpse_mints \
             WHERE transaction_id=encode($1,'hex')::uuid \
               AND item_instance_id=encode($2,'hex')::uuid AND proof=$3)",
        )
        .bind(corpse.transaction_id.as_slice())
        .bind(corpse.item_instance_id.as_slice())
        .bind(self.proof.as_slice())
        .fetch_one(&mut **tx)
        .await?)
    }

    pub(super) async fn insert(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        corpse: &FrozenMint,
    ) -> std::result::Result<(), DurabilityError> {
        sqlx::query(
            "INSERT INTO game_source_corpse_mints(transaction_id,item_instance_id,proof,binding) \
             VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,sha256($3))",
        )
        .bind(corpse.transaction_id.as_slice())
        .bind(corpse.item_instance_id.as_slice())
        .bind(self.proof.as_slice())
        .execute(&mut **tx)
        .await?;
        Ok(())
    }
}

impl DurabilityRoot {
    /// The source's one selected child shares its corpse transaction. Unknown
    /// outcomes retain both original candidates; no independent Ground MINT or
    /// fabricated reward principal is accepted here.
    #[allow(
        clippy::too_many_arguments,
        reason = "the exact source completion and retained draw supplement the existing physical lane and two candidates"
    )]
    pub async fn commit_uncredited_bakragore_corpse_loot(
        &self,
        permit: &SpellLanePermit,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        native: &NativeGameplayState,
        completed: &SourceCreatureDeathCompletion,
        source: &BakragoreDeathLoot,
        corpse: &mut ItemMintCandidate,
        loot: &mut [ItemMintCandidate],
    ) -> Result<Vec<ItemMintOutcome>> {
        let proof = SourceCorpseMintProof {
            native,
            completed,
            loot: Some(source),
        };
        self.commit_uncredited_corpse_loot_inner(permit, authority, node, corpse, loot, proof)
            .await
    }

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
        let proof = SourceCorpseMintProof {
            native,
            completed,
            loot: None,
        };
        self.commit_uncredited_corpse_mint_inner(permit, authority, node, candidate, proof)
            .await
    }
}

fn validate_source_corpse_candidate(
    native: &NativeGameplayState,
    completed: &SourceCreatureDeathCompletion,
    candidate: &ItemMintCandidate,
) -> Result<()> {
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
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copied_bakragore_draw_preserves_each_source_threshold_and_single_selection() -> Result<()> {
        let occurrence = DecisionOccurrenceId::from_bytes([5; 16]);
        let root = |seed| GameplayDecisionRoot::from_bytes([seed; 32]);
        // Actual deterministic-decision trace: both literal source Items occur,
        // and a failed probability draw must not acquire a selection draw.
        assert_eq!(
            roll_source_loot(&root(41), occurrence, 1)?,
            (Some(1), Some(2), Some(43_860))
        );
        assert_eq!(
            roll_source_loot(&root(78), occurrence, 1)?,
            (Some(5), Some(1), Some(43_895))
        );
        assert_eq!(
            roll_source_loot(&root(99), occurrence, 1)?,
            (Some(6), None, None)
        );
        assert_eq!(
            roll_source_loot(&root(99), occurrence, 2)?,
            (Some(6), Some(2), Some(43_860))
        );
        assert_eq!(
            roll_source_loot(&root(63), DecisionOccurrenceId::from_bytes([6; 16]), 2)?,
            (Some(8), Some(2), Some(43_860))
        );
        assert_eq!(
            roll_source_loot(&root(7), occurrence, 2)?,
            (Some(9), None, None)
        );
        assert_eq!(
            roll_source_loot(&root(7), occurrence, 3)?,
            (Some(9), Some(2), Some(43_860))
        );
        assert_eq!(
            roll_source_loot(&root(41), DecisionOccurrenceId::from_bytes([6; 16]), 3)?,
            (Some(12), Some(2), Some(43_860))
        );
        assert_eq!(
            roll_source_loot(&root(255), occurrence, 3)?,
            (Some(13), None, None)
        );
        let selected = roll_source_loot(&root(57), occurrence, 4)?;
        assert_eq!(selected, (Some(15), Some(1), Some(43_895)));
        assert_eq!(roll_source_loot(&root(57), occurrence, 4)?, selected);
        assert_eq!(
            roll_source_loot(&root(169), occurrence, 4)?,
            (Some(16), None, None)
        );
        assert_eq!(
            roll_source_loot(&root(41), occurrence, 0)?,
            (None, None, None)
        );
        assert!(roll_source_loot(&root(41), occurrence, 5).is_err());
        Ok(())
    }
}
