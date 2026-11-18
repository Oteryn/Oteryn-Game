use super::underwater_drowning::{QuestStepCase, health, quest_step_scenario, step};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult, fixture::debug};
use crate::content::{ReferenceRationalPercent, ReferenceResistanceKind, spell_data::Vocation};
use crate::foundation::{ExactActorRef, GameSessionId};
use oteryn_gameplay_quests::quest::world_callbacks::Callback;
use oteryn_protocol_oteryn::world_spatial::StepDirection;

#[test]
fn five_ways_step_in_uses_base_vocation_and_final_field_element_protection() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else { return Ok(()) };
    drop(admin);
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| quest_step_scenario(QuestStepCase::FiveWays).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "Five Ways PostgreSQL fixture panicked")?
        .map_err(Into::into)
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    mover: (AdmittedSession, ExactActorRef, GameSessionId),
) -> TestResult {
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
    step(owner, mover, StepDirection::East, 1).await?;
    assert_eq!(health(owner, actor, session).await?, after_fire);
    step(owner, mover, StepDirection::East, 2).await?;
    let after_physical = health(owner, actor, session).await?;
    assert!((31..=72).contains(&after_fire.saturating_sub(after_physical)));
    let events = owner.bundle_map().ok_or("map")?.facts.movement_effects_since(0);
    assert!(events.iter().any(|(_, position, effect)| position.x == 0 && *effect == 16));
    assert!(events.iter().any(|(_, position, effect)| position.x == 2 && *effect == 37));
    Ok(())
}
