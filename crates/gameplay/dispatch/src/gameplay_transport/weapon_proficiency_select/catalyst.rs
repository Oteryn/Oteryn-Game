//! P08 Proficiency Catalysts used on a carried weapon: one unit burns and the weapon's track
//! gains the catalyst's progress, capped at its final progress, in one durable write
//! (`commit_proficiency_catalyst`). A maxed track refuses before anything is spent.
use super::*;
use crate::durability::item_use::proficiency_catalyst::catalyst_progress;
use crate::durability::item_use::{ItemUseFlaskPlan, ItemUseKind, ItemUseRequest};
use crate::gameplay_transport::connection::UseOutcome;
use crate::gameplay_transport::item_view::ItemKey;
use oteryn_protocol_oteryn::world_object::UseDisposition;

fn disposition(disposition: UseDisposition) -> Option<UseOutcome> {
    Some(UseOutcome {
        disposition,
        ..UseOutcome::rejected()
    })
}

/// A UUIDv7-shaped identity of one catalyst use, stable across retries of its CommandRef.
fn derived(
    label: &[u8],
    actor: ExactActorRef,
    session: GameSessionId,
    command_id: u64,
) -> [u8; 16] {
    let digest = Sha256::new()
        .chain_update(label)
        .chain_update(actor.placement_identity())
        .chain_update(session.as_bytes())
        .chain_update(command_id.to_be_bytes())
        .finalize();
    let mut bytes = [0_u8; 16];
    bytes.copy_from_slice(&digest[..16]);
    bytes[6] = (bytes[6] & 15) | 0x70;
    bytes[8] = (bytes[8] & 63) | 0x80;
    bytes
}

/// `None` when the source is not a carried catalyst, so other use-with owners may answer.
#[allow(clippy::too_many_lines)]
pub(in crate::gameplay_transport) async fn use_catalyst(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    catalyst: ItemKey,
    weapon: ItemKey,
) -> Option<UseOutcome> {
    let (ItemKey::Instance(catalyst), ItemKey::Instance(weapon)) = (catalyst, weapon) else {
        return None;
    };
    let session = command.game_session_id;
    let fence = command.item_fence?;
    let catalogue = owner.proficiency_catalogue.as_ref()?;
    let inventory = owner
        .root
        .read_item_use_inventory(owner.character, owner.holder, fence)
        .await
        .ok()?;
    let carried = || {
        inventory
            .backpack
            .iter()
            .flat_map(|bag| &bag.entries)
            .chain(inventory.nested.iter().map(|(_, entry)| entry))
            .map(|entry| &entry.item)
    };
    let source = carried().find(|item| item.item_instance_id == catalyst)?;
    let amount = catalyst_progress(&source.definition.production_key)?;
    let Some(target) = carried()
        .chain(&inventory.equipment)
        .find(|item| item.item_instance_id == weapon)
    else {
        return disposition(UseDisposition::NotPossible);
    };
    let binding = owner.progression_binding(session)?;
    let definitions =
        catalogue.for_content(&binding.context.content) as Arc<dyn ProficiencyDefinitions>;
    // Crystal proficiency_catalysts.lua:9-16: only a weapon with a proficiency takes it.
    let Some(resolved) = definitions.resolve_weapon(&target.definition.production_key) else {
        return disposition(UseDisposition::NotPossible);
    };
    if !owner.states.player_trades.physical(session, Some(catalyst)) {
        return disposition(UseDisposition::Rejected);
    }
    let occurrence = ProficiencyOccurrence::from_bytes(derived(
        b"oteryn:proficiency-catalyst:v1",
        actor,
        session,
        command.command_id,
    ))
    .ok()?;
    // Uncheckpointed kill progress is committed first, so the catalyst adds to it.
    match owner.save_weapon_proficiency(actor, session, true).await {
        TrainingSave::Saved | TrainingSave::NotApplicable => {}
        TrainingSave::FencedOut => return None,
        TrainingSave::Unknown => return disposition(UseDisposition::Rejected),
    }
    let current = owner.current_quest_fence(session).await.ok()??;
    let mut slot = owner.revision_sequencer.acquire(current.character_id).await;
    let root = owner
        .root
        .read_current_character(owner.character, current.character_id)
        .await
        .ok()?;
    let gameplay = CurrentCharacterGameplayFence {
        expected_character_revision: root.revision,
        ..current
    };
    let (tracks, receipt) = owner
        .root
        .read_character_proficiency_with_definitions(
            owner.character,
            gameplay.character_id,
            Some(occurrence),
            Arc::clone(&definitions),
        )
        .await
        .ok()?;
    if let Some(receipt) = receipt {
        return disposition(if receipt.cause == ProficiencyCause::Catalyst {
            UseDisposition::Committed
        } else {
            UseDisposition::Rejected
        });
    }
    let item_key = resolved.canonical_item_key;
    let shape = resolved.shape.track_shape();
    let before = match tracks
        .into_iter()
        .find(|track| track.state.item_key() == item_key)
    {
        Some(track) => track.state,
        None => DurableProficiencyState::new(
            item_key.clone(),
            resolved.definition_key,
            resolved.definition_revision,
            0,
            vec![None; usize::from(shape.perk_levels())],
        )
        .ok()?,
    };
    let cap = u64::from(shape.final_progress());
    if before.progress() >= cap {
        return disposition(UseDisposition::NotPossible);
    }
    let after = DurableProficiencyState::new(
        item_key.clone(),
        before.definition_key(),
        before.definition_revision(),
        before.progress().saturating_add(u64::from(amount)).min(cap),
        before.selections().to_vec(),
    )
    .ok()?;
    let change = ProficiencyChangeRequest::new(
        occurrence,
        ProficiencyCause::Catalyst,
        vec![ProficiencyLineCandidate::new(ProficiencyCause::Catalyst, before, after).ok()?],
        None,
        definitions.digest(),
    )
    .ok()?;
    let occurred_at_unix_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .and_then(|elapsed| i64::try_from(elapsed.as_millis()).ok())?;
    let item = ItemUseRequest {
        kind: ItemUseKind::ProficiencyCatalyst,
        command_id: command.command_id,
        content_revision: inventory.content_revision.clone(),
        ruleset_revision: inventory.ruleset_revision.clone(),
        sim_revision: crate::interaction_chest_use::entry_chest::SIM_REVISION.to_owned(),
        transaction_id: *occurrence.as_bytes(),
        event_id: derived(
            b"oteryn:proficiency-catalyst-event:v1",
            actor,
            session,
            command.command_id,
        ),
        occurred_at_unix_ms,
        server_build_id: crate::durability::character_authority::SERVER_BUILD_ID.to_owned(),
        source_item_instance_id: catalyst,
        source_definition: source.definition.clone(),
        source_quantity_before: source.quantity,
        flask: ItemUseFlaskPlan::None,
    };
    let outcome = slot
        .commit_proficiency_catalyst(
            owner.root,
            owner.character,
            owner.holder,
            gameplay,
            item,
            change.clone(),
            Arc::clone(&definitions),
            Some(gameplay.expected_character_revision),
        )
        .await;
    let receipt = match outcome {
        Ok(
            ProficiencyCommitOutcome::Committed(receipt)
            | ProficiencyCommitOutcome::AlreadyCommitted(receipt),
        ) => receipt,
        Ok(ProficiencyCommitOutcome::Refused(ProficiencyWriteRefusal::CatalystMoved)) => {
            return disposition(UseDisposition::StaleState);
        }
        Ok(ProficiencyCommitOutcome::Refused(_))
        | Err(
            CharacterProgressionError::CharacterRevisionMismatch
            | CharacterProgressionError::ConflictingOccurrence,
        ) => return disposition(UseDisposition::Rejected),
        // An unknown outcome is reconciled by the occurrence; still unknown disconnects.
        Err(_) => owner
            .root
            .reconcile_character_proficiency_with_definitions(
                owner.character,
                gameplay.character_id,
                gameplay.expected_character_revision,
                change,
                Some(Arc::clone(&definitions)),
            )
            .await
            .ok()??,
    };
    drop(slot);
    let committed = StoredProficiencyTrack {
        state: receipt.lines.first()?.after().clone(),
        committed_character_revision: receipt.committed_character_revision,
        last_proficiency_occurrence_id: receipt.occurrence_id,
    };
    if let Some(track) = catalogue.live_track(&committed)
        && let Ok(mut states) = owner.states.proficiencies.lock()
    {
        states
            .entry(session)
            .or_default()
            .install_track(committed.state.item_key(), track);
    }
    disposition(UseDisposition::Committed)
}
