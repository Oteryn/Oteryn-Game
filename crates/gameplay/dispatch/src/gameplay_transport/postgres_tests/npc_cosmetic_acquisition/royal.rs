//! Original compiled map and NPCframe; no source NPC is reseeded beside a synthetic actor.
use super::*;
std::thread_local! { static FULL: std::cell::Cell<bool> = const {std::cell::Cell::new(false)}; }
std::thread_local! { static CHECK_DESCRIPTION:std::cell::Cell<bool>=const{std::cell::Cell::new(false)}; }
pub(super) fn check_description() -> bool {
    CHECK_DESCRIPTION.with(std::cell::Cell::get)
}
pub(super) fn description_scenario() -> TestResult {
    CHECK_DESCRIPTION.with(|flag| flag.set(true));
    let result = full_scenario();
    CHECK_DESCRIPTION.with(|flag| flag.set(false));
    result
}
pub(super) fn full() -> bool {
    FULL.with(std::cell::Cell::get)
}
pub(super) fn full_scenario() -> TestResult {
    FULL.with(|flag| flag.set(true));
    let result = super::scenario_for("oteryn:npc.king_tibianus");
    FULL.with(|flag| flag.set(false));
    result
}
use crate::content::{ReferenceItemField as F, ReferenceItemType};
pub(super) const ENTRY: (i32, i32, i16) = (32311, 32173, 6);
pub(super) fn world(
    world: crate::foundation::WorldId,
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> TestResult<(crate::map::boot::BundleWorld, NpcPlacementTable)> {
    let bytes = std::fs::read(
        std::env::var_os(if full() {
            "OTERYN_SRV7_ROYAL_BUNDLE"
        } else {
            "OTERYN_SRV7_HORSE_BUNDLE"
        })
        .ok_or("actual retailer source bundle")?,
    )?;
    let b = oteryn_world_bundle::bundle::read(&bytes).map_err(debug)?;
    let table = NpcPlacementTable {
        npcs: b
            .npcs
            .npcs
            .iter()
            .filter(|npc| npc.key == "oteryn:npc.king_tibianus")
            .map(|n| NpcPlacements {
                key: n.key.clone(),
                placements: n
                    .placements
                    .iter()
                    .map(|p| NpcPlacement {
                        x: p.x,
                        y: p.y,
                        floor: p.floor,
                        direction: match p.direction {
                            oteryn_world_bundle::npc::Direction::North => MovementFacing::North,
                            oteryn_world_bundle::npc::Direction::East => MovementFacing::East,
                            oteryn_world_bundle::npc::Direction::South => MovementFacing::South,
                            oteryn_world_bundle::npc::Direction::West => MovementFacing::West,
                        },
                    })
                    .collect(),
            })
            .collect(),
    };
    let king_tibianus = table
        .npcs
        .iter()
        .find(|n| n.key == "oteryn:npc.king_tibianus")
        .ok_or("real King Tibianus source placement")?;
    assert_eq!(king_tibianus.placements.len(), 1);
    assert_eq!(
        (
            king_tibianus.placements[0].x,
            king_tibianus.placements[0].y,
            king_tibianus.placements[0].floor
        ),
        (32311, 32172, -6)
    );
    let pins = crate::map::BundlePins {
        digest: b.digest,
        project_format_version: b.manifest.identity.project_format_version.clone(),
        world_schema_version: b.manifest.identity.world_schema_version.clone(),
        content_revision: b.manifest.identity.content_revision.clone(),
        production: false,
    };
    let base = crate::map::load(&bytes, &pins).map_err(debug)?;
    let index = native.item_index().ok_or("retailer native Item index")?;
    let boot=crate::map::boot::boot(&bytes,&crate::map::boot::BootPins{bundle:pins,map_revision:crate::map::overlay::map_revision(&base),start:crate::map::overlay::TilePos{x:32311,y:32173,floor:-6}},world,crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?,|key|{
 let rev=index.revision(key)?;let policy=native.item_policy(key,rev).map(|p|p.record());
 Some(crate::map::facts::ItemDefinition{reference:index.definition_ref("Item",key,rev)?,solid:policy.and_then(|p|p.attributes.blocks_movement),blocks_projectile:policy.and_then(|p|p.attributes.blocks_projectile).unwrap_or(true),pickupable:policy.is_some_and(|p|matches!(&p.semantics.physical,F::Known(p) if p.pickupable==F::Known(true))),door:policy.is_some_and(|p|matches!(&p.semantics.classification,F::Known(c) if c.item_type==F::Known(ReferenceItemType::Door)))})
 }).map_err(debug)?;
    Ok((boot, table))
}

pub(super) async fn exercise(
    h: &Harness,
    owner: &mut crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut crate::gameplay_transport::connection::AdmittedSession,
    npc: oteryn_protocol_oteryn::world_spatial_entities::EntityRef,
) -> TestResult {
    if full() {
        return super::royal_full_purchase::exercise_full_purchase(h, owner, admitted, npc).await;
    }
    // Historical standing wealth fixture, with a matching ledger head. This
    // does not claim these500m were earned by this acquisition test.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    // Existing GOLD-FEE-ACT-2 fixture boundary, as exercised by
    // type2_audit_activation_postgres_cases.rs::Harness::activate. The mature
    // fee owner permits a bank residual only under the actually active V2 tuple.
    sqlx::query("INSERT INTO game_type2_audit_activation VALUES (1, now())")
        .execute(&h.pool)
        .await?;
    // BANK0-RL-02 allows20m per deposit. Keep25 historical operation/entry
    // pairs and one exact ledger head, rather than inventing a500m deposit.
    let mut previous = None::<sqlx::types::Uuid>;
    for n in 0_i64..25 {
        let (occurrence,operation,event,platinum,gold,entry):(sqlx::types::Uuid,sqlx::types::Uuid,sqlx::types::Uuid,sqlx::types::Uuid,sqlx::types::Uuid,sqlx::types::Uuid)=sqlx::query_as("SELECT game_character_uuid_v7(),game_character_uuid_v7(),game_character_uuid_v7(),game_character_uuid_v7(),game_character_uuid_v7(),game_character_uuid_v7()").fetch_one(&mut *tx).await?;
        sqlx::query("INSERT INTO game_account_bank_operations(operation_occurrence_id,transaction_id,event_id,kind,request_binding,acting_character_id,account_id,world_id,channel_id,runtime_scope_ownership_generation,amount,outcome,occurred_at,envelope_sha256,committed_at,planned_platinum_item_instance_id,planned_gold_item_instance_id,backpack_item_instance_id) VALUES($1,$2,$3,1,sha256('historical royal wealth'),encode($4,'hex')::uuid,encode($5,'hex')::uuid,encode($6,'hex')::uuid,encode($7,'hex')::uuid,1,20000000,0,1,sha256('historical royal wealth'),0,$8,$9,encode($10,'hex')::uuid)")
            .bind(occurrence).bind(operation).bind(event).bind(id(41).as_slice()).bind(id(40).as_slice()).bind(id(42).as_slice()).bind(id(43).as_slice()).bind(platinum).bind(gold).bind(id(70).as_slice()).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_account_bank_entries(entry_id,transaction_id,account_id,world_id,kind,amount,balance_before,balance_after,previous_entry_id,acting_character_id) VALUES($1,$2,encode($3,'hex')::uuid,encode($4,'hex')::uuid,1,20000000,$5,$6,$7,encode($8,'hex')::uuid)")
            .bind(entry).bind(operation).bind(id(40).as_slice()).bind(id(42).as_slice()).bind(n*20000000).bind((n+1)*20000000).bind(previous).bind(id(41).as_slice()).execute(&mut *tx).await?;
        previous = Some(entry);
    }
    sqlx::query("INSERT INTO game_account_bank_balances(account_id,world_id,balance,last_entry_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,500000000,$3)")
        .bind(id(40).as_slice()).bind(id(42).as_slice()).bind(previous).execute(&mut *tx).await?;
    tx.commit().await?;
    super::super::grizzly_npc::wire::talk_paced(
        owner,
        admitted,
        npc,
        &[
            (1, "hi"),
            (2, "outfit"),
            (3, "yes"),
            (4, "armor"),
            (5, "yes"),
        ],
    )
    .await?;
    let kit:Vec<u8>=sqlx::query_scalar("SELECT uuid_send(i.item_instance_id) FROM game_item_character_inbox_locations inbox JOIN game_item_instances i USING(item_instance_id) WHERE inbox.character_id=encode($1,'hex')::uuid AND i.definition_production_key='oteryn:item.tibia.i23398'").bind(id(41).as_slice()).fetch_one(&h.pool).await.map_err(|error|format!("actual royal StoreInbox kit missing: {error}"))?;
    assert_eq!(kit.len(), 16);
    let stage:Option<i64>=sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key='oteryn:quest-progress/crystalserver/outfit_quest/golden_outfit'").bind(id(41).as_slice()).fetch_optional(&h.pool).await?;
    assert_eq!(stage, Some(1));
    let bank: i64 = sqlx::query_scalar(
        "SELECT balance FROM game_account_bank_balances WHERE account_id=encode($1,'hex')::uuid",
    )
    .bind(id(40).as_slice())
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(
        bank, 0,
        "actual500m fee and grant must commit with one native kit"
    );
    let target:String=sqlx::query_scalar("SELECT target_key FROM game_house_furniture_kit_origins WHERE item_instance_id=encode($1,'hex')::uuid").bind(&kit).fetch_one(&h.pool).await?;
    assert_eq!(target, "oteryn:item.tibia.i31510");
    Ok(())
}
