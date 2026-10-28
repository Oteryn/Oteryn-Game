use super::*;

pub(super) async fn authorize(
    client: &NativeLoginClient,
    config: &NativeLoginConfig,
    cancellation: CancellationToken,
) -> Result<NativeTicket, GameplayEntryError> {
    let mut entropy = [0_u8; 32];
    secure_random(&mut entropy).map_err(retry_login)?;
    let pkce = PkceMaterial::from_entropy(&entropy).map_err(retry_login)?;
    let mut nonce = [0_u8; 16];
    secure_random(&mut nonce).map_err(retry_login)?;
    let state = StateNonce::new(
        nonce
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>(),
    )
    .map_err(retry_login)?;
    let redirect = LoopbackRedirect::bind().await.map_err(retry_login)?;
    let redirect_uri = redirect.redirect_uri();
    let url = client
        .authorization_url(&redirect_uri, state.as_str(), pkce.challenge())
        .map_err(|_error| GameplayEntryError::InvalidConfiguration)?;
    if !(config.open_browser)(url.as_str()) {
        return Err(rejected(PublicClass::RetryLogin));
    }
    let flow = IdentityFlow::new(state, config.callback_timeout);
    let code = redirect
        .await_code(&flow, cancellation.clone())
        .await
        .map_err(retry_login)?;
    let token = client
        .exchange_code(
            code.expose().expose(),
            pkce.verifier().expose(),
            &redirect_uri,
            cancellation.clone(),
        )
        .await
        .map_err(login_rejected)?;
    client
        .issue_native_ticket(token, cancellation)
        .await
        .map_err(login_rejected)
}
