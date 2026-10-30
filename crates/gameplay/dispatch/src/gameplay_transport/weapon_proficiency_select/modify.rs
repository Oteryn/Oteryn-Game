//! P08 commands 379, 380 and 392-395: the six perk modification commands of PROFICIENCY-1B
//! §11.1 through the `perk_modification` writer, with the Global shaping values of
//! PROFICIENCY-1C. Checks follow 1B §5: replay by occurrence first, then the `PROF1B-RL-03`
//! rate cap for an unseen occurrence only, then the writer's own checks under the cursor.
//! `ORB_RANK` reserves the first carried Lunar Ascension Orb not in a trade (1B §5: main
//! backpack entries, then nested containers breadth first; an orb is never equipped) and the
//! writer burns it in the receipt's transaction (1C §5).
use super::*;
use crate::durability::item_use::proficiency_orb::LUNAR_ASCENSION_ORB;
use crate::durability::item_use::{ItemUseFlaskPlan, ItemUseKind, ItemUseRequest};
use crate::durability::character_proficiency_modification::{
    ProficiencyModificationBoundRevisions, ProficiencyModificationCommand,
    ProficiencyModificationContext, ProficiencyModificationOutcome, ProficiencyShapingSource,
    proficiency_shaping_key,
};
use crate::gameplay_transport::weapon_proficiency::ProductionProficiencyShaping;
use oteryn_gameplay_domain::domain::CharacterRevision;
use oteryn_gameplay_domain::domain::weapon_proficiency::{
    ProficiencyModificationCommandKind, ProficiencyModificationResult,
};
use oteryn_protocol_oteryn::weapon_proficiency::{
    ModificationOperation, ModificationReply, ModificationRequest, ModificationResult,
    ModifiedSlot,
};

const fn kind(operation: ModificationOperation) -> ProficiencyModificationCommandKind {
    match operation {
        ModificationOperation::Modify { level } => ProficiencyModificationCommandKind::Modify { level },
        ModificationOperation::RankUp => ProficiencyModificationCommandKind::RankUp,
        ModificationOperation::OrbRank => ProficiencyModificationCommandKind::OrbRank,
        ModificationOperation::ReshapeOffer => ProficiencyModificationCommandKind::ReshapeOffer,
        ModificationOperation::ReshapeChoose { choice } => {
            ProficiencyModificationCommandKind::ReshapeChoose { choice }
        }
        ModificationOperation::Clear => ProficiencyModificationCommandKind::Clear,
    }
}

const fn result(result: ProficiencyModificationResult) -> ModificationResult {
    use ProficiencyModificationResult as Domain;
    match result {
        Domain::Accepted => ModificationResult::Accepted,
        Domain::NotAdmitted => ModificationResult::NotAdmitted,
        Domain::NotUnlocked => ModificationResult::NotUnlocked,
        Domain::NotInProtectionZone => ModificationResult::NotInProtectionZone,
        Domain::UnknownTrack => ModificationResult::UnknownTrack,
        Domain::StaleRevision => ModificationResult::StaleRevision,
        Domain::RevisionChanged => ModificationResult::RevisionChanged,
        Domain::RateLimited => ModificationResult::RateLimited,
        Domain::Conflict => ModificationResult::Conflict,
        Domain::NoSelection => ModificationResult::NoSelection,
        Domain::SlotOccupied => ModificationResult::SlotOccupied,
        Domain::LevelTaken => ModificationResult::LevelTaken,
        Domain::NotModified => ModificationResult::NotModified,
        Domain::RankMax => ModificationResult::RankMax,
        Domain::OfferPending => ModificationResult::OfferPending,
        Domain::NoOffer => ModificationResult::NoOffer,
        Domain::PoolTooSmall => ModificationResult::PoolTooSmall,
        Domain::ShapingOutdated => ModificationResult::ShapingOutdated,
        Domain::InsufficientDust => ModificationResult::InsufficientDust,
        Domain::NoOrb => ModificationResult::NoOrb,
    }
}

/// The result of an outcome, with the commanded slot after an accepted command.
fn reply(outcome: &ProficiencyModificationOutcome) -> ModificationReply {
    let slot = match outcome {
        ProficiencyModificationOutcome::Committed(committed)
        | ProficiencyModificationOutcome::AlreadyCommitted(committed) => {
            committed.after.as_ref().map(|after| ModifiedSlot {
                level: after.level,
                entry_index: after.entry_index,
                rank: after.rank,
                pending_offer: after.pending_offer,
            })
        }
        _ => None,
    };
    ModificationReply {
        result: result(outcome.result()),
        slot,
    }
}

const fn refused(result: ModificationResult) -> Option<ModificationReply> {
    Some(ModificationReply { result, slot: None })
}

/// A UUIDv7-shaped identity of one modification, bound to its CommandRef and actor (1B §6.1).
fn derived(label: &[u8], actor: ExactActorRef, session: GameSessionId, command_id: u64) -> [u8; 16] {
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

fn occurrence(
    actor: ExactActorRef,
    session: GameSessionId,
    command_id: u64,
) -> Option<ProficiencyOccurrence> {
    ProficiencyOccurrence::from_bytes(derived(
        b"oteryn:proficiency-modification-occurrence:v1",
        actor,
        session,
        command_id,
    ))
    .ok()
}

/// The burn of the first carried orb not in a trade, or `None` when the Character carries none.
async fn reserve_orb(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    occurrence: &ProficiencyOccurrence,
) -> Option<ItemUseRequest> {
    let session = command.game_session_id;
    let inventory = owner
        .root
        .read_item_use_inventory(owner.character, owner.holder, command.item_fence?)
        .await
        .ok()?;
    let orb = inventory
        .backpack
        .iter()
        .flat_map(|bag| &bag.entries)
        .chain(inventory.nested.iter().map(|(_, entry)| entry))
        .map(|entry| &entry.item)
        .find(|item| {
            item.definition.production_key == LUNAR_ASCENSION_ORB
                && owner
                    .states
                    .player_trades
                    .physical(session, Some(item.item_instance_id))
        })?;
    let occurred_at_unix_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .and_then(|elapsed| i64::try_from(elapsed.as_millis()).ok())?;
    Some(ItemUseRequest {
        kind: ItemUseKind::ProficiencyOrb,
        command_id: command.command_id,
        content_revision: inventory.content_revision.clone(),
        ruleset_revision: inventory.ruleset_revision.clone(),
        sim_revision: crate::interaction_chest_use::entry_chest::SIM_REVISION.to_owned(),
        transaction_id: *occurrence.as_bytes(),
        event_id: derived(
            b"oteryn:proficiency-orb-event:v1",
            actor,
            session,
            command.command_id,
        ),
        occurred_at_unix_ms,
        server_build_id: crate::durability::character_authority::SERVER_BUILD_ID.to_owned(),
        source_item_instance_id: orb.item_instance_id,
        source_definition: orb.definition.clone(),
        source_quantity_before: orb.quantity,
        flask: ItemUseFlaskPlan::None,
    })
}

/// `None` disconnects: the session is fenced out or the outcome stays unknown.
#[allow(clippy::too_many_lines)]
pub(in crate::gameplay_transport) async fn execute(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    request: ModificationRequest,
) -> Option<ModificationReply> {
    let session = command.game_session_id;
    let (Some(catalogue), Some(binding)) = (
        owner.proficiency_catalogue.as_ref(),
        owner.progression_binding(session),
    ) else {
        return refused(ModificationResult::NotAdmitted);
    };
    let definitions =
        catalogue.for_content(&binding.context.content) as Arc<dyn ProficiencyDefinitions>;
    let resolved = {
        let runtime = owner.runtime.lock().await;
        runtime.player_control_facts(actor, session).ok()?;
        pinned_index(&runtime, owner.active_generation)
            .and_then(|index| index.definition_at(request.definition))
            .and_then(|(key, _)| definitions.resolve_weapon(key))
    };
    let Some(shaping) = ProductionProficiencyShaping::embedded(Arc::clone(catalogue)) else {
        return refused(ModificationResult::NotAdmitted);
    };
    let Some((resolved, shaping_revision)) = resolved.and_then(|resolved| {
        let active = shaping.active(&proficiency_shaping_key(&resolved.definition_key)?)?;
        Some((resolved, active.revision))
    }) else {
        return refused(ModificationResult::NotAdmitted);
    };
    let modification = ProficiencyModificationCommand::new(
        occurrence(actor, session, command.command_id)?,
        &resolved.canonical_item_key,
        request.slot,
        kind(request.operation),
        CharacterRevision::new(request.expected_revision).ok()?,
        ProficiencyModificationBoundRevisions {
            definition_revision: resolved.definition_revision,
            shaping_revision,
            simulation_revision: binding.context.simulation.clone(),
        },
    )
    .ok()?;
    // A level reached by an uncheckpointed kill unlocks a slot only once it is committed. The
    // saver takes its own revision slot, so it runs before this command acquires one.
    if owner.save_weapon_proficiency(actor, session, false).await == TrainingSave::FencedOut {
        return None;
    }
    let current = owner.current_quest_fence(session).await.ok()??;
    // §5 check 2: a known occurrence answers its original result and is never charged.
    match owner
        .root
        .reconcile_character_proficiency_modification(
            owner.character,
            current.character_id,
            modification.clone(),
        )
        .await
    {
        Ok(Some(outcome)) => return Some(reply(&outcome)),
        Ok(None) => {}
        Err(CharacterProgressionError::ConflictingOccurrence) => {
            return refused(ModificationResult::Conflict);
        }
        Err(_) => return None,
    }
    // §5 check 3: `PROF1B-RL-03`, charged when the new occurrence is reserved.
    let now_ms = u64::try_from(owner.clock_origin.elapsed().as_millis()).ok()?;
    let charged = owner
        .states
        .proficiencies
        .lock()
        .ok()?
        .entry(session)
        .or_default()
        .modification_cap
        .charge(now_ms);
    if !charged {
        return refused(ModificationResult::RateLimited);
    }
    let server_seed = owner.root.read_proficiency_shaping_secret().await.ok()?;
    // 1C §5: the writer answers `NO_ORB` when ORB_RANK has nothing to burn.
    let orb = match request.operation {
        ModificationOperation::OrbRank => {
            reserve_orb(owner, actor, command, &modification.occurrence()).await
        }
        _ => None,
    };
    let mut slot = owner.revision_sequencer.acquire(current.character_id).await;
    let root = owner
        .root
        .read_current_character(owner.character, current.character_id)
        .await
        .ok()?;
    let fence = CurrentCharacterGameplayFence {
        expected_character_revision: root.revision,
        ..current
    };
    let in_protection_zone = {
        let mut runtime = owner.runtime.lock().await;
        if runtime.player_control_facts(actor, session).is_err()
            || runtime.owner_fence().is_err()
            || fence.runtime_scope
                != crate::foundation::RuntimeScopeRefV1::channel(
                    runtime.binding().world_id(),
                    runtime.binding().channel_id(),
                )
            || fence.scope_ownership_generation != runtime.binding().scope_generation()
        {
            return None;
        }
        super::super::actor_spell::qualified_protection_zone(
            &mut runtime,
            owner.movement_cells,
            actor,
        ) == Some(true)
    };
    let outcome = slot
        .commit_proficiency_modification(
            owner.root,
            owner.character,
            owner.holder,
            fence,
            modification.clone(),
            ProficiencyModificationContext {
                in_protection_zone,
                server_seed,
            },
            orb,
            definitions,
            shaping,
        )
        .await;
    drop(slot);
    match outcome {
        Ok(outcome) => Some(reply(&outcome)),
        Err(CharacterProgressionError::ConflictingOccurrence) => {
            refused(ModificationResult::Conflict)
        }
        // The cursor moved twice; nothing was written and the command may be retried.
        Err(CharacterProgressionError::CharacterRevisionMismatch) => {
            refused(ModificationResult::StaleRevision)
        }
        // An unknown outcome is reconciled by the occurrence; still unknown disconnects.
        Err(_) => owner
            .root
            .reconcile_character_proficiency_modification(
                owner.character,
                fence.character_id,
                modification,
            )
            .await
            .ok()?
            .map(|outcome| reply(&outcome)),
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, reason = "unit fixtures construct valid values")]
mod tests {
    use super::*;
    use crate::durability::character_proficiency_modification::CommittedProficiencyModification;
    use oteryn_gameplay_domain::domain::weapon_proficiency::{
        ProficiencyModificationCost, ProficiencyModificationOperation, ProficiencyModifiedPerk,
    };

    #[test]
    fn every_writer_result_has_its_wire_result() {
        use ProficiencyModificationResult as Domain;
        let all = [
            Domain::Accepted,
            Domain::NotAdmitted,
            Domain::NotUnlocked,
            Domain::NotInProtectionZone,
            Domain::UnknownTrack,
            Domain::StaleRevision,
            Domain::RevisionChanged,
            Domain::RateLimited,
            Domain::Conflict,
            Domain::NoSelection,
            Domain::SlotOccupied,
            Domain::LevelTaken,
            Domain::NotModified,
            Domain::RankMax,
            Domain::OfferPending,
            Domain::NoOffer,
            Domain::PoolTooSmall,
            Domain::ShapingOutdated,
            Domain::InsufficientDust,
            Domain::NoOrb,
        ];
        // 1B §11.2 order is the wire byte order.
        for (byte, domain) in all.into_iter().enumerate() {
            assert_eq!(result(domain) as usize, byte, "{domain:?}");
        }
    }

    #[test]
    fn each_wire_operation_is_its_writer_command() {
        assert_eq!(
            kind(ModificationOperation::Modify { level: 4 }),
            ProficiencyModificationCommandKind::Modify { level: 4 }
        );
        assert_eq!(
            kind(ModificationOperation::ReshapeChoose { choice: None }),
            ProficiencyModificationCommandKind::ReshapeChoose { choice: None }
        );
        for (wire, writer) in [
            (
                ModificationOperation::RankUp,
                ProficiencyModificationOperation::RankUp,
            ),
            (
                ModificationOperation::OrbRank,
                ProficiencyModificationOperation::OrbRank,
            ),
            (
                ModificationOperation::ReshapeOffer,
                ProficiencyModificationOperation::ReshapeOffer,
            ),
            (
                ModificationOperation::Clear,
                ProficiencyModificationOperation::Clear,
            ),
        ] {
            assert_eq!(kind(wire).operation(), writer);
        }
    }

    #[test]
    fn an_accepted_reply_carries_the_slot_after_and_a_refusal_none() {
        let id = |seed: u8| {
            let mut bytes = [seed; 16];
            bytes[6] = 0x70;
            bytes[8] = 0x80;
            bytes
        };
        let after = ProficiencyModifiedPerk {
            level: 2,
            shaping_key: "oteryn:proficiency-shaping.tibia.p6".into(),
            shaping_revision: "shaping-r1".into(),
            entry_index: 9,
            rank: 10,
            pending_offer: Some([1, 2, 3]),
        };
        let committed = CommittedProficiencyModification {
            character_id: crate::domain::CharacterId::from_bytes(id(1)).expect("id"),
            occurrence_id: id(2),
            original_character_revision: CharacterRevision::new(4).expect("revision"),
            committed_character_revision: CharacterRevision::new(5).expect("revision"),
            item_key: "oteryn:item.tibia.i3264".into(),
            slot: 1,
            operation: ProficiencyModificationOperation::ReshapeOffer,
            before: None,
            after: Some(after),
            cost: ProficiencyModificationCost::default(),
        };
        let accepted = reply(&ProficiencyModificationOutcome::AlreadyCommitted(committed));
        assert_eq!(accepted.result, ModificationResult::Accepted);
        assert_eq!(
            accepted.slot,
            Some(ModifiedSlot {
                level: 2,
                entry_index: 9,
                rank: 10,
                pending_offer: Some([1, 2, 3]),
            })
        );
        assert_eq!(
            reply(&ProficiencyModificationOutcome::RevisionChanged),
            ModificationReply {
                result: ModificationResult::RevisionChanged,
                slot: None,
            }
        );
        assert_eq!(
            reply(&ProficiencyModificationOutcome::Refused(
                ProficiencyModificationResult::NoOrb
            ))
            .result,
            ModificationResult::NoOrb
        );
    }
}
