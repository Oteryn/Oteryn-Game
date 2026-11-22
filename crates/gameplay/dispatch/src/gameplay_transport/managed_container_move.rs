use super::{ComposedFreshAdmission, connection::AdmittedSession};
use crate::durability::item_loop_move::managed_container_move::{Admission, Request, ResultView};
use crate::foundation::{CommandId, CommandRef};
use oteryn_protocol_oteryn::container_batch::Reply;

macro_rules! authority_methods {
    () => {
        fn managed_container_move(
            &self,
            _admitted: AdmittedSession,
            _id: u64,
            _request: crate::durability::item_loop_move::managed_container_move::Request,
        ) -> impl std::future::Future<
            Output = Result<
                Option<crate::durability::item_loop_move::managed_container_move::ResultView>,
                (),
            >,
        > {
            async { Ok(None) }
        }
    };
}
pub(super) use authority_methods;

macro_rules! composed_methods {
    () => {
        async fn managed_container_move(
            &self,
            admitted: AdmittedSession,
            id: u64,
            request: crate::durability::item_loop_move::managed_container_move::Request,
        ) -> Result<Option<crate::durability::item_loop_move::managed_container_move::ResultView>, ()> {
            managed_container_move::execute(self, admitted, id, request).await
        }
    };
}
pub(super) use composed_methods;

pub(super) async fn execute(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    id: u64,
    request: Request,
) -> Result<Option<ResultView>, ()> {
    let (Some(fence), Some(actor)) = (admitted.item_fence, admitted.runtime_actor) else {
        return Ok(None);
    };
    let Some(permit) = owner.spell_lane_permit().await else {
        return Err(());
    };
    let premium = owner
        .session_premium_gate(admitted.game_session_id)
        .await
        .require()
        .is_ok();
    let allowed = {
        let runtime = owner.runtime.lock().await;
        let states = owner.states.spell_states.lock().await;
        premium
            && runtime
                .player_control_facts(actor, admitted.game_session_id)
                .is_ok_and(|facts| facts.control_loss.is_none())
            && owner
                .states
                .player_trades
                .physical_service(admitted.game_session_id)
            && super::equipment_move::native_for_move(
                &runtime,
                owner.active_generation,
                states.is_dead(actor),
                states.has_pending_spell_commit(actor, admitted.game_session_id),
            )
            .is_some()
    };
    let Some(native) = owner
        .active_generation
        .and_then(|generation| generation.native_gameplay())
    else {
        return Ok(None);
    };
    let result = owner
        .root
        .move_to_managed_containers(
            Admission {
                permit: &permit,
                recovery: owner.character,
                node: owner.holder,
                fence,
                command: CommandRef::new(
                    admitted.game_session_id,
                    CommandId::new(id).map_err(|_| ())?,
                ),
                allowed,
            },
            request,
            &super::item_ref_admission::NativeItemWeights(native),
        )
        .await
        .map_err(|_| ())?;
    if result
        .as_ref()
        .is_some_and(|result| matches!(result.reply, Reply::Committed(_)))
    {
        owner
            .states
            .attack
            .lock()
            .await
            .forget_weapon(actor, admitted.game_session_id);
    }
    Ok(result)
}
