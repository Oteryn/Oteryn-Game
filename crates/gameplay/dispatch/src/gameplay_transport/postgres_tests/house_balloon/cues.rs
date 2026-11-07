use super::*;

pub(super) async fn exercise(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    mut admitted: AdmittedSession,
    reference: std::num::NonZeroU32,
    source: &str,
) -> TestResult {
    let expected = match source {
        "oteryn:item.tibia.i23691" => 175,
        "oteryn:item.tibia.i2963" => 19,
        _ => return Err("House cue source fixture".into()),
    };
    let before = custody(h).await?;
    let viewer = {
        let runtime = owner.runtime.lock().await;
        let state = runtime
            .house_actor_state(
                admitted.runtime_actor.ok_or("House cue actor")?,
                admitted.game_session_id,
            )
            .map_err(debug)?;
        serde_json::json!({"house":state.house,"ownership_generation":state.generation.get(),
          "position":[state.position.x,state.position.y,state.position.floor]})
    };
    let captured = std::sync::Arc::new(std::sync::Mutex::new(
        super::wire::HouseWireCapture::default(),
    ));
    let body = encode_item_use_intent(&UseIntent {
        target: UseTarget::Item(std::num::NonZeroU64::MIN),
        use_with: None,
        use_with_map_item: None,
        use_with_position: None,
    })
    .map_err(debug)?;
    let view = || HouseExchange {
        rotate: false,
        current: Some(reference),
        after: None,
        nested: None,
        foreign_after_snapshot: None,
        capture: Some(std::sync::Arc::clone(&captured)),
    };
    let (next, result, actual, _) = exchange(owner, admitted, 1, &body, view()).await?;
    admitted = next;
    assert_eq!(
        result,
        UseDisposition::Committed,
        "source House decoration USE must commit and publish its cue"
    );
    let payloads = captured.lock().map_err(|_| "cue capture")?.effects.clone();
    let maps = captured.lock().map_err(|_| "map capture")?.maps.clone();
    assert!(
        !maps.is_empty(),
        "actual House viewer map snapshot captured"
    );
    assert_eq!(payloads.len(), 1);
    let events = oteryn_protocol_oteryn::presentation::decode_item_effect_batch(&payloads[0])
        .map_err(debug)?;
    assert_eq!(events.len(), 1);
    assert!(
        matches!(events[0].kind,oteryn_protocol_oteryn::presentation::PresentationEventKind::MagicEffect {effect_id,..} if effect_id==expected)
    );
    assert_eq!(
        custody(h).await?,
        before,
        "visual USE preserves House placement and reclaim identity"
    );
    captured.lock().map_err(|_| "cue capture")?.effects.clear();
    // A fresh composed command directory cannot replay a visual or draw a new outcome.
    owner
        .states
        .command_ingress
        .forget(admitted.game_session_id);
    admitted.continuity.next_command_id = 1;
    let (_, result, _, _) = exchange(
        owner,
        admitted,
        1,
        &actual,
        HouseExchange {
            current: None,
            ..view()
        },
    )
    .await?;
    assert_eq!(result, UseDisposition::Committed);
    assert!(
        captured
            .lock()
            .map_err(|_| "cue capture")?
            .effects
            .is_empty()
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM game_item_consume_transform_records WHERE source_kind=5"
        )
        .fetch_one(&h.pool)
        .await?,
        1
    );
    let payload_hex = payloads[0]
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    let name = if expected == 175 { "fish" } else { "harp" };
    let capture = serde_json::json!({"source":{"crystal_sha":"201189f871d23597b82caf88fd611454a2ddefb8",
        "producer_test":if expected==175 {"srv7_house_fish_tank_committed_cue_and_replay"}else{"srv7_house_harp_committed_cue_and_replay"}},
        "viewer":viewer,"world_map_snapshot_hex":maps[0].iter().map(|b|format!("{b:02x}")).collect::<String>(),
        "effects":[{"name":name,"expected_effect_id":expected,"payload_hex":payload_hex}]});
    println!("SRV7_HOUSE_EFFECT_CAPTURE {capture}");
    Ok(())
}

async fn custody(h: &Harness) -> TestResult<Value> {
    Ok(sqlx::query_scalar("SELECT jsonb_build_object('house',to_jsonb(h),'provenance',to_jsonb(p)) FROM game_item_house_interior_locations h JOIN game_item_house_reclaim_provenance p USING(item_instance_id,world_id,house_key) WHERE h.item_instance_id=encode($1,'hex')::uuid")
        .bind(id(71).as_slice()).fetch_one(&h.pool).await?)
}
