//! NPC-BEHAVIOUR0 §6: source-bound voice preparation in the current Channel owner.
//!
//! This consumes the existing typed catalogue; it does not admit new content or publish chat.
//! NPC-CONTENT-2 must qualify the source lines before activation. The CP integration must run
//! this only on the budgeted NPC think, select CHAT_V1 listeners in the same Channel and enqueue
//! through the chat owner. The native chat value carries the NPC's original text for both
//! say and yell, exempt from the player's yell transformation and ingress spam control.
//! There is no native source-catalogue-to-active-runtime-pin binding port here: matching the
//! catalogue and model does not establish that their voices belong to the running bundle.

#![allow(
    dead_code,
    reason = "NPC voice consumer awaits content admission and CP owner wiring"
)]

use std::collections::BTreeMap;
use std::num::NonZeroU64;

use crate::ai_think::profile_schedule::decision_occurrence;
use crate::ai_think::targeting::sees;
use crate::ai_think::{CREATURE_THINK_INTERVAL_MILLIS, ThinkOccurrence};
use crate::chat::{ChatPosition, ChatText, SpeechMode};
use crate::content::{
    NpcDataCatalogue, NpcServiceModel, ProjectV2Declaration, ProjectV2SpeechMode, ProjectV2Voices,
};
use crate::foundation::{ChannelRuntimeV1, ExactActorRef, MovementPositionContext};
use oteryn_protocol_oteryn::chat::{ChatLine, ChatSpeaker, ChatSpeechMode};
use oteryn_protocol_oteryn::world_spatial::ActorPosition;
use oteryn_simulation_determinism::{GameplayDecisionRoot, deterministic_decision_u64};

const PPM: u64 = 1_000_000;
const PURPOSE: &str = "NPC_VOICE";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum VoiceError {
    ContentSourceMismatch,
    NotCurrentNpc,
    OccurrenceSuperseded,
    Draw,
}

#[derive(Debug, Clone)]
struct VoiceLine {
    text: ChatText,
    mode: ProjectV2SpeechMode,
}

#[derive(Debug, Clone)]
struct VoiceProfile {
    name: String,
    interval_ms: u64,
    chance_ppm: u32,
    lines: Vec<VoiceLine>,
}

impl VoiceProfile {
    fn from_source(name: &str, source: &ProjectV2Voices) -> Option<Self> {
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
                    text: ChatText::parse(&line.text).ok()?,
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

/// Candidate source profiles only; this is not a replacement for NPC-CONTENT-2 admission.
#[derive(Debug, Default)]
pub(crate) struct NpcVoiceBook {
    profiles: BTreeMap<String, VoiceProfile>,
}

impl NpcVoiceBook {
    pub(crate) fn from_catalogue(
        catalogue: &NpcDataCatalogue,
        model: &NpcServiceModel,
    ) -> Result<Self, VoiceError> {
        if catalogue.source_tree_digest() != model.source_tree_digest()
            || catalogue.project_revision() != model.project_revision()
        {
            return Err(VoiceError::ContentSourceMismatch);
        }
        let mut profiles = BTreeMap::new();
        // The model excludes held NPCs. Exact native references, including revision, select
        // their Dialogue; a similarly named or unreferenced Dialogue cannot supply voices.
        for (reference, npc) in model.npcs() {
            let Some(ProjectV2Declaration::Dialogue {
                voices: Some(source),
                ..
            }) = npc
                .dialogue
                .as_ref()
                .and_then(|dialogue| catalogue.declaration(dialogue))
            else {
                continue;
            };
            if let Some(profile) = VoiceProfile::from_source(&npc.name, source) {
                profiles.insert(reference.key.clone(), profile);
            }
        }
        Ok(Self { profiles })
    }
}

/// A prepared line only. The chat owner must independently choose current, capable listeners
/// and enqueue it once. Preparation confers no actor, content or delivery authority.
#[derive(Debug, Clone)]
pub(crate) struct NpcVoiceProposal {
    pub(crate) occurrence: ThinkOccurrence,
    pub(crate) position: ChatPosition,
    pub(crate) name: String,
    line: VoiceLine,
}

impl NpcVoiceProposal {
    pub(crate) fn mode(&self) -> SpeechMode {
        match self.line.mode {
            ProjectV2SpeechMode::Say => SpeechMode::Say,
            ProjectV2SpeechMode::Yell => SpeechMode::Yell,
        }
    }

    /// Prepares the native value without the player-only yell uppercase transformation. The
    /// chat owner's `local_listeners` selection still decides recipients and floor/range rules.
    pub(crate) fn chat_line(&self) -> Result<ChatLine, VoiceError> {
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
pub(crate) enum VoiceThink {
    Idle,
    Quiet,
    Speak(NpcVoiceProposal),
    /// The owner has already prepared this occurrence; never enqueue it a second time.
    Replay,
}

/// One ephemeral state per exact actor, recreated from the source book at Channel restart.
#[derive(Debug)]
pub(crate) struct NpcVoiceState {
    actor: ExactActorRef,
    key: String,
    context: MovementPositionContext,
    profile: VoiceProfile,
    ticks_ms: u64,
    last_sequence: Option<u64>,
}

impl NpcVoiceState {
    pub(crate) fn new(
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        book: &NpcVoiceBook,
    ) -> Result<Option<Self>, VoiceError> {
        runtime
            .owner_fence()
            .map_err(|_| VoiceError::NotCurrentNpc)?;
        let key = runtime.npc_key(actor).ok_or(VoiceError::NotCurrentNpc)?;
        Ok(book.profiles.get(key).map(|profile| Self {
            actor,
            key: key.to_owned(),
            context: runtime.pinned_movement_context(),
            profile: profile.clone(),
            ticks_ms: 0,
            last_sequence: None,
        }))
    }

    /// Called once for a scheduled NPC think. No wall-clock catch-up: each active think adds
    /// exactly 1,000 ms; an idle actor's ticks pause. A retry neither advances nor redraws.
    pub(crate) fn think(
        &mut self,
        runtime: &ChannelRuntimeV1,
        occurrence: ThinkOccurrence,
        root: &GameplayDecisionRoot,
    ) -> Result<VoiceThink, VoiceError> {
        runtime
            .owner_fence()
            .map_err(|_| VoiceError::NotCurrentNpc)?;
        if occurrence.actor != self.actor
            || runtime.pinned_movement_context() != self.context
            || runtime.npc_key(self.actor) != Some(self.key.as_str())
        {
            return Err(VoiceError::NotCurrentNpc);
        }
        if let Some(last) = self.last_sequence {
            if occurrence.sequence < last {
                return Err(VoiceError::OccurrenceSuperseded);
            }
            if occurrence.sequence == last {
                return Ok(VoiceThink::Replay);
            }
        }
        let visible = runtime.visible_entities();
        let npc = visible
            .npcs
            .iter()
            .find(|npc| npc.actor == self.actor)
            .ok_or(VoiceError::NotCurrentNpc)?;
        if !visible
            .players
            .iter()
            .any(|player| sees(player.position, npc.position))
        {
            self.last_sequence = Some(occurrence.sequence);
            return Ok(VoiceThink::Idle);
        }
        let ticks = self.ticks_ms.saturating_add(CREATURE_THINK_INTERVAL_MILLIS);
        let result = if ticks < self.profile.interval_ms
            || draw_below(root, occurrence, PPM, 0)? >= u64::from(self.profile.chance_ppm)
        {
            VoiceThink::Quiet
        } else {
            let count = u64::try_from(self.profile.lines.len()).map_err(|_| VoiceError::Draw)?;
            let index = usize::try_from(draw_below(root, occurrence, count, 1)?)
                .map_err(|_| VoiceError::Draw)?;
            VoiceThink::Speak(NpcVoiceProposal {
                occurrence,
                position: ChatPosition {
                    x: npc.position.x,
                    y: npc.position.y,
                    floor: u8::try_from(npc.position.floor)
                        .map_err(|_| VoiceError::NotCurrentNpc)?,
                },
                name: self.profile.name.clone(),
                line: self.profile.lines[index].clone(),
            })
        };
        self.ticks_ms = if ticks >= self.profile.interval_ms {
            0
        } else {
            ticks
        };
        self.last_sequence = Some(occurrence.sequence);
        Ok(result)
    }
}

/// Rejection sampling keeps line choice uniform. Chance uses even draw indices; line choice
/// uses odd ones, so a rejected draw cannot reuse the other decision's index. The native
/// occurrence hashes the exact actor (World/Channel/owner/actor generations) and sequence.
fn draw_below(
    root: &GameplayDecisionRoot,
    occurrence: ThinkOccurrence,
    upper: u64,
    mut index: u64,
) -> Result<u64, VoiceError> {
    if upper == 0 {
        return Err(VoiceError::Draw);
    }
    let threshold = upper.wrapping_neg() % upper;
    loop {
        let draw =
            deterministic_decision_u64(root, decision_occurrence(occurrence), PURPOSE, index)
                .map_err(|_| VoiceError::Draw)?;
        if draw >= threshold {
            return Ok(draw % upper);
        }
        index = index.checked_add(2).ok_or(VoiceError::Draw)?;
    }
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::panic,
    reason = "native NPC voice fixtures"
)]
mod tests {
    use super::*;
    use crate::content::ProjectV2Voice;
    use crate::foundation::{
        MovementFacing, MovementLocalPosition, NpcPlacement, NpcPlacementTable, NpcPlacements,
    };
    use crate::gameplay_transport::actor_spell::tests::runtime_with_capacity;

    fn source(interval_ms: u64, chance_ppm: u32) -> ProjectV2Voices {
        ProjectV2Voices {
            interval_ms,
            chance_ppm,
            entries: vec![
                ProjectV2Voice {
                    text: "Fresh fish!".into(),
                    mode: ProjectV2SpeechMode::Say,
                },
                ProjectV2Voice {
                    text: "Come closer.".into(),
                    mode: ProjectV2SpeechMode::Say,
                },
            ],
        }
    }

    fn fixture(
        source: ProjectV2Voices,
    ) -> (ChannelRuntimeV1, ExactActorRef, ExactActorRef, NpcVoiceBook) {
        let (mut runtime, player, _) = runtime_with_capacity(0x74, 3);
        runtime
            .initialize_pinned_test_position(
                player,
                MovementLocalPosition {
                    x: 10,
                    y: 10,
                    floor: 7,
                },
            )
            .expect("player");
        let outcome = runtime
            .place_npcs(
                &NpcPlacementTable {
                    npcs: vec![NpcPlacements {
                        key: "oteryn:npc.alice".into(),
                        placements: vec![NpcPlacement {
                            floor: -7,
                            x: 11,
                            y: 10,
                            direction: MovementFacing::South,
                        }],
                    }],
                },
                |_| true,
                true,
            )
            .expect("native NPC");
        let book = NpcVoiceBook {
            profiles: BTreeMap::from([(
                "oteryn:npc.alice".into(),
                VoiceProfile::from_source("Alice", &source).expect("source"),
            )]),
        };
        (runtime, player, outcome.actors[0], book)
    }

    fn occurrence(actor: ExactActorRef, sequence: u64) -> ThinkOccurrence {
        ThinkOccurrence { actor, sequence }
    }
    fn root() -> GameplayDecisionRoot {
        GameplayDecisionRoot::from_bytes([7; 32])
    }

    fn catalogue(revision: &str, text: &str) -> NpcDataCatalogue {
        use crate::content::{
            CanonicalProjectDocuments, ProjectDraft, ProjectV2DefinitionRef, ProjectV2Draft,
            ProjectV2Family, ProjectV2Identity, ProjectV2State, npc_catalogue_preproduction_limits,
        };
        let identity = |key: &str| ProjectV2Identity {
            key: key.into(),
            revision: "definition-r1".into(),
        };
        let dialogue = |key: &str, text: &str| ProjectV2Declaration::Dialogue {
            identity: identity(key),
            greet: vec![],
            farewell: vec![],
            walkaway: vec![],
            send_trade: vec![],
            keywords: vec![],
            source_incomplete: vec![],
            fields: vec![],
            voices: Some(ProjectV2Voices {
                interval_ms: 1_000,
                chance_ppm: 1_000_000,
                entries: vec![ProjectV2Voice {
                    text: text.into(),
                    mode: ProjectV2SpeechMode::Say,
                }],
            }),
        };
        let draft = ProjectV2Draft {
            core: ProjectDraft {
                project_revision: revision.into(),
                package_key: "oteryn:content.world-project".into(),
                semantic_schema_version: "reference-schema-v1".into(),
                licensing_metadata: "license:project-owned-v1".into(),
                world_id: "0123456789ab70cd8ef0123456789abc".into(),
                coordinate_frame: "global-target-2026-09-27".into(),
                records: vec![],
                imports: vec![],
                metadata: vec![],
            },
            state: ProjectV2State {
                declarations: vec![
                    ProjectV2Declaration::Npc {
                        identity: identity("oteryn:npc.alice"),
                        presentation: None,
                        behavior: None,
                        dialogue: Some(ProjectV2DefinitionRef {
                            family: ProjectV2Family::Dialogue,
                            key: "oteryn:dialogue.alice".into(),
                            revision: "definition-r1".into(),
                        }),
                        services: vec![],
                        fields: vec![],
                    },
                    ProjectV2Declaration::Npc {
                        identity: identity("oteryn:npc.mute"),
                        presentation: None,
                        behavior: None,
                        dialogue: None,
                        services: vec![],
                        fields: vec![],
                    },
                    dialogue("oteryn:dialogue.alice", text),
                    dialogue("oteryn:dialogue.unreferenced", "Not an NPC's voice"),
                ],
                ..ProjectV2State::default()
            },
        };
        let limits = npc_catalogue_preproduction_limits().project;
        let project = CanonicalProjectDocuments::from_v2_draft(draft, limits)
            .expect("native documents")
            .into_snapshot(limits)
            .expect("snapshot")
            .parse(limits)
            .expect("native project");
        NpcDataCatalogue::from_project(project, limits).expect("native catalogue")
    }

    #[test]
    fn book_consumes_only_referenced_native_dialogue_and_rejects_mixed_source_models() {
        let a = catalogue("voice-r1", "Fresh fish!");
        let model = a.service_model_for_project().expect("native service model");
        let book = NpcVoiceBook::from_catalogue(&a, &model).expect("same source");
        assert_eq!(book.profiles.len(), 1);
        assert_eq!(
            book.profiles.get("oteryn:npc.alice").expect("Alice").lines[0]
                .text
                .as_str(),
            "Fresh fish!"
        );
        assert!(!book.profiles.contains_key("oteryn:npc.mute"));
        assert!(!book.profiles.contains_key("oteryn:dialogue.unreferenced"));
        let different_digest = catalogue("voice-r1", "Different source line");
        assert!(matches!(
            NpcVoiceBook::from_catalogue(&different_digest, &model),
            Err(VoiceError::ContentSourceMismatch)
        ));
        let different_revision = catalogue("voice-r2", "Fresh fish!");
        assert!(matches!(
            NpcVoiceBook::from_catalogue(&different_revision, &model),
            Err(VoiceError::ContentSourceMismatch)
        ));
    }

    #[test]
    fn invalid_or_unresolved_lines_never_become_voice_candidates() {
        let mut input = source(1_000, 1_000_000);
        input.entries = [
            "|PLAYERNAME|",
            "{trade}",
            "bad\nline",
            "",
            "x".repeat(256).as_str(),
        ]
        .into_iter()
        .map(|text| ProjectV2Voice {
            text: text.into(),
            mode: ProjectV2SpeechMode::Say,
        })
        .collect();
        assert!(VoiceProfile::from_source("Alice", &input).is_none());
        input.entries.push(ProjectV2Voice {
            text: "Safe line".into(),
            mode: ProjectV2SpeechMode::Say,
        });
        let profile = VoiceProfile::from_source("Alice", &input).expect("safe line");
        assert_eq!(profile.lines.len(), 1);
        assert_eq!(profile.lines[0].text.as_str(), "Safe line");
        input.interval_ms = 0;
        assert!(VoiceProfile::from_source("Alice", &input).is_none());
        input.interval_ms = 1_000;
        input.chance_ppm = 1_000_001;
        assert!(VoiceProfile::from_source("Alice", &input).is_none());
    }

    #[test]
    fn cadence_and_retry_never_add_an_extra_tick_or_second_line() {
        let (runtime, _, actor, book) = fixture(source(2_000, 1_000_000));
        let mut state = NpcVoiceState::new(&runtime, actor, &book)
            .expect("owner")
            .expect("voices");
        assert!(matches!(
            state.think(&runtime, occurrence(actor, 1), &root()),
            Ok(VoiceThink::Quiet)
        ));
        assert!(matches!(
            state.think(&runtime, occurrence(actor, 1), &root()),
            Ok(VoiceThink::Replay)
        ));
        assert!(matches!(
            state.think(&runtime, occurrence(actor, 2), &root()),
            Ok(VoiceThink::Speak(_))
        ));
        assert!(matches!(
            state.think(&runtime, occurrence(actor, 2), &root()),
            Ok(VoiceThink::Replay)
        ));
        assert!(matches!(
            state.think(&runtime, occurrence(actor, 1), &root()),
            Err(VoiceError::OccurrenceSuperseded)
        ));
        assert!(matches!(
            state.think(&runtime, occurrence(actor, 3), &root()),
            Ok(VoiceThink::Quiet)
        ));
    }

    #[test]
    fn unperceived_time_does_not_accumulate_or_catch_up() {
        let (mut runtime, player, actor, book) = fixture(source(2_000, 1_000_000));
        let mut state = NpcVoiceState::new(&runtime, actor, &book)
            .expect("owner")
            .expect("voices");
        runtime.remove_test_actor(player).expect("player leaves");
        for sequence in 1..20 {
            assert!(matches!(
                state.think(&runtime, occurrence(actor, sequence), &root()),
                Ok(VoiceThink::Idle)
            ));
        }
        let session = crate::foundation::GameSessionId::decode(&[
            0x01, 0x90, 0, 0, 0, 0x75, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 0x75,
        ])
        .expect("new session");
        let reservation = runtime
            .reserve_fresh_session(session)
            .expect("reserve returning player");
        let returning = runtime
            .commit_fresh_session(reservation)
            .expect("returning player");
        runtime
            .initialize_pinned_test_position(
                returning,
                MovementLocalPosition {
                    x: 10,
                    y: 10,
                    floor: 7,
                },
            )
            .expect("near");
        assert!(matches!(
            state.think(&runtime, occurrence(actor, 20), &root()),
            Ok(VoiceThink::Quiet)
        ));
        assert!(matches!(
            state.think(&runtime, occurrence(actor, 100), &root()),
            Ok(VoiceThink::Speak(_))
        ));
    }

    #[test]
    fn absent_profile_player_actor_and_retired_owner_fail_closed() {
        let (mut runtime, player, actor, book) = fixture(source(1_000, 1_000_000));
        assert!(
            NpcVoiceState::new(&runtime, actor, &NpcVoiceBook::default())
                .expect("owner")
                .is_none()
        );
        assert!(matches!(
            NpcVoiceState::new(&runtime, player, &book),
            Err(VoiceError::NotCurrentNpc)
        ));
        let mut state = NpcVoiceState::new(&runtime, actor, &book)
            .expect("owner")
            .expect("voices");
        assert!(matches!(
            state.think(&runtime, occurrence(player, 1), &root()),
            Err(VoiceError::NotCurrentNpc)
        ));
        runtime.retire_owner_cycle();
        assert!(matches!(
            state.think(&runtime, occurrence(actor, 1), &root()),
            Err(VoiceError::NotCurrentNpc)
        ));
        assert_eq!(state.last_sequence, None);
        assert_eq!(state.ticks_ms, 0);
    }

    #[test]
    fn zero_chance_is_silent_and_replay_of_same_seed_selects_same_lines() {
        let (runtime, _, actor, book) = fixture(source(1_000, 0));
        let mut silent = NpcVoiceState::new(&runtime, actor, &book)
            .expect("owner")
            .expect("voices");
        assert!(matches!(
            silent.think(&runtime, occurrence(actor, 1), &root()),
            Ok(VoiceThink::Quiet)
        ));
        let (runtime, _, actor, book) = fixture(source(1_000, 1_000_000));
        let mut left = NpcVoiceState::new(&runtime, actor, &book)
            .expect("owner")
            .expect("voices");
        let mut right = NpcVoiceState::new(&runtime, actor, &book)
            .expect("owner")
            .expect("voices");
        for sequence in 1..20 {
            let (Ok(VoiceThink::Speak(a)), Ok(VoiceThink::Speak(b))) = (
                left.think(&runtime, occurrence(actor, sequence), &root()),
                right.think(&runtime, occurrence(actor, sequence), &root()),
            ) else {
                panic!("certain voice");
            };
            assert_eq!(a.line.text, b.line.text);
        }
    }

    #[test]
    fn native_projection_preserves_npc_yell_case_and_speaker_identity() {
        let (runtime, _, actor, book) = fixture(source(1_000, 1_000_000));
        let mut state = NpcVoiceState::new(&runtime, actor, &book)
            .expect("owner")
            .expect("voices");
        let Ok(VoiceThink::Speak(mut line)) = state.think(&runtime, occurrence(actor, 1), &root())
        else {
            panic!("voice");
        };
        let ChatLine::Local {
            speaker,
            speaker_name,
            text,
            mode,
            position,
        } = line.chat_line().expect("native say")
        else {
            panic!("local");
        };
        assert_eq!(speaker.identity, actor.placement_identity());
        assert_eq!(speaker_name, "Alice");
        assert_eq!(mode, ChatSpeechMode::Say);
        assert_eq!(
            position,
            ActorPosition {
                x: 11,
                y: 10,
                floor: 7
            }
        );
        assert_eq!(text, line.line.text.as_str());
        line.line.mode = ProjectV2SpeechMode::Yell;
        let ChatLine::Local {
            text: yelled, mode, ..
        } = line.chat_line().expect("native yell")
        else {
            panic!("local");
        };
        assert_eq!(mode, ChatSpeechMode::Yell);
        assert_eq!(yelled, text);
        assert_eq!(line.line.text.as_str(), text);
    }
}
