//! The four restored source proposals consume the existing AI clock, never a second clock.
use super::super::soul_war_scene as scene;
use super::*;
use crate::content::bosses::activity_template::PinnedActivitySourceRoom as Room;
use crate::content::native_soul_war::QualifiedSoulWarAttackKind as Kind;
use crate::creature_auto_attack::ProfileAbilityProposal;
use crate::foundation::{GameSessionId, RuntimeScopeRefV1, ScopeOwnershipGeneration};
use crate::player_lethal::PlayerLethalVitals;

#[derive(Debug)]
struct Delayed {
    issuer: ExactActorRef,
    target: ExactActorRef,
    due: u64,
    occurrence: String,
    kind: Kind,
}
type AttackOwnerBinding = (
    RuntimeScopeRefV1,
    ScopeOwnershipGeneration,
    [u8; 32],
    [u8; 32],
    [u8; 32],
);

#[derive(Debug, Default)]
pub(super) struct SoulWarAttacks {
    binding: Option<AttackOwnerBinding>,
    delayed: Vec<Delayed>,
    seen: Vec<(ExactActorRef, String)>,
}
impl SoulWarAttacks {
    pub(super) fn next_due(&self) -> Option<u64> {
        self.delayed.iter().map(|d| d.due).min()
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

#[allow(clippy::too_many_arguments)]
pub(in crate::gameplay_transport) async fn drive(
    runtime: &mut ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    room: &QualifiedNativeEntryRoom,
    native: &NativeGameplayState,
    map: &mut CurrentMonsterMap<'_, '_>,
    proposals: &[ProfileAbilityProposal],
    now: u64,
) -> Result<Option<u64>, ()> {
    let scope = runtime.binding().runtime_scope();
    if !matches!(scope, RuntimeScopeRefV1::Instance { .. })
        || runtime.content_pin().frame_binding_digest() != room.frame_binding().digest()
        || runtime.map_revision_digest() != room.map_revision_digest()
        || native.source_digest() != runtime.content_pin().server_artifact_digest()
        || !matches!(
            room.pinned_activity_source_room(),
            Some(Room::GoshnarsCruelty | Room::GoshnarsMegalomania)
        )
    {
        states.soul_war_callbacks.attacks = SoulWarAttacks::default();
        return Ok(None);
    }
    let binding = (
        scope,
        runtime.binding().scope_generation(),
        native.source_digest(),
        runtime.content_pin().frame_binding_digest(),
        runtime.map_revision_digest(),
    );
    let mut owner = std::mem::take(&mut states.soul_war_callbacks.attacks);
    if owner.binding != Some(binding) {
        owner = SoulWarAttacks {
            binding: Some(binding),
            ..Default::default()
        };
    }
    let result = drive_owned(
        &mut owner, runtime, states, room, native, map, proposals, now,
    )
    .await;
    states.soul_war_callbacks.attacks = owner;
    result
}

#[allow(clippy::too_many_arguments)]
async fn drive_owned(
    owner: &mut SoulWarAttacks,
    runtime: &mut ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    room: &QualifiedNativeEntryRoom,
    native: &NativeGameplayState,
    map: &mut CurrentMonsterMap<'_, '_>,
    proposals: &[ProfileAbilityProposal],
    now: u64,
) -> Result<Option<u64>, ()> {
    let census = runtime.positioned_actor_census().map_err(|_| ())?;
    owner
        .seen
        .retain(|(actor, _)| runtime.contains_live_creature(*actor));
    // Due events keep their original source combat even if setType changed the issuer.
    let mut index = 0;
    while index < owner.delayed.len() {
        let delayed = &owner.delayed[index];
        if delayed.due > now {
            index += 1;
            continue;
        }
        if runtime.contains_live_creature(delayed.issuer)
            && let Some((_, position, Some(_))) =
                census.iter().find(|(actor, _, _)| *actor == delayed.target)
            && !states.is_dead(delayed.target)
        {
            execute_area(
                runtime,
                states,
                room,
                native,
                map,
                delayed.issuer,
                position.position(),
                delayed.kind,
                &delayed.occurrence,
                now,
            )
            .await?;
        }
        // Remove only after a completed attempt, preserving other work on refusal.
        owner.delayed.remove(index);
    }
    for proposal in proposals {
        let Some(kind) = current_source(runtime, native, proposal) else {
            continue;
        };
        let expected = if kind == Kind::CrueltyElemental {
            Room::GoshnarsCruelty
        } else {
            Room::GoshnarsMegalomania
        };
        if room.pinned_activity_source_room() != Some(expected) {
            continue;
        }
        let id = proposal.occurrence.id().as_str();
        if owner
            .seen
            .iter()
            .any(|(actor, last)| *actor == proposal.issuer && last == id)
        {
            continue;
        }
        if let Some((_, last)) = owner
            .seen
            .iter_mut()
            .find(|(actor, _)| *actor == proposal.issuer)
        {
            *last = id.to_owned();
        } else {
            owner.seen.push((proposal.issuer, id.to_owned()));
        }
        if kind == Kind::MegalomaniaBlue {
            let at = runtime
                .read_actor_position(proposal.issuer)
                .map_err(|_| ())?
                .position();
            execute_area(
                runtime,
                states,
                room,
                native,
                map,
                proposal.issuer,
                at,
                kind,
                id,
                now,
            )
            .await?;
        } else {
            let players = census
                .iter()
                .filter_map(|(actor, position, session)| {
                    session
                        .filter(|_| in_zone(kind, position.position()) && !states.is_dead(*actor))
                        .map(|s| (*actor, s, position.position()))
                })
                .collect::<Vec<_>>();
            if players.is_empty() {
                continue;
            }
            let Some(index) = draw(
                native.source_digest(),
                proposal.issuer,
                id,
                0,
                players.len().try_into().map_err(|_| ())?,
            ) else {
                continue;
            };
            let (target, session, at) = players[index as usize];
            // Stage bounded scheduling before publishing its correlated outfit.
            if owner.delayed.len() >= 64 {
                continue;
            }
            let Some(due) = now.checked_add(7_000_000) else {
                continue;
            };
            // The source ignores addCondition's false result, still telegraphing
            // and scheduling its delayed combat. Missing immutable appearance
            // qualification refuses the ability; a current owner refusal does
            // not invent an extra Lua gameplay prerequisite.
            if native
                .spell_appearances()
                .and_then(|a| a.for_look_type(242))
                .is_none()
            {
                continue;
            }
            let _ = apply_outfit(
                runtime,
                states,
                native,
                proposal.issuer,
                target,
                session,
                id,
                now,
            );
            scene::emit_effect(room, at, 13);
            owner.delayed.push(Delayed {
                issuer: proposal.issuer,
                target,
                due,
                occurrence: id.to_owned(),
                kind,
            });
        }
    }
    let next = owner.next_due();
    Ok(next)
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

#[allow(clippy::too_many_arguments)]
async fn execute_area(
    runtime: &mut ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    room: &QualifiedNativeEntryRoom,
    native: &NativeGameplayState,
    map: &mut CurrentMonsterMap<'_, '_>,
    issuer: ExactActorRef,
    target: MovementLocalPosition,
    kind: Kind,
    occurrence: &str,
    now: u64,
) -> Result<(), ()> {
    let caster = runtime
        .read_actor_position(issuer)
        .map_err(|_| ())?
        .position();
    if caster.floor != target.floor {
        return Ok(());
    }
    let blue = kind == Kind::MegalomaniaBlue;
    // AreaCombat::getList's LOS origin is one step from target toward caster.
    let dx = caster.x - target.x;
    let dy = caster.y - target.y;
    let (sx, sy) = if dx.abs() > dy.abs() {
        (dx.signum(), 0)
    } else if dy.abs() > dx.abs() {
        (0, dy.signum())
    } else {
        (dx.signum(), dy.signum())
    };
    let sight = MovementLocalPosition {
        x: target.x + sx,
        y: target.y + sy,
        ..target
    };
    let mut tiles = Vec::new();
    for (dx, dy) in shape(blue) {
        let at = MovementLocalPosition {
            x: target.x + dx,
            y: target.y + dy,
            ..target
        };
        let _ = map.read(runtime, at).await;
        map.read_sight(runtime, sight, at).await;
        let cells = room.movement_cells();
        let Ok(tile) = cells.spell_tiles().lookup(
            cells.scope(),
            crate::content::LogicalCell {
                x: at.x,
                y: at.y,
                z: i32::from(at.floor),
            },
        ) else {
            continue;
        };
        if tile.flags().floor_change
            || tile.flags().protection_zone
            || map.view().blocks_projectile(at) != Some(false)
            || crate::map::floor::is_teleport(
                cells,
                crate::content::LogicalCell {
                    x: at.x,
                    y: at.y,
                    z: i32::from(at.floor),
                },
            )
            || !map.view().sight(sight, at)
        {
            continue;
        }
        tiles.push(at);
    }
    if !map.validate_transaction(runtime).await {
        return Ok(());
    }
    let census = runtime.positioned_actor_census().map_err(|_| ())?;
    // Lua queues death; keep the cast's top actor rather than exposing a lower stack after lethal HP.
    let targets = census
        .iter()
        .filter_map(|(actor, position, session)| {
            let at = position.position();
            session
                .filter(|_| {
                    (!blue || in_zone(kind, at))
                        && runtime.current_top_actor_on_cell(at).ok().flatten() == Some(*actor)
                })
                .map(|s| (*actor, s, at))
        })
        .collect::<Vec<_>>();
    for (ordinal, at) in tiles.into_iter().enumerate() {
        for (actor, session, position) in &targets {
            if states.is_dead(*actor) || (!blue && *position != at) {
                continue;
            }
            if blue {
                if !scene::current_ground(room, *position).is_some_and(|g| g.appearance_id != 409) {
                    continue;
                }
                let _ = states.apply_soul_war_lua_health_loss(
                    runtime,
                    native,
                    issuer,
                    *actor,
                    *session,
                    6000,
                    crate::foundation::owner_timer::SemanticTimeMicros::from_micros(now),
                );
            } else if let Some(value) = draw(
                native.source_digest(),
                issuer,
                occurrence,
                ordinal as u64 + 1,
                701,
            ) {
                let _ = states.apply_source_player_heal(
                    runtime,
                    *actor,
                    *session,
                    2300 + value,
                    &format!("{occurrence}:tile:{ordinal}"),
                    crate::foundation::owner_timer::SemanticTimeMicros::from_micros(now),
                );
            }
        }
        scene::emit_effect(room, at, if blue { 158 } else { 7 });
    }
    Ok(())
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
