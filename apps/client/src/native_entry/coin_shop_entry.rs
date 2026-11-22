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
    let token = authorize::access_token(&client, config, cancel.clone(), true).await?;
    let mut view = crate::coin_shop::View::new(Some(character_id.to_owned()));
    crate::coin_shop::run(
        &client,
        &token,
        &config.coin_shop,
        &config.open_browser,
        &mut view,
        cancel,
    )
    .await
}
