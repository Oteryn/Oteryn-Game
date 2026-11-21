use super::*;

const CHALLENGE: &str = r#"{"error":"oteryn_challenge_required","challenge_id":"01890a5d-ac96-774b-bcce-b302099a8057","expires_in":120}"#;

#[test]
fn production_routes_machine_calls_to_api_and_authorize_to_web() -> TestResult {
    struct RejectDns;
    impl reqwest::dns::Resolve for RejectDns {
        fn resolve(&self, _: reqwest::dns::Name) -> reqwest::dns::Resolving {
            Box::pin(std::future::ready(Err(std::io::Error::other(
                "production routing test disables DNS",
            )
            .into())))
        }
    }

    let runtime = tokio::runtime::Runtime::new()?;
    let fixture = spawn_server(&runtime, vec![(403, "", CHALLENGE.into())])?;
    runtime.block_on(async {
        let web = Url::parse("https://oteryn.com/")?;
        let config = PlatformClientConfig::new(web.as_str())?;
        let mut directory = crate::PlatformClient::new(config.clone())?;
        let mut native = NativeLoginClient::new(&config, &config, "native-test")?;

        let captured = Arc::new(Mutex::new(Vec::new()));
        let requests = Arc::clone(&captured);
        // Capture reqwest's final request URI on transport failure, without contacting Platform.
        let transport = Client::builder()
            .no_proxy()
            .dns_resolver(Arc::new(RejectDns))
            .retry(
                reqwest::retry::for_host("api.oteryn.com").classify_fn(move |attempt| {
                    if let Ok(mut requests) = requests.lock() {
                        requests.push(attempt.uri().to_string());
                    }
                    attempt.success()
                }),
            )
            .build()?;
        directory.client = transport.clone();
        native.client = transport;

        let redirect = "http://127.0.0.1:1234/callback";
        let cancel = CancellationToken::new();
        let token = || AccessToken(SecretText("access".into()));
        assert!(directory.fetch_directory(cancel.clone()).await.is_err());
        assert!(
            native
                .exchange_code("code", "pkce", redirect, cancel.clone())
                .await
                .is_err()
        );
        assert!(
            native
                .issue_native_ticket(token(), cancel.clone())
                .await
                .is_err()
        );
        assert!(
            native
                .premium_status(&token(), cancel.clone())
                .await
                .is_err()
        );
        assert!(
            native
                .coin_catalogue(&token(), None, cancel.clone())
                .await
                .is_err()
        );
        assert!(
            native
                .bazaar_page(
                    &token(),
                    crate::native_login::bazaar::AuctionView::All,
                    None,
                    cancel.clone()
                )
                .await
                .is_err()
        );
        assert_eq!(
            captured
                .lock()
                .map_err(|_| "request capture lock")?
                .as_slice(),
            [
                "https://api.oteryn.com/v1/client/directory",
                "https://api.oteryn.com/oauth/token",
                "https://api.oteryn.com/api/v1/game-auth/tickets",
                "https://api.oteryn.com/api/v1/native-account/premium",
                "https://api.oteryn.com/api/v1/native-account/coin-purchases/catalogue",
                "https://api.oteryn.com/api/v1/native-account/bazaar/auctions?view=all&limit=50",
            ]
        );

        let authorize = native.authorization_url(redirect, "state", "pkce")?;
        assert_eq!(authorize.origin(), web.origin());
        assert_eq!(authorize.path(), "/oauth/authorize");
        assert!(
            authorize
                .query_pairs()
                .any(|(key, value)| key == "state" && value == "state")
        );

        let TokenExchange::Challenge(challenge) = client_for(&fixture)?
            .exchange_code_challenged("code", "pkce", redirect, None, cancel)
            .await?
        else {
            return Err("expected challenge".into());
        };
        let challenge_url = native.challenge_url(&challenge, redirect, "state")?;
        assert_eq!(challenge_url.origin(), web.origin());
        assert_eq!(challenge_url.path(), "/oauth/challenge");

        let checkout_path = format!("/native-account/coin-checkout/{ATTEMPT}");
        for (origin, valid) in [
            ("https://oteryn.com/", true),
            ("https://api.oteryn.com/", false),
        ] {
            let url = Url::parse(origin)?.join(&checkout_path)?;
            let link: coin_purchase::CheckoutLink =
                serde_json::from_value(serde_json::json!(url.as_str()))?;
            assert_eq!(native.valid_coin_checkout(&link, ATTEMPT), valid);
        }
        Ok(())
    })
}

#[test]
fn malformed_challenges_and_repeated_challenges_fail_closed() -> TestResult {
    let runtime = tokio::runtime::Runtime::new()?;
    for body in [
        CHALLENGE.replace("120", "0"),
        CHALLENGE.replace("120", "301"),
        CHALLENGE.replace("120", "-1"),
        CHALLENGE.replace("01890a5d", "01890A5D"),
        CHALLENGE.replace("oteryn_challenge_required", "invalid_grant"),
        CHALLENGE.replace("{", r#"{"url":"https://untrusted.invalid/","#),
        CHALLENGE.replace("{", r#"{"expires_in":1,"#),
    ] {
        let server = spawn_server(&runtime, vec![(403, "", body)])?;
        let result = runtime.block_on(client_for(&server)?.exchange_code_challenged(
            "code",
            "pkce",
            "redirect",
            None,
            CancellationToken::new(),
        ));
        assert!(result.is_err());
    }
    let server = spawn_server(
        &runtime,
        vec![(403, "", CHALLENGE.into()), (403, "", CHALLENGE.into())],
    )?;
    let client = client_for(&server)?;
    runtime.block_on(async {
        let TokenExchange::Challenge(challenge) = client
            .exchange_code_challenged("code", "pkce", "redirect", None, CancellationToken::new())
            .await?
        else {
            return Err("expected challenge".into());
        };
        let proof = ChallengeProof::new(challenge, "proof")?;
        assert_eq!(
            client
                .exchange_code_challenged(
                    "code",
                    "pkce",
                    "redirect",
                    Some(&proof),
                    CancellationToken::new(),
                )
                .await
                .err(),
            Some(NativeLoginError::TokenRefused)
        );
        Ok::<_, Box<dyn std::error::Error>>(())
    })
}

#[test]
fn challenge_round_trip_resubmits_bound_proof_and_redacts_token()
-> Result<(), Box<dyn std::error::Error>> {
    let runtime = tokio::runtime::Runtime::new()?;
    let server = spawn_server(&runtime, vec![
        (403, "", r#"{"error":"oteryn_challenge_required","challenge_id":"01890a5d-ac96-774b-bcce-b302099a8057","expires_in":120}"#.into()),
        (200, "", r#"{"token_type":"Bearer","access_token":"access"}"#.into()),
    ])?;
    let client = client_for(&server)?;
    runtime.block_on(async {
        let result = client
            .exchange_code_challenged(
                "code",
                "verifier",
                "http://127.0.0.1:1234/callback",
                None,
                CancellationToken::new(),
            )
            .await?;
        let TokenExchange::Challenge(challenge) = result else {
            return Err("challenge expected".into());
        };
        let url = client.challenge_url(&challenge, "http://127.0.0.1:1235/callback", "state")?;
        assert_eq!(url.path(), "/oauth/challenge");
        assert_eq!(
            url.origin().ascii_serialization(),
            server.base.trim_end_matches('/')
        );
        let proof = ChallengeProof::new(challenge, "proof-SECRET")?;
        assert!(!format!("{proof:?}").contains("proof-SECRET"));
        let result = client
            .exchange_code_challenged(
                "code",
                "verifier",
                "http://127.0.0.1:1234/callback",
                Some(&proof),
                CancellationToken::new(),
            )
            .await?;
        assert!(matches!(result, TokenExchange::Token(_)));
        Ok::<_, Box<dyn std::error::Error>>(())
    })?;
    let requests = server.requests.lock().map_err(|_| "log lock")?;
    assert_eq!(requests.len(), 2);
    assert!(!request_body(&requests[0]).contains("challenge_token"));
    assert!(request_body(&requests[1]).contains("challenge_token=proof-SECRET"));
    assert!(
        request_body(&requests[1]).contains("challenge_id=01890a5d-ac96-774b-bcce-b302099a8057")
    );
    Ok(())
}
