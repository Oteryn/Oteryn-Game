use super::*;

#[allow(
    clippy::too_many_arguments,
    reason = "actual wire session, actor/fence state and source Item identity/definition are independent test proofs"
)]
pub(super) async fn exercise(
    h: &Harness,
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    mut admitted: crate::gameplay_transport::connection::AdmittedSession,
    actor: crate::foundation::ExactActorRef,
    fence: crate::durability::item_transfer::CurrentCharacterItemFence,
    source: [u8; 16],
    reference: std::num::NonZeroU32,
    item: u32,
) -> TestResult {
    admitted.runtime_actor = Some(actor);
    admitted.first_entry = FirstEntryOutcome::Positioned;
    admitted.item_fence = Some(fence);
    admitted.continuity.selected_capabilities =
        SelectedCapabilities::select(owner.offered_capabilities(), &[4, 6, 11, 12, 15, 18])
            .ok_or("Cupcake Ground offers")?;
    let revision = {
        let mut states = owner.states.spell_states.lock().await;
        let (_, _, state) = states
            .actors
            .iter_mut()
            .find(|(a, s, _)| *a == actor && *s == admitted.game_session_id)
            .ok_or("Cupcake Ground actor")?;
        state.restore_saved_vitals(7, 9);
        state.revision()
    };
    let intent = UseIntent {
        target: UseTarget::Item(std::num::NonZeroU64::MIN),
        use_with: None,
        use_with_map_item: None,
        use_with_position: None,
    };
    let payload = encode_item_use_intent(&intent).map_err(debug)?;
    let (next, result, actual) =
        super::super::rust_lottery::exchange(owner, admitted, 1, &payload, Some(reference), None)
            .await?;
    assert_eq!(
        result,
        UseDisposition::Committed,
        "real Ground Cupcake USE must consume and apply its native effect"
    );
    let after = || {
        sqlx::query_as::<_,(i64,i16)>("SELECT quantity,lifecycle FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid").bind(source.to_vec())
    };
    assert_eq!(after().fetch_one(&h.pool).await?, (1, 1));
    let (next, replay, _) =
        super::super::rust_lottery::exchange(owner, next, 1, &actual, None, None).await?;
    assert_eq!(replay, UseDisposition::Committed);
    let (_, cooldown, _) =
        super::super::rust_lottery::exchange(owner, next, 2, &payload, Some(reference), None)
            .await?;
    assert_eq!(
        cooldown,
        UseDisposition::Exhausted,
        "source600s cooldown survives a new native Ground command"
    );
    assert_eq!(after().fetch_one(&h.pool).await?, (1, 1));
    let states = owner.states.spell_states.lock().await;
    let (_, _, state) = states
        .actors
        .iter()
        .find(|(a, s, _)| *a == actor && *s == admitted.game_session_id)
        .ok_or("Cupcake Ground successor")?;
    assert_eq!(
        state.revision(),
        revision + 1,
        "replay/cooldown never reapplies the successor"
    );
    match item {
        28484 => assert_eq!(state.vitals().mana, state.character_facts().max_mana),
        28485 => assert_eq!(state.health, state.character_facts().max_health),
        28486 => assert_eq!(
            state
                .owned_skill_adjustments(owner.owner_now().get())
                .map_err(debug)?
                .adjust_skill(10, 4),
            Some(20)
        ),
        _ => return Err("Cupcake Ground source ID".into()),
    };
    drop(states);
    let rows:(i64,i32)=sqlx::query_as("SELECT count(*)::bigint,min(source_kind)::integer FROM game_item_consume_transform_records WHERE recipe LIKE 'oteryn:seasonal.cupcake.%'").fetch_one(&h.pool).await?;
    assert_eq!(rows, (1, 3));
    let proven:i64=sqlx::query_scalar("SELECT count(*) FROM game_seasonal_cupcake_uses c JOIN game_item_consume_transform_records t ON t.transaction_id=c.transform_transaction_id JOIN game_character_condition_snapshots s ON s.character_id=c.character_id WHERE c.source_kind=2 AND c.item_id=$1 AND c.conditions_sha256=sha256(convert_to(s.snapshot::text,'UTF8'))").bind(i64::from(item)).fetch_one(&h.pool).await?;
    assert_eq!(
        proven, 1,
        "source burn/cooldown/actual prepared snapshot commit together"
    );
    eprintln!("SRV7 POSTGRES_CASE_COMPLETED Ground Cupcake source={item}");
    Ok(())
}
