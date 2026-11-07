//! Continues actual charged House expiry with a second paid cycle, not a synthetic clock.
use super::*;
pub(super) async fn exercise(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    source: [u8; 16],
    original: Value,
) -> TestResult {
    let native = owner
        .active_generation
        .and_then(|generation| generation.native_gameplay())
        .ok_or("native history content")?;
    let index = native.item_index().ok_or("history Item index")?;
    let reference = |id: u32| {
        index
            .definition_ref("Item", &format!("oteryn:item.tibia.i{id}"), "definition-r1")
            .ok_or("history definition")
    };
    let body = encode_item_use_intent(&UseIntent {
        target: UseTarget::Item(std::num::NonZeroU64::MIN),
        use_with: None,
        use_with_map_item: None,
        use_with_position: None,
    })
    .map_err(debug)?;
    let (admitted, charged, _) = super::super::rust_lottery::exchange(
        owner,
        admitted,
        3,
        &body,
        Some(reference(677)?),
        Some(reference(9040)?),
    )
    .await?;
    assert_eq!(
        charged,
        UseDisposition::Committed,
        "completed absolute history must permit a fresh paid inactive cycle"
    );
    let paid:Value=sqlx::query_scalar("SELECT to_jsonb(t) FROM game_item_timed_states t WHERE item_instance_id=encode($1,'hex')::uuid").bind(source.as_slice()).fetch_one(&h.pool).await?;
    assert!(paid["deadline_at"].as_i64().is_some());
    let retained:Value=sqlx::query_scalar("SELECT to_jsonb(p) FROM game_spell_item_temporal_schedules p WHERE item_instance_id=encode($1,'hex')::uuid").bind(source.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        retained["source_transaction_id"], original["source_transaction_id"],
        "paid cycle preserves historical source"
    );
    let mut exit = vec![8, 2, 18, HOUSE.len() as u8];
    exit.extend(HOUSE.as_bytes());
    let capture = std::sync::Arc::new(std::sync::Mutex::new(super::wire::HouseWireCapture {
        move_destination: Some(
            oteryn_protocol_oteryn::item_view::ItemMoveDestination::MainBackpack,
        ),
        exit_after_death: Some(exit),
        exit_after_commit: true,
        ..Default::default()
    }));
    let (channel, moved, _, _) = exchange(
        owner,
        admitted,
        4,
        &[],
        HouseExchange {
            rotate: false,
            current: Some(reference(9039)?),
            after: None,
            nested: None,
            foreign_after_snapshot: None,
            capture: Some(std::sync::Arc::clone(&capture)),
        },
    )
    .await?;
    assert_eq!(moved, UseDisposition::Committed);
    assert_eq!(
        capture
            .lock()
            .map_err(|_| "history exit capture")?
            .exit_status,
        Some(CommandStatus::Accepted)
    );
    let fence = channel.item_fence.ok_or("history Channel fence")?;
    assert!(matches!(
        fence.runtime_scope,
        crate::foundation::RuntimeScopeRefV1::Channel { .. }
    ));
    let permit = owner
        .spell_lane_permit()
        .await
        .ok_or("history deadline lane")?;
    assert_eq!(
        owner
            .root
            .drain_spell_item_deadlines(
                &permit,
                owner.character,
                owner.holder,
                fence.runtime_scope,
                fence.scope_ownership_generation.get()
            )
            .await
            .map_err(debug)?,
        0,
        "a House-completed historical absolute clock must never run again in the next paid cycle"
    );
    assert_eq!(sqlx::query_scalar::<_,Value>("SELECT to_jsonb(t) FROM game_item_timed_states t WHERE item_instance_id=encode($1,'hex')::uuid").bind(source.as_slice()).fetch_one(&h.pool).await?,paid);
    assert_eq!(sqlx::query_scalar::<_,String>("SELECT definition_production_key FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid").bind(source.as_slice()).fetch_one(&h.pool).await?,"oteryn:item.tibia.i9039");
    Ok(())
}
