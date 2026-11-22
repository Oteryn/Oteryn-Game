use super::{ComposedFreshAdmission, connection::AdmittedSession, death_protection::query_fence};
use oteryn_protocol_oteryn::wheel::GiftActivationReply;
use sha2::{Digest, Sha256};

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn observe_wheel_gift_activation(
        &self,
        admitted: AdmittedSession,
    ) -> Option<GiftActivationReply> {
        let actor = admitted.runtime_actor?;
        let fence = query_fence(admitted)?;
        let session = admitted.game_session_id;
        if self.current_quest_fence(session).await.ok()??.item_fence() != fence {
            return None;
        }
        let runtime = self.runtime.lock().await;
        if runtime
            .player_control_facts(actor, session)
            .ok()?
            .control_loss
            .is_some()
        {
            return None;
        }
        let states = self.states.spell_states.lock().await;
        let generation = states
            .get(&runtime, actor, session)?
            .wheel_gift_activation();
        let registration = self.holder.fact();
        Some(GiftActivationReply(generation.map(|generation| {
            let digest = Sha256::new()
                .chain_update(b"oteryn:wheel-gift-activation:v1")
                .chain_update(registration.node_id().as_bytes())
                .chain_update(registration.registration_revision().to_le_bytes())
                .chain_update(actor.placement_identity())
                .chain_update(session.as_bytes())
                .chain_update(generation.to_le_bytes())
                .finalize();
            let mut occurrence = [0; 16];
            occurrence.copy_from_slice(&digest[..16]);
            occurrence
        })))
    }
}

macro_rules! authority_methods {
    () => {
        fn wheel_gift_activation(
            &self,
            _admitted: $crate::gameplay_transport::AdmittedSession,
        ) -> impl std::future::Future<Output = Option<oteryn_protocol_oteryn::wheel::GiftActivationReply>> {
            async { None }
        }
    };
}

pub(super) use authority_methods;

macro_rules! composed_methods {
    () => {
        async fn wheel_gift_activation(
            &self,
            admitted: $crate::gameplay_transport::AdmittedSession,
        ) -> Option<oteryn_protocol_oteryn::wheel::GiftActivationReply> {
            self.observe_wheel_gift_activation(admitted).await
        }
    };
}
pub(super) use composed_methods;
