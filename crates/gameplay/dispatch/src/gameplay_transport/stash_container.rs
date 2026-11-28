use super::{ComposedFreshAdmission, connection::AdmittedSession};
use crate::durability::{
    DurabilityError,
    character_stash::container::{Admission, Request, ResultView},
};
use crate::foundation::{CommandId, CommandRef};

macro_rules! authority_methods {
    () => {
        fn stash_container(
            &self,
            _admitted: AdmittedSession,
            _id: u64,
            _request: crate::durability::character_stash::container::Request,
        ) -> impl std::future::Future<
            Output = Result<
                Option<crate::durability::character_stash::container::ResultView>,
                crate::durability::DurabilityError,
            >,
        > {
            async { Ok(None) }
        }
    };
}
pub(super) use authority_methods;

macro_rules! composed_methods {
    () => {
        async fn stash_container(
            &self,
            admitted: AdmittedSession,
            id: u64,
            request: crate::durability::character_stash::container::Request,
        ) -> Result<
            Option<crate::durability::character_stash::container::ResultView>,
            crate::durability::DurabilityError,
        > {
            stash_container::execute(self, admitted, id, request).await
        }
    };
}
pub(super) use composed_methods;

pub(super) async fn execute(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    id: u64,
    request: Request,
) -> Result<Option<ResultView>, DurabilityError> {
    let (Some(fence), Some(actor)) = (admitted.item_fence, admitted.runtime_actor) else {
        return Ok(None);
    };
    let Some(permit) = owner.spell_lane_permit().await else {
        return Err(DurabilityError::Unavailable);
    };
    let (native, reachable) = {
        let mut runtime = owner.runtime.lock().await;
        let states = owner.states.spell_states.lock().await;
        let live = runtime
            .player_control_facts(actor, admitted.game_session_id)
            .is_ok_and(|facts| facts.control_loss.is_none())
            && !owner.states.player_trades.open(admitted.game_session_id)
            && super::equipment_move::native_for_move(
                &runtime,
                owner.active_generation,
                states.is_dead(actor),
                states.has_pending_spell_commit(actor, admitted.game_session_id),
            )
            .is_some();
        let reachable = live
            && super::bundle_map_position(&mut runtime, owner.movement_cells(), actor)
                .as_ref()
                .and_then(super::depot::locker)
                .is_some();
        (
            owner
                .active_generation
                .and_then(|generation| generation.native_gameplay()),
            reachable,
        )
    };
    let Some(native) = native else {
        return Ok(None);
    };
    owner
        .root
        .stash_container(
            Admission {
                permit: &permit,
                recovery: owner.character,
                node: owner.holder,
                fence,
                command: CommandRef::new(
                    admitted.game_session_id,
                    CommandId::new(id).map_err(|_| DurabilityError::InvalidStoredState)?,
                ),
                reachable,
            },
            request,
            native,
            &super::item_ref_admission::NativeItemWeights(native),
        )
        .await
}
