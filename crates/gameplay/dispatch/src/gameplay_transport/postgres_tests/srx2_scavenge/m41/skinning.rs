//! Real Source corpse mint and normal USE; deterministic draws select a real branch, never mock it.
use super::*;
use crate::durability::item_loop_move::{LoopDestination, LoopMoveRequest};
use crate::durability::item_transfer::CurrentCharacterItemFence;
use crate::foundation::{CommandId, CommandRef, ExactActorRef};

pub(super) async fn credits(h: &Harness) -> TestResult<i64> {
    Ok(
        sqlx::query_scalar("SELECT count(*) FROM game_stag_beak_proofs")
            .fetch_one(&h.pool)
            .await?,
    )
}

pub(super) async fn quantity(h: &Harness, key: &str) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT coalesce(sum(i.quantity),0)::bigint FROM game_item_instances i JOIN game_item_container_entries e USING(item_instance_id) WHERE e.character_id=encode($1,'hex')::uuid AND i.definition_production_key=$2 AND i.lifecycle=1")
        .bind(id(41).as_slice()).bind(key).fetch_one(&h.pool).await?)
}

#[allow(
    clippy::too_many_arguments,
    reason = "each real Source skinning keeps actor, custody fence, species and deterministic outcome independent"
)]
pub(super) async fn skin(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    native: &crate::content::native_gameplay::NativeGameplayState,
    fence: CurrentCharacterItemFence,
    actor: ExactActorRef,
    species: &str,
    success: bool,
    case: u32,
) -> TestResult<UseCommand> {
    let harpy = species != "minotaur";
    let source = if harpy { 52680 } else { 5969 };
    let race = native
        .reward_table()
        .row(&format!("oteryn:creature.{species}"))
        .map_err(debug)?
        .race
        .clone()
        .ok_or("Source race")?;
    let corpse = mint_corpse(owner, h, native, source, Some(race), 1000 + case).await?;
    let low = if success { 0 } else { 100_000_000 };
    let high = if success {
        if harpy { 100_000_000 } else { 70_000_000 }
    } else {
        1_000_000_000
    };
    let command = UseCommand {
        game_session_id: fence.game_session_id,
        command_id: draw_command(
            native,
            fence.game_session_id,
            corpse,
            low,
            high,
            50_000_000 + u64::from(case) * 1_000_000,
        )?,
        item_fence: Some(fence),
    };
    if case == 0 || case == 3 {
        let before = footprint(h).await?;
        let mut stale = command;
        let mut stale_fence = fence;
        stale_fence.character_lease_generation += 1;
        stale.item_fence = Some(stale_fence);
        let result = owner
            .use_cooking_conversion(
                actor,
                stale,
                skinning_intent()?,
                Some(ItemKey::Instance(id(71))),
                Some(ItemKey::Instance(corpse)),
            )
            .await;
        assert!(
            !result.is_some_and(|r| r.disposition == UseDisposition::Committed),
            "stale lease cannot skin or credit"
        );
        assert_eq!(footprint(h).await?, before, "stale authority wrote nothing");
    }
    let result = owner
        .use_cooking_conversion(
            actor,
            command,
            skinning_intent()?,
            Some(ItemKey::Instance(id(71))),
            Some(ItemKey::Instance(corpse)),
        )
        .await
        .ok_or("M41 real skinning outcome")?;
    assert_eq!(
        result.disposition,
        UseDisposition::Committed,
        "case {case} {species}"
    );
    let outputs:serde_json::Value=sqlx::query_scalar("SELECT outputs FROM game_item_craft_records WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric")
        .bind(fence.game_session_id.as_bytes().as_slice()).bind(command.command_id.to_string()).fetch_one(&h.pool).await?;
    assert_eq!(
        outputs.as_array().ok_or("craft outputs")?.len(),
        usize::from(success)
    );
    if success {
        assert_eq!(
            outputs[0]["key"],
            if harpy {
                "oteryn:item.tibia.i52632"
            } else {
                "oteryn:item.tibia.i5878"
            }
        );
        if harpy {
            // Ordinary in-backpack merging keeps the physical beaks while freeing slots.
            // Bought and earned beaks can share a stack; only immutable skinning proofs count.
            let fresh =
                *sqlx::types::Uuid::parse_str(outputs[0]["item"].as_str().ok_or("yield UUID")?)?
                    .as_bytes();
            let permit = owner.spell_lane_permit().await.ok_or("M41 merge lane")?;
            let merged = owner
                .root
                .commit_item_loop_move(
                    &permit,
                    owner.character,
                    owner.holder,
                    &fence,
                    CommandRef::new(
                        fence.game_session_id,
                        CommandId::new(1_000_000_000 + u64::from(case)).map_err(debug)?,
                    ),
                    &crate::gameplay_transport::item_ref_admission::NativeItemWeights(native),
                    LoopMoveRequest {
                        item: fresh,
                        count: None,
                        pickup_cell: None,
                        destination: LoopDestination::MainBackpack,
                    },
                )
                .await
                .map_err(debug)?
                .map_err(|e| format!("M41 beak stack merge: {e:?}"))?;
            assert_eq!(merged.operation, 2, "real beak stack merge");
        }
    }
    Ok(command)
}
