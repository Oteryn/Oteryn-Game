//! The four restored source proposals consume the existing AI clock, never a second clock.
use super::super::soul_war_scene as scene;
use super::*;
use crate::content::bosses::activity_template::PinnedActivitySourceRoom as Room;
use crate::content::native_soul_war::QualifiedSoulWarAttackKind as Kind;
use crate::creature_auto_attack::ProfileAbilityProposal;
use crate::foundation::{GameSessionId, RuntimeScopeRefV1, ScopeOwnershipGeneration};
use crate::player_lethal::PlayerLethalVitals;
use std::sync::Arc;

mod area;
pub(in crate::gameplay_transport) use area::PreparedSoulWarArea;

type AttackOwnerBinding = (
    RuntimeScopeRefV1,
    ScopeOwnershipGeneration,
    [u8; 32],
    [u8; 32],
    [u8; 32],
    [u8; 32],
    u64,
);
fn binding(runtime: &ChannelRuntimeV1) -> AttackOwnerBinding {
    (
        runtime.binding().runtime_scope(),
        runtime.binding().scope_generation(),
        runtime.content_pin().server_artifact_digest(),
        runtime.content_pin().client_artifact_digest(),
        runtime.content_pin().frame_binding_digest(),
        runtime.map_revision_digest(),
        runtime.content_pin().activation_sequence(),
    )
}

#[derive(Debug)]
pub(in crate::gameplay_transport) struct RetainedSoulWarAttack {
    binding: AttackOwnerBinding,
    stamp: crate::foundation::RuntimeWorkStamp,
    issuer: ExactActorRef,
    occurrence: String,
    kind: Kind,
    observed_at_us: u64,
}
impl RetainedSoulWarAttack {
    fn current(&self, runtime: &ChannelRuntimeV1, native: &NativeGameplayState) -> bool {
        self.binding == binding(runtime)
            && native.source_digest() == runtime.content_pin().server_artifact_digest()
            && runtime
                .owner_fence()
                .is_ok_and(|fence| fence.accepts_stamp(self.stamp))
    }
}

#[derive(Debug)]
struct Scheduled {
    source: Arc<RetainedSoulWarAttack>,
    original_due: u64,
    due: u64,
    target: Option<(ExactActorRef, GameSessionId)>,
    telegraph: Option<MovementLocalPosition>,
    geometry: Option<area::Geometry>,
    area: Option<PreparedSoulWarArea>,
}
impl Scheduled {
    fn next_due(&self) -> u64 {
        if self.telegraph.is_some() {
            self.original_due
        } else {
            self.due
        }
    }
}

#[derive(Debug, Default)]
pub(super) struct SoulWarAttacks {
    binding: Option<AttackOwnerBinding>,
    scheduled: Vec<Scheduled>,
    seen: Vec<(ExactActorRef, String)>,
    #[cfg(test)]
    pause_after_player_commit: bool,
}
impl SoulWarAttacks {
    pub(super) fn next_due(&self) -> Option<u64> {
        self.scheduled.iter().map(Scheduled::next_due).min()
    }
}
fn in_zone(kind: Kind, p: MovementLocalPosition) -> bool {
    match kind {
        Kind::CrueltyElemental => {
            p.floor == 7 && (33847..=33864).contains(&p.x) && (31858..=31874).contains(&p.y)
        }
        _ => p.floor == 14 && (33701..=33719).contains(&p.x) && (31626..=31642).contains(&p.y),
    }
}
fn shape(blue: bool) -> Vec<(i32, i32)> {
    if blue {
        // Lua spell-megalomania_blue.lua:1-17 has centre3 at(row7,col8).
        (-7..=7)
            .flat_map(|y| (-8..=8).filter_map(move |x| (x == 0 || y == 0).then_some((x, y))))
            .collect()
    } else {
        const WIDTHS: [i32; 13] = [0, 2, 3, 4, 5, 5, 6, 5, 5, 4, 3, 2, 0];
        WIDTHS
            .into_iter()
            .enumerate()
            .flat_map(|(row, width)| (-width..=width).map(move |x| (x, row as i32 - 6)))
            .collect()
    }
}
pub(super) fn draw(
    root: [u8; 32],
    issuer: ExactActorRef,
    occurrence: &str,
    ordinal: u64,
    bound: u32,
) -> Option<u32> {
    if bound == 0 {
        return None;
    }
    let root = GameplayDecisionRoot::from_bytes(
        Sha256::new()
            .chain_update(b"srx3:soul-war-source-attack:v1")
            .chain_update(root)
            .chain_update(issuer.placement_identity())
            .chain_update(occurrence.as_bytes())
            .finalize()
            .into(),
    );
    let mut id = [0; 16];
    id[..8].copy_from_slice(&ordinal.to_be_bytes());
    let limit = u64::MAX - u64::MAX % u64::from(bound);
    for attempt in 0..64 {
        let n = deterministic_decision_u64(
            &root,
            DecisionOccurrenceId::from_bytes(id),
            "source_attack",
            attempt,
        )
        .ok()?;
        if n < limit {
            return Some((n % u64::from(bound)) as u32);
        }
    }
    None
}
fn current_source(
    runtime: &ChannelRuntimeV1,
    native: &NativeGameplayState,
    proposal: &ProfileAbilityProposal,
) -> Option<Kind> {
    if proposal.list != crate::ai_think::profile_schedule::ScheduleList::Attack {
        return None;
    }
    let current = runtime.companion_snapshot(proposal.issuer).ok()?;
    if current.state.master.is_some() || runtime.native_summon_master(proposal.issuer).is_some() {
        return None;
    }
    let row = native.creature_profiles().records.iter().find(|r| {
        r.profile.target.key == current.state.policy.definition_key
            && r.profile.target.revision == current.state.policy.definition_revision
    })?;
    let source = native.soul_war_source_attacks(&row.profile.target)?;
    if source.behavior().attacks.get(proposal.entry_index)?.ability != proposal.ability {
        return None;
    }
    source.kind(&proposal.ability)
}

fn ensure_context(
    runtime: &ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    room: &QualifiedNativeEntryRoom,
    native: &NativeGameplayState,
) -> bool {
    if !matches!(
        runtime.binding().runtime_scope(),
        RuntimeScopeRefV1::Instance { .. }
    ) || runtime.content_pin().frame_binding_digest() != room.frame_binding().digest()
        || runtime.map_revision_digest() != room.map_revision_digest()
        || native.source_digest() != runtime.content_pin().server_artifact_digest()
        || !matches!(
            room.pinned_activity_source_room(),
            Some(Room::GoshnarsCruelty | Room::GoshnarsMegalomania)
        )
    {
        states.soul_war_callbacks.attacks = SoulWarAttacks::default();
        return false;
    }
    let current = binding(runtime);
    if states.soul_war_callbacks.attacks.binding != Some(current) {
        states.soul_war_callbacks.attacks = SoulWarAttacks {
            binding: Some(current),
            ..Default::default()
        };
    }
    true
}

/// One original due callback, without accepting a fresh native Think/attack.
/// The caller selects this family's deadline against every other source owner.
#[allow(clippy::too_many_arguments)]
pub(in crate::gameplay_transport) async fn drive_due_through(
    runtime: &mut ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    room: &QualifiedNativeEntryRoom,
    native: &NativeGameplayState,
    map: &mut CurrentMonsterMap<'_, '_>,
    through: u64,
    now: u64,
) -> Result<Option<u64>, ()> {
    if !ensure_context(runtime, states, room, native) {
        return Ok(None);
    }
    drive_one_due(runtime, states, room, native, map, through, now).await?;
    Ok(states.soul_war_callbacks.attacks.next_due())
}

/// Synchronous DATA intake immediately beside the original AI report commit.
/// Target selection and all outward effects wait for the shared proposal cursor.
pub(in crate::gameplay_transport) fn capture_attack_proposal(
    runtime: &mut ChannelRuntimeV1,
    room: &QualifiedNativeEntryRoom,
    native: &NativeGameplayState,
    proposal: &ProfileAbilityProposal,
    observed_at_us: u64,
) -> Result<Option<RetainedSoulWarAttack>, ()> {
    if proposal.list != crate::ai_think::profile_schedule::ScheduleList::Attack
        || !proposal
            .ability
            .key
            .starts_with("oteryn:ability.srx3.soul-war.")
        || !matches!(
            runtime.binding().runtime_scope(),
            RuntimeScopeRefV1::Instance { .. }
        )
        || !matches!(
            room.pinned_activity_source_room(),
            Some(Room::GoshnarsCruelty | Room::GoshnarsMegalomania)
        )
    {
        return Ok(None);
    }
    if runtime.content_pin().frame_binding_digest() != room.frame_binding().digest()
        || runtime.map_revision_digest() != room.map_revision_digest()
        || native.source_digest() != runtime.content_pin().server_artifact_digest()
    {
        return Err(());
    }
    let kind = current_source(runtime, native, proposal).ok_or(())?;
    let expected = if kind == Kind::CrueltyElemental {
        Room::GoshnarsCruelty
    } else {
        Room::GoshnarsMegalomania
    };
    if room.pinned_activity_source_room() != Some(expected) {
        return Err(());
    }
    Ok(Some(RetainedSoulWarAttack {
        binding: binding(runtime),
        stamp: runtime.issue_owner_work().map_err(|_| ())?,
        issuer: proposal.issuer,
        occurrence: proposal.occurrence.id().as_str().to_owned(),
        kind,
        observed_at_us,
    }))
}

#[allow(clippy::too_many_arguments)]
pub(in crate::gameplay_transport) async fn drive_retained_attack_proposal(
    runtime: &mut ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    room: &QualifiedNativeEntryRoom,
    native: &NativeGameplayState,
    map: &mut CurrentMonsterMap<'_, '_>,
    source: Arc<RetainedSoulWarAttack>,
    now: u64,
) -> Result<Option<u64>, ()> {
    if !ensure_context(runtime, states, room, native)
        || !source.current(runtime, native)
        || source.observed_at_us > now
    {
        return Err(());
    }
    // This is an already accepted proposal from the original AI pass. A retry
    // may execute much later, but must not drain newer callbacks ahead of this
    // pass or of another source owner's earlier deadline.
    let through = source.observed_at_us;
    let registered = runtime
        .source_registered_creature_census()
        .map_err(|_| ())?;
    states
        .soul_war_callbacks
        .attacks
        .seen
        .retain(|(actor, _)| registered.iter().any(|(current, _)| current == actor));
    while states
        .soul_war_callbacks
        .attacks
        .next_due()
        .is_some_and(|due| due <= through)
    {
        drive_one_due(runtime, states, room, native, map, through, now).await?;
    }
    accept_retained(runtime, states, room, native, source, now)?;
    while states
        .soul_war_callbacks
        .attacks
        .next_due()
        .is_some_and(|due| due <= through)
    {
        drive_one_due(runtime, states, room, native, map, through, now).await?;
    }
    Ok(states.soul_war_callbacks.attacks.next_due())
}

fn accept_retained(
    runtime: &mut ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    room: &QualifiedNativeEntryRoom,
    native: &NativeGameplayState,
    source: Arc<RetainedSoulWarAttack>,
    now: u64,
) -> Result<(), ()> {
    if !source.current(runtime, native) || source.observed_at_us > now {
        return Err(());
    }
    let kind = source.kind;
    let expected = if kind == Kind::CrueltyElemental {
        Room::GoshnarsCruelty
    } else {
        Room::GoshnarsMegalomania
    };
    if room.pinned_activity_source_room() != Some(expected) {
        return Err(());
    }
    let issuer = source.issuer;
    let id = source.occurrence.as_str();
    if states
        .soul_war_callbacks
        .attacks
        .seen
        .iter()
        .any(|(actor, last)| *actor == issuer && last == id)
        || states
            .soul_war_callbacks
            .attacks
            .scheduled
            .iter()
            .any(|pending| pending.source.issuer == issuer && pending.source.occurrence == id)
    {
        return Ok(());
    }
    if !runtime
        .source_registered_creature_census()
        .map_err(|_| ())?
        .iter()
        .any(|(actor, _)| *actor == issuer)
    {
        remember(states, issuer, id)?;
        return Ok(());
    }
    let mut selected = None;
    let mut geometry = None;
    let mut telegraph = None;
    let due = if kind == Kind::MegalomaniaBlue {
        let at = runtime
            .read_actor_position(issuer)
            .map_err(|_| ())?
            .position();
        geometry = Some(area::Geometry {
            caster: at,
            target: at,
        });
        source.observed_at_us
    } else {
        let players = runtime
            .positioned_melee_census()
            .map_err(|_| ())?
            .into_iter()
            .filter_map(|(actor, position, session)| {
                session
                    .filter(|_| in_zone(kind, position.position()))
                    .map(|session| (actor, session, position.position()))
            })
            .collect::<Vec<_>>();
        if players.is_empty() {
            remember(states, issuer, id)?;
            return Ok(());
        }
        let index = draw(
            native.source_digest(),
            issuer,
            id,
            0,
            players.len().try_into().map_err(|_| ())?,
        )
        .ok_or(())?;
        let (actor, session, at) = players[index as usize];
        runtime
            .player_control_facts(actor, session)
            .map_err(|_| ())?;
        native
            .spell_appearances()
            .and_then(|a| a.for_look_type(242))
            .ok_or(())?;
        selected = Some((actor, session));
        telegraph = Some(at);
        now.checked_add(7_000_000).ok_or(())?
    };
    let owner = &mut states.soul_war_callbacks.attacks;
    if owner.scheduled.len() >= 64 {
        return Err(());
    }
    owner.scheduled.try_reserve(1).map_err(|_| ())?;
    owner.seen.try_reserve(1).map_err(|_| ())?;
    if let Some((actor, session)) = selected {
        // The source ignores addCondition's false result. It still publishes
        // its telegraph and schedules the same seven-second callback, including
        // when Zone:getRandomPlayer selected a still registered HP0 Player.
        let _ = apply_outfit(runtime, states, native, issuer, actor, session, id, now);
    }
    states.soul_war_callbacks.attacks.scheduled.push(Scheduled {
        source: Arc::clone(&source),
        original_due: source.observed_at_us,
        due,
        target: selected,
        telegraph,
        geometry,
        area: None,
    });
    // No await separates acceptance, original due retention and replay marking.
    remember(states, issuer, id)
}

fn remember(states: &mut ChannelSpellStates, actor: ExactActorRef, id: &str) -> Result<(), ()> {
    let owner = &mut states.soul_war_callbacks.attacks;
    if let Some((_, last)) = owner.seen.iter_mut().find(|(current, _)| *current == actor) {
        *last = id.to_owned();
    } else {
        owner.seen.try_reserve(1).map_err(|_| ())?;
        owner.seen.push((actor, id.to_owned()));
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn drive_one_due(
    runtime: &mut ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    room: &QualifiedNativeEntryRoom,
    native: &NativeGameplayState,
    map: &mut CurrentMonsterMap<'_, '_>,
    through: u64,
    now: u64,
) -> Result<(), ()> {
    let Some(index) = states
        .soul_war_callbacks
        .attacks
        .scheduled
        .iter()
        .enumerate()
        .filter(|(_, task)| task.next_due() <= through && task.next_due() <= now)
        .min_by_key(|(_, task)| task.next_due())
        .map(|(index, _)| index)
    else {
        return Ok(());
    };
    let task = &states.soul_war_callbacks.attacks.scheduled[index];
    if !task.source.current(runtime, native) {
        return Err(());
    }
    if let Some(at) = task.telegraph {
        if !scene::emit_effect(room, at, 13) {
            return Err(());
        }
        states.soul_war_callbacks.attacks.scheduled[index].telegraph = None;
        return Ok(());
    }
    if task.geometry.is_none() {
        // Literal Creature(id)/Player(id) test registration, not current HP or
        // the source's possibly changed monster type. A replacement is excluded.
        let registered = runtime
            .source_registered_creature_census()
            .map_err(|_| ())?;
        let target = task.target.ok_or(())?;
        let census = runtime.positioned_melee_census().map_err(|_| ())?;
        let caster = registered
            .iter()
            .find(|(actor, _)| *actor == task.source.issuer);
        let target = census
            .iter()
            .find(|(actor, _, session)| *actor == target.0 && *session == Some(target.1));
        let (Some((_, caster)), Some((_, target, _))) = (caster, target) else {
            states.soul_war_callbacks.attacks.scheduled.remove(index);
            return Ok(());
        };
        let geometry = area::Geometry {
            caster: caster.position(),
            target: target.position(),
        };
        if geometry.caster.floor != geometry.target.floor {
            states.soul_war_callbacks.attacks.scheduled.remove(index);
            return Ok(());
        }
        states.soul_war_callbacks.attacks.scheduled[index].geometry = Some(geometry);
    }
    if states.soul_war_callbacks.attacks.scheduled[index]
        .area
        .is_none()
    {
        let task = &states.soul_war_callbacks.attacks.scheduled[index];
        let prepared = area::prepare(
            runtime,
            room,
            native,
            map,
            &task.source,
            task.geometry.ok_or(())?,
        )
        .await?;
        states.soul_war_callbacks.attacks.scheduled[index].area = Some(prepared);
    }
    // Taking the task is restricted to this synchronous commit section; every
    // refusal restores its cursor and exact cohort before the function returns.
    let mut task = states.soul_war_callbacks.attacks.scheduled.remove(index);
    let result = task
        .area
        .as_mut()
        .ok_or(())
        .and_then(|pending| area::finish(runtime, states, room, native, pending, now));
    if result.is_err() {
        states
            .soul_war_callbacks
            .attacks
            .scheduled
            .insert(index, task);
    }
    result
}

#[allow(
    clippy::too_many_arguments,
    reason = "source issuer, target/session, native appearance and occurrence clock are independent owner proofs"
)]
fn apply_outfit(
    runtime: &mut ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    native: &NativeGameplayState,
    source: ExactActorRef,
    target: ExactActorRef,
    session: GameSessionId,
    occurrence: &str,
    now: u64,
) -> bool {
    let Some(appearance) = native
        .spell_appearances()
        .and_then(|a| a.for_look_type(242))
    else {
        return false;
    };
    let Some(definition) = crate::foundation::ConditionDefinition::new(
        "srx3:soul-war:elemental-outfit",
        1,
        crate::foundation::ConditionValues::Outfit {
            duration_ms: 7000,
            look_type: 242,
        },
    )
    .and_then(|d| d.with_appearance(appearance.selection())) else {
        return false;
    };
    let Some(player) = states.get(runtime, target, session) else {
        return false;
    };
    let Ok(base_speed) = u16::try_from(player.owned_base_speed()) else {
        return false;
    };
    let root = GameplayDecisionRoot::from_bytes(
        Sha256::new()
            .chain_update(native.source_digest())
            .chain_update(source.placement_identity())
            .chain_update(occurrence.as_bytes())
            .finalize()
            .into(),
    );
    let facts = crate::foundation::ApplicationFacts {
        now,
        base_speed,
        mana_shield_capacity: 0,
        target_reentry_protected: false,
        source_reentry_protected: false,
        target_is_player: true,
        decision_root: &root,
        occurrence: DecisionOccurrenceId::from_bytes([0; 16]),
    };
    let Ok(stamp) = runtime.issue_owner_work() else {
        return false;
    };
    states
        .apply_source_appearance_batch_current(
            runtime,
            stamp,
            source,
            &[definition],
            &[crate::player_lethal::SourceAppearanceTarget {
                actor: target,
                session: Some(session),
                facts,
                immunities: vec![],
            }],
        )
        .is_some()
}

#[cfg(test)]
impl SoulWarCallbacks {
    /// Observe the real accepted queue; this creates no source proposal or timer.
    pub(in crate::gameplay_transport) fn source_attack_schedule_for_test(
        &self,
    ) -> Vec<(ExactActorRef, String, u64, u64)> {
        self.attacks
            .scheduled
            .iter()
            .map(|pending| {
                (
                    pending.source.issuer,
                    pending.source.occurrence.clone(),
                    pending.original_due,
                    pending.due,
                )
            })
            .collect()
    }

    pub(in crate::gameplay_transport) fn pause_attack_after_player_commit_for_test(&mut self) {
        self.attacks.pause_after_player_commit = true;
    }

    pub(in crate::gameplay_transport) fn source_attack_seen_for_test(
        &self,
        issuer: ExactActorRef,
        occurrence: &str,
    ) -> bool {
        self.attacks
            .seen
            .iter()
            .any(|(actor, seen)| *actor == issuer && seen == occurrence)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn srx3_soul_war_source_blue_cross_has_real_centre_and_thirty_one_callbacks() {
        let cells = shape(true);
        assert_eq!(cells.len(), 31);
        assert_eq!(cells.iter().filter(|p| **p == (0, 0)).count(), 1);
        assert!(cells.contains(&(-8, 0)) && cells.contains(&(8, 0)));
        assert!(cells.contains(&(0, -7)) && cells.contains(&(0, 7)));
        assert!(!cells.contains(&(-16, -14)));
    }
    #[test]
    fn srx3_soul_war_elemental_source_area_is_symmetric_around_target_not_caster() {
        let cells = shape(false);
        assert_eq!(cells.len(), 101);
        for (x, y) in &cells {
            assert!(cells.contains(&(-x, -y)));
        }
        assert!(cells.contains(&(0, 6)) && cells.contains(&(6, 0)));
        assert!(!cells.contains(&(6, 6)));
    }
}
