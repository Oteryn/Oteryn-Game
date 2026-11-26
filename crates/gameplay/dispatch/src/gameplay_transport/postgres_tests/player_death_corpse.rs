//! Player death loss keeps the original item in the corpse at the death cell.
use super::fixture::{Harness, debug, fence, id};
use super::{Dependencies, TestResult, item_look_fixtures, prepare_current_session};
use crate::durability::character_build::{
    BuildCause, BuildChangeRequest, BuildOccurrence, DurableBuildState, convert_vocation,
};
use crate::gameplay_transport::death_ml_loss_postgres::death as death_request;
use crate::spell::mana_training::CompiledTrainingFormula;

#[test]
fn player_death_dispatch_places_lost_backpack_in_corpse() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|error| error.to_string())?
                .block_on(async {
                    let h =
                        Harness::create_with_admission_seed(admin, "player_death_corpse", true, false)
                            .await?;
                    let result = check_corpse_custody(&h).await;
                    h.cleanup().await?;
                    result
                })
                .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "player death corpse test thread panicked")??;
    Ok(())
}

async fn check_corpse_custody(h: &Harness) -> TestResult {
    prepare_current_session(h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
    let formula = CompiledTrainingFormula::from_profile(
        include_bytes!(
            "../../../../../../tools/content-schema/native-gameplay/build-training.json"
        ),
        "content-1",
    )
    .map_err(debug)?;
    let before = DurableBuildState::default();
    let after = convert_vocation(&formula, &before, "druid").map_err(debug)?;
    let mut current = fence(1)?;
    current.character_lease_generation = 2;
    h.root
        .commit_character_build(
            &authority,
            &h.node,
            current,
            BuildChangeRequest {
                occurrence: BuildOccurrence::from_bytes(id(100)).map_err(debug)?,
                cause: BuildCause::VocationChoice,
                before,
                after,
                pruned_stance: None,
            },
            &formula,
        )
        .await
        .map_err(debug)?;
    item_look_fixtures::seed_main_backpack(h, &[("oteryn:item.tibia.i3031", 5)]).await?;

    let deps = Dependencies::new()?;
    let owner = deps.owner(h, &authority);
    let death = owner
        .settle_source_player_death(current, death_request(90)?, None, None)
        .await
        .map_err(debug)?;
    let selected = death.items.as_ref().ok_or("missing item loss selection")?;
    assert_eq!(selected.lost, [id(70)]);

    let (inside_corpse, loose_on_ground): (bool, bool) = sqlx::query_as(
        "SELECT EXISTS (
            SELECT 1 FROM game_player_death_item_records r
            JOIN game_character_death_receipts d USING (death_occurrence_id)
            JOIN game_item_corpse_container_entries e
              ON (e.item_instance_id,e.world_id,e.placed_transaction_id)
              = (r.item_instance_id,r.world_id,r.transaction_id)
            JOIN game_item_instances corpse ON corpse.item_instance_id=e.parent_item_instance_id
            JOIN game_item_ground_locations g ON g.item_instance_id=corpse.item_instance_id
            WHERE r.death_occurrence_id=encode($1,'hex')::uuid
              AND r.item_instance_id=encode($2,'hex')::uuid AND r.operation=1
              AND corpse.lifecycle=1
              AND (g.world_id,g.scope_kind,g.channel_id,g.instance_id)
                  IS NOT DISTINCT FROM (r.world_id,r.scope_kind,r.channel_id,r.instance_id)
              AND (g.spatial_position,g.map_revision,g.content_revision)
                  = (d.death_spatial_position,d.death_map_revision,d.content_revision)
        ), EXISTS (
            SELECT 1 FROM game_item_ground_locations WHERE item_instance_id=encode($2,'hex')::uuid
        )",
    )
    .bind(death.occurrence.as_bytes().as_slice())
    .bind(id(70).as_slice())
    .fetch_one(&h.pool)
    .await?;
    if (inside_corpse, loose_on_ground) != (true, false) {
        return Err(format!(
            "expected the selected backpack inside the death corpse and no loose ground row; \
             inside_corpse={inside_corpse}, loose_on_ground={loose_on_ground}"
        )
        .into());
    }
    Ok(())
}
