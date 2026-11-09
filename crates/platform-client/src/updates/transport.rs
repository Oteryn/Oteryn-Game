use super::UpdateError;
use async_trait::async_trait;
use futures::StreamExt;
use reqwest::Client;
use std::pin::Pin;
use tough::{Transport, TransportError, TransportErrorKind};
use url::Url;
#[derive(Debug, Clone)]
pub struct OriginTransport {
    client: Client,
    metadata_bytes: std::sync::Arc<std::sync::atomic::AtomicU64>,
}
impl OriginTransport {
    pub fn new() -> Result<Self, UpdateError> {
        let client = Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(std::time::Duration::from_secs(10))
            .timeout(std::time::Duration::from_secs(300))
            .build()
            .map_err(|_| UpdateError::Configuration)?;
        Ok(Self {
            client,
            metadata_bytes: Default::default(),
        })
    }
}
#[async_trait]
impl Transport for OriginTransport {
    async fn fetch(
        &self,
        mut url: Url,
    ) -> Result<
        Pin<Box<dyn futures::Stream<Item = Result<bytes::Bytes, TransportError>> + Send + Sync>>,
        TransportError,
    > {
        let (routed, metadata, policy) = route(url)?;
        url = routed;
        let response = self
            .client
            .get(url.clone())
            .send()
            .await
            .map_err(|_| TransportError::new(TransportErrorKind::Other, &url))?;
        if !response.status().is_success() {
            return Err(TransportError::new(
                if response.status() == 404 {
                    TransportErrorKind::FileNotFound
                } else {
                    TransportErrorKind::Other
                },
                &url,
            ));
        }
        let limit = if metadata {
            1_048_576
        } else if policy {
            262_144
        } else {
            u64::MAX
        };
        let mut total = 0_u64;
        let aggregate = self.metadata_bytes.clone();
        Ok(Box::pin(response.bytes_stream().map(move |chunk| {
            let chunk = chunk
                .map_err(|_| TransportError::new(TransportErrorKind::Other, "update origin"))?;
            total = total.saturating_add(chunk.len() as u64);
            if total > limit
                || (metadata
                    && aggregate
                        .fetch_add(chunk.len() as u64, std::sync::atomic::Ordering::Relaxed)
                        .saturating_add(chunk.len() as u64)
                        > 8 * 1_048_576)
            {
                return Err(TransportError::new(
                    TransportErrorKind::Other,
                    "update size",
                ));
            }
            Ok(chunk)
        })))
    }
}

// tough 0.18 prefixes the entire target path. The accepted policy URL keeps that
// spelling; package storage requires the hash immediately before the basename.
fn route(mut url: Url) -> Result<(Url, bool, bool), TransportError> {
    let fail = || TransportError::new(TransportErrorKind::UnsupportedUrlScheme, &url);
    if url.scheme() != "https"
        || url.host_str() != Some("account.oteryn.com")
        || url.port().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err(fail());
    }
    let parts: Vec<_> = url.path().split('/').collect();
    if parts.len() < 5 || parts[1] != "updates" || !matches!(parts[2], "stable" | "beta") {
        return Err(fail());
    }
    if parts[3] == "metadata" {
        return Ok((url, true, false));
    }
    if parts[3] != "targets" {
        return Err(fail());
    }
    let channel = parts[2].to_owned();
    let prefix = format!("/updates/{channel}/targets/");
    let relative = url.path().strip_prefix(&prefix).ok_or_else(fail)?;
    let (hash, target) = relative.split_once('.').ok_or_else(fail)?;
    if hash.len() != 64
        || !hash
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
        || !target.starts_with(&format!("channels/{channel}/"))
    {
        return Err(fail());
    }
    if target == format!("channels/{channel}/policy-v1.json") {
        return Ok((url, false, true));
    }
    let (parent, name) = target.rsplit_once('/').ok_or_else(fail)?;
    let path = format!("{prefix}{parent}/{hash}.{name}");
    url.set_path(&path);
    url.set_host(Some("downloads.oteryn.com"))
        .map_err(|_| TransportError::new(TransportErrorKind::Other, "origin"))?;
    Ok((url, false, false))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn accepted_origin_routes_policy_and_package_to_distinct_exact_urls()
    -> Result<(), Box<dyn std::error::Error>> {
        let hash = "a".repeat(64);
        let (policy, _, is_policy) = route(Url::parse(&format!(
            "https://account.oteryn.com/updates/stable/targets/{hash}.channels/stable/policy-v1.json"
        ))?)?;
        assert!(is_policy);
        assert_eq!(policy.host_str(), Some("account.oteryn.com"));
        let (package, _, _) = route(Url::parse(&format!(
            "https://account.oteryn.com/updates/stable/targets/{hash}.channels/stable/releases/rel-a/windows/x86_64/client.zip"
        ))?)?;
        assert_eq!(
            package.as_str(),
            format!(
                "https://downloads.oteryn.com/updates/stable/targets/channels/stable/releases/rel-a/windows/x86_64/{hash}.client.zip"
            )
        );
        assert!(
            route(Url::parse(
                "https://evil.invalid/updates/stable/metadata/timestamp.json"
            )?)
            .is_err()
        );
        assert!(route(Url::parse(&format!("https://account.oteryn.com/updates/stable/targets/{hash}.channels/beta/policy-v1.json"))?).is_err());
        Ok(())
    }
}
