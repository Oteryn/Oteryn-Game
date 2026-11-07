//! Genuine ordinary-death backpack, observed and recovered by another authorized actor.
use super::*;
#[path = "death_ground/observer.rs"]
mod observer;
#[path = "death_ground/wire.rs"]
mod wire;

pub(super) async fn qualify(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    deps: &Dependencies,
    victim_at_temple: AdmittedSession,
    backpack: [u8; 16],
    expected_children: &[[u8; 16]],
    timer_before_death: &serde_json::Value,
) -> TestResult {
    assert!(
        !expected_children.is_empty(),
        "high-level death fixture must retain contents"
    );
    let victim = victim_at_temple
        .item_fence
        .ok_or("actual Temple victim fence")?
        .character_id;
    let receipt:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_player_death_item_records r JOIN game_character_death_receipts d USING(death_occurrence_id) JOIN game_item_ground_locations g USING(item_instance_id,world_id) WHERE r.character_id=encode($1,'hex')::uuid AND r.item_instance_id=encode($2,'hex')::uuid AND r.operation=1 AND r.source_slot=9 AND r.scope_kind=2 AND g.scope_kind=2 AND g.instance_id=r.instance_id AND d.death_scope_kind=2 AND d.death_instance_id=r.instance_id AND g.spatial_position=d.death_spatial_position)")
        .bind(victim.as_bytes().as_slice()).bind(backpack.as_slice()).fetch_one(&h.pool).await?;
    assert!(
        receipt,
        "actual selected whole-backpack loss must be committed in its exact House namespace"
    );
    assert!(
        timer_before_death["deadline_at"]
            .as_i64()
            .is_some_and(|deadline| deadline > 0),
        "actual lamp USE must supply its original running deadline"
    );
    assert_eq!(
        timer_before_death["remaining_ms"], 180000,
        "source lamp duration is three minutes"
    );
    let before = children(h, backpack).await?;
    assert_eq!(
        timer(h, expected_children[0]).await?,
        *timer_before_death,
        "same original timer survives whole-backpack death Ground custody"
    );
    assert_eq!(before.len(), expected_children.len());
    for child in expected_children {
        assert!(before.iter().any(|row| {
            row["id"]
                == serde_json::json!(
                    child
                        .iter()
                        .map(|byte| format!("{byte:02x}"))
                        .collect::<String>()
                )
        }));
    }
    let guest = observer::observer(h, owner, deps, victim_at_temple).await?;
    let fence = guest.item_fence.ok_or("actual guest House fence")?;
    let definition:String=sqlx::query_scalar("SELECT definition_production_key FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid").bind(backpack.as_slice()).fetch_one(&h.pool).await?;
    let reference = {
        let runtime = owner.runtime.lock().await;
        crate::gameplay_transport::item_ref_admission::pinned_index(
            &runtime,
            owner.active_generation,
        )
        .ok_or("observer index")?
        .definition_ref("Item", &definition, "definition-r1")
        .ok_or("canonical dropped backpack reference")?
    };
    let guest = wire::recover(owner, guest, reference).await?;
    assert!(
        matches!(
            guest
                .item_fence
                .ok_or("guest successor fence")?
                .runtime_scope,
            crate::foundation::RuntimeScopeRefV1::Channel { .. }
        ),
        "current guest returns through the real HouseExit before Channel clock settlement"
    );
    assert_eq!(
        timer(h, expected_children[0]).await?,
        *timer_before_death,
        "same original timer survives open/pickup/reopen/real Exit without recursive restart"
    );
    let actual:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_item_container_entries e JOIN game_item_instances i USING(item_instance_id,world_id) WHERE e.item_instance_id=encode($1,'hex')::uuid AND e.character_id=encode($2,'hex')::uuid AND i.lifecycle=1 AND i.quantity=1) AND NOT EXISTS(SELECT 1 FROM game_item_ground_locations WHERE item_instance_id=encode($1,'hex')::uuid)")
        .bind(backpack.as_slice()).bind(fence.character_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    assert!(
        actual,
        "the same original backpack UUID is now in the authorized guest's carried tree"
    );
    assert_eq!(
        children(h, backpack).await?,
        before,
        "descendant UUIDs and immutable item values survive real open/pickup/reopen"
    );
    // Explicit isolated-fixture due setup AFTER proving the original timer was
    // preserved unchanged. This advances no loot reservation or gameplay authority.
    let mut setup = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *setup)
        .await?;
    sqlx::query("UPDATE game_item_timed_states SET deadline_at=floor(extract(epoch FROM statement_timestamp())*1000)::bigint-1,remaining_ms=1 WHERE item_instance_id=encode($1,'hex')::uuid").bind(expected_children[0].as_slice()).execute(&mut *setup).await?;
    setup.commit().await?;
    tokio::time::sleep(std::time::Duration::from_millis(1100)).await;
    owner.tick_timed_items().await;
    let first:serde_json::Value=sqlx::query_scalar("SELECT to_jsonb(i) FROM game_item_instances i WHERE item_instance_id=encode($1,'hex')::uuid").bind(expected_children[0].as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        // Crystal201189 data/items/items.xml:6288–6293: lit2915 decays
        // into used2916;2914 is the original unused lamp, never refilled here.
        first["definition_production_key"],
        "oteryn:item.tibia.i2916",
        "actual current guest owner expires the same nested lamp to its source used form"
    );
    let running:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_item_timed_states WHERE item_instance_id=encode($1,'hex')::uuid AND deadline_at IS NOT NULL)").bind(expected_children[0].as_slice()).fetch_one(&h.pool).await?;
    assert!(
        !running,
        "existing timed owner removes the expired running deadline"
    );
    tokio::time::sleep(std::time::Duration::from_millis(1100)).await;
    owner.tick_timed_items().await;
    let second:serde_json::Value=sqlx::query_scalar("SELECT to_jsonb(i) FROM game_item_instances i WHERE item_instance_id=encode($1,'hex')::uuid").bind(expected_children[0].as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        first, second,
        "later native clock cannot expire or transform the original child twice"
    );
    eprintln!(
        "SRV7_HOUSE_DEATH_GROUND_REAL_WIRE_EXECUTED children={}",
        expected_children.len()
    );
    Ok(())
}
async fn children(h: &Harness, parent: [u8; 16]) -> TestResult<Vec<serde_json::Value>> {
    Ok(sqlx::query_scalar("SELECT jsonb_build_object('id',encode(uuid_send(i.item_instance_id),'hex'),'parent',encode(uuid_send(e.parent_item_instance_id),'hex'),'item',to_jsonb(i),'entry',e.exact_entry) FROM (SELECT n.item_instance_id,n.world_id,n.parent_item_instance_id,to_jsonb(n) AS exact_entry FROM game_item_nested_entries n UNION ALL SELECT d.item_instance_id,d.world_id,d.parent_item_instance_id,to_jsonb(d) AS exact_entry FROM game_item_container_entries d) e JOIN game_item_instances i USING(item_instance_id,world_id) WHERE e.parent_item_instance_id=encode($1,'hex')::uuid ORDER BY i.item_instance_id")
        .bind(parent.as_slice()).fetch_all(&h.pool).await?)
}

async fn timer(h: &Harness, item: [u8; 16]) -> TestResult<serde_json::Value> {
    Ok(sqlx::query_scalar("SELECT to_jsonb(t) FROM game_item_timed_states t WHERE item_instance_id=encode($1,'hex')::uuid").bind(item.as_slice()).fetch_one(&h.pool).await?)
}
