//! T135 admitted selection -> current actor -> fenced ownership/bank authority.
use super::{ComposedFreshAdmission, connection::AdmittedSession};
use crate::durability::house_sale::HouseSaleError;
use crate::foundation::{CommandId, CommandRef};
use oteryn_protocol_oteryn::house_sale as wire;
#[derive(Debug)]
pub enum HouseSaleReply {
    Rejected,
    Payload(Vec<u8>),
    Unavailable,
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
        if !admitted
            .continuity
            .selected_capabilities
            .contains(wire::CAPABILITY)
            || fence.game_session_id != admitted.game_session_id
            || fence.connection_generation.get() != admitted.continuity.connection_generation
            || admitted.world_id != binding.world_id()
            || Some(admitted.channel_id) != binding.channel_id()
            || fence.runtime_scope != binding.runtime_scope()
            || fence.scope_ownership_generation != binding.scope_generation()
            || runtime.owner_fence().is_err()
            || !admitted.runtime_actor.is_some_and(|actor| {
                runtime
                    .player_control_facts(actor, admitted.game_session_id)
                    .is_ok_and(|facts| facts.control_loss.is_none())
            })
        {
            return HouseSaleReply::Rejected;
        }
        let snapshot = match self
            .root
            .house_sale_request(
                self.character,
                self.holder,
                self.premium_coordinator,
                fence,
                CommandRef::new(admitted.game_session_id, id),
                request,
            )
            .await
        {
            Ok(snapshot) => snapshot,
            Err(HouseSaleError::Rejected | HouseSaleError::Conflict) => {
                return HouseSaleReply::Rejected;
            }
            Err(HouseSaleError::Unavailable(error)) => {
                #[cfg(test)]
                eprintln!("T135 house sale unavailable: {error:?}");
                #[cfg(not(test))]
                let _ = error;
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
