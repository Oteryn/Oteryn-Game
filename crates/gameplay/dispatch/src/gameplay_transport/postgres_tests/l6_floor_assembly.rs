//! Crystal's helmet altar consumes seven real floor pieces after the final wire drop.
use super::fixture::Harness;
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::character_progression::CurrentCharacterGameplayFence;
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use oteryn_gameplay_quests::quest::world_callbacks::{Offering, Position};
use prost::Message;
use sha2::{Digest, Sha256};

const ALTAR: Position = Position {
    x: 7,
    y: 1,
    floor: -7,
};

#[test]
fn l6_floor_assembly_final_drop_consumes_seven_and_creates_one_helmet() -> TestResult {
    run_case("l6-floor-assembly")
}

#[test]
fn l6_floor_assembly_missing_part_keeps_every_other_item() -> TestResult {
    run_case("l6-floor-assembly-missing")
}

#[test]
fn l6_floor_assembly_unbound_tile_cannot_assemble() -> TestResult {
    run_case("l6-floor-assembly-unbound")
}

fn run_case(name: &'static str) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    eprintln!("POSTGRES_CASE_STARTED {name}");
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || {
            super::quest_world_callbacks::scenario(name, ALTAR, false)
                .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "floor assembly scenario panicked")??;
    eprintln!(
        "POSTGRES_CASE_COMPLETED {name} duration={:?}",
        started.elapsed()
    );
    Ok(())
}

async fn verify_audit(harness: &Harness) -> TestResult {
    use crate::durability::item_mint_audit::{
        EventEnvelopeV1, OneItemOperationV1, OneItemTransactionV1,
    };
    let (wire, snapshots, digest, atomic, fresh_order): (Vec<u8>, Vec<u8>, Vec<u8>, bool, bool) = sqlx::query_as(
        "SELECT a.envelope,r.inputs_intent,r.inputs_sha256,a.created_xact_id=r.created_xact_id AND m.created_xact_id=r.created_xact_id,g.stack_ordinal>(SELECT max((v->'ground'->>'stack_ordinal')::bigint) FROM jsonb_array_elements(r.inputs) v) FROM game_floor_assembly_receipts r JOIN game_item_audit_outbox a USING(transaction_id) JOIN game_item_loop_move_receipts m ON m.transaction_id=r.move_transaction_id JOIN game_item_ground_locations g ON g.item_instance_id=r.output_item_instance_id",
    ).fetch_one(&harness.pool).await?;
    assert!(atomic && fresh_order);
    assert_eq!(Sha256::digest(&snapshots).as_slice(), digest);
    let envelope = EventEnvelopeV1::decode(wire.as_slice())?;
    assert!(envelope.payload.len() <= crate::durability::item_mint_audit::RL07_PAYLOAD_BYTES_MAX);
    let transaction = OneItemTransactionV1::decode(envelope.payload.as_slice())?;
    let Some(OneItemOperationV1::Craft(craft)) = transaction.operation else {
        return Err("assembly audit operation".into());
    };
    let proof: serde_json::Value = serde_json::from_slice(&craft.proof)?;
    let committed: Vec<u8> = serde_json::from_value(proof["inputs_sha256"].clone())?;
    assert_eq!(committed, digest);
    let original: Vec<serde_json::Value> = serde_json::from_slice(&snapshots)?;
    let balances = proof["inputs"].as_array().ok_or("input balances")?;
    assert_eq!(balances.len(), 7);
    for (balance, before) in balances.iter().zip(&original) {
        assert_eq!(balance["item"], before["item"]["item_instance_id"]);
        assert_eq!(balance["quantity"], before["item"]["quantity"]);
        assert_eq!(balance["state_revision"], before["item"]["state_revision"]);
        assert_ne!(proof["output"]["item"], balance["item"]);
    }
    Ok(())
}

async fn probe_atomic_refusals(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    harness: &Harness,
    fence: CurrentCharacterGameplayFence,
) -> TestResult {
    use crate::durability::item_loop_move::{
        FloorAssemblyTarget, LoopDestination, LoopGround, LoopMoveRefusal, LoopMoveRequest,
    };
    use crate::gameplay_transport::item_ref_admission::NativeItemWeights;
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    let source: Vec<u8> = sqlx::query_scalar("SELECT uuid_send(i.item_instance_id) FROM game_quest_callback_carried(encode($1,'hex')::uuid,encode($2,'hex')::uuid) c JOIN game_item_instances i ON i.item_instance_id=c.item WHERE i.definition_production_key='oteryn:item.tibia.i3228' LIMIT 1")
        .bind(fence.character_id.as_bytes().as_slice()).bind(owner.world_id.as_bytes().as_slice()).fetch_one(&harness.pool).await?;
    let permit = owner.spell_lane_permit().await.ok_or("assembly permit")?;
    let frame = {
        owner
            .runtime
            .lock()
            .await
            .content_pin()
            .frame_binding_digest()
            .to_vec()
    };
    let map = owner.bundle_map().ok_or("map")?;
    let key = oteryn_world_bundle::bundle::placement_key(ALTAR.floor, ALTAR.x, ALTAR.y, 1)
        .ok_or("altar")?;
    assert!(
        map.facts
            .quest_callback_matches(key, None, Some(40040), None)
    );
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let revision = crate::map::overlay::map_revision(map.overlay.base());
    let target = FloorAssemblyTarget {
        action: 40040,
        placement,
        scope: crate::durability::terrain_tool::TerrainScope {
            map_revision: revision.clone(),
            content_generation: native.source_digest(),
            reset_epoch: owner.reset_epoch.try_into()?,
        },
    };
    let request = LoopMoveRequest {
        item: source.try_into().map_err(|_| "source UUID")?,
        count: Some(1),
        pickup_cell: None,
        destination: LoopDestination::Ground(LoopGround {
            spatial_position: crate::durability::map_item_mint_audit::map_item_spatial_position(
                ALTAR.x,
                ALTAR.y,
                ALTAR.floor,
            ),
            map_revision: revision,
            content_revision: format!(
                "sha256:{}",
                native
                    .source_digest()
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect::<String>()
            ),
            placement_context: frame,
        }),
    };
    let command = crate::foundation::CommandRef::new(
        fence.game_session_id,
        crate::foundation::CommandId::new(1)?,
    );
    if owner.root.has_ready_demand() {
        owner.root.maintain_ready_once().await?;
    }
    let mut busy_ground = harness.pool.begin().await?;
    sqlx::query(
        "SELECT pg_advisory_xact_lock(hashtextextended(encode($1,'hex') || encode($2,'hex'),33))",
    )
    .bind(owner.world_id.as_bytes().as_slice())
    .bind(
        fence
            .runtime_scope
            .channel_id()
            .ok_or("channel")?
            .as_bytes()
            .as_slice(),
    )
    .execute(&mut *busy_ground)
    .await?;
    let busy = owner
        .root
        .commit_item_loop_floor_assembly(
            &permit,
            owner.character,
            owner.holder,
            fence.item_fence(),
            command,
            &NativeItemWeights(native),
            native,
            request.clone(),
            target.clone(),
        )
        .await?;
    assert!(matches!(busy, Err(LoopMoveRefusal::Rejected)));
    busy_ground.rollback().await?;
    assert_no_consequence(harness).await?;
    if owner.root.has_ready_demand() {
        owner.root.maintain_ready_once().await?;
    }
    let mut held = harness.pool.begin().await?;
    sqlx::query("SELECT 1 FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i3222' FOR UPDATE").execute(&mut *held).await?;
    let result = owner
        .root
        .commit_item_loop_floor_assembly(
            &permit,
            owner.character,
            owner.holder,
            fence.item_fence(),
            command,
            &NativeItemWeights(native),
            native,
            request.clone(),
            target.clone(),
        )
        .await
        .map_err(|error| format!("contention transaction: {error:?}"))?;
    eprintln!("FLOOR_PROBE contention returned {result:?}");
    assert!(matches!(result, Err(LoopMoveRefusal::Rejected)));
    held.rollback().await?;
    assert_no_consequence(harness).await?;
    sqlx::query("CREATE SEQUENCE test_floor_fail_seen")
        .execute(&harness.pool)
        .await?;
    sqlx::query("GRANT USAGE,SELECT ON SEQUENCE test_floor_fail_seen TO oteryn_game_runtime")
        .execute(&harness.pool)
        .await?;
    sqlx::query("CREATE FUNCTION test_floor_fail() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN PERFORM nextval('test_floor_fail_seen'); RAISE EXCEPTION 'floor rollback probe'; END $$").execute(&harness.pool).await?;
    sqlx::query("CREATE TRIGGER test_floor_fail BEFORE INSERT ON game_floor_assembly_receipts FOR EACH ROW EXECUTE FUNCTION test_floor_fail()").execute(&harness.pool).await?;
    if owner.root.has_ready_demand() {
        owner.root.maintain_ready_once().await?;
    }
    let failed = owner
        .root
        .commit_item_loop_floor_assembly(
            &permit,
            owner.character,
            owner.holder,
            fence.item_fence(),
            command,
            &NativeItemWeights(native),
            native,
            request,
            target,
        )
        .await;
    eprintln!("FLOOR_PROBE injected result {failed:?}");
    assert!(failed.is_err());
    let reached: bool = sqlx::query_scalar("SELECT is_called FROM test_floor_fail_seen")
        .fetch_one(&harness.pool)
        .await?;
    assert!(
        reached,
        "rollback fault must execute inside the assembly transaction"
    );
    assert_no_consequence(harness).await?;
    sqlx::query("DROP TRIGGER test_floor_fail ON game_floor_assembly_receipts")
        .execute(&harness.pool)
        .await?;
    sqlx::query("DROP SEQUENCE test_floor_fail_seen")
        .execute(&harness.pool)
        .await?;
    sqlx::query("DROP FUNCTION test_floor_fail()")
        .execute(&harness.pool)
        .await?;
    Ok(())
}

async fn assert_no_consequence(harness: &Harness) -> TestResult {
    let counts: (i64, i64, i64) = sqlx::query_as("SELECT (SELECT count(*) FROM game_item_instances WHERE definition_production_key=ANY(ARRAY['oteryn:item.tibia.i3222','oteryn:item.tibia.i3223','oteryn:item.tibia.i3224','oteryn:item.tibia.i3225','oteryn:item.tibia.i3226','oteryn:item.tibia.i3227','oteryn:item.tibia.i3228']) AND lifecycle=1),(SELECT count(*) FROM game_floor_assembly_receipts),(SELECT count(*) FROM game_item_loop_move_receipts WHERE command_id=1)").fetch_one(&harness.pool).await?;
    assert_eq!(counts, (7, 0, 0));
    Ok(())
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    harness: &Harness,
    fence: CurrentCharacterGameplayFence,
    name: &str,
) -> TestResult {
    eprintln!("POSTGRES_DATABASE_READY l6_floor_assembly");
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(
            owner.offered_capabilities(),
            &[
                4,
                6,
                11,
                12,
                14,
                15,
                18,
                oteryn_protocol_oteryn::item_view::CAPABILITY_ITEM_STACK_COUNT_V1,
            ],
        )
        .ok_or("floor drop capabilities")?;
    for id in 3222_u64..=3228 {
        if name.ends_with("-missing") && id == 3223 {
            continue;
        }
        super::quest_world_callbacks::input_item(
            owner,
            fence.item_fence(),
            &Offering {
                position: ALTAR,
                item: format!("oteryn:item.tibia.i{id}"),
                count: 1,
            },
            id - 3162,
            id == 3228,
        )
        .await
        .map_err(|error| format!("floor piece {id}: {error}"))?;
    }
    if name == "l6-floor-assembly" {
        probe_atomic_refusals(owner, harness, fence).await?;
    }
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    let reference = native
        .item_index()
        .ok_or("item index")?
        .definition_ref("Item", "oteryn:item.tibia.i3228", "definition-r1")
        .ok_or("helmet piece definition")?;
    eprintln!("FLOOR_PROBE starting wire drop");
    let (next, result, payload) = super::map_seed_planting::drop_exchange_at(
        owner,
        *admitted,
        1,
        None,
        reference,
        1,
        false,
        oteryn_protocol_oteryn::item_view::WorldTilePosition {
            x: 7,
            y: 1,
            floor: -7,
        },
    )
    .await?;
    *admitted = next;
    assert_eq!(
        result,
        Some(oteryn_protocol_oteryn::item_view::ItemMoveOutcome::Moved)
    );
    let counts: (i64, i64) = sqlx::query_as(
        "SELECT count(*) FILTER (WHERE definition_production_key=ANY($1::text[]) AND lifecycle=1), count(*) FILTER (WHERE definition_production_key='oteryn:item.tibia.i3229' AND lifecycle=1) FROM game_item_instances",
    ).bind((3222..=3228).map(|id| format!("oteryn:item.tibia.i{id}")).collect::<Vec<_>>())
        .fetch_one(&harness.pool).await?;
    if name.ends_with("-missing") {
        assert_eq!(counts, (6, 0));
    } else if name.ends_with("-unbound") {
        assert_eq!(counts, (7, 0));
    } else {
        assert_eq!(
            counts,
            (0, 1),
            "seven floor pieces must become one fresh helmet atomically"
        );
        verify_audit(harness).await?;
    }
    let receipt_count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM game_floor_assembly_receipts")
            .fetch_one(&harness.pool)
            .await?;
    assert_eq!(receipt_count, i64::from(name == "l6-floor-assembly"));
    let (next, replay, _) = super::map_seed_planting::drop_exchange_at(
        owner,
        *admitted,
        1,
        Some(payload),
        reference,
        1,
        false,
        oteryn_protocol_oteryn::item_view::WorldTilePosition {
            x: 7,
            y: 1,
            floor: -7,
        },
    )
    .await?;
    *admitted = next;
    assert_eq!(replay, result);
    Ok(())
}
