//! Audited Inbox and corpse input mints for the I07 qualification account.
use super::*;
use crate::durability::{
    character_inbox::{
        RewardClaimInboxMintOutput, RewardClaimInboxMintRequest, RewardClaimInboxPlan,
    },
    item_mint::{
        CORPSE_MATERIALIZATION_PURPOSE_KEY, CorpseContainerPlacement, CorpseLootMintRequest,
        GroundPlacement, ItemMintCause, ItemMintOutcome, ItemMintRequest, TypedDefinitionRef,
    },
    item_transfer::CurrentCharacterItemFence,
    spell_owner_commit::SpellLanePermit,
};
use crate::foundation::{CommandId, CommandRef};
use sha2::{Digest, Sha256};

pub(super) async fn seed(
    root: &DurabilityRoot,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    proof: &NodeIncarnationProof,
    current: &crate::foundation::GameSessionAuthoritySnapshot<
        crate::foundation::AuthenticatedTransportRefV1,
    >,
    native: &NativeGameplayState,
    character: crate::domain::CharacterId,
) -> Result<(), Box<dyn std::error::Error>> {
    let scope = current.current_runtime_scope();
    let fence = CurrentCharacterItemFence {
        character_id: character,
        game_session_id: current.current_game_session_id(),
        connection_generation: current.current_connection_generation(),
        character_lease_generation: current.current_character_lease().generation(),
        runtime_scope: scope,
        scope_ownership_generation: current.current_scope_generation(),
    };
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
    let source = RewardClaimInboxPlan {
        command: CommandRef::new(fence.game_session_id, CommandId::new(u64::MAX - 700)?),
        character_id: character,
        world_id: scope.world_id(),
        channel_id: scope.channel_id().ok_or("I07 fixture needs a channel")?,
        transaction_id,
        source_binding: Sha256::digest([transaction_id, item_id].concat()).into(),
        item_instance_ids: vec![item_id],
    };
    let pouch = item(23721)?;
    let pouch =
        oteryn_gameplay_items::item_ref_admission::generation_inbox_item_facts(native, &pouch)
            .map_err(qualification_build::debug)?;
    let mut candidate = root
        .prepare_reward_claim_inbox_mint(
            authority,
            proof,
            fence,
            &source,
            RewardClaimInboxMintRequest {
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
        )
        .await
        .map_err(qualification_build::debug)?;
    root.commit_reward_claim_inbox_mint(
        authority,
        proof,
        fence,
        &mut candidate,
        &mut (),
        move |tx, _| Box::pin(async move { tx.deliver(item_id).await }),
    )
    .await
    .map_err(qualification_build::debug)?;

    let cause = |purpose: &str, ordinal| {
        ItemMintCause::for_test_scope(
            scope,
            fence.scope_ownership_generation,
            u32::MAX - 7,
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
    let mut corpse = root
        .freeze_item_mint(
            authority,
            proof,
            ItemMintRequest {
                cause: cause(CORPSE_MATERIALIZATION_PURPOSE_KEY, 0),
                item: item(5973)?,
                quantity: 1,
                ground: GroundPlacement {
                    spatial_position:
                        crate::durability::map_item_mint_audit::map_item_spatial_position(
                            // Item custody uses the runtime floor, which is +7 at the town tile.
                            32369, 32238, 7,
                        ),
                    corpse_ref: transaction_id.to_vec(),
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
    for (ordinal, id, quantity) in [(1, 3031, 7), (2, 3350, 1), (3, 3577, 1)] {
        let mut entry = root
            .freeze_corpse_loot_mint(
                authority,
                proof,
                CorpseLootMintRequest {
                    cause: cause("oteryn:qualification.i07.entry", ordinal),
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
    Ok(())
}
