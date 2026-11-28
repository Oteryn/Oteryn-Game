//! T135 admitted selection -> current actor -> fenced ownership/bank authority.
use super::{AdmissionRetryBudget, ComposedFreshAdmission, connection::{AdmittedSession, AdmissionRefusal}};
use crate::durability::house_sale::HouseSaleError;
use crate::durability::{DurabilityError, SqlxFailureClass};
use crate::foundation::{CommandId, CommandRef};
use oteryn_protocol_oteryn::house_sale as wire;
#[derive(Debug)]
pub enum HouseSaleReply {
    Rejected,
    Payload(Vec<u8>),
    Unavailable,
}

fn catalogue_contention(error: &DurabilityError) -> bool {
    matches!(
        error,
        DurabilityError::RootUnavailable
            | DurabilityError::RootPassDeadlineExceeded
            | DurabilityError::Database(SqlxFailureClass::RetryableStatement)
    )
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn house_sale_wire(
        &self,
        admitted: AdmittedSession,
        id: u64,
        payload: &[u8],
    ) -> HouseSaleReply {
        let Ok(request) = wire::decode_request(payload) else {
            return HouseSaleReply::Rejected;
        };
        let Some(fence) = admitted.item_fence else {
            return HouseSaleReply::Rejected;
        };
        let Ok(id) = CommandId::new(id) else {
            return HouseSaleReply::Rejected;
        };
        let runtime = self.runtime.lock().await;
        let binding = runtime.binding();
        let actor_current=admitted.runtime_actor.is_some_and(|actor| match fence.runtime_scope {
            crate::foundation::RuntimeScopeRefV1::Channel{..}=>
                Some(admitted.channel_id)==binding.channel_id() && fence.runtime_scope==binding.runtime_scope()
                && fence.scope_ownership_generation==binding.scope_generation() && runtime.owner_fence().is_ok()
                && runtime.player_control_facts(actor,admitted.game_session_id).is_ok_and(|facts|facts.control_loss.is_none()),
            crate::foundation::RuntimeScopeRefV1::Instance{..}=>
                runtime.house_actor_state(actor,admitted.game_session_id).is_ok_and(|state|
                    state.scope==fence.runtime_scope && state.generation==fence.scope_ownership_generation
                    && state.lease.generation()==fence.character_lease_generation
                    && state.lease.character_id().as_bytes()==fence.character_id.as_bytes())
                && runtime.house_player_control_facts(actor,admitted.game_session_id).is_ok_and(|facts|facts.control_loss.is_none()),
        });
        if !admitted.continuity.selected_capabilities.contains(wire::CAPABILITY)
            || fence.game_session_id!=admitted.game_session_id
            || fence.connection_generation.get()!=admitted.continuity.connection_generation
            || admitted.world_id!=binding.world_id() || !actor_current {
            #[cfg(test)] eprintln!("T135 sale actor rejected current={actor_current} scope={:?}",fence.runtime_scope);
            return HouseSaleReply::Rejected;
        }
        let catalogue = request.action == 0;
        let request_sale = async |request| {
            self.root.house_sale_request(
                self.character,
                self.holder,
                self.premium_coordinator,
                fence,
                CommandRef::new(admitted.game_session_id, id),
                request,
            )
            .await
        };
        let result = if catalogue {
            // The catalogue is read-only. Each attempt re-proves the durable
            // session fence. Let settlement advance while the holder is busy.
            drop(runtime);
            let retry = AdmissionRetryBudget::for_command(self.root);
            loop {
                let result = request_sale(request.clone()).await;
                if !matches!(&result, Err(HouseSaleError::Unavailable(error)) if catalogue_contention(error))
                    || !retry.retries_unavailable(AdmissionRefusal::Unavailable).await
                {
                    break result;
                }
            }
        } else {
            request_sale(request).await
        };
        let snapshot = match result {
            Ok(snapshot) => snapshot,
            Err(HouseSaleError::Rejected | HouseSaleError::Conflict) => {
                return HouseSaleReply::Rejected;
            }
            Err(HouseSaleError::Unavailable(error)) => {
                super::operator_event(&format!(
                    "event=house_sale_unavailable catalogue={catalogue} command_id={} error={error:?}",
                    id.get(),
                ));
                if catalogue && catalogue_contention(&error) {
                    return HouseSaleReply::Rejected;
                }
                return HouseSaleReply::Unavailable;
            }
        };
        let mut rows = Vec::with_capacity(snapshot.rows.len());
        for row in snapshot.rows {
            let Some(details) = super::house_query::details(&row.house) else {
                return HouseSaleReply::Unavailable;
            };
            rows.push(wire::HouseSaleRow {
                house: row.house,
                name: details.name,
                revision: row.revision,
                owner: row.owner,
                recipient: row.recipient,
                transfer_state: row.transfer_state,
                due_at_unix_ms: row.due_at_unix_ms,
                price: row.price,
                rent: row.rent,
                allowed: row.allowed,
                move_out_state: row.move_out_state,
            });
        }
        wire::encode_reply(&wire::HouseSaleReply {
            now_unix_ms: snapshot.now_unix_ms,
            rows,
            has_more: snapshot.has_more,
        })
        .map_or(HouseSaleReply::Unavailable, HouseSaleReply::Payload)
    }
}
macro_rules! authority_methods {
    () => {
        fn house_sale_command(&self,_admitted:$crate::gameplay_transport::connection::AdmittedSession,_id:u64,_payload:&[u8])
        ->impl std::future::Future<Output=$crate::gameplay_transport::house_sale::HouseSaleReply> {
            async {$crate::gameplay_transport::house_sale::HouseSaleReply::Rejected}
        }
    };
}
pub(super) use authority_methods;
macro_rules! composed_methods {
    () => {
        async fn house_sale_command(
            &self,
            admitted: connection::AdmittedSession,
            id: u64,
            payload: &[u8],
        ) -> house_sale::HouseSaleReply {
            self.house_sale_wire(admitted, id, payload).await
        }
    };
}
pub(super) use composed_methods;
