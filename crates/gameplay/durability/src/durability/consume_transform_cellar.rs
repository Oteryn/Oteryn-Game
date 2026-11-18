use super::*;
use oteryn_gameplay_items::haunted_cellar;

pub(super) async fn questline(
    tx: &mut super::super::item_mint_audit::Type2Transaction<'_>,
    fence: CurrentCharacterItemFence,
) -> Result<i64, DurabilityError> {
    Ok(sqlx::query_scalar(
        "SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2",
    )
    .bind(fence.character_id.as_bytes().as_slice())
    .bind(haunted_cellar::QUESTLINE)
    .fetch_optional(&mut ***tx)
    .await?
    .unwrap_or(-1))
}

pub(super) async fn prepare(
    tx: &mut super::super::item_mint_audit::Type2Transaction<'_>,
    fence: CurrentCharacterItemFence,
    native: &NativeGameplayState,
    source: &TypedDefinitionRef,
    roll: u32,
) -> Result<Option<ActionPlan>, DurabilityError> {
    let Some(bucket) = source
        .production_key
        .strip_prefix("oteryn:item.tibia.i")
        .and_then(|id| id.parse().ok())
    else {
        return Ok(None);
    };
    let Some(used) = haunted_cellar::use_on_egg(bucket, questline(tx, fence).await?, roll) else {
        return Ok(None);
    };
    let (outputs, timed_output) = if let Some(id) = used.filled_bucket {
        let Some(policy) = native.item_policy_for_source_id(id) else {
            return Ok(None);
        };
        let record = policy.record();
        if !record.admission.materializable {
            return Ok(None);
        }
        let definition = &record.production_definition;
        (
            vec![(
                TypedDefinitionRef {
                    family: definition.family.clone(),
                    production_key: definition.production_key.clone(),
                    revision_ref: definition.revision_ref.clone(),
                },
                1,
            )],
            json!({"duration_ms": haunted_cellar::bucket_duration_ms(id)}),
        )
    } else {
        (Vec::new(), Value::Null)
    };
    let consume_source = used.filled_bucket.is_some();
    Ok(Some(ActionPlan {
        recipe: "oteryn:haunted-cellar-egg.v1".into(),
        outputs,
        consume_source,
        source_count: u32::from(consume_source),
        effects: json!({
            "filled_bucket": used.filled_bucket,
            "restore_after_ms": used.restore_after_ms,
            "timed_output": timed_output,
        }),
        extra_inputs: Vec::new(),
    }))
}

impl DurabilityRoot {
    #[allow(clippy::too_many_arguments)]
    pub async fn consume_transform_haunted_egg<C: ItemLoopContent>(
        &self,
        permit: &super::super::spell_owner_commit::SpellLanePermit,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        request: ConsumeTransformRequest,
        native: &NativeGameplayState,
        content: &C,
        roll: u32,
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
            ActionContext::CellarEgg { roll },
        )
        .await
    }
}
