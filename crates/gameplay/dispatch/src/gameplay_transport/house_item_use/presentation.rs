//! Publish a fresh durable House result without awaiting under the runtime lock.
use super::super::{actor_spell::ChannelSpellStates, chat_intent::ChatRuntime};
use crate::durability::consume_transform::HouseActionContext;
use crate::durability::house_scope_handoff::HouseId;
use crate::durability::item_transfer::CurrentCharacterItemFence;
use crate::foundation::{ChannelRuntimeV1, ExactActorRef, MovementLocalPosition};
use serde_json::Value;
use tokio::sync::Mutex;

pub(super) struct Publication<'a> {
    pub actor: ExactActorRef,
    pub fence: CurrentCharacterItemFence,
    pub house: &'a HouseId,
    pub effects: &'a Value,
}

pub(super) async fn publish(
    runtime: &Mutex<ChannelRuntimeV1>,
    states: &Mutex<ChannelSpellStates>,
    chat: &ChatRuntime,
    publication: Publication<'_>,
) -> bool {
    let (runtime, mut states) = loop {
        let runtime = runtime.lock().await;
        if let Ok(states) = states.try_lock() {
            break (runtime, states);
        }
        // A spell owner may need Runtime to release its state work. Retain this
        // committed publication while yielding both locks, then retry together.
        drop(runtime);
        drop(states.lock().await);
    };
    let Ok(current) = HouseActionContext::from_native_owner(
        &runtime,
        publication.actor,
        publication.fence.game_session_id,
        publication.fence,
    ) else {
        return false;
    };
    if current.house() != publication.house {
        return false;
    }
    let ids = publication
        .effects
        .get("cues")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(effect_id)
        .collect::<Vec<_>>();
    if let Some(position) = publication.effects.get("ground").and_then(ground) {
        states.publish_house_decoration_visuals(
            &runtime,
            publication.actor,
            publication.fence.game_session_id,
            position,
            &ids,
        );
    }
    // Crystal bed_modification_kits.lua:10-16,37-46 emits at the two changed
    // bed parts (twice at the selected head); preserve every committed cue.
    for cue in publication
        .effects
        .get("extra_cues")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let Some((position, effect)) = cue
            .get("position")
            .and_then(array_position)
            .zip(effect_id(cue))
        else {
            continue;
        };
        states.publish_house_decoration_visuals(
            &runtime,
            publication.actor,
            publication.fence.game_session_id,
            position,
            &[effect],
        );
    }
    if let Some(speech) = publication.effects.get("speech").and_then(Value::as_str) {
        chat.publish_house_decoration_speech(
            &runtime,
            publication.actor,
            publication.fence.game_session_id,
            speech,
        );
    }
    true
}
fn effect_id(cue: &Value) -> Option<u16> {
    u16::try_from(cue.get("effect_id")?.as_u64()?).ok()
}
fn ground(value: &Value) -> Option<MovementLocalPosition> {
    Some(MovementLocalPosition {
        x: i32::try_from(value.get("x")?.as_i64()?).ok()?,
        y: i32::try_from(value.get("y")?.as_i64()?).ok()?,
        floor: i16::try_from(value.get("floor")?.as_i64()?).ok()?,
    })
}
fn array_position(value: &Value) -> Option<MovementLocalPosition> {
    Some(MovementLocalPosition {
        x: i32::try_from(value.get(0)?.as_i64()?).ok()?,
        y: i32::try_from(value.get(1)?.as_i64()?).ok()?,
        floor: i16::try_from(value.get(2)?.as_i64()?).ok()?,
    })
}
#[cfg(test)]
#[path = "presentation_tests.rs"]
mod tests;
