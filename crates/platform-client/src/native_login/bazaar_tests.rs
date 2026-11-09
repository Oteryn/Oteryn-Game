use super::*;
use crate::native_login::bazaar::*;

#[test]
fn bazaar_wire_rejects_unknown_fields_bad_versions_and_mismatched_receipts() -> TestResult {
    let runtime = tokio::runtime::Runtime::new()?;
    let token_body = r#"{"token_type":"Bearer","access_token":"services"}"#.to_owned();
    let valid = OperationReceipt {
        version: 1,
        request_id: ATTEMPT.into(),
        state: OperationState::Committed,
        receipt_id: Some(CHARACTER.into()),
        code: "BID_RESERVED".into(),
        charged_coins: 50,
        balance: 950,
    };
    let request = BazaarRequest {
        version: 1,
        request_id: ATTEMPT.into(),
        command: BazaarCommand::Bid {
            auction_id: CHARACTER.into(),
            expected_revision: 1,
            coins: 50,
        },
    };
    let body = serde_json::to_string(&valid)?;
    let mut pending = valid.clone();
    pending.state = OperationState::Pending;
    pending.receipt_id = None;
    pending.charged_coins = 0;
    let missing_nullable = serde_json::to_string(&pending)?.replace("\"receipt_id\":null,", "");
    for invalid in [
        body.replace("\"version\":1", "\"version\":2"),
        body.replace(ATTEMPT, WORLD),
        body.replace("\"committed\"", "\"pending\""),
        body.replacen('{', "{\"unknown\":0,", 1),
        body.replace(&format!("\"{}\"", CHARACTER), "null"),
        missing_nullable,
    ] {
        let server = spawn_server(
            &runtime,
            vec![(200, "", token_body.clone()), (200, "", invalid)],
        )?;
        let client = client_for(&server)?;
        runtime.block_on(async {
            let token = client
                .exchange_code("code", "pkce", "redirect", CancellationToken::new())
                .await?;
            assert!(
                client
                    .bazaar_command(&token, &request, CancellationToken::new())
                    .await
                    .is_err()
            );
            Ok::<_, Box<dyn std::error::Error>>(())
        })?;
    }
    Ok(())
}

#[test]
fn bazaar_wire_pages_and_requests_are_bounded() -> TestResult {
    use crate::native_login::account_services::WireReply;
    let mut page = BazaarPage {
        version: 1,
        view: AuctionView::All,
        auctions: vec![],
        owned_characters: vec![],
        worlds: vec![],
        balance: 0,
        next_cursor: None,
        recent_operations: vec![],
    };
    assert!(page.valid());
    let missing_nullable = serde_json::to_string(&page)?.replace("\"next_cursor\":null,", "");
    assert!(serde_json::from_str::<BazaarPage>(&missing_nullable).is_err());
    page.worlds = vec![
        TransferWorld {
            id: WORLD.into(),
            name: "World".into()
        };
        2
    ];
    assert!(!page.valid());
    page.worlds.clear();
    page.next_cursor = Some("x".repeat(257));
    assert!(!page.valid());
    page.next_cursor = None;
    page.owned_characters = vec![
        OwnedCharacter {
            id: CHARACTER.into(),
            name: "Ari".into(),
            world_id: WORLD.into()
        };
        101
    ];
    assert!(!page.valid());
    for coins in [0, MAX_COINS + 1] {
        assert!(
            !BazaarCommand::Bid {
                auction_id: CHARACTER.into(),
                expected_revision: 1,
                coins
            }
            .valid()
        );
    }
    assert!(
        !BazaarCommand::Sell {
            character_id: CHARACTER.into(),
            minimum_bid: 1,
            duration_hours: 169
        }
        .valid()
    );
    let runtime = tokio::runtime::Runtime::new()?;
    let server = spawn_server(
        &runtime,
        vec![
            (
                200,
                "",
                r#"{"token_type":"Bearer","access_token":"services"}"#.into(),
            ),
            (200, "", "x".repeat(65_537)),
        ],
    )?;
    let client = client_for(&server)?;
    runtime.block_on(async {
        let token = client
            .exchange_code("code", "pkce", "redirect", CancellationToken::new())
            .await?;
        assert_eq!(
            client
                .bazaar_page(&token, AuctionView::All, None, CancellationToken::new())
                .await
                .err(),
            Some(NativeLoginError::Platform(
                PlatformClientError::ResponseTooLarge
            ))
        );
        Ok::<_, Box<dyn std::error::Error>>(())
    })
}
