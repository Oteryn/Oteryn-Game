//! PARTY-PVP-0 §8.1 combat lock (logout block, PZ block, kill block) and §8.2 aggression
//! relations. Readmission composes the existing fenced PVP-1 read with this monotone lock;
//! actor installation, fresh-harm write-ahead and full PvP activation remain separate owners.

use crate::durability::DurabilityRoot;
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::durability::character_progression::{
    CharacterProgressionError, CurrentCharacterGameplayFence,
};
use crate::durability::runtime_scope_assignment::NodeIncarnationProof;
use crate::foundation::owner_timer::{OwnerClock, SemanticTimeMicros as OwnerTimeMicros};
use oteryn_simulation_determinism::SemanticTimeMicros;

/// A refused readmission changes no runtime deadline. Stored state is data, not a controller grant.
#[derive(Debug)]
pub(crate) enum PvpReadmissionError {
    Read(CharacterProgressionError),
    DeadlineOverflow,
}

fn readmission_deadline(
    now: OwnerTimeMicros,
    remaining_ms: Option<u64>,
) -> Result<Option<SemanticTimeMicros>, PvpReadmissionError> {
    remaining_ms
        .map(|ms| {
            ms.checked_mul(1_000)
                .and_then(|micros| now.get().checked_add(micros))
                .map(SemanticTimeMicros::from_micros)
                .ok_or(PvpReadmissionError::DeadlineOverflow)
        })
        .transpose()
}

/// `PARTYPVP0-RL-12`: PZ block and PvP logout block, refreshed by each aggression or hit.
const BLOCK_MS: u64 = 60_000;
/// `PARTYPVP0-RL-13`: kill block.
pub(crate) const KILL_BLOCK_MS: u64 = 15 * 60_000;
/// `PARTYPVP0-RL-24`: aggression relations per actor.
pub(crate) const MAX_RELATIONS: usize = 64;

fn after(now: SemanticTimeMicros, ms: u64) -> SemanticTimeMicros {
    SemanticTimeMicros::from_micros(now.get().saturating_add(ms.saturating_mul(1_000)))
}

fn raise(slot: &mut Option<SemanticTimeMicros>, to: SemanticTimeMicros) {
    if slot.is_none_or(|current| current < to) {
        *slot = Some(to);
    }
}

fn running(slot: Option<SemanticTimeMicros>, now: SemanticTimeMicros) -> bool {
    slot.is_some_and(|until| until > now)
}

/// The PvP part of one actor's combat lock. A deadline is only ever raised, never lowered.
/// ATTACK-0's PvE in-fight deadline stays in `AttackState` and is passed to [`Self::blocks_logout`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct CombatLock {
    logout_until: Option<SemanticTimeMicros>,
    pz_until: Option<SemanticTimeMicros>,
    kill_until: Option<SemanticTimeMicros>,
}

impl CombatLock {
    /// Restore only logout/PZ/kill deadlines through the existing native admission read.
    ///
    /// The Root independently checks recovery, current session, node, World/Channel and lease.
    /// The clock is DATA from this same owner's monotonic axis, never authority. Sampling after
    /// the successful database read conservatively retains its remaining duration across I/O.
    /// Both semantic-time types have microsecond axes; conversion is explicit and checked.
    /// All three deadlines are calculated before the single monotone mutation, so any read or
    /// overflow refusal leaves the previous lock intact. `false` means no stored row exists;
    /// `true` means a row was read, including a row whose three deadlines have all elapsed.
    /// This does not restore skulls, grant a controller or activate any fresh PvP action.
    pub(crate) async fn restore_pvp_readmission(
        &mut self,
        root: &DurabilityRoot,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        clock: &impl OwnerClock,
    ) -> Result<bool, PvpReadmissionError> {
        let Some(stored) = root
            .read_pvp_state(authority, node, fence)
            .await
            .map_err(PvpReadmissionError::Read)?
        else {
            return Ok(false);
        };
        let now = clock.now();
        let logout = readmission_deadline(now, stored.logout_block_remaining_ms)?;
        let pz = readmission_deadline(now, stored.pz_block_remaining_ms)?;
        let kill = readmission_deadline(now, stored.kill_block_remaining_ms)?;
        self.restore(logout, pz, kill);
        Ok(true)
    }

    /// A PvP hit dealt to or taken from a character: the 60 s logout block (both sides).
    pub(crate) fn on_pvp_hit(&mut self, now: SemanticTimeMicros) {
        raise(&mut self.logout_until, after(now, BLOCK_MS));
    }

    /// Aggression that is not retaliation (or a field rune cast): the 60 s PZ block, which also
    /// blocks logout.
    pub(crate) fn on_aggression(&mut self, now: SemanticTimeMicros) {
        raise(&mut self.pz_until, after(now, BLOCK_MS));
    }

    /// A kill as a direct or assisting contributor: `until` is the database clock + 15 min
    /// (`KILL_BLOCK_MS`), committed in the victim's death transaction.
    pub(crate) fn on_kill(&mut self, until: SemanticTimeMicros) {
        raise(&mut self.kill_until, until);
    }

    /// Restore stored deadlines at admission; never shortens a running one.
    pub(crate) fn restore(
        &mut self,
        logout: Option<SemanticTimeMicros>,
        pz: Option<SemanticTimeMicros>,
        kill: Option<SemanticTimeMicros>,
    ) {
        for (slot, stored) in [
            (&mut self.logout_until, logout),
            (&mut self.pz_until, pz),
            (&mut self.kill_until, kill),
        ] {
            if let Some(stored) = stored {
                raise(slot, stored);
            }
        }
    }

    pub(crate) fn pz_block_running(&self, now: SemanticTimeMicros) -> bool {
        running(self.pz_until, now)
    }

    pub(crate) fn kill_block_running(&self, now: SemanticTimeMicros) -> bool {
        running(self.kill_until, now)
    }

    /// Logout: any PvP block, the PZ block, the kill block or the PvE in-fight deadline.
    pub(crate) fn blocks_logout(
        &self,
        now: SemanticTimeMicros,
        pve_in_fight_until: Option<SemanticTimeMicros>,
    ) -> bool {
        running(self.logout_until, now)
            || self.pz_block_running(now)
            || self.kill_block_running(now)
            || running(pve_in_fight_until, now)
    }

    /// Stepping onto a protection-zone tile: the PZ block or the kill block bars it.
    pub(crate) fn blocks_pz_entry(&self, now: SemanticTimeMicros) -> bool {
        self.pz_block_running(now) || self.kill_block_running(now)
    }

    /// A landing (step, teleport, fall) on `destination_is_protection_zone` is refused while a
    /// PZ-entry block runs. Hook for `map::floor` landing resolution.
    pub(crate) fn landing_allowed(
        &self,
        now: SemanticTimeMicros,
        destination_is_protection_zone: bool,
    ) -> bool {
        !(destination_is_protection_zone && self.blocks_pz_entry(now))
    }

    /// A voluntary channel switch and leaving a party: blocked by the whole combat lock.
    pub(crate) fn blocks_voluntary_switch(
        &self,
        now: SemanticTimeMicros,
        pve_in_fight_until: Option<SemanticTimeMicros>,
    ) -> bool {
        self.blocks_logout(now, pve_in_fight_until)
    }
}

/// What recording an aggression did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AggressionOutcome {
    /// New aggression: the actor was first; the PZ block applies.
    Started,
    /// The target was already an aggressor toward the actor (retaliation) or the relation is
    /// refreshed: no new PZ block.
    Retaliation,
    Refreshed,
    /// The set is full of unexpired relations.
    Refused,
}

#[derive(Debug, Clone, Copy)]
struct Relation<T> {
    target: T,
    until: SemanticTimeMicros,
    first: bool,
}

/// The characters one actor has aggression toward (§8.2), at most [`MAX_RELATIONS`].
#[derive(Debug, Clone)]
pub(crate) struct AggressionRelations<T> {
    relations: Vec<Relation<T>>,
}

impl<T> Default for AggressionRelations<T> {
    fn default() -> Self {
        Self {
            relations: Vec::new(),
        }
    }
}

impl<T: Copy + Eq> AggressionRelations<T> {
    pub(crate) fn has_aggression_toward(&self, target: T, now: SemanticTimeMicros) -> bool {
        self.relations
            .iter()
            .any(|r| r.target == target && r.until > now)
    }

    /// Whether the actor was first against `target` while that relation runs (yellow skull).
    pub(crate) fn was_first_against(&self, target: T, now: SemanticTimeMicros) -> bool {
        self.relations
            .iter()
            .any(|r| r.target == target && r.until > now && r.first)
    }

    /// Record aggression toward `target`; `target_is_aggressor` is whether the target already
    /// has aggression toward this actor. The oldest expired relation goes first.
    pub(crate) fn record(
        &mut self,
        target: T,
        target_is_aggressor: bool,
        now: SemanticTimeMicros,
    ) -> AggressionOutcome {
        let until = after(now, BLOCK_MS);
        if let Some(existing) = self.relations.iter_mut().find(|r| r.target == target) {
            let was_running = existing.until > now;
            existing.until = existing.until.max(until);
            // A running relation keeps its first-aggressor bit (yellow skull) while it runs;
            // only a lapsed retaliation (no counter-relation any more) becomes new aggression
            // (§8.1 PZ block). An expired relation restarts from the current counter-relation.
            return if !was_running {
                existing.first = !target_is_aggressor;
                if existing.first {
                    AggressionOutcome::Started
                } else {
                    AggressionOutcome::Retaliation
                }
            } else if existing.first {
                if target_is_aggressor {
                    AggressionOutcome::Retaliation
                } else {
                    AggressionOutcome::Refreshed
                }
            } else if target_is_aggressor {
                AggressionOutcome::Retaliation
            } else {
                existing.first = true;
                AggressionOutcome::Started
            };
        }
        if self.relations.len() >= MAX_RELATIONS {
            let oldest_expired = self
                .relations
                .iter()
                .enumerate()
                .filter(|(_, r)| r.until <= now)
                .min_by_key(|(_, r)| r.until)
                .map(|(i, _)| i);
            match oldest_expired {
                Some(i) => {
                    self.relations.swap_remove(i);
                }
                None => return AggressionOutcome::Refused,
            }
        }
        let first = !target_is_aggressor;
        self.relations.push(Relation {
            target,
            until,
            first,
        });
        if first {
            AggressionOutcome::Started
        } else {
            AggressionOutcome::Retaliation
        }
    }
}

#[cfg(test)]
mod readmission_conversion_tests {
    use super::{OwnerTimeMicros, PvpReadmissionError, SemanticTimeMicros, readmission_deadline};

    #[test]
    fn readmission_explicit_owner_to_simulation_axis_and_elapsed_data() {
        let now = OwnerTimeMicros::from_micros(7_000);
        assert_eq!(
            readmission_deadline(now, Some(2)).expect("fits"),
            Some(SemanticTimeMicros::from_micros(9_000))
        );
        assert_eq!(readmission_deadline(now, None).expect("unset"), None);
        assert_eq!(
            readmission_deadline(now, Some(0)).expect("elapsed"),
            Some(SemanticTimeMicros::from_micros(7_000))
        );
    }

    #[test]
    fn readmission_millisecond_multiplication_overflow_is_refused() {
        assert!(matches!(
            readmission_deadline(OwnerTimeMicros::from_micros(0), Some(u64::MAX / 1_000 + 1)),
            Err(PvpReadmissionError::DeadlineOverflow)
        ));
    }

    #[test]
    fn readmission_owner_axis_addition_overflow_is_refused() {
        assert!(matches!(
            readmission_deadline(OwnerTimeMicros::from_micros(u64::MAX - 999), Some(1)),
            Err(PvpReadmissionError::DeadlineOverflow)
        ));
    }
}
