use alloc::vec::Vec;

#[cfg(all(feature = "std", not(feature = "fips"), target_os = "linux", target_arch = "x86_64"))]
use core::alloc::Layout;
#[cfg(all(
    feature = "std",
    not(feature = "fips"),
    target_os = "linux",
    target_arch = "x86_64"
))]
use core::mem::size_of;
#[cfg(all(feature = "std", not(feature = "fips"), target_os = "linux", target_arch = "x86_64"))]
use core::sync::atomic::AtomicUsize;

// aws-lc-rs has a -- roughly -- ring-compatible API, so we just reuse all that
// glue here.  The shared files should always use `super::ring_like` to access a
// ring-compatible crate, and `super::ring_shim` to bridge the gaps where they are
// small.
pub(crate) use aws_lc_rs as ring_like;
use pki_types::PrivateKeyDer;
use webpki::aws_lc_rs as webpki_algs;

use crate::crypto::{CryptoProvider, KeyProvider, SecureRandom, SupportedKxGroup};
#[cfg(all(feature = "std", not(feature = "fips"), target_os = "linux", target_arch = "x86_64"))]
use crate::DeframerBufferOwner;
use crate::enums::SignatureScheme;
use crate::rand::GetRandomFailed;
use crate::sign::SigningKey;
use crate::suites::SupportedCipherSuite;
use crate::sync::Arc;
use crate::webpki::WebPkiSupportedAlgorithms;
use crate::{Error, OtherError};

/// Hybrid public key encryption (HPKE).
pub mod hpke;
/// Post-quantum secure algorithms.
pub(crate) mod pq;
/// Using software keys for authentication.
pub mod sign;

#[path = "../ring/hash.rs"]
pub(crate) mod hash;
#[path = "../ring/hmac.rs"]
pub(crate) mod hmac;
#[path = "../ring/kx.rs"]
pub(crate) mod kx;
#[path = "../ring/quic.rs"]
pub(crate) mod quic;
#[cfg(feature = "std")]
pub(crate) mod ticketer;
#[cfg(feature = "tls12")]
pub(crate) mod tls12;
pub(crate) mod tls13;

/// A `CryptoProvider` backed by aws-lc-rs.
pub fn default_provider() -> CryptoProvider {
    CryptoProvider {
        cipher_suites: DEFAULT_CIPHER_SUITES.to_vec(),
        kx_groups: default_kx_groups(),
        signature_verification_algorithms: SUPPORTED_SIG_ALGS,
        secure_random: &AwsLcRs,
        key_provider: &AwsLcRs,
    }
}

fn default_kx_groups() -> Vec<&'static dyn SupportedKxGroup> {
    #[cfg(feature = "fips")]
    {
        DEFAULT_KX_GROUPS
            .iter()
            .filter(|cs| cs.fips())
            .copied()
            .collect()
    }
    #[cfg(not(feature = "fips"))]
    {
        DEFAULT_KX_GROUPS.to_vec()
    }
}

#[cfg(all(feature = "std", not(feature = "fips"), target_os = "linux", target_arch = "x86_64"))]
static OWNER_DEFAULT_PROVIDER: std::sync::Mutex<Option<Arc<CryptoProvider>>> =
    std::sync::Mutex::new(None);

#[cfg(all(feature = "std", not(feature = "fips"), target_os = "linux", target_arch = "x86_64"))]
fn owner_default_provider_bytes() -> Result<usize, Error> {
    let cipher_vec = DEFAULT_CIPHER_SUITES
        .len()
        .checked_mul(size_of::<SupportedCipherSuite>())
        .ok_or(Error::FailedToGetRandomBytes)?;
    let kx_vec = DEFAULT_KX_GROUPS
        .len()
        .checked_mul(size_of::<&'static dyn SupportedKxGroup>())
        .ok_or(Error::FailedToGetRandomBytes)?;

    // Rust 1.94 alloc::sync::Arc requests the padded ArcInner<T> layout:
    // two AtomicUsize counters followed by T.
    let (arc_inner, _) = Layout::new::<[AtomicUsize; 2]>()
        .extend(Layout::new::<CryptoProvider>())
        .map_err(|_| Error::FailedToGetRandomBytes)?;
    let arc_provider = arc_inner.pad_to_align().size();

    cipher_vec
        .checked_add(kx_vec)
        .and_then(|bytes| bytes.checked_add(arc_provider))
        .ok_or(Error::FailedToGetRandomBytes)
}

/// Return the process-shared default AWS-LC provider after charging its exact
/// Rust 1.94 configuration backing to the caller's shared root.
#[cfg(all(feature = "std", not(feature = "fips"), target_os = "linux", target_arch = "x86_64"))]
pub fn default_provider_with_resource_owner(
    owner: Arc<dyn DeframerBufferOwner>,
) -> Result<Arc<CryptoProvider>, Error> {
    super::ensure_aws_lc_provider_residency(owner.clone())?;
    let mut shared = OWNER_DEFAULT_PROVIDER
        .lock()
        .map_err(|_| Error::FailedToGetRandomBytes)?;
    if let Some(provider) = shared.as_ref() {
        return Ok(provider.clone());
    }

    let bytes = owner_default_provider_bytes()?;
    owner
        .try_reserve_provider_shared(bytes)
        .map_err(|_| Error::FailedToGetRandomBytes)?;
    let provider = Arc::new(default_provider());
    if provider.cipher_suites.capacity() != DEFAULT_CIPHER_SUITES.len()
        || provider.kx_groups.capacity() != DEFAULT_KX_GROUPS.len()
        || provider.cipher_suites.as_slice() != DEFAULT_CIPHER_SUITES
        || provider.kx_groups.len() != DEFAULT_KX_GROUPS.len()
        || !provider
            .kx_groups
            .iter()
            .zip(DEFAULT_KX_GROUPS.iter())
            .all(|(actual, expected)| actual.name() == expected.name())
    {
        return Err(Error::FailedToGetRandomBytes);
    }
    *shared = Some(provider.clone());
    Ok(provider)
}

#[cfg(all(feature = "std", any(
    feature = "fips",
    not(all(target_os = "linux", target_arch = "x86_64"))
)))]
pub fn default_provider_with_resource_owner(
    _owner: Arc<dyn crate::DeframerBufferOwner>,
) -> Result<Arc<CryptoProvider>, Error> {
    Err(Error::FailedToGetRandomBytes)
}

#[cfg(all(
    test,
    feature = "std",
    not(feature = "fips"),
    target_os = "linux",
    target_arch = "x86_64"
))]
mod owner_provider_tests {
    use super::*;
    use std::sync::Mutex;

    #[derive(Debug)]
    struct Owner {
        config_limit: usize,
        debits: Mutex<Vec<usize>>,
    }

    impl DeframerBufferOwner for Owner {
        fn try_reserve(&self, _bytes: usize) -> Result<(), crate::DeframerBufferError> {
            Ok(())
        }

        fn release(&self, _bytes: usize) {}

        fn try_reserve_provider_shared(
            &self,
            bytes: usize,
        ) -> Result<(), crate::DeframerBufferError> {
            self.debits.lock().unwrap().push(bytes);
            if bytes == owner_default_provider_bytes().unwrap() && bytes > self.config_limit {
                return Err(crate::DeframerBufferError);
            }
            Ok(())
        }
    }

    #[test]
    fn owner_provider_denies_before_allocation_then_racing_first_use_is_shared() {
        let config_bytes = owner_default_provider_bytes().unwrap();
        let denied = Arc::new(Owner {
            config_limit: config_bytes - 1,
            debits: Mutex::new(Vec::new()),
        });
        assert!(default_provider_with_resource_owner(denied.clone()).is_err());
        assert_eq!(
            denied
                .debits
                .lock()
                .unwrap()
                .iter()
                .filter(|bytes| **bytes == config_bytes)
                .count(),
            1
        );

        let funded = Arc::new(Owner {
            config_limit: config_bytes,
            debits: Mutex::new(Vec::new()),
        });
        let threads = (0..4)
            .map(|_| {
                let owner = funded.clone();
                std::thread::spawn(move || default_provider_with_resource_owner(owner).unwrap())
            })
            .collect::<Vec<_>>();
        let providers = threads
            .into_iter()
            .map(|thread| thread.join().unwrap())
            .collect::<Vec<_>>();

        assert!(providers
            .iter()
            .all(|provider| Arc::ptr_eq(&providers[0], provider)));
        assert_eq!(
            funded
                .debits
                .lock()
                .unwrap()
                .iter()
                .filter(|bytes| **bytes == config_bytes)
                .count(),
            1
        );
        assert_eq!(providers[0].cipher_suites.capacity(), DEFAULT_CIPHER_SUITES.len());
        assert_eq!(providers[0].kx_groups.capacity(), DEFAULT_KX_GROUPS.len());
        assert_eq!(providers[0].cipher_suites.as_slice(), DEFAULT_CIPHER_SUITES);
        assert!(providers[0]
            .kx_groups
            .iter()
            .zip(DEFAULT_KX_GROUPS.iter())
            .all(|(actual, expected)| actual.name() == expected.name()));
    }
}

#[derive(Debug)]
struct AwsLcRs;

impl SecureRandom for AwsLcRs {
    fn fill(&self, buf: &mut [u8]) -> Result<(), GetRandomFailed> {
        use ring_like::rand::SecureRandom;

        ring_like::rand::SystemRandom::new()
            .fill(buf)
            .map_err(|_| GetRandomFailed)
    }

    fn fips(&self) -> bool {
        fips()
    }
}

impl KeyProvider for AwsLcRs {
    fn load_private_key(
        &self,
        key_der: PrivateKeyDer<'static>,
    ) -> Result<Arc<dyn SigningKey>, Error> {
        sign::any_supported_type(&key_der)
    }

    fn fips(&self) -> bool {
        fips()
    }
}

/// The cipher suite configuration that an application should use by default.
///
/// This will be [`ALL_CIPHER_SUITES`] sans any supported cipher suites that
/// shouldn't be enabled by most applications.
pub static DEFAULT_CIPHER_SUITES: &[SupportedCipherSuite] = &[
    // TLS1.3 suites
    tls13::TLS13_AES_256_GCM_SHA384,
    tls13::TLS13_AES_128_GCM_SHA256,
    #[cfg(not(feature = "fips"))]
    tls13::TLS13_CHACHA20_POLY1305_SHA256,
    // TLS1.2 suites
    #[cfg(feature = "tls12")]
    tls12::TLS_ECDHE_ECDSA_WITH_AES_256_GCM_SHA384,
    #[cfg(feature = "tls12")]
    tls12::TLS_ECDHE_ECDSA_WITH_AES_128_GCM_SHA256,
    #[cfg(all(feature = "tls12", not(feature = "fips")))]
    tls12::TLS_ECDHE_ECDSA_WITH_CHACHA20_POLY1305_SHA256,
    #[cfg(feature = "tls12")]
    tls12::TLS_ECDHE_RSA_WITH_AES_256_GCM_SHA384,
    #[cfg(feature = "tls12")]
    tls12::TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256,
    #[cfg(all(feature = "tls12", not(feature = "fips")))]
    tls12::TLS_ECDHE_RSA_WITH_CHACHA20_POLY1305_SHA256,
];

/// A list of all the cipher suites supported by the rustls aws-lc-rs provider.
pub static ALL_CIPHER_SUITES: &[SupportedCipherSuite] = &[
    // TLS1.3 suites
    tls13::TLS13_AES_256_GCM_SHA384,
    tls13::TLS13_AES_128_GCM_SHA256,
    tls13::TLS13_CHACHA20_POLY1305_SHA256,
    // TLS1.2 suites
    #[cfg(feature = "tls12")]
    tls12::TLS_ECDHE_ECDSA_WITH_AES_256_GCM_SHA384,
    #[cfg(feature = "tls12")]
    tls12::TLS_ECDHE_ECDSA_WITH_AES_128_GCM_SHA256,
    #[cfg(feature = "tls12")]
    tls12::TLS_ECDHE_ECDSA_WITH_CHACHA20_POLY1305_SHA256,
    #[cfg(feature = "tls12")]
    tls12::TLS_ECDHE_RSA_WITH_AES_256_GCM_SHA384,
    #[cfg(feature = "tls12")]
    tls12::TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256,
    #[cfg(feature = "tls12")]
    tls12::TLS_ECDHE_RSA_WITH_CHACHA20_POLY1305_SHA256,
];

/// All defined cipher suites supported by aws-lc-rs appear in this module.
pub mod cipher_suite {
    #[cfg(feature = "tls12")]
    pub use super::tls12::{
        TLS_ECDHE_ECDSA_WITH_AES_128_GCM_SHA256, TLS_ECDHE_ECDSA_WITH_AES_256_GCM_SHA384,
        TLS_ECDHE_ECDSA_WITH_CHACHA20_POLY1305_SHA256, TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256,
        TLS_ECDHE_RSA_WITH_AES_256_GCM_SHA384, TLS_ECDHE_RSA_WITH_CHACHA20_POLY1305_SHA256,
    };
    pub use super::tls13::{
        TLS13_AES_128_GCM_SHA256, TLS13_AES_256_GCM_SHA384, TLS13_CHACHA20_POLY1305_SHA256,
    };
}

/// A `WebPkiSupportedAlgorithms` value that reflects webpki's capabilities when
/// compiled against aws-lc-rs.
static SUPPORTED_SIG_ALGS: WebPkiSupportedAlgorithms = WebPkiSupportedAlgorithms {
    all: &[
        webpki_algs::ECDSA_P256_SHA256,
        webpki_algs::ECDSA_P256_SHA384,
        webpki_algs::ECDSA_P256_SHA512,
        webpki_algs::ECDSA_P384_SHA256,
        webpki_algs::ECDSA_P384_SHA384,
        webpki_algs::ECDSA_P384_SHA512,
        webpki_algs::ECDSA_P521_SHA256,
        webpki_algs::ECDSA_P521_SHA384,
        webpki_algs::ECDSA_P521_SHA512,
        webpki_algs::ED25519,
        webpki_algs::RSA_PSS_2048_8192_SHA256_LEGACY_KEY,
        webpki_algs::RSA_PSS_2048_8192_SHA384_LEGACY_KEY,
        webpki_algs::RSA_PSS_2048_8192_SHA512_LEGACY_KEY,
        webpki_algs::RSA_PKCS1_2048_8192_SHA256,
        webpki_algs::RSA_PKCS1_2048_8192_SHA384,
        webpki_algs::RSA_PKCS1_2048_8192_SHA512,
        webpki_algs::RSA_PKCS1_2048_8192_SHA256_ABSENT_PARAMS,
        webpki_algs::RSA_PKCS1_2048_8192_SHA384_ABSENT_PARAMS,
        webpki_algs::RSA_PKCS1_2048_8192_SHA512_ABSENT_PARAMS,
    ],
    mapping: &[
        // Note: for TLS1.2 the curve is not fixed by SignatureScheme. For TLS1.3 it is.
        (
            SignatureScheme::ECDSA_NISTP384_SHA384,
            &[
                webpki_algs::ECDSA_P384_SHA384,
                webpki_algs::ECDSA_P256_SHA384,
                webpki_algs::ECDSA_P521_SHA384,
            ],
        ),
        (
            SignatureScheme::ECDSA_NISTP256_SHA256,
            &[
                webpki_algs::ECDSA_P256_SHA256,
                webpki_algs::ECDSA_P384_SHA256,
                webpki_algs::ECDSA_P521_SHA256,
            ],
        ),
        (
            SignatureScheme::ECDSA_NISTP521_SHA512,
            &[
                webpki_algs::ECDSA_P521_SHA512,
                webpki_algs::ECDSA_P384_SHA512,
                webpki_algs::ECDSA_P256_SHA512,
            ],
        ),
        (SignatureScheme::ED25519, &[webpki_algs::ED25519]),
        (
            SignatureScheme::RSA_PSS_SHA512,
            &[webpki_algs::RSA_PSS_2048_8192_SHA512_LEGACY_KEY],
        ),
        (
            SignatureScheme::RSA_PSS_SHA384,
            &[webpki_algs::RSA_PSS_2048_8192_SHA384_LEGACY_KEY],
        ),
        (
            SignatureScheme::RSA_PSS_SHA256,
            &[webpki_algs::RSA_PSS_2048_8192_SHA256_LEGACY_KEY],
        ),
        (
            SignatureScheme::RSA_PKCS1_SHA512,
            &[webpki_algs::RSA_PKCS1_2048_8192_SHA512],
        ),
        (
            SignatureScheme::RSA_PKCS1_SHA384,
            &[webpki_algs::RSA_PKCS1_2048_8192_SHA384],
        ),
        (
            SignatureScheme::RSA_PKCS1_SHA256,
            &[webpki_algs::RSA_PKCS1_2048_8192_SHA256],
        ),
    ],
};

/// All defined key exchange groups supported by aws-lc-rs appear in this module.
///
/// [`ALL_KX_GROUPS`] is provided as an array of all of these values.
/// [`DEFAULT_KX_GROUPS`] is provided as an array of this provider's defaults.
pub mod kx_group {
    pub use super::kx::{SECP256R1, SECP384R1, X25519};
    pub use super::pq::{MLKEM768, MLKEM1024, SECP256R1MLKEM768, X25519MLKEM768};
}

/// A list of the default key exchange groups supported by this provider.
///
/// This does not contain MLKEM768; by default MLKEM768 is only offered
/// in hybrid with X25519.
pub static DEFAULT_KX_GROUPS: &[&dyn SupportedKxGroup] = &[
    #[cfg(feature = "prefer-post-quantum")]
    kx_group::X25519MLKEM768,
    kx_group::X25519,
    kx_group::SECP256R1,
    kx_group::SECP384R1,
    #[cfg(not(feature = "prefer-post-quantum"))]
    kx_group::X25519MLKEM768,
];

/// A list of all the key exchange groups supported by this provider.
pub static ALL_KX_GROUPS: &[&dyn SupportedKxGroup] = &[
    #[cfg(feature = "prefer-post-quantum")]
    kx_group::X25519MLKEM768,
    #[cfg(feature = "prefer-post-quantum")]
    kx_group::SECP256R1MLKEM768,
    kx_group::X25519,
    kx_group::SECP256R1,
    kx_group::SECP384R1,
    #[cfg(not(feature = "prefer-post-quantum"))]
    kx_group::X25519MLKEM768,
    #[cfg(not(feature = "prefer-post-quantum"))]
    kx_group::SECP256R1MLKEM768,
    kx_group::MLKEM768,
    kx_group::MLKEM1024,
];

#[cfg(feature = "std")]
pub use ticketer::Ticketer;

/// Compatibility shims between ring 0.16.x and 0.17.x API
mod ring_shim {
    use super::ring_like;
    use crate::crypto::SharedSecret;

    pub(super) fn agree_ephemeral(
        priv_key: ring_like::agreement::EphemeralPrivateKey,
        peer_key: &ring_like::agreement::UnparsedPublicKey<&[u8]>,
    ) -> Result<SharedSecret, ()> {
        ring_like::agreement::agree_ephemeral(priv_key, peer_key, (), |secret| {
            Ok(SharedSecret::from(secret))
        })
    }
}

/// Are we in FIPS mode?
pub(super) fn fips() -> bool {
    aws_lc_rs::try_fips_mode().is_ok()
}

pub(super) fn unspecified_err(_e: aws_lc_rs::error::Unspecified) -> Error {
    #[cfg(feature = "std")]
    {
        Error::Other(OtherError(Arc::new(_e)))
    }
    #[cfg(not(feature = "std"))]
    {
        Error::Other(OtherError())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    #[cfg(feature = "fips")]
    #[test]
    fn default_suites_are_fips() {
        assert!(
            super::DEFAULT_CIPHER_SUITES
                .iter()
                .all(|scs| scs.fips())
        );
    }

    #[cfg(not(feature = "fips"))]
    #[test]
    fn default_suites() {
        assert_eq!(super::DEFAULT_CIPHER_SUITES, super::ALL_CIPHER_SUITES);
    }

    #[test]
    fn certificate_sig_algs() {
        // `all` should not contain duplicates (not incorrect, but a waste of time)
        assert_eq!(
            super::SUPPORTED_SIG_ALGS
                .all
                .iter()
                .map(|alg| {
                    (
                        alg.public_key_alg_id()
                            .as_ref()
                            .to_vec(),
                        alg.signature_alg_id().as_ref().to_vec(),
                    )
                })
                .collect::<HashSet<_>>()
                .len(),
            super::SUPPORTED_SIG_ALGS.all.len(),
        );
    }
}
