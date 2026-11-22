use super::{Harness, TestResult, debug, id};
use crate::durability::server_save_sweep::{
    GroundSweepBinding, GroundSweepDisposition, GroundSweepFacts, GroundSweepItemFacts,
    GroundSweepPlacement, GroundSweepPolicy, GroundSweepPreserve, GroundSweepRoot,
    GroundSweepTileFacts, GroundSweepUnknown,
};
use crate::foundation::{ChannelId, WorldId};
use std::num::NonZeroU16;

const MAP: &str = "sha256:0707070707070707070707070707070707070707070707070707070707070707";
const CONTENT: &str = "sha256:0808080808080808080808080808080808080808080808080808080808080808";

#[test]
fn server_save_sweep_plan_classifies_ground_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|err| err.to_string())?
                .block_on(scenario(admin))
                .map_err(|err| err.to_string())
        })?
        .join()
        .map_err(|_| "server save sweep scenario panicked")?
        .map_err(Into::into)
}

// These are the map/content owner's qualified facts for this fixture pin.
// Instance IDs are checked against the ledger rows observed by the planner.
struct FixtureFacts(GroundSweepBinding);

impl GroundSweepFacts for FixtureFacts {
    fn binding(&self) -> &GroundSweepBinding {
        &self.0
    }

    fn tile(&self, placement: &GroundSweepPlacement) -> GroundSweepTileFacts {
        GroundSweepTileFacts {
            house: (placement.position != position(113)).then_some(false),
            protection_zone: Some(placement.position == position(103)),
        }
    }

    fn item(&self, root: &GroundSweepRoot) -> GroundSweepItemFacts {
        let item = root.item_id[0];
        if root.definition_revision != "fixture-r1" {
            return GroundSweepItemFacts::default();
        }
        GroundSweepItemFacts {
            loaded_from_map: Some(false),
            removable: Some(item != 111),
            pickupable: Some(item != 107),
            has_action_id: Some(item == 104),
            has_unique_id: Some(item == 105),
        }
    }
}

fn position(seed: u8) -> [u8; 10] {
    [0, 0, 0, seed, 0, 0, 0, 42, 0, 7]
}

async fn scenario(admin: String) -> TestResult {
    let harness = Harness::create_with_admission_seed(admin, "save_sweep", true, false).await?;
    let result = qualify(&harness).await;
    let cleanup = harness.cleanup().await;
    result.and(cleanup)
}

async fn qualify(harness: &Harness) -> TestResult {
    use GroundSweepDisposition::{Eligible, Preserved, Unknown};

    seed(harness).await?;
    let before = ledger_counts(&harness.pool).await?;
    let facts = FixtureFacts(GroundSweepBinding {
        world: WorldId::decode(&id(42)).map_err(debug)?,
        channel: ChannelId::decode(&id(43)).map_err(debug)?,
        generation: 3,
        map_revision: MAP.into(),
        content_revision: CONTENT.into(),
        frame: vec![9; 32],
    });
    let page_size = NonZeroU16::new(4).ok_or("page size")?;
    let mut cursor = None;
    let mut observed = Vec::new();
    loop {
        let page = harness
            .root
            .plan_server_save_sweep(&facts, GroundSweepPolicy::default(), cursor, page_size)
            .await?;
        assert!(page.entries.len() <= usize::from(page_size.get()));
        if let Some(next) = page.next_after {
            assert_eq!(
                page.entries.last().map(|entry| entry.root.item_id),
                Some(next)
            );
            assert!(cursor.is_none_or(|previous| previous < next));
        }
        observed.extend(page.entries);
        cursor = page.next_after;
        if cursor.is_none() {
            break;
        }
    }
    assert_eq!(
        observed
            .iter()
            .map(|entry| entry.root.item_id)
            .collect::<Vec<_>>(),
        [101, 102, 103, 104, 105, 106, 107, 108, 109, 111, 113, 114].map(id),
    );
    let mut dispositions = observed.iter().map(|entry| &entry.disposition);
    assert_eq!(dispositions.next(), Some(&Eligible(vec![])));
    for expected in [
        Preserved(GroundSweepPreserve::House),
        Preserved(GroundSweepPreserve::ProtectionZone),
        Preserved(GroundSweepPreserve::ActionId),
        Preserved(GroundSweepPreserve::UniqueId),
        Preserved(GroundSweepPreserve::LoadedFromMap),
        Preserved(GroundSweepPreserve::NotPickupable),
        Unknown(GroundSweepUnknown::ItemFacts),
    ] {
        assert_eq!(dispositions.next(), Some(&expected));
    }

    let container = observed.get(8).ok_or("container root")?;
    let Eligible(tree) = &container.disposition else {
        return Err("container should be eligible".into());
    };
    assert_eq!(container.root.item_id, id(109));
    assert_eq!(container.root.item_revision, 109);
    let placement = &container.root.placement;
    assert_eq!(placement.position, position(109));
    assert_eq!(placement.stack_ordinal, Some(109));
    assert_eq!(placement.generation, 2);
    assert_eq!(placement.map_revision, MAP);
    assert_eq!(placement.content_revision, CONTENT);
    assert_eq!(placement.frame, [9; 32]);
    assert_eq!(placement.corpse_ref, id(109));
    assert_eq!(tree.len(), 2);
    for (edge, (child, parent)) in tree.iter().zip([(110, 109), (112, 110)]) {
        assert_eq!(edge.item_id, id(child));
        assert_eq!(edge.parent_id, id(parent));
        assert_eq!(edge.item_revision, u64::from(child));
        assert_eq!(edge.ordinal, 1);
        assert_eq!(edge.placed_transaction, id(child));
    }

    assert_eq!(
        dispositions.skip(1).collect::<Vec<_>>(),
        [
            &Preserved(GroundSweepPreserve::NotRemovable),
            &Unknown(GroundSweepUnknown::TileFacts),
            &Unknown(GroundSweepUnknown::Placement),
        ],
    );

    let enabled = harness
        .root
        .plan_server_save_sweep(
            &facts,
            GroundSweepPolicy {
                clean_protection_zones: true,
            },
            None,
            NonZeroU16::MAX,
        )
        .await?;
    assert_eq!(enabled.next_after, None);
    let eligible = enabled
        .entries
        .iter()
        .filter_map(|entry| matches!(entry.disposition, Eligible(_)).then_some(entry.root.item_id));
    assert_eq!(eligible.collect::<Vec<_>>(), [101, 103, 109].map(id));
    assert_eq!(ledger_counts(&harness.pool).await?, before);
    Ok(())
}

async fn seed(harness: &Harness) -> TestResult {
    let world = id(42);
    let channel = id(43);
    let mut tx = harness.pool.begin().await?;
    // Provision the selection matrix in the migrated ledger. Mint, drop and
    // house-placement receipt writers are qualified in their own harness cases.
    sqlx::query("SET LOCAL session_replication_role = replica")
        .execute(&mut *tx)
        .await?;
    for item in 101_u8..=117 {
        sqlx::query(
            "INSERT INTO game_item_instances (item_instance_id,world_id,definition_family,
                definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id,state_revision)
             VALUES (encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item',$3,$4,1,1,encode($1,'hex')::uuid,$5::bigint)",
        )
        .bind(id(item).as_slice())
        .bind(world.as_slice())
        .bind(if matches!(item, 109 | 110) { "oteryn:item.tibia.i2854" } else { "oteryn:item.tibia.i3031" })
        .bind(if item == 108 { "unqualified-revision" } else { "fixture-r1" })
        .bind(i64::from(item))
        .execute(&mut *tx)
        .await?;
        match item {
            110 | 112 => {
                sqlx::query(
                    "INSERT INTO game_item_nested_entries VALUES
                     (encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,1,encode($1,'hex')::uuid)",
                )
                .bind(id(item).as_slice()).bind(world.as_slice())
                .bind(id(if item == 110 { 109 } else { 110 }).as_slice())
                .execute(&mut *tx).await?;
            }
            115 => {
                sqlx::query(
                    "INSERT INTO game_item_house_interior_locations
                     (item_instance_id,world_id,house_key,spatial_position,stack_ordinal,placed_transaction_id)
                     VALUES (encode($1,'hex')::uuid,encode($2,'hex')::uuid,
                       'oteryn:content.house.save_sweep_fixture',$3,1,encode($1,'hex')::uuid)",
                )
                .bind(id(item).as_slice()).bind(world.as_slice()).bind(position(102).as_slice())
                .execute(&mut *tx).await?;
            }
            _ => {
                let other_channel = id(44);
                sqlx::query(
                    "INSERT INTO game_item_ground_locations (item_instance_id,world_id,channel_id,scope_kind,instance_id,
                       runtime_scope_ownership_generation,spatial_position,corpse_ref,map_revision,content_revision,
                       native_room_placement_context,stack_ordinal)
                     VALUES (encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4,
                       encode($5,'hex')::uuid,2,$6,$1,$7,$8,$9,$10)",
                )
                .bind(id(item).as_slice()).bind(world.as_slice())
                .bind(match item { 116 => Some(other_channel.as_slice()), 117 => None, _ => Some(channel.as_slice()) })
                .bind(if item == 117 { 2_i16 } else { 1 })
                .bind((item == 117).then_some(other_channel.as_slice()))
                .bind(position(item).as_slice()).bind(MAP)
                .bind(if item == 114 { "foreign-content" } else { CONTENT })
                .bind([9_u8; 32].as_slice()).bind(i64::from(item))
                .execute(&mut *tx).await?;
            }
        }
    }
    sqlx::query(
        "INSERT INTO game_house_properties(world_id,house_key,kind,rent_gold,content_digest)
           VALUES(encode($1,'hex')::uuid,'oteryn:content.house.save_sweep_fixture','private_house',0,$2)",
    ).bind(world.as_slice()).bind([8_u8; 32].as_slice()).execute(&mut *tx).await?;
    sqlx::query(
        "INSERT INTO game_house_tiles(world_id,house_key,spatial_position)
           VALUES(encode($1,'hex')::uuid,'oteryn:content.house.save_sweep_fixture',$2)",
    )
    .bind(world.as_slice())
    .bind(position(102).as_slice())
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "INSERT INTO game_native_map_item_receipts
          (transaction_id,event_id,item_instance_id,world_id,channel_id,scope_generation,holder_node_id,
           holder_registration_revision,content_digest,map_digest,frame_digest,placement_key,
           definition_family,definition_key,definition_revision,quantity,spatial_position,map_revision,
           content_revision,placement_context,source_binding,source_attributes,blocks_movement,
           blocks_projectile,immovable_block_solid,binding,source_intent)
         SELECT i.minted_transaction_id,uuidv7(),i.item_instance_id,g.world_id,g.channel_id,2,
           uuidv7(),1,$1,$2,g.native_room_placement_context,'save-sweep-authored-tile',
           i.definition_family,i.definition_production_key,i.definition_revision_ref,i.quantity,
           g.spatial_position,g.map_revision,g.content_revision,g.native_room_placement_context,
           g.spatial_position,'{}',false,false,false,sha256(g.spatial_position),g.spatial_position
         FROM game_item_ground_locations g JOIN game_item_instances i USING(item_instance_id)
         WHERE g.item_instance_id=encode($3,'hex')::uuid",
    )
    .bind([8_u8; 32].as_slice()).bind([7_u8; 32].as_slice()).bind(id(106).as_slice())
    .execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

async fn ledger_counts(pool: &sqlx::PgPool) -> TestResult<Vec<i64>> {
    Ok(sqlx::query_scalar(
        "SELECT unnest(ARRAY[
           (SELECT count(*) FROM game_item_instances),
           (SELECT count(*) FROM game_item_instances WHERE lifecycle=1),
           (SELECT sum(state_revision)::bigint FROM game_item_instances),
           (SELECT count(*) FROM game_item_ground_locations),
           (SELECT count(*) FROM game_item_nested_entries),
           (SELECT count(*) FROM game_item_house_interior_locations),
           (SELECT count(*) FROM game_native_map_item_receipts),
           (SELECT count(*) FROM game_item_decay_retire_receipts),
           (SELECT count(*) FROM game_spell_item_receipts),
           (SELECT count(*) FROM game_item_audit_outbox)
         ])",
    )
    .fetch_all(pool)
    .await?)
}
