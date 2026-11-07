//! Source Megalomania native-think cadence; shared white-floor HP/Map owner.
//! Crystal201189f soul_war.lua1193–1212, three native form onThink callbacks.
use super::*;
use oteryn_gameplay_encounters::bosses::goshnars_megalomania::WhiteClock;
use oteryn_gameplay_entities::carrier::soul_war::SoulWarSpawnSource;
use std::sync::Arc;
const PURPLE: &str = "oteryn:creature.goshnar_s_megalomania_purple";
const FORMS: [&str; 3] = [
    PURPLE,
    "oteryn:creature.goshnar_s_megalomania_green",
    "oteryn:creature.goshnar_s_megalomania_blue",
];
#[derive(Debug)]
pub(super) struct MegalomaniaCallbacks {
    boss: ExactActorRef,
    source: Arc<SoulWarSpawnSource>,
    white: WhiteClock,
    last: Option<ThinkOccurrence>,
    revert_ms: Option<u64>,
}
impl MegalomaniaCallbacks {
    pub(super) fn initialize(
        runtime: &mut ChannelRuntimeV1,
        native: &NativeGameplayState,
        room: &QualifiedNativeEntryRoom,
        boss: ExactActorRef,
    ) -> Result<Self, ()> {
        use crate::content::bosses::activity_template::PinnedActivitySourceRoom as Role;
        if room.pinned_activity_source_room() != Some(Role::GoshnarsMegalomania)
            || !qualifies_source(runtime, native)
            || source_actor(runtime, native, boss).as_deref() != Some(PURPLE)
            || runtime
                .read_actor_position(boss)
                .map_err(|_| ())?
                .position()
                != (MovementLocalPosition {
                    x: 33710,
                    y: 31634,
                    floor: 14,
                })
            || runtime.content_pin().frame_binding_digest() != room.frame_binding().digest()
            || runtime.map_revision_digest() != room.map_revision_digest()
            || super::super::soul_war_scene::source_map(room)
                .and_then(|map| map.facts.movement_item_facts(409))
                .is_none()
        {
            return Err(());
        }
        Ok(Self {
            boss,
            source: Arc::new(
                runtime
                    .qualify_soul_war_spawn_source(boss)
                    .map_err(|_| ())?,
            ),
            white: WhiteClock::default(),
            last: None,
            revert_ms: None,
        })
    }
    pub(super) fn next_due_ms(&self) -> Option<u64> {
        self.revert_ms
    }
}
pub(super) async fn drive(
    runtime: &mut ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    room: &QualifiedNativeEntryRoom,
    native: &NativeGameplayState,
    map: &mut CurrentMonsterMap<'_, '_>,
    occurrences: &[ThinkOccurrence],
    now: u64,
) -> Result<(), ()> {
    let Some(mut owner) = states.soul_war_callbacks.megalomania.take() else {
        return Err(());
    };
    let result = async {
        if !qualifies_source(runtime, native)
            || !runtime.soul_war_white_tile_source_current(&owner.source)
            || !map.validate_transaction(runtime).await
        {
            return Err(());
        }
        if owner.revert_ms.is_some_and(|due| due <= now / 1000) {
            white::apply(runtime, states, room, native, &owner.source, now)?;
            owner.revert_ms = None;
        }
        for &occurrence in occurrences {
            if occurrence.actor != owner.boss
                || owner
                    .last
                    .is_some_and(|last| occurrence.sequence <= last.sequence)
            {
                continue;
            }
            let key = source_actor(runtime, native, owner.boss).ok_or(())?;
            if !FORMS.contains(&key.as_str())
                || !runtime
                    .companion_snapshot(owner.boss)
                    .is_ok_and(|boss| boss.health > 0)
            {
                continue;
            }
            let mut next = owner.white.clone();
            let pulse = next.think(1000).ok_or(())?;
            if pulse.warning {
                let scene = super::super::soul_war_scene::source_map(room).ok_or(())?;
                let at = runtime
                    .read_actor_position(owner.boss)
                    .map_err(|_| ())?
                    .position();
                let position = super::super::soul_war_scene::wire_position(at).ok_or(())?;
                let captured_name = native
                    .policies()
                    .policies()
                    .iter()
                    .find(|policy| policy.definition_key == key)
                    .and_then(|policy| {
                        oteryn_protocol_oteryn::entity_names::EntityDisplayName::new(
                            &policy.display_name,
                        )
                        .ok()
                    });
                let _ = scene
                    .facts
                    .emit_movement_speech(crate::map::facts::MovementSpeech {
                        speaker: owner.boss,
                        position,
                        talk_type: 36,
                        text: "FEEL THE POWER OF MY WRATH!!".into(),
                        captured_name,
                    });
            }
            if pulse.create {
                let due = (now / 1000).checked_add(8000).ok_or(())?;
                white::create(
                    runtime,
                    states,
                    room,
                    native,
                    owner.boss,
                    occurrence.sequence,
                )?;
                owner.revert_ms = Some(due);
            }
            owner.white = next;
            owner.last = Some(occurrence);
        }
        Ok(())
    }
    .await;
    states.soul_war_callbacks.megalomania = Some(owner);
    result
}
