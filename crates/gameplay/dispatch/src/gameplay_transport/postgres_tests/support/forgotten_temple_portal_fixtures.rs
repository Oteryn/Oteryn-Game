//! Crystal 201189f8, forgotten_knowledge/movements_entrance_teleport.lua:12-20,100-119.
use super::{
    AdmittedSession, ComposedFreshAdmission, TestResult, fixture::Harness, fixture::debug,
};
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::foundation::{CommandId, CommandRef};
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Position};
use oteryn_gameplay_quests::quest::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTrack,
    QuestTransition, authored_bindings::Bindings,
};
use oteryn_protocol_oteryn::world_spatial::{StepDirection, StepDisposition};
use serde_json::json;
use sha2::{Digest, Sha256};

pub(super) const CASE: &str = "forgotten-knowledge-temple-energy-portal";
const QUEST: &str = "oteryn:quest.forgotten_knowledge_quest";
const ACCESS: &str = "oteryn:quest-progress/quest/u11_02/forgotten_knowledge/access_violet";
const FIXTURE: &str = "oteryn:quest-transition/forgotten/fixture-energy-offering";
const KEY: &str = "oteryn:quest-callback/forgotten/energy-temple-portal";
const SOURCE: &str = "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/forgotten_knowledge/movements_entrance_teleport.lua:12-20,100-125";
// Crystal data-global/world/world.otbm places appearance 25051 here.
pub(super) const PORTAL: Position = Position {
    x: 32328,
    y: 32087,
    floor: -7,
};
const LANDING: Position = Position {
    x: 32786,
    y: 32820,
    floor: -13,
};
const REFUSAL: Position = Position {
    y: PORTAL.y + 2,
    ..PORTAL
};
const PURPLE: u16 = 48; // Crystal src/utils/utils_definitions.hpp:103.
const TELEPORT: u16 = 11; // Crystal src/utils/utils_definitions.hpp:66.

pub(super) fn callback() -> TestResult<Callback> {
    let document = json!({"schema":1,"callbacks":[{
        "key":KEY,"quest":QUEST,"trigger":"step_in",
        "selector":{"appearance":25051,"position":PORTAL},"reversible":false,"passages":[],"offerings":[],
        "storage":[{"track":ACCESS,"minimum":1,"maximum":1,"set":1,"read_only":true}],
        "teleport":LANDING,"return_on_failure":true,
        "failure_message":"You don't have the permission to use this portal.",
        "effects":{"teleport_effect":TELEPORT,"teleport_effect_on_source":PURPLE,
            "refusal_step_from_source":{"direction":"south","distance":2},
            "refusal_effect_on_landing":TELEPORT,"refusal_effect_on_source":TELEPORT},
        "sources":[SOURCE]
    }]});
    let [callback] = oteryn_gameplay_quests::quest::world_callbacks::parse(&document.to_string())?
        .try_into()
        .map_err(|_| "one temple callback")?;
    Ok(callback)
}

pub(super) fn world(callback: &Callback) -> TestResult<crate::map::boot::BundleWorld> {
    super::quest_world_callbacks_fixtures::world_with_cells(callback, PORTAL, &[REFUSAL])
}

pub(super) fn catalogue(callback: &Callback) -> TestResult<QuestStateCatalogue> {
    let document = json!({"schema":1,"quests":[{
        "quest":QUEST,"recipe_sha256":format!("{:x}",Sha256::digest(SOURCE)),
        "sources":[SOURCE],"tracks":[{"key":ACCESS,"initial":0,"minimum":0,"maximum":1}],
        "stages":[{"key":"temple-energy","completes":false,"trigger":{"kind":"step_in","callback":callback}}]
    }]});
    let base = QuestStateCatalogue::new(
        "content-1",
        vec![QuestTrack {
            key: ACCESS.into(),
            quest: QUEST.into(),
            initial: 0,
            min: 0,
            max: 1,
        }],
        vec![QuestTransition {
            key: FIXTURE.into(),
            quest: QUEST.into(),
            completes: false,
            experience: None,
            world_change: None,
            effects: vec![QuestEffect {
                track: ACCESS.into(),
                from: QuestComparison::Any,
                effect: QuestEffectKind::Set(1),
            }],
        }],
    )
    .map_err(debug)?;
    Ok(Bindings::parse(&serde_json::to_vec(&document)?)?.attach(&base)?)
}

pub(super) async fn exercise(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    callback: Callback,
) -> TestResult {
    owner
        .states
        .quest_callbacks
        .fixture
        .set(vec![callback])
        .map_err(|_| "callback installed")?;
    let session = admitted.game_session_id;
    let fence = owner
        .current_quest_fence(session)
        .await
        .map_err(|()| "fixture fence")?
        .ok_or("fixture fence")?;
    let catalogue = owner.quest_catalogue.as_ref().ok_or("quest catalogue")?;
    let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
    let seeded = slot
        .commit_quest_transition(
            owner.root,
            owner.character,
            owner.holder,
            fence,
            QuestTransitionRequest {
                transition_key: FIXTURE.into(),
                cause: QuestCause::Command(CommandRef::new(
                    session,
                    CommandId::new(1000).map_err(debug)?,
                )),
            },
            std::sync::Arc::clone(catalogue),
        )
        .await
        .map_err(debug)?;
    assert!(matches!(seeded, QuestTransitionOutcome::Committed(_)));
    drop(slot);
    let peer = super::ferumbras_crystal_reset_fixtures::admit_peer(
        owner,
        h,
        Position {
            y: PORTAL.y - 1,
            ..PORTAL
        },
    )
    .await?;
    let peer_session = {
        let runtime = owner.runtime.lock().await;
        runtime
            .positioned_melee_census()
            .map_err(debug)?
            .into_iter()
            .find_map(|(actor, _, session)| (actor == peer).then_some(session).flatten())
            .ok_or("peer session")?
    };
    super::questpc7_ferumbras_fixtures::step(owner, admitted, StepDirection::East).await?;
    let (reply, _) = owner
        .ordinary_paced_step(peer, peer_session, StepDirection::South, None, None)
        .await;
    assert_eq!(reply.disposition, StepDisposition::Moved);
    let runtime = owner.runtime.lock().await;
    for (actor, at) in [
        (admitted.runtime_actor.ok_or("player")?, LANDING),
        (peer, REFUSAL),
    ] {
        assert_eq!(
            runtime
                .read_actor_position(actor)
                .map_err(debug)?
                .position(),
            super::quest_world_callbacks_fixtures::local(at)
        );
    }
    let map = owner.bundle_map().ok_or("temple map")?;
    let pairs: Vec<_> = map
        .facts
        .movement_effects_since(0)
        .into_iter()
        .filter(|(_, _, effect)| [PURPLE, TELEPORT].contains(effect))
        .map(|(_, at, cue)| {
            (
                Position {
                    x: at.x,
                    y: at.y,
                    floor: at.floor,
                },
                cue,
            )
        })
        .collect();
    assert_eq!(
        pairs,
        [
            (PORTAL, PURPLE),
            (LANDING, TELEPORT),
            (REFUSAL, TELEPORT),
            (PORTAL, TELEPORT)
        ]
    );
    Ok(())
}
