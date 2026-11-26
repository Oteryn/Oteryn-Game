//! The fenced player item-loss owner can materialize the receipt's corpse in its own transaction.
use super::*;
use crate::durability::item_transfer::CurrentCharacterItemFence;
use crate::durability::player_death_items::{PlayerDeathItemError, PlayerDeathItemRequest};

pub(in crate::durability) const PLAYER_DEATH_MINT_FAMILY: &str = "PlayerDeath";
pub(in crate::durability) const PLAYER_DEATH_MINT_CAUSE: &str = "player_death_corpse_mint";
// Dead Human, the existing generic human corpse appearance. Player-specific variant
// selection and timings require Global evidence.
const DEAD_HUMAN: &str = "oteryn:item.tibia.i4240";

pub(in crate::durability) async fn mint_player_death_corpse(
    tx: &mut audit::Type2Transaction<'_>,
    permit: &SpellLanePermit,
    node: &NodeIncarnationProof,
    fence: &CurrentCharacterItemFence,
    occurrence: [u8; 16],
    loss: &PlayerDeathItemRequest,
    ground: GroundPlacement,
) -> std::result::Result<([u8; 16], u64), PlayerDeathItemError> {
    // The caller holds the exact scope permit and the fenced Character root.
    // PlayerDeath uses the complete UUID in the cause's production key. The
    // legacy numeric actor columns are reserved sentinels for this family.
    let cause = ItemMintCause {
        death: CreatureDeathKey {
            world_id: fence.runtime_scope.world_id(),
            runtime_scope: fence.runtime_scope,
            scope_ownership_generation: fence.scope_ownership_generation,
            actor_local_id: 0,
            actor_local_generation: 1,
        },
        loot_table: TypedDefinitionRef {
            family: PLAYER_DEATH_MINT_FAMILY.into(),
            production_key: format!(
                "oteryn:death.{}",
                sqlx::types::Uuid::from_bytes(occurrence).simple()
            ),
            revision_ref: "definition-r1".into(),
        },
        purpose_key: CORPSE_MATERIALIZATION_PURPOSE_KEY.into(),
        draw_ordinal: 0,
        corpse_race: None,
    };
    let corpse = sqlx::query_scalar::<_, sqlx::types::Uuid>(
        "SELECT i.item_instance_id FROM game_item_mint_receipts r
         JOIN game_item_instances i USING(item_instance_id)
         JOIN game_item_ground_locations g USING(item_instance_id)
         WHERE r.loot_table_family='PlayerDeath' AND r.loot_table_production_key=$1
           AND r.loot_purpose_key='CORPSE_MATERIALIZATION' AND i.lifecycle=1
           AND g.runtime_scope_ownership_generation<=$2::text::numeric
         FOR UPDATE OF i",
    )
    .bind(&cause.loot_table.production_key)
    .bind(fence.scope_ownership_generation.get().to_string())
    .fetch_optional(&mut ***tx)
    .await?;
    let corpse = if let Some(corpse) = corpse {
        *corpse.as_bytes()
    } else {
        if corpse_cap_recount(tx, &cause.death).await? >= COMBAT01_CORPSES_PER_SCOPE_MAX {
            return Err(PlayerDeathItemError::StaleState);
        }
        let request = ItemMintRequest {
            cause,
            item: TypedDefinitionRef {
                family: "Item".into(),
                production_key: DEAD_HUMAN.into(),
                revision_ref: "definition-r1".into(),
            },
            quantity: 1,
            ground,
            content_revision: loss.content_revision.clone(),
            ruleset_revision: loss.ruleset_revision.clone(),
            sim_revision: loss.sim_revision.clone(),
        };
        let binding = intent_binding(&request, None).map_err(|_| PlayerDeathItemError::InvalidInput)?;
        let (transaction, event, item): (sqlx::types::Uuid, sqlx::types::Uuid, sqlx::types::Uuid) =
            sqlx::query_as("SELECT game_character_uuid_v7(), game_character_uuid_v7(), game_character_uuid_v7()")
                .fetch_one(&mut ***tx)
                .await?;
        let tuple = tx.tuple();
        let envelope = audit::encode_mint_event(
            MintEventIdentity {
                event_id: *event.as_bytes(),
                transaction_id: *transaction.as_bytes(),
                occurred_at_unix_ms: loss.occurred_at_unix_ms,
                server_build_id: &loss.server_build_id,
            },
            mint_message(&request, None, *item.as_bytes()),
            tuple.tuple(),
        )
        .map_err(|_| PlayerDeathItemError::InvalidInput)?;
        let fact = node.fact();
        let reservation = Reservation {
            transaction_id: *transaction.as_bytes(),
            event_id: *event.as_bytes(),
            item_instance_id: *item.as_bytes(),
            occurred_at_unix_ms: loss.occurred_at_unix_ms,
            envelope,
            fence_node_id: *fact.node_id().as_bytes(),
            fence_registration_revision: fact.registration_revision(),
            work_units_used: 0,
        };
        insert_reservation(tx, &request, &binding, &reservation, tuple).await?;
        let frozen = FrozenMint {
            request,
            transaction_id: reservation.transaction_id,
            event_id: reservation.event_id,
            item_instance_id: reservation.item_instance_id,
            occurred_at_unix_ms: reservation.occurred_at_unix_ms,
            envelope_sha256: Sha256::digest(&reservation.envelope).into(),
            envelope: reservation.envelope,
            intent_binding: binding,
        };
        // Global's damage-priority window excludes player bodies.
        insert_mint_with_corpse_attribution(tx, permit, &frozen, tuple, None).await?;
        frozen.item_instance_id
    };
    let ordinal: i64 = sqlx::query_scalar(
        "SELECT coalesce(max(placement_ordinal),0)::bigint+1
           FROM game_item_corpse_container_entries WHERE parent_item_instance_id=encode($1,'hex')::uuid",
    )
    .bind(corpse.as_slice())
    .fetch_one(&mut ***tx)
    .await?;
    Ok((corpse, u64::try_from(ordinal).map_err(|_| DurabilityError::InvalidStoredState)?))
}
