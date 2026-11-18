//! The final Five Ways fire tiles are driven by literal movement commands.
use super::fixture::debug;
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use oteryn_gameplay_quests::quest::world_callbacks::Position;
use oteryn_protocol_oteryn::world_spatial::{
    StepDirection, StepDisposition, decode_step_result, encode_step_intent,
};

#[test]
fn areadmg_pits_final_field_matches_base_vocation() -> TestResult {
    run("areadmg-pits-matching")
}

#[test]
fn areadmg_pits_final_field_hurts_other_vocations() -> TestResult {
    run("areadmg-pits-other")
}

fn run(name: &'static str) -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e::scenario_thread::run(name, move || {
        super::quest_world_callbacks::scenario(
            name,
            Position {
                x: 100,
                y: 100,
                floor: -7,
            },
            false,
        )
    })
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    matching: bool,
) -> TestResult {
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let session = admitted.game_session_id;
    let before = {
        let runtime = owner.runtime.lock().await;
        let mut states = owner.states.spell_states.lock().await;
        let state = states
            .get_mut(&runtime, actor, session)
            .ok_or("player vitals")?;
        use crate::content::{ReferenceRationalPercent, ReferenceResistanceKind};
        state.install_equipped_element_protection(
            ReferenceResistanceKind::Fire,
            Some(ReferenceRationalPercent {
                numerator: 10,
                denominator: 1,
            }),
        );
        state.install_equipped_element_protection(
            ReferenceResistanceKind::Physical,
            Some(ReferenceRationalPercent {
                numerator: 20,
                denominator: 1,
            }),
        );
        assert_eq!(
            state.character_facts().vocation,
            crate::spell::Vocation::ExaltedMonk
        );
        state.vitals().health
    };
    let reply = super::quest_world_callbacks::wire(
        owner,
        admitted,
        1,
        1,
        encode_step_intent(StepDirection::East),
    )
    .await?;
    assert_eq!(
        decode_step_result(&reply).map_err(debug)?,
        StepDisposition::Moved
    );
    let runtime = owner.runtime.lock().await;
    let states = owner.states.spell_states.lock().await;
    let after = states
        .get(&runtime, actor, session)
        .ok_or("player vitals after step")?
        .vitals()
        .health;
    assert_eq!(after, before - if matching { 270 } else { 5760 });
    Ok(())
}
