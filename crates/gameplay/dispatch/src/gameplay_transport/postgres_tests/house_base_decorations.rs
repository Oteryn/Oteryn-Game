//! W18: static decoration owner uses the same real named-House session as item USE.
use super::TestResult;
use super::fixture::{Harness, debug};
use crate::gameplay_transport::ComposedFreshAdmission;
use crate::gameplay_transport::connection::AdmittedSession;
use crate::gameplay_transport::connection::UseCommand;
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};

pub(super) async fn window_scenario(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    admitted: AdmittedSession,
) -> TestResult {
    let fence = admitted.item_fence.ok_or("House fence")?;
    let actor = admitted.runtime_actor.ok_or("House actor")?;
    let context = {
        let runtime = owner.runtime.lock().await;
        crate::durability::consume_transform::HouseActionContext::from_native_owner(
            &runtime,
            actor,
            admitted.game_session_id,
            fence,
        )
        .map_err(debug)?
    };
    let position = [32310_i32, 32804, 7]
        .into_iter()
        .flat_map(i32::to_be_bytes)
        .collect::<Vec<_>>();
    let bundle = owner.bundle_map().ok_or("bundle")?.overlay.base().digest();
    let fact = owner.holder.fact();
    let connection = fence.connection_generation.get();
    let apply = |command: u64, expected: u64, after: &str, current_connection: u64| {
        sqlx::query_scalar::<_, String>("SELECT game_house_base_decoration_set_fenced(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric,$4::text::numeric,$5::text::numeric,encode($6,'hex')::uuid,$7::text::numeric,encode($8,'hex')::uuid,$9,$10,$11::text::numeric,$12::text::numeric,$13::text::numeric,$14,$15,$16,$17)")
            .bind(fence.game_session_id.as_bytes().to_vec())
            .bind(fence.character_id.as_bytes().to_vec())
            .bind(current_connection.to_string())
            .bind(fence.character_lease_generation.to_string())
            .bind(fence.scope_ownership_generation.get().to_string())
            .bind(fact.node_id().as_bytes().to_vec())
            .bind(fact.registration_revision().to_string())
            .bind(context.house().world_id().as_bytes().to_vec())
            .bind(context.house().house_key().to_owned())
            .bind(bundle.to_vec())
            .bind(command.to_string()).bind(1234_u64.to_string())
            .bind(expected.to_string()).bind("oteryn:item.tibia.i5302")
            .bind(after.to_owned()).bind(vec![u8::try_from(command).unwrap_or(0)])
            .bind(position.clone())
    };
    assert_eq!(
        apply(401, 0, "oteryn:item.tibia.i6447", connection)
            .fetch_one(&h.pool)
            .await?,
        "committed"
    );
    assert_eq!(
        apply(401, 0, "oteryn:item.tibia.i6447", connection)
            .fetch_one(&h.pool)
            .await?,
        "committed"
    );
    assert_eq!(
        apply(402, 0, "oteryn:item.tibia.i6447", connection)
            .fetch_one(&h.pool)
            .await?,
        "stale"
    );
    let state: (String, String) = sqlx::query_as("SELECT definition_key,revision::text FROM game_house_base_decorations WHERE world_id=encode($1,'hex')::uuid AND house_key=$2 AND bundle_digest=$3 AND placement_key=1234")
        .bind(context.house().world_id().as_bytes().as_slice()).bind(context.house().house_key())
        .bind(bundle.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(state, ("oteryn:item.tibia.i6447".into(), "1".into()));
    assert_eq!(
        apply(403, 1, "oteryn:item.tibia.i3031", connection)
            .fetch_one(&h.pool)
            .await?,
        "invalid"
    );
    assert_eq!(
        apply(404, 1, "oteryn:item.tibia.i6447", connection + 1)
            .fetch_one(&h.pool)
            .await?,
        "stale",
        "a replaced connection cannot write or replay the House placement"
    );
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_house_base_decoration_receipts WHERE game_session_id=encode($1,'hex')::uuid")
        .bind(fence.game_session_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(count, 1, "replay and refusals write no extra receipt");

    // The production caller resolves an actual window from the admitted House bundle.
    // The window is adjacent to, rather than on, the House arrival cell.
    let window_position = [32310_i32, 32804, 7];
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "INSERT INTO game_house_tiles VALUES(encode($1,'hex')::uuid,$2,$3) ON CONFLICT DO NOTHING",
    )
    .bind(context.house().world_id().as_bytes().as_slice())
    .bind(context.house().house_key())
    .bind(
        window_position
            .into_iter()
            .flat_map(i32::to_be_bytes)
            .collect::<Vec<_>>(),
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    let position = crate::map::overlay::TilePos {
        x: 32310,
        y: 32804,
        floor: -7,
    };
    let placement = crate::map::view::placement_key(position, 1).ok_or("window placement")?;
    let target = WorldObjectTarget {
        placement: bundle.into_iter().chain(placement.to_be_bytes()).collect(),
        expected_revision: 0,
    };
    let command = UseCommand {
        game_session_id: admitted.game_session_id,
        command_id: 405,
        item_fence: Some(fence),
    };
    assert_eq!(
        owner
            .use_world_object(actor, command, target.clone())
            .await
            .disposition,
        UseDisposition::Committed
    );
    assert_eq!(
        owner
            .use_world_object(actor, command, target.clone())
            .await
            .disposition,
        UseDisposition::Committed,
        "the immutable command is replayed before comparing its old placement revision"
    );
    assert_eq!(
        owner
            .use_world_object(
                actor,
                UseCommand {
                    command_id: 406,
                    ..command
                },
                target
            )
            .await
            .disposition,
        UseDisposition::StaleState
    );

    // Drive the real House observer, encode its native map snapshot, then decode it as
    // the client does. The changed source placement and revision survive a fresh view.
    let mut view = crate::gameplay_transport::world_map::SessionMapView::default();
    let mut items =
        crate::gameplay_transport::item_view::SessionItemView::resume(Default::default())
            .with_map_view();
    let update = owner
        .observe_house_item_map(admitted, &mut view, &mut items)
        .await
        .ok_or("House map observer")?
        .map_err(debug)?
        .ok_or("House map snapshot")?;
    let crate::gameplay_transport::world_map::MapUpdate::Snapshot(snapshot) = update else {
        return Err("expected fresh House snapshot".into());
    };
    let map = oteryn_protocol_oteryn::world_map::decode_world_map_snapshot(&snapshot.payload)
        .map_err(debug)?;
    let native = owner
        .active_generation
        .and_then(|active| active.native_gameplay())
        .ok_or("native content")?;
    let after = native
        .item_index()
        .and_then(|index| index.definition_ref("Item", "oteryn:item.tibia.i6447", "definition-r1"))
        .ok_or("open window")?;
    let shown = map
        .tiles
        .iter()
        .find(|tile| {
            tile.position.x == 32310 && tile.position.y == 32804 && tile.position.floor == -7
        })
        .and_then(|tile| {
            tile.items.iter().find(|item| {
                item.definition == oteryn_protocol_oteryn::world_map::MapDefinition::Item(after)
            })
        })
        .ok_or("open window native projection")?;
    assert_eq!(shown.appearance_id, 6447);
    assert_eq!(
        shown.origin,
        oteryn_protocol_oteryn::world_map::MapOrigin::BaseOrdinal {
            ordinal: 1,
            object_revision: 1
        }
    );
    let total: i64 = sqlx::query_scalar("SELECT count(*) FROM game_house_base_decoration_receipts WHERE game_session_id=encode($1,'hex')::uuid")
        .bind(fence.game_session_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        total, 2,
        "actual caller replay and stale input create no extra transition"
    );
    eprintln!("W18_STATIC_POSTGRES_COMPLETED window_fenced_commit_replay_stale_and_wrong_pair");
    Ok(())
}

pub(super) async fn aqueduct_scenario(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    admitted: AdmittedSession,
) -> TestResult {
    let fence = admitted.item_fence.ok_or("House fence")?;
    let actor = admitted.runtime_actor.ok_or("House actor")?;
    let context = {
        let runtime = owner.runtime.lock().await;
        crate::durability::consume_transform::HouseActionContext::from_native_owner(
            &runtime,
            actor,
            admitted.game_session_id,
            fence,
        )
        .map_err(debug)?
    };
    let bundle = owner.bundle_map().ok_or("bundle")?.overlay.base().digest();
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,0,$2,'static-fixture',$2,$2,$2,'TIBIA_SERVER_SAVE_BERLIN_06',1,clock_timestamp()-interval '201.5 seconds','2026-11') ON CONFLICT(world_id,reset_epoch) DO UPDATE SET target_digest=EXCLUDED.target_digest,activated_at=EXCLUDED.activated_at")
        .bind(context.house().world_id().as_bytes().as_slice()).bind(bundle.as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,0) ON CONFLICT(world_id) DO UPDATE SET reset_epoch=0")
        .bind(context.house().world_id().as_bytes().as_slice()).execute(&mut *tx).await?;
    sqlx::query(
        "INSERT INTO game_house_tiles VALUES(encode($1,'hex')::uuid,$2,$3) ON CONFLICT DO NOTHING",
    )
    .bind(context.house().world_id().as_bytes().as_slice())
    .bind(context.house().house_key())
    .bind(
        [32310_i32, 32804, 7]
            .into_iter()
            .flat_map(i32::to_be_bytes)
            .collect::<Vec<_>>(),
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    let changes = owner
        .house_base_decoration_facts(&context, bundle)
        .await
        .ok_or("timed House materialization")?;
    assert_eq!(changes.len(), 1);
    assert_eq!(
        changes[0].appearance_id, 35951,
        "offline reset epoch advances through both due phases"
    );
    assert_eq!(changes[0].revision, 1);
    let advanced: serde_json::Value = sqlx::query_scalar(
        "SELECT game_house_base_decay_advance('oteryn:item.tibia.i35949',1000000,1000000 + 402000::bigint * 100000 + 1500)",
    )
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(advanced["key"], "oteryn:item.tibia.i35951");
    eprintln!("W18_STATIC_POSTGRES_COMPLETED aqueduct_reset_epoch_offline_catchup");
    Ok(())
}

pub(super) async fn bed_scenario(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    admitted: AdmittedSession,
    kit: u32,
) -> TestResult {
    use oteryn_protocol_oteryn::world_object::{
        MapUseTarget, UseIntent, UseTarget, encode_item_use_intent,
    };
    let fence = admitted.item_fence.ok_or("House fence")?;
    let actor = admitted.runtime_actor.ok_or("House actor")?;
    let context = {
        let runtime = owner.runtime.lock().await;
        crate::durability::consume_transform::HouseActionContext::from_native_owner(
            &runtime,
            actor,
            admitted.game_session_id,
            fence,
        )
        .map_err(debug)?
    };
    let bundle = owner.bundle_map().ok_or("bundle")?.overlay.base().digest();
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    for y in [32804, 32805] {
        sqlx::query("INSERT INTO game_house_tiles VALUES(encode($1,'hex')::uuid,$2,$3) ON CONFLICT DO NOTHING")
            .bind(context.house().world_id().as_bytes().as_slice()).bind(context.house().house_key())
            .bind([32310_i32,y,7].into_iter().flat_map(i32::to_be_bytes).collect::<Vec<_>>()).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    let placement = crate::map::view::placement_key(
        crate::map::overlay::TilePos {
            x: 32310,
            y: 32804,
            floor: -7,
        },
        1,
    )
    .ok_or("bed placement")?;
    let target = WorldObjectTarget {
        placement: bundle.into_iter().chain(placement.to_be_bytes()).collect(),
        expected_revision: 0,
    };
    let source = owner
        .active_generation
        .and_then(|a| a.native_gameplay())
        .and_then(|n| n.item_index())
        .and_then(|i| {
            i.definition_ref(
                "Item",
                &format!("oteryn:item.tibia.i{kit}"),
                "definition-r1",
            )
        })
        .ok_or("kit definition")?;
    let payload = encode_item_use_intent(&UseIntent {
        target: UseTarget::Item(std::num::NonZeroU64::new(1).ok_or("handle")?),
        use_with: None,
        use_with_map_item: Some(MapUseTarget::Object(target)),
        use_with_position: None,
    })
    .map_err(debug)?;
    let (_, outcome, _, _) = super::house_balloon::wire::exchange(
        owner,
        admitted,
        admitted.continuity.next_command_id,
        &payload,
        super::house_balloon::wire::HouseExchange {
            current: Some(source),
            after: None,
            nested: None,
            foreign_after_snapshot: None,
        },
    )
    .await?;
    assert_eq!(outcome, UseDisposition::Committed);
    let keys: Vec<String> = sqlx::query_scalar("SELECT definition_key FROM game_house_base_decorations WHERE world_id=encode($1,'hex')::uuid AND house_key=$2 ORDER BY placement_key")
        .bind(context.house().world_id().as_bytes().as_slice()).bind(context.house().house_key()).fetch_all(&h.pool).await?;
    assert_eq!(
        keys,
        vec!["oteryn:item.tibia.i734", "oteryn:item.tibia.i735"]
    );
    eprintln!("W18_STATIC_POSTGRES_COMPLETED bed_kit_atomic_dual_part");
    Ok(())
}
