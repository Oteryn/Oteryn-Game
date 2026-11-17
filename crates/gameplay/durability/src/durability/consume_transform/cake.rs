//! Crystal biff_the_baker.lua:69-100: surrender all carried substances up to the
//! shared200 goal. A validated native NPC caller supplies this private context;
//! ordinary USE never selects this recipe.
use super::*;

pub const RECIPE: &str = "oteryn:cake.baker-substances";
#[derive(Debug, Clone)]
pub struct CakeBakerContext {
    pub npc_key: String,
    pub expected_epoch: u64,
}

pub fn is_baker(key: &str) -> bool {
    matches!(
        key,
        "oteryn:npc.benny_the_baker"
            | "oteryn:npc.biff_the_baker"
            | "oteryn:npc.brian_the_baker"
            | "oteryn:npc.balduin_the_baker"
            | "oteryn:npc.billy_the_baker"
    )
}

impl DurabilityRoot {
    #[allow(clippy::too_many_arguments)]
    pub async fn consume_cake_substances<C: ItemLoopContent>(
        &self,
        permit: &super::super::spell_owner_commit::SpellLanePermit,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        request: ConsumeTransformRequest,
        native: &NativeGameplayState,
        content: &C,
        context: &CakeBakerContext,
    ) -> Result<Option<ConsumeTransformResult>, DurabilityError> {
        self.consume_transform_inner(
            permit,
            recovery,
            node,
            fence,
            request,
            native,
            content,
            false,
            None,
            ActionContext::CakeBaker(context),
        )
        .await
    }

    /// Probe only the original source receipt; never consume current inventory.
    #[allow(clippy::too_many_arguments)]
    pub async fn reconcile_cake_substances<C: ItemLoopContent>(
        &self,
        permit: &super::super::spell_owner_commit::SpellLanePermit,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        request: ConsumeTransformRequest,
        native: &NativeGameplayState,
        content: &C,
        context: &CakeBakerContext,
    ) -> Result<Option<ConsumeTransformResult>, DurabilityError> {
        self.consume_transform_inner(
            permit,
            recovery,
            node,
            fence,
            request,
            native,
            content,
            true,
            None,
            ActionContext::CakeBaker(context),
        )
        .await
    }
}

pub(super) async fn prepare(
    tx: &mut super::super::item_mint_audit::Type2Transaction<'_>,
    fence: CurrentCharacterItemFence,
    request: &ConsumeTransformRequest,
    definition: &TypedDefinitionRef,
    quantity: i64,
    context: &CakeBakerContext,
    source_equipped: bool,
) -> Result<Option<ActionPlan>, DurabilityError> {
    if !is_baker(&context.npc_key)
        || request.target.is_some()
        || definition.family != "Item"
        || definition.production_key != "oteryn:item.tibia.i12143"
        || definition.revision_ref != "definition-r1"
        || !(1..=100).contains(&quantity)
    {
        return Ok(None);
    }
    let world = fence.runtime_scope.world_id();
    let Some(progress) = super::super::raid_runner::cake::phase_one_in_transaction(
        tx,
        world,
        context.expected_epoch,
    )
    .await?
    else {
        return Ok(None);
    };
    let remaining = 200_u32
        .checked_sub(u32::from(progress.substances))
        .ok_or(DurabilityError::InvalidStoredState)?;
    if remaining == 0 || !progress.phase_one_active() {
        return Ok(None);
    }
    let source_count = u32::try_from(quantity)
        .map_err(|_| DurabilityError::InvalidStoredState)?
        .min(remaining);
    let mut left = remaining - source_count;
    let mut extra_inputs = Vec::new();
    // The Character root is already locked by the existing item owner. Retain
    // each exact current custody row; the ordinary SQL extra-input guard checks
    // that predecessor again before the same receipt can burn anything.
    let rows = sqlx::query("SELECT uuid_send(i.item_instance_id) AS item,to_jsonb(i) AS before_item,coalesce(uuid_send(e.parent_item_instance_id),uuid_send(n.parent_item_instance_id),uuid_send(h.character_id)) AS parent,coalesce(e.placement_ordinal,n.placement_ordinal,h.slot)::text AS ordinal,CASE WHEN h.item_instance_id IS NOT NULL THEN 2 WHEN e.item_instance_id IS NOT NULL THEN 1 ELSE 4 END AS custody,i.quantity FROM game_item_instances i LEFT JOIN game_item_container_entries e ON e.item_instance_id=i.item_instance_id AND e.character_id=encode($1,'hex')::uuid LEFT JOIN game_item_container_slots b ON b.character_id=e.character_id AND b.item_instance_id=e.parent_item_instance_id LEFT JOIN game_item_nested_entries n ON n.item_instance_id=i.item_instance_id LEFT JOIN LATERAL game_item_loop_owner(n.parent_item_instance_id) owner ON n.item_instance_id IS NOT NULL LEFT JOIN game_character_equipment_slots h ON h.item_instance_id=i.item_instance_id AND h.character_id=encode($1,'hex')::uuid AND h.world_id=i.world_id WHERE i.world_id=encode($2,'hex')::uuid AND i.item_instance_id<>encode($3,'hex')::uuid AND i.lifecycle=1 AND i.definition_family='Item' AND i.definition_production_key='oteryn:item.tibia.i12143' AND i.definition_revision_ref='definition-r1' AND ((e.world_id=i.world_id AND b.world_id=i.world_id) OR (n.world_id=i.world_id AND owner.owner_character=encode($1,'hex')::uuid AND owner.depth BETWEEN 0 AND 7) OR h.item_instance_id IS NOT NULL) ORDER BY CASE WHEN e.item_instance_id IS NOT NULL THEN 0 ELSE owner.depth END,coalesce(e.placement_ordinal,n.placement_ordinal,h.slot),i.item_instance_id LIMIT 501 FOR UPDATE OF i")
        .bind(fence.character_id.as_bytes().as_slice()).bind(world.as_bytes().as_slice()).bind(request.source.as_slice()).fetch_all(&mut ***tx).await?;
    if rows.len() > 500 {
        return Err(DurabilityError::InvalidStoredState);
    }
    for row in rows {
        if left == 0 {
            break;
        }
        let quantity = u32::try_from(row.try_get::<i64, _>("quantity")?)
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        if !(1..=100).contains(&quantity) {
            return Err(DurabilityError::InvalidStoredState);
        }
        let count = left.min(quantity);
        extra_inputs.push(ExtraInput {
            item: row
                .try_get::<Vec<u8>, _>("item")?
                .try_into()
                .map_err(|_| DurabilityError::InvalidStoredState)?,
            before: row.try_get("before_item")?,
            parent: row
                .try_get::<Vec<u8>, _>("parent")?
                .try_into()
                .map_err(|_| DurabilityError::InvalidStoredState)?,
            ordinal: row.try_get("ordinal")?,
            count,
            source_kind: row.try_get("custody")?,
        });
        left -= count;
    }
    let equipment_before: Option<Value> = if source_equipped
        || extra_inputs.iter().any(|input| input.source_kind == 2)
    {
        Some(sqlx::query_scalar("SELECT to_jsonb(s) FROM game_character_equipment_state s WHERE character_id=encode($1,'hex')::uuid FOR UPDATE")
            .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&mut ***tx).await?)
    } else {
        None
    };
    let accepted = remaining - left;
    Ok(Some(ActionPlan {
        recipe: RECIPE.to_owned(),
        outputs: Vec::new(),
        consume_source: true,
        source_count,
        extra_inputs,
        effects: json!({"kind":"cake_substances","equipment_before":equipment_before,"npc_key":context.npc_key,"reset_epoch":context.expected_epoch.to_string(),
            "year":progress.year,"before":progress.substances,"after":u32::from(progress.substances)+accepted,"accepted":accepted}),
    }))
}

pub(super) fn valid_shape(plan: &ActionPlan) -> bool {
    plan.recipe == RECIPE
        && plan.consume_source
        && plan.outputs.is_empty()
        && (1..=100).contains(&plan.source_count)
        && plan.extra_inputs.len() <= 199
        && plan
            .extra_inputs
            .iter()
            .all(|input| (1..=100).contains(&input.count) && matches!(input.source_kind, 1 | 2 | 4))
        && plan
            .extra_inputs
            .iter()
            .map(|input| input.count)
            .sum::<u32>()
            + plan.source_count
            <= 200
}

pub(super) async fn apply_equipment(
    tx: &mut super::super::item_mint_audit::Type2Transaction<'_>,
    fence: CurrentCharacterItemFence,
    plan: &ActionPlan,
    transaction: &str,
) -> Result<(), DurabilityError> {
    if plan.recipe != RECIPE {
        return Ok(());
    }
    let Some(before) = plan
        .effects
        .get("equipment_before")
        .filter(|value| !value.is_null())
    else {
        return Ok(());
    };
    let changed = sqlx::query("UPDATE game_character_equipment_state SET revision=revision+1,last_transaction_id=$2::uuid WHERE character_id=encode($1,'hex')::uuid AND revision=($3->>'revision')::numeric")
        .bind(fence.character_id.as_bytes().as_slice()).bind(transaction).bind(before).execute(&mut ***tx).await?.rows_affected();
    if changed != 1 {
        return Err(DurabilityError::InvalidStoredState);
    }
    Ok(())
}
