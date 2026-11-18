//! Crystal 201189f dreamers_challenge_quest/actions_tic_tac.lua:3-12.
use super::{Callback, DurabilityError, QuestWorldToolContext};
use crate::character_recovery_fence::CharacterRecoveryFenceV1;
use crate::durability::{
    DurabilityRoot,
    character_progression::CurrentCharacterGameplayFence,
    item_mint::GroundPlacement,
    native_map_items,
    native_map_items_abi::*,
    quest_state::{QuestCause, QuestTransitionRequest},
    runtime_scope_assignment::NodeIncarnationProof,
    spell_items_abi::QualifiedItemDefinition,
    spell_owner_commit::SpellLanePermit,
};
use oteryn_gameplay_content::content::native_gameplay::NativeGameplayState;
use serde_json::json;
use sqlx::{Postgres, Transaction};

struct GroundSupply {
    binding: NativeMapOwnerBinding,
    placements: Vec<NativeMapItemPlacement>,
}

impl map_initializer_seal::Sealed for GroundSupply {}
impl NativeMapInitializationProof for GroundSupply {
    fn current_binding(&self) -> Result<NativeMapOwnerBinding, &'static str> {
        Ok(self.binding.clone())
    }
    fn placements(&self) -> &[NativeMapItemPlacement] {
        &self.placements
    }
}

#[allow(
    clippy::too_many_arguments,
    reason = "the Quest receipt, physical source, current scope and native Ground owner"
)]
pub(in crate::durability) async fn mint(
    tx: &mut Transaction<'_, Postgres>,
    permit: &SpellLanePermit,
    root: &DurabilityRoot,
    recovery: &CharacterRecoveryFenceV1,
    node: &NodeIncarnationProof,
    fence: CurrentCharacterGameplayFence,
    request: &QuestTransitionRequest,
    callback: &Callback,
    context: &QuestWorldToolContext,
    native: &NativeGameplayState,
    receipt: &[u8],
) -> Result<(), DurabilityError> {
    use crate::foundation::RuntimeScopeRefV1;
    let (
        QuestCause::Use(command),
        RuntimeScopeRefV1::Channel {
            world_id,
            channel_id,
        },
        Some(selector),
        Some([opening, returning]),
        Some(face),
    ) = (
        request.cause,
        fence.runtime_scope,
        &callback.selector.native,
        callback.lever,
        &context.ground_supply,
    )
    else {
        return Err(DurabilityError::InvalidStoredState);
    };
    let at = selector.position;
    let source = u64::from(at.x) << 32
        | u64::from(at.y) << 16
        | u64::from(at.floor.unsigned_abs()) << 8
        | u64::from(selector.ordinal);
    if face.source != source || ![opening, returning].contains(&face.before) {
        return Err(DurabilityError::InvalidStoredState);
    }
    let supply_face = if callback.effects.ground_supply_on_open {
        opening
    } else {
        returning
    };
    if face.before != supply_face {
        return Ok(());
    }

    let digest = native.source_digest();
    if context.content_revision != format!("sha256:{}", hex(&digest)) {
        return Err(DurabilityError::InvalidStoredState);
    }
    let frame_digest = context
        .placement_context
        .as_slice()
        .try_into()
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let map_digest =
        parse_map_digest(&context.map_revision).ok_or(DurabilityError::InvalidStoredState)?;
    let binding = NativeMapOwnerBinding {
        world: world_id,
        channel: channel_id,
        scope_generation: fence.scope_ownership_generation.get(),
        content_digest: digest,
        map_digest,
        frame_digest,
    };
    let session = hex(command.game_session_id().as_bytes());
    let character = hex(fence.character_id.as_bytes());
    let ordinal = command.command_id().get();
    let mut placements = Vec::new();
    for (index, supply) in callback.effects.ground_supply.iter().enumerate() {
        let reference = native
            .item_policy(&supply.item, "definition-r1")
            .ok_or(DurabilityError::InvalidStoredState)?;
        let item = QualifiedItemDefinition::from_native_policy(reference)
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        let count = u32::from(supply.count);
        if !item.movable
            || !item.inventory_destination
            || !item.ground_destination
            || item.container_capacity.is_some()
            || item.decay.is_some()
            || count > item.stack_maximum
        {
            return Err(DurabilityError::InvalidStoredState);
        }
        let p = supply.position;
        let mut spatial_position = i32::from(p.x).to_be_bytes().to_vec();
        spatial_position.extend_from_slice(&i32::from(p.y).to_be_bytes());
        spatial_position.extend_from_slice(&i16::from(p.floor).to_be_bytes());
        placements.push(NativeMapItemPlacement {
            placement_key: format!("quest-supply/{session}/{ordinal}/{index}"),
            definition: item.definition,
            quantity: count,
            ground: GroundPlacement {
                spatial_position,
                corpse_ref: b"native-source-map-init-r21".to_vec(),
                map_revision: context.map_revision.clone(),
                content_revision: context.content_revision.clone(),
                native_room_placement_context: context.placement_context.clone(),
            },
            source_binding: receipt.to_vec(),
            source_attributes: json!({"quest_supply":{"character":character,"session":session,
                "command":ordinal.to_string(),"transition":callback.key,"index":index}}),
            blocks_movement: false,
            blocks_projectile: false,
            immovable_block_solid: false,
            owner_kind: None,
        });
    }
    let proof = GroundSupply {
        binding,
        placements,
    };
    native_map_items::materialize_in_transaction(tx, permit, root, recovery, node, &proof, false)
        .await
        .map_err(|error| match error {
            crate::durability::spell_item_transaction::SpellItemError::Durability(error) => error,
            crate::durability::spell_item_transaction::SpellItemError::Database(error) => {
                error.into()
            }
            _ => DurabilityError::InvalidStoredState,
        })?;
    Ok(())
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn parse_map_digest(revision: &str) -> Option<[u8; 32]> {
    let text = revision.strip_prefix("sha256:")?;
    if text.len() != 64 || !text.is_ascii() {
        return None;
    }
    let mut digest = [0; 32];
    for (i, byte) in digest.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&text[i * 2..i * 2 + 2], 16).ok()?;
    }
    Some(digest)
}
