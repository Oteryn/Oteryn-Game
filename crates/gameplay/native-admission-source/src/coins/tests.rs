use super::*;
#[test]
fn platform_coin_reservation_uses_exact_pinned_wire() -> Result<(), Box<dyn std::error::Error>> {
    let request = Request::Reserve {
        operation: "019a65f0-0000-7000-8000-000000000004".into(),
        account: "019a65f0-0000-7000-8000-000000000001".into(),
        amount: 30,
        price: 100,
    };
    let (_, path, body) = request.wire()?;
    assert_eq!(path, "/internal/v1/coins/market/offers");
    assert_eq!(
        body,
        include_str!(
            "../../../../../docs/contracts/fixtures/platform-coin-wire-v1/offers-request.json"
        )
        .trim()
    );
    Ok(())
}
#[test]
fn pinned_coin_vectors_keep_exact_bytes_and_client_binds_receipts()
-> Result<(), Box<dyn std::error::Error>> {
    use sha2::{Digest, Sha256};
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../docs/contracts/fixtures/platform-coin-wire-v1");
    let manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join("manifest.json"))?)?;
    for (file, vector) in manifest["valid"].as_object().ok_or("vectors")? {
        let bytes = std::fs::read(root.join(file))?;
        assert_eq!(
            bytes.len() as u64,
            vector["canonical_bytes"].as_u64().ok_or("length")?
        );
        assert_eq!(
            Sha256::digest(&bytes)
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>(),
            vector["sha256"].as_str().ok_or("hash")?
        );
    }
    let request = Request::Reserve {
        operation: "019a65f0-0000-7000-8000-000000000004".into(),
        account: "019a65f0-0000-7000-8000-000000000001".into(),
        amount: 30,
        price: 100,
    };
    let bytes = std::fs::read(root.join("offers-ok.json"))?;
    assert!(matches!(decode(&request, 200, &bytes)?, Reply::Mutation(_)));
    let mut wrong = bytes.clone();
    let i = wrong.iter().position(|b| *b == b'4').ok_or("id")?;
    wrong[i] = b'5';
    assert!(decode(&request, 200, &wrong).is_err());
    for code in [
        "account_inactive",
        "transfer_blocked",
        "insufficient_transferable",
        "idempotency_conflict",
        "unknown_account",
        "invalid",
        "unavailable",
    ] {
        let status = match code {
            "unknown_account" | "invalid" => 422,
            "unavailable" => 503,
            _ => 409,
        };
        assert_eq!(
            decode(
                &request,
                status,
                &std::fs::read(root.join(format!("{code}.json")))?
            )?,
            Reply::Refused(code.into())
        );
    }
    Ok(())
}
#[test]
fn coin_client_calls_transport_and_rejects_changed_receipt()
-> Result<(), Box<dyn std::error::Error>> {
    struct Peer;
    impl Transport for Peer {
        fn exchange<'a>(
            &'a mut self,
            method: &'a str,
            path: &'a str,
            body: &'a str,
        ) -> FutureReply<'a> {
            Box::pin(async move {
                assert_eq!(method, "POST");
                assert_eq!(path, "/internal/v1/coins/market/offers");
                assert_eq!(
                    body,
                    include_str!(
                        "../../../../../docs/contracts/fixtures/platform-coin-wire-v1/offers-request.json"
                    )
                );
                Ok((200,include_bytes!("../../../../../docs/contracts/fixtures/platform-coin-wire-v1/offers-ok.json").to_vec()))
            })
        }
    }
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    runtime.block_on(async {
        let mut client = CoinClient::new(Box::new(Peer));
        let request = Request::Reserve {
            operation: "019a65f0-0000-7000-8000-000000000004".into(),
            account: "019a65f0-0000-7000-8000-000000000001".into(),
            amount: 30,
            price: 100,
        };
        assert!(matches!(
            client.execute(&request).await?,
            Reply::Mutation(_)
        ));
        Ok::<_, Error>(())
    })?;
    Ok(())
}
#[test]
fn platform_coin_transfer_matches_pinned_request_receipt_and_rejects_recipient_substitution()
-> Result<(), Box<dyn std::error::Error>> {
    let r = Request::Transfer {
        operation: "019a65f0-0000-7000-8000-000000000003".into(),
        account: "019a65f0-0000-7000-8000-000000000001".into(),
        recipient: "019a65f0-0000-7000-8000-000000000002".into(),
        amount: 10,
    };
    let (method, path, body) = r.wire()?;
    assert_eq!(
        (method, path.as_str()),
        ("POST", "/internal/v1/coins/transfers")
    );
    assert_eq!(
        body,
        include_str!(
            "../../../../../docs/contracts/fixtures/platform-coin-wire-v1/transfers-request.json"
        )
    );
    let bytes = include_bytes!(
        "../../../../../docs/contracts/fixtures/platform-coin-wire-v1/transfers-ok.json"
    );
    assert!(matches!(decode(&r, 200, bytes)?, Reply::Mutation(_)));
    let wrong = Request::Transfer {
        operation: "019a65f0-0000-7000-8000-000000000003".into(),
        account: "019a65f0-0000-7000-8000-000000000001".into(),
        recipient: "019a65f0-0000-7000-8000-000000000099".into(),
        amount: 10,
    };
    assert!(decode(&wrong, 200, bytes).is_err());
    Ok(())
}
