//! Spam bucket, mute and yell rules of one character (CHAT-0 §3 and §6).
//!
//! The state is in memory only; the durable mute row that carries it across relogs and Channel
//! transfers is CHAT-2. Spell casts do not take from the bucket, but a muted character cannot cast
//! ([`ChatLimiter::is_muted`] is read by the cast path).

use std::collections::VecDeque;

use oteryn_simulation_determinism::SemanticTimeMicros;

use super::SpeechMode;

const MICROS_PER_SECOND: u64 = 1_000_000;
/// `CHAT0-RL-02`: a bucket of 4 lines.
pub(crate) const BUCKET_LINES: u8 = 4;
/// `CHAT0-RL-02`: one line comes back every 2.5 s.
const REFILL_MICROS: u64 = 2_500_000;
/// `CHAT0-RL-03`: 30 s between two yells.
const YELL_COOLDOWN_MICROS: u64 = 30 * MICROS_PER_SECOND;
/// `CHAT0-RL-04`: offences of the last 30 minutes count, at most the last 16.
const OFFENCE_WINDOW_MICROS: u64 = 30 * 60 * MICROS_PER_SECOND;
const OFFENCES_KEPT: usize = 16;
/// A mute lasts 5 s × n², n being the offences in the window (the Tibia manual).
const MUTE_UNIT_MICROS: u64 = 5 * MICROS_PER_SECOND;
/// Yell needs level 20, or `premium_current` once Premium is delivered (CHAT-0 §6).
const YELL_FREE_LEVEL: u32 = 20;

/// Why a line that passed the text checks is not sent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ChatRefusal {
    /// `MUTED {seconds}`: the character is muted, or this line started a mute.
    Muted { seconds: u32 },
    /// `EXHAUSTED {seconds}`: the yell cooldown runs.
    Exhausted { seconds: u32 },
    /// `LEVEL_TOO_LOW`: yell at level 1, or below level 20 without `premium_current`.
    LevelTooLow,
}

/// The speaker's facts a yell is gated on. `premium_current` stays `false` until Premium is
/// delivered (PREMIUM-DELIVERY-0).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct YellGate {
    pub(crate) level: u32,
    pub(crate) premium_current: bool,
}

impl YellGate {
    fn allows(self) -> bool {
        self.level > 1 && (self.level >= YELL_FREE_LEVEL || self.premium_current)
    }
}

/// One character's spam bucket, offences, mute and yell cooldown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ChatLimiter {
    lines: u8,
    /// When the next line comes back; meaningful only while the bucket is not full.
    refill_anchor: SemanticTimeMicros,
    offences: VecDeque<SemanticTimeMicros>,
    muted_until: Option<SemanticTimeMicros>,
    yell_ready_at: Option<SemanticTimeMicros>,
}

impl ChatLimiter {
    /// A full bucket, no offences, not muted.
    pub(crate) fn new(now: SemanticTimeMicros) -> Self {
        Self {
            lines: BUCKET_LINES,
            refill_anchor: now,
            offences: VecDeque::new(),
            muted_until: None,
            yell_ready_at: None,
        }
    }

    /// Whether the character is muted at `now`: it can neither chat nor cast a spell.
    pub(crate) fn is_muted(&self, now: SemanticTimeMicros) -> bool {
        self.muted_until.is_some_and(|until| until > now)
    }

    /// Admits one local line at `now`, in the order: mute, yell gate, yell cooldown, bucket. An
    /// admitted line takes one line from the bucket and, for a yell, starts the cooldown. A line
    /// beyond the bucket is dropped and starts a mute.
    pub(crate) fn admit_local(
        &mut self,
        mode: SpeechMode,
        gate: YellGate,
        now: SemanticTimeMicros,
    ) -> Result<(), ChatRefusal> {
        if let Some(until) = self.muted_until.filter(|until| *until > now) {
            return Err(ChatRefusal::Muted {
                seconds: seconds_until(now, until),
            });
        }
        if mode == SpeechMode::Yell {
            if !gate.allows() {
                return Err(ChatRefusal::LevelTooLow);
            }
            if let Some(ready) = self.yell_ready_at.filter(|ready| *ready > now) {
                return Err(ChatRefusal::Exhausted {
                    seconds: seconds_until(now, ready),
                });
            }
        }
        self.refill(now);
        if self.lines == 0 {
            let until = self.offend(now);
            return Err(ChatRefusal::Muted {
                seconds: seconds_until(now, until),
            });
        }
        self.lines -= 1;
        if mode == SpeechMode::Yell {
            self.yell_ready_at = Some(after(now, YELL_COOLDOWN_MICROS));
        }
        Ok(())
    }

    fn refill(&mut self, now: SemanticTimeMicros) {
        if self.lines >= BUCKET_LINES {
            self.lines = BUCKET_LINES;
            self.refill_anchor = now;
            return;
        }
        // A clock that goes backwards refills nothing.
        let gained = now.elapsed_since(self.refill_anchor).unwrap_or(0) / REFILL_MICROS;
        if gained == 0 {
            return;
        }
        let lines = u64::from(self.lines).saturating_add(gained);
        if lines >= u64::from(BUCKET_LINES) {
            self.lines = BUCKET_LINES;
            self.refill_anchor = now;
        } else {
            self.lines = lines as u8;
            self.refill_anchor = after(self.refill_anchor, gained * REFILL_MICROS);
        }
    }

    /// Records an offence at `now` and mutes for 5 s × n²; the bucket is full when the mute ends.
    fn offend(&mut self, now: SemanticTimeMicros) -> SemanticTimeMicros {
        self.offences.retain(|at| {
            now.elapsed_since(*at)
                .is_ok_and(|age| age < OFFENCE_WINDOW_MICROS)
        });
        if self.offences.len() == OFFENCES_KEPT {
            self.offences.pop_front();
        }
        self.offences.push_back(now);
        let n = self.offences.len() as u64;
        let until = after(now, MUTE_UNIT_MICROS * n * n);
        self.muted_until = Some(until);
        self.lines = BUCKET_LINES;
        self.refill_anchor = until;
        until
    }
}

fn after(at: SemanticTimeMicros, micros: u64) -> SemanticTimeMicros {
    SemanticTimeMicros::from_micros(at.get().saturating_add(micros))
}

/// Whole seconds left until `until`, rounded up; at least 1.
fn seconds_until(now: SemanticTimeMicros, until: SemanticTimeMicros) -> u32 {
    let micros = until.elapsed_since(now).unwrap_or(0);
    u32::try_from(micros.div_ceil(MICROS_PER_SECOND))
        .unwrap_or(u32::MAX)
        .max(1)
}
