use super::underwater_drowning::{QuestStepCase, health, quest_step_scenario, step};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult, fixture::{Harness, debug, id}};
use crate::content::{ReferenceRationalPercent, ReferenceResistanceKind, spell_data::Vocation};
use crate::foundation::{ExactActorRef, GameSessionId};
use oteryn_gameplay_quests::quest::world_callbacks::Callback;
use oteryn_protocol_oteryn::world_spatial::StepDirection;

#[test]
fn five_ways_step_in_uses_base_vocation_and_final_field_element_protection() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else { return Ok(()) };
    drop(admin);
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| quest_step_scenario(QuestStepCase::FiveWays).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "Five Ways PostgreSQL fixture panicked")?
        .map_err(Into::into)
}

#[test]
fn five_ways_final_field_spends_one_ordinary_equipped_protection_charge_per_hit() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else { return Ok(()) };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run("five ways equipped protection", || {
        quest_step_scenario(QuestStepCase::FiveWaysCharged)
    })
}

async fn equip_protector(owner: &ComposedFreshAdmission<'_, '_, '_>, h: &Harness) -> TestResult {
    // Crystal items.xml:7264-7273 gives the equipped amulet five charges
    // and 80% physical absorption. Its protection does not cover fire.
    let definition = &owner.active_generation
        .and_then(|active| active.native_gameplay())
        .and_then(|native| native.item_policy_for_source_id(3081))
        .ok_or("stone skin amulet policy")?
        .record().production_definition;
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item',$3,$4,1,1,encode($5,'hex')::uuid)")
        .bind(id(180).as_slice()).bind(id(42).as_slice()).bind(&definition.production_key)
        .bind(&definition.revision_ref).bind(id(181).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_character_equipment_slots(character_id,slot,item_instance_id,world_id,placed_transaction_id) VALUES(encode($1,'hex')::uuid,7,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid)")
        .bind(id(41).as_slice()).bind(id(180).as_slice()).bind(id(42).as_slice())
        .bind(id(182).as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

async fn charges(h: &Harness) -> TestResult<Option<i32>> {
    Ok(sqlx::query_scalar("SELECT charges FROM game_item_timed_states WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(id(180).as_slice()).fetch_optional(&h.pool).await?)
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    mover: (AdmittedSession, ExactActorRef, GameSessionId),
    h: &Harness,
    charged: bool,
) -> TestResult {
    if charged {
        equip_protector(owner, h).await?;
    }
    let (_, actor, session) = mover;
    let (matching, other) = {
        let runtime = owner.runtime.lock().await;
        let mut states = owner.states.spell_states.lock().await;
        let state = states.get_mut(&runtime, actor, session).ok_or("spell state")?;
        state.install_equipped_element_protection(
            ReferenceResistanceKind::Fire,
            Some(ReferenceRationalPercent::new(90, 1).map_err(debug)?),
        );
        state.install_equipped_element_protection(
            ReferenceResistanceKind::Physical,
            Some(ReferenceRationalPercent::new(99, 1).map_err(debug)?),
        );
        match state.source_party_vocation() {
            Vocation::Sorcerer | Vocation::MasterSorcerer => ("sorcerer", "druid"),
            Vocation::Druid | Vocation::ElderDruid => ("druid", "sorcerer"),
            Vocation::Paladin | Vocation::RoyalPaladin => ("paladin", "sorcerer"),
            Vocation::Knight | Vocation::EliteKnight => ("knight", "sorcerer"),
            Vocation::Monk | Vocation::ExaltedMonk => ("monk", "sorcerer"),
            Vocation::None => return Err("fixture must have a base vocation".into()),
        }
    };
    // Crystal 201189f8 data-global/scripts/quests/the_pits_of_inferno_quest/movements_fire_tile.lua:1-31,47-54.
    let callback = |x, action, vocation| -> TestResult<Callback> {
        Ok(serde_json::from_value(serde_json::json!({
            "key": format!("oteryn:quest-callback/five-ways-fixture-{x}"),
            "quest": "oteryn:quest.pits_of_inferno",
            "trigger": "step_in",
            "selector": {"position":{"x":x,"y":0,"floor":-7},"ordinal":0,"action":action},
            "reversible":false, "passages":[], "storage":[], "offerings":[], "return_on_failure":false,
            "effects":{"vocation_step_damage":{
                "vocation":vocation,
                "matching":{"kind":"fire","amount":300,"effect":16},
                "other":{"kind":"physical","amount":7200,"effect":37}
            }},
            "sources":["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_pits_of_inferno_quest/movements_fire_tile.lua:1-65"]
        }))?)
    };
    // The two fixture cells each carry exactly one matching placement.
    assert!(owner.states.quest_callbacks.fixture.set(vec![callback(0, 2045, matching)?, callback(2, 2051, other)?]).is_ok());
    let before = health(owner, actor, session).await?;
    step(owner, mover, StepDirection::West, 0).await?;
    let after_fire = health(owner, actor, session).await?;
    assert!((1..=30).contains(&before.saturating_sub(after_fire)));
    if charged {
        assert_eq!(charges(h).await?, None, "fire leaves the physical amulet untouched");
    }
    step(owner, mover, StepDirection::East, 1).await?;
    assert_eq!(health(owner, actor, session).await?, after_fire);
    step(owner, mover, StepDirection::East, 2).await?;
    let after_physical = health(owner, actor, session).await?;
    if charged {
        assert!((1..=15).contains(&after_fire.saturating_sub(after_physical)));
        assert_eq!(charges(h).await?, Some(4));
        step(owner, mover, StepDirection::West, 1).await?;
        assert_eq!(charges(h).await?, Some(4));
        step(owner, mover, StepDirection::East, 2).await?;
        assert_eq!(charges(h).await?, Some(3));
    } else {
        assert!((31..=72).contains(&after_fire.saturating_sub(after_physical)));
    }
    let events = owner.bundle_map().ok_or("map")?.facts.movement_effects_since(0);
    assert!(events.iter().any(|(_, position, effect)| position.x == 0 && *effect == 16));
    assert!(events.iter().any(|(_, position, effect)| position.x == 2 && *effect == 37));
    Ok(())
}
