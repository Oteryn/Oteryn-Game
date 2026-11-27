//! A recovery mint can replace only an outstanding item with no surviving issuance.
use super::{CurrentCharacterGameplayFence, DurabilityError, RewardCallback};

const NPC: &str = "oteryn:quest-reward/qc-twenty-miles-";
const WORLD: &str = "oteryn:quest-callback/qc-twenty-miles-";

pub(super) async fn missing(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    fence: CurrentCharacterGameplayFence,
    callback: &RewardCallback,
    item: &str,
) -> Result<bool, DurabilityError> {
    let sources: &[(&str, &str)] = match item {
        "oteryn:item.tibia.i21770" => &[
            (WORLD, "pile-1"), (WORLD, "pile-2"), (WORLD, "pile-3"),
            (WORLD, "pile-4"), (WORLD, "pile-5"),
        ],
        "oteryn:item.tibia.i21746" => &[(NPC, "guardian-skulls-laid")],
        "oteryn:item.tibia.i21745" => &[(WORLD, "ring-chest")],
        "oteryn:item.tibia.i21753" => &[(WORLD, "shipwreck-chest"), (NPC, "guardian-third-slab")],
        "oteryn:item.tibia.i21861" => &[(WORLD, "shrine-chest"), (NPC, "guardian-second-slab-back")],
        "oteryn:item.tibia.i21862" => &[(NPC, "yeo-slab")],
        "oteryn:item.tibia.i21754" => &[(WORLD, "sea-ear-shell")],
        "oteryn:item.tibia.i21795" => &[(WORLD, "sundial")],
        _ => return Err(DurabilityError::InvalidStoredState),
    };
    let character = fence.character_id.as_bytes().as_slice();
    let claims: Vec<_> = sources.iter().map(|(prefix, key)| format!("{prefix}{key}")).collect();
    let recovery = callback.key.strip_suffix("-after-report").unwrap_or(&callback.key);
    let recovery_keys = [recovery.to_owned(), format!("{recovery}-after-report")];
    // The root lock serializes this Character's reward writers. Lock the exact
    // issued instances too, including those in other players' or Ground custody.
    // Follow quantity transfers in Item Loop receipts, including fresh split
    // instances and merges where the source remains live. Follow only transfers
    // from revisions at or after the issuance entered that particular stack.
    // An instance can be reached at multiple revisions; count its live quantity once.
    // Select live descendants in the CTE, then lock by identity so a concurrent
    // retirement of a selected descendant still refuses this attempt.
    let active: Vec<i64> = sqlx::query_scalar(
        "WITH RECURSIVE issues AS (
           SELECT claimed_transaction_id AS transaction_id FROM game_reward_claims
           WHERE character_id=encode($1,'hex')::uuid AND claim_family='RewardClaim'
             AND claim_production_key=ANY($2::text[])
           UNION
           SELECT mint_transaction_id FROM game_quest_reward_sources
           WHERE character_id=encode($1,'hex')::uuid AND transition_key=ANY($3::text[])
         ), lineage(item_instance_id, arrival_revision) AS (
           SELECT r.item_instance_id, 1::numeric
           FROM issues JOIN game_reward_claim_mint_receipts r USING(transaction_id)
           WHERE r.character_id=encode($1,'hex')::uuid
           UNION
           SELECT receiver.item_instance_id, COALESCE(receiver.state_revision_before + 1, 1)
           FROM lineage JOIN game_item_loop_move_lines source USING(item_instance_id)
           JOIN game_item_loop_move_lines receiver
             ON receiver.transaction_id=source.transaction_id
             AND receiver.quantity_after > receiver.quantity_before
           WHERE source.definition_production_key=$4
             AND source.state_revision_before >= lineage.arrival_revision
             AND source.quantity_after < source.quantity_before
         ), surviving AS MATERIALIZED (
           SELECT DISTINCT item_instance_id FROM lineage JOIN game_item_instances USING(item_instance_id)
           WHERE lifecycle=1
         )
         SELECT i.quantity FROM surviving JOIN game_item_instances i USING(item_instance_id)
         ORDER BY i.item_instance_id LIMIT 6 FOR UPDATE OF i",
    )
    .bind(character)
    .bind(&claims)
    .bind(&recovery_keys[..])
    .bind(item)
    .fetch_all(&mut **tx)
    .await?;
    let issued: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_reward_claims
         WHERE character_id=encode($1,'hex')::uuid AND claim_family='RewardClaim'
           AND claim_production_key=ANY($2::text[])",
    ).bind(character).bind(&claims).fetch_one(&mut **tx).await?;
    if item != "oteryn:item.tibia.i21770" {
        return Ok(issued > 0 && active.is_empty());
    }
    let offered: i64 = sqlx::query_scalar(
        "SELECT value FROM game_character_quest_tracks
         WHERE character_id=encode($1,'hex')::uuid
           AND track_key='oteryn:quest-progress/qc/twenty_miles/skulls'",
    ).bind(character).fetch_optional(&mut **tx).await?.unwrap_or(0);
    Ok(issued - offered > active.iter().sum::<i64>())
}
