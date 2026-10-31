//! W22a: the real auto-attack kill drains large and nested loot plans through SQL custody guards.
use super::TestResult;
use super::fixture::{Harness, debug, id};
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
    FluidSubtype,
    Gut,
    DuplexOn,
    DuplexOff,
    DuplexOfflineIgnored,
    DuplexOnlineSpent,
    BoostedBossUnique,
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
fn w22a_fluid_loot_with_a_subtype_mints_the_filled_container() -> TestResult {
    scenario_thread::run("w22a-fluid-subtype", || {
        CASE.set(Some(Case::FluidSubtype));
        scenario()
    })
}
#[test]
fn w22b_gut_widens_creature_product_rolls_on_the_real_kill_path() -> TestResult {
    scenario_thread::run("w22b-gut", || {
        CASE.set(Some(Case::Gut));
        scenario()
    })
}
#[test]
fn w22c_wealth_duplex_rolls_the_kill_loot_once_more() -> TestResult {
    scenario_thread::run("w22c-duplex-on", || {
        CASE.set(Some(Case::DuplexOn));
        scenario()
    })
}
#[test]
fn w22c_a_kill_without_wealth_duplex_rolls_its_loot_once() -> TestResult {
    scenario_thread::run("w22c-duplex-off", || {
        CASE.set(Some(Case::DuplexOff));
        scenario()
    })
}
#[test]
fn w22b_wealth_duplex_ignores_wall_clock_time_spent_offline() -> TestResult {
    scenario_thread::run("w22b-duplex-offline", || {
        CASE.set(Some(Case::DuplexOfflineIgnored));
        scenario()
    })
}
#[test]
fn w22b_wealth_duplex_ends_after_one_online_hour() -> TestResult {
    scenario_thread::run("w22b-duplex-online-spent", || {
        CASE.set(Some(Case::DuplexOnlineSpent));
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

/// W13: the next scenario kills a boosted boss whose table holds a unique amulet, a ring and
/// gold, each at 100%.
pub(super) fn boosted_boss_unique_case() {
    CASE.set(Some(Case::BoostedBossUnique));
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
        Case::DuplexOn | Case::DuplexOff | Case::DuplexOfflineIgnored | Case::DuplexOnlineSpent => {
            table["entries"] = Value::Array(vec![entry(3031); DUPLEX_BASE_ENTRIES]);
        }
        Case::ChargedSubtype => {
            // A stone skin amulet (5 charges) whose loot names its subtype, as donor content
            // derives it from the charges (`register_monster_type.lua`).
            let mut amulet = entry(3081);
            amulet["subtype"] = json!(5);
            table["entries"] = json!([amulet]);
        }
        Case::FluidSubtype => {
            // Crystal 201189f8 `data/libs/functions/container.lua:48-52`: a vial entry with a
            // sub-type is minted holding that fluid (1, water); one without stays empty.
            let mut filled = entry(2874);
            filled["subtype"] = json!(1);
            table["entries"] = json!([filled, entry(2874)]);
            // The vial is not materializable content loot (no Crystal monster drops one by
            // sub-type), so this case admits it to the loot facts as it admits the table.
            let vial = section["items"]
                .as_array_mut()
                .ok_or("items")?
                .iter_mut()
                .find(|row| row["item"]["key"] == "oteryn:item.tibia.i2874");
            match vial {
                Some(row) => row["materializable"] = json!(true),
                None => section["items"].as_array_mut().ok_or("items")?.push(json!({
                    "container_capacity": null,
                    "item": {"family":"Item", "key":"oteryn:item.tibia.i2874",
                        "revision":"definition-r1"},
                    "materializable": true
                })),
            }
        }
        Case::BoostedBossUnique => {
            let mut amulet = entry(3081);
            amulet["unique"] = json!(true);
            table["entries"] = json!([entry(3031), amulet, entry(3051)]);
        }
        Case::Gut => {
            // Twenty minotaur leathers, a client "Creature Products" item, at 80% each.
            let mut leather = entry(5878);
            leather["probability_ppm"] = json!(GUT_PROBABILITY_PPM);
            table["entries"] = Value::Array(vec![leather; 20]);
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

const GUT_PROBABILITY_PPM: u32 = 800_000;
const DUPLEX_BASE_ENTRIES: usize = 5;
const DUPLEX_KEY: &str = "oteryn:item.tibia.i36727";

/// W22b: the loot owner holds Gut at stage 3 (12%) on the rat before the kill.
pub(super) async fn before_kill(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    actor: crate::foundation::ExactActorRef,
    session: crate::foundation::GameSessionId,
) -> TestResult {
    if matches!(CASE.get(), Some(Case::DuplexOn)) {
        return use_wealth_duplex(owner, h, actor, session).await;
    }
    if let Some(case @ (Case::DuplexOfflineIgnored | Case::DuplexOnlineSpent)) = CASE.get() {
        use_wealth_duplex(owner, h, actor, session).await?;
        return age_wealth_duplex(h, session, matches!(case, Case::DuplexOnlineSpent)).await;
    }
    if !matches!(CASE.get(), Some(Case::Gut)) {
        return Ok(());
    }
    use crate::domain::charm::{BestiaryRaceKey, CharmKey, CharmStage};
    use crate::durability::charm_state::{CharacterCharmState, CombatCharmState};
    let key = CharmKey::new("oteryn:charm.gut").map_err(debug)?;
    let state = CharacterCharmState {
        unlocks: [(key.clone(), CharmStage::new(3).map_err(debug)?)].into(),
        assignments: [(
            key,
            BestiaryRaceKey::new("oteryn:creature.rat").map_err(debug)?,
        )]
        .into(),
    };
    let runtime = owner.runtime.lock().await;
    assert!(owner.states.spell_states.lock().await.install_charm_state(
        &runtime,
        actor,
        session,
        CombatCharmState::new(
            crate::domain::CharacterId::from_bytes(id(41)).map_err(debug)?,
            state
        )
    ));
    Ok(())
}

/// W22c: a carried Wealth Duplex stack of two is used through the real USE path. The replayed
/// command and a second use inside the cooldown burn nothing more.
async fn use_wealth_duplex(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    actor: crate::foundation::ExactActorRef,
    session: crate::foundation::GameSessionId,
) -> TestResult {
    use crate::gameplay_transport::connection::UseCommand;
    use crate::gameplay_transport::item_view::ItemKey;
    use oteryn_protocol_oteryn::world_object::UseDisposition;
    let current =
        crate::durability::fresh_admission::FreshAdmissionStore::from_root(owner.root.clone())
            .current_session_at(session)
            .await?
            .0;
    let fence = crate::gameplay_transport::item_fence_of(current).ok_or("item fence")?;
    let revision = {
        let runtime = owner.runtime.lock().await;
        let index = crate::gameplay_transport::item_ref_admission::pinned_index(
            &runtime,
            owner.active_generation,
        )
        .ok_or("index")?;
        (
            index.revision("oteryn:item.tibia.i2854").ok_or("bag")?.to_owned(),
            index.revision(DUPLEX_KEY).ok_or("wealth duplex")?.to_owned(),
        )
    };
    let pool = &h.pool;
    let mut tx = pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role = replica")
        .execute(&mut *tx)
        .await?;
    for (seed, key, revision, quantity) in [
        (70, "oteryn:item.tibia.i2854", &revision.0, 1_i64),
        (80, DUPLEX_KEY, &revision.1, 2),
    ] {
        sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item',$3,$4,$5,1,encode($6,'hex')::uuid)")
            .bind(id(seed).as_slice()).bind(owner.world_id.as_bytes().as_slice()).bind(key).bind(revision)
            .bind(quantity).bind(id(seed + 1).as_slice()).execute(&mut *tx).await?;
    }
    sqlx::query("INSERT INTO game_item_container_slots(character_id,item_instance_id,world_id,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid)")
        .bind(id(41).as_slice()).bind(id(70).as_slice()).bind(owner.world_id.as_bytes().as_slice()).bind(id(72).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_item_container_entries(item_instance_id,world_id,character_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,1,encode($5,'hex')::uuid)")
        .bind(id(80).as_slice()).bind(owner.world_id.as_bytes().as_slice()).bind(id(41).as_slice()).bind(id(70).as_slice()).bind(id(82).as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    let item = ItemKey::Instance(id(80));
    let command = |command_id| UseCommand {
        game_session_id: session,
        command_id,
        item_fence: Some(fence),
    };
    let used = owner.use_backpack_item(actor, command(900), item).await;
    assert_eq!(used.disposition, UseDisposition::Committed);
    let replayed = owner.use_backpack_item(actor, command(900), item).await;
    assert_eq!(replayed.disposition, UseDisposition::Committed);
    let cooling = owner.use_backpack_item(actor, command(901), item).await;
    assert_eq!(cooling.disposition, UseDisposition::Exhausted);
    let left: i64 = sqlx::query_scalar(
        "SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid",
    )
    .bind(id(80).as_slice())
    .fetch_one(pool)
    .await?;
    let burns: i64 =
        sqlx::query_scalar("SELECT count(*) FROM game_item_use_records WHERE cause_kind=527")
            .fetch_one(pool)
            .await?;
    let effects: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_concoction_effects WHERE expires_at=started_at+interval '1 hour'")
        .fetch_one(pool)
        .await?;
    assert_eq!((left, burns, effects), (1, 1, 1), "one unit, one burn, one effect");
    Ok(())
}

/// W22b: the activation is moved two hours into the past on the wall clock. With `online`
/// the current session is moved back as well, so those two hours were spent online; without
/// it the Character was offline for them. Also proves the terminal stamp ends a session's span.
async fn age_wealth_duplex(
    h: &Harness,
    session: crate::foundation::GameSessionId,
    online: bool,
) -> TestResult {
    const TWO_HOURS_US: i64 = 7_200_000_000;
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role = replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE game_character_concoction_effects SET started_at=started_at-interval '2 hours', expires_at=expires_at-interval '2 hours'")
        .execute(&mut *tx)
        .await?;
    if online {
        sqlx::query("UPDATE game_durability_reconnect_sessions SET training_created_at_us=training_created_at_us-$1 WHERE character_id=encode($2,'hex')::uuid")
            .bind(TWO_HOURS_US)
            .bind(id(41).as_slice())
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    // A terminal session counts only up to its stamp, never up to a later `now` (rolled back).
    let mut tx = h.pool.begin().await?;
    sqlx::query("UPDATE game_durability_reconnect_sessions SET session_state=3,current_transport_ref=NULL,prepared_attempt_ref=NULL WHERE game_session_id=encode($1,'hex')::uuid")
        .bind(session.as_bytes().as_slice())
        .execute(&mut *tx)
        .await?;
    let (stamped, at_end, an_hour_later): (i64, i64, i64) = sqlx::query_as(
        "SELECT (SELECT count(*) FROM game_character_session_ends \
                  WHERE game_session_id=encode($1,'hex')::uuid AND character_id=encode($2,'hex')::uuid), \
                game_character_online_us(encode($2,'hex')::uuid, clock_timestamp()-interval '1 day', clock_timestamp()), \
                game_character_online_us(encode($2,'hex')::uuid, clock_timestamp()-interval '1 day', clock_timestamp()+interval '1 hour')",
    )
    .bind(session.as_bytes().as_slice())
    .bind(id(41).as_slice())
    .fetch_one(&mut *tx)
    .await?;
    tx.rollback().await?;
    eprintln!("W22B_ONLINE stamped={stamped} at_end={at_end} an_hour_later={an_hour_later}");
    assert_eq!(stamped, 1, "the terminal transition stamps the session end");
    assert!(
        an_hour_later - at_end < 1_000_000,
        "an ended session stops counting online time"
    );
    assert_eq!(at_end >= TWO_HOURS_US, online, "only online time is counted");
    Ok(())
}

/// The draw ordinals the rat's table plans at this death, with `gut` on its creature products.
fn planned_ordinals(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    death: crate::combat::LootPlanDeathKey,
    gut: u32,
) -> TestResult<Vec<i64>> {
    let row = owner
        .reward_table()
        .row("oteryn:creature.rat")
        .map_err(debug)?;
    let mut table = row.loot_table.clone();
    for entry in &mut table.entries {
        assert!(entry.details.creature_product, "the loot facts mark the leather");
        entry.details.gut_bonus_hundredths = gut;
    }
    let plan = crate::combat::plan_creature_loot(death, &row.loot_table_ref, &table)
        .map_err(debug)?;
    Ok(plan
        .entries
        .iter()
        .map(|entry| i64::from(entry.draw_ordinal))
        .collect())
}

async fn gut_widened_the_roll(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
) -> TestResult {
    use sqlx::Row;
    let row = sqlx::query("SELECT death_scope_ownership_generation::text AS generation, death_actor_local_id, death_actor_local_generation::text AS actor_generation FROM game_item_mint_receipts WHERE loot_purpose_key='CORPSE_MATERIALIZATION' LIMIT 1")
        .fetch_one(&h.pool).await?;
    let death = crate::combat::LootPlanDeathKey::new(
        *owner.world_id.as_bytes(),
        *owner.channel_id.as_bytes(),
        row.try_get::<String, _>("generation")?.parse()?,
        u32::try_from(row.try_get::<i64, _>("death_actor_local_id")?)?,
        row.try_get::<String, _>("actor_generation")?.parse()?,
    );
    let minted: Vec<i64> = sqlx::query_scalar("SELECT draw_ordinal FROM game_item_mint_receipts WHERE loot_purpose_key='oteryn:item.tibia.i5878' ORDER BY draw_ordinal LIMIT 21")
        .fetch_all(&h.pool).await?;
    let without = planned_ordinals(owner, death, 0)?;
    let with = planned_ordinals(owner, death, 1_200)?;
    eprintln!(
        "W22B_GUT minted={} without_gut={} with_gut={}",
        minted.len(),
        without.len(),
        with.len()
    );
    assert_eq!(minted, with, "the kill mints exactly the Gut-widened plan");
    assert!(
        without.iter().all(|ordinal| with.contains(ordinal)) && with.len() > without.len(),
        "Gut adds creature products this death would not drop without it"
    );
    Ok(())
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
        Case::Gut => assert_eq!(nested, 0),
        Case::DuplexOn | Case::DuplexOff | Case::DuplexOfflineIgnored | Case::DuplexOnlineSpent => {
            // W22b: two wall-clock hours spent offline leave the online hour live; spent
            // online, they end it.
            let rolls = if matches!(case, Case::DuplexOn | Case::DuplexOfflineIgnored) {
                2
            } else {
                1
            };
            eprintln!("W22C_DUPLEX corpse_entries={direct} rolls={rolls}");
            assert_eq!(direct, i64::try_from(DUPLEX_BASE_ENTRIES * rolls)?);
            assert_eq!(nested, 0);
        }
        Case::BoostedBossUnique => {
            // Crystal 201189f8 `data/libs/functions/monster.lua:219-221`: the boss bonus rolls
            // take equipment only and never a unique entry, so gold and the unique amulet drop
            // once from the base roll and the ring once per roll (1 + 2 or 3 bonus rolls).
            assert_eq!(nested, 0);
            let rows: Vec<(String, i64)> = sqlx::query_as("SELECT i.definition_production_key, count(*) FROM game_item_corpse_container_entries e JOIN game_item_instances i USING(item_instance_id) GROUP BY 1 ORDER BY 1")
                .fetch_all(&h.pool).await?;
            eprintln!("W13_BOOSTED_BOSS corpse={rows:?}");
            let count = |key: &str| rows.iter().find(|(k, _)| k == key).map_or(0, |(_, n)| *n);
            assert_eq!(count("oteryn:item.tibia.i3031"), 1);
            assert_eq!(count("oteryn:item.tibia.i3081"), 1);
            assert!(matches!(count("oteryn:item.tibia.i3051"), 3 | 4));
            assert_eq!(direct, 2 + count("oteryn:item.tibia.i3051"));
        }
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
        Case::FluidSubtype => {
            assert_eq!((direct, nested), (2, 0));
            let fluids: Vec<(i16, bool)> = sqlx::query_as("SELECT s.fluid_subtype, EXISTS(SELECT 1 FROM game_item_corpse_container_entries e WHERE e.item_instance_id=s.item_instance_id) FROM game_item_fluid_states s JOIN game_item_fluid_loot_mints m ON m.item_instance_id=s.item_instance_id AND m.transaction_id=s.last_transaction_id AND m.fluid_subtype=s.fluid_subtype")
                .fetch_all(&h.pool).await?;
            assert_eq!(fluids, vec![(1, true)]);
            let all: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_fluid_states")
                .fetch_one(&h.pool)
                .await?;
            assert_eq!(all, 1, "the vial without a sub-type stays empty");
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
    if matches!(case, Case::Gut) {
        gut_widened_the_roll(owner, h).await?;
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
            fluid_subtype: None,
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
