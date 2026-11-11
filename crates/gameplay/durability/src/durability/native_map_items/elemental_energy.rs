//! Generation adoption after the map initializer's actual scope/Node qualification.
use super::*;
use serde_json::Value;

pub(super) async fn adopt(
    tx: &mut Transaction<'_, Postgres>,
    binding: &NativeMapOwnerBinding,
    node: &NodeIncarnationProof,
    epoch: u64,
) -> Result<()> {
    // The initializer already holds key-33 and the actual scope/Node fences.
    // The accepted channel field budget bounds this complete ownership census.
    let rows = sqlx::query("SELECT r.transaction_id::text,to_jsonb(g) AS ground_before FROM game_elemental_energy_field_receipts r JOIN game_item_instances i USING(item_instance_id) JOIN game_item_ground_locations g USING(item_instance_id) WHERE r.world_id=encode($1,'hex')::uuid AND r.channel_id=encode($2,'hex')::uuid AND r.map_revision=$3 AND r.content_revision=$4 AND r.placement_context=$5 AND r.reset_epoch=$6::text::bigint AND g.runtime_scope_ownership_generation<$7::text::numeric AND game_elemental_energy_field_source(i.item_instance_id) ORDER BY i.item_instance_id LIMIT 20001 FOR UPDATE OF i")
        .bind(binding.world.as_bytes().as_slice()).bind(binding.channel.as_bytes().as_slice())
        .bind(format!("sha256:{}",hex(&binding.map_digest)))
        .bind(format!("sha256:{}",hex(&binding.content_digest)))
        .bind(binding.frame_digest.as_slice()).bind(epoch.to_string())
        .bind(binding.scope_generation.to_string()).fetch_all(&mut **tx).await?;
    if rows.len()
        > usize::try_from(super::super::field_removal::CHANNEL_FIELDS_MAX)
            .map_err(|_| SpellItemError::Rejected("energy field ownership budget"))?
    {
        return Err(SpellItemError::Rejected("energy field ownership budget"));
    }
    let predecessors = rows
        .iter()
        .map(|row| {
            Ok(
                json!({"source_transaction_id":row.try_get::<String,_>("transaction_id")?,
            "ground_before":row.try_get::<Value,_>("ground_before")?}),
            )
        })
        .collect::<std::result::Result<Vec<_>, sqlx::Error>>()?;
    if predecessors.is_empty() {
        return Ok(());
    }
    let fact = node.fact();
    sqlx::query("INSERT INTO game_elemental_energy_field_adoptions(source_transaction_id,ownership_generation,holder_node_id,holder_registration_revision,ground_before) SELECT d.source_transaction_id,$2::text::numeric,encode($3,'hex')::uuid,$4::text::numeric,d.ground_before FROM jsonb_to_recordset($1::jsonb) AS d(source_transaction_id uuid,ground_before jsonb) ON CONFLICT(source_transaction_id,ownership_generation) DO NOTHING")
        .bind(json!(predecessors)).bind(binding.scope_generation.to_string())
        .bind(fact.node_id().as_bytes().as_slice()).bind(fact.registration_revision().to_string())
        .execute(&mut **tx).await?;
    Ok(())
}
