//! Server-owned eligible-time sampling. The spell lane serializes equipment moves,
//! checkpoints and release; no client command or capability advances these clocks.
use super::ComposedFreshAdmission;
use crate::durability::character_progression::CurrentCharacterGameplayFence;
use crate::durability::imbuement::{EquippedImbuement, ImbuementCheckpoint, ImbuementElapsed};
use crate::foundation::{ExactActorRef, GameSessionId};
use sha2::{Digest, Sha256};

pub(super) struct ImbuementClock {
    at_ms: u64,
    observed: Vec<EquippedImbuement>,
    pub(super) pending: Option<ImbuementCheckpoint>,
    serial: u64,
}

fn uuid(bytes: &[u8; 16]) -> String {
    let h: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    format!(
        "{}-{}-{}-{}-{}",
        &h[..8],
        &h[8..12],
        &h[12..16],
        &h[16..20],
        &h[20..]
    )
}

impl ComposedFreshAdmission<'_, '_, '_> {
    /// Called under the spell lane, before and after equipment moves and on release.
    pub(super) async fn checkpoint_imbuement_equipment(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        fence: CurrentCharacterGameplayFence,
    ) -> bool {
        let now = u64::try_from(self.clock_origin.elapsed().as_millis()).unwrap_or(u64::MAX);
        let Some(combat) = self.observe_actor_combat_state(actor, session).await else {
            return false;
        };
        let Ok(observed) = self
            .root
            .read_worn_imbuements(self.character, self.holder, fence)
            .await
        else {
            return false;
        };
        let mut clocks = self.states.timed_items.imbuements.lock().await;
        let clock = clocks.entry(session).or_insert_with(|| ImbuementClock {
            at_ms: now,
            observed: observed.clone(),
            pending: None,
            serial: 0,
        });
        if clock.pending.is_none() {
            let elapsed = now.saturating_sub(clock.at_ms).min(72_000_000);
            let changes = observed
                .iter()
                .filter_map(|b| {
                    let previous = clock
                        .observed
                        .iter()
                        .find(|old| old.item == b.item && old.occurrence == b.occurrence)?;
                    // New or replaced lifetimes start now. A committed external checkpoint
                    // is authoritative; never overwrite it with an older in-memory budget.
                    if previous.remaining_ms != b.remaining_ms
                        || (b.remaining_ms > 0
                            && (elapsed == 0
                                || !oteryn_gameplay_items::imbuement::consumes_elapsed(
                                    b.category,
                                    combat.in_fight,
                                )))
                    {
                        return None;
                    }
                    Some(ImbuementElapsed {
                        item: uuid(&b.item),
                        occurrence: uuid(&b.occurrence),
                        before_ms: b.remaining_ms,
                        after_ms: b.remaining_ms.saturating_sub(u32::try_from(elapsed).ok()?),
                    })
                })
                .collect::<Vec<_>>();
            if !changes.is_empty() {
                let Some(serial) = clock.serial.checked_add(1) else {
                    return false;
                };
                clock.serial = serial;
                let digest: [u8; 32] = Sha256::new()
                    .chain_update(b"oteryn:imbuement-checkpoint:v2")
                    .chain_update(session.as_bytes())
                    .chain_update(self.holder.fact().registration_revision().to_be_bytes())
                    .chain_update(fence.connection_generation.get().to_be_bytes())
                    .chain_update(now.to_be_bytes())
                    .chain_update(serial.to_be_bytes())
                    .finalize()
                    .into();
                let mut transaction_id = [0; 16];
                transaction_id.copy_from_slice(&digest[..16]);
                transaction_id[6] = (transaction_id[6] & 15) | 0x70;
                transaction_id[8] = (transaction_id[8] & 63) | 0x80;
                clock.pending = Some(ImbuementCheckpoint {
                    transaction_id,
                    changes,
                });
            }
        }
        if let Some(request) = clock.pending.clone() {
            if self
                .root
                .checkpoint_imbuements(self.character, self.holder, fence, request.clone())
                .await
                .is_err()
            {
                return false;
            }
            // Replay is resolved before accepting a new baseline. Failure time is paused;
            // a successful but unacknowledged write cannot consume a second interval.
            clock.observed = observed;
            for b in &mut clock.observed {
                if let Some(change) = request
                    .changes
                    .iter()
                    .find(|d| d.occurrence == uuid(&b.occurrence))
                {
                    b.remaining_ms = change.after_ms;
                }
            }
            clock.pending = None;
        } else {
            clock.observed = observed;
        }
        clock.at_ms = now;
        true
    }
}
