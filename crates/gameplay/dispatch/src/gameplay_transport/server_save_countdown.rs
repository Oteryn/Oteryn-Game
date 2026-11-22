//! PARITY-EVENT-CALLBACKS-1 (coordinator decision, NIGHT-W1): the server-save countdown.
//! Canary `global_server_save.lua:29,48` broadcasts once a minute from 5 to 1 minutes before
//! the save. Countdown reads and due-save publication have separate authority.
use super::ComposedFreshAdmission;
use crate::durability::server_save_sweep::{
    GroundSweepBinding, GroundSweepFacts, GroundSweepItemFacts, GroundSweepPlacement,
    GroundSweepPolicy, GroundSweepRoot, GroundSweepTileFacts, ServerSaveCleanup, ServerSaveOwner,
};
use crate::durability::world_reset::PlannedWorldReset;
use oteryn_foundation::CancellationToken;
use oteryn_protocol_oteryn::server_save_countdown::ServerSaveCountdown;

/// The planned read cadence; a minute's window is 60 s, so a notice is at most this late.
const SERVER_SAVE_COUNTDOWN_POLL: std::time::Duration = std::time::Duration::from_secs(5);

/// One countdown minute of one planned reset. `reset_at_unix_ms` keeps the receipt apart
/// from a later day's countdown that ends the same epoch (a skipped reset).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DueServerSaveCountdown {
    /// The planned reset instant this minute counts down to.
    pub reset_at_unix_ms: i64,
    /// The wire notice.
    pub notice: ServerSaveCountdown,
}

impl DueServerSaveCountdown {
    /// Whether a session whose last notice was `sent` takes this one: a new reset, or a
    /// strictly smaller minute of the same one. A clock step back never repeats a minute.
    pub fn follows(&self, sent: Option<(i64, u8)>) -> bool {
        sent.is_none_or(|(reset_at, minutes)| {
            reset_at != self.reset_at_unix_ms || self.notice.minutes_remaining < minutes
        })
    }

    /// The (planned reset instant, minute) a session keeps after sending this notice.
    pub fn receipt(&self) -> (i64, u8) {
        (self.reset_at_unix_ms, self.notice.minutes_remaining)
    }
}

/// The Channel's currently due notice. A leaf lock, never held across an await.
#[derive(Default)]
pub(crate) struct ServerSaveCountdownHub {
    due: std::sync::Mutex<Option<DueServerSaveCountdown>>,
}

impl ServerSaveCountdownHub {
    pub(crate) fn current(&self) -> Option<DueServerSaveCountdown> {
        *self
            .due
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn publish(&self, due: Option<DueServerSaveCountdown>) {
        *self
            .due
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = due;
    }
}

/// The pure pass. The first successful read fixes the serve start, so a minute whose window
/// opened before this incarnation (a restart) is never published.
#[derive(Default)]
pub(crate) struct ServerSaveCountdownPass {
    serve_started_unix_ms: Option<i64>,
}

impl ServerSaveCountdownPass {
    pub(crate) fn observe(
        &mut self,
        planned: Option<PlannedWorldReset>,
    ) -> Option<DueServerSaveCountdown> {
        let planned = planned?;
        let started = *self
            .serve_started_unix_ms
            .get_or_insert(planned.database_now_unix_ms);
        let minutes_remaining = planned.warning_due(started)?;
        Some(DueServerSaveCountdown {
            reset_at_unix_ms: planned.reset_at_unix_ms,
            notice: ServerSaveCountdown {
                ending_epoch: planned.ending_epoch,
                minutes_remaining,
            },
        })
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    /// Polled beside the respawn job for the serve lifetime. A failed read publishes nothing
    /// and is retried; it never stops serving.
    pub(super) async fn run_server_save_countdown(&self, shutdown: &CancellationToken) {
        let mut pass = ServerSaveCountdownPass::default();
        let enabled = |key| std::env::var(key).is_ok_and(|value| value == "true");
        let cleanup = ServerSaveCleanup {
            enabled: enabled("OTERYN_SERVER_SAVE_CLEAN_MAP"),
            policy: GroundSweepPolicy {
                clean_protection_zones: enabled("OTERYN_CLEAN_PROTECTION_ZONES"),
            },
        };
        loop {
            let Some(read) = super::world_respawns::stopping_first(
                self.root.planned_world_reset(self.world_id),
                shutdown,
            )
            .await
            else {
                return;
            };
            match read {
                Ok(planned) => self.states.server_save.publish(pass.observe(planned)),
                Err(error) => {
                    self.states.server_save.publish(None);
                    super::operator_event(&format!(
                        "server_save_countdown_read_deferred reason={error}"
                    ));
                }
            }
            let Some(_save) =
                super::world_respawns::stopping_first(self.advance_server_save(cleanup), shutdown)
                    .await
            else {
                return;
            };
            if super::world_respawns::stopping_first(
                tokio::time::sleep(SERVER_SAVE_COUNTDOWN_POLL),
                shutdown,
            )
            .await
            .is_none()
            {
                return;
            }
        }
    }

    async fn advance_server_save(
        &self,
        cleanup: ServerSaveCleanup,
    ) -> Result<(), crate::durability::DurabilityError> {
        let Some(room) = self.rooms.gate_monster_ai() else {
            return Ok(());
        };
        let (binding, server_digest) = {
            let runtime = self.runtime.lock().await;
            let scope = runtime.binding();
            let pin = runtime.content_pin();
            if scope.world_id() != self.world_id
                || scope.channel_id() != Some(self.channel_id)
                || pin.server_artifact_digest() != room.compiled().server_digest()
                || pin.map_revision_digest() != room.map_revision_digest()
                || pin.frame_binding_digest() != room.frame_binding().digest()
                || room.movement_cells().scope().world_id != self.world_id
            {
                return Ok(());
            }
            (
                GroundSweepBinding {
                    world: self.world_id,
                    channel: self.channel_id,
                    generation: scope.scope_generation().get(),
                    map_revision: revision(pin.map_revision_digest()),
                    content_revision: revision(pin.server_artifact_digest()),
                    frame: pin.frame_binding_digest().to_vec(),
                },
                pin.server_artifact_digest(),
            )
        };
        let owner = ServerSaveOwner {
            node: self.holder,
            epoch: self.reset_epoch,
            epoch_map_revision: room.movement_cells().scope().map_revision.as_str(),
            server_digest,
        };
        if !self
            .root
            .publish_due_server_save(self.world_id, self.channel_id, &owner, cleanup)
            .await?
        {
            return Ok(());
        }
        let Some(permit) = self.spell_lane_permit().await else {
            return Ok(());
        };
        let facts = PinnedSweepFacts {
            binding,
            room,
            profiles: room.item_profile_source(),
            bundle_revision: self
                .bundle_map()
                .map(|bundle| crate::map::overlay::map_revision(bundle.overlay.base())),
        };
        // Also invalidates if cancellation or a lost COMMIT response interrupts
        // publication. The next Ground read rebuilds under the cache epoch.
        let _projection = super::loop_move::GroundChange(Some(&self.states.loop_ground));
        self.root
            .run_server_save_sweep(&owner, &permit, &facts)
            .await
    }
}

struct PinnedSweepFacts<'a> {
    binding: GroundSweepBinding,
    room: &'a crate::content::QualifiedNativeEntryRoom,
    profiles: Option<crate::content::native_gameplay::NativeItemProfileSource>,
    bundle_revision: Option<String>,
}

impl GroundSweepFacts for PinnedSweepFacts<'_> {
    fn binding(&self) -> &GroundSweepBinding {
        &self.binding
    }

    fn alternate_map_revision(&self) -> Option<&str> {
        self.bundle_revision.as_deref()
    }

    fn tile(&self, placement: &GroundSweepPlacement) -> GroundSweepTileFacts {
        let Ok(cell) = crate::durability::spell_items_abi::decode_ground_cell(&placement.position)
        else {
            return GroundSweepTileFacts::default();
        };
        let cell = crate::content::LogicalCell {
            x: cell.x,
            y: cell.y,
            z: cell.z,
        };
        let cells = self.room.movement_cells();
        let scope = cells.scope();
        GroundSweepTileFacts {
            house: cells
                .house_tiles()
                .house_at(scope, cell)
                .ok()
                .map(|house| house.is_some()),
            protection_zone: cells
                .spell_tiles()
                .lookup(scope, cell)
                .ok()
                .map(|tile| tile.flags().protection_zone),
        }
    }

    fn item(&self, root: &GroundSweepRoot) -> GroundSweepItemFacts {
        use crate::content::ReferenceItemField::Known;
        let policy = self.profiles.as_ref().and_then(|profiles| {
            profiles.item_policy(&root.definition_key, &root.definition_revision)
        });
        let (removable, pickupable) = match policy
            .as_ref()
            .map(|policy| &policy.record().semantics.physical)
        {
            Some(Known(physical)) => {
                let known = |field: &crate::content::ReferenceItemField<bool>| match field {
                    Known(value) => Some(*value),
                    _ => None,
                };
                (known(&physical.movable), known(&physical.pickupable))
            }
            _ => (None, None),
        };
        let qualified_absence = root.runtime_without_source_ids.then_some(false);
        GroundSweepItemFacts {
            loaded_from_map: qualified_absence,
            removable,
            pickupable,
            has_action_id: qualified_absence,
            has_unique_id: qualified_absence,
        }
    }
}

fn revision(digest: [u8; 32]) -> String {
    let encoded: String = digest.iter().map(|byte| format!("{byte:02x}")).collect();
    format!("sha256:{encoded}")
}

#[cfg(test)]
#[path = "server_save_countdown_tests.rs"]
mod tests;
