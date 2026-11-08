//! W18 actual native House item-owner regression cases, sharing the admitted fixture.
use super::*;

pub(super) async fn scenario(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    admitted: AdmittedSession,
    case: Case,
    body: &[u8],
    before: &Value,
) -> TestResult {
    let (source_id, after_id) = match case {
        Case::Decoration(source, after) => (source, if after == 0 { source } else { after }),
        Case::CarpetTile { .. } => (22736, 22737),
        Case::NativeNoop(source) => (source, source),
        _ => return Err("not a decoration case".into()),
    };
    let before_key = before["definition_production_key"]
        .as_str()
        .ok_or("source key")?;
    let after_key = format!("oteryn:item.tibia.i{after_id}");
    let after_key = after_key.as_str();
    let native = owner
        .active_generation
        .and_then(|generation| generation.native_gameplay())
        .ok_or("native decoration")?;
    use super::scenario::{custody, item, receipts};
    let context = {
        let runtime = owner.runtime.lock().await;
        crate::durability::consume_transform::HouseActionContext::from_native_owner(
            &runtime,
            admitted.runtime_actor.ok_or("House actor")?,
            admitted.game_session_id,
            admitted.item_fence.ok_or("House fence")?,
        )
        .map_err(debug)?
    };
    let rotation = matches!(case, Case::NativeNoop(_)) || matches!(source_id, 2358 | 34278);
    let context = if rotation {
        context.with_rotation(1)
    } else {
        context
    };
    let intent = if rotation {
        let mut bytes = oteryn_protocol_oteryn::house_rotation::COMMAND
            .to_be_bytes()
            .to_vec();
        bytes.extend(
            oteryn_protocol_oteryn::house_rotation::Request::Rotate(
                oteryn_protocol_oteryn::house_rotation::Target::Item {
                    item: id(71),
                    expected_revision: 1,
                },
            )
            .encode()
            .ok_or("rotation intent")?,
        );
        bytes
    } else {
        body.to_vec()
    };
    let request = crate::durability::consume_transform::ConsumeTransformRequest {
        command: crate::foundation::CommandRef::new(
            admitted.game_session_id,
            crate::foundation::CommandId::new(1).map_err(debug)?,
        ),
        source: id(71),
        target: None,
        intent,
    };
    let old_custody = custody(h).await?;
    let result = owner
        .root
        .consume_transform_in_house(owner.holder, context.clone(), request.clone(), native)
        .await?;
    if let Case::CarpetTile { other, removed } = case {
        if !removed {
            assert_eq!(result.disposition, UseDisposition::NotPossible);
            assert!(!result.fresh);
            assert_eq!(item(h).await?, *before);
            assert_eq!(custody(h).await?, old_custody);
            assert_eq!(receipts(h).await?, 0);
        } else {
            assert_eq!(result.disposition, UseDisposition::Committed);
            assert!(result.fresh);
            assert_eq!(result.effects["house_remove"], true);
            let retired = item(h).await?;
            assert_eq!(retired["quantity"], 0);
            assert_eq!(retired["lifecycle"], 2);
            assert_eq!(retired["state_revision"], 2);
            assert_eq!(
                retired["minted_transaction_id"],
                before["minted_transaction_id"]
            );
            assert!(
                custody(h)
                    .await?
                    .as_object()
                    .ok_or("retired custody")?
                    .values()
                    .all(Value::is_null)
            );
            assert_eq!(receipts(h).await?, 1);
            let replay = owner
                .root
                .consume_transform_in_house(owner.holder, context, request, native)
                .await?;
            assert_eq!(replay.disposition, UseDisposition::Committed);
            assert!(!replay.fresh);
            assert_eq!(item(h).await?, retired);
            assert_eq!(receipts(h).await?, 1);
        }
        let kept: Value = sqlx::query_scalar("SELECT to_jsonb(i) FROM game_item_instances i WHERE item_instance_id=encode($1,'hex')::uuid")
            .bind(id(72).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(
            kept["definition_production_key"],
            format!("oteryn:item.tibia.i{other}")
        );
        assert_eq!(kept["lifecycle"], 1);
        assert_eq!(kept["quantity"], 1);
        assert_eq!(kept["state_revision"], 1);
        eprintln!(
            "W18 POSTGRES_CASE_COMPLETED carpet_removed={removed} exact_tile_source_and_other_item_retained"
        );
        return Ok(());
    }
    assert_eq!(
        result.disposition,
        UseDisposition::Committed,
        "actual House USE owner must execute source decoration"
    );
    if matches!(case, Case::NativeNoop(_)) {
        assert!(!result.fresh);
        assert!(result.recipe.is_none());
        assert_eq!(
            item(h).await?,
            *before,
            "source engine returns the original item without a revision/lineage change"
        );
        assert_eq!(custody(h).await?, old_custody);
        assert_eq!(receipts(h).await?, 0);
        let again = owner
            .root
            .consume_transform_in_house(owner.holder, context, request, native)
            .await?;
        assert_eq!(again.disposition, UseDisposition::Committed);
        assert_eq!(item(h).await?, *before);
        assert_eq!(receipts(h).await?, 0);
        eprintln!(
            "W18 POSTGRES_CASE_COMPLETED source={source_id} original_item_noop_no_synthetic_identity"
        );
        return Ok(());
    }
    assert!(result.fresh);
    let expected_after = if source_id == 2976 {
        let selected = result.effects["house_after"]["key"]
            .as_str()
            .ok_or("bird result")?;
        assert!(matches!(
            selected,
            "oteryn:item.tibia.i2975" | "oteryn:item.tibia.i2976"
        ));
        assert_eq!(
            result.effects["cues"].as_array().ok_or("bird cues")?.len(),
            usize::from(selected.ends_with("2976"))
        );
        selected
    } else {
        after_key
    };
    assert_eq!(item(h).await?["definition_production_key"], expected_after);
    let expected_revision = if before_key == expected_after { 1 } else { 2 };
    assert_eq!(item(h).await?["state_revision"], expected_revision);
    assert_eq!(custody(h).await?, old_custody);
    assert_eq!(result.effects["source_item_id"], source_id);
    let mut expected_receipts = 1;
    if matches!(source_id, 2959 | 23691) {
        let expected_effect = if source_id == 2959 { 24 } else { 175 };
        assert_eq!(result.effects["cues"][0]["effect_id"], expected_effect);
        assert_eq!(
            item(h).await?,
            *before,
            "presentation does not transform the item"
        );
    }
    if source_id == 2603 {
        assert!(
            result.effects["speech"]
                .as_str()
                .is_some_and(|text| !text.is_empty())
        );
        assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_house_decoration_cooldowns WHERE character_id=encode($1,'hex')::uuid")
                .bind(id(41).as_slice()).fetch_one(&h.pool).await?, 1);
        let cooldown_before: i64 = sqlx::query_scalar("SELECT until_ms FROM game_house_decoration_cooldowns WHERE character_id=encode($1,'hex')::uuid AND cooldown_key='wall-mirror'")
                .bind(id(41).as_slice()).fetch_one(&h.pool).await?;
        let mut second = request.clone();
        second.command = crate::foundation::CommandRef::new(
            admitted.game_session_id,
            crate::foundation::CommandId::new(2).map_err(debug)?,
        );
        let exhausted = owner
            .root
            .consume_transform_in_house(owner.holder, context.clone(), second, native)
            .await?;
        assert_eq!(exhausted.disposition, UseDisposition::Committed);
        assert!(exhausted.fresh);
        assert_eq!(
            exhausted.effects["speech"],
            "Don't be so vain about your appearance."
        );
        assert_eq!(sqlx::query_scalar::<_,i64>("SELECT until_ms FROM game_house_decoration_cooldowns WHERE character_id=encode($1,'hex')::uuid AND cooldown_key='wall-mirror'")
                .bind(id(41).as_slice()).fetch_one(&h.pool).await?, cooldown_before, "exhausted use never extends the source cooldown");
        assert_eq!(item(h).await?, *before);
        expected_receipts = 2;
    }
    let replay = owner
        .root
        .consume_transform_in_house(owner.holder, context, request, native)
        .await?;
    assert_eq!(replay.disposition, UseDisposition::Committed);
    assert!(!replay.fresh);
    assert_eq!(replay.effects, result.effects);
    assert_eq!(receipts(h).await?, expected_receipts);
    assert_eq!(item(h).await?["state_revision"], expected_revision);
    eprintln!(
        "W18 POSTGRES_CASE_COMPLETED source={source_id} native_owner_custody_cooldown_replay"
    );
    Ok(())
}
