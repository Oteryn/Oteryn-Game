//! Player death loss keeps the original item in the corpse at the death cell.
use super::fixture::{Harness, debug, fence, id};
use super::{Dependencies, TestResult, item_look_fixtures, prepare_current_session};
use crate::durability::character_build::{
    BuildCause, BuildChangeRequest, BuildOccurrence, DurableBuildState, convert_vocation,
};
use crate::durability::item_decay_retire::{CorpseDecayFence, PLAYER_CORPSE_DECAY_AFTER_MS};
use crate::gameplay_transport::death_ml_loss_postgres::death as death_request;
use crate::spell::mana_training::CompiledTrainingFormula;

#[test]
fn player_death_dispatch_places_lost_backpack_in_corpse() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run_case(admin, Case::Custody)
}

#[test]
fn replacement_bag_after_backpack_loss() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run_case(admin, Case::ReplacementBag)
}

#[test]
fn player_corpse_registers_decay_receipt() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run_case(admin, Case::Decay)
}

enum Case {
    Custody,
    ReplacementBag,
    Decay,
}

fn run_case(admin: String, case: Case) -> TestResult {
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
                    let result = check_corpse_custody(&h, case).await;
                    h.cleanup().await?;
                    result
                })
                .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "player death corpse test thread panicked")??;
    Ok(())
}

async fn check_corpse_custody(h: &Harness, case: Case) -> TestResult {
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
    let request = death_request(90)?;
    let death = owner
        .settle_source_player_death(current, request.clone(), None, None)
        .await
        .map_err(debug)?;
    let selected = death.items.as_ref().ok_or("missing item loss selection")?;
    assert_eq!(selected.lost, [id(70)]);

    let (corpse, loose_on_ground): (Option<sqlx::types::Uuid>, bool) = sqlx::query_as(
        "SELECT (
            SELECT corpse.item_instance_id FROM game_player_death_item_records r
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
    if corpse.is_none() || loose_on_ground {
        return Err(format!(
            "expected the selected backpack inside the death corpse and no loose ground row; \
             corpse={corpse:?}, loose_on_ground={loose_on_ground}"
        )
        .into());
    }
    let corpse = corpse.ok_or("player corpse not materialized")?;
    match case {
        Case::Custody => {}
        Case::ReplacementBag => {
            let replacement = empty_replacement_bag(h).await?;
            assert_eq!(
                owner.settle_source_player_death(current, request, None, None).await.map_err(debug)?,
                death
            );
            assert_eq!(empty_replacement_bag(h).await?, replacement);
        }
        Case::Decay => {
            let schedule = h.root.read_scoped_corpse_decay_schedule(&authority, current.runtime_scope)
                .await.map_err(debug)?;
            let deadline = schedule.corpses.iter()
                .find(|entry| entry.corpse_item_instance_id == *corpse.as_bytes())
                .ok_or("player corpse missing from the durable decay schedule")?;
            assert_eq!(deadline.decay_at_unix_ms, Some(deadline.materialized_at_unix_ms + PLAYER_CORPSE_DECAY_AFTER_MS));

            // Move only the fixture's clock anchor. The production deadline and
            // retirement guards run with their normal transaction settings.
            let mut tx = h.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role = replica").execute(&mut *tx).await?;
            sqlx::query("UPDATE game_item_mint_receipts SET materialized_at=materialized_at-$2 WHERE item_instance_id=$1")
                .bind(corpse).bind(PLAYER_CORPSE_DECAY_AFTER_MS).execute(&mut *tx).await?;
            tx.commit().await?;

            let permit = owner.spell_lane_permit().await.ok_or("item lane unavailable")?;
            let retired = h.root.retire_decayed_corpse(&permit, &authority, &h.node, CorpseDecayFence {
                runtime_scope: current.runtime_scope,
                scope_ownership_generation: current.scope_ownership_generation,
            }, *corpse.as_bytes()).await.map_err(debug)?;
            assert_eq!(retired.corpse.item_instance_id, *corpse.as_bytes());
            assert_eq!(retired.entries.iter().map(|entry| entry.item_instance_id).collect::<Vec<_>>(), [id(71), id(70)]);
            let cleared: bool = sqlx::query_scalar("SELECT
                NOT EXISTS(SELECT 1 FROM game_item_ground_locations WHERE item_instance_id=$1)
                AND NOT EXISTS(SELECT 1 FROM game_item_corpse_container_entries WHERE parent_item_instance_id=$1)
                AND NOT EXISTS(SELECT 1 FROM game_item_nested_entries WHERE parent_item_instance_id=encode($2,'hex')::uuid)")
                .bind(corpse).bind(id(70).as_slice()).fetch_one(&h.pool).await?;
            assert!(cleared);
        }
    }
    eprintln!("C20b player corpse assertions reached");
    Ok(())
}

async fn empty_replacement_bag(h: &Harness) -> TestResult<sqlx::types::Uuid> {
    let (bag, empty): (sqlx::types::Uuid, bool) = sqlx::query_as(
        "SELECT i.item_instance_id,
         NOT EXISTS(SELECT 1 FROM game_item_container_entries e WHERE e.parent_item_instance_id=i.item_instance_id)
         AND NOT EXISTS(SELECT 1 FROM game_item_nested_entries e WHERE e.parent_item_instance_id=i.item_instance_id)
         FROM game_item_container_slots s JOIN game_item_instances i USING(item_instance_id,world_id)
         JOIN game_player_death_item_records r ON r.replacement_bag_id=i.item_instance_id
         WHERE s.character_id=encode($1,'hex')::uuid AND r.item_instance_id=encode($2,'hex')::uuid
           AND r.source_slot=9 AND s.placed_transaction_id=r.transaction_id AND i.minted_transaction_id=r.transaction_id
           AND i.definition_production_key='oteryn:item.tibia.i2853' AND i.quantity=1 AND i.lifecycle=1"
    ).bind(id(41).as_slice()).bind(id(70).as_slice()).fetch_one(&h.pool).await?;
    assert!(empty, "replacement bag must be empty");
    Ok(bag)
}
