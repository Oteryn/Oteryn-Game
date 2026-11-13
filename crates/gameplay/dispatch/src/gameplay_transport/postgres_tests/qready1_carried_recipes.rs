//! M20's two selected, owned operands enter through the normal handle dispatcher.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::gameplay_transport::connection::UseCommand;
use crate::gameplay_transport::item_view::{ItemKey, SessionItemView, inventory_from_backpack};
use oteryn_gameplay_quests::quest::world_callbacks::{Offering, Position};
use oteryn_protocol_oteryn::world_object::{MapUseTarget, UseDisposition, UseIntent, UseTarget};

pub(super) async fn exercise(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    h: &Harness,
) -> TestResult {
    let started = std::time::Instant::now();
    let fence = admitted.item_fence.ok_or("recipe fence")?;
    for (ordinal, item) in [(401, 21505)] {
        super::quest_world_callbacks::input_item(
            owner,
            fence,
            &Offering {
                position: Position {
                    x: 32011,
                    y: 32907,
                    floor: -7,
                },
                item: format!("oteryn:item.tibia.i{item}"),
                count: 1,
            },
            ordinal,
            true,
        )
        .await?;
    }
    let inventory = owner
        .root
        .read_item_use_inventory(owner.character, owner.holder, fence)
        .await
        .map_err(debug)?;
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native recipes")?;
    let index = native.item_index().ok_or("native item index")?;
    let mut view = SessionItemView::resume(Default::default());
    view.snapshot(
        inventory_from_backpack(inventory.backpack.as_ref(), |definition| {
            index.definition_ref(
                &definition.family,
                &definition.production_key,
                &definition.revision_ref,
            )
        })
        .ok_or("recipe inventory")?,
        None,
    )
    .map_err(debug)?;
    let bag = inventory.backpack.as_ref().ok_or("recipe backpack")?;
    let operand = |key: &str| -> TestResult<_> {
        let item = bag
            .entries
            .iter()
            .find(|entry| entry.item.definition.production_key == key)
            .ok_or("selected operand")?;
        view.table()
            .handle(&ItemKey::Instance(item.item.item_instance_id))
            .ok_or_else(|| "selected operand handle".into())
    };
    let intent = UseIntent {
        target: UseTarget::Item(operand("oteryn:item.tibia.i21507")?),
        use_with: None,
        use_with_map_item: Some(MapUseTarget::Item(operand("oteryn:item.tibia.i21505")?)),
        use_with_position: None,
    };
    let command = UseCommand {
        game_session_id: admitted.game_session_id,
        command_id: 10,
        item_fence: Some(fence),
    };
    let actor = admitted.runtime_actor.ok_or("recipe actor")?;
    let result = crate::gameplay_transport::item_craft::dispatch(
        owner,
        actor,
        command,
        intent.clone(),
        Some(&view),
    )
    .await
    .ok_or("recipe dispatcher outcome")?;
    assert_eq!(result.disposition, UseDisposition::Committed);
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_quest_callback_consumptions WHERE transition_key='oteryn:quest-reward/m20/cursed-crystal-final-dust'")
        .fetch_one(&h.pool).await?;
    assert_eq!(
        count, 2,
        "both selected instances are consumed by one recipe"
    );
    let outputs: (i64, i64) = sqlx::query_as("SELECT count(*),coalesce(sum(quantity),0)::bigint FROM game_item_instances WHERE lifecycle=1 AND definition_production_key='oteryn:item.tibia.i21506'")
        .fetch_one(&h.pool).await?;
    assert_eq!(outputs, (1, 1));
    let progress: i64 = sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE track_key='oteryn:quest-progress/quest/u10_70/the_cursed_crystal/questline'")
        .fetch_one(&h.pool).await?;
    assert_eq!(
        progress, 2,
        "the selected recipe raises the real Cursed Crystal quest track"
    );
    let inventory = owner
        .root
        .read_item_use_inventory(owner.character, owner.holder, fence)
        .await
        .map_err(debug)?;
    view.inventory_committed(
        inventory_from_backpack(inventory.backpack.as_ref(), |definition| {
            index.definition_ref(
                &definition.family,
                &definition.production_key,
                &definition.revision_ref,
            )
        })
        .ok_or("updated recipe inventory")?,
    )
    .map_err(debug)?;
    let UseTarget::Item(source_handle) = intent.target else {
        return Err("recipe source handle".into());
    };
    let Some(MapUseTarget::Item(target_handle)) = intent.use_with_map_item else {
        return Err("recipe target handle".into());
    };
    assert_eq!(view.resolve(source_handle), None);
    assert_eq!(view.resolve(target_handle), None);
    let replay = crate::gameplay_transport::item_craft::dispatch(
        owner,
        actor,
        command,
        intent.clone(),
        Some(&view),
    )
    .await
    .ok_or("recipe replay")?;
    assert_eq!(replay.disposition, UseDisposition::Committed);
    let retry = crate::gameplay_transport::item_craft::dispatch(
        owner,
        actor,
        UseCommand {
            command_id: 11,
            ..command
        },
        intent,
        Some(&view),
    )
    .await
    .ok_or("spent operand refusal")?;
    assert_eq!(retry.disposition, UseDisposition::StaleState);
    let after: (i64, i64) = sqlx::query_as("SELECT count(*),coalesce(sum(quantity),0)::bigint FROM game_item_instances WHERE lifecycle=1 AND definition_production_key='oteryn:item.tibia.i21506'")
        .fetch_one(&h.pool).await?;
    assert_eq!(
        after, outputs,
        "replay and another claim cannot mint another mixture"
    );
    eprintln!(
        "POSTGRES_CASE_COMPLETED qready1-cursed-final-mixture duration={:?} success=true",
        started.elapsed()
    );
    Ok(())
}
