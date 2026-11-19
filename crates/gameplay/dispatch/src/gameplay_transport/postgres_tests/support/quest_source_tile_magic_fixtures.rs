//! Crystal 201189f8 data-global/scripts/quests/unnatural_selection/actions_mission5_ray_of_light.lua:2-12.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::{unnatural_selection, world_callbacks::Position};
use oteryn_protocol_oteryn::{
    chat::{ChatLine, ChatSpeechMode},
    world_object::{UseDisposition, WorldObjectTarget},
};

pub(super) const NAME: &str = "unnatural-selection-ray-of-light";
const HOLY_AREA: u16 = 50;
const SPEECH: &str = "A ray of sunlight comes down from the heaven and hits the crystal. Wow. That probably means Fasuon is supporting.";

pub(super) fn effects()
-> TestResult<oteryn_gameplay_quests::quest::world_callbacks::CallbackEffectSet> {
    Ok(serde_json::from_value(serde_json::json!({
        "source_magic": HOLY_AREA,
        "player_speech": SPEECH,
    }))?)
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    trigger: Position,
) -> TestResult {
    let session = admitted.game_session_id;
    let actor = admitted.runtime_actor.ok_or("player actor")?;
    owner
        .states
        .chat
        .admit(session, "Fixture Hero", owner.owner_now());

    let refused = use_crystal(owner, admitted, trigger, 1).await?;
    assert_ne!(refused, UseDisposition::Committed);
    let map = owner.bundle_map().ok_or("bundle map")?;
    assert!(map.facts.movement_effects_since(0).is_empty());
    assert!(owner.states.chat.drain(session).is_empty());

    let mission = "oteryn:quest-progress/quest/u8_54/unnatural_selection/mission05";
    super::qengine_outfit_stories_fixtures::seed_quest_prerequisites(
        h,
        owner,
        admitted,
        &[(mission.into(), 1)],
        126,
    )
    .await?;
    let used = use_crystal(owner, admitted, trigger, 2).await?;
    assert_eq!(used, UseDisposition::Committed);
    assert_eq!(
        super::m39_bone_journey_fixtures::stage(h, mission).await?,
        Some(2)
    );
    assert_eq!(
        super::m39_bone_journey_fixtures::stage(h, unnatural_selection::QUESTLINE_TRACK).await?,
        Some(11),
    );
    let map = owner.bundle_map().ok_or("bundle map")?;
    let effects = map.facts.movement_effects_since(0);
    let target = crate::map::overlay::TilePos {
        x: trigger.x,
        y: trigger.y,
        floor: trigger.floor,
    };
    let [(_, position, effect)] = effects.as_slice() else {
        return Err(format!("expected one holy-area occurrence, got {effects:?}").into());
    };
    assert_eq!((*position, *effect), (target, HOLY_AREA));
    let lines = owner.states.chat.drain(session);
    assert!(
        matches!(
            lines.as_slice(),
            [ChatLine::Local { mode: ChatSpeechMode::MonsterSay, speaker, text, .. }]
                if speaker.identity == actor.placement_identity() && text == SPEECH
        ),
        "{lines:?}"
    );
    let again = use_crystal(owner, admitted, trigger, 3).await?;
    assert_ne!(again, UseDisposition::Committed);
    assert_eq!(
        owner
            .bundle_map()
            .ok_or("bundle map")?
            .facts
            .movement_effects_since(0),
        effects
    );
    assert!(owner.states.chat.drain(session).is_empty());
    Ok(())
}

async fn use_crystal(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    trigger: Position,
    command_id: u64,
) -> TestResult<UseDisposition> {
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    let map = owner.bundle_map().ok_or("bundle map")?;
    let key = oteryn_world_bundle::bundle::placement_key(trigger.floor, trigger.x, trigger.y, 1)
        .ok_or("crystal placement")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(debug)?
        .ok_or("quest fence")?;
    Ok(owner
        .use_object(
            admitted.runtime_actor.ok_or("player actor")?,
            UseCommand {
                game_session_id: admitted.game_session_id,
                command_id,
                item_fence: Some(fence.item_fence()),
            },
            WorldObjectTarget {
                placement,
                expected_revision: map.facts.object_revision(key),
            },
        )
        .await
        .disposition)
}
