use alloc::boxed::Box;
use alloc::vec::Vec;
#[cfg(feature = "std")]
use core::mem::size_of;

use aws_lc_rs::kem;

use super::INVALID_KEY_SHARE;
use crate::crypto::{ActiveKeyExchange, CompletedKeyExchange, SharedSecret, SupportedKxGroup};
#[cfg(feature = "std")]
use crate::crypto::{ResourceOwnedKx, ensure_aws_lc_provider_residency};
#[cfg(feature = "std")]
use crate::{DeframerBufferOwner, sync::Arc};
use crate::ffdhe_groups::FfdheGroup;
use crate::{Error, NamedGroup, ProtocolVersion};

#[derive(Debug)]
pub(crate) struct MlKem {
    pub(crate) alg: &'static kem::Algorithm<kem::AlgorithmId>,
    pub(crate) group: NamedGroup,
}

impl SupportedKxGroup for MlKem {
    fn start(&self) -> Result<Box<dyn ActiveKeyExchange>, Error> {
        let decaps_key = kem::DecapsulationKey::generate(self.alg)
            .map_err(|_| Error::General("key generation failed".into()))?;

        let pub_key_bytes = decaps_key
            .encapsulation_key()
            .and_then(|encaps_key| encaps_key.key_bytes())
            .map_err(|_| Error::General("encaps failed".into()))?;

        Ok(Box::new(Active {
            decaps_key: Box::new(decaps_key),
            encaps_key_bytes: Vec::from(pub_key_bytes.as_ref()),
            group: self.group,
        }))
    }

    #[cfg(feature = "std")]
    fn start_with_resource_owner(
        &self,
        owner: Arc<dyn DeframerBufferOwner>,
    ) -> Result<Box<dyn ActiveKeyExchange>, Error> {
        ensure_aws_lc_provider_residency(owner.clone())?;
        if self.group != NamedGroup::MLKEM768
            || size_of::<Active>() != 40
            || size_of::<kem::DecapsulationKey<kem::AlgorithmId>>() != 16
        {
            return Err(Error::FailedToGetRandomBytes);
        }
        ResourceOwnedKx::start(owner, 6_264, || self.start())
    }

    fn start_and_complete(&self, client_share: &[u8]) -> Result<CompletedKeyExchange, Error> {
        let encaps_key =
            kem::EncapsulationKey::new(self.alg, client_share).map_err(|_| INVALID_KEY_SHARE)?;

        let (ciphertext, shared_secret) = encaps_key
            .encapsulate()
            .map_err(|_| INVALID_KEY_SHARE)?;

        Ok(CompletedKeyExchange {
            group: self.name(),
            pub_key: Vec::from(ciphertext.as_ref()),
            secret: SharedSecret::from(shared_secret.as_ref()),
        })
    }

    fn ffdhe_group(&self) -> Option<FfdheGroup<'static>> {
        None
    }

    fn name(&self) -> NamedGroup {
        self.group
    }

    fn fips(&self) -> bool {
        // AUDITORS:
        // At the time of writing, the ML-KEM implementation in AWS-LC-FIPS module 3.0
        // is FIPS-pending.  Some regulatory regimes (eg, FedRAMP rev 5 SC-13) allow
        // use of implementations in this state, as if they are already approved.
        //
        // We follow this liberal interpretation, and say MlKem768 is FIPS-compliant
        // if the underlying library is in FIPS mode.
        //
        // TODO: adjust the `fips()` function return type to allow more policies to
        // be expressed, perhaps following something like
        // <https://github.com/golang/go/issues/70200#issuecomment-2490017956> --
        // see <https://github.com/rustls/rustls/issues/2309>
        super::super::fips()
    }

    fn usable_for_version(&self, version: ProtocolVersion) -> bool {
        version == ProtocolVersion::TLSv1_3
    }
}

struct Active {
    decaps_key: Box<kem::DecapsulationKey<kem::AlgorithmId>>,
    encaps_key_bytes: Vec<u8>,
    group: NamedGroup,
}

impl ActiveKeyExchange for Active {
    // The received 'peer_pub_key' is actually the ML-KEM ciphertext,
    // which when decapsulated with our `decaps_key` produces the shared
    // secret.
    fn complete(self: Box<Self>, peer_pub_key: &[u8]) -> Result<SharedSecret, Error> {
        let shared_secret = self
            .decaps_key
            .decapsulate(peer_pub_key.into())
            .map_err(|_| INVALID_KEY_SHARE)?;

        Ok(SharedSecret::from(shared_secret.as_ref()))
    }

    fn pub_key(&self) -> &[u8] {
        &self.encaps_key_bytes
    }

    fn ffdhe_group(&self) -> Option<FfdheGroup<'static>> {
        None
    }

    fn group(&self) -> NamedGroup {
        self.group
    }
}
