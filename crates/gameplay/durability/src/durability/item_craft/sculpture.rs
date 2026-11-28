use super::*;
use crate::character_recovery_fence::CharacterRecoveryFenceV1;
use super::super::ground_cell_limit::{GroundCellAdmissionError, GroundCellLimitError};
use super::super::item_mint::{GroundPlacement, uuid_text};
use super::super::item_mint_audit::Type2Transaction;
use super::super::spell_item_transaction::SpellItemError;
use super::super::spell_owner_commit::SpellLanePermit;

/// Current map custody for an on-ground sculpture material.
#[derive(Clone, Debug)]
pub struct SculptureGroundTarget {
    pub position: Vec<u8>,
    pub ordinal: u64,
}

/// Actor tile for a marble yield that cannot be carried, and an optional dropped target.
#[derive(Clone, Debug)]
pub struct GroundSculpture {
    pub actor: GroundPlacement,
    pub target: Option<SculptureGroundTarget>,
}

pub(super) struct GroundContext<'a> {
    pub operand: &'a GroundSculpture,
    pub recovery: &'a CharacterRecoveryFenceV1,
    pub node: &'a NodeIncarnationProof,
}

impl GroundContext<'_> {
    pub(super) fn placement(&self, ice: bool) -> GroundPlacement {
        let mut placement = self.operand.actor.clone();
        if let Some(target) = self.operand.target.as_ref().filter(|_| ice) {
            placement.spatial_position.clone_from(&target.position);
        }
        placement
    }

    pub(super) async fn check_room(
        &self,
        tx: &mut Type2Transaction<'_>,
        permit: &SpellLanePermit,
        fence: CurrentCharacterItemFence,
        placement: &GroundPlacement,
        output: &Value,
    ) -> Result<Result<(), UseDisposition>, DurabilityError> {
        let id = uuid_text(output["item"].as_str().ok_or(DurabilityError::InvalidStoredState)?)?;
        match super::super::ground_cell_limit::check_ground_cell_root_insert_with_recovery(
            tx, permit, self.recovery, self.node,
            (fence.runtime_scope, fence.scope_ownership_generation.get()),
            placement, &id,
        ).await {
            Ok(()) => Ok(Ok(())),
            Err(GroundCellAdmissionError::Limit(GroundCellLimitError::NotEnoughRoom)) =>
                Ok(Err(UseDisposition::NoRoom)),
            Err(GroundCellAdmissionError::Limit(GroundCellLimitError::IncompatibleInput)) =>
                Ok(Err(UseDisposition::NotPossible)),
            Err(GroundCellAdmissionError::Source(SpellItemError::Rejected(_) | SpellItemError::FieldCapabilityRefused)) =>
                Ok(Err(UseDisposition::Rejected)),
            Err(GroundCellAdmissionError::Source(SpellItemError::Database(error))) => Err(error.into()),
            Err(GroundCellAdmissionError::Source(SpellItemError::Durability(error))) => Err(error),
            Err(GroundCellAdmissionError::Source(SpellItemError::Training(_))) => Err(DurabilityError::Unavailable),
        }
    }
}

pub(super) async fn lock_ground_material(
    tx: &mut Type2Transaction<'_>,
    fence: CurrentCharacterItemFence,
    material: [u8; 16],
    target: &SculptureGroundTarget,
    context: &GroundSculpture,
) -> Result<Option<sqlx::postgres::PgRow>, DurabilityError> {
    let (world, channel) = scope_of(&fence).map_err(|_| DurabilityError::InvalidStoredState)?;
    Ok(sqlx::query(
        "SELECT i.quantity,i.definition_family,i.definition_production_key,i.definition_revision_ref,to_jsonb(g) AS ground_source
           FROM game_item_instances i JOIN game_item_ground_locations g USING(item_instance_id,world_id)
          WHERE i.item_instance_id=encode($1,'hex')::uuid AND i.world_id=encode($2,'hex')::uuid
            AND i.lifecycle=1 AND i.quantity=1 AND g.channel_id=encode($3,'hex')::uuid
            AND g.runtime_scope_ownership_generation=$4::text::numeric AND g.spatial_position=$5
            AND g.stack_ordinal=$6 AND g.map_revision=$7 AND g.content_revision=$8
            AND g.native_room_placement_context=$9
            AND (game_item_loop_drop_ref(g.corpse_ref) OR game_item_action_ground_source(i.item_instance_id))
          FOR UPDATE OF i",
    )
    .bind(material.as_slice()).bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice())
    .bind(fence.scope_ownership_generation.get().to_string()).bind(&target.position)
    .bind(i64::try_from(target.ordinal).map_err(|_| DurabilityError::InvalidStoredState)?)
    .bind(&context.actor.map_revision).bind(&context.actor.content_revision)
    .bind(&context.actor.native_room_placement_context)
    .fetch_optional(&mut ***tx).await?)
}

pub(super) fn ground_proof(
    fence: CurrentCharacterItemFence,
    placement: &GroundPlacement,
) -> Result<Value, DurabilityError> {
    let (_, channel) = scope_of(&fence).map_err(|_| DurabilityError::InvalidStoredState)?;
    let hex = |bytes: &[u8]| bytes.iter().map(|byte| format!("{byte:02x}")).collect::<String>();
    Ok(json!({
        "channel": hex(channel.as_bytes()),
        "generation": fence.scope_ownership_generation.get().to_string(),
        "position": hex(&placement.spatial_position),
        "map_revision": placement.map_revision,
        "content_revision": placement.content_revision,
        "frame": hex(&placement.native_room_placement_context),
    }))
}

pub(super) async fn place_ground(
    tx: &mut Type2Transaction<'_>,
    permit: &crate::durability::spell_owner_commit::SpellLanePermit,
    fence: CurrentCharacterItemFence,
    output: &Value,
    placement: &GroundPlacement,
) -> Result<(), DurabilityError> {
    permit.check_scope(fence.runtime_scope)?;
    let (world, channel) = scope_of(&fence).map_err(|_| DurabilityError::InvalidStoredState)?;
    sqlx::query(
        "INSERT INTO game_item_ground_locations(item_instance_id,world_id,channel_id,
            runtime_scope_ownership_generation,spatial_position,corpse_ref,map_revision,content_revision,native_room_placement_context)
         VALUES(($1->>'item')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4::text::numeric,$5,$6,$7,$8,$9)",
    )
    .bind(output).bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice())
    .bind(fence.scope_ownership_generation.get().to_string()).bind(&placement.spatial_position)
    .bind(&placement.corpse_ref).bind(&placement.map_revision).bind(&placement.content_revision)
    .bind(&placement.native_room_placement_context)
    .execute(&mut ***tx).await?;
    Ok(())
}

/// Try managed obtain routing, then ordinary carried slots, before consuming the marble.
pub(super) async fn nested_marble_destination<C: ItemLoopContent>(
    tx: &mut Type2Transaction<'_>,
    character: crate::domain::CharacterId,
    backpack: &[u8],
    main_has_room: bool,
    output: &TypedDefinitionRef,
    content: &C,
) -> Result<Option<(String, u64)>, DurabilityError> {
    use oteryn_gameplay_items::quick_loot::Recipient;

    let targets = super::super::character_quick_loot::load_obtain(tx, character.as_bytes()).await?;
    let main: [u8; 16] = backpack.try_into().map_err(|_| DurabilityError::InvalidStoredState)?;
    let target = |recipient| match recipient {
        Recipient::Container(id) => Some(id),
        Recipient::MainContainer => Some(main),
        Recipient::GoldPouch(_) => None,
    };
    let category = super::super::item_loop_move::preferred_category(content, output, |category| {
        targets.contains_key(&Some(category))
    });
    let category = targets.get(&Some(category)).copied().and_then(target);
    let unassigned = targets.get(&None).copied().and_then(target);
    if main_has_room && category.or(unassigned).is_none_or(|id| id == main) {
        return Ok(None);
    }
    // Negative ordinals keep newest siblings first and each parent before its descendants.
    let rows = sqlx::query(
        "WITH RECURSIVE tree(item, path, ordinals) AS (
           SELECT encode($1,'hex')::uuid, ARRAY[encode($1,'hex')::uuid], ARRAY[]::numeric[]
           UNION ALL
           SELECT e.item_instance_id, t.path || e.item_instance_id, t.ordinals || (-e.placement_ordinal)
             FROM tree t JOIN LATERAL (
               SELECT item_instance_id, placement_ordinal FROM game_item_container_entries
                WHERE parent_item_instance_id=t.item AND cardinality(t.ordinals)=0
               UNION ALL
               SELECT item_instance_id, placement_ordinal FROM game_item_nested_entries
                WHERE parent_item_instance_id=t.item AND cardinality(t.ordinals)>0
             ) e ON true WHERE cardinality(t.ordinals)<7
         ), preferred AS (
           SELECT coalesce(
             (SELECT item FROM tree WHERE item=encode($2,'hex')::uuid),
             (SELECT item FROM tree WHERE item=encode($3,'hex')::uuid)
           ) AS item
         )
         SELECT tree.item::text AS item, cardinality(ordinals)=0 AS main,
                i.definition_family, i.definition_production_key, i.definition_revision_ref
           FROM tree JOIN game_item_instances i ON i.item_instance_id=tree.item CROSS JOIN preferred
          WHERE i.lifecycle=1
          ORDER BY CASE WHEN preferred.item=ANY(path) THEN 0 ELSE 1 END,
                   CASE WHEN preferred.item=ANY(path) THEN 0 ELSE cardinality(ordinals) END,
                   ordinals LIMIT 501",
    )
    .bind(backpack)
    .bind(category.as_ref().map(<[u8; 16]>::as_slice))
    .bind(unassigned.as_ref().map(<[u8; 16]>::as_slice))
    .fetch_all(&mut ***tx)
    .await?;
    for row in rows {
        if row.try_get::<bool, _>("main")? {
            if main_has_room {
                return Ok(None);
            }
            continue;
        }
        let definition = TypedDefinitionRef {
            family: row.try_get("definition_family")?,
            production_key: row.try_get("definition_production_key")?,
            revision_ref: row.try_get("definition_revision_ref")?,
        };
        let Some(capacity) = content
            .loop_facts(&definition)
            .and_then(|f| f.container_capacity)
        else {
            continue;
        };
        if content.is_quiver(&definition) {
            continue;
        }
        let parent: String = row.try_get("item")?;
        let (occupied, highest): (i64, String) = sqlx::query_as(
            "SELECT count(*), coalesce(max(placement_ordinal),0)::text
               FROM game_item_nested_entries WHERE parent_item_instance_id=$1::uuid",
        )
        .bind(&parent)
        .fetch_one(&mut ***tx)
        .await?;
        if occupied < i64::from(capacity.min(20)) {
            let ordinal = highest
                .parse::<u64>()
                .ok()
                .and_then(|ordinal| ordinal.checked_add(1))
                .ok_or(DurabilityError::InvalidStoredState)?;
            return Ok(Some((parent, ordinal)));
        }
    }
    Ok(None)
}
