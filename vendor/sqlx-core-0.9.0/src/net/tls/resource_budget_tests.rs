use crate::net::resource_budget::{BudgetError, ResourceBudget, ResourceReservation};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

#[derive(Debug)]
struct Ledger {
    limit: usize,
    used: AtomicUsize,
    provider_shared_debits: Mutex<Vec<usize>>,
}
impl ResourceBudget for Ledger {
    fn try_reserve(&self, bytes: usize) -> Result<(), BudgetError> {
        self.used
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |used| {
                used.checked_add(bytes).filter(|next| *next <= self.limit)
            })
            .map(|_| ())
            .map_err(|_| BudgetError::Unavailable)
    }
    fn release(&self, bytes: usize) {
        let prior = self.used.fetch_sub(bytes, Ordering::AcqRel);
        assert!(prior >= bytes, "reservation released twice");
    }
    fn try_reserve_provider_shared(&self, bytes: usize) -> Result<(), BudgetError> {
        self.try_reserve(bytes)?;
        self.provider_shared_debits.lock().unwrap().push(bytes);
        Ok(())
    }
}
fn ledger(limit: usize) -> Arc<Ledger> {
    Arc::new(Ledger {
        limit,
        used: AtomicUsize::new(0),
        provider_shared_debits: Mutex::new(Vec::new()),
    })
}

#[test]
fn provider_shared_default_denies_and_adapter_delegates_to_same_root() {
    #[derive(Debug)]
    struct Unsupported;
    impl ResourceBudget for Unsupported {
        fn try_reserve(&self, _bytes: usize) -> Result<(), BudgetError> { Ok(()) }
        fn release(&self, _bytes: usize) {}
    }
    assert_eq!(
        Unsupported.try_reserve_provider_shared(1),
        Err(BudgetError::Unavailable)
    );

    let budget = ledger(10);
    let owner = super::tls_rustls::DeframerBudgetOwner(budget.clone());
    rustls::DeframerBufferOwner::try_reserve_provider_shared(&owner, 10).unwrap();
    assert_eq!(budget.used.load(Ordering::Acquire), 10);
    assert!(rustls::DeframerBufferOwner::try_reserve_provider_shared(&owner, 1).is_err());
}

#[cfg(feature = "_tls-rustls-aws-lc-rs")]
#[test]
fn aws_lc_kx_full_lifetime_bounds_and_returned_secrets() {
    use super::tls_rustls::DeframerBudgetOwner;
    use rustls::crypto::{ResourceOwnedKx, SupportedKxGroup};

    // This is inline caller/control state, not another provider heap object.
    // The provider allocation layouts are separately compile-time asserted in
    // their private modules at 200 (AWS-LC classical), 40 (ML-KEM), and 96
    // (hybrid). The same classical source also pins ring's 208-byte layout.
    assert_eq!(core::mem::size_of::<ResourceOwnedKx>(), 40);

    let process = ledger(1_000_000);
    let process_owner: Arc<dyn rustls::DeframerBufferOwner> =
        Arc::new(DeframerBudgetOwner(process.clone()));
    // This test is run by its exact name in a fresh test process. Race the
    // first provider use before any provider can be cached in that process.
    let threads = (0..4)
        .map(|_| {
            let owner = process_owner.clone();
            std::thread::spawn(move || {
                rustls::crypto::aws_lc_rs::default_provider_with_resource_owner(owner).unwrap()
            })
        })
        .collect::<Vec<_>>();
    let providers = threads
        .into_iter()
        .map(|thread| thread.join().unwrap())
        .collect::<Vec<_>>();
    let provider = &providers[0];
    assert!(providers
        .iter()
        .all(|candidate| Arc::ptr_eq(provider, candidate)));
    let main_thread_provider =
        rustls::crypto::aws_lc_rs::default_provider_with_resource_owner(process_owner).unwrap();
    assert!(Arc::ptr_eq(provider, &main_thread_provider));
    assert_eq!(provider.cipher_suites.capacity(), provider.cipher_suites.len());
    assert_eq!(provider.kx_groups.capacity(), provider.kx_groups.len());
    let ordinary = rustls::crypto::aws_lc_rs::default_provider();
    let expected = [
        rustls::NamedGroup::X25519MLKEM768,
        rustls::NamedGroup::X25519,
        rustls::NamedGroup::secp256r1,
        rustls::NamedGroup::secp384r1,
    ];
    assert_eq!(provider.kx_groups.len(), expected.len());
    assert_eq!(ordinary.kx_groups.len(), expected.len());
    assert!(provider
        .kx_groups
        .iter()
        .zip(expected)
        .all(|(group, expected)| group.name() == expected));
    assert!(ordinary
        .kx_groups
        .iter()
        .zip(expected)
        .all(|(group, expected)| group.name() == expected));
    assert_eq!(
        provider
            .kx_groups
            .iter()
            .map(|group| group.name())
            .collect::<Vec<_>>(),
        ordinary
            .kx_groups
            .iter()
            .map(|group| group.name())
            .collect::<Vec<_>>()
    );
    let (arc_inner, _) = core::alloc::Layout::new::<[AtomicUsize; 2]>()
        .extend(core::alloc::Layout::new::<rustls::crypto::CryptoProvider>())
        .unwrap();
    let provider_config = provider.cipher_suites.len()
        * core::mem::size_of::<rustls::SupportedCipherSuite>()
        + provider.kx_groups.len()
            * core::mem::size_of::<&'static dyn rustls::crypto::SupportedKxGroup>()
        + arc_inner.pad_to_align().size();
    let debits = process.provider_shared_debits.lock().unwrap();
    assert_eq!(debits.iter().filter(|debit| **debit == 1360).count(), 5);
    assert_eq!(
        debits
            .iter()
            .filter(|debit| **debit == provider_config)
            .count(),
        1
    );
    assert_eq!(
        debits.len(),
        7,
        "one process, five thread, one config debit"
    );
    drop(debits);

    let groups: [(&dyn SupportedKxGroup, usize); 5] = [
        (rustls::crypto::aws_lc_rs::kx_group::X25519, 554),
        (rustls::crypto::aws_lc_rs::kx_group::SECP256R1, 1_625),
        (rustls::crypto::aws_lc_rs::kx_group::SECP384R1, 1_705),
        (rustls::crypto::aws_lc_rs::kx_group::MLKEM768, 6_264),
        (rustls::crypto::aws_lc_rs::kx_group::X25519MLKEM768, 7_881),
    ];

    for (group, bound) in groups {
        let denied = ledger(bound - 1);
        let owner: Arc<dyn rustls::DeframerBufferOwner> =
            Arc::new(DeframerBudgetOwner(denied.clone()));
        assert!(group.start_with_resource_owner(owner).is_err());
        assert_eq!(denied.used.load(Ordering::Acquire), 0);

        let funded = ledger(bound);
        let owner: Arc<dyn rustls::DeframerBufferOwner> =
            Arc::new(DeframerBudgetOwner(funded.clone()));
        let active = group.start_with_resource_owner(owner).unwrap();
        assert_eq!(funded.used.load(Ordering::Acquire), bound);
        let completed = group.start_and_complete(active.pub_key()).unwrap();
        let secret = active.complete(&completed.pub_key).unwrap();
        assert_eq!(secret.secret_bytes(), completed.secret.secret_bytes());
        assert_eq!(funded.used.load(Ordering::Acquire), bound);
        drop(secret);
        assert_eq!(funded.used.load(Ordering::Acquire), 0);
    }

    let funded = ledger(7_881);
    let owner: Arc<dyn rustls::DeframerBufferOwner> = Arc::new(DeframerBudgetOwner(funded.clone()));
    let hybrid = rustls::crypto::aws_lc_rs::kx_group::X25519MLKEM768
        .start_with_resource_owner(owner)
        .unwrap();
    let (_, component_public) = hybrid.hybrid_component().unwrap();
    let peer = rustls::crypto::aws_lc_rs::kx_group::X25519.start().unwrap();
    let peer_public = peer.pub_key().to_vec();
    let expected = peer.complete(component_public).unwrap();
    let secret = hybrid.complete_hybrid_component(&peer_public).unwrap();
    assert_eq!(secret.secret_bytes(), expected.secret_bytes());
    assert_eq!(funded.used.load(Ordering::Acquire), 7_881);
    drop(expected);
    drop(secret);
    assert_eq!(funded.used.load(Ordering::Acquire), 0);

    let hrr = ledger(7_881 + 1_625);
    let owner: Arc<dyn rustls::DeframerBufferOwner> = Arc::new(DeframerBudgetOwner(hrr.clone()));
    let initial = rustls::crypto::aws_lc_rs::kx_group::X25519MLKEM768
        .start_with_resource_owner(owner.clone())
        .unwrap();
    let replacement = rustls::crypto::aws_lc_rs::kx_group::SECP256R1
        .start_with_resource_owner(owner)
        .unwrap();
    assert_eq!(hrr.used.load(Ordering::Acquire), 7_881 + 1_625);
    drop(initial);
    assert_eq!(hrr.used.load(Ordering::Acquire), 1_625);
    drop(replacement);
    assert_eq!(hrr.used.load(Ordering::Acquire), 0);

    let failed = ledger(554);
    let owner: Arc<dyn rustls::DeframerBufferOwner> = Arc::new(DeframerBudgetOwner(failed.clone()));
    let active = rustls::crypto::aws_lc_rs::kx_group::X25519
        .start_with_resource_owner(owner)
        .unwrap();
    assert!(active.complete(&[]).is_err());
    assert_eq!(failed.used.load(Ordering::Acquire), 0);
}

#[cfg(feature = "_tls-rustls-aws-lc-rs")]
#[test]
fn unsupported_custom_kx_fails_before_ordinary_start() {
    use rustls::crypto::{ActiveKeyExchange, SupportedKxGroup};
    #[derive(Debug)]
    struct Unsupported(AtomicUsize);
    impl SupportedKxGroup for Unsupported {
        fn start(&self) -> Result<Box<dyn ActiveKeyExchange>, rustls::Error> {
            self.0.fetch_add(1, Ordering::Relaxed);
            Err(rustls::Error::FailedToGetRandomBytes)
        }
        fn name(&self) -> rustls::NamedGroup { rustls::NamedGroup::Unknown(0xff01) }
    }
    let group = Unsupported(AtomicUsize::new(0));
    let owner: Arc<dyn rustls::DeframerBufferOwner> =
        Arc::new(super::tls_rustls::DeframerBudgetOwner(ledger(usize::MAX)));
    assert!(group.start_with_resource_owner(owner).is_err());
    assert_eq!(group.0.load(Ordering::Relaxed), 0);
}

#[test]
fn denial_precedes_input_controlled_allocation() {
    let budget = ledger(8);
    let held = ResourceReservation::try_new(budget.clone(), 8).unwrap();
    let calls = AtomicUsize::new(0);
    let result = ResourceReservation::try_new(budget.clone(), 1).map(|reservation| {
        calls.fetch_add(1, Ordering::Relaxed);
        reservation.bind(vec![0u8; 1])
    });
    assert!(matches!(result, Err(BudgetError::Unavailable)));
    assert_eq!(calls.load(Ordering::Relaxed), 0);
    assert_eq!(budget.used.load(Ordering::Acquire), 8);
    drop(held);
    assert_eq!(budget.used.load(Ordering::Acquire), 0);
}

#[test]
fn reservation_uses_same_owner_balance_without_extra_driver_capacity() {
    let budget = ledger(4_194_304);
    let request = ResourceReservation::try_new(budget.clone(), 524_288).unwrap();
    let driver = ResourceReservation::try_new(budget.clone(), 3_670_016).unwrap();
    assert!(ResourceReservation::try_new(budget.clone(), 1).is_err());
    drop(driver);
    assert_eq!(budget.used.load(Ordering::Acquire), 524_288);
    drop(request);
    assert_eq!(budget.used.load(Ordering::Acquire), 0);
}

#[test]
fn growth_denial_and_overflow_leave_original_custody_unchanged() {
    let budget = ledger(64);
    let mut charge = ResourceReservation::try_new(budget.clone(), 48).unwrap();
    assert_eq!(charge.try_grow(17), Err(BudgetError::Unavailable));
    assert_eq!(charge.try_grow(usize::MAX), Err(BudgetError::Overflow));
    assert_eq!(charge.bytes(), 48);
    assert_eq!(budget.used.load(Ordering::Acquire), 48);
    charge.try_grow(16).unwrap();
    assert_eq!(charge.bytes(), 64);
}

#[test]
fn splitting_transfers_without_minting_capacity_and_failed_split_is_inert() {
    let budget = ledger(100);
    let mut charge = ResourceReservation::try_new(budget.clone(), 100).unwrap();
    assert!(charge.split_off(101).is_err());
    let retained = charge.split_off(70).unwrap();
    assert_eq!(charge.bytes(), 30);
    assert_eq!(retained.bytes(), 70);
    assert_eq!(budget.used.load(Ordering::Acquire), 100);
    drop(charge);
    assert_eq!(budget.used.load(Ordering::Acquire), 70);
    drop(retained);
    assert_eq!(budget.used.load(Ordering::Acquire), 0);
}

#[test]
fn backing_is_dropped_before_its_reservation_is_released() {
    struct Backing(Arc<Ledger>);
    impl Drop for Backing {
        fn drop(&mut self) {
            assert_eq!(self.0.used.load(Ordering::Acquire), 48);
        }
    }
    let budget = ledger(48);
    let retained = ResourceReservation::try_new(budget.clone(), 48)
        .unwrap()
        .bind(Backing(budget.clone()));
    assert_eq!(retained.charged_bytes(), 48);
    let _ = retained.get();
    drop(retained);
    assert_eq!(budget.used.load(Ordering::Acquire), 0);
}

#[test]
fn connection_style_shared_backing_remains_charged_until_last_clone_drops() {
    let budget = ledger(786_432);
    let held = ResourceReservation::try_new(budget.clone(), 786_432)
        .unwrap()
        .bind([0u8; 0]);
    // The Arc itself is intentionally fixture-owned; a production Arc allocation needs its own charge.
    let first = Arc::new(held);
    let second = first.clone();
    drop(first);
    assert_eq!(budget.used.load(Ordering::Acquire), 786_432);
    drop(second);
    assert_eq!(budget.used.load(Ordering::Acquire), 0);
}

#[test]
fn concurrent_callers_cannot_exceed_shared_owner_balance() {
    use std::sync::Barrier;
    let budget = ledger(64);
    let barrier = Arc::new(Barrier::new(9));
    let handles: Vec<_> = (0..8)
        .map(|_| {
            let budget = budget.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                let reservation = ResourceReservation::try_new(budget, 16).ok();
                barrier.wait();
                barrier.wait();
                reservation.is_some()
            })
        })
        .collect();
    barrier.wait();
    assert_eq!(budget.used.load(Ordering::Acquire), 64);
    barrier.wait();
    assert_eq!(
        handles
            .into_iter()
            .map(|h| usize::from(h.join().unwrap()))
            .sum::<usize>(),
        4
    );
    assert_eq!(budget.used.load(Ordering::Acquire), 0);
}

#[test]
fn cancelled_pending_work_keeps_charge_until_captured_backing_drops() {
    use std::future::{pending, Future};
    use std::task::{Context, Poll, Waker};
    let budget = ledger(48);
    let retained = ResourceReservation::try_new(budget.clone(), 48)
        .unwrap()
        .bind(vec![0u8; 48]);
    let mut work = Box::pin(async move {
        pending::<()>().await;
        retained
    });
    let mut context = Context::from_waker(Waker::noop());
    assert!(matches!(work.as_mut().poll(&mut context), Poll::Pending));
    assert_eq!(budget.used.load(Ordering::Acquire), 48);
    drop(work);
    assert_eq!(budget.used.load(Ordering::Acquire), 0);
}

#[test]
fn debugging_custody_never_formats_the_retained_input() {
    struct Sensitive;
    impl std::fmt::Debug for Sensitive {
        fn fmt(&self, _: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            panic!("must not format protected backing")
        }
    }
    let retained = ResourceReservation::try_new(ledger(8), 8)
        .unwrap()
        .bind(Sensitive);
    assert_eq!(format!("{retained:?}"), "Charged { charged_bytes: 8 }");
}

#[cfg(feature = "_tls-rustls-ring-webpki")]
#[test]
fn pinned_tls_decoder_bound_covers_correlated_nested_ech_growth() {
    use super::tls_rustls::handshake_decode_heap_bound;
    use rustls::internal::msgs::{codec::Codec, handshake::EchConfigPayload};
    let count = 8193usize;
    let mut wire = Vec::with_capacity(2 + 4 * count);
    wire.extend_from_slice(&u16::try_from(4 * count).unwrap().to_be_bytes());
    for _ in 0..count {
        wire.extend_from_slice(&[0, 1, 0, 0]);
    }
    let decoded = Vec::<EchConfigPayload>::read_bytes(&wire).unwrap();
    assert_eq!(decoded.capacity(), 16384);
    let moving_outer = (8192 + decoded.capacity()) * size_of::<EchConfigPayload>();
    assert!(handshake_decode_heap_bound().unwrap() >= moving_outer);
    // Independent mixed known/unknown public-decoder allocation witness, with
    // 8,180 nested extension entries in one of 8,193 ECH configurations.
    assert!(handshake_decode_heap_bound().unwrap() >= 3_014_690);
}

#[cfg(feature = "_tls-rustls-ring-webpki")]
#[test]
fn tls_decoder_reservation_uses_actual_remaining_owner_balance() {
    use super::tls_rustls::handshake_decode_heap_bound;
    let estimate = handshake_decode_heap_bound().unwrap();
    let budget = ledger(4_194_304);
    let request = ResourceReservation::try_new(budget.clone(), 65_536).unwrap();
    let decoder = ResourceReservation::try_new(budget.clone(), estimate).unwrap();
    assert_eq!(
        budget.used.load(Ordering::Acquire),
        request.bytes() + estimate
    );
    drop(decoder);
    let remainder =
        ResourceReservation::try_new(budget.clone(), 4_194_304 - 65_536 - estimate + 1).unwrap();
    assert_eq!(
        ResourceReservation::try_new(budget.clone(), estimate).unwrap_err(),
        BudgetError::Unavailable
    );
    assert_eq!(
        budget.used.load(Ordering::Acquire),
        request.bytes() + remainder.bytes()
    );
}

#[cfg(feature = "_tls-rustls-ring-webpki")]
#[test]
fn private_handshake_chain_bound_tracks_input_without_a_lifetime_cap() {
    use super::tls_rustls::HandshakeInputBound;
    let mut bound = HandshakeInputBound::default();
    assert_eq!(bound.peer_chain_heap_bound().unwrap(), 0);
    bound.add_read(4096);
    let small = bound.peer_chain_heap_bound().unwrap();
    assert!(small >= 4096);
    bound.add_read(usize::MAX);
    let full = bound.peer_chain_heap_bound().unwrap();
    assert!(full >= small);
    // This is a saturated mathematical bound from rustls' existing message
    // maximum, not an ever-growing lifetime counter or a rejection threshold.
    for _ in 0..128 {
        bound.add_read(usize::MAX);
    }
    assert_eq!(bound.peer_chain_heap_bound().unwrap(), full);
}

#[test]
fn certificate_reader_charges_growth_overlap_and_retains_result_custody() {
    use super::{read_certificate_data_accounted, CertificateReadError};
    let source = vec![7u8; 7000];
    let budget = ledger(16_384);
    let owned = read_certificate_data_accounted(&mut source.as_slice(), budget.clone()).unwrap();
    assert_eq!(owned.get().as_slice(), source);
    assert_eq!(owned.get().capacity(), 8192);
    assert_eq!(budget.used.load(Ordering::Acquire), owned.charged_bytes());
    let queued = Some(owned);
    assert_eq!(budget.used.load(Ordering::Acquire), 8192);
    drop(queued);
    assert_eq!(budget.used.load(Ordering::Acquire), 0);

    let smaller = ledger(16_383);
    assert!(matches!(
        read_certificate_data_accounted(&mut source.as_slice(), smaller.clone()),
        Err(CertificateReadError::Budget(BudgetError::Unavailable))
    ));
    assert_eq!(smaller.used.load(Ordering::Acquire), 0);
}

#[cfg(target_os = "linux")]
#[test]
fn certificate_file_loader_returns_complete_data_under_the_same_ledger() {
    use super::read_certificate_file_accounted;
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let path = std::env::temp_dir().join(format!(
        "oteryn351-cert-data-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    struct Cleanup(std::path::PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }
    let cleanup = Cleanup(path);
    let data = vec![9u8; 7000];
    std::fs::write(&cleanup.0, &data).unwrap();
    let budget = ledger(16_384);
    let result = read_certificate_file_accounted(&cleanup.0, budget.clone()).unwrap();
    assert_eq!(result.get().as_slice(), data);
    assert_eq!(budget.used.load(Ordering::Acquire), result.get().capacity());
    drop(result);
    assert_eq!(budget.used.load(Ordering::Acquire), 0);
}

#[cfg(all(target_os = "linux", feature = "_rt-tokio"))]
#[test]
fn owned_certificate_loader_is_funded_or_denied_without_fallback() {
    use super::read_certificate_file_owned;
    use crate::rt::resource_owner::blocking_job_owner;
    let path = std::env::temp_dir().join(format!("oteryn351-owned-cert-{}", std::process::id()));
    struct Cleanup(std::path::PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }
    let cleanup = Cleanup(path);
    let data = vec![11u8; 7000];
    std::fs::write(&cleanup.0, &data).unwrap();

    let denied = ledger(cleanup.0.as_os_str().len());
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    let denied_owner = blocking_job_owner(denied.clone());
    assert!(runtime
        .block_on(read_certificate_file_owned(
            &cleanup.0,
            &denied_owner
        ))
        .is_err());
    assert_eq!(denied.used.load(Ordering::Acquire), 0);

    let funded = ledger(4 * 1024 * 1024);
    let funded_owner = blocking_job_owner(funded.clone());
    let loaded = runtime
        .block_on(read_certificate_file_owned(
            &cleanup.0,
            &funded_owner,
        ))
        .unwrap();
    assert_eq!(loaded.get().as_slice(), data);
    drop(loaded);
    let loaded_again = runtime
        .block_on(read_certificate_file_owned(
            &cleanup.0,
            &funded_owner,
        ))
        .unwrap();
    assert_eq!(loaded_again.get().as_slice(), data);
    drop(loaded_again);
    assert!(
        funded.used.load(Ordering::Acquire) > 0,
        "owned pool remains charged"
    );
    drop(runtime);
    assert_eq!(funded.used.load(Ordering::Acquire), 0);
}

#[cfg(any(
    feature = "_tls-rustls-ring-webpki",
    feature = "_tls-rustls-ring-native-roots"
))]
#[test]
fn rustls_deframer_owner_uses_the_operation_ledger() {
    use super::tls_rustls::DeframerBudgetOwner;
    use std::io::{self, Read};
    use std::sync::Arc;

    struct WouldBlock;
    impl Read for WouldBlock {
        fn read(&mut self, _buf: &mut [u8]) -> io::Result<usize> {
            Err(io::ErrorKind::WouldBlock.into())
        }
    }

    let budget = ledger(4096);
    let owner: Arc<dyn rustls::DeframerBufferOwner> = Arc::new(DeframerBudgetOwner(budget.clone()));
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let config = rustls::ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .unwrap()
        .with_root_certificates(rustls::RootCertStore::empty())
        .with_no_client_auth();
    let name = rustls::pki_types::ServerName::try_from("localhost").unwrap();
    let mut connection =
        rustls::ClientConnection::new_with_resource_owner(Arc::new(config), name, owner.clone())
            .unwrap();
    assert_eq!(Arc::strong_count(&owner), 3);
    drop(owner);

    let error = connection.read_tls(&mut WouldBlock).unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::WouldBlock);
    assert_eq!(budget.used.load(Ordering::Acquire), 4096);
    drop(connection);
    assert_eq!(budget.used.load(Ordering::Acquire), 0);

    let denied = ledger(1);
    let owner: Arc<dyn rustls::DeframerBufferOwner> = Arc::new(DeframerBudgetOwner(denied.clone()));
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let mut config = rustls::ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .unwrap()
        .with_root_certificates(rustls::RootCertStore::empty())
        .with_no_client_auth();
    config.alpn_protocols = vec![b"h2".to_vec()];
    let name = rustls::pki_types::ServerName::try_from("localhost").unwrap();
    let error =
        rustls::ClientConnection::new_with_resource_owner(Arc::new(config), name, owner)
            .unwrap_err();
    assert!(error.to_string().contains("resource budget unavailable"));
    assert_eq!(denied.used.load(Ordering::Acquire), 0);
}

#[cfg(feature = "_tls-rustls")]
#[test]
fn client_pem_parsing_borrows_charged_backing_without_a_second_copy() {
    const CERT: &[u8] = b"-----BEGIN CERTIFICATE-----\nMIIBfTCCASOgAwIBAgIUDZBk0JEdbOds6TsGRPkwvhxFUSMwCgYIKoZIzj0EAwIw\nFDESMBAGA1UEAwwJbG9jYWxob3N0MB4XDTI2MDkwODEyNTMxMFoXDTI2MDkwOTEy\nNTMxMFowFDESMBAGA1UEAwwJbG9jYWxob3N0MFkwEwYHKoZIzj0CAQYIKoZIzj0D\nAQcDQgAEy2WKazyI8TXUnJTbx1vSKqaJx8w+RW8JXa+v/pP7FzXgzntfmqjK9yh8\nm970RDgI5shoO5vx4GbStl1EgzFY1KNTMFEwHQYDVR0OBBYEFEOj0jJhWWip3SDz\n7NKpJwCjWRqhMB8GA1UdIwQYMBaAFEOj0jJhWWip3SDz7NKpJwCjWRqhMA8GA1Ud\nEwEB/wQFMAMBAf8wCgYIKoZIzj0EAwIDSAAwRQIhAPyz4a/hpM3FdPkGujIcZQp1\nw2Jgh0bjZ//2tCW0AMl4AiARhMbhcwqLnNlemlE2HQcfkezW3Zpjt75Zi8tXaGUM\nEQ==\n-----END CERTIFICATE-----\n";
    const KEY: &[u8] = b"-----BEGIN PRIVATE KEY-----\nMIGHAgEAMBMGByqGSM49AgEGCCqGSM49AwEHBG0wawIBAQQg/YnH3AP6GLGXmx4q\nIqRsG/wjKmRE+eY/UJuBHsRM4y6hRANCAATLZYprPIjxNdSclNvHW9IqponHzD5F\nbwldr6/+k/sXNeDOe1+aqMr3KHyb3vREOAjmyGg7m/HgZtK2XUSDMVjU\n-----END PRIVATE KEY-----\n";

    let budget = ledger(CERT.len() + KEY.len());
    let cert = ResourceReservation::try_new(budget.clone(), CERT.len())
        .unwrap()
        .bind(CERT.to_vec());
    let key = ResourceReservation::try_new(budget.clone(), KEY.len())
        .unwrap()
        .bind(KEY.to_vec());
    let used_before = budget.used.load(Ordering::Acquire);
    let cert_ptr = cert.get().as_ptr();
    let key_ptr = key.get().as_ptr();

    let (chain, private_key) =
        super::tls_rustls::client_auth_from_pem(cert.get(), key.get()).unwrap();

    assert_eq!(budget.used.load(Ordering::Acquire), used_before);
    assert_eq!(cert.get().as_ptr(), cert_ptr);
    assert_eq!(key.get().as_ptr(), key_ptr);
    assert_eq!(chain.len(), 1);
    assert!(matches!(private_key, rustls::pki_types::PrivateKeyDer::Pkcs8(_)));
    drop((chain, private_key));
    assert_eq!(budget.used.load(Ordering::Acquire), used_before);
    drop((cert, key));
    assert_eq!(budget.used.load(Ordering::Acquire), 0);

    let denied = ledger(CERT.len() + KEY.len() - 1);
    let held = ResourceReservation::try_new(denied.clone(), CERT.len())
        .unwrap()
        .bind(CERT.to_vec());
    assert!(matches!(
        ResourceReservation::try_new(denied.clone(), KEY.len()),
        Err(BudgetError::Unavailable)
    ));
    assert_eq!(denied.used.load(Ordering::Acquire), CERT.len());
    drop(held);
    assert_eq!(denied.used.load(Ordering::Acquire), 0);

    let malformed_budget = ledger(3);
    let malformed = ResourceReservation::try_new(malformed_budget.clone(), 3)
        .unwrap()
        .bind(b"bad".to_vec());
    assert!(super::tls_rustls::client_auth_from_pem(CERT, malformed.get()).is_err());
    assert_eq!(malformed_budget.used.load(Ordering::Acquire), 3);
    drop(malformed);
    assert_eq!(malformed_budget.used.load(Ordering::Acquire), 0);
}
