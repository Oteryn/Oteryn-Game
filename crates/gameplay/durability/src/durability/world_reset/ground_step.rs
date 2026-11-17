//! The next Ground step is derived from durable locations, never a caller's list.
use super::*;
use crate::durability::item_decay_retire::DecayRetireStep;
use crate::durability::item_mint::uuid_text;

pub(super) async fn next(
    tx: &mut Transaction<'_, Postgres>,
    reset: RetiringWorldReset,
) -> Result<Option<([u8; 16], DecayRetireStep)>> {
    // House custody survives a map reset.  The house tile catalogue is the durable
    // authority here; relying on the retiring map's in-memory facts would make a
    // restart capable of deleting property before the replacement map is loaded.
    let row = sqlx::query("SELECT g.item_instance_id::text,g.channel_id::text FROM game_item_ground_locations g JOIN game_item_instances i USING(item_instance_id) WHERE g.world_id=encode($1,'hex')::uuid AND g.scope_kind=1 AND i.lifecycle=1 AND NOT EXISTS(SELECT 1 FROM game_house_tiles h WHERE h.world_id=g.world_id AND h.spatial_position=g.spatial_position) ORDER BY g.channel_id,g.item_instance_id LIMIT 1")
        .bind(reset.world.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
    let Some(row) = row else {
        return Ok(None);
    };
    let root = uuid_text(row.try_get("item_instance_id")?)?;
    let channel = uuid_text(row.try_get("channel_id")?)?;
    validate_root(tx, root).await?;
    let item: Option<String> = sqlx::query_scalar("SELECT item_instance_id::text FROM game_item_corpse_container_entries WHERE parent_item_instance_id=encode($1,'hex')::uuid ORDER BY placement_ordinal LIMIT 1")
        .bind(root.as_slice()).fetch_optional(&mut **tx).await?;
    let item = item
        .map(|item| uuid_text(&item))
        .transpose()?
        .unwrap_or(root);
    Ok(Some((
        channel,
        DecayRetireStep::for_world_reset(reset, root, item),
    )))
}

pub(in crate::durability) async fn validate_root(
    tx: &mut Transaction<'_, Postgres>,
    root: [u8; 16],
) -> Result<()> {
    let death_container = is_death_container(tx, root).await?;
    if death_container {
        death_tree(tx, root).await?;
        return Ok(());
    }
    let invalid: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_item_nested_entries WHERE parent_item_instance_id=encode($1,'hex')::uuid OR parent_item_instance_id IN(SELECT item_instance_id FROM game_item_corpse_container_entries WHERE parent_item_instance_id=encode($1,'hex')::uuid)) OR EXISTS(SELECT 1 FROM game_item_container_entries WHERE parent_item_instance_id=encode($1,'hex')::uuid OR parent_item_instance_id IN(SELECT item_instance_id FROM game_item_corpse_container_entries WHERE parent_item_instance_id=encode($1,'hex')::uuid)) OR EXISTS(SELECT 1 FROM game_item_corpse_container_entries WHERE parent_item_instance_id IN(SELECT item_instance_id FROM game_item_corpse_container_entries WHERE parent_item_instance_id=encode($1,'hex')::uuid))")
        .bind(root.as_slice()).fetch_one(&mut **tx).await?;
    if invalid {
        return Err(DurabilityError::InvalidStoredState);
    }
    Ok(())
}

pub(in crate::durability) async fn is_death_container(
    tx: &mut Transaction<'_, Postgres>,
    root: [u8; 16],
) -> Result<bool> {
    Ok(sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_player_death_item_records r JOIN game_item_ground_locations g USING(item_instance_id) WHERE r.item_instance_id=encode($1,'hex')::uuid AND r.operation=1 AND g.world_id=r.world_id AND (g.scope_kind,g.channel_id,g.instance_id) IS NOT DISTINCT FROM (r.scope_kind,r.channel_id,r.instance_id) AND g.corpse_ref=convert_to('OTERYN_ITEM_LOOP_DROP/v1:','UTF8')||uuid_send(r.transaction_id))")
        .bind(root.as_slice()).fetch_one(&mut **tx).await?)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::durability) enum DeathEntryKind {
    Direct,
    Nested,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::durability) struct DeathTreeEntry {
    pub item: [u8; 16],
    pub parent: [u8; 16],
    pub ordinal: u64,
    pub kind: DeathEntryKind,
    pub depth: i32,
}

// These are the exact equipped-container roots admitted by game_item_loop_owner.
// Their native destination policy accepts ammunition leaves only, capped at 20.
pub(super) const EQUIPPED_QUIVERS: [&str; 8] = [
    "oteryn:item.tibia.i35562",
    "oteryn:item.tibia.i35848",
    "oteryn:item.tibia.i35849",
    "oteryn:item.tibia.i35524",
    "oteryn:item.tibia.i36666",
    "oteryn:item.tibia.i39150",
    "oteryn:item.tibia.i39160",
    "oteryn:item.tibia.i45644",
];

/// Retained historical direct entries and ordinary nested entries keep their exact edges.
/// The materialized limit bounds traversal even when restored storage is malformed.
pub(in crate::durability) async fn death_tree(
    tx: &mut Transaction<'_, Postgres>,
    root: [u8; 16],
) -> Result<Vec<DeathTreeEntry>> {
    use crate::durability::item_loop_move::{ITEM_LOOP_DEPTH_MAX, ITEM_LOOP_TREE_ITEMS_MAX};
    let source: Option<(Vec<u8>, i16, String)> = sqlx::query_as(
        "SELECT uuid_send(g.world_id),r.source_slot,i.definition_production_key FROM game_player_death_item_records r \
         JOIN game_item_ground_locations g USING(item_instance_id) \
         JOIN game_item_instances i USING(item_instance_id) \
         WHERE r.item_instance_id=encode($1,'hex')::uuid AND r.operation=1 \
         AND g.world_id=r.world_id \
         AND (g.scope_kind,g.channel_id,g.instance_id) IS NOT DISTINCT FROM (r.scope_kind,r.channel_id,r.instance_id) \
         AND g.corpse_ref=convert_to('OTERYN_ITEM_LOOP_DROP/v1:','UTF8')||uuid_send(r.transaction_id)",
    )
    .bind(root.as_slice())
    .fetch_optional(&mut **tx)
    .await?;
    let (world, source_slot, root_definition) =
        source.ok_or(DurabilityError::InvalidStoredState)?;
    let rows = sqlx::query(
        "WITH RECURSIVE tree(item,parent,ordinal,kind,world,depth,path,cycle) AS ( \
           SELECT encode($1,'hex')::uuid,NULL::uuid,NULL::numeric,0,encode($2,'hex')::uuid,0,ARRAY[encode($1,'hex')::uuid],false \
           UNION ALL SELECT e.item,e.parent,e.ordinal,e.kind,e.world,t.depth+1,t.path||e.item,e.item=ANY(t.path) \
           FROM tree t CROSS JOIN LATERAL ( \
             SELECT * FROM ( \
               SELECT item_instance_id AS item,parent_item_instance_id AS parent,placement_ordinal AS ordinal,1 AS kind,world_id AS world \
               FROM game_item_container_entries WHERE parent_item_instance_id=t.item \
               UNION ALL SELECT item_instance_id,parent_item_instance_id,placement_ordinal,2,world_id \
               FROM game_item_nested_entries WHERE parent_item_instance_id=t.item \
             ) edges LIMIT $3 \
           ) e WHERE t.depth<$4 AND NOT t.cycle \
         ), bounded AS MATERIALIZED (SELECT * FROM tree LIMIT $5) \
         SELECT uuid_send(t.item) AS item,uuid_send(t.parent) AS parent,t.ordinal::text,t.kind,t.depth,t.cycle, \
           (i.item_instance_id IS NULL OR i.lifecycle<>1 OR i.quantity<=0 OR i.world_id<>t.world OR t.world<>encode($2,'hex')::uuid \
            OR EXISTS(SELECT 1 FROM game_item_corpse_container_entries c WHERE c.item_instance_id=t.item OR c.parent_item_instance_id=t.item) \
            OR EXISTS(SELECT 1 FROM game_item_ground_locations g WHERE g.item_instance_id=t.item AND t.depth<>0) \
            OR EXISTS(SELECT 1 FROM game_item_container_slots s WHERE s.item_instance_id=t.item) \
            OR EXISTS(SELECT 1 FROM game_character_equipment_slots s WHERE s.item_instance_id=t.item) \
            OR (SELECT count(*) FROM (SELECT 1 FROM game_item_container_entries e WHERE e.item_instance_id=t.item \
                UNION ALL SELECT 1 FROM game_item_nested_entries e WHERE e.item_instance_id=t.item) custody)<>CASE WHEN t.depth=0 THEN 0 ELSE 1 END) AS invalid \
         FROM bounded t LEFT JOIN game_item_instances i ON i.item_instance_id=t.item",
    )
    .bind(root.as_slice())
    .bind(world)
    .bind(i64::from(crate::durability::item_transfer::GAMEITEM01_CONTAINER_ENTRIES_MAX) + 1)
    .bind(ITEM_LOOP_DEPTH_MAX + 1)
    .bind(ITEM_LOOP_TREE_ITEMS_MAX + 2)
    .fetch_all(&mut **tx)
    .await?;
    let mut entries = Vec::with_capacity(rows.len().saturating_sub(1));
    for row in rows {
        if row.try_get::<bool, _>("invalid")? || row.try_get::<bool, _>("cycle")? {
            return Err(DurabilityError::InvalidStoredState);
        }
        let item: [u8; 16] = row
            .try_get::<Vec<u8>, _>("item")?
            .try_into()
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        let depth: i32 = row.try_get("depth")?;
        if depth == 0 {
            if item != root {
                return Err(DurabilityError::InvalidStoredState);
            }
            continue;
        }
        entries.push(DeathTreeEntry {
            item,
            parent: row
                .try_get::<Vec<u8>, _>("parent")?
                .try_into()
                .map_err(|_| DurabilityError::InvalidStoredState)?,
            ordinal: row
                .try_get::<String, _>("ordinal")?
                .parse()
                .map_err(|_| DurabilityError::InvalidStoredState)?,
            kind: match row.try_get::<i32, _>("kind")? {
                1 => DeathEntryKind::Direct,
                2 => DeathEntryKind::Nested,
                _ => return Err(DurabilityError::InvalidStoredState),
            },
            depth,
        });
    }
    order_death_tree(root, source_slot, &root_definition, entries)
}

fn order_death_tree(
    root: [u8; 16],
    source_slot: i16,
    root_definition: &str,
    mut entries: Vec<DeathTreeEntry>,
) -> Result<Vec<DeathTreeEntry>> {
    use crate::durability::item_loop_move::{ITEM_LOOP_DEPTH_MAX, ITEM_LOOP_TREE_ITEMS_MAX};
    use std::collections::{BTreeMap, BTreeSet};
    let (depth_limit, descendants_limit) = match source_slot {
        9 => (ITEM_LOOP_DEPTH_MAX, ITEM_LOOP_TREE_ITEMS_MAX),
        0 => (ITEM_LOOP_DEPTH_MAX - 1, ITEM_LOOP_TREE_ITEMS_MAX - 1),
        6 | 10 if EQUIPPED_QUIVERS.contains(&root_definition) => (
            1,
            i64::from(crate::durability::item_transfer::GAMEITEM01_CONTAINER_ENTRIES_MAX),
        ),
        _ => (0, 0),
    };
    if entries.len() > descendants_limit as usize {
        return Err(DurabilityError::InvalidStoredState);
    }
    let mut depths = BTreeMap::from([(root, 0)]);
    let mut children = BTreeMap::<_, BTreeSet<_>>::new();
    for entry in &entries {
        if !(1..=depth_limit).contains(&entry.depth)
            || entry.ordinal == 0
            || depths.insert(entry.item, entry.depth).is_some()
        {
            return Err(DurabilityError::InvalidStoredState);
        }
        let ordinals = children.entry(entry.parent).or_default();
        if !ordinals.insert(entry.ordinal)
            || ordinals.len()
                > crate::durability::item_transfer::GAMEITEM01_CONTAINER_ENTRIES_MAX as usize
        {
            return Err(DurabilityError::InvalidStoredState);
        }
    }
    if entries
        .iter()
        .any(|entry| depths.get(&entry.parent) != Some(&(entry.depth - 1)))
    {
        return Err(DurabilityError::InvalidStoredState);
    }
    entries.sort_unstable_by_key(|entry| (std::cmp::Reverse(entry.depth), entry.item));
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(value: u64) -> [u8; 16] {
        let mut id = [0; 16];
        id[8..].copy_from_slice(&value.to_be_bytes());
        id
    }

    fn wide_tree(descendants: u64) -> Vec<DeathTreeEntry> {
        let mut depths = vec![0];
        (1..=descendants)
            .map(|item| {
                let parent = (item - 1) / 20;
                let depth = depths[parent as usize] + 1;
                depths.push(depth);
                DeathTreeEntry {
                    item: id(item),
                    parent: id(parent),
                    ordinal: (item - 1) % 20 + 1,
                    kind: if parent == 0 && item % 2 == 0 {
                        DeathEntryKind::Direct
                    } else {
                        DeathEntryKind::Nested
                    },
                    depth,
                }
            })
            .collect()
    }

    fn chain(depth: i32) -> Vec<DeathTreeEntry> {
        (1..=depth)
            .map(|depth| DeathTreeEntry {
                item: id(depth as u64),
                parent: id((depth - 1) as u64),
                ordinal: u64::MAX,
                kind: DeathEntryKind::Nested,
                depth,
            })
            .collect()
    }

    #[test]
    fn death_tree_preserves_the_full_carried_count_and_postorder() -> Result<()> {
        let tree = order_death_tree(id(0), 9, "fixture:backpack", wide_tree(500))?;
        assert_eq!(tree.len(), 500); // Plus the separately retired main root: 501.
        assert!(tree.windows(2).all(|pair| pair[0].depth >= pair[1].depth));
        assert!(order_death_tree(id(0), 9, "fixture:backpack", wide_tree(501)).is_err());
        let mut direct = wide_tree(499);
        for entry in &mut direct {
            entry.kind = DeathEntryKind::Nested;
        }
        assert_eq!(
            order_death_tree(id(0), 0, "fixture:backpack", direct)?.len(),
            499
        );
        assert!(order_death_tree(id(0), 0, "fixture:backpack", wide_tree(500)).is_err());
        Ok(())
    }

    #[test]
    fn death_tree_preserves_depth_eight_and_monotonic_ordinals() -> Result<()> {
        let main = order_death_tree(id(0), 9, "fixture:backpack", chain(8))?;
        assert_eq!(main[0].item, id(8));
        assert_eq!(main[0].ordinal, u64::MAX);
        assert!(order_death_tree(id(0), 9, "fixture:backpack", chain(9)).is_err());
        assert!(order_death_tree(id(0), 0, "fixture:backpack", chain(7)).is_ok());
        assert!(order_death_tree(id(0), 0, "fixture:backpack", chain(8)).is_err());
        let mut historical = chain(3);
        historical[1].kind = DeathEntryKind::Direct;
        assert!(order_death_tree(id(0), 0, "fixture:backpack", historical).is_ok());
        Ok(())
    }

    #[test]
    fn equipped_quivers_keep_their_leaf_capacity_and_exact_source_scope() {
        for slot in [6, 10] {
            for definition in EQUIPPED_QUIVERS {
                let mut tree = wide_tree(20);
                for entry in &mut tree {
                    entry.kind = DeathEntryKind::Nested;
                }
                assert!(order_death_tree(id(0), slot, definition, tree).is_ok());
                assert!(order_death_tree(id(0), slot, definition, wide_tree(21)).is_err());
                assert!(order_death_tree(id(0), slot, definition, chain(2)).is_err());
            }
            assert!(order_death_tree(id(0), slot, "fixture:backpack", chain(1)).is_err());
            assert!(order_death_tree(id(0), slot, "fixture:ordinary-gear", vec![]).is_ok());
        }
        for slot in [1, 2, 3, 4, 5, 7, 8] {
            assert!(order_death_tree(id(0), slot, EQUIPPED_QUIVERS[0], chain(1)).is_err());
        }
    }

    #[test]
    fn death_tree_rejects_duplicate_or_foreign_edges_and_capacity_overflow() {
        let mutations: [fn(&mut Vec<DeathTreeEntry>); 6] = [
            |tree| tree[0].item = id(0),
            |tree| tree[0].parent = id(999),
            |tree| tree[1].item = tree[0].item,
            |tree| tree[1].ordinal = tree[0].ordinal,
            |tree| tree[0].ordinal = 0,
            |tree| {
                tree[20].parent = id(0);
                tree[20].depth = 1;
                tree[20].ordinal = 21;
            },
        ];
        for mutate in mutations {
            let mut tree = wide_tree(21);
            mutate(&mut tree);
            assert!(order_death_tree(id(0), 9, "fixture:backpack", tree).is_err());
        }
        let mut tree = chain(2);
        tree[1].depth = 1;
        assert!(order_death_tree(id(0), 9, "fixture:backpack", tree).is_err());
        assert!(order_death_tree(id(0), 7, "fixture:backpack", chain(1)).is_err());
    }
}
