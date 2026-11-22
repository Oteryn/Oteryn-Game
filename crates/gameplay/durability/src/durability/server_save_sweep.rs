//! Read-only server-save Ground census. Each page observes one ledger snapshot.
use super::db::{begin_observation_transaction, commit_semantic_transaction};
use super::item_loop_move::{ITEM_LOOP_DEPTH_MAX, ITEM_LOOP_TREE_ITEMS_MAX};
use super::item_mint::uuid_text;
use super::{DurabilityError, DurabilityRoot};
use crate::foundation::{ChannelId, WorldId};
use sqlx::{Postgres, Row, Transaction, postgres::PgRow};
use std::collections::BTreeSet;
use std::num::NonZeroU16;

const GROUND_PAGE_MAX: u16 = 64;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GroundSweepBinding {
    pub world: WorldId,
    pub channel: ChannelId,
    pub generation: u64,
    pub map_revision: String,
    pub content_revision: String,
    pub frame: Vec<u8>,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct GroundSweepPolicy {
    pub clean_protection_zones: bool,
}

/// Facts supplied by the owners of the pinned map and Item metadata. Lookups
/// must return unknown for absent or mismatched data. An incomplete house
/// catalogue cannot supply a negative house fact. An absent Item attribute can
/// supply a negative fact only when its source is known to cover that attribute.
pub trait GroundSweepFacts: Sync {
    fn binding(&self) -> &GroundSweepBinding;
    fn tile(&self, placement: &GroundSweepPlacement) -> GroundSweepTileFacts;
    fn item(&self, root: &GroundSweepRoot) -> GroundSweepItemFacts;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct GroundSweepTileFacts {
    pub house: Option<bool>,
    pub protection_zone: Option<bool>,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct GroundSweepItemFacts {
    pub loaded_from_map: Option<bool>,
    pub removable: Option<bool>,
    pub pickupable: Option<bool>,
    pub has_action_id: Option<bool>,
    pub has_unique_id: Option<bool>,
}

#[derive(Debug, Eq, PartialEq)]
pub struct GroundSweepPlacement {
    pub world: WorldId,
    pub channel: ChannelId,
    pub generation: u64,
    pub position: Vec<u8>,
    pub map_revision: String,
    pub content_revision: String,
    pub frame: Vec<u8>,
    pub stack_ordinal: Option<i64>,
    pub corpse_ref: Vec<u8>,
}

#[derive(Debug, Eq, PartialEq)]
pub struct GroundSweepRoot {
    pub item_id: [u8; 16],
    pub item_revision: u64,
    pub definition_family: String,
    pub definition_key: String,
    pub definition_revision: String,
    pub quantity: i64,
    pub placement: GroundSweepPlacement,
}

#[derive(Debug, Eq, PartialEq)]
pub struct GroundSweepEdge {
    pub item_id: [u8; 16],
    pub item_revision: u64,
    pub parent_id: [u8; 16],
    pub ordinal: u64,
    pub placed_transaction: [u8; 16],
    pub custody: GroundSweepCustody,
}

#[derive(Debug, Eq, PartialEq)]
pub enum GroundSweepCustody {
    Corpse,
    Nested,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GroundSweepPreserve {
    House,
    ProtectionZone,
    LoadedFromMap,
    ActionId,
    UniqueId,
    NotRemovable,
    NotPickupable,
}

#[derive(Debug, Eq, PartialEq)]
pub enum GroundSweepUnknown {
    Placement,
    TileFacts,
    ItemFacts,
    CustodyTree,
}

#[derive(Debug, Eq, PartialEq)]
pub enum GroundSweepDisposition {
    Eligible(Vec<GroundSweepEdge>),
    Preserved(GroundSweepPreserve),
    Unknown(GroundSweepUnknown),
}

#[derive(Debug, Eq, PartialEq)]
pub struct GroundSweepEntry {
    pub root: GroundSweepRoot,
    pub disposition: GroundSweepDisposition,
}

#[derive(Debug, Eq, PartialEq)]
pub struct GroundSweepPlan {
    pub entries: Vec<GroundSweepEntry>,
    pub next_after: Option<[u8; 16]>,
}

impl DurabilityRoot {
    /// Keyset pagination uses the last reported Item UUID. A continuation is
    /// another live observation. The page grants no authority to remove custody.
    #[allow(dead_code, reason = "W05a-2 wires the save-owner caller")]
    pub async fn plan_server_save_sweep(
        &self,
        mut facts: &impl GroundSweepFacts,
        policy: GroundSweepPolicy,
        after: Option<[u8; 16]>,
        page_size: NonZeroU16,
    ) -> Result<GroundSweepPlan, DurabilityError> {
        self.try_issue_semantic_pass()?
            .run_with_context(&mut facts, move |holder, deadline, facts| {
                Box::pin(async move {
                    let mut tx = begin_observation_transaction(holder, deadline).await?;
                    let plan = plan_page(&mut tx, *facts, policy, after, page_size).await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(plan)
                })
            })
            .await
    }
}

async fn plan_page(
    tx: &mut Transaction<'_, Postgres>,
    facts: &impl GroundSweepFacts,
    policy: GroundSweepPolicy,
    after: Option<[u8; 16]>,
    page_size: NonZeroU16,
) -> Result<GroundSweepPlan, DurabilityError> {
    let binding = facts.binding();
    let limit = GROUND_PAGE_MAX.min(page_size.get());
    let mut rows = sqlx::query(
        "SELECT g.item_instance_id::text AS item_id,i.state_revision::text AS item_revision,
                i.definition_family,i.definition_production_key,i.definition_revision_ref,i.quantity,
                g.runtime_scope_ownership_generation::text AS generation,g.spatial_position,g.map_revision,
                g.content_revision,g.native_room_placement_context,g.stack_ordinal,g.corpse_ref,
                EXISTS(SELECT 1 FROM game_house_tiles h
                       WHERE h.world_id=g.world_id AND h.spatial_position=g.spatial_position) AS house,
                (EXISTS(SELECT 1 FROM game_native_map_item_receipts n WHERE n.item_instance_id=g.item_instance_id)
                 OR EXISTS(SELECT 1 FROM game_map_item_mint_receipts m WHERE m.item_instance_id=g.item_instance_id)) AS authored
         FROM game_item_ground_locations g
         JOIN game_item_instances i USING(item_instance_id,world_id)
         WHERE g.world_id=encode($1,'hex')::uuid AND g.channel_id=encode($2,'hex')::uuid
           AND g.scope_kind=1 AND g.instance_id IS NULL AND i.lifecycle=1 AND i.quantity>0
           AND ($3::bytea IS NULL OR g.item_instance_id>encode($3,'hex')::uuid)
         ORDER BY g.item_instance_id LIMIT $4",
    )
    .bind(binding.world.as_bytes().as_slice())
    .bind(binding.channel.as_bytes().as_slice())
    .bind(after.as_ref().map(<[u8; 16]>::as_slice))
    .bind(i64::from(limit) + 1)
    .fetch_all(&mut **tx)
    .await?;
    let has_more = rows.len() > usize::from(limit);
    rows.truncate(usize::from(limit));
    let mut entries = Vec::with_capacity(rows.len());
    for row in rows {
        let root = GroundSweepRoot {
            item_id: uuid_text(row.try_get("item_id")?)?,
            item_revision: read_u64(&row, "item_revision")?,
            definition_family: row.try_get("definition_family")?,
            definition_key: row.try_get("definition_production_key")?,
            definition_revision: row.try_get("definition_revision_ref")?,
            quantity: row.try_get("quantity")?,
            placement: GroundSweepPlacement {
                world: binding.world,
                channel: binding.channel,
                generation: read_u64(&row, "generation")?,
                position: row.try_get("spatial_position")?,
                map_revision: row.try_get("map_revision")?,
                content_revision: row.try_get("content_revision")?,
                frame: row.try_get("native_room_placement_context")?,
                stack_ordinal: row.try_get("stack_ordinal")?,
                corpse_ref: row.try_get("corpse_ref")?,
            },
        };
        let disposition = match exclusion(facts, binding, policy, &root, &row)? {
            Some(excluded) => excluded,
            None => match read_tree(tx, &root).await? {
                Some(tree) => GroundSweepDisposition::Eligible(tree),
                None => GroundSweepDisposition::Unknown(GroundSweepUnknown::CustodyTree),
            },
        };
        entries.push(GroundSweepEntry { root, disposition });
    }
    let next_after = entries
        .last()
        .filter(|_| has_more)
        .map(|entry| entry.root.item_id);
    Ok(GroundSweepPlan {
        entries,
        next_after,
    })
}

fn exclusion(
    facts: &impl GroundSweepFacts,
    binding: &GroundSweepBinding,
    policy: GroundSweepPolicy,
    root: &GroundSweepRoot,
    row: &PgRow,
) -> Result<Option<GroundSweepDisposition>, DurabilityError> {
    use GroundSweepDisposition::{Preserved, Unknown};

    if row.try_get("house")? {
        return Ok(Some(Preserved(GroundSweepPreserve::House)));
    }
    if row.try_get("authored")? {
        return Ok(Some(Preserved(GroundSweepPreserve::LoadedFromMap)));
    }
    let placement = &root.placement;
    // Retained Ground may predate the current owner generation.
    if placement.generation > binding.generation
        || placement.map_revision != binding.map_revision
        || placement.content_revision != binding.content_revision
        || placement.frame != binding.frame
        || placement.stack_ordinal.is_none()
    {
        return Ok(Some(Unknown(GroundSweepUnknown::Placement)));
    }
    let tile = facts.tile(placement);
    match (tile.house, tile.protection_zone) {
        (Some(true), _) => return Ok(Some(Preserved(GroundSweepPreserve::House))),
        (_, Some(true)) if !policy.clean_protection_zones => {
            return Ok(Some(Preserved(GroundSweepPreserve::ProtectionZone)));
        }
        (Some(false), Some(_)) => {}
        _ => return Ok(Some(Unknown(GroundSweepUnknown::TileFacts))),
    }

    if root.definition_family != "Item" {
        return Ok(Some(Unknown(GroundSweepUnknown::ItemFacts)));
    }
    let item = facts.item(root);
    let criteria = [
        (
            item.loaded_from_map,
            false,
            GroundSweepPreserve::LoadedFromMap,
        ),
        (item.has_action_id, false, GroundSweepPreserve::ActionId),
        (item.has_unique_id, false, GroundSweepPreserve::UniqueId),
        (item.removable, true, GroundSweepPreserve::NotRemovable),
        (item.pickupable, true, GroundSweepPreserve::NotPickupable),
    ];
    if let Some((_, _, reason)) = criteria
        .into_iter()
        .find(|(fact, required, _)| fact.is_some_and(|value| value != *required))
    {
        return Ok(Some(Preserved(reason)));
    }
    if criteria.iter().any(|(fact, _, _)| fact.is_none()) {
        return Ok(Some(Unknown(GroundSweepUnknown::ItemFacts)));
    }
    Ok(None)
}

async fn read_tree(
    tx: &mut Transaction<'_, Postgres>,
    root: &GroundSweepRoot,
) -> Result<Option<Vec<GroundSweepEdge>>, DurabilityError> {
    // Stop at one extra depth or item beyond the supported bound, then reject
    // the entire tree. Direct backpack entries are character custody.
    let rows = sqlx::query(
        "WITH RECURSIVE tree(item,parent,ordinal,placement,corpse,depth) AS (
           SELECT encode($1,'hex')::uuid,NULL::uuid,NULL::numeric,NULL::uuid,false,0
           UNION ALL
           SELECT e.item_instance_id,t.item,e.placement_ordinal,e.placed_transaction_id,e.corpse,t.depth+1
           FROM tree t CROSS JOIN LATERAL (
             SELECT item_instance_id,placement_ordinal,placed_transaction_id,false AS corpse
             FROM game_item_nested_entries WHERE parent_item_instance_id=t.item
             UNION ALL
             SELECT item_instance_id,placement_ordinal,placed_transaction_id,true
             FROM game_item_corpse_container_entries WHERE parent_item_instance_id=t.item
           ) e WHERE t.depth<$3
         ), bounded AS MATERIALIZED (SELECT * FROM tree LIMIT $4)
         SELECT t.item::text,t.parent::text,t.ordinal::text,t.placement::text,t.corpse,t.depth,
                i.state_revision::text AS item_revision,
                (i.item_instance_id IS NULL OR i.lifecycle<>1 OR i.quantity<=0
                 OR EXISTS(SELECT 1 FROM game_item_container_entries e WHERE e.parent_item_instance_id=t.item)
                 OR EXISTS(SELECT 1 FROM game_item_ground_locations g WHERE g.item_instance_id=t.item AND t.depth>0)
                 OR EXISTS(SELECT 1 FROM game_item_house_interior_locations h WHERE h.item_instance_id=t.item)) AS invalid
         FROM bounded t LEFT JOIN game_item_instances i
           ON i.item_instance_id=t.item AND i.world_id=encode($2,'hex')::uuid
         ORDER BY t.depth,t.item",
    )
    .bind(root.item_id.as_slice())
    .bind(root.placement.world.as_bytes().as_slice())
    .bind(ITEM_LOOP_DEPTH_MAX + 1)
    .bind(ITEM_LOOP_TREE_ITEMS_MAX + 1)
    .fetch_all(&mut **tx)
    .await?;
    if rows.len() > ITEM_LOOP_TREE_ITEMS_MAX as usize {
        return Ok(None);
    }
    let mut tree = Vec::with_capacity(rows.len().saturating_sub(1));
    let mut seen = BTreeSet::new();
    for row in rows {
        let depth: i32 = row.try_get("depth")?;
        let item_id = uuid_text(row.try_get("item")?)?;
        if row.try_get::<bool, _>("invalid")?
            || depth > ITEM_LOOP_DEPTH_MAX
            || !seen.insert(item_id)
        {
            return Ok(None);
        }
        if depth == 0 {
            continue;
        }
        tree.push(GroundSweepEdge {
            item_id,
            item_revision: read_u64(&row, "item_revision")?,
            parent_id: uuid_text(row.try_get("parent")?)?,
            ordinal: read_u64(&row, "ordinal")?,
            placed_transaction: uuid_text(row.try_get("placement")?)?,
            custody: if row.try_get("corpse")? {
                GroundSweepCustody::Corpse
            } else {
                GroundSweepCustody::Nested
            },
        });
    }
    Ok(Some(tree))
}

fn read_u64(row: &PgRow, column: &str) -> Result<u64, DurabilityError> {
    row.try_get::<&str, _>(column)?
        .parse()
        .map_err(|_| DurabilityError::InvalidStoredState)
}
