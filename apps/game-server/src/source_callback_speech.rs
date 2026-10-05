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
    pub(super) fn cast(
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
#[cfg(test)]
mod tests {
    use super::*;
    use crate::gameplay_transport::actor_spell::tests::runtime_with_player;
    #[test]
    fn fixed_source_chatline_current_recipient_historical_speaker_and_stale_refusal() {
        let (mut r, player, session) = runtime_with_player(0x67);
        let at = MovementLocalPosition {
            x: 100,
            y: 100,
            floor: 7,
        };
        r.initialize_movement_test_position(player, at).unwrap();
        let actor = r
            .admit_monster_lab_creature(at, "oteryn:creature.gaz_haragoth", 300000)
            .unwrap();
        let line = CallbackSpeech::cast(&r, actor, "oteryn:creature.gaz_haragoth", false)
            .unwrap()
            .unwrap();
        let mut box_ = crate::weak_spot_speech::WeakSpotSpeechMailbox::default();
        assert_eq!(
            box_.publish_callback(&r, &line, &[(player, session), (player, session)])
                .unwrap(),
            1
        );
        r.remove_test_actor(actor).unwrap();
        assert!(matches!(
            CallbackSpeech::cast(&r, actor, "oteryn:creature.gaz_haragoth", false),
            Err(AttackError::StaleIssuer)
        ));
        let bytes = box_.drain(&r, player, session).unwrap();
        assert_eq!(bytes.len(), 1);
        let decoded = oteryn_protocol_oteryn::chat::decode_chat_line(&bytes[0]).unwrap();
        assert_eq!(&decoded, line.line());
        assert!(box_.drain(&r, player, session).unwrap().is_empty());
        let ChatLine::Local {
            speaker,
            mode,
            text,
            ..
        } = decoded
        else {
            panic!("fixed callback must encode Local")
        };
        assert_eq!(speaker.identity, actor.placement_identity());
        assert_eq!(speaker.generation.get(), actor.actor_local_generation());
        assert_eq!(mode, ChatSpeechMode::Yell);
        assert_eq!(
            text,
            "Gaz'haragoth begins to channel DEATH AND DOOM into the area! RUN!"
        );
        // Old session cannot receive any queued fixed payload even with the same actor.
        assert_eq!(
            box_.publish_callback(&r, &line, &[(player, runtime_with_player(0x69).2)])
                .unwrap(),
            0
        );
    }
    #[test]
    fn callback_say_and_yell_use_existing_distinct_hearing_and_fixed_spawn_text() {
        let (mut r, player, session) =
            crate::gameplay_transport::actor_spell::tests::runtime_with_capacity(0x68, 4);
        let at = MovementLocalPosition {
            x: 100,
            y: 100,
            floor: 7,
        };
        r.initialize_movement_test_position(
            player,
            MovementLocalPosition {
                x: 109,
                y: 100,
                floor: 7,
            },
        )
        .unwrap();
        let gaz = r
            .admit_monster_lab_creature(at, "oteryn:creature.gaz_haragoth", 300000)
            .unwrap();
        let say = CallbackSpeech::spawn(&r, gaz, "oteryn:creature.gaz_haragoth")
            .unwrap()
            .unwrap();
        let yell = CallbackSpeech::cast(&r, gaz, "oteryn:creature.gaz_haragoth", true)
            .unwrap()
            .unwrap();
        let mut box_ = crate::weak_spot_speech::WeakSpotSpeechMailbox::default();
        assert_eq!(
            box_.publish_callback(&r, &say, &[(player, session)])
                .unwrap(),
            0
        );
        assert_eq!(
            box_.publish_callback(&r, &yell, &[(player, session)])
                .unwrap(),
            1
        );
        assert_eq!(box_.drain(&r, player, session).unwrap().len(), 1);
        let generator = r
            .admit_monster_lab_creature(
                MovementLocalPosition {
                    x: 101,
                    y: 100,
                    floor: 7,
                },
                "oteryn:creature.glooth_generator",
                20000,
            )
            .unwrap();
        let warning = CallbackSpeech::generator_warning(&r, generator).unwrap();
        let explosion = CallbackSpeech::spawn(&r, generator, "oteryn:creature.glooth_generator")
            .unwrap()
            .unwrap();
        for line in [&warning, &explosion] {
            assert!(matches!(
                line.line(),
                ChatLine::Local {
                    mode: ChatSpeechMode::Yell,
                    ..
                }
            ));
        }
        let child = r
            .admit_monster_lab_creature(
                MovementLocalPosition {
                    x: 102,
                    y: 100,
                    floor: 7,
                },
                "oteryn:creature.shadow_fiend",
                8000,
            )
            .unwrap();
        let line = CallbackSpeech::spawn(&r, child, "oteryn:creature.shadow_fiend")
            .unwrap()
            .unwrap();
        assert!(
            matches!(line.line(),ChatLine::Local{text,mode:ChatSpeechMode::Say,..}if text=="The shadow fiend revives!")
        );
    }
}
