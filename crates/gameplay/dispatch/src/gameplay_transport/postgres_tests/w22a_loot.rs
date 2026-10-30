//! W22a: the real auto-attack kill drains large and nested loot plans through SQL custody guards.
use super::TestResult;
use super::fixture::{Harness, debug};
use super::kill_reward_e2e::{scenario, scenario_thread};
use crate::gameplay_transport::ComposedFreshAdmission;
use serde_json::{Value, json};
use std::cell::Cell;

#[derive(Clone, Copy)]
enum Case {
    Large,
    Nested,
    Outside,
    ChargedSubtype,
}
thread_local! { static CASE: Cell<Option<Case>> = const { Cell::new(None) }; }

#[test]
fn w22a_more_than_sixteen_entries_drop_into_the_kill_corpse() -> TestResult {
    scenario_thread::run("w22a-large", || {
        CASE.set(Some(Case::Large));
        scenario()
    })
}
#[test]
fn w22a_nested_container_drop_mints_its_children_inside_the_container() -> TestResult {
    scenario_thread::run("w22a-nested", || {
        CASE.set(Some(Case::Nested));
        scenario()
    })
}
#[test]
fn w22a_charged_loot_with_a_subtype_mints_with_its_full_charges() -> TestResult {
    scenario_thread::run("w22a-charged-subtype", || {
        CASE.set(Some(Case::ChargedSubtype));
        scenario()
    })
}
#[test]
fn w22a_mint_into_a_container_outside_the_kill_corpse_is_refused() -> TestResult {
    scenario_thread::run("w22a-outside", || {
        CASE.set(Some(Case::Outside));
        scenario()
    })
}

pub(super) fn table_override() -> TestResult<Option<Vec<u8>>> {
    let Some(case) = CASE.get() else {
        return Ok(None);
    };
    let mut section: Value = serde_json::from_slice(include_bytes!(
        "../../../../../../tools/content-schema/native-gameplay/loot-tables.json"
    ))?;
    let entry = |id: u32| {
        json!({"item": {"family":"Item", "key":format!("oteryn:item.tibia.i{id}"),
        "revision":"definition-r1"}, "min_count":1, "max_count":1, "probability_ppm":1_000_000})
    };
    let table = section["tables"]
        .as_array_mut()
        .ok_or("tables")?
        .iter_mut()
        .find(|row| row["identity"]["key"] == "oteryn:loot.creature.rat")
        .ok_or("rat table")?;
    match case {
        Case::Large => table["entries"] = Value::Array(vec![entry(3031); 20]),
        Case::ChargedSubtype => {
            // A stone skin amulet (5 charges) whose loot names its subtype, as donor content
            // derives it from the charges (`register_monster_type.lua`).
            let mut amulet = entry(3081);
            amulet["subtype"] = json!(5);
            table["entries"] = json!([amulet]);
        }
        Case::Nested | Case::Outside => {
            table["algorithm"] = json!("NestedGroups");
            let mut bag = entry(2871);
            let mut inner = entry(2871);
            inner["children"] = json!([entry(3031)]);
            bag["children"] = json!([inner, entry(3607)]);
            table["entries"] = json!([bag]);
        }
    }
    Ok(Some(serde_json::to_vec(&section)?))
}

pub(super) async fn qualify(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
) -> TestResult {
    let Some(case) = CASE.get() else {
        return Ok(());
    };
    let direct: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_corpse_container_entries")
        .fetch_one(&h.pool)
        .await?;
    let nested: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_nested_entries")
        .fetch_one(&h.pool)
        .await?;
    match case {
        Case::Large => {
            assert_eq!(direct, 20);
            assert_eq!(nested, 0);
        }
        Case::ChargedSubtype => {
            assert_eq!(nested, 0);
            let minted: Vec<(String, i64)> = sqlx::query_as("SELECT i.definition_production_key, i.quantity FROM game_item_corpse_container_entries e JOIN game_item_instances i USING(item_instance_id) LIMIT 2")
                .fetch_all(&h.pool).await?;
            assert_eq!(minted, vec![("oteryn:item.tibia.i3081".to_owned(), 1)]);
            // No partial charge state is written: the amulet reads its full definition charges.
            let partial: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_timed_states")
                .fetch_one(&h.pool)
                .await?;
            assert_eq!(partial, 0);
        }
        Case::Nested | Case::Outside => {
            assert_eq!(direct, 1);
            assert_eq!(nested, 3);
            let colocated: bool = sqlx::query_scalar(
                "SELECT count(DISTINCT created_xact_id)=1 FROM game_item_mint_receipts",
            )
            .fetch_one(&h.pool)
            .await?;
            assert!(
                colocated,
                "corpse, containers and descendants commit in the same physical transaction"
            );
            let depths: Vec<i64> = sqlx::query_scalar("WITH RECURSIVE tree AS (SELECT item_instance_id, 1::bigint AS depth FROM game_item_corpse_container_entries UNION ALL SELECT e.item_instance_id, t.depth+1 FROM tree t JOIN game_item_nested_entries e ON e.parent_item_instance_id=t.item_instance_id) SELECT depth FROM tree ORDER BY depth LIMIT 8")
                .fetch_all(&h.pool).await?;
            assert_eq!(depths, vec![1, 2, 2, 3]);
        }
    }
    if matches!(case, Case::Outside) {
        outside_container_is_refused(owner, h).await?;
    }
    Ok(())
}

async fn outside_container_is_refused(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
) -> TestResult {
    use crate::durability::item_mint::{
        CorpseContainerPlacement, CorpseLootMintRequest, GroundPlacement, ItemMintCause,
        ItemMintError, ItemMintOutcome, ItemMintRequest, TypedDefinitionRef,
    };
    use crate::foundation::ScopeOwnershipGeneration;
    use sqlx::Row;
    let row = sqlx::query("SELECT death_scope_ownership_generation::text AS generation, death_actor_local_id, death_actor_local_generation::text AS actor_generation FROM game_item_mint_receipts WHERE loot_purpose_key='CORPSE_MATERIALIZATION' LIMIT 1")
        .fetch_one(&h.pool).await?;
    let generation =
        ScopeOwnershipGeneration::new(row.try_get::<String, _>("generation")?.parse()?)
            .map_err(debug)?;
    let actor = u32::try_from(row.try_get::<i64, _>("death_actor_local_id")?)?;
    let actor_generation = row.try_get::<String, _>("actor_generation")?.parse()?;
    let item = |id| TypedDefinitionRef {
        family: "Item".into(),
        production_key: format!("oteryn:item.tibia.i{id}"),
        revision_ref: "definition-r1".into(),
    };
    let cause = |purpose: &str| {
        ItemMintCause::for_test(
            owner.world_id,
            owner.channel_id,
            generation,
            actor,
            actor_generation,
            TypedDefinitionRef {
                family: "Loot".into(),
                production_key: "oteryn:loot.creature.rat".into(),
                revision_ref: "definition-r1".into(),
            },
            purpose.into(),
            100,
        )
    };
    let ground_row = sqlx::query("SELECT g.spatial_position, g.corpse_ref, g.map_revision, g.content_revision, g.native_room_placement_context FROM game_item_ground_locations g JOIN game_item_mint_receipts r USING(item_instance_id) WHERE r.loot_purpose_key='CORPSE_MATERIALIZATION' LIMIT 1")
        .fetch_one(&h.pool).await?;
    let ground = GroundPlacement {
        spatial_position: ground_row.try_get("spatial_position")?,
        corpse_ref: ground_row.try_get("corpse_ref")?,
        map_revision: ground_row.try_get("map_revision")?,
        content_revision: ground_row.try_get("content_revision")?,
        native_room_placement_context: ground_row.try_get("native_room_placement_context")?,
    };
    let permit = owner.spell_lane_permit().await.ok_or("lane")?;
    let mut bag = h
        .root
        .freeze_item_mint(
            owner.character,
            &h.node,
            ItemMintRequest {
                cause: cause("fixture:w22a.outside"),
                item: item(2871),
                quantity: 1,
                ground,
                content_revision: "content-1".into(),
                ruleset_revision: "ruleset-1".into(),
                sim_revision: "sim-1".into(),
            },
        )
        .await
        .map_err(debug)?;
    let bag = h
        .root
        .commit_item_mint(&permit, owner.character, &h.node, &mut bag)
        .await
        .map_err(debug)?;
    let (ItemMintOutcome::Committed(bag) | ItemMintOutcome::AlreadyCommitted(bag)) = bag;
    let request = CorpseLootMintRequest {
        cause: cause("fixture:w22a.illegal-child"),
        item: item(3031),
        quantity: 1,
        placement: CorpseContainerPlacement {
            corpse_item_instance_id: bag.item_instance_id,
            placement_ordinal: 1,
        },
        content_revision: "content-1".into(),
        ruleset_revision: "ruleset-1".into(),
        sim_revision: "sim-1".into(),
    };
    assert!(matches!(
        h.root
            .freeze_corpse_loot_mint(owner.character, &h.node, request)
            .await,
        Err(ItemMintError::InvalidInput)
    ));
    let escaped: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_nested_entries WHERE parent_item_instance_id=encode($1,'hex')::uuid")
        .bind(bag.item_instance_id.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(escaped, 0);
    Ok(())
}
