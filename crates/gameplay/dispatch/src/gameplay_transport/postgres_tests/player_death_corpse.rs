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

#[test]
fn player_death_completes_above_corpse_scope_and_entry_limits() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run_case(admin, Case::Crowded)
}

#[test]
fn manual_death_corpse_loot_enforces_carried_tree_limits() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run_case(admin, Case::Deep)
}

#[test]
fn manual_death_corpse_loot_counts_all_incoming_descendants() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run_case(admin, Case::Wide)
}

#[test]
fn manual_death_corpse_loot_weighs_loaded_backpack() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run_case(admin, Case::Heavy)
}

#[test]
fn player_corpse_decay_records_nested_child_ordinal() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run_case(admin, Case::NestedAudit)
}

#[derive(Clone, Copy)]
enum Case {
    Custody,
    ReplacementBag,
    Decay,
    Crowded,
    Deep,
    Wide,
    Heavy,
    NestedAudit,
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

    match case {
        Case::Crowded => seed_crowded_death(h).await?,
        Case::Deep => seed_loaded_backpack(h, 8, true).await?,
        Case::Wide => seed_loaded_backpack(h, 499, false).await?,
        Case::NestedAudit => {
            let mut tx = h.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
            sqlx::query("UPDATE game_item_container_entries SET placement_ordinal=7 WHERE item_instance_id=$1")
                .bind(sqlx::types::Uuid::from_bytes(id(71))).execute(&mut *tx).await?;
            tx.commit().await?;
        }
        _ => {}
    }

    let deps = Dependencies::new()?;
    let owner = deps.owner(h, &authority);
    let request = death_request(90)?;
    let death = owner
        .settle_source_player_death(current, request.clone(), None, None)
        .await
        .map_err(debug)?;
    let selected = death.items.as_ref().ok_or("missing item loss selection")?;
    assert_eq!(selected.lost, [id(70)]);
    if matches!(case, Case::Crowded) {
        let complete: bool = sqlx::query_scalar("SELECT game_player_death_items_complete($1)")
            .bind(sqlx::types::Uuid::from_bytes(*death.occurrence.as_bytes())).fetch_one(&h.pool).await?;
        assert!(complete);
        let ground = h.root.read_item_loop_ground_for_scope(current.runtime_scope).await?;
        let corpse = ground.iter().find(|item| item.player_corpse).ok_or("player corpse")?;
        assert_eq!(h.root.read_corpse_contents(&authority, corpse.item.item_instance_id)
            .await.map_err(debug)?.ok_or("corpse contents")?.len(), 61);
        let schedule = h.root.read_scoped_corpse_decay_schedule(&authority, current.runtime_scope).await.map_err(debug)?;
        assert_eq!(schedule.corpses.len(), 65);
        return Ok(());
    }
    if matches!(case, Case::Deep | Case::Wide | Case::Heavy) {
        return refuse_loaded_backpack(&owner, current, case).await;
    }

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
        Case::Crowded | Case::Deep | Case::Wide | Case::Heavy => unreachable!("handled before the ordinary custody assertions"),
        Case::Decay | Case::NestedAudit => {
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
            if matches!(case, Case::NestedAudit) {
                let envelope: Vec<u8> = sqlx::query_scalar("SELECT envelope FROM game_item_audit_outbox WHERE item_instance_id=$1")
                    .bind(sqlx::types::Uuid::from_bytes(id(71))).fetch_one(&h.pool).await?;
                let (_, event) = crate::durability::item_decay_retire_audit::decode_decay_retire_envelope(&envelope)
                    .map_err(debug)?;
                let edge = event.ground_container_entry.ok_or("nested audit edge")?;
                assert_eq!(edge.placement_ordinal, 7);
                assert_eq!(edge.parent_item_instance_id, id(70));
                assert_eq!(edge.tree_root_item_instance_id.as_deref(), Some(corpse.as_bytes().as_slice()));
            }
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

async fn seed_loaded_backpack(h: &Harness, items: i32, chain: bool) -> TestResult {
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
    // The fixture already has a coin at ordinal 1. Add a legal depth-eight
    // chain or enough branches to fill the donor's entire descendant budget.
    sqlx::query(
        "WITH nodes AS MATERIALIZED (
            SELECT n,game_character_uuid_v7() AS item,game_character_uuid_v7() AS txn,
                CASE WHEN $4 THEN n-1 ELSE (n-1)/19 END AS parent,
                CASE WHEN $4 THEN 2 ELSE (n-1)%19+2 END AS ordinal
            FROM generate_series(1,$3::int) n
         ), inserted AS (
            INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,
                definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id)
            SELECT item,$2,'Item','oteryn:item.tibia.i2854','definition-r1',1,1,txn FROM nodes
         )
         INSERT INTO game_item_nested_entries(item_instance_id,world_id,parent_item_instance_id,
            placement_ordinal,placed_transaction_id)
         SELECT n.item,$2,coalesce(p.item,$1),n.ordinal,n.txn
         FROM nodes n LEFT JOIN nodes p ON p.n=n.parent",
    ).bind(sqlx::types::Uuid::from_bytes(id(70))).bind(sqlx::types::Uuid::from_bytes(id(42)))
        .bind(items).bind(chain).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

async fn seed_crowded_death(h: &Harness) -> TestResult {
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
    // Existing scope occupancy is seeded before invoking the real death owner.
    sqlx::query(
        "WITH bodies AS MATERIALIZED (
            SELECT n,game_character_uuid_v7() AS item,game_character_uuid_v7() AS txn,
              game_character_uuid_v7() AS event FROM generate_series(1,64) n
         ), items AS (
            INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,
                definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id)
            SELECT item,$1,'Item','oteryn:item.tibia.i5973','definition-r1',1,1,txn FROM bodies
         ), locations AS (
            INSERT INTO game_item_ground_locations(item_instance_id,world_id,channel_id,
                runtime_scope_ownership_generation,spatial_position,corpse_ref,map_revision,
                content_revision,native_room_placement_context,stack_ordinal)
            SELECT item,$1,$2,1,int4send(n)||int4send(0)||int2send(7::smallint),
                uuid_send(item),'map-1','content-1',uuid_send(item),10000+n FROM bodies
         )
         INSERT INTO game_item_mint_receipts(death_world_id,death_channel_id,death_scope_ownership_generation,
            death_actor_local_id,death_actor_local_generation,loot_table_family,loot_table_production_key,
            loot_table_revision_ref,loot_purpose_key,draw_ordinal,intent_binding,transaction_id,event_id,
            item_instance_id,occurred_at,envelope_sha256,committed_at,materialized_at)
         SELECT $1,$2,1,n,1,'LootTable','oteryn:loot.creature.rat','definition-r1','CORPSE_MATERIALIZATION',
            0,decode(repeat('ab',33),'hex'),txn,event,item,1,sha256(uuid_send(event)),1,1 FROM bodies",
    ).bind(sqlx::types::Uuid::from_bytes(id(42))).bind(sqlx::types::Uuid::from_bytes(id(43)))
        .execute(&mut *tx).await?;
    sqlx::query(
        "INSERT INTO game_character_pvp_state(character_id,world_id,skull,skull_until)
         VALUES($1,$2,1,clock_timestamp()+interval '1 day')",
    ).bind(sqlx::types::Uuid::from_bytes(id(41))).bind(sqlx::types::Uuid::from_bytes(id(42)))
        .execute(&mut *tx).await?;
    // Three bound bags each hold twenty unbound branches. The ordinary coin
    // already in MAIN is the 61st branch and remains unbound as well.
    sqlx::query(
        "WITH nodes AS MATERIALIZED (
            SELECT n,game_character_uuid_v7() AS item,game_character_uuid_v7() AS txn
            FROM generate_series(1,63) n
         ), items AS (
            INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,
                definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id)
            SELECT item,$2,'Item',CASE WHEN n<=3 THEN 'oteryn:item.tibia.i2854' ELSE 'oteryn:item.tibia.i3031' END,
              'definition-r1',1,1,txn FROM nodes
         ), bound AS (
            INSERT INTO game_item_bindings(item_instance_id,world_id,character_id,cause)
            SELECT item,$2,$3,'STORE_PURCHASE' FROM nodes WHERE n<=3
            UNION ALL SELECT $1,$2,$3,'STORE_PURCHASE'
         )
         INSERT INTO game_item_nested_entries(item_instance_id,world_id,parent_item_instance_id,
            placement_ordinal,placed_transaction_id)
         SELECT n.item,$2,coalesce(p.item,$1),CASE WHEN n.n<=3 THEN n.n+1 ELSE (n.n-4)%20+1 END,n.txn
         FROM nodes n LEFT JOIN nodes p ON n.n>3 AND p.n=(n.n-4)/20+1",
    ).bind(sqlx::types::Uuid::from_bytes(id(70))).bind(sqlx::types::Uuid::from_bytes(id(42)))
        .bind(sqlx::types::Uuid::from_bytes(id(41))).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

async fn refuse_loaded_backpack(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    current: crate::durability::character_progression::CurrentCharacterGameplayFence,
    case: Case,
) -> TestResult {
    use crate::durability::item_mint::TypedDefinitionRef;
    use crate::durability::item_transfer::{
        CurrentCharacterItemFence, ItemDefinitionFacts, ItemStackClass, ItemTransferDestination,
        ItemTransferError, ItemTransferRefusal, ItemTransferRequest, ItemWeightLookup, ItemWeightRefused,
    };
    use crate::foundation::{CommandId, CommandRef};

    struct Weights(bool);
    impl ItemWeightLookup for Weights {
        fn generation_digest(&self) -> [u8; 32] { [1; 32] }
        fn weight_of(&self, definition: &TypedDefinitionRef) -> Result<u32, ItemWeightRefused> {
            match definition.production_key.as_str() {
                "oteryn:item.tibia.i2854" | "oteryn:item.tibia.i2853" => Ok(1),
                "oteryn:item.tibia.i3031" => Ok(if self.0 { 100_000 } else { 1 }),
                _ => Err(ItemWeightRefused::Unknown),
            }
        }
    }
    let backpack = |key: &str| ItemDefinitionFacts {
        definition: TypedDefinitionRef { family: "Item".into(), production_key: key.into(), revision_ref: "definition-r1".into() },
        weight: Some(1),
        stack: ItemStackClass::NonStackable,
        container_capacity: Some(20),
        container_slot_equip_pattern: true,
    };
    let weights = Weights(matches!(case, Case::Heavy));
    let outcome = owner.root.freeze_item_transfer(
        owner.character, owner.holder,
        CurrentCharacterItemFence {
            character_id: current.character_id,
            game_session_id: current.game_session_id,
            connection_generation: current.connection_generation,
            character_lease_generation: current.character_lease_generation,
            runtime_scope: current.runtime_scope,
            scope_ownership_generation: current.scope_ownership_generation,
        },
        ItemTransferRequest {
            weight_generation_digest: weights.generation_digest(),
            command: CommandRef::new(current.game_session_id, CommandId::new(11).map_err(debug)?),
            source_item_instance_id: id(70),
            destination: ItemTransferDestination::MainBackpack,
            item: backpack("oteryn:item.tibia.i2854"),
            backpack: Some(backpack("oteryn:item.tibia.i2853")),
            content_revision: "content-1".into(),
            ruleset_revision: "ruleset-1".into(),
            sim_revision: "sim-1".into(),
        }, &weights,
    ).await;
    match (case, outcome) {
        (Case::Deep | Case::Wide, Err(ItemTransferError::CapacityExceeded))
        | (Case::Heavy, Err(ItemTransferError::Refused(ItemTransferRefusal::Overweight))) => Ok(()),
        (_, Err(error)) => Err(debug(error)),
        _ => Err("loaded death backpack should have been refused".into()),
    }
}
