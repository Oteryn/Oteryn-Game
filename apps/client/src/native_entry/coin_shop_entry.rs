use super::*;

#[cfg_attr(not(test), allow(dead_code))]
pub(crate) async fn open(
    config: &NativeLoginConfig,
    character_id: &str,
    cancel: CancellationToken,
) -> Result<(), GameplayEntryError> {
    let platform = PlatformClientConfig::new(&config.platform_url)
        .map_err(|_| GameplayEntryError::InvalidConfiguration)?;
    let gateway = PlatformClientConfig::new(&config.gateway_url)
        .map_err(|_| GameplayEntryError::InvalidConfiguration)?;
    let client = login_client(
        &platform,
        &gateway,
        &config.oauth_client_id,
        config.retry_unit,
    )?;
    let mut session = config.device_sessions.commerce_session(
        &config.platform_url,
        &config.oauth_client_id,
        Some(character_id.to_owned()),
    );
    let token = authorize::access_token(&client, config, cancel.clone(), true).await?;
    crate::coin_shop::run(
        &client,
        &token,
        &config.coin_shop,
        &config.open_browser,
        &mut session.coin,
        cancel,
    )
    .await
}
