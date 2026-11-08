use super::*;

pub(super) async fn authorize(
    client: &NativeLoginClient,
    config: &NativeLoginConfig,
    cancellation: CancellationToken,
) -> Result<(NativeTicket, String, Option<String>, String), GameplayEntryError> {
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
    let token = super::challenge::exchange(
        client,
        config,
        code.expose().expose(),
        pkce.verifier().expose(),
        &redirect_uri,
        cancellation.clone(),
    )
    .await?;
    let platform = PlatformClientConfig::new(&config.platform_url)
        .map_err(|_| GameplayEntryError::InvalidConfiguration)?;
    let directory_client = oteryn_platform_client::PlatformClient::new(platform)
        .map_err(|_| GameplayEntryError::InvalidConfiguration)?;
    let (character_id, channel_id, character_name) = loop {
        let characters = client
            .owner_characters(&token, cancellation.clone())
            .await
            .map_err(login_rejected)?;
        let directory = directory_client
            .fetch_directory(cancellation.clone())
            .await
            .map_err(retry_login)?;
        let rows = crate::character_selection::join(characters, &directory);
        let selector = config.select_character.clone();
        let preselect = config.character_id.clone();
        let cancel = cancellation.clone();
        let shown = rows.clone();
        let action = tokio::task::spawn_blocking(move || selector(&shown, &preselect, cancel))
            .await
            .map_err(retry_login)?;
        use crate::character_selection::SelectionAction;
        match action {
            SelectionAction::Play {
                character_id,
                channel_id,
            } => {
                let selected = rows.iter().find(|row| {
                    row.available
                        && row.character.character_id == character_id
                        && channel_id
                            .as_ref()
                            .is_none_or(|id| row.channels.iter().any(|channel| &channel.0 == id))
                });
                let selected = selected.ok_or_else(|| rejected(PublicClass::RetryLogin))?;
                break (character_id, channel_id, selected.character.name.clone());
            }
            SelectionAction::Create => {
                let url = format!(
                    "{}/account/characters/create",
                    config.platform_url.trim_end_matches('/')
                );
                if !(config.open_browser)(&url) {
                    return Err(rejected(PublicClass::RetryLogin));
                }
            }
            SelectionAction::Refresh => {}
            SelectionAction::Cancel => return Err(rejected(PublicClass::RetryLogin)),
        }
    };
    let ticket = client
        .issue_native_ticket(token, cancellation)
        .await
        .map_err(login_rejected)?;
    Ok((ticket, character_id, channel_id, character_name))
}
