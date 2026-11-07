//! S06: a real auto-attack death mints a newly admitted taming source into its corpse.
use super::TestResult;
use super::fixture::Harness;
use serde_json::{Value, json};
use std::cell::Cell;

thread_local! { static ACTIVE: Cell<u8> = const { Cell::new(0) }; static NATURAL_ITEM:Cell<u32>=const{Cell::new(0)}; }

struct Guard;
impl Drop for Guard {
    fn drop(&mut self) {
        ACTIVE.set(0);
    }
}

#[test]
fn srv7_taming_source_is_reachable_through_real_death_loot_mint() -> TestResult {
    super::kill_reward_e2e::scenario_thread::run("srv7-taming-source-loot", || {
        ACTIVE.set(1);
        let _guard = Guard;
        super::kill_reward_e2e::scenario()
    })
}

#[test]
fn srv7_official_cosmetic_sources_are_reachable_through_real_death_loot_mint() -> TestResult {
    super::kill_reward_e2e::scenario_thread::run("srv7-official-cosmetic-source-loot", || {
        ACTIVE.set(2);
        let _guard = Guard;
        super::kill_reward_e2e::scenario()
    })
}

#[test]
fn srv7_charged_dracoyle_loot_starts_its_clock_on_real_corpse_pickup() -> TestResult {
    super::kill_reward_e2e::scenario_thread::run("srv7-dracoyle-first-pickup", || {
        ACTIVE.set(3);
        let _guard = Guard;
        super::kill_reward_e2e::scenario()
    })
}

#[test]
fn srv7_afflicted_natural_feverish_death_uses_actual_loot_and_world_source() -> TestResult {
    super::kill_reward_e2e::scenario_thread::run("srv7-natural-feverish-source", || {
        ACTIVE.set(4);
        NATURAL_ITEM.set(0);
        let _guard = Guard;
        super::kill_reward_e2e::scenario()
    })
}
pub(super) fn natural_active() -> bool {
    ACTIVE.get() >= 4
}
pub(super) fn natural_mode() -> u8 {
    ACTIVE.get()
}
pub(super) fn natural_creature_key() -> &'static str {
    match ACTIVE.get() {
        5 => "oteryn:creature.bear",
        6 => "oteryn:creature.chicken",
        7 => "oteryn:creature.hellspawn",
        8 => "oteryn:creature.minotaur",
        9 => "oteryn:creature.rotworm",
        10 => "oteryn:creature.wasp",
        11 => "oteryn:creature.wolf",
        12 => "oteryn:creature.demon_skeleton",
        13 => "oteryn:creature.stampor",
        _ => "oteryn:creature.feverish_citizen",
    }
}
pub(super) fn natural_plan_accepts(plan: &oteryn_gameplay_items::loot_plan::LootPlan) -> bool {
    if !natural_active() {
        return !plan.entries.is_empty();
    }
    if super::stampor_natural_death::active() || super::spike_natural_death::active() {
        return !plan.entries.is_empty();
    }
    if let Some(id) = super::acquisition_natural::target_item() {
        return plan
            .entries
            .iter()
            .any(|e| e.item.production_key == format!("oteryn:item.tibia.i{id}"));
    }
    for id in 12551..=12556 {
        if plan
            .entries
            .iter()
            .any(|e| e.item.production_key == format!("oteryn:item.tibia.i{id}"))
        {
            NATURAL_ITEM.set(id);
            return true;
        }
    }
    false
}
fn source_ids() -> &'static [u32] {
    match ACTIVE.get() {
        1 => &[5907],
        4 => match NATURAL_ITEM.get() {
            12551 => &[12551],
            12552 => &[12552],
            12553 => &[12553],
            12554 => &[12554],
            12555 => &[12555],
            12556 => &[12556],
            _ => &[],
        },
        3 => &[9034],
        _ => &[12551, 44048, 50064],
    }
}

pub(super) fn table_override() -> TestResult<Option<Vec<u8>>> {
    if ACTIVE.get() == 0 || natural_active() {
        return Ok(None);
    }
    let mut section: Value = serde_json::from_slice(include_bytes!(
        "../../../../../../tools/content-schema/native-gameplay/loot-tables.json"
    ))?;
    // Keep the real provider's source admission untouched. Only the fixture's rat
    // roll is made deterministic, just like other source-item death owner cases.
    let table = section["tables"]
        .as_array_mut()
        .ok_or("loot tables")?
        .iter_mut()
        .find(|t| t["identity"]["key"] == "oteryn:loot.creature.rat")
        .ok_or("rat table")?;
    for id in source_ids() {
        table["entries"].as_array_mut().ok_or("rat entries")?.push(json!({
            "item":{"family":"Item","key":format!("oteryn:item.tibia.i{id}"),"revision":"definition-r1"},
            "min_count":1,"max_count":1,"probability_ppm":1_000_000,
        }));
    }
    Ok(Some(serde_json::to_vec(&section)?))
}

pub(super) async fn qualify(
    h: &Harness,
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: crate::gameplay_transport::connection::AdmittedSession,
) -> TestResult {
    if ACTIVE.get() == 0 {
        return Ok(());
    }
    if super::acquisition_natural::creature().is_some() {
        return super::acquisition_natural::qualify(h, owner, admitted).await;
    }
    if natural_active() {
        assert!(
            !source_ids().is_empty(),
            "actual natural drop occurrence selected"
        );
        let native = owner
            .active_generation
            .and_then(crate::content::ActiveGeneration::native_gameplay)
            .ok_or("native natural source")?;
        let reward = native
            .reward_table()
            .row("oteryn:creature.feverish_citizen")
            .map_err(super::fixture::debug)?;
        for id in 12551..=12556 {
            let key = format!("oteryn:item.tibia.i{id}");
            assert!(
                reward
                    .loot_table
                    .entries
                    .iter()
                    .any(|e| e.item.production_key == key),
                "actual Feverish natural source for{id}"
            );
            let index = native.item_index().ok_or("native item index")?;
            let revision = index.revision(&key).ok_or("canonical natural item")?;
            let policy = native
                .item_policy(&key, revision)
                .ok_or("native natural item policy")?;
            assert!(
                policy.record().admission.materializable,
                "native source admission{id}"
            );
        }
    }
    for id in source_ids() {
        let key = format!("oteryn:item.tibia.i{id}");
        let minted: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_item_corpse_container_entries e JOIN game_item_instances i USING(item_instance_id,world_id) JOIN game_item_mint_receipts r ON r.item_instance_id=i.item_instance_id AND r.death_world_id=i.world_id WHERE i.definition_production_key=$1 AND i.quantity=1",
        ).bind(&key).fetch_one(&h.pool).await?;
        assert_eq!(
            minted, 1,
            "the real death-loot owner must mint source {key}"
        );
        eprintln!("SRV7_S06_LOOT_PG_EXECUTED source={id} minted={minted}");
    }
    if ACTIVE.get() == 3 {
        exercise_first_pickup(h, owner, admitted).await?;
    }
    Ok(())
}

async fn exercise_first_pickup(
    h: &Harness,
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: crate::gameplay_transport::connection::AdmittedSession,
) -> TestResult {
    use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
    use crate::gameplay_transport::item_view::ItemKey;
    let actor = admitted.runtime_actor.ok_or("dracoyle pickup actor")?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "dracoyle current pickup fence")?
        .ok_or("dracoyle current pickup fence")?
        .item_fence();
    let source:Vec<u8>=sqlx::query_scalar("SELECT uuid_send(item_instance_id) FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i9034'").fetch_one(&h.pool).await?;
    let source: [u8; 16] = source.try_into().map_err(|_| "dracoyle UUID")?;
    let before:i64=sqlx::query_scalar("SELECT (SELECT count(*) FROM game_spell_item_temporal_schedules WHERE item_instance_id=encode($1,'hex')::uuid)+(SELECT count(*) FROM game_item_timed_states WHERE item_instance_id=encode($1,'hex')::uuid)").bind(source.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        before, 0,
        "donor loot creation does not start decay before its first movement"
    );
    if owner
        .root
        .read_character_backpack(owner.character, fence.character_id)
        .await?
        .is_none()
    {
        super::item_look::seed_main_backpack(h, &[]).await?;
    }
    let corpses = owner.runtime.lock().await.visible_corpses();
    let mut selected = None;
    for corpse in corpses {
        let contents = owner
            .root
            .read_corpse_contents(owner.character, corpse.item.item_instance_id)
            .await?
            .ok_or("corpse contents")?;
        if contents
            .iter()
            .any(|entry| entry.item.item_instance_id == source)
        {
            selected = Some(corpse);
            break;
        }
    }
    let corpse = selected.ok_or("source-bearing visible corpse")?;
    let command = UseCommand {
        game_session_id: admitted.game_session_id,
        command_id: 900,
        item_fence: Some(fence),
    };
    owner
        .take_corpse_entry(
            actor,
            command,
            ItemKey::Entity(corpse.identity),
            ItemKey::Instance(source),
        )
        .await
        .map_err(|e| format!("actual corpse take: {e:?}"))?;
    let timer:Option<(String,String,i64,i64,bool)>=sqlx::query_as("SELECT p.definition_production_key,p.target_production_key,p.duration_millis,p.expires_at_unix_ms,p.source_transaction_id=r.transaction_id AND p.expires_at_unix_ms=r.physical_started_at_unix_ms+604800000 FROM game_spell_item_temporal_schedules p JOIN game_item_transfer_receipts r ON r.source_item_instance_id=p.item_instance_id WHERE p.item_instance_id=encode($1,'hex')::uuid")
        .bind(source.as_slice()).fetch_optional(&h.pool).await?;
    let timer = timer
        .ok_or("first actual corpse pickup must start the source-qualified seven-day clock")?;
    assert_eq!(
        (&timer.0, &timer.1, timer.2, timer.4),
        (
            &"oteryn:item.tibia.i9034".to_string(),
            &"oteryn:item.tibia.i9035".to_string(),
            604800000,
            true
        )
    );
    assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_item_container_entries WHERE item_instance_id=encode($1,'hex')::uuid").bind(source.as_slice()).fetch_one(&h.pool).await?,1);
    let _ = owner
        .take_corpse_entry(
            actor,
            command,
            ItemKey::Entity(corpse.identity),
            ItemKey::Instance(source),
        )
        .await;
    let deadline:i64=sqlx::query_scalar("SELECT expires_at_unix_ms FROM game_spell_item_temporal_schedules WHERE item_instance_id=encode($1,'hex')::uuid").bind(source.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(deadline, timer.3, "pickup replay must not reset decay");
    eprintln!("SRV7_DRACOYLE_LOOT_PICKUP_EXECUTED source=9034 original_deadline={deadline}");
    Ok(())
}

#[test]
fn srv7_natural_bear_death_mints_actual_acquisition_operand() -> TestResult {
    super::kill_reward_e2e::scenario_thread::run("srv7-natural-bear", || {
        let schedule =
            oteryn_gameplay_foundation::scheduled_events::Schedule::parse(r#"{"events":[]}"#)
                .map_err(super::fixture::debug)?;
        let _clock =
            oteryn_gameplay_foundation::scheduled_events::test_clock::set(schedule, 1793923200);
        ACTIVE.set(5);
        NATURAL_ITEM.set(0);
        let _guard = Guard;
        super::kill_reward_e2e::scenario()
    })
}

#[test]
fn srv7_natural_chicken_death_mints_actual_acquisition_operand() -> TestResult {
    super::kill_reward_e2e::scenario_thread::run("srv7-natural-chicken", || {
        let schedule =
            oteryn_gameplay_foundation::scheduled_events::Schedule::parse(r#"{"events":[]}"#)
                .map_err(super::fixture::debug)?;
        let _clock =
            oteryn_gameplay_foundation::scheduled_events::test_clock::set(schedule, 1793923200);
        ACTIVE.set(6);
        NATURAL_ITEM.set(0);
        let _guard = Guard;
        super::kill_reward_e2e::scenario()
    })
}

#[test]
fn srv7_natural_hellspawn_death_mints_actual_acquisition_operand() -> TestResult {
    super::kill_reward_e2e::scenario_thread::run("srv7-natural-hellspawn", || {
        let schedule =
            oteryn_gameplay_foundation::scheduled_events::Schedule::parse(r#"{"events":[]}"#)
                .map_err(super::fixture::debug)?;
        let _clock =
            oteryn_gameplay_foundation::scheduled_events::test_clock::set(schedule, 1793923200);
        ACTIVE.set(7);
        NATURAL_ITEM.set(0);
        let _guard = Guard;
        super::kill_reward_e2e::scenario()
    })
}

#[test]
fn srv7_natural_minotaur_death_mints_actual_acquisition_operand() -> TestResult {
    super::kill_reward_e2e::scenario_thread::run("srv7-natural-minotaur", || {
        let schedule =
            oteryn_gameplay_foundation::scheduled_events::Schedule::parse(r#"{"events":[]}"#)
                .map_err(super::fixture::debug)?;
        let _clock =
            oteryn_gameplay_foundation::scheduled_events::test_clock::set(schedule, 1793923200);
        ACTIVE.set(8);
        NATURAL_ITEM.set(0);
        let _guard = Guard;
        super::kill_reward_e2e::scenario()
    })
}

#[test]
fn srv7_natural_rotworm_death_mints_actual_acquisition_operand() -> TestResult {
    super::kill_reward_e2e::scenario_thread::run("srv7-natural-rotworm", || {
        let schedule =
            oteryn_gameplay_foundation::scheduled_events::Schedule::parse(r#"{"events":[]}"#)
                .map_err(super::fixture::debug)?;
        let _clock =
            oteryn_gameplay_foundation::scheduled_events::test_clock::set(schedule, 1793923200);
        ACTIVE.set(9);
        NATURAL_ITEM.set(0);
        let _guard = Guard;
        super::kill_reward_e2e::scenario()
    })
}

#[test]
fn srv7_natural_wasp_death_mints_actual_acquisition_operand() -> TestResult {
    super::kill_reward_e2e::scenario_thread::run("srv7-natural-wasp", || {
        let schedule =
            oteryn_gameplay_foundation::scheduled_events::Schedule::parse(r#"{"events":[]}"#)
                .map_err(super::fixture::debug)?;
        let _clock =
            oteryn_gameplay_foundation::scheduled_events::test_clock::set(schedule, 1793923200);
        ACTIVE.set(10);
        NATURAL_ITEM.set(0);
        let _guard = Guard;
        super::kill_reward_e2e::scenario()
    })
}

#[test]
fn srv7_natural_wolf_death_mints_actual_acquisition_operand() -> TestResult {
    super::kill_reward_e2e::scenario_thread::run("srv7-natural-wolf", || {
        let schedule =
            oteryn_gameplay_foundation::scheduled_events::Schedule::parse(r#"{"events":[]}"#)
                .map_err(super::fixture::debug)?;
        let _clock =
            oteryn_gameplay_foundation::scheduled_events::test_clock::set(schedule, 1793923200);
        ACTIVE.set(11);
        NATURAL_ITEM.set(0);
        let _guard = Guard;
        super::kill_reward_e2e::scenario()
    })
}

#[test]
fn srv7_spike_actual_support_demon_skeleton_death_materializes_source5963_body() -> TestResult {
    super::kill_reward_e2e::scenario_thread::run("srv7-spike-natural-demon-skeleton", || {
        let schedule =
            oteryn_gameplay_foundation::scheduled_events::Schedule::parse(r#"{"events":[]}"#)
                .map_err(super::fixture::debug)?;
        let _clock =
            oteryn_gameplay_foundation::scheduled_events::test_clock::set(schedule, 1793923200);
        ACTIVE.set(12);
        NATURAL_ITEM.set(0);
        let _guard = Guard;
        super::kill_reward_e2e::scenario()
    })
}

#[test]
fn srv7_stampor_actual_source_death_materializes_unchanged12325_body() -> TestResult {
    super::kill_reward_e2e::scenario_thread::run("srv7-natural-stampor", || {
        let schedule =
            oteryn_gameplay_foundation::scheduled_events::Schedule::parse(r#"{"events":[]}"#)
                .map_err(super::fixture::debug)?;
        let _clock =
            oteryn_gameplay_foundation::scheduled_events::test_clock::set(schedule, 1793923200);
        ACTIVE.set(13);
        NATURAL_ITEM.set(0);
        let _guard = Guard;
        super::kill_reward_e2e::scenario()
    })
}
