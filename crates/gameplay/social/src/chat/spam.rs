//! Spam rule adapters for the runtime speech mode and protocol room types.

use oteryn_protocol_oteryn::chat::ChatRoom;
use oteryn_simulation_determinism::SemanticTimeMicros;

use super::SpeechMode;
use oteryn_gameplay_social_rules::spam as rules;
pub use rules::{BUCKET_LINES, ChatRefusal};

/// The speaker's facts a yell is gated on. `premium_current` stays `false` until Premium is
/// delivered (PREMIUM-DELIVERY-0).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct YellGate {
    pub level: u32,
    pub premium_current: bool,
}

impl YellGate {
    fn rules(self) -> rules::YellGate {
        rules::YellGate {
            level: self.level,
            premium_current: self.premium_current,
        }
    }

    /// Whether the speaker may write to `room`: Help is open to all, the other rooms need level 20
    /// or `premium_current`. The level-1 ban is the yell's own and does not apply here.
    pub fn allows_room(self, room: ChatRoom) -> bool {
        self.rules().allows_room(room == ChatRoom::Help)
    }

    /// Whether the speaker may send a private message: level 3.
    pub fn allows_private(self) -> bool {
        self.rules().allows_private()
    }
}

/// One character's spam bucket, offences, mute and yell cooldown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatLimiter(rules::ChatLimiter);

impl ChatLimiter {
    /// A full bucket, no offences, not muted.
    pub fn new(now: SemanticTimeMicros) -> Self {
        Self(rules::ChatLimiter::new(now))
    }

    /// Whether the character is muted at `now`: it can neither chat nor cast a spell.
    pub fn is_muted(&self, now: SemanticTimeMicros) -> bool {
        self.0.is_muted(now)
    }

    /// Apply the pure pre-gate before taking the Channel lock.
    pub fn pre_gate(
        &mut self,
        mode: SpeechMode,
        now: SemanticTimeMicros,
    ) -> Result<(), ChatRefusal> {
        self.0.pre_gate(mode == SpeechMode::Yell, now)
    }

    /// Admit local speech with the speaker's level and Premium facts.
    pub fn admit_local(
        &mut self,
        mode: SpeechMode,
        gate: YellGate,
        now: SemanticTimeMicros,
    ) -> Result<(), ChatRefusal> {
        self.0
            .admit_local(mode == SpeechMode::Yell, gate.rules(), now)
    }

    /// Apply the room/private pre-gate before consulting the relay.
    pub fn pre_gate_relay(
        &mut self,
        advertising: bool,
        now: SemanticTimeMicros,
    ) -> Result<(), ChatRefusal> {
        self.0.pre_gate_relay(advertising, now)
    }

    /// Claim the Advertising interval for a line about to be sent.
    pub fn claim_advertising(&mut self, now: SemanticTimeMicros) -> Result<(), ChatRefusal> {
        self.0.claim_advertising(now)
    }

    /// Record a private recipient against the distinct-recipient limit.
    pub fn admit_recipient(
        &mut self,
        recipient: [u8; 16],
        now: SemanticTimeMicros,
    ) -> Result<(), ChatRefusal> {
        self.0.admit_recipient(recipient, now)
    }
}
