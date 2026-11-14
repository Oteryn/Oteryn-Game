//! Purpose-separated B1/B2 clients. No endpoint, product or credential can come from a body.
use super::*;
use crate::platform_streams::enforcement::transport::{
    Failure, READER_IDENTITY_VAR, RESULTS_IDENTITY_VAR,
};
use crate::platform_streams::{self, STREAMS_PLATFORM_CA_VAR, STREAMS_URL_VAR};
use crate::premium::client::{ClientConfigError, PremiumClientConfig, platform_http};
use std::{
    ffi::OsString,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
pub const AUTHORITY_ENABLED_VAR: &str = "OTERYN_PLATFORM_PROFILE_B_AUTHORITY_ENABLED";
pub const CHANGES_ENABLED_VAR: &str = "OTERYN_PLATFORM_PROFILE_B_CHANGES_ENABLED";
pub const AUTHORITY_IDENTITY_VAR: &str = "OTERYN_PLATFORM_PROFILE_B_AUTHORITY_CLIENT_IDENTITY_PEM";
pub const CHANGES_IDENTITY_VAR: &str = "OTERYN_PLATFORM_PROFILE_B_CHANGES_CLIENT_IDENTITY_PEM";
pub const PRODUCTS_VAR: &str = "OTERYN_PLATFORM_PROFILE_B_PRODUCTS";
pub const AUTHORITY_PATH: &str = "/internal/v1/products-entitlements/profile-b/authority/read";
pub const CHANGES_PATH: &str = "/internal/v1/products-entitlements/profile-b/changes/read";
pub const EXCHANGE_TIMEOUT: Duration = Duration::from_secs(3);
#[derive(Clone)]
pub struct Client {
    http: reqwest::Client,
    endpoint: reqwest::Url,
    authority: bool,
    stopped: Arc<AtomicBool>,
}
#[derive(Clone)]
pub struct Clients {
    pub authority: Option<Client>,
    pub changes: Option<Client>,
    pub products: Vec<(String, u64)>,
    leaves: Vec<(&'static str, String)>,
}
#[cfg(test)]
impl Clients {
    pub(crate) fn authority_only(client: Client, products: Vec<(String, u64)>) -> Self {
        Self {
            authority: Some(client),
            changes: None,
            products,
            leaves: Vec::new(),
        }
    }
}
fn flag(key: &'static str, value: Option<OsString>) -> Result<bool, ClientConfigError> {
    match value {
        None => Ok(false),
        Some(v) if v == "true" => Ok(true),
        Some(_) => Err(ClientConfigError::Invalid(key)),
    }
}
pub fn products(raw: &str) -> Result<Vec<(String, u64)>, ClientConfigError> {
    let values: Vec<String> =
        serde_json::from_str(raw).map_err(|_| ClientConfigError::Invalid(PRODUCTS_VAR))?;
    if values.len() > 16 {
        return Err(ClientConfigError::Invalid(PRODUCTS_VAR));
    }
    let mut result = Vec::new();
    for value in values {
        let (id, v) = value
            .split_once('@')
            .ok_or(ClientConfigError::Invalid(PRODUCTS_VAR))?;
        let v = v
            .parse::<u64>()
            .ok()
            .filter(|v| (1..=INT53_MAX).contains(v))
            .ok_or(ClientConfigError::Invalid(PRODUCTS_VAR))?;
        let key = (id.to_owned(), v);
        if !product(id)
            || value != format!("{id}@{v}")
            || id == "oteryn.premium_time" && v == 1
            || result.contains(&key)
        {
            return Err(ClientConfigError::Invalid(PRODUCTS_VAR));
        }
        result.push(key);
    }
    Ok(result)
}
impl Clients {
    pub fn from_lookup(
        lookup: impl Fn(&str) -> Option<OsString>,
    ) -> Result<Option<Self>, ClientConfigError> {
        let authority = flag(AUTHORITY_ENABLED_VAR, lookup(AUTHORITY_ENABLED_VAR))?;
        let changes = flag(CHANGES_ENABLED_VAR, lookup(CHANGES_ENABLED_VAR))?;
        if !authority && !changes {
            return Ok(None);
        }
        let read = |key: &'static str| {
            lookup(key)
                .and_then(|v| std::fs::read(v).ok())
                .ok_or(ClientConfigError::Invalid(key))
        };
        let mut own = Vec::new();
        for (enabled, key) in [
            (authority, AUTHORITY_IDENTITY_VAR),
            (changes, CHANGES_IDENTITY_VAR),
        ] {
            if enabled || lookup(key).is_some() {
                own.push((key, read(key)?));
            }
        }
        let mut identities = Vec::new();
        for key in [
            platform_streams::SANCTION_IDENTITY_VAR,
            platform_streams::LOYALTY_IDENTITY_VAR,
            platform_streams::SECURITY_REVOCATION_IDENTITY_VAR,
            crate::premium::client::IDENTITY_VAR,
            READER_IDENTITY_VAR,
            RESULTS_IDENTITY_VAR,
        ] {
            if lookup(key).is_some() {
                identities.push((key, read(key)?));
            }
        }
        identities.extend(own.clone());
        platform_streams::check_identities(
            &identities
                .iter()
                .map(|(k, p)| (*k, p.as_slice()))
                .collect::<Vec<_>>(),
            None,
        )?;
        let leaves = own
            .iter()
            .map(|(key, pem)| {
                platform_streams::leaf_certificate(pem)
                    .map(|leaf| (*key, leaf))
                    .ok_or(ClientConfigError::Invalid(key))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let origin = lookup(STREAMS_URL_VAR)
            .and_then(|v| v.into_string().ok())
            .ok_or(ClientConfigError::Invalid(STREAMS_URL_VAR))?;
        let ca = read(STREAMS_PLATFORM_CA_VAR)?;
        let mut authority_client = None;
        let mut changes_client = None;
        for (key, pem) in own {
            if key == AUTHORITY_IDENTITY_VAR && !authority
                || key == CHANGES_IDENTITY_VAR && !changes
            {
                continue;
            }
            let (http, origin) = platform_http(&PremiumClientConfig {
                origin: origin.clone(),
                identity_pem: pem,
                platform_ca_pem: ca.clone(),
            })
            .map_err(|ClientConfigError::Invalid(k)| {
                ClientConfigError::Invalid(match k {
                    crate::premium::client::URL_VAR => STREAMS_URL_VAR,
                    crate::premium::client::PLATFORM_CA_VAR => STREAMS_PLATFORM_CA_VAR,
                    _ => key,
                })
            })?;
            let is_authority = key == AUTHORITY_IDENTITY_VAR;
            let endpoint = origin
                .join(if is_authority {
                    AUTHORITY_PATH
                } else {
                    CHANGES_PATH
                })
                .map_err(|_| ClientConfigError::Invalid(STREAMS_URL_VAR))?;
            let client = Client {
                http,
                endpoint,
                authority: is_authority,
                stopped: Arc::default(),
            };
            if is_authority {
                authority_client = Some(client);
            } else {
                changes_client = Some(client);
            }
        }
        let configured = if authority {
            products(
                &lookup(PRODUCTS_VAR)
                    .map(|v| {
                        v.into_string()
                            .map_err(|_| ClientConfigError::Invalid(PRODUCTS_VAR))
                    })
                    .transpose()?
                    .unwrap_or_else(|| "[]".into()),
            )?
        } else {
            Vec::new()
        };
        Ok(Some(Self {
            authority: authority_client,
            changes: changes_client,
            products: configured,
            leaves,
        }))
    }
    pub fn check_other_identities(
        &self,
        others: &[(&'static str, Vec<u8>)],
    ) -> Result<(), ClientConfigError> {
        for (key, pem) in others {
            let leaf =
                platform_streams::leaf_certificate(pem).ok_or(ClientConfigError::Invalid(key))?;
            if let Some((key, _)) = self.leaves.iter().find(|(_, own)| *own == leaf) {
                return Err(ClientConfigError::Invalid(key));
            }
        }
        Ok(())
    }
}
impl Client {
    pub fn stop(&self) {
        self.stopped.store(true, Ordering::Release);
    }
    pub fn stopped(&self) -> bool {
        self.stopped.load(Ordering::Acquire)
    }
    #[cfg(any(test, feature = "test-support"))]
    pub fn local(origin: &str, authority: bool) -> Result<Self, ClientConfigError> {
        let http = reqwest::Client::builder()
            .http1_only()
            .no_proxy()
            .build()
            .map_err(|_| ClientConfigError::Invalid(STREAMS_URL_VAR))?;
        let origin =
            reqwest::Url::parse(origin).map_err(|_| ClientConfigError::Invalid(STREAMS_URL_VAR))?;
        let endpoint = origin
            .join(if authority {
                AUTHORITY_PATH
            } else {
                CHANGES_PATH
            })
            .map_err(|_| ClientConfigError::Invalid(STREAMS_URL_VAR))?;
        Ok(Self {
            http,
            endpoint,
            authority,
            stopped: Arc::default(),
        })
    }
    async fn exchange(&self, body: Vec<u8>) -> Result<Vec<u8>, Failure> {
        if self.stopped() {
            return Err(Failure::Configuration);
        }
        if body.is_empty() || body.len() > 256 {
            return Err(Failure::Defect);
        }
        let bound = if self.authority {
            AUTHORITY_BYTES
        } else {
            PAGE_BYTES
        };
        tokio::time::timeout(EXCHANGE_TIMEOUT, async {
            let mut response = self
                .http
                .post(self.endpoint.clone())
                .header("content-type", "application/json")
                .header("connection", "close")
                .body(body)
                .send()
                .await
                .map_err(|_| Failure::Retry(None))?;
            let retry = response
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<u64>().ok())
                .map(|s| Duration::from_secs(s).min(Duration::from_secs(60)));
            let status = response.status().as_u16();
            if status != 200 {
                return Err(match status {
                    400 => Failure::Defect,
                    401 => Failure::Configuration,
                    404 => Failure::Missing,
                    429 | 503 => Failure::Retry(retry),
                    _ => Failure::Retry(None),
                });
            }
            if response
                .headers()
                .get("content-type")
                .and_then(|v| v.to_str().ok())
                != Some("application/json")
                || response.headers().contains_key("transfer-encoding")
                || response
                    .headers()
                    .get("content-encoding")
                    .is_some_and(|v| v != "identity")
                || response.content_length().is_some_and(|n| n > bound as u64)
            {
                return Err(Failure::Retry(None));
            }
            let mut raw = Vec::new();
            while let Some(chunk) = response.chunk().await.map_err(|_| Failure::Retry(None))? {
                if raw.len().saturating_add(chunk.len()) > bound {
                    return Err(Failure::Retry(None));
                }
                raw.extend_from_slice(&chunk);
            }
            Ok(raw)
        })
        .await
        .unwrap_or(Err(Failure::Retry(None)))
    }
    pub async fn read_authority(
        &self,
        request: &Request,
        products: &[(String, u64)],
    ) -> Result<Vec<u8>, Failure> {
        if !self.authority {
            return Err(Failure::Defect);
        }
        let raw = self
            .exchange(
                Message::Request(request.clone())
                    .encode()
                    .ok_or(Failure::Defect)?,
            )
            .await?;
        match Message::parse(&raw) {
            Some(Message::Answer(answer)) if answer.bound_to(request, products) => Ok(raw),
            _ => Err(Failure::Defect),
        }
    }
    pub async fn read_changes(
        &self,
        after: Option<String>,
        limit: u16,
    ) -> Result<Vec<u8>, Failure> {
        if self.authority {
            return Err(Failure::Defect);
        }
        let raw = self
            .exchange(
                Message::ChangesRequest(ChangesRequest {
                    schema: "oteryn.entitlement_change_request.v1".into(),
                    after,
                    limit,
                })
                .encode()
                .ok_or(Failure::Defect)?,
            )
            .await?;
        match Message::parse(&raw) {
            Some(Message::Page(page)) if page.items.len() <= usize::from(limit) => Ok(raw),
            _ => Err(Failure::Defect),
        }
    }
}
