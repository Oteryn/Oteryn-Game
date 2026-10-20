//! Fixed source text captured after current source/fence admission; no arbitrary text API.
use crate::creature_auto_attack::AttackError;
use crate::foundation::{ChannelRuntimeV1, ExactActorRef, MovementLocalPosition};
use oteryn_protocol_oteryn::chat::{ChatLine, ChatSpeaker, ChatSpeechMode};
use oteryn_protocol_oteryn::world_spatial::ActorPosition;
use std::num::NonZeroU64;
pub(crate) struct CallbackSpeech {
    at: MovementLocalPosition,
    line: ChatLine,
}
impl CallbackSpeech {
    fn capture(
        r: &ChannelRuntimeV1,
        actor: ExactActorRef,
        key: &str,
        name: &str,
        text: &str,
        mode: ChatSpeechMode,
    ) -> Result<Self, AttackError> {
        if !r.matches_live_creature_identity(actor, key.as_bytes()) {
            return Err(AttackError::StaleIssuer);
        }
        let at = r
            .read_actor_position(actor)
            .map_err(|_| AttackError::StaleIssuer)?
            .position();
        Ok(Self {
            at,
            line: ChatLine::Local {
                speaker: ChatSpeaker {
                    identity: actor.placement_identity(),
                    generation: NonZeroU64::new(actor.actor_local_generation())
                        .ok_or(AttackError::StaleIssuer)?,
                },
                speaker_name: name.into(),
                mode,
                text: text.into(),
                position: ActorPosition {
                    x: at.x,
                    y: at.y,
                    floor: at.floor,
                },
            },
        })
    }
    pub(crate) fn cast(
        r: &ChannelRuntimeV1,
        actor: ExactActorRef,
        key: &str,
        completion: bool,
    ) -> Result<Option<Self>, AttackError> {
        let (name, text) = match (key, completion) {
            ("oteryn:creature.gaz_haragoth", false) => (
                "Gaz'haragoth",
                "Gaz'haragoth begins to channel DEATH AND DOOM into the area! RUN!",
            ),
            ("oteryn:creature.gaz_haragoth", true) => {
                ("Gaz'haragoth", "Gaz'haragoth calls down: DEATH AND DOOM!")
            }
            ("oteryn:creature.lady_tenebris", false) => (
                "Lady Tenebris",
                "LADY TENEBRIS BEGINS TO CHANNEL A POWERFULL SPELL! TAKE COVER!",
            ),
            _ => return Ok(None),
        };
        Self::capture(r, actor, key, name, text, ChatSpeechMode::Yell).map(Some)
    }
    // Only the spawn owner calls this after source-qualified physical admissions.
    pub(crate) fn spawn(
        r: &ChannelRuntimeV1,
        actor: ExactActorRef,
        key: &str,
    ) -> Result<Option<Self>, AttackError> {
        let (name, text, mode) = match key {
            "oteryn:creature.gaz_haragoth" => (
                "Gaz'haragoth",
                "Minions! Follow my call!",
                ChatSpeechMode::Say,
            ),
            "oteryn:creature.glooth_generator" => (
                "Glooth Generator",
                "The fully charged generator explodes in a blast!",
                ChatSpeechMode::Yell,
            ),
            "oteryn:creature.shadow_fiend" => (
                "Shadow Fiend",
                "The shadow fiend revives!",
                ChatSpeechMode::Say,
            ),
            _ => return Ok(None),
        };
        Self::capture(r, actor, key, name, text, mode).map(Some)
    }
    pub(crate) fn generator_warning(
        r: &ChannelRuntimeV1,
        actor: ExactActorRef,
    ) -> Result<Self, AttackError> {
        Self::capture(
            r,
            actor,
            "oteryn:creature.glooth_generator",
            "Glooth Generator",
            "THE GLOOTH GENERATOR CHARGES UP FOR A LETHAL EXPLOSION!",
            ChatSpeechMode::Yell,
        )
    }
    pub(crate) fn position(&self) -> MovementLocalPosition {
        self.at
    }
    pub(crate) fn line(&self) -> &ChatLine {
        &self.line
    }
}
