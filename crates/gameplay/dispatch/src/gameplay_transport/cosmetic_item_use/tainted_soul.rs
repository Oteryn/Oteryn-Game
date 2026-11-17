//! Spawn and expiry after the Sacred Antler Talisman credit commits.

use super::super::ComposedFreshAdmission;
use crate::content::{CollisionClass, LogicalCell, native_gameplay::NativeGameplayState};
use crate::foundation::{ChannelRuntimeV1, ExactActorRef, MovementLocalPosition};
use std::{sync::Mutex, time::Duration};

struct RedemptionTiming {
    tainted_remove_after: Duration,
    redeemed_remove_after: Option<Duration>,
}

fn redemption_timing(turn: Duration, has_spawn_room: bool) -> RedemptionTiming {
    RedemptionTiming {
        tainted_remove_after: turn,
        redeemed_remove_after: has_spawn_room.then_some(Duration::from_secs(30)),
    }
}

const TAINTED: &str = "oteryn:creature.tainted_soul";
const REDEEMED: &str = "oteryn:creature.redeemed_soul";
const OWNER_TURN: Duration =
    Duration::from_micros(crate::ai_think::CREATUREAI0_THINK_WINDOW_MICROS);

#[derive(Default)]
pub(in crate::gameplay_transport) struct Expiry(Mutex<Vec<(u64, ExactActorRef, &'static str)>>);

impl Expiry {
    pub(in crate::gameplay_transport) fn next_due(&self) -> Option<u64> {
        self.0.lock().ok()?.iter().map(|entry| entry.0).min()
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) fn publish_tainted_soul_redemption(
        &self,
        runtime: &mut ChannelRuntimeV1,
        player: ExactActorRef,
        tainted: ExactActorRef,
        native: &NativeGameplayState,
    ) -> Result<(), ()> {
        let mut expiry = self.states.tainted_soul_expiry.0.lock().map_err(|_| ())?;
        let center = runtime
            .read_actor_position(player)
            .map_err(|_| ())?
            .position();
        let redeemed = (|| {
            let definition = &native
                .creature_profiles()
                .records
                .iter()
                .find(|record| record.profile.target.key == REDEEMED)?
                .profile
                .target;
            let spec = oteryn_gameplay_entities::NativeEncounterSpawnSpec::qualified(
                TAINTED,
                REDEEMED,
                &definition.revision,
                native.source_digest(),
            )
            .ok()?;
            let stamp = runtime.issue_owner_work().ok()?;
            let cells = self.movement_cells();
            for (dx, dy) in [
                (-1, -1),
                (0, -1),
                (1, -1),
                (-1, 0),
                (1, 0),
                (-1, 1),
                (0, 1),
                (1, 1),
            ] {
                let at = MovementLocalPosition {
                    x: center.x.saturating_add(dx),
                    y: center.y.saturating_add(dy),
                    floor: center.floor,
                };
                if let Ok(actor) = runtime.admit_current_native_encounter_masterless(
                    stamp,
                    tainted,
                    &spec,
                    at,
                    |current, at| {
                        Some(
                            current.current_top_actor_on_cell(at).ok()?.is_none()
                                && cells
                                    .index()
                                    .lookup(
                                        cells.scope(),
                                        LogicalCell {
                                            x: at.x,
                                            y: at.y,
                                            z: i32::from(at.floor),
                                        },
                                    )
                                    .ok()?
                                    == CollisionClass::Walkable,
                        )
                    },
                ) {
                    return Some(actor);
                }
            }
            None
        })();
        let timing = redemption_timing(OWNER_TURN, redeemed.is_some());
        let now = self.owner_now().get();
        let due = |delay: Duration| {
            now.saturating_add(u64::try_from(delay.as_micros()).unwrap_or(u64::MAX))
        };
        expiry.push((due(timing.tainted_remove_after), tainted, TAINTED));
        if let Some((actor, lifetime)) = redeemed.zip(timing.redeemed_remove_after) {
            expiry.push((due(lifetime), actor, REDEEMED));
        }
        Ok(())
    }

    pub(in crate::gameplay_transport) async fn drain_tainted_soul_expiry(&self, now: u64) {
        if self
            .states
            .tainted_soul_expiry
            .next_due()
            .is_none_or(|due| due > now)
        {
            return;
        }
        let Some(_permit) = self.spell_lane_permit().await else {
            return;
        };
        let mut runtime = self.runtime.lock().await;
        let Ok(mut expiry) = self.states.tainted_soul_expiry.0.lock() else {
            return;
        };
        let content = runtime.content_pin().server_artifact_digest();
        expiry.retain_mut(|(due, actor, key)| {
            if *due > now {
                return true;
            }
            if !runtime.matches_live_creature_identity(*actor, key.as_bytes()) {
                return false;
            }
            let Ok(stamp) = runtime.issue_owner_work() else {
                return false;
            };
            if runtime
                .remove_current_native_encounter_subject(stamp, *actor, key, content)
                .is_ok()
            {
                return false;
            }
            *due = now.saturating_add(crate::ai_think::CREATUREAI0_THINK_WINDOW_MICROS);
            true
        });
    }
}

#[cfg(test)]
mod tests {
    use super::redemption_timing;
    use std::time::Duration;

    #[test]
    fn tainted_soul_redemption_bounds_actor_lifetimes_with_or_without_room() {
        let turn = Duration::from_millis(200);
        let blocked = redemption_timing(turn, false);
        assert_eq!(blocked.tainted_remove_after, turn);
        assert_eq!(blocked.redeemed_remove_after, None);

        let open = redemption_timing(turn, true);
        assert_eq!(open.tainted_remove_after, turn);
        assert_eq!(open.redeemed_remove_after, Some(Duration::from_secs(30)));
    }
}
