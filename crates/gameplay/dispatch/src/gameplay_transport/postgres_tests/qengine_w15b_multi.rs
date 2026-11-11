//! Multiple top-level chest rewards must retain atomic claim, quest and replay semantics.
use super::TestResult;
#[test]
fn w15b_multiple_top_level_items_real_dispatch_once_and_replay() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::quest_chest_postgres_tests::run_multiple();
    eprintln!(
        "POSTGRES_CASE_COMPLETED W15B_MULTIPLE_ROOTS duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

#[test]
fn w15b_multiple_top_level_items_one_free_slot_refuses_atomically() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::quest_chest_postgres_tests::run_one_free_slot();
    eprintln!(
        "POSTGRES_CASE_COMPLETED W15B_NO_ROOM duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

pub(super) async fn fill_to_one_free_slot(
    owner: &super::ComposedFreshAdmission<'_, '_, '_>,
    fence: crate::durability::item_transfer::CurrentCharacterItemFence,
    content: &crate::content::CanonicalReferencePlayableContent,
) -> TestResult {
    use crate::combat_pickup::resolve_item_definition_facts;
    use crate::durability::item_mint::TypedDefinitionRef;
    use crate::durability::reward_claim_mint::{RewardClaimMintOutcome, RewardClaimMintRequest};
    use crate::foundation::{CommandId, CommandRef};
    use crate::interaction_chest_use::entry_chest;
    let definition = |family: &str, key: &str| TypedDefinitionRef {
        family: family.into(),
        production_key: key.into(),
        revision_ref: entry_chest::DEFINITION_REVISION.into(),
    };
    let item =
        resolve_item_definition_facts(content, &definition("Item", entry_chest::REWARD_ITEM))
            .map_err(super::fixture::debug)?;
    let backpack =
        resolve_item_definition_facts(content, &definition("Item", entry_chest::BACKPACK_ITEM))
            .map_err(super::fixture::debug)?;
    for ordinal in 1..entry_chest::BACKPACK_CAPACITY {
        if owner.root.has_ready_demand() {
            owner
                .root
                .maintain_ready_once()
                .await
                .map_err(super::fixture::debug)?;
        }
        let request = RewardClaimMintRequest {
            backpack_outputs: false,
            root_charges_remaining: None,
            root_fluid_subtype: None,
            command: CommandRef::new(
                fence.game_session_id,
                CommandId::new(2000 + u64::from(ordinal)).map_err(super::fixture::debug)?,
            ),
            claim: definition(
                "RewardClaim",
                &format!("oteryn:reward-claim/w15b-capacity-input-{ordinal}"),
            ),
            source_placement: format!("oteryn:placement/w15b-capacity-input/{ordinal}"),
            item: item.clone(),
            quantity: 1,
            contents: Vec::new(),
            additional_items: Vec::new(),
            root_text: None,
            backpack: backpack.clone(),
            content_revision: entry_chest::CONTENT_REVISION.into(),
            ruleset_revision: entry_chest::RULESET_REVISION.into(),
            sim_revision: entry_chest::SIM_REVISION.into(),
            achievement: None,
            quest_transition: None,
            key_number: None,
        };
        let mut candidate = owner
            .root
            .freeze_reward_claim_mint(owner.character, owner.holder, fence, request)
            .await
            .map_err(super::fixture::debug)?;
        if owner.root.has_ready_demand() {
            owner
                .root
                .maintain_ready_once()
                .await
                .map_err(super::fixture::debug)?;
        }
        assert!(matches!(
            owner
                .root
                .commit_reward_claim_mint(owner.character, owner.holder, fence, &mut candidate)
                .await
                .map_err(super::fixture::debug)?,
            RewardClaimMintOutcome::Committed(_)
        ));
    }
    Ok(())
}

pub(super) fn assert_no_room(output: &[u8]) -> TestResult {
    let mut cursor = 0;
    let mut results = Vec::new();
    while cursor < output.len() {
        let length = u32::from_be_bytes(output[cursor..cursor + 4].try_into()?) as usize;
        let envelope =
            crate::foundation::decode_wire_envelope(&output[cursor + 4..cursor + 4 + length])?;
        if envelope.message_type() == crate::foundation::MessageType::CommandResult {
            let result = oteryn_protocol_oteryn::decode_command_result(envelope.payload())?;
            results.push(
                oteryn_protocol_oteryn::world_object::decode_use_result(result.payload)
                    .map_err(super::fixture::debug)?,
            );
        }
        cursor += 4 + length;
    }
    assert_eq!(
        results,
        // Existing chest USE contract maps capacity refusals to Rejected.
        [oteryn_protocol_oteryn::world_object::UseDisposition::Rejected; 2]
    );
    Ok(())
}
