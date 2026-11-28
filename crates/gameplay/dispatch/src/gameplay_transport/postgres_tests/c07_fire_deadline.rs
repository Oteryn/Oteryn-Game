use super::death_ml_loss_composed_fixtures::production_generation_with_pin;
use super::fixture::{Harness, debug, id, join_two, run};
use super::item_look_fixtures::large_stack;
use super::{Dependencies, TestResult, prepare_current_session};
use crate::durability::DurabilityRoot;
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::durability::native_world_items::{
    QualifiedItemDefinition, SpellGroundTarget, qualify_source_field_chain,
};
use crate::durability::spell_item_transaction::{
    apply_spell_items_in_transaction, assert_spell_item_authority_in_transaction,
};
use crate::durability::spell_items_abi::{
    CastCostBinding, SourceCasterOrigin, SpellItemOperation, SpellItemTransactionOutcome,
    SpellItemTransactionRequest,
};
use crate::foundation::{CommandId, CommandRef, MovementLocalPosition, WorldId};
use crate::gameplay_transport::source_item_cycle::SOURCE_OWNER_SWEEP_COMPLETE;
use crate::gameplay_transport::world_required_capabilities::BoundRequiredCapabilities;
use std::{sync::Arc, time::Duration};

#[test]
fn c07_idle_ground_fire_retires_with_one_receipt_per_stage_postgres() -> TestResult {
    large_stack(|| run(scenario))
}

async fn scenario(admin: String) -> TestResult {
    let h = Harness::create_with_admission_seed(admin, "c07_idle_ground_fire", true, false).await?;
    prepare_current_session(&h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let recovery = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let world = WorldId::decode(&id(42)).map_err(debug)?;
    let (controller, pin) = production_generation_with_pin(world, "content-1")?;
    let active = controller.active().ok_or("active native generation")?;
    let native = active.native_gameplay().ok_or("native gameplay")?;
    let room = pin.qualified_room().clone();
    let entry = pin.entry_start();
    let deps =
        Dependencies::with_content(pin.into_channel_pin(), room, native.spell_book().clone())?;
    let mut owner = deps.owner(&h, &recovery);
    owner.active_generation = Some(active);
    owner.required_capabilities = BoundRequiredCapabilities::for_channel_boot(
        &[11, 30],
        crate::foundation::RuntimeScopeRefV1::channel(world, deps.admitted.channel_id),
        1,
        "policy-1",
        crate::gameplay_transport::capabilities::BUNDLE_WORLD_OFFERED_CAPABILITIES,
    )
    .map_err(debug)?;

    let policy = native
        .item_policy_for_source_id(2118)
        .ok_or("regular Fire")?;
    let definition =
        qualify_source_field_chain(native, QualifiedItemDefinition::from_native_policy(policy)?)
            .map_err(debug)?;
    let stage_count = i64::try_from(definition.decay_chain.len())?;
    assert!(
        stage_count > 1,
        "regular Fire must transform before retiring"
    );
    let blocks_movement = policy
        .record()
        .attributes
        .blocks_movement
        .ok_or("Fire movement flag")?;
    let blocks_projectile = policy
        .record()
        .attributes
        .blocks_projectile
        .ok_or("Fire projectile flag")?;
    let session = deps.admitted.game_session_id;
    let (current, _) = FreshAdmissionStore::from_root(h.root.clone())
        .current_session_at(session)
        .await?;
    let fence = crate::gameplay_transport::item_fence_of(current).ok_or("current Item fence")?;
    let command = CommandRef::new(session, CommandId::new(1).map_err(debug)?);
    let permit = owner.spell_lane_permit().await.ok_or("spell lane")?;
    let runtime = owner.runtime.lock().await;
    let target = SpellGroundTarget::from_native_owner(
        &deps.room,
        &runtime,
        MovementLocalPosition {
            x: entry.x,
            y: entry.y,
            floor: entry.floor,
        },
    )
    .map_err(debug)?;
    drop(runtime);
    let rune = native.spell_book().rune(3188).ok_or("Fire Field rune")?;
    let spell = &rune
        .authored
        .as_ref()
        .ok_or("Fire Field source")?
        .header
        .identity;
    let item = id(180);
    // The fixture supplies the paid-cast context. The production writer mints
    // the exact admitted Fire profile and all of its absolute deadlines.
    let request = SpellItemTransactionRequest {
        swing_ordinal: None,
        command,
        spell: crate::durability::item_mint::TypedDefinitionRef {
            family: "Spell".into(),
            production_key: spell.key.clone(),
            revision_ref: spell.revision.clone(),
        },
        catalog_digest: native.source_digest(),
        transaction_id: id(181),
        event_id: id(182),
        cost: CastCostBinding {
            vitals_revision_before: 1,
            vitals_revision_after: 2,
            mana_before: 200,
            mana_after: 180,
            soul_before: 100,
            soul_after: 100,
            cooldowns_before: Vec::new(),
            cooldowns_after: vec![(format!("spell:{}", rune.key), rune.cooldown_micros)],
            caster_digest_before: [11; 32],
            caster_digest_after: [12; 32],
        },
        caster_origin: Some(SourceCasterOrigin {
            actor: deps.actor,
            character_lease_generation: fence.character_lease_generation,
        }),
        operations: vec![SpellItemOperation::MintGround {
            item_instance_id: item,
            definition,
            quantity: 1,
            placement: target.placement(b"c07-idle-ground-fire".to_vec()),
            lifetime_millis: None,
            blocks_movement,
            blocks_projectile,
            description: None,
        }],
        companion: None,
        direct_companion: None,
    };
    let mut tx = h.pool.begin().await?;
    let authority = assert_spell_item_authority_in_transaction(
        &mut tx,
        &permit,
        owner.root,
        owner.character,
        owner.holder,
        &fence,
        command,
        native.source_digest(),
    )
    .await
    .map_err(debug)?;
    let origin = crate::gameplay_transport::world_required_field_guard::player_origin(
        &owner.required_capabilities,
    );
    let outcome = apply_spell_items_in_transaction(&mut tx, &authority, &request, &origin)
        .await
        .map_err(debug)?;
    assert!(matches!(outcome, SpellItemTransactionOutcome::Applied(_)));
    tx.commit().await?;
    drop(permit);
    assert_eq!(fire_state(&h, item).await?, (1, 1, 1, 0, 0));

    // Advance fixture time past the final deadline without erasing the authored
    // spacing. The background Channel cycle owns all resulting writes.
    let mut aging = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *aging)
        .await?;
    let aged = sqlx::query(
        "UPDATE game_spell_field_temporal_schedules
            SET expires_at_unix_ms=expires_at_unix_ms-(
                SELECT max(expires_at_unix_ms)
                       - floor(extract(epoch FROM statement_timestamp())*1000)::bigint + 1
                FROM game_spell_field_temporal_schedules
                WHERE item_instance_id=encode($1,'hex')::uuid)
          WHERE item_instance_id=encode($1,'hex')::uuid",
    )
    .bind(item.as_slice())
    .execute(&mut *aging)
    .await?;
    assert_eq!(aged.rows_affected(), u64::try_from(stage_count)?);
    aging.commit().await?;

    let wake = owner.states.spell_states.lock().await.owner_wake.clone();
    let swept = Arc::new(tokio::sync::Notify::new());
    let stop = oteryn_foundation::CancellationToken::new();
    let state = tokio::time::timeout(Duration::from_secs(20), async {
        let cycle = DurabilityRoot::with_background_holder(
            SOURCE_OWNER_SWEEP_COMPLETE
                .scope(Arc::clone(&swept), owner.run_source_owner_cycles(&stop)),
        );
        let observe = async {
            // Give a refused background holder checkout a bounded chance to retry.
            let result = async {
                let mut state = fire_state(&h, item).await?;
                for _ in 0..8 {
                    swept.notified().await;
                    state = fire_state(&h, item).await?;
                    if state.3 == stage_count {
                        break;
                    }
                }
                Ok::<_, Box<dyn std::error::Error>>(state)
            }
            .await;
            stop.cancel();
            result
        };
        wake.notify_one();
        join_two(cycle, observe).await.1
    })
    .await??;
    assert_eq!(state, (2, 0, 0, stage_count, stage_count));
    eprintln!("C07 idle Fire retired with {stage_count} audited deadline receipts");
    Ok(())
}

async fn fire_state(h: &Harness, item: [u8; 16]) -> TestResult<(i16, i64, i64, i64, i64)> {
    Ok(sqlx::query_as(
        "SELECT i.lifecycle,i.quantity,
              (SELECT count(*) FROM game_item_ground_locations g WHERE g.item_instance_id=i.item_instance_id),
              (SELECT count(*) FROM game_spell_field_temporal_receipts r WHERE r.item_instance_id=i.item_instance_id),
              (SELECT count(*) FROM game_spell_field_temporal_receipts r
               JOIN game_spell_field_temporal_audit a USING(transaction_id)
               WHERE r.item_instance_id=i.item_instance_id)
         FROM game_item_instances i WHERE i.item_instance_id=encode($1,'hex')::uuid",
    ).bind(item.as_slice()).fetch_one(&h.pool).await?)
}
