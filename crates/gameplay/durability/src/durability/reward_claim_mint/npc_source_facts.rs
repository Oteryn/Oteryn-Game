//! Current-native facts for the closed Source-NPC MINT child.
use super::*;
use crate::durability::consume_transform_cosmetics::ItemCosmeticPremium;
use crate::durability::item_transfer::{ItemWeightLookup, ItemWeightRefused};
use crate::durability::npc_cosmetics::NpcProgramContext;
use oteryn_gameplay_content::content::native_gameplay::NativeGameplayState;
use oteryn_gameplay_items::item_ref_admission::generation_item_facts;

pub(in crate::durability) struct SourceWeights<'a> {
    pub native: &'a NativeGameplayState,
    pub premium: Option<&'a dyn ItemCosmeticPremium>,
}
impl ItemWeightLookup for SourceWeights<'_> {
    fn generation_digest(&self) -> [u8; 32] {
        self.native.source_digest()
    }
    fn wheel_premium_current(&self, account: [u8; 16]) -> bool {
        self.premium.is_some_and(|premium| premium.current(account))
    }
    fn weight_of(
        &self,
        definition: &TypedDefinitionRef,
    ) -> std::result::Result<u32, ItemWeightRefused> {
        let facts = generation_item_facts(self.native, definition)
            .map_err(|_| ItemWeightRefused::DefinitionMismatch)?;
        if facts.stack == ItemStackClass::Unknown {
            return Err(ItemWeightRefused::DefinitionMismatch);
        }
        facts.weight.ok_or(ItemWeightRefused::Unknown)
    }
}

/// Called inside the actual parent TX after material costs. This reads the
/// equipped main bag, never a caller-provided bag identity or capacity.
pub(in crate::durability) async fn request_in_transaction(
    tx: &mut crate::durability::item_mint_audit::Type2Transaction<'_>,
    ctx: &NpcProgramContext<'_>,
) -> Pass<RewardClaimMintRequest> {
    let Some(id) = super::npc_source::output_id(ctx.source_key) else {
        return Ok(Err(RewardClaimMintError::InvalidInput));
    };
    let Some(slot) = load_slot(tx, ctx.fence.character_id).await? else {
        return Ok(Err(RewardClaimMintError::Refused(
            RewardClaimRefusal::NoMainBackpack,
        )));
    };
    let definition = TypedDefinitionRef {
        family: "Item".into(),
        production_key: format!("oteryn:item.tibia.i{id}"),
        revision_ref: "definition-r1".into(),
    };
    let item = generation_item_facts(ctx.native, &definition)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let backpack = generation_item_facts(ctx.native, &slot.definition)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    if item.stack != ItemStackClass::NonStackable || item.container_capacity.is_some() {
        return Err(DurabilityError::InvalidStoredState);
    }
    let profile=sqlx::query("SELECT ruleset_revision,simulation_revision FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid")
        .bind(ctx.fence.character_id.as_bytes().as_slice()).fetch_one(&mut ***tx).await?;
    let hex = |bytes: &[u8]| bytes.iter().map(|b| format!("{b:02x}")).collect::<String>();
    Ok(Ok(RewardClaimMintRequest {
        command: ctx.command,
        claim: TypedDefinitionRef {
            family: "NpcCosmetic".into(),
            production_key: format!(
                "oteryn:npc-source-output.{}",
                sqlx::types::Uuid::from_bytes(ctx.npc_transaction)
            ),
            revision_ref: format!("npc-source-program.{}", hex(&ctx.program_sha256)),
        },
        source_placement: ctx.source_key.into(),
        item,
        quantity: 1,
        contents: Vec::new(),
        root_text: None,
        backpack,
        content_revision: format!("sha256:{}", hex(&ctx.native.source_digest())),
        ruleset_revision: profile.try_get("ruleset_revision")?,
        sim_revision: profile.try_get("simulation_revision")?,
        achievement: None,
        quest_transition: None,
        key_number: None,
    }))
}
