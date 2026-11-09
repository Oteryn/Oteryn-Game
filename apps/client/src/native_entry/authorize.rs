use super::*;

pub(super) async fn access_token(
    client: &NativeLoginClient,
    config: &NativeLoginConfig,
    cancellation: CancellationToken,
    services: bool,
) -> Result<oteryn_platform_client::native_login::AccessToken, GameplayEntryError> {
    if cancellation.is_cancelled() {
        return Err(rejected(PublicClass::RetryLogin));
    }
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
    let url = if services {
        client.account_services_authorization_url(&redirect_uri, state.as_str(), pkce.challenge())
    } else {
        client.authorization_url(&redirect_uri, state.as_str(), pkce.challenge())
    }
    .map_err(|_| GameplayEntryError::InvalidConfiguration)?;
    if !(config.open_browser)(url.as_str()) {
        return Err(rejected(PublicClass::RetryLogin));
    }
    let flow = IdentityFlow::new(state, config.callback_timeout);
    let code = redirect
        .await_code(&flow, cancellation.clone())
        .await
        .map_err(retry_login)?;
    super::challenge::exchange(
        client,
        config,
        code.expose().expose(),
        pkce.verifier().expose(),
        &redirect_uri,
        cancellation.clone(),
    )
    .await
}

pub(super) async fn authorize(
    client: &NativeLoginClient,
    config: &NativeLoginConfig,
    cancellation: CancellationToken,
) -> Result<(NativeTicket, String, Option<String>, String, bool), GameplayEntryError> {
    let mut token = access_token(client, config, cancellation.clone(), false).await?;
    let platform = PlatformClientConfig::new(&config.platform_url)
        .map_err(|_| GameplayEntryError::InvalidConfiguration)?;
    let directory_client = oteryn_platform_client::PlatformClient::new(platform)
        .map_err(|_| GameplayEntryError::InvalidConfiguration)?;
    let (character_id, channel_id, character_name, training_requested) = loop {
        let characters = client
            .owner_characters(&token, cancellation.clone())
            .await
            .map_err(login_rejected)?;
        let directory = directory_client
            .fetch_directory(cancellation.clone())
            .await
            .map_err(retry_login)?;
        let rows = crate::character_selection::join(characters, &directory);
        use crate::character_selection::SelectionAction;
        let action = if let Some(selection) = config.training_queue.next() {
            SelectionAction::Train {
                character_id: selection.character_id,
                channel_id: selection.channel_id,
            }
        } else {
            config.training_queue.clear();
            let selector = config.select_character.clone();
            let preselect = config.character_id.clone();
            let cancel = cancellation.clone();
            let shown = rows.clone();
            tokio::task::spawn_blocking(move || selector(&shown, &preselect, cancel))
                .await
                .map_err(retry_login)?
        };
        let action = match action {
            SelectionAction::TrainMany { characters } => {
                let first = config
                    .training_queue
                    .start(&rows, characters)
                    .ok_or_else(|| rejected(PublicClass::RetryLogin))?;
                SelectionAction::Train {
                    character_id: first.character_id,
                    channel_id: first.channel_id,
                }
            }
            action => action,
        };
        let training_requested = matches!(&action, SelectionAction::Train { .. });
        match action {
            SelectionAction::Play {
                character_id,
                channel_id,
            }
            | SelectionAction::Train {
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
                break (
                    character_id,
                    channel_id,
                    selected.character.name.clone(),
                    training_requested,
                );
            }
            SelectionAction::TrainMany { .. } => return Err(rejected(PublicClass::RetryLogin)),
            SelectionAction::Create => {
                let url = format!(
                    "{}/account/characters/create",
                    config.platform_url.trim_end_matches('/')
                );
                if !(config.open_browser)(&url) {
                    return Err(rejected(PublicClass::RetryLogin));
                }
            }
            SelectionAction::AccountServices => {
                let services = access_token(client, config, cancellation.clone(), true).await;
                match services {
                    Ok(services) => {
                        crate::account_services::run(
                            client,
                            &services,
                            config,
                            cancellation.clone(),
                        )
                        .await?
                    }
                    Err(_) => {
                        let mut view = crate::account_services::View::default();
                        view.feedback = "Account services sign-in was not completed. Return to character selection to try again.".into();
                        let _ = crate::account_services::ask(
                            view,
                            &config.account_services,
                            cancellation.clone(),
                        )
                        .await?;
                    }
                }
                // Visiting account services may outlive the normal token; acquire fresh game-only authority.
                token = access_token(client, config, cancellation.clone(), false).await?;
            }
            SelectionAction::Refresh => {}
            SelectionAction::Cancel => {
                return Err(rejected(PublicClass::RetryLogin));
            }
        }
    };
    let ticket = client
        .issue_native_ticket(token, cancellation)
        .await
        .map_err(login_rejected)?;
    Ok((
        ticket,
        character_id,
        channel_id,
        character_name,
        training_requested,
    ))
}
