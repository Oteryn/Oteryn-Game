//! C20b: production dispatch death settlement on routed PostgreSQL, including durable replay.
use super::fixture::{Harness, debug, fence, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::domain::death::{EquipmentSlot, EquippedItem, PveDeathInput, calculate_pve_death};
use crate::durability::character_build::{
    BuildCause, BuildChangeRequest, BuildOccurrence, DurableBuildState, convert_vocation,
};
use crate::gameplay_transport::death_ml_loss_postgres::death as death_request;
use crate::spell::mana_training::CompiledTrainingFormula;
use oteryn_simulation_determinism::{DecisionOccurrenceId, ExactI64, GameplayDecisionRoot};
use sqlx::ConnectOptions;

#[derive(Default)]
struct Case {
    amulet: bool,
    skull: i16,
    blessings: u8,
    interrupt: bool,
    exempt: bool,
}

fn scenario(case: Case) -> TestResult {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            run(async move |admin| run_case(admin, case).await).map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "death test thread panicked")?
        .map_err(Into::into)
}

async fn run_case(admin: String, case: Case) -> TestResult {
    let Case {
        amulet,
        skull,
        blessings,
        interrupt,
        exempt,
    } = case;
    let h = Harness::create_with_admission_seed(admin, "dispatch_death_items", true, false).await?;
    prepare_current_session(&h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
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
    if !exempt {
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
    }
    super::item_look::seed_main_backpack(&h, &[("oteryn:item.tibia.i3031", 5)]).await?;
    // Initial custody fixture only; death and all item changes use the real runtime writers.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role = replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)")
        .bind(id(41).as_slice()).execute(&mut *tx).await?;
    if amulet {
        sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item','oteryn:item.tibia.i3057','definition-r1',1,1,encode($3,'hex')::uuid)")
            .bind(id(75).as_slice()).bind(id(42).as_slice()).bind(id(76).as_slice()).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_character_equipment_slots(character_id,slot,item_instance_id,world_id,placed_transaction_id) VALUES(encode($1,'hex')::uuid,7,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid)")
            .bind(id(41).as_slice()).bind(id(75).as_slice()).bind(id(42).as_slice()).bind(id(76).as_slice()).execute(&mut *tx).await?;
    }
    if skull > 0 {
        sqlx::query("INSERT INTO game_character_pvp_state(character_id,world_id,skull,skull_until) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,clock_timestamp()+interval '1 day')")
            .bind(id(41).as_slice()).bind(id(42).as_slice()).bind(skull).execute(&mut *tx).await?;
    }
    let blessing_keys: Vec<String> = (0..blessings).map(|i| format!("blessing-{i}")).collect();
    for key in &blessing_keys {
        sqlx::query("INSERT INTO game_character_blessings(character_id,blessing_key,provenance) VALUES(encode($1,'hex')::uuid,$2,'fixture')")
            .bind(id(41).as_slice()).bind(key).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    let deps = Dependencies::new()?;
    let owner = deps.owner(&h, &authority);
    let mut request = death_request(90)?;
    request.held_blessings = blessing_keys;
    if interrupt {
        sqlx::raw_sql("CREATE FUNCTION test_interrupt_death() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.source_slot=9 THEN RAISE EXCEPTION 'injected crash before backpack drop'; END IF; RETURN NEW; END $$; CREATE TRIGGER test_interrupt_death BEFORE INSERT ON game_player_death_item_records FOR EACH ROW EXECUTE FUNCTION test_interrupt_death()")
            .execute(&h.pool).await?;
        assert!(
            owner
                .settle_source_player_death(current, request.clone(), None, None)
                .await
                .is_err()
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_player_death_item_records")
                .fetch_one(&h.pool)
                .await?,
            1
        );
        let stored = h
            .root
            .reconcile_character_death(&authority, request.occurrence)
            .await
            .map_err(debug)?
            .ok_or("death receipt lost")?;
        let after = crate::durability::character_progression::CurrentCharacterGameplayFence {
            expected_character_revision: stored.committed_character_revision,
            ..current
        };
        assert!(
            h.root
                .consume_pending_respawn(&authority, &h.node, after, Some(stored.occurrence))
                .await
                .is_err()
        );
        sqlx::raw_sql("DROP TRIGGER test_interrupt_death ON game_player_death_item_records; DROP FUNCTION test_interrupt_death()")
            .execute(&h.pool).await?;
        // Reconstruct both owner and runtime, just as restart admission does.
        let restarted = Dependencies::new()?;
        let reconnect = restarted.owner(&h, &authority);
        let permit = reconnect
            .spell_lane_permit()
            .await
            .ok_or("lane unavailable")?;
        reconnect
            .resume_pending_player_death_items(&permit, after)
            .await
            .map_err(debug)?;
        drop(permit);
    }
    let committed = owner
        .settle_source_player_death(current, request.clone(), None, None)
        .await
        .map_err(debug)?;
    let mut equipment = vec![EquippedItem {
        slot: EquipmentSlot::Container,
        item: id(70),
    }];
    if amulet {
        equipment.push(EquippedItem {
            slot: EquipmentSlot::Necklace,
            item: id(75),
        });
    }
    equipment.sort_by_key(|entry| entry.slot);
    let (expected_amulet, expected_lost) = if exempt {
        (None, Vec::new())
    } else if skull > 0 {
        (None, equipment.iter().map(|entry| entry.item).collect())
    } else {
        let root = deps
            .runtime
            .lock()
            .await
            .content_pin()
            .server_artifact_digest();
        let expected = calculate_pve_death(
            &PveDeathInput {
                level: 50,
                total_experience: ExactI64::new(1000),
                regular_blessings: blessings,
                promotion_benefit_current: false,
                has_vocation: true,
                equipment: &equipment,
                necklace_is_amulet_of_loss: amulet,
            },
            &GameplayDecisionRoot::from_bytes(root),
            DecisionOccurrenceId::from_bytes(*request.occurrence.as_bytes()),
        )
        .map_err(debug)?;
        (
            expected.amulet_of_loss_consumed,
            expected
                .lost_items
                .into_iter()
                .map(|entry| entry.item)
                .collect::<Vec<_>>(),
        )
    };
    let selection = committed
        .items
        .as_ref()
        .ok_or("missing persisted selection")?;
    assert_eq!(selection.amulet, expected_amulet);
    assert_eq!(selection.lost, expected_lost);
    let should_burn = expected_amulet.is_some();
    let ground: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_ground_locations WHERE spatial_position=$1 AND map_revision=$2 AND world_id=encode($3,'hex')::uuid AND channel_id=encode($4,'hex')::uuid")
        .bind(&request.death_cell.spatial_position).bind(&request.death_cell.map_revision)
        .bind(id(42).as_slice()).bind(id(43).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        ground,
        i64::try_from(expected_lost.len())?,
        "computed death loss must reach Ground"
    );
    // A lost backpack carries its child by identity, without expanding the death plan.
    let parent: String = sqlx::query_scalar("SELECT parent_item_instance_id::text FROM game_item_container_entries WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(id(71).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        parent.replace('-', ""),
        id(70)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    );
    if amulet {
        let lifecycle: i16 = sqlx::query_scalar("SELECT lifecycle FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid")
            .bind(id(75).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(lifecycle, if should_burn { 2 } else { 1 });
    }
    let records: i64 = sqlx::query_scalar("SELECT count(*) FROM game_player_death_item_records")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(
        records,
        i64::try_from(expected_lost.len() + usize::from(should_burn))?
    );
    // Reconstruct the dispatch owner: no in-memory plan survives, and replay must read the receipt.
    drop(owner);
    drop(deps);
    drop(authority);
    let options = h.pool.connect_options();
    let reopened =
        crate::durability::DurabilityRoot::connect_test_runtime(options.to_url_lossy().as_str())?;
    assert!(reopened.maintain_ready_once().await?);
    let authority = reopened
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let restarted = Dependencies::new()?;
    let mut owner = restarted.owner(&h, &authority);
    owner.root = &reopened;
    let replay = owner
        .settle_source_player_death(current, request, None, None)
        .await
        .map_err(debug)?;
    assert_eq!(committed, replay);
    let after: i64 = sqlx::query_scalar("SELECT count(*) FROM game_player_death_item_records")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(after, records);
    reopened
        .consume_pending_respawn(
            &authority,
            &h.node,
            crate::durability::character_progression::CurrentCharacterGameplayFence {
                expected_character_revision: committed.committed_character_revision,
                ..current
            },
            Some(committed.occurrence),
        )
        .await
        .map_err(debug)?
        .ok_or("pending respawn lost")?;
    eprintln!(
        "C20b_POSTGRES_EXECUTED: amulet={amulet} skull={skull} ground={ground} records={records}; replay and respawn durable"
    );
    drop(owner);
    drop(authority);
    drop(reopened);
    drop(seal);
    h.cleanup().await
}

#[test]
fn player_death_dispatch_drops_backpack_and_recovers() -> TestResult {
    scenario(Case::default())
}
#[test]
fn player_death_dispatch_burns_amulet_and_recovers() -> TestResult {
    scenario(Case {
        amulet: true,
        ..Case::default()
    })
}
#[test]
fn player_death_dispatch_red_skull_drops_amulet_and_backpack() -> TestResult {
    scenario(Case {
        amulet: true,
        skull: 1,
        ..Case::default()
    })
}

#[test]
fn player_death_dispatch_black_skull_full_loss() -> TestResult {
    scenario(Case {
        amulet: true,
        skull: 2,
        ..Case::default()
    })
}
#[test]
fn player_death_dispatch_five_blessings_preserve_amulet_and_items() -> TestResult {
    scenario(Case {
        amulet: true,
        blessings: 5,
        ..Case::default()
    })
}
#[test]
fn player_death_dispatch_no_vocation_preserves_amulet_and_items() -> TestResult {
    scenario(Case {
        amulet: true,
        exempt: true,
        ..Case::default()
    })
}
#[test]
fn player_death_dispatch_reconnect_resumes_partial_drops_before_respawn() -> TestResult {
    scenario(Case {
        amulet: true,
        skull: 1,
        interrupt: true,
        ..Case::default()
    })
}

#[test]
fn player_death_dispatch_one_blessing_uses_canonical_draw() -> TestResult {
    scenario(Case {
        blessings: 1,
        ..Case::default()
    })
}
