//! Audited Inbox and corpse input mints for the I07 qualification account.
use super::*;
use crate::durability::{
    character_authority::CharacterAuthorityRecord,
    character_inbox::{RewardClaimInboxMintOutput, RewardClaimInboxMintRequest},
    character_progression::CurrentCharacterGameplayFence,
    item_mint::{
        CORPSE_MATERIALIZATION_PURPOSE_KEY, CorpseContainerPlacement, CorpseLootMintRequest,
        GroundPlacement, ItemMintCause, ItemMintOutcome, ItemMintRequest, TypedDefinitionRef,
    },
    spell_owner_commit::SpellLanePermit,
    store::DeliveryPlan,
};
use crate::foundation::{CommandId, CommandRef};

pub(super) async fn seed(
    root: &DurabilityRoot,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    proof: &NodeIncarnationProof,
    current: &crate::foundation::GameSessionAuthoritySnapshot<
        crate::foundation::AuthenticatedTransportRefV1,
    >,
    native: &NativeGameplayState,
    record: &CharacterAuthorityRecord,
    spawn: SpawnOwners<'_>,
) -> Result<(), Box<dyn std::error::Error>> {
    let character = record.character_id;
    let scope = current.current_runtime_scope();
    let gameplay_fence = CurrentCharacterGameplayFence {
        character_id: character,
        game_session_id: current.current_game_session_id(),
        connection_generation: current.current_connection_generation(),
        character_lease_generation: current.current_character_lease().generation(),
        runtime_scope: scope,
        scope_ownership_generation: current.current_scope_generation(),
        expected_character_revision: record.revision,
    };
    let fence = gameplay_fence.item_fence();
    let inventory = root
        .read_item_use_inventory(authority, proof, fence)
        .await
        .map_err(qualification_build::debug)?;
    let index = native
        .item_index()
        .ok_or("I07 fixture item index missing")?;
    let item = |id: u32| -> Result<TypedDefinitionRef, Box<dyn std::error::Error>> {
        let key = format!("oteryn:item.tibia.i{id}");
        let revision = index
            .revision(&key)
            .ok_or("I07 fixture item missing")?
            .into();
        Ok(TypedDefinitionRef {
            family: "Item".into(),
            production_key: key,
            revision_ref: revision,
        })
    };
    let mut transaction_id = *fence.game_session_id.as_bytes();
    transaction_id[15] ^= 0xe1;
    let mut item_id = *fence.game_session_id.as_bytes();
    item_id[15] ^= 0xe2;
    let mut fulfilment_id = *fence.game_session_id.as_bytes();
    fulfilment_id[15] ^= 0xe3;
    let fulfilment = uuid_text(&fulfilment_id);
    let pouch = oteryn_gameplay_items::item_ref_admission::generation_inbox_item_facts(
        native,
        &item(23721)?,
    )
    .map_err(qualification_build::debug)?;
    // Store delivery is a fixture input. Its production writer creates Inbox custody and
    // the matching receipt atomically. The player still assigns and uses the pouch over the wire.
    let receipt = root
        .apply_store_delivery(
            authority,
            proof,
            transaction_id,
            DeliveryPlan {
                account_id: *record.account_id.as_bytes(),
                character_id: *character.as_bytes(),
                world_id: *scope.world_id().as_bytes(),
                fulfilment_unit_id: fulfilment.clone(),
                item_instance_id: item_id,
                purchase_request_id: None,
                command: CommandRef::new(fence.game_session_id, CommandId::new(u64::MAX - 700)?),
                mint: RewardClaimInboxMintRequest {
                    claim: TypedDefinitionRef {
                        family: "RewardClaim".into(),
                        production_key: fixture_gear_claim(fence.game_session_id, "i07-pouch"),
                        revision_ref: "qualification-v1".into(),
                    },
                    content_revision: inventory.content_revision.clone(),
                    ruleset_revision: inventory.ruleset_revision.clone(),
                    sim_revision: "qualification-v1".into(),
                    outputs: vec![RewardClaimInboxMintOutput {
                        source_line: 0,
                        item: pouch,
                        quantity: 1,
                        key_number: None,
                    }],
                },
            },
            gameplay_fence,
        )
        .await
        .map_err(qualification_build::debug)?;
    if receipt.result != "COMMITTED" || receipt.fulfilment_unit_id != fulfilment {
        return Err("I07 Store fixture did not commit its pouch delivery".into());
    }

    let cause = |death_slot, purpose: &str, ordinal| {
        ItemMintCause::for_test_scope(
            scope,
            fence.scope_ownership_generation,
            death_slot,
            1,
            TypedDefinitionRef {
                family: "LootTable".into(),
                production_key: "oteryn:qualification.i07.loot".into(),
                revision_ref: "qualification-v1".into(),
            },
            purpose.into(),
            ordinal,
        )
    };
    let permit = SpellLanePermit::of_fresh_scope(scope).await?;
    for (death_slot, items) in [
        (
            u32::MAX - 7,
            &[(1, 3031, 7), (2, 3357, 1), (3, 3577, 1), (4, 3577, 1)][..],
        ),
        (u32::MAX - 8, &[(1, 3578, 1), (2, 3582, 1)][..]),
        (u32::MAX - 9, &[(1, 3577, 1), (2, 3585, 1)][..]),
        (u32::MAX - 10, &[(1, 3031, 3)][..]),
    ] {
        let mut corpse = root
            .freeze_item_mint(
                authority,
                proof,
                ItemMintRequest {
                    cause: cause(death_slot, CORPSE_MATERIALIZATION_PURPOSE_KEY, 0),
                    item: item(5973)?,
                    quantity: 1,
                    ground: GroundPlacement {
                        spatial_position:
                            crate::durability::map_item_mint_audit::map_item_spatial_position(
                                // Item custody uses the runtime floor, which is +7 at the town tile.
                                if death_slot == u32::MAX - 7 {
                                    32369
                                } else {
                                    32370
                                },
                                if death_slot == u32::MAX - 10 {
                                    32239
                                } else {
                                    32238
                                },
                                7,
                            ),
                        corpse_ref: [transaction_id.as_slice(), &death_slot.to_be_bytes()].concat(),
                        map_revision: "qualification-v1".into(),
                        content_revision: inventory.content_revision.clone(),
                        native_room_placement_context: transaction_id.to_vec(),
                    },
                    content_revision: inventory.content_revision.clone(),
                    ruleset_revision: inventory.ruleset_revision.clone(),
                    sim_revision: "qualification-v1".into(),
                },
            )
            .await
            .map_err(qualification_build::debug)?;
        let ItemMintOutcome::Committed(corpse) = root
            .commit_corpse_mint(
                &permit,
                authority,
                proof,
                &mut corpse,
                *character.as_bytes(),
            )
            .await
            .map_err(qualification_build::debug)?
        else {
            return Err("I07 fixture corpse did not commit".into());
        };
        for &(ordinal, id, quantity) in items {
            let mut entry = root
                .freeze_corpse_loot_mint(
                    authority,
                    proof,
                    CorpseLootMintRequest {
                        cause: cause(death_slot, "oteryn:qualification.i07.entry", ordinal),
                        item: item(id)?,
                        quantity,
                        placement: CorpseContainerPlacement {
                            corpse_item_instance_id: corpse.item_instance_id,
                            placement_ordinal: ordinal,
                            fluid_subtype: None,
                        },
                        content_revision: inventory.content_revision.clone(),
                        ruleset_revision: inventory.ruleset_revision.clone(),
                        sim_revision: "qualification-v1".into(),
                    },
                )
                .await
                .map_err(qualification_build::debug)?;
            if !matches!(
                root.commit_corpse_loot_mint(&permit, authority, proof, &mut entry)
                    .await
                    .map_err(qualification_build::debug)?,
                ItemMintOutcome::Committed(_)
            ) {
                return Err("I07 fixture corpse entry did not commit".into());
            }
        }
        publish_corpse(
            spawn,
            fence.game_session_id,
            corpse.item_instance_id,
            i32::from(death_slot != u32::MAX - 7),
            i32::from(death_slot == u32::MAX - 10),
        )
        .await?;
    }
    Ok(())
}

async fn publish_corpse(
    spawn: SpawnOwners<'_>,
    session: crate::foundation::GameSessionId,
    item: [u8; 16],
    east: i32,
    south: i32,
) -> Result<(), Box<dyn std::error::Error>> {
    use oteryn_gameplay_entities::MovementLocalPosition;
    let mut runtime = spawn.runtime.lock().await;
    let (_, position) = runtime
        .positioned_player_for_session(session)
        .map_err(qualification_build::debug)?
        .ok_or("I07 fixture actor is not positioned")?;
    let position = position.position();
    let creature = runtime
        .admit_pinned_test_creature(MovementLocalPosition {
            x: position
                .x
                .checked_add(east)
                .ok_or("I07 corpse position overflow")?,
            y: position
                .y
                .checked_add(south)
                .ok_or("I07 corpse position overflow")?,
            floor: position.floor,
        })
        .map_err(qualification_build::debug)?;
    runtime
        .commit_monster_lab_damage(creature, b"quick-loot-corpse", 20)
        .map_err(qualification_build::debug)?;
    let mut death = runtime.borrow_combat_death();
    let receipt = death
        .committed_lethal_receipt(creature)
        .map_err(qualification_build::debug)?;
    death
        .project_committed_lethal(receipt)
        .map_err(qualification_build::debug)?;
    runtime
        .bind_corpse_item(
            creature,
            item,
            "Item",
            "oteryn:item.tibia.i5973",
            "definition-r1",
        )
        .map_err(qualification_build::debug)?;
    Ok(())
}
