//! Spam bucket, mute and yell rules of one character (CHAT-0 §3 and §6).
//!
//! The state is in memory only; the durable mute row that carries it across relogs and Channel
//! transfers is CHAT-2C. Spell casts do not take from the bucket, but a muted character cannot cast
//! ([`ChatLimiter::is_muted`] is read by the cast path). Room and private lines (CHAT-2B) share the
//! bucket and the mute with local speech.

use std::collections::VecDeque;

use oteryn_protocol_oteryn::chat::ChatRoom;
use oteryn_simulation_determinism::SemanticTimeMicros;

use super::SpeechMode;

const MICROS_PER_SECOND: u64 = 1_000_000;
/// `CHAT0-RL-02`: a bucket of 4 lines.
pub const BUCKET_LINES: u8 = 4;
/// `CHAT0-RL-02`: one line comes back every 2.5 s.
const REFILL_MICROS: u64 = 2_500_000;
/// How long an under-level yell is refused by [`ChatLimiter::pre_gate`] without the Channel (each
/// refusal still takes its line): the refill interval, so repeated under-level yells cost the
/// Channel at most what the bucket lets a say cost in steady state. A level gained meanwhile is
/// seen at the next full-path yell.
const LEVEL_RECHECK_MICROS: u64 = REFILL_MICROS;
/// `CHAT0-RL-03`: 30 s between two yells.
const YELL_COOLDOWN_MICROS: u64 = 30 * MICROS_PER_SECOND;
/// `CHAT0-RL-04`: offences of the last 30 minutes count, at most the last 16.
const OFFENCE_WINDOW_MICROS: u64 = 30 * 60 * MICROS_PER_SECOND;
const OFFENCES_KEPT: usize = 16;
/// A mute lasts 5 s × n², n being the offences in the window (the Tibia manual).
const MUTE_UNIT_MICROS: u64 = 5 * MICROS_PER_SECOND;
/// Yell and the World, English and Advertising rooms need level 20, or `premium_current` once
/// Premium is delivered (CHAT-0 §6).
const YELL_FREE_LEVEL: u32 = 20;
/// Private messages need the sender at level 3 (CHAT-0 §6).
const PRIVATE_LEVEL: u32 = 3;
/// `CHAT0-RL-05`: more than 20 distinct private recipients in 10 minutes is an offence.
const RECIPIENTS_MAX: usize = 20;
const RECIPIENT_WINDOW_MICROS: u64 = 10 * 60 * MICROS_PER_SECOND;
/// `CHAT0-RL-10`: one Advertising line per 2 minutes.
const ADVERTISING_INTERVAL_MICROS: u64 = 2 * 60 * MICROS_PER_SECOND;

/// Why a line that passed the text checks is not sent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatRefusal {
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
pub struct YellGate {
    pub level: u32,
    pub premium_current: bool,
}

impl YellGate {
    fn allows(self) -> bool {
        self.level > 1 && (self.level >= YELL_FREE_LEVEL || self.premium_current)
    }

    /// Whether the speaker may write to `room`: Help is open to all, the other rooms need level 20
    /// or `premium_current`. The level-1 ban is the yell's own and does not apply here.
    pub fn allows_room(self, room: ChatRoom) -> bool {
        room == ChatRoom::Help || self.level >= YELL_FREE_LEVEL || self.premium_current
    }

    /// Whether the speaker may send a private message: level 3.
    pub fn allows_private(self) -> bool {
        self.level >= PRIVATE_LEVEL
    }
}

/// One character's spam bucket, offences, mute and yell cooldown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatLimiter {
    lines: u8,
    /// When the next line comes back; meaningful only while the bucket is not full.
    refill_anchor: SemanticTimeMicros,
    offences: VecDeque<SemanticTimeMicros>,
    muted_until: Option<SemanticTimeMicros>,
    yell_ready_at: Option<SemanticTimeMicros>,
    /// Until when a yell is refused `LEVEL_TOO_LOW` at the pre-gate (taking its line): set when the
    /// full path refused one for its level, cleared when the full path finds the level.
    yell_level_recheck_at: Option<SemanticTimeMicros>,
    /// The distinct private recipients of the last 10 minutes with the time each was last
    /// written to, oldest first. In memory only; never logged.
    recipients: VecDeque<(SemanticTimeMicros, [u8; 16])>,
    /// When the next Advertising line may be sent.
    advertising_ready_at: Option<SemanticTimeMicros>,
    /// The latest time seen; an earlier `now` is read as this one, so no deadline moves back.
    last_seen: SemanticTimeMicros,
}

impl ChatLimiter {
    /// A full bucket, no offences, not muted.
    pub fn new(now: SemanticTimeMicros) -> Self {
        Self {
            lines: BUCKET_LINES,
            refill_anchor: now,
            offences: VecDeque::new(),
            muted_until: None,
            yell_ready_at: None,
            yell_level_recheck_at: None,
            recipients: VecDeque::new(),
            advertising_ready_at: None,
            last_seen: now,
        }
    }

    /// Whether the character is muted at `now`: it can neither chat nor cast a spell.
    pub fn is_muted(&self, now: SemanticTimeMicros) -> bool {
        self.muted_until.is_some_and(|until| until > now)
    }

    /// The cheap gate in front of the Channel lock (CHAT-0 §6), in the order: mute, yell cooldown,
    /// empty bucket, yell level recheck. It refuses a muted character, and a yell inside its
    /// cooldown before the level is known (it needs no Channel state; a speaker who lost the level
    /// meanwhile reads `Exhausted` instead of `LevelTooLow`); neither takes a line. A say, whisper
    /// or yell beyond an empty bucket starts the offence here, before the level is known. A yell
    /// within [`LEVEL_RECHECK_MICROS`] of a full-path `LevelTooLow` takes its line and is refused
    /// `LevelTooLow` here, so repeated under-level yells drain the bucket and reach the offence
    /// without taking the Channel locks. Any other line takes no line here: it goes through
    /// [`Self::admit_local`], which takes it.
    pub fn pre_gate(
        &mut self,
        mode: SpeechMode,
        now: SemanticTimeMicros,
    ) -> Result<(), ChatRefusal> {
        let now = now.max(self.last_seen);
        self.last_seen = now;
        if let Some(until) = self.muted_until.filter(|until| *until > now) {
            return Err(ChatRefusal::Muted {
                seconds: seconds_until(now, until),
            });
        }
        if let Some(ready) = self
            .yell_ready_at
            .filter(|ready| mode == SpeechMode::Yell && *ready > now)
        {
            return Err(ChatRefusal::Exhausted {
                seconds: seconds_until(now, ready),
            });
        }
        self.refill(now);
        if self.lines == 0 {
            let until = self.offend(now);
            return Err(ChatRefusal::Muted {
                seconds: seconds_until(now, until),
            });
        }
        if mode == SpeechMode::Yell
            && self
                .yell_level_recheck_at
                .is_some_and(|recheck| recheck > now)
        {
            self.lines -= 1;
            return Err(ChatRefusal::LevelTooLow);
        }
        Ok(())
    }

    /// Admits one local line at `now`, in the order: mute, yell cooldown, bucket, yell gate. Every
    /// line that passes the mute and the cooldown takes one line from the bucket, an under-level
    /// yell included; a line beyond the bucket is dropped and starts a mute. An admitted yell
    /// starts the cooldown.
    pub fn admit_local(
        &mut self,
        mode: SpeechMode,
        gate: YellGate,
        now: SemanticTimeMicros,
    ) -> Result<(), ChatRefusal> {
        let now = now.max(self.last_seen);
        self.last_seen = now;
        if let Some(until) = self.muted_until.filter(|until| *until > now) {
            return Err(ChatRefusal::Muted {
                seconds: seconds_until(now, until),
            });
        }
        if let Some(ready) = self
            .yell_ready_at
            .filter(|ready| mode == SpeechMode::Yell && *ready > now)
        {
            return Err(ChatRefusal::Exhausted {
                seconds: seconds_until(now, ready),
            });
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
            if !gate.allows() {
                self.yell_level_recheck_at = Some(after(now, LEVEL_RECHECK_MICROS));
                return Err(ChatRefusal::LevelTooLow);
            }
            self.yell_level_recheck_at = None;
            self.yell_ready_at = Some(after(now, YELL_COOLDOWN_MICROS));
        }
        Ok(())
    }

    /// The gate of a room or private line in front of the Channel lock and the relay (CHAT-0 §6),
    /// in the order: mute, the Advertising interval (`advertising`), empty bucket. A muted line and
    /// an Advertising line inside its interval take no line; a line beyond an empty bucket starts
    /// the offence. Any other line takes its line here, before its level, its recipient or the
    /// relay is consulted, so a refused line (`LEVEL_TOO_LOW`, `NOT_ONLINE`, `NO_VOCATION`) costs a
    /// line like a sent one and a flood of them reaches the mute without the Channel locks.
    pub fn pre_gate_relay(
        &mut self,
        advertising: bool,
        now: SemanticTimeMicros,
    ) -> Result<(), ChatRefusal> {
        let now = now.max(self.last_seen);
        self.last_seen = now;
        if let Some(until) = self.muted_until.filter(|until| *until > now) {
            return Err(ChatRefusal::Muted {
                seconds: seconds_until(now, until),
            });
        }
        if let Some(ready) = self
            .advertising_ready_at
            .filter(|ready| advertising && *ready > now)
        {
            return Err(ChatRefusal::Exhausted {
                seconds: seconds_until(now, ready),
            });
        }
        self.refill(now);
        if self.lines == 0 {
            let until = self.offend(now);
            return Err(ChatRefusal::Muted {
                seconds: seconds_until(now, until),
            });
        }
        self.lines -= 1;
        Ok(())
    }

    /// Claims the Advertising interval for a line about to be sent: `Exhausted` while it runs.
    /// Checked again here, under the same lock as the claim, so two lines racing past
    /// [`Self::pre_gate_relay`] cannot both be sent.
    pub fn claim_advertising(&mut self, now: SemanticTimeMicros) -> Result<(), ChatRefusal> {
        let now = now.max(self.last_seen);
        self.last_seen = now;
        if let Some(ready) = self.advertising_ready_at.filter(|ready| *ready > now) {
            return Err(ChatRefusal::Exhausted {
                seconds: seconds_until(now, ready),
            });
        }
        self.advertising_ready_at = Some(after(now, ADVERTISING_INTERVAL_MICROS));
        Ok(())
    }

    /// Records a private message to `recipient` (`CHAT0-RL-05`). A 21st distinct recipient within
    /// 10 minutes is an offence: the line is dropped and the character is muted as for an empty
    /// bucket. A recipient already written to in the window is always admitted.
    pub fn admit_recipient(
        &mut self,
        recipient: [u8; 16],
        now: SemanticTimeMicros,
    ) -> Result<(), ChatRefusal> {
        let now = now.max(self.last_seen);
        self.last_seen = now;
        self.recipients.retain(|(at, _)| {
            now.elapsed_since(*at)
                .is_ok_and(|age| age < RECIPIENT_WINDOW_MICROS)
        });
        if let Some(index) = self.recipients.iter().position(|(_, id)| *id == recipient) {
            self.recipients.remove(index);
        } else if self.recipients.len() >= RECIPIENTS_MAX {
            let until = self.offend(now);
            return Err(ChatRefusal::Muted {
                seconds: seconds_until(now, until),
            });
        }
        self.recipients.push_back((now, recipient));
        Ok(())
    }

    fn refill(&mut self, now: SemanticTimeMicros) {
        if self.lines >= BUCKET_LINES {
            self.lines = BUCKET_LINES;
            self.refill_anchor = now;
            return;
        }
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
