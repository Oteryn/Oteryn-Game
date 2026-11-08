use super::*;

const CHALLENGE: &str = r#"{"error":"oteryn_challenge_required","challenge_id":"01890a5d-ac96-774b-bcce-b302099a8057","expires_in":120}"#;

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
