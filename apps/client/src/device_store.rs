//! Remembered-device credentials belong exclusively to the OS vault, never preferences.
//! Blocking operations must run off the UI/async executor. No password or OAuth refresh token
//! belongs here: this stores only the separately contracted, rotating device-session secret.
//! These are vault primitives, not a completed rotation transaction: the integrating coordinator
//! must provide revocation/tombstones, recovery ordering and cross-process serialization.

use sha2::{Digest, Sha256};
use std::fmt;
use url::{Host, Url};
use zeroize::Zeroizing;

const SERVICE: &str = "Oteryn.Game.DeviceSession.v1";
const MAX_SECRET_BYTES: usize = 2048;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceStoreError {
    InvalidConfiguration,
    InvalidSecret,
    Unavailable,
    Locked,
    Ambiguous,
}

impl fmt::Display for DeviceStoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::InvalidConfiguration => "invalid remembered-device vault configuration",
            Self::InvalidSecret => "invalid remembered-device credential",
            Self::Unavailable => "system credential vault unavailable",
            Self::Locked => "system credential vault locked",
            Self::Ambiguous => "ambiguous remembered-device credentials",
        })
    }
}
impl std::error::Error for DeviceStoreError {}

/// Secret buffers are erased on drop and never exposed through formatting or serialization.
pub struct DeviceSecret(Zeroizing<String>);

impl DeviceSecret {
    pub fn new(secret: String) -> Result<Self, DeviceStoreError> {
        let secret = Zeroizing::new(secret);
        if secret.is_empty()
            || secret.len() > MAX_SECRET_BYTES
            || !secret.bytes().all(|byte| byte.is_ascii_graphic())
        {
            return Err(DeviceStoreError::InvalidSecret);
        }
        Ok(Self(secret))
    }

    pub fn expose(&self) -> &str {
        &self.0
    }

    fn from_bytes(bytes: Vec<u8>) -> Result<Self, DeviceStoreError> {
        let bytes = Zeroizing::new(bytes);
        if bytes.len() > MAX_SECRET_BYTES {
            return Err(DeviceStoreError::InvalidSecret);
        }
        let text = std::str::from_utf8(&bytes).map_err(|_| DeviceStoreError::InvalidSecret)?;
        Self::new(text.to_owned())
    }
}

impl fmt::Debug for DeviceSecret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("DeviceSecret([REDACTED])")
    }
}
impl fmt::Display for DeviceSecret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[REDACTED]")
    }
}

/// One local credential per canonical Platform origin and public OAuth client identifier.
/// Changing issuer, client identifier, protocol scheme or non-default port selects another entry.
#[derive(Clone)]
pub struct DeviceStore {
    namespace: String,
}

impl DeviceStore {
    pub fn new(
        platform_origin: &str,
        oauth_client_id: &str,
        allow_loopback_development: bool,
    ) -> Result<Self, DeviceStoreError> {
        let invalid = DeviceStoreError::InvalidConfiguration;
        if platform_origin.len() > 2048
            || oauth_client_id.is_empty()
            || oauth_client_id.len() > 128
            || !oauth_client_id.bytes().all(|byte| byte.is_ascii_graphic())
        {
            return Err(invalid);
        }
        let origin = Url::parse(platform_origin).map_err(|_| invalid)?;
        let loopback = matches!(origin.host(), Some(Host::Domain("localhost")))
            || matches!(origin.host(), Some(Host::Ipv4(ip)) if ip == std::net::Ipv4Addr::LOCALHOST)
            || matches!(origin.host(), Some(Host::Ipv6(ip)) if ip == std::net::Ipv6Addr::LOCALHOST);
        if origin.host().is_none()
            || !origin.username().is_empty()
            || origin.password().is_some()
            || origin.query().is_some()
            || origin.fragment().is_some()
            || origin.path() != "/"
            || origin.port() == Some(0)
            || !(origin.scheme() == "https"
                || (origin.scheme() == "http" && loopback && allow_loopback_development))
        {
            return Err(invalid);
        }
        let mut digest = Sha256::new();
        digest.update(SERVICE.as_bytes());
        digest.update([0]);
        digest.update(origin.origin().ascii_serialization().as_bytes());
        digest.update([0]);
        digest.update(oauth_client_id.as_bytes());
        let digest: String = digest
            .finalize()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        let namespace = format!("{SERVICE}:{digest}");
        Ok(Self { namespace })
    }

    pub fn load(&self) -> Result<Option<DeviceSecret>, DeviceStoreError> {
        backend::load(&self.namespace)
    }

    /// Write one credential. This primitive does not complete a server rotation transaction.
    /// A write error must not leave the caller treating the old secret as usable remembered access.
    pub fn save(&self, secret: &DeviceSecret) -> Result<(), DeviceStoreError> {
        backend::save(&self.namespace, secret)
    }

    /// Idempotent local deletion, including duplicate exact-namespace records. Server revocation
    /// and a durable forget/failed-rotation tombstone remain separate coordinator responsibilities.
    pub fn delete(&self) -> Result<(), DeviceStoreError> {
        backend::delete(&self.namespace)
    }
}

#[cfg(windows)]
mod backend {
    use super::*;
    use keyring_core::api::CredentialStoreApi;
    use std::collections::HashMap;
    fn entry(namespace: &str) -> Result<keyring_core::Entry, DeviceStoreError> {
        // Explicit backend; Local persistence prevents enterprise credential roaming.
        windows_native_keyring_store::Store::new()
            .and_then(|store| {
                store.build(
                    SERVICE,
                    "remembered-device",
                    Some(&HashMap::from([
                        ("target", namespace),
                        ("persistence", "Local"),
                    ])),
                )
            })
            .map_err(|_| DeviceStoreError::Unavailable)
    }
    pub fn load(namespace: &str) -> Result<Option<DeviceSecret>, DeviceStoreError> {
        let entry = entry(namespace)?;
        match entry.get_attributes() {
            Ok(attributes)
                if attributes.get("persistence").map(String::as_str) == Some("Local") =>
            {
                DeviceSecret::from_bytes(
                    entry
                        .get_secret()
                        .map_err(|_| DeviceStoreError::Unavailable)?,
                )
                .map(Some)
            }
            Err(keyring_core::Error::NoEntry) => Ok(None),
            _ => Err(DeviceStoreError::Unavailable),
        }
    }
    pub fn save(namespace: &str, secret: &DeviceSecret) -> Result<(), DeviceStoreError> {
        entry(namespace)?
            .set_secret(secret.expose().as_bytes())
            .map_err(|_| DeviceStoreError::Unavailable)
    }
    pub fn delete(namespace: &str) -> Result<(), DeviceStoreError> {
        match entry(namespace)?.delete_credential() {
            Ok(()) | Err(keyring_core::Error::NoEntry) => Ok(()),
            Err(_) => Err(DeviceStoreError::Unavailable),
        }
    }
}

#[cfg(target_os = "linux")]
mod backend {
    use super::*;
    use secret_service::{EncryptionType, blocking::SecretService};
    use std::collections::HashMap;

    enum Operation<'a> {
        Load,
        Save(&'a DeviceSecret),
        Delete,
    }
    impl Operation<'_> {
        fn validate_match_count(&self, count: usize) -> Result<(), DeviceStoreError> {
            if count > 1 && !matches!(self, Self::Delete) {
                Err(DeviceStoreError::Ambiguous)
            } else {
                Ok(())
            }
        }
    }
    fn apply(
        namespace: &str,
        operation: Operation<'_>,
    ) -> Result<Option<DeviceSecret>, DeviceStoreError> {
        let unavailable = |_| DeviceStoreError::Unavailable;
        let service = SecretService::connect(EncryptionType::Dh).map_err(unavailable)?;
        let collection = service.get_default_collection().map_err(unavailable)?;
        // Never accept Secret Service's ephemeral session collection or fallback to it.
        if collection.collection_path.as_str() == "/org/freedesktop/secrets/collection/session" {
            return Err(DeviceStoreError::Unavailable);
        }
        match service.get_collection_by_alias("session") {
            Ok(session) if session.collection_path == collection.collection_path => {
                return Err(DeviceStoreError::Unavailable);
            }
            Ok(_) | Err(secret_service::Error::NoResult) => {}
            Err(_) => return Err(DeviceStoreError::Unavailable),
        }
        collection.ensure_unlocked().map_err(|error| match error {
            secret_service::Error::Locked => DeviceStoreError::Locked,
            _ => DeviceStoreError::Unavailable,
        })?;
        let attributes = HashMap::from([("application", SERVICE), ("namespace", namespace)]);
        let items = collection
            .search_items(attributes.clone())
            .map_err(unavailable)?;
        operation.validate_match_count(items.len())?;
        for item in &items {
            item.ensure_unlocked().map_err(|error| match error {
                secret_service::Error::Locked => DeviceStoreError::Locked,
                _ => DeviceStoreError::Unavailable,
            })?;
        }
        match operation {
            Operation::Load => items
                .first()
                .map(|item| DeviceSecret::from_bytes(item.get_secret().map_err(unavailable)?))
                .transpose(),
            Operation::Save(secret) => {
                collection
                    .create_item(
                        "Oteryn remembered device",
                        attributes,
                        secret.expose().as_bytes(),
                        true,
                        "application/octet-stream",
                    )
                    .map_err(unavailable)?;
                Ok(None)
            }
            Operation::Delete => {
                for item in &items {
                    item.delete().map_err(unavailable)?;
                }
                Ok(None)
            }
        }
    }
    pub fn load(namespace: &str) -> Result<Option<DeviceSecret>, DeviceStoreError> {
        apply(namespace, Operation::Load)
    }
    pub fn save(namespace: &str, secret: &DeviceSecret) -> Result<(), DeviceStoreError> {
        apply(namespace, Operation::Save(secret)).map(|_| ())
    }
    pub fn delete(namespace: &str) -> Result<(), DeviceStoreError> {
        apply(namespace, Operation::Delete).map(|_| ())
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        #[test]
        fn duplicate_namespace_can_be_forgotten_but_never_loaded_or_overwritten()
        -> Result<(), DeviceStoreError> {
            let secret = DeviceSecret::new("unused-regression-test-value".to_owned())?;
            for operation in [Operation::Load, Operation::Save(&secret)] {
                assert_eq!(
                    operation.validate_match_count(2),
                    Err(DeviceStoreError::Ambiguous)
                );
                operation.validate_match_count(0)?;
                operation.validate_match_count(1)?;
            }
            Operation::Delete.validate_match_count(0)?;
            Operation::Delete.validate_match_count(1)?;
            Operation::Delete.validate_match_count(2)?;
            Ok(())
        }
    }
}

#[cfg(not(any(windows, target_os = "linux")))]
mod backend {
    use super::*;
    pub fn load(_: &str) -> Result<Option<DeviceSecret>, DeviceStoreError> {
        Err(DeviceStoreError::Unavailable)
    }
    pub fn save(_: &str, _: &DeviceSecret) -> Result<(), DeviceStoreError> {
        Err(DeviceStoreError::Unavailable)
    }
    pub fn delete(_: &str) -> Result<(), DeviceStoreError> {
        Err(DeviceStoreError::Unavailable)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn vault_scope_canonicalizes_origin_and_separates_trust_boundaries() {
        let scope =
            |origin, client| DeviceStore::new(origin, client, false).map(|store| store.namespace);
        assert_eq!(
            scope("https://OTERYN.com:443", "client"),
            scope("https://oteryn.com/", "client")
        );
        let expected = scope("https://oteryn.com", "client");
        assert_ne!(expected, scope("https://elsewhere.com", "client"));
        assert_ne!(expected, scope("https://oteryn.com:444", "client"));
        assert_ne!(expected, scope("https://oteryn.com", "other-client"));
    }
    #[test]
    fn rejects_untrusted_transport_and_non_origin_configuration() {
        for origin in [
            "http://oteryn.com",
            "https://user@oteryn.com",
            "https://oteryn.com/path",
            "https://oteryn.com/?issuer=other",
            "https://oteryn.com/#fragment",
            "file:///tmp",
            "https://oteryn.com:0",
        ] {
            assert!(DeviceStore::new(origin, "client", true).is_err());
        }
        assert!(DeviceStore::new("https://oteryn.com", "client\n", false).is_err());
        assert!(DeviceStore::new("http://127.0.0.1:18584", "client", false).is_err());
        assert!(DeviceStore::new("http://127.0.0.1:18584", "client", true).is_ok());
        assert!(DeviceStore::new("http://127.0.0.2:18584", "client", true).is_err());
    }
    #[test]
    fn credentials_are_bounded_and_redacted() -> Result<(), DeviceStoreError> {
        let secret = DeviceSecret::new("test-opaque-device-secret".to_owned())?;
        assert_eq!(format!("{secret:?}"), "DeviceSecret([REDACTED])");
        assert_eq!(format!("{secret}"), "[REDACTED]");
        for invalid in [
            String::new(),
            "a\nb".to_owned(),
            "x".repeat(MAX_SECRET_BYTES + 1),
        ] {
            assert!(DeviceSecret::new(invalid).is_err());
        }
        assert!(DeviceSecret::from_bytes(vec![0xff]).is_err());
        Ok(())
    }
}
