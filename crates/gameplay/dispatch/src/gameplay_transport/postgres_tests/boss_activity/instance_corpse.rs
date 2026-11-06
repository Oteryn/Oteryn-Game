//! Generic physical corpse custody from a real Instance lethal occurrence.
//! The item definition is isolated test data, not source quest qualification.
use super::*;
use crate::durability::item_mint::{
    CORPSE_MATERIALIZATION_PURPOSE_KEY, GroundPlacement, ItemMintCause, ItemMintOutcome,
    ItemMintRequest, TypedDefinitionRef,
};
use crate::gameplay_transport::kill_reward::TakeOutcome;

pub(super) async fn qualify(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
) -> TestResult {
    let started = std::time::Instant::now();
    let fence = admitted.item_fence.ok_or("Instance item fence")?;
    let taken = owner
        .states
        .attack
        .lock()
        .await
        .kills()
        .take(admitted.game_session_id, fence.character_lease_generation);
    let TakeOutcome::Taken(taken) = taken else {
        return Err("native death must be queued".into());
    };
    let entry = taken.entry().ok_or("retained native death")?;
    let server: String = sqlx::query_scalar("SHOW server_version_num")
        .fetch_one(&h.pool)
        .await?;
    assert!(server.starts_with("18"));
    eprintln!("INSTANCE_CORPSE_MINT POSTGRES_CASE_OBSERVED server_version_num={server}");
    let mut position = Vec::with_capacity(10);
    position.extend_from_slice(&entry.facts.corpse.x.to_be_bytes());
    position.extend_from_slice(&entry.facts.corpse.y.to_be_bytes());
    position.extend_from_slice(&entry.facts.corpse.floor.to_be_bytes());
    let item = TypedDefinitionRef {
        family: "ItemType".into(),
        production_key: "fixture:instance_corpse".into(),
        revision_ref: "r1".into(),
    };
    let request = ItemMintRequest {
        cause: ItemMintCause::from_creature_death(
            entry.facts.death,
            item.clone(),
            CORPSE_MATERIALIZATION_PURPOSE_KEY.into(),
            0,
        )
        .map_err(debug)?,
        item,
        quantity: 1,
        ground: GroundPlacement {
            spatial_position: position,
            corpse_ref: b"fixture:instance-corpse".to_vec(),
            map_revision: entry.ground.map_revision.clone(),
            content_revision: entry.ground.content_revision.clone(),
            native_room_placement_context: entry.ground.native_room_placement_context.clone(),
        },
        content_revision: entry.ground.content_revision.clone(),
        ruleset_revision: entry.ground.ruleset_revision.clone(),
        sim_revision: entry.ground.sim_revision.clone(),
    };
    let mut candidate = owner
        .root
        .freeze_item_mint(owner.character, owner.holder, request)
        .await
        .map_err(debug)?;
    let instance = fence.runtime_scope.instance_id().ok_or("Instance scope")?;
    let channel_twin = RuntimeScopeRefV1::channel(
        fence.runtime_scope.world_id(),
        crate::foundation::ChannelId::decode(&instance).map_err(debug)?,
    );
    let foreign_instance =
        RuntimeScopeRefV1::instance(fence.runtime_scope.world_id(), fixture::id(231))
            .map_err(debug)?;
    for scope in [channel_twin, foreign_instance] {
        let wrong =
            crate::durability::spell_owner_commit::SpellLanePermit::of_fresh_scope(scope).await?;
        assert!(matches!(
            owner
                .root
                .commit_corpse_mint(
                    &wrong,
                    owner.character,
                    owner.holder,
                    &mut candidate,
                    *fence.character_id.as_bytes(),
                )
                .await,
            Err(crate::durability::item_mint::ItemMintError::Unavailable(
                crate::durability::DurabilityError::InvalidStoredState
            ))
        ));
    }
    let charged: i16 = sqlx::query_scalar("SELECT work_units_used FROM game_item_mint_reservations WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(candidate.item_instance_id().as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(charged, 0, "foreign permits cannot charge the logical MINT");
    let permit = owner.spell_lane_permit().await.ok_or("Instance lane")?;
    let first = owner
        .root
        .commit_corpse_mint(
            &permit,
            owner.character,
            owner.holder,
            &mut candidate,
            *fence.character_id.as_bytes(),
        )
        .await
        .map_err(debug)?;
    assert!(matches!(first, ItemMintOutcome::Committed(_)));
    let replay = owner
        .root
        .commit_corpse_mint(
            &permit,
            owner.character,
            owner.holder,
            &mut candidate,
            *fence.character_id.as_bytes(),
        )
        .await
        .map_err(debug)?;
    assert!(matches!(replay, ItemMintOutcome::AlreadyCommitted(_)));
    let exact: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM game_item_ground_locations g JOIN game_item_mint_receipts r USING(item_instance_id) JOIN game_item_audit_outbox a USING(item_instance_id) WHERE g.item_instance_id=encode($1,'hex')::uuid AND g.scope_kind=2 AND g.channel_id IS NULL AND g.instance_id=encode($2,'hex')::uuid AND r.materialized_at IS NOT NULL AND r.corpse_top_damage_character_id=encode($3,'hex')::uuid AND a.transaction_id=r.transaction_id AND a.envelope_sha256=r.envelope_sha256)",
    ).bind(candidate.item_instance_id().as_slice())
        .bind(fence.runtime_scope.instance_id().ok_or("Instance scope")?.as_slice())
        .bind(fence.character_id.as_bytes().as_slice())
        .fetch_one(&h.pool).await?;
    assert!(
        exact,
        "corpse, custody, attribution and audit commit together"
    );
    let envelope: Vec<u8> = sqlx::query_scalar(
        "SELECT envelope FROM game_item_audit_outbox WHERE item_instance_id=encode($1,'hex')::uuid",
    )
    .bind(candidate.item_instance_id().as_slice())
    .fetch_one(&h.pool)
    .await?;
    let (event, payload) =
        crate::durability::item_mint_audit::decode_envelope(&envelope).map_err(debug)?;
    assert_eq!(event.channel_id, None);
    assert_eq!(event.instance_id.as_deref(), Some(instance.as_slice()));
    assert_eq!(
        payload
            .source
            .ok_or("mint source")?
            .death_occurrence
            .ok_or("death occurrence")?
            .runtime_scope()
            .map_err(debug)?,
        fence.runtime_scope
    );
    drop(permit);
    taken.requeue();
    eprintln!(
        "INSTANCE_CORPSE_MINT POSTGRES_CASE_COMPLETED elapsed_ms={}",
        started.elapsed().as_millis()
    );
    Ok(())
}
