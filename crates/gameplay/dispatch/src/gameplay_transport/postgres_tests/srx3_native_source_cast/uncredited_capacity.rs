use super::*;
use crate::durability::item_mint::{
    GroundPlacement, ItemMintCause, ItemMintOutcome, ItemMintRequest, TypedDefinitionRef,
};
use crate::gameplay_transport::ComposedFreshAdmission;

pub(super) async fn fill_source_tile(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    actor: crate::foundation::ExactActorRef,
) -> TestResult<GroundPlacement> {
    let begin = {
        let mut runtime = owner.runtime.lock().await;
        let notice = runtime
            .next_source_creature_death()
            .map_err(debug)?
            .ok_or("source death")?;
        assert_eq!(notice.actor(), actor);
        runtime
            .begin_source_creature_death(&notice)
            .map_err(debug)?
    };
    let digest = |bytes: [u8; 32]| {
        format!(
            "sha256:{}",
            bytes.iter().map(|b| format!("{b:02x}")).collect::<String>()
        )
    };
    let position = begin.position();
    let mut spatial_position = Vec::with_capacity(10);
    spatial_position.extend_from_slice(&position.x.to_be_bytes());
    spatial_position.extend_from_slice(&position.y.to_be_bytes());
    spatial_position.extend_from_slice(&position.floor.to_be_bytes());
    let ground = GroundPlacement {
        spatial_position,
        corpse_ref: [
            crate::durability::item_loop_move::ITEM_LOOP_DROP_REF_PREFIX,
            id(3).as_slice(),
        ]
        .concat(),
        map_revision: digest(begin.map_digest()),
        content_revision: digest(begin.content_digest()),
        native_room_placement_context: begin.frame_digest().to_vec(),
    };
    let permit = owner.spell_lane_permit().await.ok_or("Ground MINT lane")?;
    // Genuine ordinary MINTs occupy the exact native tile. Neither synthetic
    // corpse rows nor a replaced capacity limit can satisfy this regression.
    for ordinal in 0..64 {
        let mut candidate = owner
            .root
            .freeze_item_mint(
                owner.character,
                owner.holder,
                ItemMintRequest {
                    cause: ItemMintCause::for_test(
                        owner.world_id,
                        owner.channel_id,
                        begin.death_key().scope_ownership_generation(),
                        20_000 + ordinal,
                        1,
                        TypedDefinitionRef {
                            family: "LootTable".into(),
                            production_key: "fixture:loot.uncredited-full-ground".into(),
                            revision_ref: "definition-r1".into(),
                        },
                        "fixture:purpose.uncredited-full-ground".into(),
                        0,
                    ),
                    item: TypedDefinitionRef {
                        family: "Item".into(),
                        production_key: "oteryn:item.tibia.i3031".into(),
                        revision_ref: "definition-r1".into(),
                    },
                    quantity: 1,
                    ground: ground.clone(),
                    content_revision: ground.content_revision.clone(),
                    ruleset_revision: "ruleset-1".into(),
                    sim_revision: "sim-1".into(),
                },
            )
            .await
            .map_err(debug)?;
        assert!(matches!(
            owner
                .root
                .commit_item_mint(&permit, owner.character, owner.holder, &mut candidate)
                .await
                .map_err(debug)?,
            ItemMintOutcome::Committed(_)
        ));
    }
    assert_eq!(tile_items(h, &ground).await?, 64);
    owner.states.loop_ground.invalidate();
    Ok(ground)
}

async fn tile_items(h: &Harness, ground: &GroundPlacement) -> TestResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT count(*) FROM game_item_ground_locations g JOIN game_item_instances i USING(item_instance_id) WHERE g.spatial_position=$1 AND i.lifecycle=1",
    ).bind(&ground.spatial_position).fetch_one(&h.pool).await?)
}

async fn corpse_reservation(h: &Harness) -> TestResult<(String, String, String, i16)> {
    let reservations: Vec<(String, String, String, i16)> = sqlx::query_as(
        "SELECT transaction_id::text,event_id::text,item_instance_id::text,work_units_used FROM game_item_mint_reservations WHERE loot_purpose_key='CORPSE_MATERIALIZATION'",
    ).fetch_all(&h.pool).await?;
    assert_eq!(
        reservations.len(),
        1,
        "one frozen cause, including its original identities and budget"
    );
    reservations
        .into_iter()
        .next()
        .ok_or_else(|| "corpse reservation".into())
}

pub(super) async fn assert_terminal_absence(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    actor: crate::foundation::ExactActorRef,
    ground: &GroundPlacement,
) -> TestResult {
    let reserved = corpse_reservation(h).await?;
    assert_eq!(
        reserved.3, 2,
        "one capacity refusal plus its successful cause-locked reconciliation"
    );
    assert_eq!(sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_item_mint_receipts WHERE loot_purpose_key='CORPSE_MATERIALIZATION'").fetch_one(&h.pool).await?, 0, "the full tile did not produce a corpse MINT");
    {
        let runtime = owner.runtime.lock().await;
        assert!(
            !runtime.corpse_projection_pinned(actor),
            "certain absence releases the source pin"
        );
        assert!(
            runtime.visible_corpses().is_empty(),
            "no invented Item binding"
        );
    }
    for _ in 0..3 {
        owner.drain_monster_melee_at(owner.owner_now()).await;
    }
    assert_eq!(
        corpse_reservation(h).await?,
        reserved,
        "terminal replay neither re-freezes nor charges another unit"
    );
    assert_eq!(
        tile_items(h, ground).await?,
        64,
        "all real Ground roots survive the refusal"
    );
    assert!(
        owner
            .states
            .spell_states
            .lock()
            .await
            .source_death_corpse
            .is_none()
    );
    Ok(())
}
