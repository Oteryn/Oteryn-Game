//! Generic fixture: exact carried input -> live source corpse -> one atomic quest receipt.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::item_mint::{
    CORPSE_MATERIALIZATION_PURPOSE_KEY, GroundPlacement, ItemMintCause, ItemMintOutcome,
    ItemMintRequest, TypedDefinitionRef,
};
use crate::gameplay_transport::connection::UseCommand;
use crate::gameplay_transport::item_view::ItemKey;
use oteryn_protocol_oteryn::world_object::{MapUseTarget, UseDisposition, UseIntent, UseTarget};

const TRACK: &str = "oteryn:quest-progress/engine-corpse-fixture";

#[test]
fn engenc_corpse_use_postgres_exact_source_expiry_atomicity_and_replay() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-engenc-corpse")
}

async fn corpse(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    ordinal: u64,
    race: &str,
    dx: i32,
) -> TestResult<[u8; 16]> {
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|_| "fence")?
        .ok_or("fence")?;
    let (position, digest, frame) = {
        let runtime = owner.runtime.lock().await;
        (
            runtime
                .read_actor_position(admitted.runtime_actor.ok_or("actor")?)
                .map_err(debug)?
                .position(),
            runtime.content_pin().server_artifact_digest(),
            runtime.content_pin().frame_binding_digest(),
        )
    };
    let definition = TypedDefinitionRef {
        family: "Item".into(),
        production_key: "oteryn:item.tibia.i5964".into(),
        revision_ref: "definition-r1".into(),
    };
    let cause = ItemMintCause::for_test(
        owner.world_id,
        owner.channel_id,
        fence.scope_ownership_generation,
        100,
        ordinal,
        definition.clone(),
        CORPSE_MATERIALIZATION_PURPOSE_KEY.into(),
        0,
    )
    .with_corpse_race(
        crate::domain::bestiary::BestiaryRace::new(race, "definition-r1", vec![10, 100, 250])
            .map_err(debug)?,
    )
    .map_err(debug)?;
    let map = owner.bundle_map().ok_or("map")?;
    let mut mint = owner
        .root
        .freeze_item_mint(
            owner.character,
            owner.holder,
            ItemMintRequest {
                cause,
                item: definition,
                quantity: 1,
                ground: GroundPlacement {
                    spatial_position:
                        crate::durability::map_item_mint_audit::map_item_spatial_position(
                            (position.x + dx).try_into()?,
                            position.y.try_into()?,
                            (-position.floor).try_into()?,
                        ),
                    corpse_ref: [ordinal.to_be_bytes(), ordinal.to_be_bytes()].concat(),
                    map_revision: crate::map::overlay::map_revision(map.overlay.base()),
                    content_revision: format!(
                        "sha256:{}",
                        crate::gameplay_transport::quest_callbacks::hex(&digest)
                    ),
                    native_room_placement_context: frame.to_vec(),
                },
                content_revision: "content-1".into(),
                ruleset_revision: "ruleset-1".into(),
                sim_revision: "sim-1".into(),
            },
        )
        .await
        .map_err(debug)?;
    let permit = owner.spell_lane_permit().await.ok_or("lane")?;
    match owner
        .root
        .commit_corpse_mint(
            &permit,
            owner.character,
            owner.holder,
            &mut mint,
            *fence.character_id.as_bytes(),
        )
        .await
        .map_err(debug)?
    {
        ItemMintOutcome::Committed(receipt) => Ok(receipt.item_instance_id),
        outcome => Err(format!("corpse mint: {outcome:?}").into()),
    }
}

async fn use_target(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    source: [u8; 16],
    target: [u8; 16],
    command: u64,
    target_handle: u64,
) -> TestResult<UseDisposition> {
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|_| "fence")?
        .ok_or("fence")?;
    let intent = UseIntent {
        target: UseTarget::Item(std::num::NonZeroU64::new(1).ok_or("source handle")?),
        use_with: None,
        use_with_map_item: Some(MapUseTarget::Item(
            std::num::NonZeroU64::new(target_handle).ok_or("target handle")?,
        )),
        use_with_position: None,
    };
    Ok(owner
        .use_cooking_conversion(
            admitted.runtime_actor.ok_or("actor")?,
            UseCommand {
                game_session_id: admitted.game_session_id,
                command_id: command,
                item_fence: Some(fence.item_fence()),
            },
            intent,
            Some(ItemKey::Instance(source)),
            Some(ItemKey::Instance(target)),
        )
        .await
        .ok_or("quest route")?
        .disposition)
}

async fn expire(h: &Harness, corpse: [u8; 16]) -> TestResult {
    // Fixture clock setup: only the immutable mint's historical timestamp is moved.
    // The live admission and consumption still execute every production guard.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE game_item_mint_receipts SET materialized_at=materialized_at-1000000 WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(corpse.as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    let mut ordinal = 800;
    for _ in 0..2 {
        super::qengine_voucher_stories::mint(owner, admitted, 5908, 1, &mut ordinal).await?;
    }
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    let sources: Vec<sqlx::types::Uuid> = sqlx::query_scalar(
        "SELECT item_instance_id FROM game_item_instances WHERE lifecycle=1 AND definition_production_key='oteryn:item.tibia.i5908' ORDER BY item_instance_id LIMIT 2")
        .fetch_all(&h.pool).await?;
    assert_eq!(sources.len(), 2);
    let selected = *sources[1].as_bytes();
    let valid = corpse(owner, admitted, 900, "oteryn:creature.rat", 0).await?;
    let wrong_race = corpse(owner, admitted, 901, "oteryn:creature.cave_rat", 0).await?;
    let far = corpse(owner, admitted, 902, "oteryn:creature.rat", 3).await?;
    let expired = corpse(owner, admitted, 903, "oteryn:creature.rat", 0).await?;
    expire(h, expired).await?;
    for (n, target) in [wrong_race, far, expired, [0; 16]].into_iter().enumerate() {
        assert_eq!(
            use_target(owner, admitted, selected, target, 5000 + n as u64, 2).await?,
            UseDisposition::Rejected
        );
    }
    assert_eq!(super::qengine_voucher_stories::quantity(h, 5908).await?, 2);
    let refused: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_quest_callback_consumptions WHERE transition_key='oteryn:quest-reward/engine-corpse-fixture'",
    ).fetch_one(&h.pool).await?;
    assert_eq!(refused, 0, "refusals leave no partial consumption proof");
    let disposition = use_target(owner, admitted, selected, valid, 5010, 2).await?;
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    let admitted_corpse = native
        .item_index()
        .and_then(|index| index.definition_ref("Item", "oteryn:item.tibia.i5964", "definition-r1"))
        .is_some();
    let spell_policy = native
        .item_policy("oteryn:item.tibia.i5964", "definition-r1")
        .is_some();
    assert_eq!(
        disposition,
        UseDisposition::Committed,
        "corpse Item admission={admitted_corpse}; optional spell-use policy={spell_policy}"
    );
    let quantity: i64 =
        sqlx::query_scalar("SELECT quantity FROM game_item_instances WHERE item_instance_id=$1")
            .bind(sources[0])
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(quantity, 1, "the unselected lookalike tool is untouched");
    let retired: (i64, i16) = sqlx::query_as(
        "SELECT quantity,lifecycle FROM game_item_instances WHERE item_instance_id=$1",
    )
    .bind(sources[1])
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(retired, (0, 2), "only the selected tool is retired");
    assert_eq!(super::qengine_voucher_stories::quantity(h, 5908).await?, 1);
    let progress: i64 =
        sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE track_key=$1")
            .bind(TRACK)
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(progress, 1);
    expire(h, valid).await?;
    assert_eq!(
        use_target(owner, admitted, selected, valid, 5010, 2).await?,
        UseDisposition::Committed
    );
    assert_eq!(
        use_target(owner, admitted, selected, valid, 5010, 3).await?,
        UseDisposition::Rejected
    );
    assert_eq!(super::qengine_voucher_stories::quantity(h, 5908).await?, 1);
    Ok(())
}
