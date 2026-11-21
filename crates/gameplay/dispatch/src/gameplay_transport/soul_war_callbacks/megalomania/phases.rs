//! Actual copied Aspect deaths and the existing pure phase planner. Captured
//! positions/actors remain exact; fresh source-name timers use the name owner.
use super::*;
use crate::gameplay_transport::source_names::{SourceNameRole, SourceNamedActor, creature_named};
use crate::movement::monster_map::MonsterMap;
use crate::movement::source_placement::SourcePlacementPlan;
use oteryn_gameplay_encounters::bosses::goshnars_megalomania::{Effect, Form, Megalomania};
use oteryn_gameplay_entities::carrier::native_source_death::SourceCreatureDeathSeal;
use std::collections::{BTreeMap, VecDeque};

mod death;
mod timer;
pub(in crate::gameplay_transport) use death::CopiedDeath;
const NAME: &str = "Goshnar's Megalomania";
const ASPECT: &str = "oteryn:creature.aspect_of_power";
const HOME: MovementLocalPosition = MovementLocalPosition {
    x: 33710,
    y: 31634,
    floor: 14,
};

#[derive(Debug)]
enum Capture {
    Aspect(SourcePlacementPlan),
    Boss(ExactActorRef),
}
#[derive(Debug)]
struct PendingAspect {
    begin: SourceCreatureDeathSeal,
    boss: Option<ExactActorRef>,
    green: bool,
}
#[derive(Debug, Default)]
pub(super) struct Phases {
    clock: Megalomania,
    revision: u64,
    token: u64,
    captures: BTreeMap<u64, Capture>,
    pending: VecDeque<(u64, Effect)>,
    aspect: Option<PendingAspect>,
    completed_aspect: Option<SourceCreatureDeathSeal>,
    cleanup: Option<Vec<ExactActorRef>>,
    disappeared: bool,
}
impl Phases {
    pub(super) fn at_birth() -> Self {
        let mut owner = Self::default();
        owner.clock.spawned();
        owner
    }
    pub(super) fn next_due_ms(&self) -> Option<u64> {
        self.pending
            .front()
            .map(|(due, _)| *due)
            .into_iter()
            .chain(self.clock.next_due_ms())
            .min()
    }
    fn next_token(&mut self, runtime: &ChannelRuntimeV1) -> Result<u64, ()> {
        if self.captures.len() >= runtime.actor_slot_capacity().checked_add(4).ok_or(())? {
            return Err(());
        }
        self.token = self.token.checked_add(1).ok_or(())?;
        Ok(self.token)
    }
    fn changed(&mut self) -> Result<(), ()> {
        self.revision = self.revision.checked_add(1).ok_or(())?;
        Ok(())
    }
}

fn form(key: &str) -> Option<Form> {
    match key {
        PURPLE => Some(Form::Purple),
        "oteryn:creature.goshnar_s_megalomania_green" => Some(Form::Green),
        "oteryn:creature.goshnar_s_megalomania_blue" => Some(Form::Blue),
        _ => None,
    }
}
fn key(form: Form) -> &'static str {
    match form {
        Form::Purple => FORMS[0],
        Form::Green => FORMS[1],
        Form::Blue => FORMS[2],
    }
}
fn speech(
    runtime: &ChannelRuntimeV1,
    room: &QualifiedNativeEntryRoom,
    actor: ExactActorRef,
    text: &str,
) {
    let Some(map) = super::super::super::soul_war_scene::source_map(room) else {
        return;
    };
    let Ok(at) = runtime.read_actor_position(actor) else {
        return;
    };
    let Some(position) = super::super::super::soul_war_scene::wire_position(at.position()) else {
        return;
    };
    let captured_name = runtime
        .companion_snapshot_including_dead(actor)
        .ok()
        .and_then(|s| {
            oteryn_protocol_oteryn::entity_names::EntityDisplayName::new(
                &s.state.policy.display_name,
            )
            .ok()
        });
    let _ = map
        .facts
        .emit_movement_speech(crate::map::facts::MovementSpeech {
            speaker: actor,
            position,
            talk_type: 36,
            text: text.into(),
            captured_name,
        });
}
fn transform(runtime: &mut ChannelRuntimeV1, actor: ExactActorRef, to: Form) -> Result<(), ()> {
    let before = runtime
        .companion_snapshot_including_dead(actor)
        .map_err(|_| ())?;
    let prepared = runtime
        .prepare_soul_war_megalomania_profile_change(&before, key(to))
        .map_err(|_| ())?;
    runtime
        .commit_creature_profile_transformation(prepared)
        .map_err(|_| ())?;
    Ok(())
}
fn named_boss(names: &[SourceNamedActor]) -> Result<Option<&SourceNamedActor>, ()> {
    let named = creature_named(names, NAME)?;
    if named.is_some_and(|named| named.role != SourceNameRole::Creature) {
        return Err(());
    }
    Ok(named)
}
fn draw(
    runtime: &ChannelRuntimeV1,
    boss: ExactActorRef,
    token: u64,
    bound: u32,
    ordinal: &mut u64,
) -> u32 {
    let root = GameplayDecisionRoot::from_bytes(
        Sha256::new()
            .chain_update(b"srx3:megalomania:aspect-respawn:v1")
            .chain_update(runtime.content_pin().server_artifact_digest())
            .chain_update(boss.placement_identity())
            .finalize()
            .into(),
    );
    let mut event = [0; 16];
    event[8..].copy_from_slice(&token.to_be_bytes());
    let limit = u64::MAX - u64::MAX % u64::from(bound);
    for _ in 0..64 {
        let value = deterministic_decision_u64(
            &root,
            DecisionOccurrenceId::from_bytes(event),
            "megalomania_aspect",
            *ordinal,
        );
        *ordinal += 1;
        if let Ok(value) = value
            && value < limit
        {
            return (value % u64::from(bound)) as u32;
        }
    }
    bound
}

fn consume_aspect(
    runtime: &mut ChannelRuntimeV1,
    owner: &mut MegalomaniaCallbacks,
    room: &QualifiedNativeEntryRoom,
    native: &NativeGameplayState,
    begin: &SourceCreatureDeathSeal,
    names: &[SourceNamedActor],
    now: u64,
) -> Result<(), ()> {
    if !begin.current_for(runtime)
        || !qualifies_source(runtime, native)
        || !runtime.soul_war_white_tile_source_current(&owner.source)
    {
        return Err(());
    }
    if owner.phases.completed_aspect.as_ref() == Some(begin) {
        return Ok(());
    }
    if owner.phases.aspect.is_none() {
        let named = named_boss(names)?;
        let current_form = named
            .map(|named| {
                source_actor(runtime, native, named.actor)
                    .and_then(|key| form(&key))
                    .ok_or(())
            })
            .transpose()?;
        if let Some(named) = named {
            runtime
                .assert_actor_spell_unreserved(named.actor)
                .map_err(|_| ())?;
        }
        let position = named.map(|named| named.position.position()).unwrap_or(
            runtime
                .read_actor_position(begin.actor())
                .map_err(|_| ())?
                .position(),
        );
        let token = owner.phases.next_token(runtime)?;
        let mut ordinal = 0;
        let placement = SourcePlacementPlan::prepare(position, false, false, |bound| {
            draw(runtime, owner.boss, token, bound, &mut ordinal)
        })
        .map_err(|_| ())?;
        owner
            .phases
            .captures
            .insert(token, Capture::Aspect(placement));
        let effects = owner
            .phases
            .clock
            .aspect_died(now / 1000, current_form, token);
        owner.phases.aspect = Some(PendingAspect {
            begin: begin.clone(),
            boss: named.map(|named| named.actor),
            green: effects.contains(&Effect::Green),
        });
        owner.phases.changed()?;
    }
    let pending = owner.phases.aspect.as_ref().ok_or(())?;
    if &pending.begin != begin {
        return Err(());
    }
    if pending.green {
        let boss = pending.boss.ok_or(())?;
        transform(runtime, boss, Form::Green)?;
        if boss == owner.boss {
            owner.last_form = Form::Green;
        }
        speech(
            runtime,
            room,
            boss,
            "THE DEATH OF ASPECTS DIMINISHES GOSHNAR'S POWER AND HE TURNS VULNERABLE!",
        );
    }
    owner.phases.completed_aspect = Some(begin.clone());
    owner.phases.aspect = None;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn publish_timer(
    runtime: &mut ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    room: &QualifiedNativeEntryRoom,
    native: &NativeGameplayState,
    map: &MonsterMap,
    prepared: &timer::PreparedTimer,
    names: &[SourceNamedActor],
    now: u64,
) -> Result<(), ()> {
    let prepared = match prepared {
        timer::PreparedTimer::Phase(phase) => phase,
        timer::PreparedTimer::Splinter(plan) => {
            if !map.matches(runtime) || !qualifies_source(runtime, native) {
                return Err(());
            }
            return super::splinter::publish(runtime, native, *plan, now);
        }
    };
    let mut owner = states.soul_war_callbacks.megalomania.take().ok_or(())?;
    let result = (|| {
        if owner.boss != prepared.boss
            || owner.phases.revision != prepared.revision
            || owner.next_due_ms() != Some(prepared.due)
            || !map.matches(runtime)
            || !qualifies_source(runtime, native)
            || !runtime.soul_war_white_tile_source_current(&owner.source)
        {
            return Err(());
        }
        if owner.revert_ms == Some(prepared.due) {
            white::apply(runtime, states, room, native, &owner.source, now)?;
            owner.revert_ms = None;
        }
        if owner.phases.pending.is_empty() {
            for effect in owner.phases.clock.advance(prepared.due) {
                owner.phases.pending.push_back((prepared.due, effect));
            }
        }
        while let Some((_, effect)) = owner.phases.pending.front().copied() {
            match effect {
                Effect::Green => return Err(()),
                Effect::LookupBlue => {
                    if let Some(boss) = named_boss(names)? {
                        let before = runtime
                            .companion_snapshot_including_dead(boss.actor)
                            .map_err(|_| ())?;
                        runtime
                            .prepare_soul_war_megalomania_profile_change(&before, key(Form::Blue))
                            .map_err(|_| ())?;
                        let cell = map
                            .source_placement_cell(
                                runtime,
                                native,
                                &before.state.policy.definition_key,
                                HOME,
                                None,
                            )
                            .ok_or(())?;
                        runtime
                            .assert_actor_spell_unreserved(boss.actor)
                            .map_err(|_| ())?;
                        let token = owner.phases.next_token(runtime)?;
                        if cell.exists {
                            runtime
                                .relocate_soul_war_megalomania_home(&before)
                                .map_err(|_| ())?;
                        }
                        transform(runtime, boss.actor, Form::Blue)?;
                        if boss.actor == owner.boss {
                            owner.last_form = Form::Blue;
                        }
                        speech(
                            runtime,
                            room,
                            boss.actor,
                            "ENOUGH! I WILL MAKE YOU SUFFER FOR YOUR INSOLENCE! NOW - I - WILL - ANIHILATE - YOU!",
                        );
                        owner
                            .phases
                            .captures
                            .insert(token, Capture::Boss(boss.actor));
                        owner.phases.clock.blue_executed(now / 1000, token);
                    }
                }
                Effect::ReturnCapturedPurple(token) => {
                    let Some(Capture::Boss(boss)) = owner.phases.captures.get(&token) else {
                        return Err(());
                    };
                    let boss = *boss;
                    if runtime.source_creature_is_registered(boss).unwrap_or(false) {
                        transform(runtime, boss, Form::Purple)?;
                        if boss == owner.boss {
                            owner.last_form = Form::Purple;
                        }
                    }
                    owner.phases.captures.remove(&token);
                }
                Effect::ReturnIfGreen => {
                    if let Some(boss) = named_boss(names)?
                        && source_actor(runtime, native, boss.actor).as_deref() == Some(FORMS[1])
                    {
                        transform(runtime, boss.actor, Form::Purple)?;
                        if boss.actor == owner.boss {
                            owner.last_form = Form::Purple;
                        }
                        speech(
                            runtime,
                            room,
                            boss.actor,
                            "GOSHNAR REGAINED ENOUGH POWER TO TURN INVULNERABLE AGAIN!",
                        );
                    }
                }
                Effect::RespawnAspect(token) => {
                    let Some(Capture::Aspect(plan)) = owner.phases.captures.get(&token) else {
                        return Err(());
                    };
                    let selected = plan
                        .select(
                            |at| map.source_placement_cell(runtime, native, ASPECT, at, None),
                            |_, _| None,
                        )
                        .map_err(|_| ())?;
                    let mut ordinal = 32;
                    let look = 1303 + draw(runtime, owner.boss, token, 5, &mut ordinal);
                    if look > 1307 {
                        return Err(());
                    }
                    if let Some(at) = selected
                        && let Ok(actor) = runtime.spawn_soul_war_child_from_source(
                            &owner.source,
                            ASPECT,
                            at,
                            |r, p| {
                                map.source_placement_cell(r, native, ASPECT, p, None)
                                    .map(|cell| cell.query_add)
                            },
                        )
                    {
                        let before = runtime.companion_snapshot(actor).map_err(|_| ())?;
                        let mut next = before.state.clone();
                        next.outfit_look_type = look;
                        runtime
                            .compare_companion_state(&before, next)
                            .map_err(|_| ())?;
                    }
                    owner.phases.captures.remove(&token);
                }
            }
            owner.phases.pending.pop_front();
            owner.phases.changed()?;
        }
        Ok(())
    })();
    states.soul_war_callbacks.megalomania = Some(owner);
    result
}

/// Green/Blue alone register this source onDisappear. Capture only this room's
/// current Monsters; Players and later incarnations never join a retained pass.
pub(super) fn cleanup(
    runtime: &mut ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    owner: &mut MegalomaniaCallbacks,
) -> Result<(), ()> {
    if owner.phases.disappeared
        || runtime
            .source_creature_is_registered(owner.boss)
            .unwrap_or(false)
    {
        return Ok(());
    }
    let current = runtime
        .companion_snapshot_including_dead(owner.boss)
        .ok()
        .and_then(|snapshot| form(&snapshot.state.policy.definition_key))
        .unwrap_or(owner.last_form);
    if current == Form::Purple {
        owner.phases.disappeared = true;
        return Ok(());
    }
    if owner.phases.cleanup.is_none() {
        owner.phases.cleanup = Some(runtime.source_registered_creature_census().map_err(|_| ())?.into_iter()
            .filter(|(actor, at)| *actor != owner.boss && super::super::taints::in_boss(
                crate::content::bosses::activity_template::PinnedActivitySourceRoom::GoshnarsMegalomania, at.position()))
            .map(|(actor, _)| actor).collect());
    }
    let cohort = owner.phases.cleanup.as_mut().ok_or(())?;
    let mut index = 0;
    while index < cohort.len() {
        let actor = cohort[index];
        if runtime
            .source_creature_is_registered(actor)
            .unwrap_or(false)
        {
            if !runtime.contains_live_creature(actor) {
                index += 1;
                continue;
            }
            runtime.remove_encounter_actor(actor).map_err(|_| ())?;
            states.monster_melee.creature_ai.remove(actor);
        }
        cohort.remove(index);
    }
    owner.phases.disappeared = cohort.is_empty();
    Ok(())
}
