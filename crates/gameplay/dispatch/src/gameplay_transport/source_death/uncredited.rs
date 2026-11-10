//! Retain the canonical completion through the existing fenced empty-corpse writer.
use super::*;
use crate::durability::item_mint::{
    CORPSE_MATERIALIZATION_PURPOSE_KEY, CommittedItemMint, GroundPlacement, ItemMintCandidate,
    ItemMintCause, ItemMintOutcome, ItemMintRequest, TypedDefinitionRef,
};
use oteryn_gameplay_entities::carrier::native_source_death::{
    SourceCreatureDeathCompletion, SourceCreatureDeathSeal,
};

#[derive(Debug)]
pub(in crate::gameplay_transport) struct PendingSourceCorpse {
    completed: SourceCreatureDeathCompletion,
    request: ItemMintRequest,
    candidate: Option<ItemMintCandidate>,
    committed: Option<CommittedItemMint>,
}

impl PendingSourceCorpse {
    pub(super) fn request(
        native: &crate::content::native_gameplay::NativeGameplayState,
        begin: &SourceCreatureDeathSeal,
    ) -> Result<ItemMintRequest, ()> {
        if begin.definition_key() != "oteryn:creature.outburst"
            || begin.has_reward_damage_credit()
            || native.source_digest() != begin.content_digest()
            || !begin
                .events()
                .iter()
                .any(|event| event.name() == "HeartBossDeath")
        {
            return Err(());
        }
        let row = native
            .reward_table()
            .row(begin.definition_key())
            .map_err(|_| ())?;
        let corpse = row.corpse_item.as_ref().ok_or(())?;
        if corpse.production_key != "oteryn:item.tibia.i23564" {
            return Err(());
        }
        let item = TypedDefinitionRef {
            family: corpse.family.clone(),
            production_key: corpse.production_key.clone(),
            revision_ref: corpse.revision_ref.clone(),
        };
        let cause = ItemMintCause::from_creature_death(
            begin.death_key(),
            item.clone(),
            CORPSE_MATERIALIZATION_PURPOSE_KEY.into(),
            0,
        )
        .map_err(|_| ())?;
        let digest_text = |digest: [u8; 32]| {
            format!(
                "sha256:{}",
                digest
                    .iter()
                    .map(|byte| format!("{byte:02x}"))
                    .collect::<String>()
            )
        };
        let at = begin.position();
        let mut spatial_position = Vec::with_capacity(10);
        spatial_position.extend_from_slice(&at.x.to_be_bytes());
        spatial_position.extend_from_slice(&at.y.to_be_bytes());
        spatial_position.extend_from_slice(&at.floor.to_be_bytes());
        Ok(ItemMintRequest {
            cause,
            item,
            quantity: 1,
            ground: GroundPlacement {
                spatial_position,
                corpse_ref: crate::durability::party_kill_xp::encode_death_key(begin.death_key()),
                map_revision: digest_text(begin.map_digest()),
                content_revision: digest_text(begin.content_digest()),
                native_room_placement_context: begin.frame_digest().to_vec(),
            },
            content_revision: digest_text(begin.content_digest()),
            ruleset_revision: crate::interaction_chest_use::entry_chest::RULESET_REVISION.into(),
            sim_revision: crate::interaction_chest_use::entry_chest::SIM_REVISION.into(),
        })
    }

    pub(super) fn new(completed: SourceCreatureDeathCompletion, request: ItemMintRequest) -> Self {
        Self {
            completed,
            request,
            candidate: None,
            committed: None,
        }
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    /// `true` means this owner turn belongs to retained corpse work, including an Unknown result.
    pub(in crate::gameplay_transport) async fn drain_uncredited_source_corpse(
        &self,
        permit: &crate::durability::spell_owner_commit::SpellLanePermit,
    ) -> bool {
        let retained = {
            self.states
                .spell_states
                .lock()
                .await
                .source_death_corpse
                .clone()
        };
        let Some(retained) = retained else {
            return false;
        };
        // The Arc stays in Channel state across cancellation; the non-Clone candidate keeps
        // its original durable identities and writer budget across every retry.
        let mut pending = retained.lock().await;
        let Some(native) = self
            .active_generation
            .and_then(|active| active.native_gameplay())
        else {
            return true;
        };
        {
            let runtime = self.runtime.lock().await;
            if !pending.completed.current_for(&runtime)
                || native.source_digest() != pending.completed.begin().content_digest()
            {
                return true;
            }
        }
        if pending.candidate.is_none() {
            let candidate = self
                .root
                .freeze_item_mint(self.character, self.holder, pending.request.clone())
                .await;
            let Ok(candidate) = candidate else {
                return true;
            };
            pending.candidate = Some(candidate);
        }
        if pending.committed.is_none() {
            let PendingSourceCorpse {
                completed,
                candidate,
                ..
            } = &mut *pending;
            let Some(candidate) = candidate.as_mut() else {
                return true;
            };
            let outcome = self
                .root
                .commit_uncredited_source_corpse_mint(
                    permit,
                    self.character,
                    self.holder,
                    native,
                    completed,
                    candidate,
                )
                .await;
            let Ok(
                ItemMintOutcome::Committed(committed)
                | ItemMintOutcome::AlreadyCommitted(committed),
            ) = outcome
            else {
                return true;
            };
            pending.committed = Some(committed);
        }
        let Some(committed) = pending.committed.as_ref() else {
            return true;
        };
        let mut runtime = self.runtime.lock().await;
        if !pending.completed.current_for(&runtime)
            || runtime
                .bind_corpse_item(
                    pending.completed.actor(),
                    committed.item_instance_id,
                    &pending.request.item.family,
                    &pending.request.item.production_key,
                    &pending.request.item.revision_ref,
                )
                .is_err()
        {
            return true;
        }
        let Ok(mut states) = self.states.spell_states.try_lock() else {
            return true;
        };
        if !states
            .source_death_corpse
            .as_ref()
            .is_some_and(|current| std::sync::Arc::ptr_eq(current, &retained))
        {
            return true;
        }
        if states
            .native_callbacks
            .acknowledge_outburst_null_death(&runtime, &pending.completed)
            .is_err()
        {
            return true;
        }
        if runtime
            .release_uncredited_source_corpse_projection(&pending.completed)
            .is_err()
        {
            return true;
        }
        self.states.loop_ground.invalidate();
        states.source_death_corpse = None;
        true
    }
}
