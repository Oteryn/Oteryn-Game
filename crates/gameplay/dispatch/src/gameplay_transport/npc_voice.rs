//! NPC-BEHAVIOUR0 §6: source-bound voice preparation in the current Channel owner.
//!
//! This consumes the existing typed catalogue; it does not admit new content or publish chat.
//! NPC-CONTENT-2 supplies the exact typed presence fields on NPC definitions. The CP integration
//! must run this only on the budgeted NPC think, select CHAT_V1 listeners in the same Channel and enqueue
//! through the chat owner. The native chat value carries the NPC's original text for both
//! say and yell, exempt from the player's yell transformation and ingress spam control.
//! There is no native source-catalogue-to-active-runtime-pin binding port here: matching the
//! catalogue and model does not establish that their voices belong to the running bundle.

#![allow(
    dead_code,
    reason = "NPC voice consumer awaits content admission and CP owner wiring"
)]

use crate::ai_think::profile_schedule::decision_occurrence;
use crate::ai_think::targeting::sees;
use crate::ai_think::{CREATURE_THINK_INTERVAL_MILLIS, ThinkOccurrence};
use crate::chat::ChatText;
use crate::content::{NpcDataCatalogue, NpcServiceModel};
use crate::foundation::{ChannelRuntimeV1, ExactActorRef, MovementPositionContext};
use oteryn_simulation_determinism::GameplayDecisionRoot;
use std::collections::BTreeMap;

#[allow(unused_imports, reason = "preserve the former NPC module path")]
pub use oteryn_gameplay_npc::npc_voice::{
    NpcVoiceProposal, VoiceError, VoiceLine, VoiceProfile, VoiceThink,
};

fn qualify_text(text: &str) -> Option<String> {
    ChatText::parse(text)
        .ok()
        .map(|text| text.as_str().to_owned())
}

/// Candidate source profiles only; this is not a replacement for NPC-CONTENT-2 admission.
#[derive(Debug, Default)]
pub struct NpcVoiceBook {
    profiles: BTreeMap<String, VoiceProfile>,
}

impl NpcVoiceBook {
    pub fn from_catalogue(
        catalogue: &NpcDataCatalogue,
        model: &NpcServiceModel,
    ) -> Result<Self, VoiceError> {
        oteryn_gameplay_npc::npc_voice::profiles_from_catalogue(catalogue, model, qualify_text)
            .map(|profiles| Self { profiles })
    }
}

/// One ephemeral state per exact actor, recreated from the source book at Channel restart.
#[derive(Debug)]
pub struct NpcVoiceState {
    actor: ExactActorRef,
    key: String,
    context: MovementPositionContext,
    profile: VoiceProfile,
    ticks_ms: u64,
    last_sequence: Option<u64>,
}

impl NpcVoiceState {
    pub fn new(
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
    pub fn think(
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
        let (result, ticks_ms) = self.profile.prepare(
            occurrence,
            npc.position,
            self.ticks_ms,
            CREATURE_THINK_INTERVAL_MILLIS,
            root,
            decision_occurrence(occurrence),
        )?;
        self.ticks_ms = ticks_ms;
        self.last_sequence = Some(occurrence.sequence);
        Ok(result)
    }
}

#[cfg(test)]
use crate::content::npc_catalogue::{VOICE_CHANCE, VOICE_INTERVAL, VOICE_LINES};
#[cfg(test)]
use crate::content::{
    ProjectV2CandidateField, ProjectV2CandidateValue, ProjectV2Declaration, ProjectV2SpeechMode,
    ProjectV2Voices,
};

#[cfg(test)]
use oteryn_protocol_oteryn::chat::{ChatLine, ChatSpeechMode};
#[cfg(test)]
use oteryn_protocol_oteryn::world_spatial::ActorPosition;

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
                VoiceProfile::from_source("Alice", &source, qualify_text).expect("source"),
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

    fn catalogue(revision: &str, text: &str) -> NpcDataCatalogue {
        catalogue_with_presence(revision, text, true)
    }

    fn catalogue_with_presence(revision: &str, text: &str, present: bool) -> NpcDataCatalogue {
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
                        fields: if present {
                            presence_fields(
                                1_000,
                                100,
                                &serde_json::json!([{"text": text, "yell": false}]).to_string(),
                            )
                        } else {
                            vec![]
                        },
                    },
                    ProjectV2Declaration::Npc {
                        identity: identity("oteryn:npc.mute"),
                        presentation: None,
                        behavior: None,
                        // A linked Dialogue is not a substitute for absent source voice fields.
                        dialogue: Some(ProjectV2DefinitionRef {
                            family: ProjectV2Family::Dialogue,
                            key: "oteryn:dialogue.alice".into(),
                            revision: "definition-r1".into(),
                        }),
                        services: vec![],
                        fields: vec![],
                    },
                    dialogue(
                        "oteryn:dialogue.alice",
                        "Dialogue-only text is not an NPC voice",
                    ),
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
    fn book_consumes_exact_native_npc_presence_fields_and_rejects_mixed_source_models() {
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
        assert!(VoiceProfile::from_source("Alice", &input, qualify_text).is_none());
        input.entries.push(ProjectV2Voice {
            text: "Safe line".into(),
            mode: ProjectV2SpeechMode::Say,
        });
        let profile = VoiceProfile::from_source("Alice", &input, qualify_text).expect("safe line");
        assert_eq!(profile.lines.len(), 1);
        assert_eq!(profile.lines[0].text.as_str(), "Safe line");
        input.interval_ms = 0;
        assert!(VoiceProfile::from_source("Alice", &input, qualify_text).is_none());
        input.interval_ms = 1_000;
        input.chance_ppm = 1_000_001;
        assert!(VoiceProfile::from_source("Alice", &input, qualify_text).is_none());
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

    #[test]
    fn missing_presence_fields_never_fall_back_to_linked_or_unreferenced_dialogue() {
        // Source-held and unsourced voices omit the presence fields. Both native NPCs
        // still reference an admitted Dialogue with voice data; neither may borrow it.
        let catalogue = catalogue_with_presence("voice-held-r1", "Never admitted", false);
        let model = catalogue.service_model_for_project().expect("native model");
        assert_eq!(model.npcs().len(), 2);
        assert!(model.npcs().values().all(|npc| npc.dialogue.is_some()));
        let book = NpcVoiceBook::from_catalogue(&catalogue, &model).expect("same source");
        assert!(book.profiles.is_empty());
        let (runtime, _, actor, _) = fixture(source(1_000, 1_000_000));
        assert!(
            NpcVoiceState::new(&runtime, actor, &book)
                .expect("native owner")
                .is_none()
        );
    }
}
