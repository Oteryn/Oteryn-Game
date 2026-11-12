//! SV16/G51 live-child lifetime, independently of the donor's two-minute portal TTL.
use super::*;
use oteryn_gameplay_domain::domain::echo_warden::CHILD_TTL_MS;

impl ChannelRuntimeV1 {
    pub(super) fn track_echo_child_expiry(&mut self, actor: ExactActorRef, now_ms: u64) {
        self.echo_wardens
            .expirations
            .push((actor, now_ms.saturating_add(CHILD_TTL_MS)));
    }

    pub(super) fn expire_echo_children(&mut self, now_ms: u64) {
        for (actor, due) in std::mem::take(&mut self.echo_wardens.expirations) {
            let Ok(snapshot) = self.companion_snapshot_including_dead(actor) else {
                continue;
            };
            // The existing dead-child cleanup owns pending kill and corpse settlement.
            // An expired exact reference must never act on a replacement incarnation.
            if snapshot.health <= 0 {
                continue;
            }
            if now_ms < due {
                self.echo_wardens.expirations.push((actor, due));
            } else if self.carrier.corpse_projection_pins.contains(&actor)
                || self.remove_encounter_actor(actor).is_err()
            {
                // A reserved actor is retried on the existing owner cadence, without
                // keeping an already elapsed deadline that would busy-loop dispatch.
                self.echo_wardens
                    .expirations
                    .push((actor, now_ms.saturating_add(SPAWN_STEP_MS)));
            }
        }
    }
}
