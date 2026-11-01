//! Crystal 201189f player_wheel.cpp:1634-1643: the seven live Atelier balances.
//! Display counts are by item type, as in player.cpp:5190-5210; they are not a
//! payment reservation and never enter the Atelier state or mutation receipt.
use super::super::db::{begin_semantic_transaction, commit_semantic_transaction};
use super::*;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ResourceBalances {
    pub bank_gold: u64,
    pub carried_gold: u64,
    pub lesser_gems: u64,
    pub regular_gems: u64,
    pub greater_gems: u64,
    pub basic_fragments: u64,
    pub supreme_fragments: u64,
}

impl ResourceBalances {
    fn from_totals(
        bank_gold: u64,
        carried_gold: u64,
        pouch_gold: u64,
        gems: [u64; 3],
        carried_fragments: [u64; 2],
        stash_fragments: [u64; 2],
    ) -> std::result::Result<Self, DurabilityError> {
        Ok(Self {
            bank_gold,
            carried_gold: carried_gold
                .checked_add(pouch_gold)
                .ok_or(DurabilityError::InvalidStoredState)?,
            lesser_gems: gems[0],
            regular_gems: gems[1],
            greater_gems: gems[2],
            basic_fragments: carried_fragments[0]
                .checked_add(stash_fragments[0])
                .ok_or(DurabilityError::InvalidStoredState)?,
            supreme_fragments: carried_fragments[1]
                .checked_add(stash_fragments[1])
                .ok_or(DurabilityError::InvalidStoredState)?,
        })
    }
}

fn gem_keys<'a>(data: &'a WheelGemData, vocation: &str) -> Option<[&'a str; 3]> {
    let family = &data.gems()["vocations"][base_vocation(vocation)?]["gem_family"]["items"];
    Some([
        family["lesser"].as_str()?,
        family["regular"].as_str()?,
        family["greater"].as_str()?,
    ])
}

impl DurabilityRoot {
    /// Read current resource counts under the native session, generation, node,
    /// interpretation and Character revision fence. The root lock serializes
    /// carried-tree and stash writers; one SQL snapshot also includes bank
    /// transfers from another character. No receipt, revision or balance changes.
    pub async fn read_character_atelier_resources(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
    ) -> Result<ResourceBalances> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    if let Err(error) = assert_gameplay_fence(&mut tx, &fence, &node).await? {
                        return Ok(Err(error));
                    }
                    let vocation: Option<String> = sqlx::query_scalar(
                        "SELECT vocation FROM game_character_build_state WHERE character_id=encode($1,'hex')::uuid",
                    )
                    .bind(fence.character_id.as_bytes().as_slice())
                    .fetch_optional(&mut *tx)
                    .await?;
                    let Some(vocation) = vocation else {
                        return Ok(Err(CharacterProgressionError::MissingProgressionState));
                    };
                    let data = WheelGemData::embedded()
                        .map_err(|_| DurabilityError::InvalidStoredState)?;
                    let Some(keys) = gem_keys(&data, &vocation) else {
                        return Ok(Err(CharacterProgressionError::InvalidInput));
                    };
                    // Same carried authority bound as Atelier materials: 500 tree
                    // nodes (including the main bag), nine equipped roots, depth 8.
                    // The sentinel and excess depth/cycle are rejected as a whole.
                    // Each positive stash row uses >=1 of the existing 15000 depot
                    // slots (0189 game_stash_capacity); zero rows do not contribute.
                    // The Inbox Gold Pouch has its separate existing coin owner:
                    // pouch_weight admits at most three definition groups, not a
                    // carried-tree slot limit. Count it for display only.
                    let row = sqlx::query(
                        "WITH RECURSIVE roots(item,world,slot) AS (
                           SELECT item_instance_id,world_id,0::smallint FROM game_item_container_slots
                            WHERE character_id=encode($1,'hex')::uuid
                           UNION ALL
                           SELECT item_instance_id,world_id,slot FROM game_character_equipment_slots
                            WHERE character_id=encode($1,'hex')::uuid
                         ), carried(item,world,slot,depth,ancestors,cycle) AS (
                           SELECT item,world,slot,0,ARRAY[item],false FROM roots
                           UNION ALL
                           SELECT e.item_instance_id,e.world_id,c.slot,c.depth+1,
                                  c.ancestors||e.item_instance_id,e.item_instance_id=ANY(c.ancestors)
                             FROM carried c JOIN (
                               SELECT item_instance_id,world_id,parent_item_instance_id FROM game_item_container_entries
                                WHERE character_id=encode($1,'hex')::uuid
                               UNION ALL
                               SELECT item_instance_id,world_id,parent_item_instance_id FROM game_item_nested_entries
                             ) e ON e.parent_item_instance_id=c.item
                            WHERE c.depth<9 AND NOT c.cycle
                         ), bounded AS MATERIALIZED (
                           SELECT * FROM carried LIMIT 510
                         ), inventory AS (
                           SELECT c.*,i.world_id AS item_world,i.lifecycle,i.quantity,
                                  i.definition_family,i.definition_production_key
                             FROM bounded c LEFT JOIN game_item_instances i ON i.item_instance_id=c.item
                         ), stash AS MATERIALIZED (
                           SELECT definition_key,quantity FROM game_character_stash_balances
                            WHERE character_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid
                              AND quantity>0 AND definition_key IN ('oteryn:item.tibia.i46625','oteryn:item.tibia.i46626')
                            LIMIT 15001
                         ), pouch AS MATERIALIZED (
                           SELECT i.definition_family,i.definition_production_key,i.definition_revision_ref,
                                  sum(i.quantity) AS quantity,
                                  bool_and(COALESCE(p.definition_family='Item' AND p.lifecycle=1 AND p.quantity=1
                                      AND p.world_id=loc.world_id AND n.world_id=loc.world_id AND i.world_id=loc.world_id
                                      AND i.lifecycle=1 AND i.quantity BETWEEN 1 AND 100,false)) AS valid
                             FROM game_item_character_inbox_locations loc
                             JOIN game_item_instances p ON p.item_instance_id=loc.item_instance_id
                             JOIN game_item_nested_entries n ON n.parent_item_instance_id=p.item_instance_id
                             LEFT JOIN game_item_instances i ON i.item_instance_id=n.item_instance_id
                            WHERE loc.character_id=encode($1,'hex')::uuid AND loc.world_id=encode($2,'hex')::uuid
                              AND p.definition_production_key='oteryn:item.tibia.i23721'
                            GROUP BY i.definition_family,i.definition_production_key,i.definition_revision_ref
                            LIMIT 4
                         ), totals AS (
                           SELECT count(*)<510 AND count(*)=count(DISTINCT item)
                                  AND count(*) FILTER(WHERE NOT(depth=0 AND slot<>0))<=500
                                  AND count(*) FILTER(WHERE depth=0 AND slot<>0)<=9
                                  AND count(*) FILTER(WHERE depth=0 AND slot=0)<=1
                                  AND COALESCE(bool_and(COALESCE(NOT cycle AND depth<=8
                                      AND (slot=0 OR slot BETWEEN 1 AND 8 OR slot=10)
                                      AND world=encode($2,'hex')::uuid AND item_world=world
                                      AND lifecycle=1 AND quantity BETWEEN 1 AND 4294967295,false)),true) AS valid,
                                  COALESCE(sum(CASE WHEN definition_family='Item' THEN quantity::numeric *
                                      CASE definition_production_key WHEN 'oteryn:item.tibia.i3031' THEN 1
                                      WHEN 'oteryn:item.tibia.i3035' THEN 100 WHEN 'oteryn:item.tibia.i3043' THEN 10000 ELSE 0 END ELSE 0 END),0)::text AS carried_gold,
                                  COALESCE(sum(quantity) FILTER(WHERE definition_family='Item' AND definition_production_key=$3),0)::text AS lesser_gems,
                                  COALESCE(sum(quantity) FILTER(WHERE definition_family='Item' AND definition_production_key=$4),0)::text AS regular_gems,
                                  COALESCE(sum(quantity) FILTER(WHERE definition_family='Item' AND definition_production_key=$5),0)::text AS greater_gems,
                                  COALESCE(sum(quantity) FILTER(WHERE definition_family='Item' AND definition_production_key='oteryn:item.tibia.i46625'),0)::text AS carried_basic,
                                  COALESCE(sum(quantity) FILTER(WHERE definition_family='Item' AND definition_production_key='oteryn:item.tibia.i46626'),0)::text AS carried_supreme
                             FROM inventory
                         ), stash_totals AS (
                           SELECT count(*)<=15000 AND COALESCE(sum((quantity+99)/100),0)<=15000 AS stash_valid,
                                  COALESCE(sum(quantity) FILTER(WHERE definition_key='oteryn:item.tibia.i46625'),0)::text AS stash_basic,
                                  COALESCE(sum(quantity) FILTER(WHERE definition_key='oteryn:item.tibia.i46626'),0)::text AS stash_supreme
                             FROM stash
                         ), pouch_totals AS (
                           SELECT count(*)<=3 AND COALESCE(bool_and(valid AND definition_family='Item'
                                      AND definition_production_key IN ('oteryn:item.tibia.i3031','oteryn:item.tibia.i3035','oteryn:item.tibia.i3043')),true) AS pouch_valid,
                                  COALESCE(sum(quantity * CASE definition_production_key
                                      WHEN 'oteryn:item.tibia.i3031' THEN 1 WHEN 'oteryn:item.tibia.i3035' THEN 100
                                      WHEN 'oteryn:item.tibia.i3043' THEN 10000 ELSE 0 END),0)::text AS pouch_gold
                             FROM pouch
                         )
                         SELECT totals.*,stash_totals.*,pouch_totals.*,
                                COALESCE((SELECT b.balance FROM game_account_bank_balances b
                                  JOIN game_character_roots r ON r.account_id=b.account_id AND r.world_id=b.world_id
                                 WHERE r.character_id=encode($1,'hex')::uuid AND r.world_id=encode($2,'hex')::uuid),0)::text AS bank_gold
                           FROM totals CROSS JOIN stash_totals CROSS JOIN pouch_totals",
                    )
                    .bind(fence.character_id.as_bytes().as_slice())
                    .bind(fence.runtime_scope.world_id().as_bytes().as_slice())
                    .bind(keys[0])
                    .bind(keys[1])
                    .bind(keys[2])
                    .fetch_one(&mut *tx)
                    .await?;
                    if !row.try_get::<bool, _>("valid")?
                        || !row.try_get::<bool, _>("stash_valid")?
                        || !row.try_get::<bool, _>("pouch_valid")?
                    {
                        return Err(DurabilityError::InvalidStoredState);
                    }
                    let balances = ResourceBalances::from_totals(
                        numeric_u64(&row, "bank_gold")?,
                        numeric_u64(&row, "carried_gold")?,
                        numeric_u64(&row, "pouch_gold")?,
                        [
                            numeric_u64(&row, "lesser_gems")?,
                            numeric_u64(&row, "regular_gems")?,
                            numeric_u64(&row, "greater_gems")?,
                        ],
                        [
                            numeric_u64(&row, "carried_basic")?,
                            numeric_u64(&row, "carried_supreme")?,
                        ],
                        [
                            numeric_u64(&row, "stash_basic")?,
                            numeric_u64(&row, "stash_supreme")?,
                        ],
                    )?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(balances))
                })
            })
            .await?
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resource_fragment_totals_are_live_carried_plus_stash()
    -> std::result::Result<(), DurabilityError> {
        assert_eq!(
            ResourceBalances::from_totals(
                9_000_000_000,
                12_345,
                200_000,
                [2, 3, 4],
                [7, 11],
                [13, 17]
            )?,
            ResourceBalances {
                bank_gold: 9_000_000_000,
                carried_gold: 212_345,
                lesser_gems: 2,
                regular_gems: 3,
                greater_gems: 4,
                basic_fragments: 20,
                supreme_fragments: 28,
            }
        );
        assert_eq!(
            ResourceBalances::from_totals(0, 0, 0, [0; 3], [0; 2], [0; 2])?,
            ResourceBalances::default()
        );
        assert!(ResourceBalances::from_totals(0, u64::MAX, 1, [0; 3], [0; 2], [0; 2]).is_err());
        assert!(ResourceBalances::from_totals(0, 0, 0, [0; 3], [u64::MAX, 0], [1, 0]).is_err());
        assert!(ResourceBalances::from_totals(0, 0, 0, [0; 3], [0, 1], [0, u64::MAX]).is_err());
        Ok(())
    }

    #[test]
    fn resource_gem_keys_follow_the_current_vocation_catalogue()
    -> std::result::Result<(), DurabilityError> {
        let data = WheelGemData::embedded().map_err(|_| DurabilityError::InvalidStoredState)?;
        assert_eq!(
            gem_keys(&data, "sorcerer"),
            Some([
                "oteryn:item.tibia.i44608",
                "oteryn:item.tibia.i44609",
                "oteryn:item.tibia.i44610"
            ])
        );
        assert_eq!(
            gem_keys(&data, "master_sorcerer"),
            gem_keys(&data, "sorcerer")
        );
        assert!(gem_keys(&data, "knight").is_some());
        assert!(gem_keys(&data, "paladin").is_some());
        assert!(gem_keys(&data, "druid").is_some());
        assert!(gem_keys(&data, "monk").is_some());
        assert!(gem_keys(&data, "none").is_none());
        Ok(())
    }
}
