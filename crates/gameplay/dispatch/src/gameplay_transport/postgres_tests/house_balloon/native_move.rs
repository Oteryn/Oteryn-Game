//! Actual framed9 House placement/take; no historical floor-row staging.
use super::*;
use oteryn_protocol_oteryn::item_view::{ItemMoveDestination, ItemMoveOutcome, WorldTilePosition};
pub(super) async fn exercise(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    mut admitted: AdmittedSession,
    source: [u8; 16],
    reference: std::num::NonZeroU32,
) -> TestResult {
    let original:Value=sqlx::query_scalar("SELECT to_jsonb(i) FROM game_item_instances i WHERE item_instance_id=encode($1,'hex')::uuid").bind(source.as_slice()).fetch_one(&h.pool).await?;
    let toy = original["definition_production_key"]
        .as_str()
        .and_then(|k| k.strip_prefix("oteryn:item.tibia.i"))
        .and_then(|k| k.parse::<u32>().ok())
        .filter(|id| matches!(id, 6566..=6568));
    if let Some(id) = toy {
        super::santa_toy_lifetime::before_first_move(h, source, id).await?;
    }
    let place = ItemMoveDestination::Ground(WorldTilePosition {
        x: POSITION[0],
        y: POSITION[1],
        floor: -i16::try_from(POSITION[2])?,
    });
    let take = ItemMoveDestination::MainBackpack;
    for (ordinal, destination) in [place, take].into_iter().enumerate() {
        let command = u64::try_from(ordinal)? + 1;
        let capture = std::sync::Arc::new(std::sync::Mutex::new(super::wire::HouseWireCapture {
            move_destination: Some(destination),
            ..Default::default()
        }));
        let view = || HouseExchange {
            rotate: false,
            current: Some(reference),
            after: None,
            nested: None,
            foreign_after_snapshot: None,
            capture: Some(std::sync::Arc::clone(&capture)),
        };
        let (next, result, body, _) = exchange(owner, admitted, command, &[], view()).await?;
        admitted = next;
        assert_eq!(
            capture
                .lock()
                .map_err(|_| "nativeHouseMOVEcapture")?
                .move_outcome,
            Some(ItemMoveOutcome::Moved),
            "framedcmd9 must reach actualHouse placement/take owner"
        );
        assert_eq!(result, UseDisposition::Committed);
        let in_house:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_item_house_interior_locations WHERE item_instance_id=encode($1,'hex')::uuid)").bind(source.as_slice()).fetch_one(&h.pool).await?;
        let in_bag:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_item_container_entries WHERE item_instance_id=encode($1,'hex')::uuid AND character_id=encode($2,'hex')::uuid)").bind(source.as_slice()).bind(id(41).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(
            (in_house, in_bag),
            if ordinal == 0 {
                (true, false)
            } else {
                (false, true)
            }
        );
        let now:Value=sqlx::query_scalar("SELECT to_jsonb(i) FROM game_item_instances i WHERE item_instance_id=encode($1,'hex')::uuid").bind(source.as_slice()).fetch_one(&h.pool).await?;
        for field in [
            "item_instance_id",
            "definition_production_key",
            "definition_revision_ref",
            "minted_transaction_id",
            "quantity",
            "lifecycle",
        ] {
            assert_eq!(now[field], original[field], "MOVE9 preserves {field}");
        }
        assert!(
            !capture
                .lock()
                .map_err(|_| "nativeHousemapcapture")?
                .map_deltas
                .is_empty(),
            "actualHousemap reflects committedfloor custody"
        );
        let toy_deadline = if let Some(id) = toy {
            Some(super::santa_toy_lifetime::after_first_move(h, source, id).await?)
        } else {
            None
        };
        let before_replay = now;
        capture
            .lock()
            .map_err(|_| "nativeHousemapcapture")?
            .map_deltas
            .clear();
        // Place exercises cold durable replay. Take exercises the real retained
        // ingress: deleting all state at command2 is not a resumable handshake.
        if ordinal == 0 {
            owner
                .states
                .command_ingress
                .forget(admitted.game_session_id);
            admitted.continuity.next_command_id = command;
        }
        // Retained Take replay reconnects at the current high-water (3), then
        // resends command2; lowering the handshake cursor would lose its proof.
        let (next, replayed, _, _) = exchange(
            owner,
            admitted,
            command,
            &body,
            HouseExchange {
                current: None,
                ..view()
            },
        )
        .await?;
        admitted = next;
        assert_eq!(replayed, UseDisposition::Committed);
        assert_eq!(sqlx::query_scalar::<_,Value>("SELECT to_jsonb(i) FROM game_item_instances i WHERE item_instance_id=encode($1,'hex')::uuid").bind(source.as_slice()).fetch_one(&h.pool).await?,before_replay,"replay performs no second movement");
        assert!(
            capture
                .lock()
                .map_err(|_| "nativeHousereplaycapture")?
                .map_deltas
                .is_empty()
        );
        if let Some(deadline) = toy_deadline {
            super::santa_toy_lifetime::assert_replay_deadline(h, source, deadline).await?;
            super::santa_toy_lifetime::expire_on_house_owner(
                h,
                owner,
                admitted,
                source,
                toy.ok_or("toy source")?,
            )
            .await?;
            return Ok(());
        }
    }
    assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_item_transfer_receipts WHERE source_item_instance_id=encode($1,'hex')::uuid AND shape IN(5,6)").bind(source.as_slice()).fetch_one(&h.pool).await?,2);
    eprintln!("SRV7_NATIVE_HOUSE_MOVE9_EXECUTED sameUUID Place+Take+replay");
    Ok(())
}

/// Pinned source Container::queryAdd rejects a nonpickupable House object.
pub(super) async fn refuse_nonpickupable_take(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    source: [u8; 16],
    reference: std::num::NonZeroU32,
) -> TestResult {
    let policy = owner
        .active_generation
        .ok_or("native pickup source generation")?
        .native_gameplay()
        .ok_or("native pickup source data")?
        .item_policy("oteryn:item.tibia.i2976", "definition-r1")
        .ok_or("native birdcage source policy")?;
    assert!(
        matches!(&policy.record().semantics.physical,crate::content::ReferenceItemField::Known(physical) if physical.pickupable==crate::content::ReferenceItemField::Known(false))
    );
    let owner_qualified:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_item_house_interior_locations location JOIN game_item_house_reclaim_provenance provenance USING(item_instance_id,world_id,house_key) CROSS JOIN LATERAL game_house_access(location.world_id,location.house_key,encode($2,'hex')::uuid) access WHERE location.item_instance_id=encode($1,'hex')::uuid AND access.role='OWNER' AND provenance.reclaim_subject_character_id=encode($2,'hex')::uuid)")
        .bind(source.as_slice()).bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    assert!(
        owner_qualified,
        "nonpickupability probe must first prove actual OWNER access and its own historical reclaim provenance"
    );
    let before = nonpickupable_state(h, source).await?;
    let capture = std::sync::Arc::new(std::sync::Mutex::new(super::wire::HouseWireCapture {
        move_destination: Some(ItemMoveDestination::MainBackpack),
        ..Default::default()
    }));
    let view = || HouseExchange {
        rotate: false,
        current: Some(reference),
        after: None,
        nested: None,
        foreign_after_snapshot: None,
        capture: Some(std::sync::Arc::clone(&capture)),
    };
    let command = admitted.continuity.next_command_id;
    let (next, outcome, body, _) = exchange(owner, admitted, command, &[], view()).await?;
    assert_eq!(
        capture
            .lock()
            .map_err(|_| "native nonpickupable Take capture")?
            .move_outcome,
        Some(ItemMoveOutcome::Rejected),
        "actual framed9 must report its existing current-native destination refusal without mutating or disconnecting"
    );
    assert_eq!(outcome, UseDisposition::Rejected);
    assert_eq!(nonpickupable_state(h, source).await?, before);
    assert!(
        capture
            .lock()
            .map_err(|_| "native nonpickupable Take map capture")?
            .map_deltas
            .is_empty()
    );
    capture
        .lock()
        .map_err(|_| "native nonpickupable replay capture")?
        .move_outcome = None;
    let (_, replayed, _, _) = exchange(
        owner,
        next,
        command,
        &body,
        HouseExchange {
            current: None,
            ..view()
        },
    )
    .await?;
    assert_eq!(replayed, UseDisposition::Rejected);
    assert_eq!(
        capture
            .lock()
            .map_err(|_| "native nonpickupable replay capture")?
            .move_outcome,
        Some(ItemMoveOutcome::Rejected)
    );
    assert_eq!(nonpickupable_state(h, source).await?, before);
    assert!(
        capture
            .lock()
            .map_err(|_| "native nonpickupable replay map capture")?
            .map_deltas
            .is_empty()
    );
    Ok(())
}
async fn nonpickupable_state(h: &Harness, source: [u8; 16]) -> TestResult<Value> {
    Ok(sqlx::query_scalar("SELECT jsonb_build_object('item',to_jsonb(i),'house',to_jsonb(h),'provenance',to_jsonb(p)) FROM game_item_instances i JOIN game_item_house_interior_locations h USING(item_instance_id,world_id) JOIN game_item_house_reclaim_provenance p USING(item_instance_id,world_id,house_key) WHERE i.item_instance_id=encode($1,'hex')::uuid").bind(source.as_slice()).fetch_one(&h.pool).await?)
}
