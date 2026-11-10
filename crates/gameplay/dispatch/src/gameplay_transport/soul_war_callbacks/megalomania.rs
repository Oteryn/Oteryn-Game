//! Source Megalomania native-think cadence; shared white-floor HP/Map owner.
//! Crystal201189f soul_war.lua1193–1212, three native form onThink callbacks.
use super::*;
use oteryn_gameplay_encounters::bosses::goshnars_megalomania::WhiteClock;
use oteryn_gameplay_entities::carrier::soul_war::SoulWarSpawnSource;
use std::sync::Arc;
pub(in crate::gameplay_transport) mod phases;
pub(in crate::gameplay_transport) mod sanity;
pub(in crate::gameplay_transport) mod splinter;
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
    phases: phases::Phases,
    last_form: oteryn_gameplay_encounters::bosses::goshnars_megalomania::Form,
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
            phases: phases::Phases::at_birth(),
            last_form: oteryn_gameplay_encounters::bosses::goshnars_megalomania::Form::Purple,
        })
    }
    pub(super) fn next_due_ms(&self) -> Option<u64> {
        self.revert_ms
            .into_iter()
            .chain(self.phases.next_due_ms())
            .min()
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
    if states.soul_war_callbacks.sanity.pending.is_some() {
        return Ok(());
    }
    if splinter::next_due_us(runtime, native)?.is_some_and(|due| due <= now) {
        // The detached timer owner publishes this original XML occurrence
        // before the regular Think pass; no first-Think birth clock is invented.
        return Err(());
    }
    let owner = states.soul_war_callbacks.megalomania.as_ref().ok_or(())?;
    if !qualifies_source(runtime, native)
        || !runtime.soul_war_white_tile_source_current(&owner.source)
        || !map.validate_transaction(runtime).await
    {
        return Err(());
    }
    let needs_clock = occurrences.iter().any(|occurrence| {
        occurrence.actor == owner.boss
            && owner
                .last
                .is_none_or(|last| occurrence.sequence > last.sequence)
    }) && runtime
        .companion_snapshot(owner.boss)
        .is_ok_and(|boss| boss.health > 0);
    let utc = if needs_clock {
        Some(map.source_observed_seconds(runtime).await.ok_or(())?)
    } else {
        None
    };
    // Retained callbacks stay in ChannelSpellStates through every awaited read.
    // The publication below is one synchronous turn and always restores them.
    let mut owner = states.soul_war_callbacks.megalomania.take().ok_or(())?;
    let result = (|| {
        phases::cleanup(runtime, states, &mut owner)?;
        if owner
            .phases
            .next_due_ms()
            .is_some_and(|due| due <= now / 1000)
        {
            // The original timer needs its detached all-role name/Map read;
            // keep this actual Think batch reserved until that callback runs.
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
            let utc = utc.ok_or(())?;
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
            let mut defense_clock = states.soul_war_callbacks.sanity.clock.clone();
            if defense_clock.increase_when_due(utc).is_some() {
                let boss = runtime.companion_snapshot(owner.boss).map_err(|_| ())?;
                let expected = runtime.soul_war_megalomania_drain(&boss).map_err(|_| ())?;
                runtime
                    .increase_soul_war_megalomania(&boss, expected)
                    .map_err(|_| ())?;
            }
            states.soul_war_callbacks.sanity.clock = defense_clock;
            owner.white = next;
            owner.last = Some(occurrence);
        }
        Ok(())
    })();
    states.soul_war_callbacks.megalomania = Some(owner);
    result
}
