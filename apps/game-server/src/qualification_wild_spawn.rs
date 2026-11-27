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
use oteryn_gameplay_domain::domain::carrier_values::CarrierError;
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use std::sync::atomic::{AtomicU16, Ordering};

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

pub(crate) const C18A_RAT_FIXTURE: &str = "qualification:c18a/fixed-rat";
pub(crate) const C18A_RAT_POSITION: MovementLocalPosition = MovementLocalPosition {
    x: 32491,
    y: 32169,
    floor: 7,
};
const C18A_RAT_KEY: &str = "oteryn:creature.rat";

/// Spawn source ids of the `spawn` control command; the harness Rat keeps `u16::MAX`.
const FIRST_COMMAND_SOURCE: u16 = 40_000;
static NEXT_COMMAND_SOURCE: AtomicU16 = AtomicU16::new(FIRST_COMMAND_SOURCE);

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum SpawnRefusal {
    UnknownCreature,
    SessionUnknown,
    NoFreeTile(String),
    Fence(String),
    StaticTile(String),
    ItemBlocked,
    Occupied,
    Physical(CarrierError),
    ExactPlacement(String),
}

impl SpawnRefusal {
    /// The `rejected reason=` token of the control answer.
    pub(crate) const fn token(&self) -> &'static str {
        match self {
            Self::UnknownCreature => "unknown_creature",
            Self::SessionUnknown => "session_unknown",
            Self::NoFreeTile(_) => "no_free_tile",
            Self::Fence(_) => "fence",
            Self::StaticTile(_) => "tile_ineligible",
            Self::ItemBlocked => "tile_item_blocked",
            Self::Occupied => "tile_occupied",
            Self::Physical(CarrierError::PlanConflict) => "placement_conflict",
            Self::Physical(CarrierError::CapacityExceeded) => "capacity_exceeded",
            Self::Physical(_) => "placement_refused",
            Self::ExactPlacement(_) => "c18a_exact_placement_refused",
        }
    }

    /// The refused rule, for the node's diagnostic line.
    pub(crate) fn detail(&self) -> Option<std::borrow::Cow<'_, str>> {
        match self {
            Self::NoFreeTile(message)
            | Self::Fence(message)
            | Self::StaticTile(message)
            | Self::ExactPlacement(message) => Some(std::borrow::Cow::Borrowed(message)),
            Self::Physical(error) => Some(std::borrow::Cow::Owned(error.to_string())),
            _ => None,
        }
    }

    #[cfg(test)]
    fn into_message(self) -> String {
        match self {
            Self::UnknownCreature => "qualification spawn active Creature policies: unknown".into(),
            Self::SessionUnknown => "qualification spawn session has no positioned actor".into(),
            Self::NoFreeTile(message)
            | Self::Fence(message)
            | Self::StaticTile(message)
            | Self::ExactPlacement(message) => message,
            Self::Physical(error) => error.to_string(),
            Self::ItemBlocked | Self::Occupied => self.token().to_owned(),
        }
    }
}

/// A fresh spawn source id for one `spawn` command.
pub(crate) fn next_command_source() -> Result<u16, SpawnRefusal> {
    NEXT_COMMAND_SOURCE
        .try_update(Ordering::Relaxed, Ordering::Relaxed, |next| {
            (next < u16::MAX - 1).then_some(next + 1)
        })
        .map_err(|_| SpawnRefusal::Fence("qualification spawn sources exhausted".into()))
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
    source: u16,
    key: &str,
    candidates: &[MovementLocalPosition],
) -> Result<CompanionSnapshot, SpawnRefusal> {
    let mut last = "qualification spawn has no eligible cell".to_owned();
    for cell in candidates {
        match runtime.realize_native_qualification_spawn_for_source(source, pin.clone(), key, *cell)
        {
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

enum Placement {
    // Only the cfg(test) seam rat (realize_rat_in_transaction) places at a fixed cell.
    #[cfg_attr(not(test), allow(dead_code))]
    Fixed(MovementLocalPosition),
    Session {
        session: GameSessionId,
        exact: Option<MovementLocalPosition>,
    },
    C18aRat(GameSessionId),
}

/// The Rat of the seam harness, at its Thalom cell, unchanged.
#[cfg(test)]
#[allow(clippy::too_many_arguments)]
pub(crate) async fn realize_rat_in_transaction(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    root: &DurabilityRoot,
    recovery: &ReconciledCharacterAuthority<'_, '_>,
    node: &NodeIncarnationProof,
    room: &QualifiedNativeEntryRoom,
    native: &NativeGameplayState,
    runtime: &mut ChannelRuntimeV1,
) -> Result<CompanionSnapshot, String> {
    let plan = snapshot_spawn(room, native, runtime, RAT_KEY, Placement::Fixed(POSITION))
        .map_err(SpawnRefusal::into_message)?;
    let eligible = read_spawn_tiles(tx, root, recovery, node, &plan)
        .await
        .map_err(SpawnRefusal::into_message)?;
    place_first(runtime, &plan.pin, u16::MAX, RAT_KEY, &eligible)
        .map_err(SpawnRefusal::into_message)
}

pub(crate) struct SpawnPlan {
    exact_rat: bool,
    pin: ChannelContentPin,
    scope: crate::foundation::RuntimeScopeRefV1,
    scope_generation: u64,
    exact: Option<MovementLocalPosition>,
    player: Option<(
        GameSessionId,
        crate::foundation::ExactActorRef,
        MovementLocalPosition,
    )>,
    targets: Vec<(
        MovementLocalPosition,
        oteryn_gameplay_durability::durability::native_world_items::SpellGroundTarget,
    )>,
}

pub(crate) fn snapshot_session_spawn(
    room: &QualifiedNativeEntryRoom,
    native: &NativeGameplayState,
    runtime: &mut ChannelRuntimeV1,
    key: &str,
    session: GameSessionId,
    exact: Option<MovementLocalPosition>,
) -> Result<SpawnPlan, SpawnRefusal> {
    snapshot_spawn(
        room,
        native,
        runtime,
        key,
        Placement::Session { session, exact },
    )
}

pub(crate) fn snapshot_c18a_rat(
    room: &QualifiedNativeEntryRoom,
    native: &NativeGameplayState,
    runtime: &mut ChannelRuntimeV1,
    session: GameSessionId,
) -> Result<SpawnPlan, SpawnRefusal> {
    snapshot_spawn(
        room,
        native,
        runtime,
        C18A_RAT_KEY,
        Placement::C18aRat(session),
    )
}

fn snapshot_spawn(
    room: &QualifiedNativeEntryRoom,
    native: &NativeGameplayState,
    runtime: &mut ChannelRuntimeV1,
    key: &str,
    placement: Placement,
) -> Result<SpawnPlan, SpawnRefusal> {
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
    let exact_rat = matches!(placement, Placement::C18aRat(_));
    let (candidates, player, exact) = match placement {
        Placement::Fixed(position) => (vec![position], None, None),
        Placement::Session { session, .. } | Placement::C18aRat(session) => {
            let exact = match placement {
                Placement::Session { exact, .. } => exact,
                _ => None,
            };
            let (actor, snapshot) = runtime
                .positioned_player_for_session(session)
                .map_err(|error| fence(format!("qualification spawn session actor: {error:?}")))?
                .ok_or(SpawnRefusal::SessionUnknown)?;
            let position = snapshot.position();
            let candidates = if exact_rat {
                vec![C18A_RAT_POSITION]
            } else {
                exact.map_or_else(|| adjacent_cells(position), |tile| vec![tile])
            };
            (candidates, Some((session, actor, position)), exact)
        }
    };
    let mut targets = Vec::with_capacity(candidates.len());
    let mut skipped = "qualification spawn has no eligible cell".to_owned();
    for position in candidates {
        if let Err(reason) = static_cell_refusal(room, position) {
            if exact.is_some() {
                return Err(SpawnRefusal::StaticTile(reason));
            }
            skipped = reason;
            continue;
        }
        let target = oteryn_gameplay_durability::durability::native_world_items::SpellGroundTarget::for_native_tile_read(
            room, runtime, position,
        ).map_err(|error| fence(error.to_owned()))?;
        targets.push((position, target));
    }
    if targets.is_empty() {
        return Err(SpawnRefusal::NoFreeTile(skipped));
    }
    Ok(SpawnPlan {
        exact_rat,
        pin,
        scope: runtime.binding().runtime_scope(),
        scope_generation: runtime.binding().scope_generation().get(),
        exact,
        player,
        targets,
    })
}

pub(crate) async fn read_spawn_tiles(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    root: &DurabilityRoot,
    recovery: &ReconciledCharacterAuthority<'_, '_>,
    node: &NodeIncarnationProof,
    plan: &SpawnPlan,
) -> Result<Vec<MovementLocalPosition>, SpawnRefusal> {
    let fence = SpawnRefusal::Fence;
    let scope = plan.scope;
    // Qualification-only spawn: the topology's seam owns the real lane and is out of reach
    // here, so this test path proves the same Channel fixture's exact tagged scope.
    let permit = crate::durability::spell_owner_commit::SpellLanePermit::of_fresh_scope(scope)
        .await
        .map_err(|error| fence(format!("qualification spawn scope lane: {error:?}")))?;
    let owned = crate::durability::spell_item_transaction::assert_spell_item_scope_in_transaction(
        tx,
        &permit,
        root,
        recovery,
        node,
        scope,
        plan.scope_generation,
    )
    .await
    .map_err(|error| fence(format!("qualification spawn scope fence: {error:?}")))?;
    let mut eligible = Vec::with_capacity(plan.targets.len());
    for (position, target) in &plan.targets {
        let items = crate::durability::spell_item_transaction::read_qualification_scope_tile_in_transaction(
            tx, &owned, target,
        ).await.map_err(|error| fence(format!("qualification spawn current Item tile: {error:?}")))?;
        if !items
            .items
            .iter()
            .any(|item| item.blocks_movement || item.immovable_block_solid)
        {
            eligible.push(*position);
        } else if plan.exact.is_some() {
            return Err(SpawnRefusal::ItemBlocked);
        }
    }
    if eligible.is_empty() {
        return Err(SpawnRefusal::NoFreeTile(
            "qualification spawn blocked by actual current Item owner".into(),
        ));
    }
    Ok(eligible)
}

pub(crate) fn publish_spawn(
    runtime: &mut ChannelRuntimeV1,
    plan: SpawnPlan,
    source: u16,
    key: &str,
    eligible: &[MovementLocalPosition],
) -> Result<CompanionSnapshot, SpawnRefusal> {
    let fence = SpawnRefusal::Fence;
    if runtime.owner_fence().is_err()
        || runtime.content_pin() != &plan.pin
        || runtime.binding().runtime_scope() != plan.scope
        || runtime.binding().scope_generation().get() != plan.scope_generation
    {
        return Err(fence(
            "qualification spawn runtime changed while checking tiles".into(),
        ));
    }
    if let Some((session, actor, position)) = plan.player {
        let current = runtime
            .positioned_player_for_session(session)
            .map_err(|error| fence(format!("qualification spawn session actor: {error:?}")))?;
        if !current.is_some_and(|(current_actor, snapshot)| {
            current_actor == actor && snapshot.position() == position
        }) {
            return Err(SpawnRefusal::SessionUnknown);
        }
        if let Some(tile) = plan.exact {
            if !eligible.contains(&tile) {
                return Err(SpawnRefusal::ItemBlocked);
            }
            if tile == position
                || runtime
                    .position_occupied_by_other(actor, tile)
                    .map_err(SpawnRefusal::Physical)?
            {
                return Err(SpawnRefusal::Occupied);
            }
            return runtime
                .realize_native_qualification_spawn_for_source(source, plan.pin, key, tile)
                .map_err(SpawnRefusal::Physical);
        }
    }
    if plan.exact_rat {
        return place_c18a_rat(runtime, eligible);
    }
    place_first(runtime, &plan.pin, source, key, eligible)
}

fn place_c18a_rat(
    runtime: &mut ChannelRuntimeV1,
    eligible: &[MovementLocalPosition],
) -> Result<CompanionSnapshot, SpawnRefusal> {
    let refusal = SpawnRefusal::ExactPlacement;
    if eligible != [C18A_RAT_POSITION] {
        return Err(refusal(
            "fixed tile failed the durable placement check".into(),
        ));
    }
    let points = runtime.spawn_point_creatures();
    let mut origins = points
        .iter()
        .filter(|(_, origin, _)| *origin == C18A_RAT_POSITION);
    let Some((_, _, Some(rat))) = origins.next() else {
        return Err(refusal(
            "fixed native spawn has no live Rat generation".into(),
        ));
    };
    let snapshot = runtime
        .companion_snapshot(*rat)
        .map_err(|error| refusal(format!("read exact Rat: {error:?}")))?;
    if origins.next().is_some() || snapshot.state.policy.definition_key != C18A_RAT_KEY {
        return Err(refusal("fixed native spawn is not one Rat".into()));
    }
    if runtime
        .position_occupied_by_other(*rat, C18A_RAT_POSITION)
        .map_err(|error| refusal(format!("read fixed occupancy: {error:?}")))?
    {
        return Err(refusal("fixed tile is occupied by another actor".into()));
    }
    if snapshot.position == C18A_RAT_POSITION {
        return Ok(snapshot);
    }
    let before = runtime
        .read_actor_position(*rat)
        .map_err(|error| refusal(format!("read exact Rat position: {error:?}")))?;
    runtime
        .borrow_movement_position()
        .commit_creature_relocation(*rat, before, C18A_RAT_POSITION)
        .map_err(|error| refusal(format!("place exact Rat: {error:?}")))?;
    runtime
        .companion_snapshot(*rat)
        .map_err(|error| refusal(format!("read placed Rat: {error:?}")))
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
    fn runtime_with_player_and_policy(
        key: &str,
    ) -> (ChannelRuntimeV1, ChannelContentPin, GameSessionId) {
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
            .install_companion_policies(policy(key, pin.server_artifact_digest()))
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
    fn qualification_spawn_c18a_restores_only_the_authored_rat_generation() {
        let (mut runtime, pin, _) = runtime_with_player_and_policy(C18A_RAT_KEY);
        let other_origin = MovementLocalPosition {
            x: C18A_RAT_POSITION.x - 1,
            ..C18A_RAT_POSITION
        };
        let other = place_first(&mut runtime, &pin, 40_001, C18A_RAT_KEY, &[other_origin])
            .expect("other Rat");
        assert!(matches!(
            place_c18a_rat(&mut runtime, &[C18A_RAT_POSITION]),
            Err(SpawnRefusal::ExactPlacement(_))
        ));
        let authored = place_first(
            &mut runtime,
            &pin,
            40_000,
            C18A_RAT_KEY,
            &[C18A_RAT_POSITION],
        )
        .expect("authored Rat");
        let displaced = MovementLocalPosition {
            y: C18A_RAT_POSITION.y + 1,
            ..C18A_RAT_POSITION
        };
        let before = runtime
            .read_actor_position(authored.actor)
            .expect("position");
        runtime
            .borrow_movement_position()
            .commit_creature_relocation(authored.actor, before, displaced)
            .expect("displace authored Rat");
        let placed =
            place_c18a_rat(&mut runtime, &[C18A_RAT_POSITION]).expect("place authored Rat");
        assert_eq!(placed.actor, authored.actor);
        assert_eq!(placed.position, C18A_RAT_POSITION);
        assert_eq!(placed.health, authored.health);
        assert_eq!(
            runtime
                .read_actor_position(other.actor)
                .expect("other Rat")
                .position(),
            other_origin
        );
    }

    #[test]
    fn a_non_rat_creature_is_realised_adjacent_to_the_session_actor() {
        let (mut runtime, pin, session) = runtime_with_player_and_policy(SPIDER);
        let (_, actor) = runtime
            .positioned_player_for_session(session)
            .expect("session read")
            .expect("positioned player");
        assert_eq!(actor.position(), PLAYER_AT);
        let cells = adjacent_cells(actor.position());
        assert_eq!(cells.len(), 8);
        let first = place_first(&mut runtime, &pin, 40_000, SPIDER, &cells).expect("first spawn");
        assert_eq!(first.state.policy.definition_key, SPIDER);
        assert!(first.state.master.is_none());
        assert_eq!(first.position, cells[0]);
        assert!(adjacent(PLAYER_AT, first.position));
        // The first cell is now occupied, so the next creature takes the next free neighbour.
        let second = place_first(&mut runtime, &pin, 40_001, SPIDER, &cells).expect("second spawn");
        assert_eq!(second.position, cells[1]);
        assert!(adjacent(PLAYER_AT, second.position));
        assert_ne!(
            first.actor.placement_identity(),
            second.actor.placement_identity()
        );
    }

    #[test]
    fn an_unknown_creature_key_is_refused_before_any_placement() {
        let (runtime, _, _) = runtime_with_player_and_policy(SPIDER);
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
