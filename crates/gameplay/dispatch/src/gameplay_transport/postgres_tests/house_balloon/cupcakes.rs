use super::*;

pub(super) async fn exercise(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    reference: std::num::NonZeroU32,
    item_id: u32,
) -> TestResult {
    let actor = admitted.runtime_actor.ok_or("House Cupcake actor")?;
    let revision = {
        let mut states = owner.states.spell_states.lock().await;
        let (_, _, state) = states
            .actors
            .iter_mut()
            .find(|(a, s, _)| *a == actor && *s == admitted.game_session_id)
            .ok_or("House Cupcake state")?;
        state.restore_saved_vitals(7, 9);
        state.revision()
    };
    let body = encode_item_use_intent(&UseIntent {
        target: UseTarget::Item(std::num::NonZeroU64::MIN),
        use_with: None,
        use_with_map_item: None,
        use_with_position: None,
    })
    .map_err(debug)?;
    let wire = || HouseExchange {
        rotate: false,
        current: Some(reference),
        after: None,
        nested: None,
        capture: None,
        foreign_after_snapshot: None,
    };
    let (next, result, actual, _) = exchange(owner, admitted, 1, &body, wire()).await?;
    assert_eq!(
        result,
        UseDisposition::Committed,
        "real placed House Cupcake must burn one source unit and apply the native successor"
    );
    let after = || {
        sqlx::query_as::<_,(i64,i16)>("SELECT quantity,lifecycle FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid").bind(id(71).to_vec())
    };
    assert_eq!(after().fetch_one(&h.pool).await?, (1, 1));
    let (next, replay, _, _) = exchange(
        owner,
        next,
        1,
        &actual,
        HouseExchange {
            current: None,
            ..wire()
        },
    )
    .await?;
    assert_eq!(replay, UseDisposition::Committed);
    let (_, cooldown, _, _) = exchange(owner, next, 2, &body, wire()).await?;
    assert_eq!(
        cooldown,
        UseDisposition::Exhausted,
        "same per-character source cooldown applies in placed House custody"
    );
    assert_eq!(after().fetch_one(&h.pool).await?, (1, 1));
    let states = owner.states.spell_states.lock().await;
    let (_, _, state) = states
        .actors
        .iter()
        .find(|(a, s, _)| *a == actor && *s == admitted.game_session_id)
        .ok_or("House Cupcake successor")?;
    assert_eq!(state.revision(), revision + 1);
    match item_id {
        28484 => assert_eq!(state.vitals().mana, state.character_facts().max_mana),
        28485 => assert_eq!(state.health, state.character_facts().max_health),
        28486 => assert_eq!(
            state
                .owned_skill_adjustments(owner.owner_now().get())
                .map_err(debug)?
                .adjust_skill(10, 4),
            Some(20)
        ),
        _ => return Err("House Cupcake source ID".into()),
    };
    drop(states);
    let proven:(i64,i32)=sqlx::query_as("SELECT count(*)::bigint,min(t.source_kind)::integer FROM game_seasonal_cupcake_uses c JOIN game_item_consume_transform_records t ON t.transaction_id=c.transform_transaction_id JOIN game_character_condition_snapshots s ON s.character_id=c.character_id WHERE c.source_kind=2 AND c.item_id=$1 AND c.conditions_sha256=sha256(convert_to(s.snapshot::text,'UTF8'))").bind(i64::from(item_id)).fetch_one(&h.pool).await?;
    assert_eq!(proven, (1, 5));
    let custody:i64=sqlx::query_scalar("SELECT count(*) FROM game_item_house_interior_locations WHERE item_instance_id=encode($1,'hex')::uuid").bind(id(71).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        custody, 1,
        "the remaining source unit retains actual House custody"
    );
    eprintln!("SRV7 POSTGRES_CASE_COMPLETED House Cupcake source={item_id}");
    Ok(())
}
