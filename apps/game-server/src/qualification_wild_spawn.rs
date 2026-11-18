//! Explicit Game-owned test spawn; position is NOT a public Canary spawn record.
//! Source-qualified creature policy/geometry and actual scope/Item owners remain independent.
//! One owner serves the harness's Thalom Rat and the node-boot `spawn` control command.
use crate::content::native_gameplay::NativeGameplayState;
use crate::content::{CollisionClass, LogicalCell, QualifiedNativeEntryRoom};
use crate::durability::runtime_scope_assignment::NodeIncarnationProof;
use crate::durability::{DurabilityRoot, character_authority::ReconciledCharacterAuthority};
use crate::foundation::{
    ChannelRuntimeV1, CompanionSnapshot, GameSessionId, MovementLocalPosition,
};
use oteryn_gameplay_content::content::channel_content_pin::ChannelContentPin;
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;

#[cfg(test)]
pub(crate) const POSITION: MovementLocalPosition = MovementLocalPosition {
    x: 1,
    y: 11,
    floor: -7,
};
#[cfg(test)]
pub(crate) const DECLARATION: &str = "oteryn:qualification.spawn/thalom-rat-r1";
#[cfg(test)]
const RAT_KEY: &str = "canary:creature/rat";

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum SpawnRefusal {
    UnknownCreature,
    Deadline,
    SessionUnknown,
    NoFreeTile(String),
    Fence(String),
}

impl SpawnRefusal {
    /// The `rejected reason=` token of the control answer.
    pub(crate) const fn token(&self) -> &'static str {
        match self {
            Self::UnknownCreature => "unknown_creature",
            Self::Deadline => "deadline",
            Self::SessionUnknown => "session_unknown",
            Self::NoFreeTile(_) => "no_free_tile",
            Self::Fence(_) => "fence",
        }
    }

    /// The refused rule, for the node's diagnostic line.
    pub(crate) fn detail(&self) -> Option<&str> {
        match self {
            Self::NoFreeTile(message) | Self::Fence(message) => Some(message),
            Self::UnknownCreature | Self::SessionUnknown | Self::Deadline => None,
        }
    }

    #[cfg(test)]
    fn into_message(self) -> String {
        match self {
            Self::UnknownCreature => "qualification spawn active Creature policies: unknown".into(),
            Self::Deadline => "qualification spawn deadline".into(),
            Self::SessionUnknown => "qualification spawn session has no positioned actor".into(),
            Self::NoFreeTile(message) | Self::Fence(message) => message,
        }
    }
}

/// The eight neighbours of `center` on its floor, orthogonal first, in a fixed order.
pub(crate) fn adjacent_cells(center: MovementLocalPosition) -> Vec<MovementLocalPosition> {
    [
        (0, -1),
        (1, 0),
        (0, 1),
        (-1, 0),
        (1, -1),
        (1, 1),
        (-1, 1),
        (-1, -1),
    ]
    .into_iter()
    .filter_map(|(dx, dy)| {
        Some(MovementLocalPosition {
            x: center.x.checked_add(dx)?,
            y: center.y.checked_add(dy)?,
            floor: center.floor,
        })
    })
    .collect()
}

/// The first of `candidates` the runtime's spawn owner accepts; occupied, reserved and
/// capacity conflicts stay with that owner, which restores its state on every refusal.
pub(crate) fn place_first(
    runtime: &mut ChannelRuntimeV1,
    pin: &ChannelContentPin,
    key: &str,
    candidates: &[MovementLocalPosition],
) -> Result<CompanionSnapshot, SpawnRefusal> {
    let mut last = "qualification spawn has no eligible cell".to_owned();
    for cell in candidates {
        match runtime.realize_native_qualification_once(pin.clone(), key, *cell) {
            Ok(snapshot) => return Ok(snapshot),
            Err(error) => last = format!("qualification spawn physical realization: {error:?}"),
        }
    }
    Err(SpawnRefusal::NoFreeTile(last))
}

/// The creature must have an active qualified policy; no key is realised from guesswork.
fn require_policy(runtime: &ChannelRuntimeV1, key: &str) -> Result<(), SpawnRefusal> {
    runtime
        .companion_policy(key)
        .map(drop)
        .map_err(|_| SpawnRefusal::UnknownCreature)
}

#[cfg(test)]
async fn spawn_when_verified<T, V, C, R>(
    runtime: &tokio::sync::Mutex<T>,
    verify: impl Future<Output = Result<(V, C), SpawnRefusal>>,
    commit: impl AsyncFnOnce(C) -> Result<(), SpawnRefusal>,
    publish: impl FnOnce(&mut T, V) -> Result<R, SpawnRefusal>,
) -> Result<R, SpawnRefusal> {
    let (verified, transaction) = verify.await?;
    commit(transaction).await?;
    let mut owner = runtime.lock().await;
    publish(&mut owner, verified)
}

#[cfg(test)]
#[test]
fn qualification_spawn_verification_leaves_the_live_runtime_available()
-> Result<(), Box<dyn std::error::Error>> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let runtime = tokio::sync::Mutex::new(0);
            let result = spawn_when_verified(
                &runtime,
                async {
                    assert!(
                        runtime.try_lock().is_ok(),
                        "database verification held the live Channel"
                    );
                    Ok((2, ()))
                },
                async |()| Ok(()),
                |live, increment| {
                    *live += increment;
                    Ok(())
                },
            )
            .await;
            assert_eq!(result, Ok(()));
            assert_eq!(*runtime.lock().await, 2);
        });
    Ok(())
}

#[cfg(test)]
#[test]
fn qualification_spawn_commit_refusal_has_no_visible_actor()
-> Result<(), Box<dyn std::error::Error>> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let runtime = tokio::sync::Mutex::new(Vec::new());
            let result = spawn_when_verified(
                &runtime,
                async { Ok((42, ())) },
                async |()| Err(SpawnRefusal::Fence("commit failed".into())),
                |live, actor| {
                    live.push(actor);
                    Ok(())
                },
            )
            .await;
            assert!(result.is_err());
            assert!(
                runtime.lock().await.is_empty(),
                "a refused spawn became visible"
            );
        });
    Ok(())
}

#[derive(Clone, Copy)]
enum Placement {
    // Only the cfg(test) seam rat (realize_rat_in_transaction) places at a fixed cell.
    #[cfg_attr(not(test), allow(dead_code))]
    Fixed(MovementLocalPosition),
    Adjacent(GameSessionId),
}

/// The Rat of the seam harness, at its Thalom cell, unchanged.
#[cfg(test)]
#[allow(clippy::too_many_arguments)]
pub(crate) async fn realize_rat_in_transaction(
    mut tx: sqlx::Transaction<'_, sqlx::Postgres>,
    permit: crate::durability::spell_owner_commit::SpellLanePermit,
    root: &DurabilityRoot,
    recovery: &ReconciledCharacterAuthority<'_, '_>,
    node: &NodeIncarnationProof,
    room: &QualifiedNativeEntryRoom,
    native: &NativeGameplayState,
    runtime: &tokio::sync::Mutex<ChannelRuntimeV1>,
) -> Result<CompanionSnapshot, String> {
    let plan = {
        let mut live = runtime.lock().await;
        prepare_placement(room, native, &mut live, RAT_KEY, Placement::Fixed(POSITION))
            .map_err(SpawnRefusal::into_message)?
    };
    spawn_when_verified(
        runtime,
        async {
            let eligible =
                validate_placement(&mut tx, root, recovery, node, &permit, &plan).await?;
            Ok((eligible, tx))
        },
        async |tx| {
            tx.commit().await.map_err(|error| {
                SpawnRefusal::Fence(format!("qualification spawn transaction commit: {error:?}"))
            })
        },
        |live, eligible| publish_placement(live, RAT_KEY, &plan, &eligible),
    )
    .await
    .map_err(SpawnRefusal::into_message)
}

pub(crate) struct PreparedSpawn {
    pub(crate) scope: crate::foundation::RuntimeScopeRefV1,
    pin: ChannelContentPin,
    generation: u64,
    candidates: Vec<(
        MovementLocalPosition,
        crate::durability::native_world_items::SpellGroundTarget,
    )>,
    session: Option<GameSessionId>,
    player: Option<(crate::foundation::ExactActorRef, MovementLocalPosition)>,
}

pub(crate) fn prepare_placement_adjacent(
    room: &QualifiedNativeEntryRoom,
    native: &NativeGameplayState,
    runtime: &mut ChannelRuntimeV1,
    key: &str,
    session: GameSessionId,
) -> Result<PreparedSpawn, SpawnRefusal> {
    prepare_placement(room, native, runtime, key, Placement::Adjacent(session))
}

fn prepare_placement(
    room: &QualifiedNativeEntryRoom,
    native: &NativeGameplayState,
    runtime: &mut ChannelRuntimeV1,
    key: &str,
    placement: Placement,
) -> Result<PreparedSpawn, SpawnRefusal> {
    let fence = SpawnRefusal::Fence;
    let pin = runtime.content_pin().clone();
    let cells = room.movement_cells();
    if runtime.owner_fence().is_err()
        || native.source_digest() != pin.server_artifact_digest()
        || room.compiled().server_digest() != pin.server_artifact_digest()
        || room.frame_binding().digest() != pin.frame_binding_digest()
        || room.map_revision_digest() != pin.map_revision_digest()
        || cells.scope().world_id != runtime.binding().world_id()
        || cells.scope().generation_digest != pin.server_artifact_digest()
    {
        return Err(fence(
            "qualification spawn current Content/scope mismatch".into(),
        ));
    }
    native
        .install_companion_policies(runtime)
        .map_err(|error| {
            fence(format!(
                "qualification spawn active Creature policies: {error:?}"
            ))
        })?;
    require_policy(runtime, key)?;
    let candidates = match &placement {
        Placement::Fixed(position) => vec![*position],
        Placement::Adjacent(session) => {
            let (_, actor) = runtime
                .positioned_player_for_session(*session)
                .map_err(|error| fence(format!("qualification spawn session actor: {error:?}")))?
                .ok_or(SpawnRefusal::SessionUnknown)?;
            adjacent_cells(actor.position())
        }
    };
    let scope = runtime.binding().runtime_scope();
    let generation = runtime.binding().scope_generation().get();
    let candidates = candidates
        .into_iter()
        .filter(|position| static_cell_refusal(room, *position).is_ok())
        .map(|position| {
            Ok((
                position,
                crate::durability::native_world_items::SpellGroundTarget::for_native_tile_read(
                    room, runtime, position,
                )
                .map_err(|error| fence(error.into()))?,
            ))
        })
        .collect::<Result<Vec<_>, SpawnRefusal>>()?;
    Ok(PreparedSpawn {
        pin,
        scope,
        generation,
        candidates,
        session: match placement {
            Placement::Adjacent(session) => Some(session),
            Placement::Fixed(_) => None,
        },
        player: match placement {
            Placement::Adjacent(session) => runtime
                .positioned_player_for_session(session)
                .map_err(|error| fence(format!("spawn player: {error:?}")))?
                .map(|(actor, snapshot)| (actor, snapshot.position())),
            Placement::Fixed(_) => None,
        },
    })
}

pub(crate) async fn validate_placement(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    root: &DurabilityRoot,
    recovery: &ReconciledCharacterAuthority<'_, '_>,
    node: &NodeIncarnationProof,
    permit: &crate::durability::spell_owner_commit::SpellLanePermit,
    plan: &PreparedSpawn,
) -> Result<Vec<MovementLocalPosition>, SpawnRefusal> {
    let fence = SpawnRefusal::Fence;
    let owned = crate::durability::spell_item_transaction::assert_spell_item_scope_in_transaction(
        tx,
        permit,
        root,
        recovery,
        node,
        plan.scope,
        plan.generation,
    )
    .await
    .map_err(|error| fence(format!("qualification spawn scope fence: {error:?}")))?;
    let mut eligible = Vec::new();
    for (position, target) in &plan.candidates {
        let items = crate::durability::spell_item_transaction::read_qualification_scope_tile_in_transaction(tx, &owned, target)
            .await.map_err(|error| fence(format!("qualification spawn Item tile: {error:?}")))?;
        if !items
            .items
            .iter()
            .any(|item| item.blocks_movement || item.immovable_block_solid)
        {
            eligible.push(*position);
        }
    }
    if eligible.is_empty() {
        return Err(SpawnRefusal::NoFreeTile(
            "no eligible unblocked tile".into(),
        ));
    }
    Ok(eligible)
}

pub(crate) fn publish_placement(
    runtime: &mut ChannelRuntimeV1,
    key: &str,
    plan: &PreparedSpawn,
    eligible: &[MovementLocalPosition],
) -> Result<CompanionSnapshot, SpawnRefusal> {
    if runtime.owner_fence().is_err()
        || runtime.content_pin() != &plan.pin
        || runtime.binding().runtime_scope() != plan.scope
        || runtime.binding().scope_generation().get() != plan.generation
    {
        return Err(SpawnRefusal::Fence("spawn current owner changed".into()));
    }
    if let Some(session) = plan.session {
        let position = runtime
            .positioned_player_for_session(session)
            .map_err(|error| SpawnRefusal::Fence(format!("spawn player: {error:?}")))?
            .map(|(actor, snapshot)| (actor, snapshot.position()));
        if position.is_none() || position != plan.player {
            return Err(SpawnRefusal::Fence(
                "spawn player moved during validation".into(),
            ));
        }
    }
    place_first(runtime, &plan.pin, key, eligible)
}

/// The qualified tile and source-metadata rules of one spawn cell.
fn static_cell_refusal(
    room: &QualifiedNativeEntryRoom,
    position: MovementLocalPosition,
) -> Result<(), String> {
    let cells = room.movement_cells();
    let cell = LogicalCell {
        x: position.x,
        y: position.y,
        z: i32::from(position.floor),
    };
    // The fixture room declares completely qualified source metadata, and a pending source
    // placement cannot be treated as an absent durable Item. A bundle World's room has no
    // source world and no pending placements; its Items are read per tile from the durable
    // owner by the caller, so only the bundle's own collision index qualifies the cell there.
    let pending_or_unqualified = match room.source_world() {
        Some(world) => world
            .pending_mutable_placements()
            .iter()
            .any(|placement| placement.cell() == cell),
        None => !matches!(
            cells.index(),
            crate::content::native_cell_lookup::NativeMovementCollisionIndex::Bundle(_)
        ),
    };
    if pending_or_unqualified
        || cells.index().lookup(cells.scope(), cell) != Ok(CollisionClass::Walkable)
    {
        return Err(
            "qualification spawn cell is unknown, blocked or pending materialization".into(),
        );
    }
    let tile = cells
        .spell_tiles()
        .lookup(cells.scope(), cell)
        .map_err(|_| "qualification spawn has no exact tile metadata".to_owned())?;
    let flags = tile.flags();
    if !tile.ground_present() || flags.block_solid || flags.protection_zone || flags.floor_change {
        return Err("qualification spawn ground/solidity/PZ/floor eligibility refused".into());
    }
    Ok(())
}

#[cfg(test)]
// Test-only fixture setup; a failed setup is the test failure.
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::foundation::{ChannelId, NodeId, WorldId};
    use oteryn_gameplay_content::content::creature_policy::{
        CompiledCreaturePolicies, CompiledCreaturePolicy, CreatureFlags,
    };

    const SPIDER: &str = "fixture:cave_spider";
    const PLAYER_AT: MovementLocalPosition = MovementLocalPosition {
        x: 2,
        y: 11,
        floor: -7,
    };

    fn uuid(raw: u8) -> [u8; 16] {
        let mut bytes = [0; 16];
        bytes[6] = 0x70;
        bytes[8] = 0x80;
        bytes[15] = raw;
        bytes
    }

    fn policy(key: &str, digest: [u8; 32]) -> CompiledCreaturePolicies {
        CompiledCreaturePolicies::from_active_artifact(
            digest,
            vec![CompiledCreaturePolicy {
                definition_key: key.into(),
                definition_revision: "fixture:1".into(),
                display_name: "fixture creature".into(),
                maximum_health: 20,
                forge_difficulty: None,
                base_speed: 110,
                outfit_look_type: 21,
                object_look_type: None,
                summonable: false,
                convinceable: false,
                mana_cost: None,
                is_familiar: false,
                condition_immunities: vec![],
                defense: Some(1),
                armor: Some(1),
                mitigation: None,
                resistances: vec![],
                damage_immunities: vec![],
                healing_from_damage: vec![],
                preferred_distance: Some(1),
                reward_boss: Some(false),
                blocked_by_nearby_players: false,
                flags: CreatureFlags {
                    attackable: true,
                    illusionable: false,
                    health_hidden: false,
                },
            }],
        )
        .expect("qualified fixture table")
    }

    /// A Channel runtime whose only policy is `SPIDER` and whose one player stands at `PLAYER_AT`.
    fn runtime_with_player() -> (ChannelRuntimeV1, ChannelContentPin, GameSessionId) {
        let world = WorldId::decode(&uuid(1)).expect("world");
        let pin = ChannelContentPin::from_activation(
            world,
            1,
            [1; 32],
            [2; 32],
            [3; 32],
            [4; 32],
            (PLAYER_AT.x, PLAYER_AT.y, PLAYER_AT.floor),
        );
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            world,
            ChannelId::decode(&uuid(2)).expect("channel"),
            NodeId::decode(&uuid(3)).expect("node"),
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            8,
            pin.clone(),
        )
        .expect("runtime");
        runtime
            .install_companion_policies(policy(SPIDER, pin.server_artifact_digest()))
            .expect("policies");
        let session = GameSessionId::decode(&uuid(4)).expect("session");
        let reservation = runtime.reserve_fresh_session(session).expect("reservation");
        let player = runtime.commit_fresh_session(reservation).expect("player");
        runtime
            .initialize_first_entry_position(player)
            .expect("entry position");
        (runtime, pin, session)
    }

    fn adjacent(center: MovementLocalPosition, other: MovementLocalPosition) -> bool {
        other != center
            && other.floor == center.floor
            && (other.x - center.x).abs() <= 1
            && (other.y - center.y).abs() <= 1
    }

    #[test]
    fn qualification_creatures_do_not_register_a_respawning_source()
    -> Result<(), Box<dyn std::error::Error>> {
        let (mut runtime, pin, _session) = runtime_with_player();
        let body = place_first(&mut runtime, &pin, SPIDER, &adjacent_cells(PLAYER_AT))
            .map_err(|error| format!("{error:?}"))?;
        runtime
            .remove_native_qualification_once(body.actor, SPIDER, pin.server_artifact_digest())
            .map_err(|error| format!("{error:?}"))?;
        for now in [1_000_000, 61_000_000, 121_000_000] {
            runtime
                .drive_spawn_respawns(now, |_, _| false)
                .map_err(|error| format!("{error:?}"))?;
        }
        assert!(
            runtime.visible_entities().creatures.is_empty(),
            "a previous sweep specimen must never respawn after its removal"
        );
        Ok(())
    }

    #[test]
    fn a_non_rat_creature_is_realised_adjacent_to_the_session_actor() {
        let (mut runtime, pin, session) = runtime_with_player();
        let (_, actor) = runtime
            .positioned_player_for_session(session)
            .expect("session read")
            .expect("positioned player");
        assert_eq!(actor.position(), PLAYER_AT);
        let cells = adjacent_cells(actor.position());
        assert_eq!(cells.len(), 8);
        let first = place_first(&mut runtime, &pin, SPIDER, &cells).expect("first spawn");
        assert_eq!(first.state.policy.definition_key, SPIDER);
        assert!(first.state.master.is_none());
        assert_eq!(first.position, cells[0]);
        assert!(adjacent(PLAYER_AT, first.position));
        // The first cell is now occupied, so the next creature takes the next free neighbour.
        let second = place_first(&mut runtime, &pin, SPIDER, &cells).expect("second spawn");
        assert_eq!(second.position, cells[1]);
        assert!(adjacent(PLAYER_AT, second.position));
        assert_ne!(
            first.actor.placement_identity(),
            second.actor.placement_identity()
        );
    }

    #[test]
    fn an_unknown_creature_key_is_refused_before_any_placement() {
        let (runtime, _, _) = runtime_with_player();
        assert_eq!(require_policy(&runtime, SPIDER), Ok(()));
        let refused = require_policy(&runtime, "fixture:not_a_creature");
        assert_eq!(refused, Err(SpawnRefusal::UnknownCreature));
        assert_eq!(
            refused.err().map(|refusal| refusal.token()),
            Some("unknown_creature")
        );
    }

    #[test]
    fn adjacent_cells_stay_on_the_floor_and_skip_overflow() {
        let center = MovementLocalPosition {
            x: i32::MAX,
            y: 0,
            floor: -3,
        };
        let cells = adjacent_cells(center);
        assert_eq!(cells.len(), 5);
        assert!(
            cells
                .iter()
                .all(|cell| cell.floor == -3 && adjacent(center, *cell))
        );
    }
}
