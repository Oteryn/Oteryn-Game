use super::*;
use serde::de::DeserializeOwned;

pub(super) trait WireReply {
    fn valid(&self) -> bool;
}
pub(super) fn valid_id(value: &str) -> bool {
    parse_uuid_v7(value).is_some()
}
pub(super) fn valid_text(value: &str, max: usize) -> bool {
    !value.is_empty() && value.len() <= max && !value.chars().any(char::is_control)
}
pub(super) fn unique_ids<'a>(values: impl IntoIterator<Item = &'a str>) -> bool {
    let mut seen = std::collections::BTreeSet::new();
    values.into_iter().all(|id| seen.insert(id))
}
pub(super) fn required_option<'de, D: serde::Deserializer<'de>, T: serde::Deserialize<'de>>(
    decoder: D,
) -> Result<Option<T>, D::Error> {
    <Option<T> as serde::Deserialize>::deserialize(decoder)
}

impl NativeLoginClient {
    /// Opt-in scope: normal gameplay authorization remains game:ticket only.
    pub fn account_services_authorization_url(
        &self,
        redirect: &str,
        state: &str,
        pkce: &str,
    ) -> Result<Url, PlatformClientError> {
        let mut url = self.authorization_url(redirect, state, pkce)?;
        let pairs: Vec<_> = url
            .query_pairs()
            .map(|(k, v)| (k.into_owned(), v.into_owned()))
            .collect();
        url.query_pairs_mut().clear();
        for (key, value) in pairs {
            url.query_pairs_mut().append_pair(
                &key,
                if key == "scope" {
                    "game:account-services"
                } else {
                    &value
                },
            );
        }
        Ok(url)
    }
    pub(super) async fn service_request<T: DeserializeOwned + WireReply>(
        &self,
        token: &AccessToken,
        method: reqwest::Method,
        url: Url,
        body: Option<serde_json::Value>,
        cancel: CancellationToken,
    ) -> Result<T, NativeLoginError> {
        run(cancel, async {
            let mut request = self
                .client
                .request(method, url)
                .bearer_auth(token.0.expose());
            if let Some(body) = body {
                let bytes =
                    serde_json::to_vec(&body).map_err(|_| PlatformClientError::InvalidPayload)?;
                if bytes.len() > 8192 {
                    return Err(PlatformClientError::InvalidPayload.into());
                }
                request = request.header(CONTENT_TYPE, "application/json").body(bytes);
            }
            let response = request
                .send()
                .await
                .map_err(|_| PlatformClientError::Request)?;
            if response.status() != StatusCode::OK {
                return Err(PlatformClientError::HttpStatus(response.status().as_u16()).into());
            }
            let bytes = read_bounded(response, 65_536).await?;
            let decoded: T =
                serde_json::from_slice(&bytes).map_err(|_| PlatformClientError::InvalidPayload)?;
            if !decoded.valid() {
                return Err(PlatformClientError::InvalidPayload.into());
            }
            Ok(decoded)
        })
        .await
    }
}
