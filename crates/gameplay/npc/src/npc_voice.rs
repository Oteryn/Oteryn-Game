//! NPC-BEHAVIOUR0 §6: source-bound voice preparation in the current Channel owner.
//!
//! This consumes the existing typed catalogue; it does not admit new content or publish chat.
//! NPC-CONTENT-2 supplies the exact typed presence fields on NPC definitions. The CP integration
//! must run this only on the budgeted NPC think, select CHAT_V1 listeners in the same Channel and enqueue
//! through the chat owner. The native chat value carries the NPC's original text for both
//! say and yell, exempt from the player's yell transformation and ingress spam control.
//! There is no native source-catalogue-to-active-runtime-pin binding port here: matching the
//! catalogue and model does not establish that their voices belong to the running bundle.

use crate::content::npc_catalogue::{VOICE_CHANCE, VOICE_INTERVAL, VOICE_LINES};
use crate::content::{
    NpcDataCatalogue, NpcServiceModel, ProjectV2CandidateField, ProjectV2CandidateValue,
    ProjectV2Declaration, ProjectV2Voice,
};
use crate::content::{ProjectV2SpeechMode, ProjectV2Voices};
use crate::foundation::speech::{ChatPosition, SpeechMode};
use crate::foundation::{MovementLocalPosition, ThinkOccurrence};
use oteryn_protocol_oteryn::chat::{ChatLine, ChatSpeaker, ChatSpeechMode};
use oteryn_protocol_oteryn::world_spatial::ActorPosition;
use oteryn_simulation_determinism::{
    DecisionOccurrenceId, GameplayDecisionRoot, deterministic_decision_u64,
};
use std::collections::BTreeMap;
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

/// Exact JSON line shape emitted into NPC-CONTENT-2's typed Text field. Missing or
/// unknown fields do not default to an admitted line or a speech mode.
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct PresenceVoice {
    text: String,
    yell: bool,
}

fn presence_field<'a>(
    fields: &'a [ProjectV2CandidateField],
    path: &str,
) -> Option<&'a ProjectV2CandidateValue> {
    let mut matches = fields.iter().filter(|field| field.field_path == path);
    let value = &matches.next()?.value;
    matches.next().is_none().then_some(value)
}

pub fn profile_from_presence(
    name: &str,
    fields: &[ProjectV2CandidateField],
    qualify_text: impl Fn(&str) -> Option<String>,
) -> Option<VoiceProfile> {
    let ProjectV2CandidateValue::Integer(interval_ms) = presence_field(fields, VOICE_INTERVAL)?
    else {
        return None;
    };
    let ProjectV2CandidateValue::Integer(percent) = presence_field(fields, VOICE_CHANCE)? else {
        return None;
    };
    let ProjectV2CandidateValue::Text(lines) = presence_field(fields, VOICE_LINES)? else {
        return None;
    };
    let interval_ms = u64::try_from(*interval_ms).ok()?;
    let percent = u32::try_from(*percent).ok().filter(|value| *value <= 100)?;
    // The native catalogue has already bounded the containing source Text. Decode this
    // producer's JSON once at book construction, preserving its order and exact text.
    let lines: Vec<PresenceVoice> = serde_json::from_str(lines).ok()?;
    let source = ProjectV2Voices {
        interval_ms,
        chance_ppm: percent * 10_000,
        entries: lines
            .into_iter()
            .map(|line| ProjectV2Voice {
                text: line.text,
                mode: if line.yell {
                    ProjectV2SpeechMode::Yell
                } else {
                    ProjectV2SpeechMode::Say
                },
            })
            .collect(),
    };
    VoiceProfile::from_source(name, &source, qualify_text)
}

/// Candidate source profiles only; this is not a replacement for NPC-CONTENT-2 admission.
pub fn profiles_from_catalogue(
    catalogue: &NpcDataCatalogue,
    model: &NpcServiceModel,
    qualify_text: impl Fn(&str) -> Option<String>,
) -> Result<BTreeMap<String, VoiceProfile>, VoiceError> {
    if catalogue.source_tree_digest() != model.source_tree_digest()
        || catalogue.project_revision() != model.project_revision()
    {
        return Err(VoiceError::ContentSourceMismatch);
    }
    let mut profiles = BTreeMap::new();
    // The model excludes held NPCs. NPC-CONTENT-2's source-held or unsourced voices omit
    // these fields. Never fall back to a linked Dialogue: that would bypass the hold.
    for (reference, npc) in model.npcs() {
        let Some(ProjectV2Declaration::Npc { fields, .. }) = catalogue.declaration(reference)
        else {
            continue;
        };
        if let Some(profile) = profile_from_presence(&npc.name, fields, &qualify_text) {
            profiles.insert(reference.key.clone(), profile);
        }
    }
    Ok(profiles)
}

#[cfg(test)]
#[allow(clippy::expect_used, reason = "native NPC voice fixtures")]
mod tests {
    use super::*;

    fn qualify_text(text: &str) -> Option<String> {
        Some(text.to_owned())
    }

    fn presence_fields(interval: i64, percent: i64, lines: &str) -> Vec<ProjectV2CandidateField> {
        vec![
            ProjectV2CandidateField {
                field_path: VOICE_CHANCE.into(),
                value: ProjectV2CandidateValue::Integer(percent),
            },
            ProjectV2CandidateField {
                field_path: VOICE_INTERVAL.into(),
                value: ProjectV2CandidateValue::Integer(interval),
            },
            ProjectV2CandidateField {
                field_path: VOICE_LINES.into(),
                value: ProjectV2CandidateValue::Text(lines.into()),
            },
        ]
    }

    #[test]
    fn native_presence_projection_preserves_percent_cadence_line_order_and_yell() {
        // NPC-CONTENT-2 emits Integer percent, Integer milliseconds and a Text JSON array
        // of {text, yell}, with authored order and case retained.
        let fields = presence_fields(
            5_000,
            17,
            r#"[{"text":"Fresh fish!","yell":false},{"text":"Żółw mówi \"Hej\"!","yell":true}]"#,
        );
        let profile =
            profile_from_presence("Alice", &fields, qualify_text).expect("native presence");
        assert_eq!(profile.name, "Alice");
        assert_eq!(profile.interval_ms, 5_000);
        assert_eq!(profile.chance_ppm, 170_000);
        assert_eq!(profile.lines.len(), 2);
        assert_eq!(profile.lines[0].text.as_str(), "Fresh fish!");
        assert_eq!(profile.lines[0].mode, ProjectV2SpeechMode::Say);
        assert_eq!(profile.lines[1].text.as_str(), "Żółw mówi \"Hej\"!");
        assert_eq!(profile.lines[1].mode, ProjectV2SpeechMode::Yell);
    }

    #[test]
    fn presence_fields_fail_closed_on_missing_duplicate_mistyped_or_invalid_payloads() {
        let fields = presence_fields(1_000, 100, r#"[{"text":"Fresh fish!","yell":false}]"#);
        for missing in 0..fields.len() {
            let mut changed = fields.clone();
            changed.remove(missing);
            assert!(profile_from_presence("Alice", &changed, qualify_text).is_none());
            let mut duplicated = fields.clone();
            duplicated.push(fields[missing].clone());
            assert!(profile_from_presence("Alice", &duplicated, qualify_text).is_none());
        }
        for (index, value) in [
            (0, ProjectV2CandidateValue::Integer(-1)),
            (0, ProjectV2CandidateValue::Integer(101)),
            (0, ProjectV2CandidateValue::Text("100".into())),
            (1, ProjectV2CandidateValue::Integer(-1)),
            (1, ProjectV2CandidateValue::Integer(0)),
            (1, ProjectV2CandidateValue::Text("1000".into())),
            (2, ProjectV2CandidateValue::Boolean(false)),
        ] {
            let mut changed = fields.clone();
            changed[index].value = value;
            assert!(profile_from_presence("Alice", &changed, qualify_text).is_none());
        }
        for payload in [
            "",
            "[]",
            "{}",
            "[null]",
            r#"[{"text":"Fresh fish!"}]"#,
            r#"[{"text":"Fresh fish!","yell":"false"}]"#,
            r#"[{"text":"Fresh fish!","yell":false,"unknown":0}]"#,
            r#"[{"text":"Fresh fish!","text":"Duplicate","yell":false}]"#,
            r#"[{"text":"|PLAYERNAME|","yell":false}]"#,
            r#"[{"text":"{trade}","yell":true}]"#,
        ] {
            assert!(
                profile_from_presence("Alice", &presence_fields(1_000, 100, payload), qualify_text)
                    .is_none()
            );
        }
        let mixed = presence_fields(
            1_000,
            100,
            r#"[{"text":"|PLAYERNAME|","yell":false},{"text":"Safe line","yell":true}]"#,
        );
        let profile =
            profile_from_presence("Alice", &mixed, qualify_text).expect("one admitted line");
        assert_eq!(profile.lines.len(), 1);
        assert_eq!(profile.lines[0].text.as_str(), "Safe line");
        assert_eq!(profile.lines[0].mode, ProjectV2SpeechMode::Yell);
    }
}
