use super::*;
use crate::native_admission_source::{
    descriptor::Operation,
    store::{StoreTransport, TransportFailure},
};

struct Platform {
    foreign_account: bool,
}
impl StoreTransport for Platform {
    fn exchange<'a>(
        &'a mut self,
        operation: Operation,
        body: &'a str,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Vec<u8>, TransportFailure>> + Send + 'a>,
    > {
        Box::pin(async move {
            assert_eq!(operation, Operation::ReadStoreHistoryPageV1);
            let request: serde_json::Value =
                serde_json::from_str(body).map_err(|_| TransportFailure::InvalidResponse)?;
            assert_eq!(request["before_entry_id"], "0");
            assert_eq!(request["max_entries"], 32);
            let kinds = [
                "STORE_PURCHASE",
                "STORE_REVERSAL",
                "COIN_PACKAGE_CREDIT",
                "COIN_PACKAGE_REVERSAL",
                "AUCTION_RESERVE",
                "AUCTION_RELEASE",
                "AUCTION_SETTLE",
                "OPERATOR_ADJUSTMENT",
                "OTHER",
            ];
            let entries: Vec<_> = kinds.iter().enumerate().map(|(i, kind)| {
                let purchase = i < 2;
                let (available, reserved) = [(-25, 0), (25, 0), (250, 0), (-250, 0), (-100, 100), (100, -100), (0, -100), (3, 0), (-4, 0)][i];
                serde_json::json!({ "entry_id": (99-i).to_string(), "kind": kind, "currency": "COINS",
                    "available_delta": available.to_string(), "reserved_delta": reserved.to_string(),
                    "order_id": purchase.then_some("018f3a00-0000-7000-8000-000000000007"),
                    "offer_key": purchase.then_some("store:exercise-sword"), "quantity": purchase.then_some(1), "created_at": "100" })
            }).collect();
            // Platform's signed wire requires canonical member order, including nested rows.
            let rows = entries
                .iter()
                .map(|entry| {
                    let mut fields = Vec::new();
                    for key in [
                        "entry_id",
                        "kind",
                        "currency",
                        "available_delta",
                        "reserved_delta",
                        "order_id",
                        "offer_key",
                        "quantity",
                        "created_at",
                    ] {
                        let value = serde_json::to_string(&entry[key])
                            .map_err(|_| TransportFailure::InvalidResponse)?;
                        fields.push(format!("\"{key}\":{value}"));
                    }
                    Ok(format!("{{{}}}", fields.join(",")))
                })
                .collect::<Result<Vec<_>, TransportFailure>>()?;
            let account = if self.foreign_account {
                "018f3a00-0000-7000-8000-000000000009"
            } else {
                request["account_id"]
                    .as_str()
                    .ok_or(TransportFailure::InvalidResponse)?
            };
            Ok(format!("{{\"contract_version\":1,\"result\":\"OK\",\"account_id\":\"{account}\",\"entries\":[{}],\"next_before_entry_id\":\"91\"}}", rows.join(",")).into_bytes())
        })
    }
}
#[tokio::test]
async fn coin_history_dispatch_projection_reads_all_platform_kinds_and_rejects_foreign_accounts()
-> Result<(), Box<dyn std::error::Error>> {
    for foreign_account in [false, true] {
        let mut client = StoreClient::new(
            Box::new(Platform { foreign_account }),
            "oteryn-game/store-consumer".into(),
        );
        let page = page(
            &mut client,
            "018f3a00-0000-7000-8000-000000000008",
            &wire::StoreHistoryRequest {
                page_cursor: String::new(),
            },
        )
        .await;
        if foreign_account {
            assert!(page.is_none());
            continue;
        }
        let page = page.ok_or("ledger page")?;
        let bytes = wire::encode_coin_history_page(&page).map_err(|e| format!("{e:?}"))?;
        let decoded = wire::decode_coin_history_page(&bytes).map_err(|e| format!("{e:?}"))?;
        assert_eq!(decoded.entries.len(), 9);
        assert_eq!(decoded.entries[0].available_delta, -25);
        assert_eq!(decoded.entries[1].available_delta, 25);
        assert_eq!(decoded.entries[2].kind, "COIN_PACKAGE_CREDIT");
        assert_eq!(decoded.next_page_cursor, "91");
    }
    Ok(())
}
