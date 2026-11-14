//! Purpose-separated enforcement clients. Construction validates the whole group before use.
use super::*;
use crate::premium::client::{ClientConfigError, PremiumClientConfig, platform_http};
use std::{ffi::OsString, time::Duration};

pub const READER_IDENTITY_VAR: &str = "OTERYN_PLATFORM_ENFORCEMENT_READER_CLIENT_IDENTITY_PEM";
pub const RESULTS_IDENTITY_VAR: &str = "OTERYN_PLATFORM_ENFORCEMENT_RESULTS_CLIENT_IDENTITY_PEM";
pub const EXCHANGE_TIMEOUT: Duration = Duration::from_secs(3);
const PREFIX: &str = "/internal/v1/support/enforcement-commands/";

#[derive(Clone)]
pub struct Clients {
    reader: reqwest::Client,
    results: reqwest::Client,
    origin: reqwest::Url,
    leaves: Vec<(&'static str, String)>,
}
impl Clients {
    pub fn from_lookup(
        lookup: impl Fn(&str) -> Option<OsString>,
    ) -> Result<Option<Self>, ClientConfigError> {
        if !super::enabled_value(lookup(ENABLED_VAR))? {
            return Ok(None);
        }
        let read = |var: &'static str| {
            lookup(var)
                .and_then(|path| std::fs::read(path).ok())
                .ok_or(ClientConfigError::Invalid(var))
        };
        // Resolve identities first so a missing purpose fails with its own configuration key.
        let reader = read(READER_IDENTITY_VAR)?;
        let results = read(RESULTS_IDENTITY_VAR)?;
        let mut identities = Vec::new();
        for var in [
            super::super::SANCTION_IDENTITY_VAR,
            super::super::LOYALTY_IDENTITY_VAR,
            super::super::SECURITY_REVOCATION_IDENTITY_VAR,
            crate::premium::client::IDENTITY_VAR,
        ] {
            if lookup(var).is_some() {
                identities.push((var, read(var)?));
            }
        }
        identities.push((READER_IDENTITY_VAR, reader.clone()));
        identities.push((RESULTS_IDENTITY_VAR, results.clone()));
        super::super::check_identities(
            &identities
                .iter()
                .map(|(key, pem)| (*key, pem.as_slice()))
                .collect::<Vec<_>>(),
            None,
        )?;
        let leaves = identities[identities.len() - 2..]
            .iter()
            .map(|(key, pem)| {
                super::super::leaf_certificate(pem)
                    .map(|leaf| (*key, leaf))
                    .ok_or(ClientConfigError::Invalid(key))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let rename = |identity_var| {
            move |ClientConfigError::Invalid(key)| {
                ClientConfigError::Invalid(match key {
                    crate::premium::client::URL_VAR => super::super::STREAMS_URL_VAR,
                    crate::premium::client::PLATFORM_CA_VAR => {
                        super::super::STREAMS_PLATFORM_CA_VAR
                    }
                    _ => identity_var,
                })
            }
        };
        let origin = lookup(super::super::STREAMS_URL_VAR)
            .and_then(|value| value.into_string().ok())
            .ok_or(ClientConfigError::Invalid(super::super::STREAMS_URL_VAR))?;
        let ca = read(super::super::STREAMS_PLATFORM_CA_VAR)?;
        let (reader, url) = platform_http(&PremiumClientConfig {
            origin: origin.clone(),
            identity_pem: reader,
            platform_ca_pem: ca.clone(),
        })
        .map_err(rename(READER_IDENTITY_VAR))?;
        let (results, _) = platform_http(&PremiumClientConfig {
            origin,
            identity_pem: results,
            platform_ca_pem: ca,
        })
        .map_err(rename(RESULTS_IDENTITY_VAR))?;
        Ok(Some(Self {
            reader,
            results,
            origin: url,
            leaves,
        }))
    }
    pub fn check_other_identities(
        &self,
        others: &[(&'static str, Vec<u8>)],
    ) -> Result<(), ClientConfigError> {
        for (key, pem) in others {
            let leaf =
                super::super::leaf_certificate(pem).ok_or(ClientConfigError::Invalid(key))?;
            if let Some((key, _)) = self.leaves.iter().find(|(_, own)| own == &leaf) {
                return Err(ClientConfigError::Invalid(key));
            }
        }
        Ok(())
    }
    #[cfg(any(test, feature = "test-support"))]
    pub fn local(origin: String) -> Result<Self, ClientConfigError> {
        let http = reqwest::Client::builder()
            .http1_only()
            .no_proxy()
            .build()
            .map_err(|_| ClientConfigError::Invalid(super::super::STREAMS_URL_VAR))?;
        let origin = reqwest::Url::parse(&origin)
            .map_err(|_| ClientConfigError::Invalid(super::super::STREAMS_URL_VAR))?;
        Ok(Self {
            reader: http.clone(),
            results: http,
            origin,
            leaves: Vec::new(),
        })
    }
    pub async fn exchange(&self, route: Route, body: Vec<u8>) -> Result<Vec<u8>, Failure> {
        let (path, bound, request_bound, result) = match route {
            Route::Stream => ("stream/read", PAGE_BYTES, 256, false),
            Route::Read => ("commands/read", COMMAND_BYTES, 256, false),
            Route::Results => ("results", 128, 1024, true),
            Route::Reconcile => ("reconcile/read", PAGE_BYTES, 256, false),
        };
        if body.is_empty() || body.len() > request_bound {
            return Err(Failure::Defect);
        }
        let endpoint = self
            .origin
            .join(&format!("{PREFIX}{path}"))
            .map_err(|_| Failure::Defect)?;
        let client = if result { &self.results } else { &self.reader };
        tokio::time::timeout(EXCHANGE_TIMEOUT, async {
            let mut response = client
                .post(endpoint)
                .header("content-type", "application/json")
                .header("connection", "close")
                .body(body)
                .send()
                .await
                .map_err(|_| Failure::Retry(None))?;
            let status = response.status().as_u16();
            let retry = response
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<u64>().ok())
                .map(Duration::from_secs);
            if status != 200 {
                return Err(match status {
                    400 => Failure::Defect,
                    401 => Failure::Configuration,
                    404 => Failure::Missing,
                    409 => Failure::Conflict,
                    _ => Failure::Retry(retry),
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
}
#[derive(Clone, Copy)]
pub enum Route {
    Stream,
    Read,
    Results,
    Reconcile,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Failure {
    Defect,
    Configuration,
    Missing,
    Conflict,
    Retry(Option<Duration>),
}
pub fn backoff(attempt: u32, retry: Option<Duration>) -> Duration {
    Duration::from_secs(1u64 << attempt.min(6))
        .min(Duration::from_secs(60))
        .max(retry.unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn enforcement_other_client_missing_leaf_reports_its_key() -> Result<(), ClientConfigError> {
        let clients = Clients::local("http://127.0.0.1:1".into())?;
        assert!(matches!(
            clients.check_other_identities(&[(
                "platform.runtime_status.client_certificate_file",
                vec![]
            )]),
            Err(ClientConfigError::Invalid(
                "platform.runtime_status.client_certificate_file"
            ))
        ));
        Ok(())
    }
    #[test]
    fn enforcement_flag_off_constructs_no_clients() {
        assert!(matches!(Clients::from_lookup(|_| None), Ok(None)));
        assert!(matches!(
            Clients::from_lookup(|key| (key == ENABLED_VAR).then(|| OsString::from("TRUE"))),
            Err(ClientConfigError::Invalid(ENABLED_VAR))
        ));
    }
    #[test]
    fn enforcement_flag_on_without_identity_fails_startup() {
        assert!(matches!(
            Clients::from_lookup(|key| (key == ENABLED_VAR).then(|| OsString::from("true"))),
            Err(ClientConfigError::Invalid(READER_IDENTITY_VAR))
        ));
    }
    #[test]
    fn enforcement_reader_results_identity_overlap_is_rejected()
    -> Result<(), Box<dyn std::error::Error>> {
        use std::collections::HashMap;
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let directory = std::env::temp_dir().join(format!(
            "enforcement-identities-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        std::fs::create_dir(&directory)?;
        let first = directory.join("reader.pem");
        let second = directory.join("results.pem");
        std::fs::write(
            &first,
            "-----BEGIN CERTIFICATE-----\nYQ==\n-----END CERTIFICATE-----\n",
        )?;
        std::fs::write(
            &second,
            "-----BEGIN CERTIFICATE-----\nYg==\n-----END CERTIFICATE-----\n",
        )?;
        for results in [&first, &second] {
            let mut values = HashMap::from([
                (ENABLED_VAR, OsString::from("true")),
                (READER_IDENTITY_VAR, first.clone().into_os_string()),
                (RESULTS_IDENTITY_VAR, results.clone().into_os_string()),
            ]);
            if results == &second {
                values.insert(
                    super::super::super::SECURITY_REVOCATION_IDENTITY_VAR,
                    second.clone().into_os_string(),
                );
            }
            assert!(matches!(
                Clients::from_lookup(|key| values.get(key).cloned()),
                Err(ClientConfigError::Invalid(RESULTS_IDENTITY_VAR))
            ));
        }
        std::fs::remove_dir_all(directory)?;
        Ok(())
    }
}
