//! Ground census and fenced Channel settlement for durable server saves.
use super::db::{
    begin_observation_transaction, begin_semantic_transaction, commit_semantic_transaction,
};
use super::item_loop_move::{ITEM_LOOP_DEPTH_MAX, ITEM_LOOP_TREE_ITEMS_MAX};
use super::item_mint::uuid_text;
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::spell_item_transaction::assert_creature_event_scope_in_transaction;
use super::spell_owner_commit::SpellLanePermit;
use super::{DurabilityError, DurabilityRoot};
use crate::foundation::{ChannelId, RuntimeScopeRefV1, ScopeOwnershipGeneration, WorldId};
use serde_json::{Value, json};
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
    fn alternate_map_revision(&self) -> Option<&str> {
        None
    }
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
    pub runtime_without_source_ids: bool,
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
    depth: i32,
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
                    let (entries, next_after) = census_page(
                        &mut tx,
                        *facts,
                        policy,
                        CensusPosition::After(after),
                        page_size,
                    )
                    .await?;
                    let plan = GroundSweepPlan {
                        entries: entries.into_iter().map(|(entry, _)| entry).collect(),
                        next_after,
                    };
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(plan)
                })
            })
            .await
    }
}

enum CensusPosition {
    After(Option<[u8; 16]>),
    Exact([u8; 16]),
}

type CensusPage = (Vec<(GroundSweepEntry, Value)>, Option<[u8; 16]>);

async fn census_page(
    tx: &mut Transaction<'_, Postgres>,
    facts: &impl GroundSweepFacts,
    policy: GroundSweepPolicy,
    position: CensusPosition,
    page_size: NonZeroU16,
) -> Result<CensusPage, DurabilityError> {
    let (key, exact) = match position {
        CensusPosition::After(after) => (after, false),
        CensusPosition::Exact(id) => (Some(id), true),
    };
    let binding = facts.binding();
    let limit = GROUND_PAGE_MAX.min(page_size.get());
    let mut rows = sqlx::query(
        "SELECT g.item_instance_id::text AS item_id,i.state_revision::text AS item_revision,
                i.definition_family,i.definition_production_key,i.definition_revision_ref,i.quantity,
                to_jsonb(i) AS item_evidence,to_jsonb(g) AS ground_evidence,
                (EXISTS(SELECT 1 FROM game_item_nested_entries e WHERE e.parent_item_instance_id=i.item_instance_id)
                 OR EXISTS(SELECT 1 FROM game_item_corpse_container_entries e WHERE e.parent_item_instance_id=i.item_instance_id)
                 OR EXISTS(SELECT 1 FROM game_item_container_entries e WHERE e.parent_item_instance_id=i.item_instance_id)
                 OR EXISTS(SELECT 1 FROM game_item_house_interior_locations h WHERE h.item_instance_id=i.item_instance_id)) AS inspect_tree,
                (EXISTS(SELECT 1 FROM game_item_mint_receipts m WHERE m.item_instance_id=i.item_instance_id
                           AND m.transaction_id=i.minted_transaction_id AND m.loot_purpose_key<>'CORPSE_MATERIALIZATION')
                 OR EXISTS(SELECT 1 FROM game_character_starter_grants m WHERE m.item_instance_id=i.item_instance_id
                           AND m.transaction_id=i.minted_transaction_id AND m.outcome=1)) AS runtime_without_source_ids,
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
           AND g.item_instance_id>=COALESCE(encode($3::bytea,'hex')::uuid,'00000000-0000-0000-0000-000000000000'::uuid)
           AND (CASE WHEN $5 THEN g.item_instance_id=encode($3,'hex')::uuid
                 ELSE g.item_instance_id IS DISTINCT FROM encode($3,'hex')::uuid END)
         ORDER BY g.item_instance_id LIMIT $4",
    )
    .bind(binding.world.as_bytes().as_slice())
    .bind(binding.channel.as_bytes().as_slice())
    .bind(key.as_ref().map(<[u8; 16]>::as_slice))
    .bind(i64::from(limit) + 1)
    .bind(exact)
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
            runtime_without_source_ids: row.try_get("runtime_without_source_ids")?,
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
            None if root.item_revision == u64::MAX => {
                GroundSweepDisposition::Unknown(GroundSweepUnknown::CustodyTree)
            }
            None if !row.try_get::<bool, _>("inspect_tree")? => {
                GroundSweepDisposition::Eligible(Vec::new())
            }
            None => match read_tree(tx, &root).await? {
                Some(tree) => GroundSweepDisposition::Eligible(tree),
                None => GroundSweepDisposition::Unknown(GroundSweepUnknown::CustodyTree),
            },
        };
        let tree = match &disposition {
            GroundSweepDisposition::Eligible(tree) => Some(
                tree.iter()
                    .map(|edge| {
                        json!({
                            "item_id": hex(&edge.item_id),
                            "item_revision": edge.item_revision,
                            "parent_id": hex(&edge.parent_id),
                            "ordinal": edge.ordinal,
                            "placed_transaction": hex(&edge.placed_transaction),
                            "corpse": edge.custody == GroundSweepCustody::Corpse,
                            "depth": edge.depth,
                        })
                    })
                    .collect::<Vec<_>>(),
            ),
            _ => None,
        };
        let evidence = json!({
            "item": row.try_get::<Value, _>("item_evidence")?,
            "ground": row.try_get::<Value, _>("ground_evidence")?,
            "tree": tree,
        });
        entries.push((GroundSweepEntry { root, disposition }, evidence));
    }
    let next_after = entries
        .last()
        .filter(|_| has_more)
        .map(|(entry, _)| entry.root.item_id);
    Ok((entries, next_after))
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
        || (placement.map_revision != binding.map_revision
            && facts.alternate_map_revision() != Some(placement.map_revision.as_str()))
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
        let corpse = row.try_get::<bool, _>("corpse")?;
        let item_revision = read_u64(&row, "item_revision")?;
        if (corpse && depth != 1) || item_revision == u64::MAX {
            return Ok(None);
        }
        tree.push(GroundSweepEdge {
            item_id,
            depth,
            item_revision,
            parent_id: uuid_text(row.try_get("parent")?)?,
            ordinal: read_u64(&row, "ordinal")?,
            placed_transaction: uuid_text(row.try_get("placement")?)?,
            custody: if corpse {
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

/// Immutable policy captured by the save publisher at the first observation of a slot.
#[derive(Clone, Copy, Debug, Default)]
pub struct ServerSaveCleanup {
    pub enabled: bool,
    pub policy: GroundSweepPolicy,
}

#[derive(Default)]
pub struct ServerSavePoll {
    pub needs_pass: bool,
    pub latest_retirement: Option<[u8; 16]>,
}

/// Current runtime evidence. The epoch map revision names its content issuance,
/// while the planner's map revision is the Item ledger's placement codec.
pub struct ServerSaveOwner<'a> {
    pub node: &'a NodeIncarnationProof,
    pub epoch: u64,
    pub epoch_map_revision: &'a str,
    pub server_digest: [u8; 32],
}

impl DurabilityRoot {
    /// Save owner publication is independent of the warning read and the Item lane.
    /// Duplicate World publishers converge on one immutable scheduled occurrence.
    pub async fn publish_due_server_save(
        &self,
        binding: &GroundSweepBinding,
        owner: &ServerSaveOwner<'_>,
        cleanup: ServerSaveCleanup,
    ) -> Result<ServerSavePoll, DurabilityError> {
        let world = binding.world;
        let channel = binding.channel;
        let scope = RuntimeScopeRefV1::channel(world, channel);
        let generation = ScopeOwnershipGeneration::new(binding.generation)
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        let mut context = owner;
        self.try_issue_semantic_pass()?
            .run_with_context(&mut context, move |holder, deadline, owner| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    super::db::lock_admission_relations(&mut tx).await?;
                    if !super::character_progression::assert_runtime_scope_fence(&mut tx, scope, generation, owner.node).await? {
                        return Ok(ServerSavePoll::default());
                    }
                    sqlx::query("SELECT game_server_save_publish_due(encode($1,'hex')::uuid,$2::text::numeric,$3,$4)")
                        .bind(world.as_bytes().as_slice()).bind(owner.epoch.to_string())
                        .bind(cleanup.enabled).bind(cleanup.policy.clean_protection_zones)
                        .execute(&mut *tx).await?;
                    let (needs_pass, latest): (bool, Option<String>) = sqlx::query_as("SELECT EXISTS (
                        SELECT 1 FROM game_server_save_occurrences s
                        LEFT JOIN game_server_save_channels c ON c.occurrence_id=s.occurrence_id AND c.channel_id=encode($2,'hex')::uuid
                        WHERE s.world_id=encode($1,'hex')::uuid AND s.reset_epoch=$3::text::numeric AND s.cleanup_enabled
                          AND (c.state='frozen' OR c.state IS NULL)),
                        (SELECT transaction_id::text FROM game_spell_item_receipts
                         WHERE world_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid AND cause_kind=7
                         ORDER BY transaction_id DESC LIMIT 1)")
                        .bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice())
                        .bind(owner.epoch.to_string()).fetch_one(&mut *tx).await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(ServerSavePoll {
                        needs_pass,
                        latest_retirement: latest.as_deref().map(uuid_text).transpose()?,
                    })
                })
            })
            .await
    }

    /// Freeze one cohort or settle one complete tree. Every pass re-proves the
    /// current owner before acquiring the physical Item lock, including replay.
    pub async fn run_server_save_sweep(
        &self,
        owner: &ServerSaveOwner<'_>,
        permit: &SpellLanePermit,
        facts: &impl GroundSweepFacts,
    ) -> Result<(), DurabilityError> {
        let mut context = (owner, permit, facts);
        self.try_issue_semantic_pass()?
            .run_with_context(&mut context, |holder, deadline, context| {
                Box::pin(async move {
                    let (owner, permit, facts) = *context;
                    let binding = facts.binding();
                    let scope = RuntimeScopeRefV1::Channel {
                        world_id: binding.world,
                        channel_id: binding.channel,
                    };
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_creature_event_scope_in_transaction(
                        &mut tx,
                        permit,
                        owner.node,
                        scope,
                        binding.generation,
                    )
                    .await
                    .map_err(|_| DurabilityError::Unavailable)?;
                    let pinned: bool = sqlx::query_scalar(
                        "SELECT game_server_save_pin(encode($1,'hex')::uuid,
                   encode($2,'hex')::uuid,$3::text::numeric,$4,$5,$6)",
                    )
                    .bind(binding.world.as_bytes().as_slice())
                    .bind(binding.channel.as_bytes().as_slice())
                    .bind(owner.epoch.to_string())
                    .bind(owner.epoch_map_revision)
                    .bind(owner.server_digest.as_slice())
                    .bind(&binding.frame)
                    .fetch_one(&mut *tx)
                    .await?;
                    if !pinned {
                        return Ok(());
                    }
                    sqlx::query("SET LOCAL jit = off").execute(&mut *tx).await?;
                    sqlx::query("SET LOCAL plan_cache_mode = force_generic_plan")
                        .execute(&mut *tx)
                        .await?;
                    let pass = SweepPass {
                        owner,
                        binding,
                        facts,
                    };
                    pass.advance(&mut tx).await?;
                    commit_semantic_transaction(tx, deadline).await
                })
            })
            .await
    }
}

struct SweepPass<'a, F> {
    owner: &'a ServerSaveOwner<'a>,
    binding: &'a GroundSweepBinding,
    facts: &'a F,
}

impl<F: GroundSweepFacts> SweepPass<'_, F> {
    async fn advance(&self, tx: &mut Transaction<'_, Postgres>) -> Result<(), DurabilityError> {
        let next: Option<(String, Option<String>, bool)> = sqlx::query_as(
            "SELECT s.occurrence_id::text,c.state,s.clean_protection_zones
             FROM game_server_save_occurrences s
             LEFT JOIN game_server_save_channels c ON c.occurrence_id=s.occurrence_id AND c.channel_id=encode($2,'hex')::uuid
             WHERE s.world_id=encode($1,'hex')::uuid AND s.reset_epoch=$3::text::numeric AND s.cleanup_enabled AND s.scheduled_at<=clock_timestamp()
               AND (s.map_revision,s.server_digest,s.frame_digest)=($4,$5,$6)
               AND (c.state='frozen' OR c.state IS NULL)
             ORDER BY (c.state IS NULL AND clock_timestamp()<s.freeze_until) DESC NULLS LAST,s.scheduled_at
             LIMIT 1")
            .bind(self.binding.world.as_bytes().as_slice())
            .bind(self.binding.channel.as_bytes().as_slice())
            .bind(self.owner.epoch.to_string()).bind(self.owner.epoch_map_revision)
            .bind(self.owner.server_digest.as_slice()).bind(&self.binding.frame)
            .fetch_optional(&mut **tx).await?;
        let Some((occurrence, state, clean_protection_zones)) = next else {
            return Ok(());
        };
        let policy = GroundSweepPolicy {
            clean_protection_zones,
        };
        if state.is_some() {
            self.settle(tx, &occurrence, policy).await?;
        } else {
            self.freeze(tx, &occurrence, policy).await?;
        }
        Ok(())
    }

    async fn freeze(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        occurrence: &str,
        policy: GroundSweepPolicy,
    ) -> Result<(), DurabilityError> {
        let state: String = sqlx::query_scalar(
            "INSERT INTO game_server_save_channels(occurrence_id,channel_id,state)
             SELECT occurrence_id,encode($2,'hex')::uuid,
               CASE WHEN clock_timestamp()<freeze_until THEN 'frozen' ELSE 'missed' END
             FROM game_server_save_occurrences WHERE occurrence_id=$1::uuid RETURNING state",
        )
        .bind(occurrence)
        .bind(self.binding.channel.as_bytes().as_slice())
        .fetch_one(&mut **tx)
        .await?;
        if state == "missed" {
            return Ok(());
        }
        let mut after = None;
        loop {
            let (entries, next) = census_page(
                tx,
                self.facts,
                policy,
                CensusPosition::After(after),
                NonZeroU16::MAX,
            )
            .await?;
            let entries = entries.into_iter().map(|(entry, evidence)| {
                let (disposition, reason) = match &entry.disposition {
                    GroundSweepDisposition::Eligible(tree) if tree.len() < 256 => ("pending", None),
                    GroundSweepDisposition::Eligible(_) => ("unknown", Some("TreeWorkLimit".into())),
                    GroundSweepDisposition::Preserved(reason) => ("preserved", Some(format!("{reason:?}"))),
                    GroundSweepDisposition::Unknown(reason) => ("unknown", Some(format!("{reason:?}"))),
                };
                json!({"item_id":hex(&entry.root.item_id),"evidence":evidence,"disposition":disposition,"reason":reason})
            }).collect::<Vec<_>>();
            sqlx::query("INSERT INTO game_server_save_ground(occurrence_id,channel_id,item_id,evidence,disposition,reason)
                SELECT $1::uuid,encode($2,'hex')::uuid,item_id,evidence,disposition,reason FROM
                  jsonb_to_recordset($3) AS e(item_id uuid,evidence jsonb,disposition text,reason text)")
                .bind(occurrence).bind(self.binding.channel.as_bytes().as_slice()).bind(json!(entries))
                .execute(&mut **tx).await?;
            if next.is_none() {
                break;
            }
            after = next;
        }
        // Even a census whose pages ran past the deadline leaves no cohort.
        let in_window: bool = sqlx::query_scalar(
            "SELECT clock_timestamp()<freeze_until FROM game_server_save_occurrences WHERE occurrence_id=$1::uuid")
            .bind(occurrence).fetch_one(&mut **tx).await?;
        if !in_window {
            return Err(DurabilityError::Unavailable);
        }
        self.complete_if_settled(tx, occurrence).await
    }

    async fn settle(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        occurrence: &str,
        policy: GroundSweepPolicy,
    ) -> Result<(), DurabilityError> {
        let channel = self.binding.channel.as_bytes().as_slice();
        let candidate: Option<(String, Value)> = sqlx::query_as(
            "SELECT item_id::text,evidence FROM game_server_save_ground
             WHERE occurrence_id=$1::uuid AND channel_id=encode($2,'hex')::uuid AND disposition='pending'
             ORDER BY item_id LIMIT 1 FOR UPDATE")
            .bind(occurrence).bind(channel).fetch_optional(&mut **tx).await?;
        if let Some((item, evidence)) = candidate {
            let locked: bool = sqlx::query_scalar("SELECT game_server_save_lock_root($1::uuid,$2)")
                .bind(&item)
                .bind(&evidence)
                .fetch_one(&mut **tx)
                .await?;
            let reason = if locked {
                self.recheck(tx, &item, &evidence, policy).await?
            } else {
                Some("PlacementOrItemChanged".into())
            };
            if let Some(reason) = reason {
                sqlx::query("UPDATE game_server_save_ground SET disposition='skipped',reason=$4
                   WHERE occurrence_id=$1::uuid AND channel_id=encode($2,'hex')::uuid AND item_id=$3::uuid")
                    .bind(occurrence).bind(channel).bind(&item).bind(reason).execute(&mut **tx).await?;
            } else {
                sqlx::query("SELECT game_server_save_retire_tree($1::uuid,encode($2,'hex')::uuid,$3::uuid,$4::text::numeric)")
                    .bind(occurrence).bind(channel).bind(&item).bind(self.binding.generation.to_string())
                    .execute(&mut **tx).await?;
            }
        }
        self.complete_if_settled(tx, occurrence).await
    }

    async fn recheck(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        item: &str,
        evidence: &Value,
        policy: GroundSweepPolicy,
    ) -> Result<Option<String>, DurabilityError> {
        // Lock even a descendant whose revision changed. The re-read classifies
        // the whole tree after those locks and the same scope lock as pickup/drop.
        sqlx::query("SELECT i.item_instance_id FROM game_item_instances i
            JOIN jsonb_to_recordset($1::jsonb->'tree') AS t(item_id uuid) ON t.item_id=i.item_instance_id
            ORDER BY i.item_instance_id FOR UPDATE OF i")
            .bind(evidence).fetch_all(&mut **tx).await?;
        let id = uuid_text(item)?;
        let (mut entries, _) = census_page(
            tx,
            self.facts,
            policy,
            CensusPosition::Exact(id),
            NonZeroU16::MIN,
        )
        .await?;
        Ok(match entries.pop() {
            Some((entry, current)) if entry.root.item_id == id => match entry.disposition {
                GroundSweepDisposition::Eligible(_) if &current == evidence => None,
                GroundSweepDisposition::Eligible(_) => Some("TreeChanged".into()),
                GroundSweepDisposition::Preserved(reason) => Some(format!("{reason:?}")),
                GroundSweepDisposition::Unknown(reason) => Some(format!("{reason:?}")),
            },
            _ => Some("PlacementOrItemChanged".into()),
        })
    }

    async fn complete_if_settled(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        occurrence: &str,
    ) -> Result<(), DurabilityError> {
        sqlx::query("UPDATE game_server_save_channels c SET state='complete'
            WHERE c.occurrence_id=$1::uuid AND c.channel_id=encode($2,'hex')::uuid AND c.state='frozen'
            AND NOT EXISTS(SELECT 1 FROM game_server_save_ground g
                WHERE (g.occurrence_id,g.channel_id)=(c.occurrence_id,c.channel_id) AND g.disposition='pending')")
            .bind(occurrence).bind(self.binding.channel.as_bytes().as_slice())
            .execute(&mut **tx).await?;
        Ok(())
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
