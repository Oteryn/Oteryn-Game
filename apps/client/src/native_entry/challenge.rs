//! The Oteryn challenge opens in the system browser and returns through a fresh loopback
//! listener. The identity owner verifies the proof; Game never treats it as an access token.
use super::*;
use oteryn_platform_client::native_login::{AccessToken, ChallengeProof, TokenExchange};

#[cfg(test)]
mod tests;

pub(super) async fn exchange(
    client: &NativeLoginClient,
    config: &NativeLoginConfig,
    code: &str,
    verifier: &str,
    redirect_uri: &str,
    cancellation: CancellationToken,
) -> Result<AccessToken, GameplayEntryError> {
    let result = client
        .exchange_code_challenged(code, verifier, redirect_uri, None, cancellation.clone())
        .await
        .map_err(login_rejected)?;
    let challenge = match result {
        TokenExchange::Token(token) => return Ok(token),
        TokenExchange::Challenge(challenge) => challenge,
    };
    let mut nonce = [0_u8; 32];
    secure_random(&mut nonce).map_err(retry_login)?;
    let state = StateNonce::new(
        nonce
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>(),
    )
    .map_err(retry_login)?;
    let redirect = LoopbackRedirect::bind().await.map_err(retry_login)?;
    let url = client
        .challenge_url(&challenge, &redirect.redirect_uri(), state.as_str())
        .map_err(login_rejected)?;
    if !(config.open_browser)(url.as_str()) {
        return Err(rejected(PublicClass::RetryLogin));
    }
    // The signed opaque proof is carried in the existing callback's `code` slot, with an
    // independent CSRF state. Neither the OAuth code nor PKCE verifier goes into this URL.
    let flow = IdentityFlow::new(state, config.callback_timeout.min(challenge.remaining()));
    let returned = redirect
        .await_code(&flow, cancellation.clone())
        .await
        .map_err(retry_login)?;
    let proof =
        ChallengeProof::new(challenge, returned.expose().expose()).map_err(login_rejected)?;
    match client
        .exchange_code_challenged(code, verifier, redirect_uri, Some(&proof), cancellation)
        .await
        .map_err(login_rejected)?
    {
        TokenExchange::Token(token) => Ok(token),
        TokenExchange::Challenge(_) => Err(rejected(PublicClass::RetryLogin)),
    }
}
