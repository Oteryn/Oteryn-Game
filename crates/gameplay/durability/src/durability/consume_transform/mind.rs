//! Fenced consumption for Crystal 201189f8's Mind potion and energy field.
use super::*;

pub const POTION: &str = "oteryn:item.tibia.i31350";
const RING: &str = "oteryn:item.tibia.i31306";
const SHARDS: [&str; 3] = [
    "oteryn:item.tibia.i31354",
    "oteryn:item.tibia.i31355",
    "oteryn:item.tibia.i31356",
];

#[derive(Clone, Copy)]
pub enum MindAction {
    Potion,
    Field,
}

impl MindAction {
    pub(super) const fn recipe(self) -> &'static str {
        match self {
            Self::Potion => "oteryn:kilmaresh.mind-potion",
            Self::Field => "oteryn:kilmaresh.mind-field",
        }
    }
}

pub(super) fn is_recipe(recipe: &str) -> bool {
    [MindAction::Potion.recipe(), MindAction::Field.recipe()].contains(&recipe)
}

impl DurabilityRoot {
    #[allow(clippy::too_many_arguments, reason = "binds the action to the existing Item owner fences")]
    pub async fn consume_mind_item<C: ItemLoopContent>(
        &self,
        permit: &super::super::spell_owner_commit::SpellLanePermit,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        request: ConsumeTransformRequest,
        native: &NativeGameplayState,
        content: &C,
        action: MindAction,
    ) -> Result<Option<ConsumeTransformResult>, DurabilityError> {
        self.consume_transform_inner(
            permit, recovery, node, fence, request, native, content, false, None,
            ActionContext::Mind(action),
        ).await
    }
}

pub(super) async fn select_shard(
    tx: &mut super::super::item_mint_audit::Type2Transaction<'_>,
    fence: CurrentCharacterItemFence,
    command: CommandRef,
) -> Result<Option<[u8; 16]>, DurabilityError> {
    // One candidate per possessed definition, regardless of stack sizes.
    let rows: Vec<Vec<u8>> = sqlx::query_scalar(
        "SELECT DISTINCT ON (i.definition_production_key) uuid_send(i.item_instance_id)
         FROM game_quest_callback_carried(encode($1,'hex')::uuid,encode($2,'hex')::uuid) c
         JOIN game_item_instances i ON i.item_instance_id=c.item
         WHERE c.kind IN ('container','nested') AND i.lifecycle=1
           AND i.definition_family='Item' AND i.definition_production_key=ANY($3)
         ORDER BY i.definition_production_key,i.item_instance_id LIMIT 3",
    )
    .bind(fence.character_id.as_bytes().as_slice())
    .bind(fence.runtime_scope.world_id().as_bytes().as_slice())
    .bind(SHARDS.as_slice())
    .fetch_all(&mut ***tx)
    .await?;
    let Some(index) = reward_bag_bounded_draw(
        *fence.game_session_id.as_bytes(), command.command_id().get(),
        *fence.character_id.as_bytes(), 51, rows.len() as u64,
    ) else {
        return Ok(None);
    };
    rows[index as usize].as_slice().try_into().map(Some)
        .map_err(|_| DurabilityError::InvalidStoredState)
}

pub fn potion_landing(command: CommandRef, source: [u8; 16]) -> Option<usize> {
    reward_bag_bounded_draw(
        *command.game_session_id().as_bytes(), command.command_id().get(), source, 50, 2,
    ).map(|index| index as usize)
}

pub(super) async fn prepare(
    tx: &mut super::super::item_mint_audit::Type2Transaction<'_>,
    fence: CurrentCharacterItemFence,
    request: &ConsumeTransformRequest,
    definition: &TypedDefinitionRef,
    action: MindAction,
) -> Result<Option<ActionPlan>, DurabilityError> {
    let effects = match action {
        MindAction::Potion if definition.production_key == POTION => {
            let equipped: bool = sqlx::query_scalar(
                "SELECT EXISTS (SELECT 1 FROM game_character_equipment_slots s
                 JOIN game_item_instances i USING (item_instance_id)
                 WHERE s.character_id=encode($1,'hex')::uuid AND s.slot=8
                   AND s.world_id=encode($2,'hex')::uuid AND i.world_id=s.world_id
                   AND i.lifecycle=1 AND i.definition_family='Item' AND i.definition_production_key=$3)",
            )
            .bind(fence.character_id.as_bytes().as_slice())
            .bind(fence.runtime_scope.world_id().as_bytes().as_slice())
            .bind(RING)
            .fetch_one(&mut ***tx)
            .await?;
            if !equipped {
                return Ok(None);
            }
            let landing = potion_landing(request.command, request.source).ok_or(DurabilityError::InvalidStoredState)?;
            json!({"kind":"consume_effect","landing":landing})
        }
        MindAction::Field if SHARDS.contains(&definition.production_key.as_str()) => json!({}),
        _ => return Ok(None),
    };
    Ok(Some(ActionPlan {
        recipe: action.recipe().into(),
        outputs: Vec::new(),
        consume_source: true,
        source_count: 1,
        effects,
        extra_inputs: Vec::new(),
    }))
}
