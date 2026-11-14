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
    let mut device = config
        .device_sessions
        .begin(
            client,
            session_options(config.preference_path.as_deref()),
            cancellation.clone(),
        )
        .await
        .map_err(login_rejected)?;
    let mut token = if device.as_ref().is_some_and(|attempt| attempt.has_session()) {
        None
    } else {
        Some(access_token(client, config, cancellation.clone(), false).await?)
    };
    if let Some(bearer) = token.take() {
        token = Some(
            enroll_or_fresh_browser(client, config, &mut device, bearer, cancellation.clone())
                .await?,
        );
    }
    let mut quick = true;
    let platform = PlatformClientConfig::new(&config.platform_url)
        .map_err(|_| GameplayEntryError::InvalidConfiguration)?;
    let directory_client = oteryn_platform_client::PlatformClient::new(platform)
        .map_err(|_| GameplayEntryError::InvalidConfiguration)?;
    let (character_id, channel_id, character_name, training_requested) = loop {
        let characters = if let Some(token) = &token {
            client
                .owner_characters(token, cancellation.clone())
                .await
                .map_err(login_rejected)?
        } else if let Some(attempt) = &mut device {
            match attempt.characters(client, cancellation.clone()).await {
                Ok(characters) => characters,
                Err(_) => {
                    let fresh = access_token(client, config, cancellation.clone(), false).await?;
                    let fresh = if attempt
                        .enroll(client, &fresh, cancellation.clone())
                        .await
                        .is_err()
                    {
                        access_token(client, config, cancellation.clone(), false).await?
                    } else {
                        fresh
                    };
                    let characters = client
                        .owner_characters(&fresh, cancellation.clone())
                        .await
                        .map_err(login_rejected)?;
                    token = Some(fresh);
                    characters
                }
            }
        } else {
            return Err(rejected(PublicClass::RetryLogin));
        };
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
        } else if quick
            && let Some(selection) = device
                .as_ref()
                .and_then(|attempt| attempt.quick_selection(&rows))
        {
            selection
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
        quick = false;
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
                        view.feedback = tr!("Account services sign-in was not completed. Return to character selection to try again.").into();
                        let _ = crate::account_services::ask(
                            view,
                            &config.account_services,
                            cancellation.clone(),
                        )
                        .await?;
                    }
                }
                // Visiting account services may outlive the normal token; acquire fresh game-only authority.
                let fresh = access_token(client, config, cancellation.clone(), false).await?;
                let fresh = enroll_or_fresh_browser(
                    client,
                    config,
                    &mut device,
                    fresh,
                    cancellation.clone(),
                )
                .await?;
                token = Some(fresh);
            }
            SelectionAction::Refresh => {}
            SelectionAction::Cancel => {
                if let Some(attempt) = &mut device {
                    attempt.sign_out(client).await;
                }
                return Err(rejected(PublicClass::RetryLogin));
            }
        }
    };
    let ticket = if let Some(token) = token {
        client
            .issue_native_ticket(token, cancellation)
            .await
            .map_err(login_rejected)?
    } else if let Some(attempt) = &mut device {
        attempt
            .ticket(client, cancellation)
            .await
            .map_err(login_rejected)?
    } else {
        return Err(rejected(PublicClass::RetryLogin));
    };
    if let Some(attempt) = &device {
        attempt.remember_selection(&character_id, channel_id.as_deref());
    }
    Ok((
        ticket,
        character_id,
        channel_id,
        character_name,
        training_requested,
    ))
}

async fn enroll_or_fresh_browser(
    client: &NativeLoginClient,
    config: &NativeLoginConfig,
    device: &mut Option<crate::device_login::DeviceAttempt>,
    token: oteryn_platform_client::native_login::AccessToken,
    cancel: CancellationToken,
) -> Result<oteryn_platform_client::native_login::AccessToken, GameplayEntryError> {
    if let Some(attempt) = device
        && attempt
            .enroll(client, &token, cancel.clone())
            .await
            .is_err()
    {
        // Never reuse the OAuth bearer of an uncertain enrollment. Do not loop enrollment.
        return access_token(client, config, cancel, false).await;
    }
    Ok(token)
}

fn session_options(preference_path: Option<&std::path::Path>) -> crate::play::MiscellaneousPreferences {
    preference_path
        .and_then(|path| {
            crate::play::MiscellaneousPreferences::read(&path.with_extension("session-options.json")).ok()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod session_options_tests {
    use super::session_options;
    use crate::play::{MiscellaneousPreferences, SettingsState};

    #[test]
    fn admission_reads_session_options_from_selected_preference_path()
    -> Result<(), Box<dyn std::error::Error>> {
        let folder = std::env::temp_dir().join(format!("admission-options-{}", std::process::id()));
        std::fs::create_dir_all(&folder)?;
        let result = (|| -> Result<(), Box<dyn std::error::Error>> {
            let first = folder.join("isolated.json");
            let second = folder.join("other.json");
            let global = MiscellaneousPreferences::load();
            let flags = MiscellaneousPreferences {
                stay_logged_in: !global.stay_logged_in,
                remember_device: !global.remember_device,
                quick_login: !global.quick_login,
                confirm_logout: false,
                confirm_exit: false,
            };
            flags.save(&first.with_extension("session-options.json"))?;
            MiscellaneousPreferences::default().save(&second.with_extension("session-options.json"))?;
            assert_eq!(session_options(Some(&first)), flags);
            assert_eq!(session_options(Some(&first)), SettingsState::new(Some(first.clone())).preferences.miscellaneous);
            assert_eq!(session_options(Some(&second)), MiscellaneousPreferences::default());
            assert_eq!(session_options(None), MiscellaneousPreferences::default());
            assert_eq!(session_options(Some(&folder.join("missing.json"))), MiscellaneousPreferences::default());
            std::fs::write(first.with_extension("session-options.json"), b"invalid")?;
            assert_eq!(session_options(Some(&first)), MiscellaneousPreferences::default());
            Ok(())
        })();
        std::fs::remove_dir_all(folder)?;
        result
    }
}
