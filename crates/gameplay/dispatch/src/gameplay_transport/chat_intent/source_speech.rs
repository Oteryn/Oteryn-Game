//! The source portal speaks privately as each current spectator at the room centre.
use super::*;

impl ChatRuntime {
    pub(in crate::gameplay_transport) fn publish_rotten_massive_warning(
        &self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        session: GameSessionId,
        position: oteryn_protocol_oteryn::world_spatial::ActorPosition,
    ) {
        if runtime.player_control_facts(actor, session).is_err() {
            return;
        }
        let Some(generation) = NonZeroU64::new(actor.actor_local_generation()) else {
            return;
        };
        let Ok(mut sessions) = self.sessions.lock() else {
            return;
        };
        let Some(entry) = sessions.get_mut(&session) else {
            return;
        };
        entry.egress.push(ChatLine::Local {
            speaker: ChatSpeaker { identity: actor.placement_identity(), generation },
            speaker_name: entry.name.clone(),
            text: "the discharge of energy weakened the resistance of the boss shortly! PREPARE FOR 2 MASSIVE ATTACKS ON THE MAIN TARGET!".into(),
            mode: ChatSpeechMode::MonsterSay,
            position,
        });
    }
}
