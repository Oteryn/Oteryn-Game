//! Canonical Platform producer descriptor facts (OPS-NODE-BOOT-01 D1/D3).
//!
//! The facts bind the S1 source authority, endpoint, peer name, trust-root
//! digest and client-identity digest. The operator records exactly these
//! bytes in a control-plane issuance, and the serving node derives the same
//! bytes from its own configuration and files before registering them. The
//! Character intent issuer is the compiled constant and the operation paths
//! are compiled, so neither is an issued value.

use rustls::pki_types::CertificateDer;
use rustls::pki_types::pem::PemObject;
use sha2::{Digest, Sha256};
use std::net::SocketAddr;

const FACTS_VERSION: &str = "oteryn-platform-producer-descriptor-v1";
/// Upper bound on a PEM trust-material file.
pub const MAX_PEM_BYTES: usize = 64 * 1024;

/// A PEM certificate list that is empty, malformed or over its bound.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidCertificates;

/// Parse a bounded, non-empty PEM certificate list.
pub fn certificates(pem: &[u8]) -> Result<Vec<CertificateDer<'static>>, InvalidCertificates> {
    if pem.len() > MAX_PEM_BYTES {
        return Err(InvalidCertificates);
    }
    let certificates = CertificateDer::pem_slice_iter(pem)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| InvalidCertificates)?;
    if certificates.is_empty() {
        return Err(InvalidCertificates);
    }
    Ok(certificates)
}

fn digest(certificates: &[CertificateDer<'_>]) -> String {
    let mut hash = Sha256::new();
    for certificate in certificates {
        hash.update((certificate.as_ref().len() as u64).to_be_bytes());
        hash.update(certificate.as_ref());
    }
    hash.finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// The canonical facts bytes for one producer.
#[must_use]
pub fn descriptor_facts(
    source_authority: &str,
    endpoint: SocketAddr,
    peer_name: &str,
    trust_roots: &[CertificateDer<'_>],
    client_chain: &[CertificateDer<'_>],
) -> Vec<u8> {
    format!(
        "{FACTS_VERSION}\nsource_authority={source_authority}\nendpoint={endpoint}\npeer_name={peer_name}\n\
         trust_roots_sha256={}\nclient_identity_sha256={}\ncharacter_issuer={}\n",
        digest(trust_roots),
        digest(client_chain),
        crate::character_bootstrap_intent::ISSUER_AUTHORITY,
    )
    .into_bytes()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]
    use super::*;

    #[test]
    fn facts_bind_every_descriptor_field() {
        let a = CertificateDer::from(vec![1_u8, 2, 3]);
        let b = CertificateDer::from(vec![4_u8]);
        let endpoint: SocketAddr = "10.0.0.5:8443".parse().expect("endpoint");
        let base = descriptor_facts(
            "platform",
            endpoint,
            "peer",
            std::slice::from_ref(&a),
            std::slice::from_ref(&b),
        );
        assert!(base.len() < 4096);
        for changed in [
            descriptor_facts(
                "other",
                endpoint,
                "peer",
                std::slice::from_ref(&a),
                std::slice::from_ref(&b),
            ),
            descriptor_facts(
                "platform",
                "10.0.0.6:8443".parse().expect("endpoint"),
                "peer",
                std::slice::from_ref(&a),
                std::slice::from_ref(&b),
            ),
            descriptor_facts(
                "platform",
                endpoint,
                "other",
                std::slice::from_ref(&a),
                std::slice::from_ref(&b),
            ),
            descriptor_facts(
                "platform",
                endpoint,
                "peer",
                std::slice::from_ref(&b),
                std::slice::from_ref(&b),
            ),
            descriptor_facts(
                "platform",
                endpoint,
                "peer",
                std::slice::from_ref(&a),
                std::slice::from_ref(&a),
            ),
        ] {
            assert_ne!(changed, base);
        }
        assert_eq!(
            descriptor_facts("platform", endpoint, "peer", &[a], &[b]),
            base
        );
        assert_eq!(certificates(b""), Err(InvalidCertificates));
    }
}
