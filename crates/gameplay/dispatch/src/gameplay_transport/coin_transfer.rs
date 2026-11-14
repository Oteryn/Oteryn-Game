//! Session-fenced recipient review and Platform-authoritative gifts.
use super::{ComposedFreshAdmission, connection::AdmittedSession};
use crate::native_admission_source::coins::{Reply, Request as PlatformRequest};
use oteryn_protocol_oteryn::coin_transfer as wire;
use serde_json::{Value, json};
macro_rules! authority_methods {
    () => {
        fn coin_transfer_command(
            &self,
            _admitted: AdmittedSession,
            _id: u64,
            _payload: &[u8],
        ) -> impl std::future::Future<Output = Option<Vec<u8>>> {
            async {
                oteryn_protocol_oteryn::coin_transfer::encode_reply(r#"{"result":"unavailable"}"#)
                    .ok()
            }
        }
    };
}
pub(super) use authority_methods;
macro_rules! composed_methods {
    () => {
        async fn coin_transfer_command(
            &self,
            admitted: AdmittedSession,
            id: u64,
            payload: &[u8],
        ) -> Option<Vec<u8>> {
            crate::gameplay_transport::coin_transfer::execute(self, admitted, id, payload).await
        }
    };
}
pub(super) use composed_methods;
fn reply(v: Value) -> Option<Vec<u8>> {
    wire::encode_reply(&v.to_string()).ok()
}
fn uuid(v: &[u8; 16]) -> String {
    let h = v.iter().map(|b| format!("{b:02x}")).collect::<String>();
    format!(
        "{}-{}-{}-{}-{}",
        &h[..8],
        &h[8..12],
        &h[12..16],
        &h[16..20],
        &h[20..]
    )
}
pub(super) async fn execute(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    id: u64,
    payload: &[u8],
) -> Option<Vec<u8>> {
    let Ok(r) = wire::decode(payload) else {
        return reply(json!({"result":"invalid"}));
    };
    let Some(fence) = admitted.item_fence else {
        return reply(json!({"result":"rejected"}));
    };
    if id == 0
        || !admitted
            .continuity
            .selected_capabilities
            .contains(wire::CAPABILITY)
        || admitted.game_session_id != fence.game_session_id
        || admitted.continuity.connection_generation != fence.connection_generation.get()
        || fence.runtime_scope
            != crate::foundation::RuntimeScopeRefV1::channel(admitted.world_id, admitted.channel_id)
    {
        return reply(json!({"result":"rejected"}));
    }
    let coin = owner
        .root
        .coin_owner(
            owner.character,
            owner.holder,
            admitted.world_id,
            Some(fence),
        )
        .ok()?;
    let account = coin.account().await.ok()?;
    if r.action == 3 {
        return reply(coin.detail(uuid(&r.operation)).await.ok()?);
    }
    let Some(platform) = &owner.store else {
        return reply(json!({"result":"unavailable"}));
    };
    let mut platform = platform.lock().await;
    let Some(coins) = platform.coins() else {
        return reply(json!({"result":"unavailable"}));
    };
    if owner.store_purchases == super::StorePurchaseMode::DisabledRestoredBackup
        || !coins.transfers_enabled()
    {
        return reply(json!({"result":"unavailable"}));
    }
    let value = if r.action == 1 {
        match coins.execute(&PlatformRequest::Balance(account)).await {
            Ok(Reply::Balance(b)) => {
                let available = b
                    .transferable
                    .parse::<u64>()
                    .ok()?
                    .saturating_add(b.non_transferable.parse::<u64>().ok()?);
                if r.amount > available {
                    json!({"result":"insufficient_transferable"})
                } else {
                    let mut v = coin.gift_review(r).await.ok()?;
                    if v["result"] == "ok" {
                        v["transferable"] = b.transferable.into();
                        v["non_transferable"] = b.non_transferable.into();
                    }
                    v
                }
            }
            Ok(Reply::Refused(code)) => json!({"result":code}),
            _ => json!({"result":"unavailable"}),
        }
    } else {
        let result = coin.gift_confirm(r.clone()).await.ok()?;
        if result["result"] == "ok" {
            super::coin_market::reconcile_coin_market(&coin, coins).await;
            coin.detail(uuid(&r.operation)).await.ok()?
        } else {
            result
        }
    };
    reply(value)
}
