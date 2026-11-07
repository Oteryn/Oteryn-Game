//! Source-qualified native death speech uses the existing bounded CHAT egress.
//! Crystal201189f giant_spider_wyda_death.lua:4 (SHA256
//!4b78cf145d150264e8630ec73381e66b0fa4e1dca855292b93505805acacf0c2).
use super::*;
use oteryn_protocol_oteryn::world_spatial::ActorPosition;

impl ChatRuntime {
    pub(in crate::gameplay_transport) fn publish_native_death_notice(
        &self,
        runtime: &mut ChannelRuntimeV1,
        target: ExactActorRef,
        expected: &str,
        text: &'static str,
    ) -> bool {
        if runtime.owner_fence().is_err() {
            return false;
        }
        let Ok(snapshot) = runtime.companion_snapshot_including_dead(target) else {
            return false;
        };
        if snapshot.health != 0 || snapshot.state.policy.definition_key != expected {
            return false;
        }
        let name = snapshot.state.policy.display_name.clone();
        let Ok((death, at)) = runtime.borrow_combat_death().projected_death(target) else {
            return false;
        };
        let (Some(generation), Ok(floor)) = (
            NonZeroU64::new(death.actor_local_generation()),
            i8::try_from(at.floor),
        ) else {
            return false;
        };
        let Ok(mut sessions) = self.sessions.lock() else {
            return false;
        };
        for (session, state) in sessions.iter_mut() {
            let Ok(Some((actor, position))) = runtime.positioned_player_for_session(*session)
            else {
                continue;
            };
            if !runtime
                .player_control_facts(actor, *session)
                .is_ok_and(|facts| facts.control_loss.is_none())
            {
                continue;
            }
            let position = position.position();
            if position.floor != at.floor
                || position.x.abs_diff(at.x) > 8
                || position.y.abs_diff(at.y) > 6
            {
                continue;
            }
            state.egress.push(ChatLine::Local {
                speaker: ChatSpeaker {
                    identity: target.placement_identity(),
                    generation,
                },
                speaker_name: name.clone(),
                mode: ChatSpeechMode::MonsterSay,
                text: text.into(),
                position: ActorPosition {
                    x: at.x,
                    y: at.y,
                    floor: i16::from(floor),
                },
            });
        }
        true
    }
}
