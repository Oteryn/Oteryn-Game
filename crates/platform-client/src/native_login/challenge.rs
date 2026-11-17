//! Oteryn-only OAuth challenge extension. The Platform verifier remains authoritative.

use super::*;

/// A bounded challenge response, never an arbitrary browser URL from the network.
#[derive(Debug)]
pub struct LoginChallenge {
    id: String,
    usable_until: Instant,
}

impl LoginChallenge {
    #[must_use]
    pub fn remaining(&self) -> Duration {
        self.usable_until.saturating_duration_since(Instant::now())
    }
}

#[derive(Debug)]
pub enum TokenExchange {
    Token(AccessToken),
    Challenge(LoginChallenge),
}

/// Opaque, short-lived proof; only Platform can verify it. Never persisted or logged.
#[derive(Debug)]
pub struct ChallengeProof {
    challenge: LoginChallenge,
    token: SecretText,
}

impl ChallengeProof {
    pub fn new(challenge: LoginChallenge, token: &str) -> Result<Self, NativeLoginError> {
        // Unescaped query-safe proof bytes keep 2048 bytes plus state within the callback budget.
        if !is_visible_ascii(token, 2048)
            || !token
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
            || challenge.remaining().is_zero()
        {
            return Err(NativeLoginError::TokenRefused);
        }
        Ok(Self {
            challenge,
            token: SecretText(token.to_owned()),
        })
    }
}

impl NativeLoginClient {
    /// System-browser challenge page on the configured Oteryn Platform origin only.
    pub fn challenge_url(
        &self,
        challenge: &LoginChallenge,
        redirect_uri: &str,
        state: &str,
    ) -> Result<Url, NativeLoginError> {
        if challenge.remaining().is_zero() {
            return Err(NativeLoginError::TokenRefused);
        }
        let mut url = self
            .platform
            .join("oauth/challenge")
            .map_err(|_| PlatformClientError::InvalidBaseUrl)?;
        url.query_pairs_mut()
            .append_pair("client_id", &self.oauth_client_id)
            .append_pair("challenge_id", &challenge.id)
            .append_pair("redirect_uri", redirect_uri)
            .append_pair("state", state);
        Ok(url)
    }

    /// OAuth exchange with at most one caller-controlled challenge retry. A challenge response
    /// must not consume the authorization code; Platform binds its proof to code + PKCE.
    pub async fn exchange_code_challenged(
        &self,
        code: &str,
        code_verifier: &str,
        redirect_uri: &str,
        proof: Option<&ChallengeProof>,
        cancellation: CancellationToken,
    ) -> Result<TokenExchange, NativeLoginError> {
        if proof.is_some_and(|proof| proof.challenge.remaining().is_zero()) {
            return Err(NativeLoginError::TokenRefused);
        }
        let endpoint = self
            .platform
            .join("oauth/token")
            .map_err(|_| PlatformClientError::InvalidBaseUrl)?;
        let request = async {
            let mut fields = vec![
                ("grant_type", "authorization_code"),
                ("client_id", self.oauth_client_id.as_str()),
                ("redirect_uri", redirect_uri),
                ("code", code),
                ("code_verifier", code_verifier),
            ];
            if let Some(proof) = proof {
                fields.push(("challenge_id", &proof.challenge.id));
                fields.push(("challenge_token", proof.token.expose()));
            }
            let response = self
                .client
                .post(endpoint)
                .form(&fields)
                .send()
                .await
                .map_err(|_| PlatformClientError::Request)?;
            let status = response.status();
            let received = Instant::now();
            if !status.is_success() && status != StatusCode::FORBIDDEN {
                return Err(NativeLoginError::TokenRefused);
            }
            let body = read_bounded(response, MAX_OAUTH_RESPONSE_BYTES).await?;
            if status == StatusCode::FORBIDDEN {
                // No unbounded challenge loop or replacement of a previously bound challenge.
                if proof.is_some() {
                    return Err(NativeLoginError::TokenRefused);
                }
                return parse_challenge(&body, received).map(TokenExchange::Challenge);
            }
            #[derive(Deserialize)]
            struct TokenResponse {
                token_type: String,
                access_token: String,
            }
            let token: TokenResponse =
                serde_json::from_slice(&body).map_err(|_| PlatformClientError::InvalidPayload)?;
            if !token.token_type.eq_ignore_ascii_case("bearer")
                || !is_visible_ascii(&token.access_token, MAX_ACCESS_TOKEN_BYTES)
            {
                return Err(PlatformClientError::InvalidPayload.into());
            }
            Ok(TokenExchange::Token(AccessToken(SecretText(
                token.access_token,
            ))))
        };
        run(cancellation, request).await
    }
}

fn parse_challenge(body: &[u8], received: Instant) -> Result<LoginChallenge, NativeLoginError> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Response {
        error: String,
        challenge_id: String,
        expires_in: u64,
    }
    let response: Response =
        serde_json::from_slice(body).map_err(|_| PlatformClientError::InvalidPayload)?;
    if response.error != "oteryn_challenge_required"
        || parse_uuid_v7(&response.challenge_id).is_none()
        || !(1..=300).contains(&response.expires_in)
    {
        return Err(PlatformClientError::InvalidPayload.into());
    }
    Ok(LoginChallenge {
        id: response.challenge_id,
        usable_until: received + Duration::from_secs(response.expires_in),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proof_encoding_fits_the_bounded_loopback_callback() {
        let fresh = || LoginChallenge {
            id: "01890a5d-ac96-774b-bcce-b302099a8057".into(),
            usable_until: Instant::now() + Duration::from_secs(60),
        };
        // These require percent-encoding and could triple a maximum-sized proof's query size.
        for escaped in ["+", "/", "=", "%", "&", "?", "#"] {
            assert!(ChallengeProof::new(fresh(), &escaped.repeat(2048)).is_err());
        }
        assert!(ChallengeProof::new(fresh(), &"a-Z_0.1".repeat(292)).is_ok());
        assert!(ChallengeProof::new(fresh(), &"a".repeat(2048)).is_ok());
        assert!(ChallengeProof::new(fresh(), &"a".repeat(2049)).is_err());
    }

    #[test]
    fn expired_or_invalid_proof_never_reaches_transport() -> Result<(), Box<dyn std::error::Error>>
    {
        let expired = || LoginChallenge {
            id: "01890a5d-ac96-774b-bcce-b302099a8057".into(),
            usable_until: Instant::now(),
        };
        assert!(ChallengeProof::new(expired(), "proof").is_err());
        for token in ["", "line\nbreak", "nonascii-ą"] {
            let challenge = LoginChallenge {
                usable_until: Instant::now() + Duration::from_secs(60),
                ..expired()
            };
            assert!(ChallengeProof::new(challenge, token).is_err());
        }
        let config = PlatformClientConfig::new("https://platform.example.test/")?;
        let client = NativeLoginClient::new(&config, &config, "native")?;
        assert!(
            client
                .challenge_url(&expired(), "redirect", "state")
                .is_err()
        );
        let proof = ChallengeProof {
            challenge: expired(),
            token: SecretText("secret".into()),
        };
        let runtime = tokio::runtime::Runtime::new()?;
        assert_eq!(
            runtime
                .block_on(client.exchange_code_challenged(
                    "code",
                    "pkce",
                    "redirect",
                    Some(&proof),
                    CancellationToken::new()
                ))
                .err(),
            Some(NativeLoginError::TokenRefused)
        );
        let cancelled = CancellationToken::new();
        cancelled.cancel();
        assert_eq!(
            runtime
                .block_on(
                    client.exchange_code_challenged("code", "pkce", "redirect", None, cancelled)
                )
                .err(),
            Some(NativeLoginError::Platform(PlatformClientError::Cancelled))
        );
        Ok(())
    }
}
