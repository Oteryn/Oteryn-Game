//! Crystal 201189f8 movements_mission3_dance_dance_evolution.lua:33-60.
use super::TestResult;
use oteryn_gameplay_quests::quest::{unnatural_selection as dance, world_callbacks::Position};
use oteryn_protocol_oteryn::chat::{ChatLine, ChatSpeechMode};

pub(super) const NAME: &str = "unnatural-selection-dance-hints";
const PLANTS: [u16; 2] = [46, 47];

#[test]
fn quest_step_in_hints_follow_committed_progress_and_reset() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run("quest-progress-magic", || {
        super::quest_world_callbacks::scenario(
            NAME,
            Position {
                x: 32991,
                y: 31496,
                floor: -1,
            },
            false,
        )
    })
}

pub(super) fn effects()
-> TestResult<oteryn_gameplay_quests::quest::world_callbacks::CallbackEffectSet> {
    let stages = dance::SEQUENCE
        .into_iter()
        .enumerate()
        .map(|(index, tile)| ((index + 1).to_string(), position(tile)))
        .collect::<std::collections::BTreeMap<_, _>>();
    Ok(serde_json::from_value(serde_json::json!({
        "progress_magic": {
            "track": dance::DANCE_STATUS_TRACK,
            "stages": stages,
            "effects": PLANTS
        }
    }))?)
}

fn position(tile: u8) -> Position {
    let (x, y, floor) = dance::TILES[usize::from(tile - 1)];
    Position { x, y, floor }
}

pub(super) fn assert_hint(
    owner: &super::ComposedFreshAdmission<'_, '_, '_>,
    cursor: &mut u64,
    next: Option<u8>,
) -> TestResult {
    let map = owner.bundle_map().ok_or("dance map")?;
    let effects = map.facts.movement_effects_since(*cursor);
    if let Some(tile) = next {
        let Position { x, y, floor } = position(tile);
        let target = crate::map::overlay::TilePos { x, y, floor };
        assert_eq!(
            effects
                .iter()
                .map(|(_, at, effect)| (*at, *effect))
                .collect::<Vec<_>>(),
            [(target, 46), (target, 47)],
        );
        *cursor = effects.last().ok_or("dance hint")?.0;
    } else {
        assert!(effects.is_empty(), "{effects:?}");
    }
    Ok(())
}

pub(super) fn assert_speech(
    owner: &super::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &super::AdmittedSession,
    expected: Option<&str>,
) -> TestResult {
    let lines = owner.states.chat.drain(admitted.game_session_id);
    if let Some(expected) = expected {
        let actor = admitted.runtime_actor.ok_or("dance actor")?;
        assert!(matches!(
            lines.as_slice(),
            [ChatLine::Local { speaker, mode: ChatSpeechMode::MonsterSay, text, .. }]
                if speaker.identity == actor.placement_identity() && text == expected
        ), "{lines:?}");
    } else {
        assert!(lines.is_empty(), "{lines:?}");
    }
    Ok(())
}
