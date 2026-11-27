//! Exaltation Forge composition: authenticated command, retained receipt, then native mutation.
use super::{
    ComposedFreshAdmission,
    connection::{AdmittedSession, UseCommand, UseOutcome},
    item_view::{ItemKey, SessionItemView},
};
use crate::durability::DurabilityError;
use crate::durability::character_forge_dust::{ForgeDustLimitError, ForgeResolvedItems};
use crate::durability::item_transfer::CurrentCharacterItemFence;
use crate::foundation::{ChannelRuntimeV1, CommandId, CommandRef, ExactActorRef, GameSessionId};
use crate::map::{overlay::TilePos, view::MapFacts};
use oteryn_protocol_oteryn::item_forge::{CAPABILITY_ITEM_FORGE_V1, Operation, Request};
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};
use std::{collections::HashMap, sync::Mutex};

#[derive(Default)]
pub(super) struct Stations(Mutex<HashMap<GameSessionId, StationUse>>);

struct StationUse {
    actor: ExactActorRef,
    fence: CurrentCharacterItemFence,
    target: WorldObjectTarget,
}

impl Stations {
    pub(super) fn forget(&self, session: GameSessionId) {
        if let Ok(mut stations) = self.0.lock() {
            stations.remove(&session);
        }
    }
}

/// Crystal 201189f8 data/scripts/actions/objects/exaltation_forge.lua:3–9
/// registers these usable pieces of the Global Exaltation Forge object.
/// Fandom Equipment Upgrade: "In order to upgrade an item, players must visit the
/// [[Exaltation Forge]] in the [[Adventurers' Guild]]."
fn station_position(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    runtime: &ChannelRuntimeV1,
    target: &WorldObjectTarget,
) -> Option<TilePos> {
    let map = owner.bundle_map()?;
    let index = super::item_ref_admission::pinned_index(runtime, owner.active_generation)?;
    let facts = map.facts.pinned();
    let resolved = super::world_map::resolve_map_target(
        &super::world_map::MapViewSource {
            overlay: &map.overlay,
            facts: &facts,
            content_generation: runtime.content_pin().client_artifact_digest(),
            reset_epoch: owner.reset_epoch,
        },
        target,
        |_| None::<()>,
    )
    .ok()?;
    let tile = map
        .overlay
        .base()
        .tile(resolved.pos.x, resolved.pos.y, resolved.pos.floor)?;
    let (&id, _) = tile
        .ids()
        .iter()
        .zip(tile.depths())
        .filter(|(_, depth)| **depth == 0)
        .nth(usize::from(resolved.ordinal))?;
    let entry = facts.base_entry(resolved.pos, resolved.ordinal, id)?;
    let oteryn_protocol_oteryn::world_map::MapDefinition::Item(reference) = entry.definition else {
        return None;
    };
    let (key, _) = index.definition_at(reference)?;
    let id = key
        .strip_prefix("oteryn:item.tibia.i")?
        .parse::<u32>()
        .ok()?;
    (!entry.pickupable
        && matches!(
            id,
            37122 | 37128 | 37129 | 37131 | 37132 | 37133 | 37153 | 37157
        ))
    .then_some(resolved.pos)
}

fn station_reachable(runtime: &ChannelRuntimeV1, actor: ExactActorRef, target: TilePos) -> bool {
    runtime.read_actor_position(actor).is_ok_and(|position| {
        let at = position.position();
        oteryn_gameplay_items::reach::within_reach(
            (at.x, at.y, i32::from(at.floor)),
            (
                i32::from(target.x),
                i32::from(target.y),
                -i32::from(target.floor),
            ),
        )
    })
}

fn controls(
    runtime: &ChannelRuntimeV1,
    actor: ExactActorRef,
    fence: CurrentCharacterItemFence,
) -> bool {
    let binding = runtime.binding();
    fence.runtime_scope == binding.runtime_scope()
        && fence.scope_ownership_generation == binding.scope_generation()
        && runtime.owner_fence().is_ok()
        && runtime
            .player_control_facts(actor, fence.game_session_id)
            .is_ok_and(|facts| facts.control_loss.is_none())
}

pub(super) async fn use_station(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    target: &WorldObjectTarget,
) -> Option<UseOutcome> {
    {
        let runtime = owner.runtime.lock().await;
        station_position(owner, &runtime, target)?;
    }
    let Some(fence) = command
        .item_fence
        .filter(|fence| fence.game_session_id == command.game_session_id)
    else {
        return Some(UseOutcome::rejected());
    };
    let Some(_permit) = owner.spell_lane_permit().await else {
        return Some(UseOutcome::rejected());
    };
    let store =
        crate::durability::fresh_admission::FreshAdmissionStore::from_root(owner.root.clone());
    let current = super::AdmissionRetryBudget::for_command(owner.root)
        .retry_command_guard(async || store.current_session(fence.game_session_id).await)
        .await;
    if !current.is_ok_and(|current| {
        current.session_state() == crate::foundation::GameSessionState::Active
            && current.current_control_loss_epoch().is_none()
            && super::item_fence_of(current) == Some(fence)
    }) {
        return Some(UseOutcome::rejected());
    }

    let runtime = owner.runtime.lock().await;
    let Some(position) = station_position(owner, &runtime, target) else {
        return Some(UseOutcome {
            disposition: UseDisposition::StaleState,
            ..UseOutcome::rejected()
        });
    };
    if !controls(&runtime, actor, fence) || owner.states.spell_states.lock().await.is_dead(actor) {
        return Some(UseOutcome::rejected());
    }
    if !station_reachable(&runtime, actor, position) {
        return Some(UseOutcome {
            disposition: UseDisposition::TooFar,
            ..UseOutcome::rejected()
        });
    }
    let mut stations = owner.states.forge_stations.0.lock().ok()?;
    stations.insert(
        fence.game_session_id,
        StationUse {
            actor,
            fence,
            target: target.clone(),
        },
    );
    Some(UseOutcome {
        disposition: UseDisposition::ForgeOpened,
        ..UseOutcome::rejected()
    })
}

async fn at_open_station(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    fence: CurrentCharacterItemFence,
) -> bool {
    let runtime = owner.runtime.lock().await;
    if !controls(&runtime, actor, fence) || owner.states.spell_states.lock().await.is_dead(actor) {
        return false;
    }
    owner.states.forge_stations.0.lock().is_ok_and(|stations| {
        stations.get(&fence.game_session_id).is_some_and(|open| {
            open.fence == fence
                && open.actor == actor
                && station_position(owner, &runtime, &open.target)
                    .is_some_and(|target| station_reachable(&runtime, actor, target))
        })
    })
}

fn station_required(request: Request) -> Option<Vec<u8>> {
    serde_json::to_vec(&serde_json::json!({
        "operation": request.operation.name(), "applied": false, "succeeded": false,
        "reason": oteryn_protocol_oteryn::item_forge::FORGE_STATION_REQUIRED,
        "bonus": 0, "result": null
    }))
    .ok()
}

macro_rules! authority_methods {
    () => {
        fn use_forge_station(
            &self,
            _actor: crate::foundation::ExactActorRef,
            _command: crate::gameplay_transport::connection::UseCommand,
            _target: &oteryn_protocol_oteryn::world_object::WorldObjectTarget,
        ) -> impl std::future::Future<Output = Option<crate::gameplay_transport::connection::UseOutcome>> {
            async { None }
        }
        fn item_forge_available(&self) -> bool {
            false
        }
        fn item_forge_balance(
            &self,
            _admitted: AdmittedSession,
            _command_id: u64,
        ) -> impl std::future::Future<Output = Option<Vec<u8>>> {
            async { Some(Vec::new()) }
        }
        fn item_forge(
            &self,
            _admitted: AdmittedSession,
            _command_id: u64,
            _request: oteryn_protocol_oteryn::item_forge::Request,
            _item_view: Option<&crate::gameplay_transport::item_view::SessionItemView>,
        ) -> impl std::future::Future<Output = Option<Vec<u8>>> {
            async { Some(Vec::new()) }
        }
    };
}
pub(super) use authority_methods;

macro_rules! composed_methods {
    () => {
        async fn use_forge_station(
            &self,
            actor: crate::foundation::ExactActorRef,
            command: crate::gameplay_transport::connection::UseCommand,
            target: &oteryn_protocol_oteryn::world_object::WorldObjectTarget,
        ) -> Option<crate::gameplay_transport::connection::UseOutcome> {
            item_forge::use_station(self, actor, command, target).await
        }
        fn item_forge_available(&self) -> bool {
            self.active_generation
                .and_then(|generation| generation.native_gameplay())
                .is_some()
        }
        async fn item_forge_balance(
            &self,
            admitted: AdmittedSession,
            command_id: u64,
        ) -> Option<Vec<u8>> {
            item_forge::balance(self, admitted, command_id).await
        }
        async fn item_forge(
            &self,
            admitted: AdmittedSession,
            command_id: u64,
            request: oteryn_protocol_oteryn::item_forge::Request,
            item_view: Option<&crate::gameplay_transport::item_view::SessionItemView>,
        ) -> Option<Vec<u8>> {
            item_forge::execute(self, admitted, command_id, request, item_view).await
        }
    };
}
pub(super) use composed_methods;

fn unavailable(command_id: u64, stage: &'static str, error: &DurabilityError) -> Option<Vec<u8>> {
    super::operator_event(&format!(
        "event=forge_owner_unavailable command_id={command_id} stage={stage} error={error:?}"
    ));
    None
}

pub(super) async fn balance(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    command_id: u64,
) -> Option<Vec<u8>> {
    let (Some(fence), Some(actor), Ok(id)) = (
        admitted.item_fence,
        admitted.runtime_actor,
        CommandId::new(command_id),
    ) else {
        return Some(Vec::new());
    };
    if !admitted
        .continuity
        .selected_capabilities
        .contains(CAPABILITY_ITEM_FORGE_V1)
        || fence.game_session_id != admitted.game_session_id
        || fence.connection_generation.get() != admitted.continuity.connection_generation
    {
        return Some(Vec::new());
    }
    {
        let runtime = owner.runtime.lock().await;
        let binding = runtime.binding();
        if admitted.world_id != binding.world_id()
            || Some(admitted.channel_id) != binding.channel_id()
            || fence.runtime_scope != binding.runtime_scope()
            || fence.scope_ownership_generation != binding.scope_generation()
            || runtime.owner_fence().is_err()
            || runtime
                .player_control_facts(actor, admitted.game_session_id)
                .is_err()
        {
            return Some(Vec::new());
        }
    }
    match owner
        .root
        .read_current_forge_dust(
            owner.character,
            owner.holder,
            fence,
            CommandRef::new(admitted.game_session_id, id),
        )
        .await
    {
        Ok(value) => serde_json::to_vec(&value).ok(),
        Err(ForgeDustLimitError::Unavailable(error)) => unavailable(command_id, "balance", &error),
        Err(_) => Some(Vec::new()),
    }
}

pub(super) async fn execute(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    command_id: u64,
    request: Request,
    item_view: Option<&SessionItemView>,
) -> Option<Vec<u8>> {
    let (Some(fence), Some(actor), Ok(id)) = (
        admitted.item_fence,
        admitted.runtime_actor,
        CommandId::new(command_id),
    ) else {
        return Some(Vec::new());
    };
    if !request.is_canonical()
        || !admitted
            .continuity
            .selected_capabilities
            .contains(CAPABILITY_ITEM_FORGE_V1)
        || fence.game_session_id != admitted.game_session_id
        || fence.connection_generation.get() != admitted.continuity.connection_generation
    {
        return Some(Vec::new());
    }
    // The item mutation lane precedes Channel and database locks.
    let permit = match owner.try_spell_lane_permit().await {
        Ok(permit) => permit,
        Err(error) => return unavailable(command_id, "lane", &error),
    };
    {
        let runtime = owner.runtime.lock().await;
        let binding = runtime.binding();
        if admitted.world_id != binding.world_id()
            || Some(admitted.channel_id) != binding.channel_id()
            || fence.runtime_scope != binding.runtime_scope()
            || fence.scope_ownership_generation != binding.scope_generation()
            || runtime.owner_fence().is_err()
        {
            return Some(Vec::new());
        }
    }
    let command = CommandRef::new(admitted.game_session_id, id);
    // The read independently proves current session, lease, scope and node authority.
    let retained = match owner
        .root
        .read_item_forge_reply(owner.character, owner.holder, fence, command, request)
        .await
    {
        Ok(result) => result,
        Err(ForgeDustLimitError::Unavailable(error)) => {
            return unavailable(command_id, "receipt", &error);
        }
        Err(_) => return Some(Vec::new()),
    };
    // A consumed item, changed content, new trade or cooldown must not change a retained reply.
    if let Some(result) = retained {
        if request.operation != Operation::QueryHistory
            && !at_open_station(owner, actor, fence).await
        {
            return station_required(request);
        }
        return serde_json::to_vec(&result).ok();
    }
    // Resolve only after receipt lookup. A reconnect has new handles, and a
    // consumed source no longer has a live handle, but both must still replay.
    let resolved_items = request
        .first_item
        .zip(request.second_item)
        .and_then(|(first, second)| {
            let view = item_view?;
            let (ItemKey::Instance(first), ItemKey::Instance(second)) =
                (view.resolve(first)?, view.resolve(second)?)
            else {
                return None;
            };
            Some(ForgeResolvedItems { first, second })
        });
    if request.operation != Operation::QueryHistory
        && owner.states.spell_states.lock().await.is_dead(actor)
    {
        return Some(Vec::new());
    }
    let native = {
        let runtime = owner.runtime.lock().await;
        if !runtime
            .player_control_facts(actor, admitted.game_session_id)
            .is_ok_and(|facts| facts.control_loss.is_none())
            || super::item_ref_admission::pinned_index(&runtime, owner.active_generation).is_none()
        {
            return Some(Vec::new());
        }
        owner.active_generation?.native_gameplay()?
    };
    // The service resolves coins and cores internally, so it cannot consume an offered trade item.
    if request.operation.changes_inventory()
        && !owner
            .states
            .player_trades
            .physical_service(admitted.game_session_id)
    {
        return Some(Vec::new());
    }
    if request.operation != Operation::QueryHistory && !at_open_station(owner, actor, fence).await {
        return station_required(request);
    }
    match owner
        .root
        .execute_item_forge(
            &permit,
            owner.character,
            owner.holder,
            fence,
            command,
            request,
            resolved_items,
            native,
            &super::item_ref_admission::NativeItemWeights(native)
                .with_wheel(owner.premium_coordinator),
        )
        .await
    {
        Ok(result) => serde_json::to_vec(&result).ok(),
        Err(ForgeDustLimitError::Unavailable(error)) => unavailable(command_id, "execute", &error),
        Err(_) => Some(Vec::new()),
    }
}

pub(super) fn offered(
    base: &[super::capabilities::OfferedCapability],
    available: bool,
) -> Vec<super::capabilities::OfferedCapability> {
    // Capability 152 remains bundle-world only. Availability may remove an offer, never add
    // it to the production fallback or an Item-view offer that does not contain it.
    base.iter()
        .copied()
        .filter(|capability| available || capability.id != CAPABILITY_ITEM_FORGE_V1)
        .collect()
}

#[cfg(test)]
#[path = "item_forge_tests.rs"]
mod tests;
