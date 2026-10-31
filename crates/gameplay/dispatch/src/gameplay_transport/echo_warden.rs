//! Echo callbacks composed with the existing Channel actor, map and kill owners.
use super::{ComposedFreshAdmission, operator_event};
use crate::content::item_ref::ItemDefinitionIndex;
use crate::foundation::{ChannelRuntimeV1, ExactActorRef, GameSessionId, MovementLocalPosition};
use crate::map::overlay::{AddedItem, ChannelOverlay, TilePos, VolatileItem};
use crate::map::view::{EntryFacts, appearance_id};
use crate::monster_owner_cycle::respawn_job::RespawnExit;
use oteryn_foundation::CancellationToken;
use oteryn_gameplay_domain::domain::echo_warden::{PORTAL_ITEM_ID, SPAWN_STEP_MS};
use oteryn_protocol_oteryn::world_map::MapDefinition;
use oteryn_simulation_determinism::{
    DecisionOccurrenceId, GameplayDecisionRoot, deterministic_decision_u64,
};
use std::num::NonZeroU32;

const PORTAL_KEY: &str = "oteryn:item.tibia.i54133";
const PORTAL_MARKER: &[u8] = b"oteryn:echo-portal:v1:";

/// Server-owned draws use the physical death occurrence and the admitted generation.
pub(super) fn draw(
    runtime: &ChannelRuntimeV1,
    occurrence: [u8; 16],
    purpose: &str,
    minimum: u32,
    maximum: u32,
) -> Option<u32> {
    let entropy = deterministic_decision_u64(
        &GameplayDecisionRoot::from_bytes(runtime.content_pin().server_artifact_digest()),
        DecisionOccurrenceId::from_bytes(occurrence),
        purpose,
        0,
    )
    .ok()?;
    u32::try_from(oteryn_gameplay_domain::domain::uniform_draw::uniform_draw(
        entropy,
        i64::from(minimum),
        i64::from(maximum),
    ))
    .ok()
}

fn tile(position: MovementLocalPosition) -> Option<TilePos> {
    Some(TilePos {
        x: u16::try_from(position.x).ok()?,
        y: u16::try_from(position.y).ok()?,
        floor: i8::try_from(position.floor.checked_neg()?).ok()?,
    })
}

fn has_ground(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    position: MovementLocalPosition,
) -> bool {
    // A bound world portal needs an admitted Item identity, not carried-item custody facts.
    if admission
        .active_generation
        .and_then(|generation| generation.native_gameplay())
        .and_then(|native| native.item_index())
        .and_then(|index| index.revision(PORTAL_KEY))
        .is_none()
    {
        return false;
    }
    let Some(position) = tile(position) else {
        return false;
    };
    admission
        .bundle_map()
        .and_then(|map| {
            map.overlay
                .base()
                .tile(position.x, position.y, position.floor)
        })
        .is_some_and(|tile| tile.ground().is_some())
}

/// Called inside the existing successful movement work item, never from item USE.
pub(super) fn step_in(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    runtime: &mut ChannelRuntimeV1,
    actor: ExactActorRef,
    session: GameSessionId,
    now_ms: u64,
) {
    let Ok(position) = runtime.read_actor_position(actor) else {
        return;
    };
    let Some(occurrence) = runtime
        .echo_portals(now_ms)
        .find(|portal| portal.position == position.position())
        .map(|portal| portal.occurrence)
    else {
        return;
    };
    let Some(minions) = draw(runtime, occurrence, "ECHO_WARDEN_ADDS", 7, 12)
        .and_then(|count| u8::try_from(count).ok())
    else {
        return;
    };
    if runtime.activate_echo_portal(actor, session, now_ms, minions) == Ok(true) {
        admission.states.loop_ground.invalidate();
        if let Err(error) = runtime.drive_echo_wardens(now_ms, |at| has_ground(admission, at)) {
            operator_event(&format!("echo_warden_step_failed reason={error:?}"));
        }
    }
}

/// Portals are volatile encounter objects. They never create a carried Item or custody row.
pub(super) fn append_portals(
    overlay: &mut ChannelOverlay,
    runtime: &ChannelRuntimeV1,
    index: Option<&ItemDefinitionIndex>,
    now_ms: u64,
) {
    let Some(index) = index else { return };
    let Some(revision) = index.revision(PORTAL_KEY) else {
        return;
    };
    let Some(reference) = index.definition_ref("Item", PORTAL_KEY, revision) else {
        return;
    };
    for portal in runtime.echo_portals(now_ms) {
        let Some(position) = tile(portal.position) else {
            continue;
        };
        let mut attributes = PORTAL_MARKER.to_vec();
        attributes.extend_from_slice(&reference.get().to_be_bytes());
        attributes.extend_from_slice(&portal.occurrence);
        // The actor owner's monotonic timer removes it; overlay decay uses Unix time.
        if let Err(error) = overlay.add_volatile(
            position,
            VolatileItem {
                id: u32::from(PORTAL_ITEM_ID),
                count: 1,
                attributes,
            },
            None,
        ) {
            operator_event(&format!("echo_warden_portal_view_failed reason={error:?}"));
        }
    }
}

pub(super) fn portal_entry(entry: &AddedItem) -> Option<EntryFacts> {
    let AddedItem::Volatile { item, .. } = entry else {
        return None;
    };
    if item.id != u32::from(PORTAL_ITEM_ID) || item.count != 1 {
        return None;
    }
    let bytes = item.attributes.strip_prefix(PORTAL_MARKER)?;
    if bytes.len() != 20 {
        return None;
    }
    let reference = NonZeroU32::new(u32::from_be_bytes(bytes[..4].try_into().ok()?))?;
    Some(EntryFacts {
        definition: MapDefinition::Item(reference),
        terrain_kind: None,
        appearance_id: appearance_id(PORTAL_KEY, None),
        blocks_projectile: false,
        pickupable: false,
        bound: true,
        count: 1,
        sub_type: 0,
    })
}

impl ComposedFreshAdmission<'_, '_, '_> {
    /// Runs beside World respawns, including when every player has left the area.
    pub(super) async fn run_echo_wardens(&self, shutdown: &CancellationToken) -> RespawnExit {
        loop {
            let pass = async {
                let mut runtime = self.runtime.lock().await;
                runtime.owner_fence().map_err(|_| RespawnExit::StaleOwner)?;
                let now_ms = self.owner_now().get() / 1000;
                if runtime
                    .drive_echo_wardens(now_ms, |at| has_ground(self, at))
                    .map_err(|_| RespawnExit::StaleOwner)?
                {
                    self.states.loop_ground.invalidate();
                }
                // At most one source trickle interval before noticing a newly queued death;
                // existing deadlines wake exactly, without a player-session timer owner.
                let delay = runtime.next_echo_due_ms().map_or(SPAWN_STEP_MS, |due| {
                    due.saturating_sub(now_ms).min(SPAWN_STEP_MS)
                });
                Ok::<_, RespawnExit>(delay)
            };
            let delay = match super::world_respawns::stopping_first(pass, shutdown).await {
                None => return RespawnExit::Shutdown,
                Some(Err(exit)) => return exit,
                Some(Ok(delay)) => delay,
            };
            if super::world_respawns::stopping_first(
                tokio::time::sleep(std::time::Duration::from_millis(delay)),
                shutdown,
            )
            .await
            .is_none()
            {
                return RespawnExit::Shutdown;
            }
        }
    }
}

pub(super) mod drops;
pub(super) mod rewards;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn portal_uses_the_existing_runtime_to_bundle_floor_conversion() {
        assert_eq!(
            tile(MovementLocalPosition {
                x: 123,
                y: 456,
                floor: 7
            }),
            Some(TilePos {
                x: 123,
                y: 456,
                floor: -7
            })
        );
        assert_eq!(
            tile(MovementLocalPosition {
                x: -1,
                y: 456,
                floor: 7
            }),
            None
        );
        assert_eq!(
            tile(MovementLocalPosition {
                x: 123,
                y: 456,
                floor: i16::MIN
            }),
            None
        );
    }
}
