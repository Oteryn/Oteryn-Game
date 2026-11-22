use super::{ComposedFreshAdmission, connection::AdmittedSession};
use oteryn_protocol_oteryn::{
    world_spatial_entities::EntityRef,
    zones_nologout::{PlayerPresence, PlayerStatus},
};

macro_rules! authority_methods {
    () => {
        fn player_status(
            &self,
            _admitted: AdmittedSession,
            _target: oteryn_protocol_oteryn::world_spatial_entities::EntityRef,
        ) -> impl std::future::Future<
            Output = Option<oteryn_protocol_oteryn::zones_nologout::PlayerStatus>,
        > {
            async { None }
        }
    };
}
pub(super) use authority_methods;

macro_rules! composed_methods {
    () => {
        async fn player_status(
            &self,
            admitted: AdmittedSession,
            target: oteryn_protocol_oteryn::world_spatial_entities::EntityRef,
        ) -> Option<oteryn_protocol_oteryn::zones_nologout::PlayerStatus> {
            player_status::observe(self, admitted, target).await
        }
    };
}
pub(super) use composed_methods;

pub(super) async fn observe(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    target: EntityRef,
) -> Option<PlayerStatus> {
    let observer = admitted.runtime_actor?;
    let mut runtime = owner.runtime.lock().await;
    runtime.owner_fence().ok()?;
    if runtime
        .player_control_facts(observer, admitted.game_session_id)
        .ok()?
        .control_loss
        .is_some()
        || observer.placement_identity() == target.identity
    {
        return None;
    }
    let super::item_look::RuntimeLook::Player { actor, session, .. } =
        super::item_look::runtime_look_creature(&mut runtime, None, observer, target).ok()?
    else {
        return None;
    };
    let facts = runtime.player_control_facts(actor, session).ok()?;
    let away = if facts.control_loss.is_some() {
        true
    } else {
        let idle = owner
            .states
            .gameplay_idle_presence
            .retained_idle(session, actor)?;
        let last = idle.last_activity()?;
        last.elapsed().as_millis() >= u128::from(owner.states.gameplay_idle_policy.warning_after_ms)
    };
    Some(PlayerStatus {
        presence: if away {
            PlayerPresence::Away
        } else {
            PlayerPresence::Online
        },
        no_logout: owner.no_logout_tile(&mut runtime, actor)?,
    })
}
