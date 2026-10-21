//! NPC-BEHAVIOUR0 §6: source-bound voice preparation in the current Channel owner.
//!
//! This consumes the existing typed catalogue; it does not admit new content or publish chat.
//! NPC-CONTENT-2 supplies the exact typed presence fields on NPC definitions. The CP integration
//! must run this only on the budgeted NPC think, select CHAT_V1 listeners in the same Channel and enqueue
//! through the chat owner. The native chat value carries the NPC's original text for both
//! say and yell, exempt from the player's yell transformation and ingress spam control.
//! There is no native source-catalogue-to-active-runtime-pin binding port here: matching the
//! catalogue and model does not establish that their voices belong to the running bundle.

use crate::content::{ProjectV2SpeechMode, ProjectV2Voices};
use crate::foundation::speech::{ChatPosition, SpeechMode};
use crate::foundation::{MovementLocalPosition, ThinkOccurrence};
use oteryn_protocol_oteryn::chat::{ChatLine, ChatSpeaker, ChatSpeechMode};
use oteryn_protocol_oteryn::world_spatial::ActorPosition;
use oteryn_simulation_determinism::{
    DecisionOccurrenceId, GameplayDecisionRoot, deterministic_decision_u64,
};
use std::num::NonZeroU64;

const PPM: u64 = 1_000_000;
const PURPOSE: &str = "NPC_VOICE";
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoiceError {
    ContentSourceMismatch,
    NotCurrentNpc,
    OccurrenceSuperseded,
    Draw,
}

#[derive(Debug, Clone)]
pub struct VoiceLine {
    pub text: String,
    pub mode: ProjectV2SpeechMode,
}

#[derive(Debug, Clone)]
pub struct VoiceProfile {
    pub name: String,
    pub interval_ms: u64,
    pub chance_ppm: u32,
    pub lines: Vec<VoiceLine>,
}

impl VoiceProfile {
    pub fn from_source(
        name: &str,
        source: &ProjectV2Voices,
        qualify: impl Fn(&str) -> Option<String>,
    ) -> Option<Self> {
        if source.interval_ms == 0 || u64::from(source.chance_ppm) > PPM {
            return None;
        }
        let lines = source
            .entries
            .iter()
            .filter_map(|line| {
                // Source placeholders have no voice substitution owner. Fail closed rather than
                // broadcasting a player's name token or an unresolved dialogue link verbatim.
                if line.text.contains(['|', '{', '}']) {
                    return None;
                }
                Some(VoiceLine {
                    text: qualify(&line.text)?,
                    mode: line.mode,
                })
            })
            .collect::<Vec<_>>();
        (!lines.is_empty()).then(|| Self {
            name: name.to_owned(),
            interval_ms: source.interval_ms,
            chance_ppm: source.chance_ppm,
            lines,
        })
    }
}

/// A prepared line only. The chat owner must independently choose current, capable listeners
/// and enqueue it once. Preparation confers no actor, content or delivery authority.
#[derive(Debug, Clone)]
pub struct NpcVoiceProposal {
    pub occurrence: ThinkOccurrence,
    pub position: ChatPosition,
    pub name: String,
    pub line: VoiceLine,
}

impl NpcVoiceProposal {
    pub fn mode(&self) -> SpeechMode {
        match self.line.mode {
            ProjectV2SpeechMode::Say => SpeechMode::Say,
            ProjectV2SpeechMode::Yell => SpeechMode::Yell,
        }
    }

    /// Prepares the native value without the player-only yell uppercase transformation. The
    /// chat owner's `local_listeners` selection still decides recipients and floor/range rules.
    pub fn chat_line(&self) -> Result<ChatLine, VoiceError> {
        let generation = NonZeroU64::new(self.occurrence.actor.actor_local_generation())
            .ok_or(VoiceError::NotCurrentNpc)?;
        Ok(ChatLine::Local {
            speaker: ChatSpeaker {
                identity: self.occurrence.actor.placement_identity(),
                generation,
            },
            speaker_name: self.name.clone(),
            mode: match self.line.mode {
                ProjectV2SpeechMode::Yell => ChatSpeechMode::Yell,
                ProjectV2SpeechMode::Say => ChatSpeechMode::Say,
            },
            text: self.line.text.as_str().to_owned(),
            position: ActorPosition {
                x: self.position.x,
                y: self.position.y,
                floor: i16::from(self.position.floor),
            },
        })
    }
}

#[derive(Debug, Clone)]
pub enum VoiceThink {
    Idle,
    Quiet,
    Speak(NpcVoiceProposal),
    /// The owner has already prepared this occurrence; never enqueue it a second time.
    Replay,
}

impl VoiceProfile {
    /// Prepare one visible scheduled occurrence; the adapter owns current actor admission.
    pub fn prepare(
        &self,
        occurrence: ThinkOccurrence,
        position: MovementLocalPosition,
        ticks_ms: u64,
        interval_millis: u64,
        root: &GameplayDecisionRoot,
        decision: DecisionOccurrenceId,
    ) -> Result<(VoiceThink, u64), VoiceError> {
        let ticks = ticks_ms.saturating_add(interval_millis);
        let result = if ticks < self.interval_ms
            || draw_below(root, decision, PPM, 0)? >= u64::from(self.chance_ppm)
        {
            VoiceThink::Quiet
        } else {
            let count = u64::try_from(self.lines.len()).map_err(|_| VoiceError::Draw)?;
            let index = usize::try_from(draw_below(root, decision, count, 1)?)
                .map_err(|_| VoiceError::Draw)?;
            VoiceThink::Speak(NpcVoiceProposal {
                occurrence,
                position: ChatPosition {
                    x: position.x,
                    y: position.y,
                    floor: u8::try_from(position.floor).map_err(|_| VoiceError::NotCurrentNpc)?,
                },
                name: self.name.clone(),
                line: self.lines[index].clone(),
            })
        };
        let next_ticks_ms = if ticks >= self.interval_ms { 0 } else { ticks };
        Ok((result, next_ticks_ms))
    }
}

/// Rejection sampling keeps line choice uniform. Chance uses even draw indices; line choice
/// uses odd ones, so a rejected draw cannot reuse the other decision's index. The native
/// occurrence hashes the exact actor (World/Channel/owner/actor generations) and sequence.
fn draw_below(
    root: &GameplayDecisionRoot,
    occurrence: DecisionOccurrenceId,
    upper: u64,
    mut index: u64,
) -> Result<u64, VoiceError> {
    if upper == 0 {
        return Err(VoiceError::Draw);
    }
    let threshold = upper.wrapping_neg() % upper;
    loop {
        let draw = deterministic_decision_u64(root, occurrence, PURPOSE, index)
            .map_err(|_| VoiceError::Draw)?;
        if draw >= threshold {
            return Ok(draw % upper);
        }
        index = index.checked_add(2).ok_or(VoiceError::Draw)?;
    }
}
