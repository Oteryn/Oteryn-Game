//! Crystal 201189f8 dark_cathedral_teleports.lua:1-55.
use super::{
    AdmittedSession, ComposedFreshAdmission, TestResult,
    annihilator_route::{step, use_target},
    fixture::debug,
    l6_shared_gate_native::relocate,
    quest_demonrage::{appearance, target},
    quest_world_callbacks::local,
};
use crate::map::boot::BundleMap;
use oteryn_gameplay_quests::quest::world_callbacks::{self, Callback, Position};
use oteryn_protocol_oteryn::{world_object::UseDisposition, world_spatial::StepDirection};
use serde_json::json;
use std::sync::atomic::Ordering;

const QUEST: &str = "oteryn:quest.the_thieves_guild_quest";
const SOURCE: &str = "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/movements/teleport/dark_cathedral_teleports.lua:1-55";
pub(super) const START: Position = at(32615, 32483, -9);
const PORTAL: Position = at(32615, 32484, -9);
const LANDING: Position = at(32615, 32485, -10);
const RETURN_PORTAL: Position = at(32615, 32484, -10);
const RETURN_LANDING: Position = at(32615, 32482, -9);
const LEVERS: [Position; 6] = [
    at(32645, 32465, -9),
    at(32647, 32483, -9),
    at(32619, 32523, -9),
    at(32610, 32523, -9),
    at(32583, 32482, -9),
    at(32584, 32465, -9),
];

const fn at(x: u16, y: u16, floor: i8) -> Position {
    Position { x, y, floor }
}

fn source_bindings() -> TestResult<Vec<Callback>> {
    let mut callbacks = Vec::new();
    for (index, lever) in LEVERS.iter().enumerate() {
        callbacks.push(serde_json::from_value(json!({
            "key": format!("oteryn:quest-callback/cathedral-switch-{}", index + 1),
            "quest": QUEST,
            "trigger": "use",
            "selector": {"native":{"position":lever,"ordinal":1,"appearance":2772}},
            "lever": [2772,2773], "reversible": false, "return_on_failure": false,
            "passages": [], "storage": [], "offerings": [],
            "effects": {"reset_after_ms":600_000,"active_lever_speech":"It doesn't move."},
            "sources": [SOURCE]
        }))?);
    }
    let requirements: Vec<_> = LEVERS
        .iter()
        .map(|lever| json!({"position":lever,"appearance":2773}))
        .collect();
    for (name, portal, landing, requirements) in [
        ("entrance", PORTAL, LANDING, requirements),
        ("return", RETURN_PORTAL, RETURN_LANDING, Vec::new()),
    ] {
        callbacks.push(serde_json::from_value(json!({
            "key": format!("oteryn:quest-callback/cathedral-switch-{name}"),
            "quest": QUEST,
            "trigger": "step_in",
            "selector": {"native":{"position":portal,"ordinal":1,"appearance":1949}},
            "reversible": false, "return_on_failure": true,
            "passages": [], "storage": [], "offerings": [],
            "world_requirements": requirements, "teleport": landing,
            "effects": {"native_portal_destination":landing,"teleport_effect":11},
            "sources": [SOURCE]
        }))?);
    }
    Ok(callbacks)
}

#[test]
fn cathedral_portal_requires_all_six_switches_and_closes_when_the_first_expires() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::annihilator_route::cathedral_switches(admin)
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    member: &mut AdmittedSession,
    map: &BundleMap,
) -> TestResult {
    let mut source = source_bindings()?;
    // Another legal StepIn on the same tile cannot expose the raw teleport.
    // This fixture-only cue selects the retained ground beneath the portal.
    source.push(serde_json::from_value(json!({
        "key": "oteryn:quest-callback/cathedral-test-ground-cue",
        "quest": QUEST,
        "trigger": "step_in",
        "selector": {"native":{"position":PORTAL,"ordinal":0,"appearance":410}},
        "reversible": false, "return_on_failure": false,
        "passages": [], "storage": [], "offerings": [],
        "effects": {"magic":{"position":PORTAL,"effect":11}},
        "sources": [SOURCE]
    }))?);
    let bindings = world_callbacks::load_with_source_fixture(&source)?;
    assert!(owner.states.quest_callbacks.fixture.set(bindings).is_ok());
    let actor = member.runtime_actor.ok_or("Cathedral switch operator")?;
    let session = member.game_session_id;
    async fn position(
        owner: &ComposedFreshAdmission<'_, '_, '_>,
        actor: crate::foundation::ExactActorRef,
    ) -> Result<crate::foundation::MovementLocalPosition, String> {
        Ok(owner
            .runtime
            .lock()
            .await
            .read_actor_position(actor)
            .map_err(debug)?
            .position())
    }

    step(owner, member, StepDirection::South).await?;
    assert_eq!(position(owner, actor).await?, local(START));

    for (index, lever) in LEVERS.into_iter().enumerate() {
        let now = 100_000_001 + u64::try_from(index)? * 1_000_000;
        owner.states.test_owner_time.store(now, Ordering::Relaxed);
        relocate(owner, actor, session, (lever.x, lever.y - 1, 9)).await?;
        assert_eq!(
            use_target(owner, member, &target(map, lever)?).await?,
            UseDisposition::Committed
        );
        assert_eq!(appearance(map, lever)?, 2773);
        if index == 0 {
            let opened = target(map, lever)?;
            owner.states.test_owner_time.store(now + 500_000, Ordering::Relaxed);
            assert_eq!(use_target(owner, member, &opened).await?, UseDisposition::Committed);
            assert_eq!(target(map, lever)?.expected_revision, opened.expected_revision);
        }
    }
    // Run admission just before the first lever's deadline.
    owner.states.test_owner_time.store(700_000_000, Ordering::Relaxed);
    relocate(owner, actor, session, (START.x, START.y, 9)).await?;
    step(owner, member, StepDirection::South).await?;
    assert_eq!(position(owner, actor).await?, local(LANDING));
    step(owner, member, StepDirection::North).await?;
    assert_eq!(position(owner, actor).await?, local(RETURN_LANDING));

    // The scheduler has not projected the reset. Native StepIn must enforce
    // the deadline even though the first lever still looks open.
    relocate(owner, actor, session, (START.x, START.y, 9)).await?;
    owner.states.test_owner_time.store(700_000_001, Ordering::Relaxed);
    assert_eq!(appearance(map, LEVERS[0])?, 2773);
    step(owner, member, StepDirection::South).await?;
    assert_eq!(position(owner, actor).await?, local(START));
    assert_eq!(appearance(map, LEVERS[0])?, 2772);
    for lever in &LEVERS[1..] {
        assert_eq!(appearance(map, *lever)?, 2773);
    }
    Ok(())
}
