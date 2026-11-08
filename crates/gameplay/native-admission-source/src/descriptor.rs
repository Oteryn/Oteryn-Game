use super::SourceError;
use rustls::{
    DigitallySignedStruct, RootCertStore, SignatureScheme,
    client::{
        WebPkiServerVerifier,
        danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier},
    },
    pki_types::{CertificateDer, PrivateKeyDer, ServerName, UnixTime},
};
use std::sync::Arc;
pub const TRUST_ROOTS_MAX: usize = 4;
pub const TRUST_ROOT_DER_MAX: usize = 4096;
pub const TRUST_ROOT_DER_AGGREGATE_MAX: usize = 16384;
pub const PEER_CERTIFICATES_MAX: usize = 4;
pub const PEER_CERT_DER_MAX: usize = 4096;
pub const PEER_CERT_DER_AGGREGATE_MAX: usize = 16384;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operation {
    ReadAccountSecurityV1,
    ReadFreshSigningTrustV1,
    ReadRecoveryAccountSecurityV2,
    ReadRecoverySigningTrustV2,
    /// Separate private Character bootstrap-intent reconciliation read.
    ReadCharacterBootstrapIntentV1,
    /// Purpose-separated authenticated Premium snapshot pull.
    ReadPremiumSnapshotV1,
    /// Node runtime-status report (`oteryn-game-native-runtime-status-v1` §3).
    ReportRuntimeStatusV1,
    /// `ListCharactersForAccount` snapshot (`oteryn-game-list-characters-for-account-v1` §3).
    PublishAccountCharactersV1,
    /// `ListCharactersForAccount` liveness watermark (same contract, §5.1).
    PublishProjectionWatermarkV1,
    /// Ownership-authority assignment report (`oteryn-game-native-runtime-status-v1` §5).
    ReportScopeAssignmentV1,
    /// Ownership-authority revocation report (same contract, §16.1).
    ReportScopeRevocationV1,
    /// Character command pending list (`oteryn-game-character-authority-commands-v1` §4.1).
    ListPendingCharacterCommandsV1,
    /// Character command intent read (same contract, §4.2).
    ReadCharacterCommandIntentV1,
    /// Character command receipt publication (same contract, §4.3).
    PublishCharacterCommandReceiptV1,
    /// Internal CC v2 closure receipt bound; the wire path and operation stay CC v1.
    PublishAccountClosureCommandReceiptV2,
    ReadAccountClosureLifecycleV1,
    PublishAccountClosureV1,
    PublishAccountClosureWatermarkV1,
    ListPendingStoreOperationsV1,
    ReadStoreOperationIntentV1,
    PublishStoreDeliveryReceiptV1,
    PurchaseStoreOfferV1,
    ReadStoreBalanceV1,
    ReadStoreHistoryPageV1,
    ListStoreOperationStatesV1,
    /// World highscores snapshot (`oteryn-game-public-projections-v1` §4.1).
    PublishWorldHighscoresV1,
    /// Character profile snapshot (same contract, §4.2).
    PublishCharacterProfileV1,
    /// World online snapshot (same contract, §4.6).
    PublishWorldOnlineV1,
    PublishWorldTotalsV1,
    PublishCharacterDeathsV2,
    PublishGuildV2,
    PublishHouseV2,
    PublishCharacterOnlineSessionV2,
    PublishWorldDailyKillsV2,
    PublishPublicProjectionWatermarkV2,
    /// Public projection liveness watermark (same contract, §5.1).
    PublishPublicProjectionWatermarkV1,
    PublishGameRestoreNoticeV1,
    ListPendingBazaarCommandsV1,
    ReadBazaarCommandIntentV1,
    ListBazaarCommandStatesV1,
    PublishBazaarTransferabilityV1,
    PublishBazaarListingStatusV1,
    PublishBazaarWatermarkV1,
    /// Internal transport bound for the accepted CC v2 extension. Same path,
    /// authenticated Character receipt identity and wire operation as CC v1.
    PublishBazaarCharacterCommandReceiptV2,
    PublishWorldEventOutcomeV1,
    PublishGuildSeasonV1,
    PublishLeagueWatermarkV1,
}
impl Operation {
    pub const fn path(self) -> &'static str {
        match self {
            // The four closed evidence tags share the accepted Platform endpoint.
            // Operation identity remains bound by the existing request/response codec.
            Self::ReadAccountSecurityV1
            | Self::ReadFreshSigningTrustV1
            | Self::ReadRecoveryAccountSecurityV2
            | Self::ReadRecoverySigningTrustV2 => "/internal/v1/game-auth/native-evidence",
            Self::ReadCharacterBootstrapIntentV1 => {
                "/internal/v1/game-auth/character-bootstrap-intents/read"
            }
            Self::ReadPremiumSnapshotV1 => "/v1/premium/snapshot",
            Self::ReportRuntimeStatusV1 => "/internal/v1/game-auth/native-runtime-status",
            Self::PublishAccountCharactersV1 => "/internal/v1/game-auth/native-account-characters",
            Self::PublishProjectionWatermarkV1 => {
                "/internal/v1/game-auth/native-account-characters/watermark"
            }
            Self::ReportScopeAssignmentV1 => super::scope_assignment::PATH,
            Self::ReportScopeRevocationV1 => super::scope_assignment::REVOCATION_PATH,
            Self::ListPendingCharacterCommandsV1 => super::character_commands::PENDING_PATH,
            Self::ReadCharacterCommandIntentV1 => super::character_commands::READ_PATH,
            Self::PublishCharacterCommandReceiptV1
            | Self::PublishAccountClosureCommandReceiptV2 => {
                super::character_commands::RECEIPT_PATH
            }
            Self::ReadAccountClosureLifecycleV1 => super::account_closure::LIFECYCLE_PATH,
            Self::PublishAccountClosureV1 => super::account_closure::PATH,
            Self::PublishAccountClosureWatermarkV1 => super::account_closure::WATERMARK_PATH,
            Self::ListPendingStoreOperationsV1 => super::store::PENDING_PATH,
            Self::ReadStoreOperationIntentV1 => super::store::READ_PATH,
            Self::PublishStoreDeliveryReceiptV1 => super::store::RECEIPT_PATH,
            Self::PurchaseStoreOfferV1 => super::store::PURCHASE_PATH,
            Self::ReadStoreBalanceV1 => super::store::BALANCE_PATH,
            Self::ReadStoreHistoryPageV1 => super::store::HISTORY_PATH,
            Self::ListStoreOperationStatesV1 => super::store::RECONCILE_PATH,
            Self::PublishWorldHighscoresV1 => super::public_projections::HIGHSCORES_PATH,
            Self::PublishCharacterProfileV1 => super::public_projections::PROFILE_PATH,
            Self::PublishWorldOnlineV1 => super::public_projections::ONLINE_PATH,
            Self::PublishWorldTotalsV1 => super::public_projections::world_totals::PATH,
            Self::PublishCharacterDeathsV2 => "/internal/v2/game-public/character-deaths",
            Self::PublishGuildV2 => "/internal/v2/game-public/guild",
            Self::PublishHouseV2 => "/internal/v2/game-public/house",
            Self::PublishWorldDailyKillsV2 => "/internal/v2/game-public/world-daily-kills",
            Self::PublishCharacterOnlineSessionV2 => {
                "/internal/v2/game-public/character-online-session"
            }
            Self::PublishPublicProjectionWatermarkV2 => "/internal/v2/game-public/watermark",
            Self::PublishPublicProjectionWatermarkV1 => super::public_projections::WATERMARK_PATH,
            Self::PublishGameRestoreNoticeV1 => super::restore_notice::PATH,
            Self::ListPendingBazaarCommandsV1 => "/internal/v1/bazaar/commands/pending",
            Self::ReadBazaarCommandIntentV1 => "/internal/v1/bazaar/commands/read",
            Self::ListBazaarCommandStatesV1 => "/internal/v1/bazaar/commands/reconcile",
            Self::PublishBazaarTransferabilityV1 => "/internal/v1/bazaar/transferability",
            Self::PublishBazaarListingStatusV1 => "/internal/v1/bazaar/status",
            Self::PublishBazaarWatermarkV1 => "/internal/v1/bazaar/watermark",
            Self::PublishBazaarCharacterCommandReceiptV2 => super::character_commands::RECEIPT_PATH,
            Self::PublishWorldEventOutcomeV1 => "/internal/v1/game-public/world-event-outcome",
            Self::PublishGuildSeasonV1 => "/internal/v1/game-public/guild-season",
            Self::PublishLeagueWatermarkV1 => "/internal/v1/game-public/league-watermark",
        }
    }
    /// Request body cap: `NSRC-HTTP-REQUEST-BODY-BYTES`, or `NRS-REPORT-BYTES`
    /// for the runtime-status report, `LCA-REQUEST-BYTES` and
    /// `LCA-WATERMARK-BYTES` for the account-characters projection, and the
    /// `CHARCMD-*-REQUEST-BYTES` bounds for the Character commands, and the
    /// `PUBPROJ-*-REQUEST-BYTES` and `PUBPROJ-WATERMARK-BYTES` bounds for the
    /// public projections.
    pub const fn request_bytes_max(self) -> usize {
        match self {
            Self::ReadPremiumSnapshotV1 => 256,
            Self::ListPendingBazaarCommandsV1
            | Self::ReadBazaarCommandIntentV1
            | Self::ListBazaarCommandStatesV1
            | Self::PublishBazaarWatermarkV1 => 512,
            Self::PublishBazaarTransferabilityV1 => super::bazaar::transport::TRANSFERABILITY_BYTES,
            Self::PublishBazaarListingStatusV1 => super::bazaar::transport::STATUS_BYTES,
            Self::PublishBazaarCharacterCommandReceiptV2 => super::bazaar::transport::RECEIPT_BYTES,
            Self::ReportRuntimeStatusV1 => super::runtime_status::REPORT_BYTES,
            Self::PublishAccountCharactersV1 => super::account_characters::SNAPSHOT_BYTES,
            Self::PublishProjectionWatermarkV1 => super::account_characters::WATERMARK_BYTES,
            Self::ReportScopeAssignmentV1 | Self::ReportScopeRevocationV1 => {
                super::scope_assignment::REPORT_BYTES
            }
            Self::ListPendingCharacterCommandsV1 => super::character_commands::LIST_REQUEST_BYTES,
            Self::ReadCharacterCommandIntentV1 => super::character_commands::READ_REQUEST_BYTES,
            Self::PublishCharacterCommandReceiptV1 => {
                super::character_commands::RECEIPT_REQUEST_BYTES
            }
            Self::PublishAccountClosureCommandReceiptV2 => super::account_closure::RECEIPT_BYTES,
            Self::ReadAccountClosureLifecycleV1 => 512,
            Self::PublishAccountClosureV1 => super::account_closure::SNAPSHOT_BYTES,
            Self::PublishAccountClosureWatermarkV1 => super::account_closure::WATERMARK_BYTES,
            Self::ListPendingStoreOperationsV1
            | Self::ReadStoreOperationIntentV1
            | Self::ReadStoreBalanceV1
            | Self::ListStoreOperationStatesV1
            | Self::ReadStoreHistoryPageV1 => super::store::READ_REQUEST_BYTES,
            Self::PublishStoreDeliveryReceiptV1 => super::store::RECEIPT_REQUEST_BYTES,
            Self::PurchaseStoreOfferV1 => super::store::PURCHASE_REQUEST_BYTES,
            Self::PublishWorldHighscoresV1 => super::public_projections::HIGHSCORES_BYTES,
            Self::PublishCharacterProfileV1 => super::public_projections::PROFILE_BYTES,
            Self::PublishWorldOnlineV1 => super::public_projections::ONLINE_BYTES,
            Self::PublishWorldTotalsV1 => super::public_projections::world_totals::REQUEST_BYTES,
            Self::PublishCharacterDeathsV2 => 32_768,
            Self::PublishGuildV2 => 393_216,
            Self::PublishHouseV2 => 2_048,
            Self::PublishWorldDailyKillsV2 => 2_048,
            Self::PublishCharacterOnlineSessionV2 => 2_048,
            Self::PublishPublicProjectionWatermarkV2 => 512,
            Self::PublishPublicProjectionWatermarkV1 => super::public_projections::WATERMARK_BYTES,
            Self::PublishGameRestoreNoticeV1 => super::restore_notice::NOTICE_BYTES,
            Self::PublishWorldEventOutcomeV1 => super::world_event_outcomes::EVENT_BYTES,
            Self::PublishGuildSeasonV1 => super::world_event_outcomes::SEASON_BYTES,
            Self::PublishLeagueWatermarkV1 => super::world_event_outcomes::WATERMARK_BYTES,
            Self::ReadAccountSecurityV1
            | Self::ReadFreshSigningTrustV1
            | Self::ReadRecoveryAccountSecurityV2
            | Self::ReadRecoverySigningTrustV2
            | Self::ReadCharacterBootstrapIntentV1 => 1024,
        }
    }
}
#[derive(Clone)]
pub struct ProducerDescriptor {
    pub(super) source_authority: String,
    pub(super) connect_addr: std::net::SocketAddr,
    pub(super) server_name: ServerName<'static>,
    pub(super) host_header: String,
    pub(super) tls: Arc<rustls::ClientConfig>,
}
impl ProducerDescriptor {
    /// The configured source authority this descriptor authenticates.
    #[must_use]
    pub fn source_authority(&self) -> &str {
        &self.source_authority
    }

    pub fn new(
        source_authority: String,
        connect_endpoint: (String, u16),
        server_name: String,
        host_header: String,
        roots: Vec<CertificateDer<'static>>,
        client_chain: Vec<CertificateDer<'static>>,
        client_key: PrivateKeyDer<'static>,
    ) -> Result<Self, SourceError> {
        let (connect_host, port) = connect_endpoint;
        if source_authority.is_empty()
            || source_authority.len() > 128
            || !source_authority
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"._:/-".contains(&b))
            || server_name.len() > 128
            || format!("https://{connect_host}:{port}").len() > 256
            || connect_host.is_empty()
            || connect_host.len() > 253
            || host_header != server_name
            || port == 0
            || roots.is_empty()
            || roots.len() > TRUST_ROOTS_MAX
        {
            return Err(SourceError::InvalidDescriptor);
        }
        let connect_addr = std::net::SocketAddr::new(
            connect_host
                .parse::<std::net::IpAddr>()
                .map_err(|_| SourceError::InvalidDescriptor)?,
            port,
        );
        let total = roots
            .iter()
            .try_fold(0usize, |n, c| n.checked_add(c.as_ref().len()))
            .ok_or(SourceError::InvalidDescriptor)?;
        if total > TRUST_ROOT_DER_AGGREGATE_MAX
            || roots.iter().any(|c| c.as_ref().len() > TRUST_ROOT_DER_MAX)
        {
            return Err(SourceError::InvalidDescriptor);
        }
        let mut store = RootCertStore::empty();
        for root in roots {
            store
                .add(root)
                .map_err(|_| SourceError::InvalidDescriptor)?
        }
        let provider = Arc::new(rustls::crypto::aws_lc_rs::default_provider());
        let inner = WebPkiServerVerifier::builder_with_provider(Arc::new(store), provider.clone())
            .build()
            .map_err(|_| SourceError::InvalidDescriptor)?;
        let verifier = Arc::new(BoundedServerVerifier { inner });
        let mut tls = rustls::ClientConfig::builder_with_provider(provider)
            .with_protocol_versions(&[&rustls::version::TLS13])
            .map_err(|_| SourceError::InvalidDescriptor)?
            .dangerous()
            .with_custom_certificate_verifier(verifier)
            .with_client_auth_cert(client_chain, client_key)
            .map_err(|_| SourceError::InvalidDescriptor)?;
        tls.alpn_protocols = vec![b"http/1.1".to_vec()];
        Ok(Self {
            source_authority,
            connect_addr,
            server_name: ServerName::try_from(server_name)
                .map_err(|_| SourceError::InvalidDescriptor)?,
            host_header,
            tls: Arc::new(tls),
        })
    }
}
#[derive(Debug)]
pub struct BoundedServerVerifier {
    inner: Arc<WebPkiServerVerifier>,
}
impl ServerCertVerifier for BoundedServerVerifier {
    fn verify_server_cert(
        &self,
        end: &CertificateDer<'_>,
        inter: &[CertificateDer<'_>],
        name: &ServerName<'_>,
        ocsp: &[u8],
        now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        validate_peer_lengths(std::iter::once(end).chain(inter).map(|c| c.as_ref().len()))?;
        self.inner.verify_server_cert(end, inter, name, ocsp, now)
    }
    fn verify_tls12_signature(
        &self,
        m: &[u8],
        c: &CertificateDer<'_>,
        d: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        self.inner.verify_tls12_signature(m, c, d)
    }
    fn verify_tls13_signature(
        &self,
        m: &[u8],
        c: &CertificateDer<'_>,
        d: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        self.inner.verify_tls13_signature(m, c, d)
    }
    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.inner.supported_verify_schemes()
    }
}
fn limit_error() -> rustls::Error {
    rustls::Error::InvalidCertificate(rustls::CertificateError::ApplicationVerificationFailure)
}

fn validate_peer_lengths(lengths: impl IntoIterator<Item = usize>) -> Result<(), rustls::Error> {
    let mut count = 0usize;
    let mut total = 0usize;
    for length in lengths {
        count = count.checked_add(1).ok_or_else(limit_error)?;
        total = total.checked_add(length).ok_or_else(limit_error)?;
        if count > PEER_CERTIFICATES_MAX
            || length > PEER_CERT_DER_MAX
            || total > PEER_CERT_DER_AGGREGATE_MAX
        {
            return Err(limit_error());
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn peer_numeric_caps_remain_coupled() {
        assert!(validate_peer_lengths([4096; 4]).is_ok());
        assert!(validate_peer_lengths([4097]).is_err());
        assert!(validate_peer_lengths([4096, 4096, 4096, 4097]).is_err());
        assert!(validate_peer_lengths([1; 5]).is_err());
        assert!(validate_peer_lengths([1, usize::MAX]).is_err());
    }
}
