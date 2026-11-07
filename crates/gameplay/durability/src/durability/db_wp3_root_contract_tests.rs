use super::*;
use std::net::{IpAddr, Ipv4Addr};

fn config() -> Result<DurabilityRootConfig, DurabilityError> {
    DurabilityRootConfig::new_with_isolated_test_ledger(
        IpAddr::V4(Ipv4Addr::new(203, 0, 113, 7)),
        5432,
        "db.example",
        "oteryn",
        "explicit",
        "test-secret",
        b"-----BEGIN CERTIFICATE-----\nAA==\n-----END CERTIFICATE-----\n",
    )
}

#[test]
fn root_configuration_rejects_missing_identity_material_before_retention() {
    let ip = IpAddr::V4(Ipv4Addr::new(203, 0, 113, 7));
    let ca = b"-----BEGIN CERTIFICATE-----\nAA==\n-----END CERTIFICATE-----\n";

    assert!(matches!(
        DurabilityRootConfig::new(ip, 0, "db.example", "oteryn", "explicit", "secret", ca),
        Err(DurabilityError::InvalidConfiguration)
    ));
    assert!(matches!(
        DurabilityRootConfig::new(ip, 5432, "203.0.113.9", "oteryn", "explicit", "secret", ca),
        Err(DurabilityError::InvalidConfiguration)
    ));
    assert!(matches!(
        DurabilityRootConfig::new(ip, 5432, "db.example", " ", "explicit", "secret", ca),
        Err(DurabilityError::InvalidConfiguration)
    ));
    assert!(matches!(
        DurabilityRootConfig::new(ip, 5432, "db.example", "oteryn", " ", "secret", ca),
        Err(DurabilityError::InvalidConfiguration)
    ));
    assert!(matches!(
        DurabilityRootConfig::new(ip, 5432, "db.example", "oteryn", "explicit", "secret", &[]),
        Err(DurabilityError::InvalidConfiguration)
    ));
}

fn max_dns_server_name() -> String {
    format!(
        "{}.{}.{}.{}",
        "a".repeat(63),
        "b".repeat(63),
        "c".repeat(63),
        "d".repeat(61)
    )
}

fn pem_block(der_len: usize, newline: &str) -> Vec<u8> {
    let encoded = STANDARD.encode(vec![0x30; der_len]);
    let mut pem = String::from("-----BEGIN CERTIFICATE-----");
    pem.push_str(newline);
    for chunk in encoded.as_bytes().chunks(64) {
        for byte in chunk {
            pem.push(char::from(*byte));
        }
        pem.push_str(newline);
    }
    pem.push_str("-----END CERTIFICATE-----");
    pem.push_str(newline);
    pem.into_bytes()
}

fn pem_bundle(certificates: usize, der_len: usize, newline: &str) -> Vec<u8> {
    let block = pem_block(der_len, newline);
    let mut bundle = Vec::with_capacity(block.len() * certificates);
    for _ in 0..certificates {
        bundle.extend_from_slice(&block);
    }
    bundle
}

#[test]
fn retained_configuration_accepts_exact_protected_maxima_before_copy() {
    let ip = IpAddr::V4(Ipv4Addr::new(203, 0, 113, 7));
    let tls = max_dns_server_name();
    let database = "d".repeat(DATABASE_MAX_BYTES);
    let username = "u".repeat(USERNAME_MAX_BYTES);
    let password = "p".repeat(PASSWORD_MAX_BYTES);
    let root_ca = pem_bundle(ROOT_CA_MAX_CERTIFICATES, ROOT_CA_MAX_DER_BYTES, "\r\n");

    assert_eq!(tls.len(), TLS_SERVER_NAME_MAX_BYTES);
    assert_eq!(root_ca.len(), ROOT_CA_PEM_MAX_BYTES);
    assert!(matches!(
        validate_retained_config_input_lengths([
            tls.len(),
            database.len(),
            username.len(),
            password.len(),
            root_ca.len(),
        ]),
        Ok(RETAINED_CONFIG_MAX_BYTES)
    ));

    let config = DurabilityRootConfig::new_with_isolated_test_ledger(
        ip, 5432, &tls, &database, &username, &password, &root_ca,
    );
    assert!(config.is_ok());
    let Ok(config) = config else {
        return;
    };
    assert_eq!(config.tls_server_name.capacity(), tls.len());
    assert_eq!(config.database.capacity(), database.len());
    assert_eq!(config.username.capacity(), username.len());
    assert_eq!(config.password.capacity(), password.len());
    assert_eq!(config.root_ca_pem.capacity(), root_ca.len());
}

#[test]
fn retained_configuration_rejects_every_first_byte_above_as_unavailable() {
    let ip = IpAddr::V4(Ipv4Addr::new(203, 0, 113, 7));
    let valid_ca = pem_block(32, "\n");
    let tls_over = format!(
        "{}.{}.{}.{}",
        "a".repeat(63),
        "b".repeat(63),
        "c".repeat(63),
        "d".repeat(62)
    );
    let database_over = "d".repeat(DATABASE_MAX_BYTES + 1);
    let username_over = "u".repeat(USERNAME_MAX_BYTES + 1);
    let password_over = "p".repeat(PASSWORD_MAX_BYTES + 1);
    let mut root_ca_over = pem_bundle(ROOT_CA_MAX_CERTIFICATES, ROOT_CA_MAX_DER_BYTES, "\r\n");
    root_ca_over.push(b'X');

    assert_eq!(tls_over.len(), TLS_SERVER_NAME_MAX_BYTES + 1);
    assert_eq!(root_ca_over.len(), ROOT_CA_PEM_MAX_BYTES + 1);
    for result in [
        DurabilityRootConfig::new(ip, 5432, &tls_over, "d", "u", "p", &valid_ca),
        DurabilityRootConfig::new(ip, 5432, "a", &database_over, "u", "p", &valid_ca),
        DurabilityRootConfig::new(ip, 5432, "a", "d", &username_over, "p", &valid_ca),
        DurabilityRootConfig::new(ip, 5432, "a", "d", "u", &password_over, &valid_ca),
        DurabilityRootConfig::new(ip, 5432, "a", "d", "u", "p", &root_ca_over),
    ] {
        assert!(matches!(result, Err(DurabilityError::RootUnavailable)));
    }

    assert!(matches!(
        checked_retained_config_input_len([
            TLS_SERVER_NAME_MAX_BYTES + 1,
            DATABASE_MAX_BYTES,
            USERNAME_MAX_BYTES,
            PASSWORD_MAX_BYTES,
            ROOT_CA_PEM_MAX_BYTES,
        ]),
        Ok(value) if value == RETAINED_CONFIG_MAX_BYTES + 1
    ));
    assert!(matches!(
        validate_retained_config_input_lengths([
            TLS_SERVER_NAME_MAX_BYTES + 1,
            DATABASE_MAX_BYTES,
            USERNAME_MAX_BYTES,
            PASSWORD_MAX_BYTES,
            ROOT_CA_PEM_MAX_BYTES,
        ]),
        Err(DurabilityError::RootUnavailable)
    ));
    assert!(matches!(
        checked_retained_config_input_len([usize::MAX, 1, 0, 0, 0]),
        Err(DurabilityError::RootUnavailable)
    ));
}

#[test]
fn retained_configuration_enforces_dns_and_secret_grammar() {
    let ip = IpAddr::V4(Ipv4Addr::new(203, 0, 113, 7));
    let ca = pem_block(32, "\n");
    for tls in [
        format!("{}.example", "a".repeat(64)),
        "db.example.".to_owned(),
        "-db.example".to_owned(),
        "db_.example".to_owned(),
        "d\u{00e9}b.example".to_owned(),
    ] {
        assert!(matches!(
            DurabilityRootConfig::new(ip, 5432, &tls, "d", "u", "p", &ca),
            Err(DurabilityError::RootUnavailable)
        ));
    }
    assert!(matches!(
        DurabilityRootConfig::new(ip, 5432, "db.example", "d", "u", "", &ca),
        Err(DurabilityError::InvalidConfiguration)
    ));
}

#[test]
fn retained_root_ca_accepts_lf_and_crlf_canonical_bundles() {
    let ip = IpAddr::V4(Ipv4Addr::new(203, 0, 113, 7));
    for newline in ["\n", "\r\n"] {
        let ca = pem_bundle(ROOT_CA_MAX_CERTIFICATES, ROOT_CA_MAX_DER_BYTES, newline);
        assert!(
            DurabilityRootConfig::new_with_isolated_test_ledger(
                ip,
                5432,
                "db.example",
                "d",
                "u",
                "p",
                &ca
            )
            .is_ok()
        );
    }
}

#[test]
fn retained_root_ca_rejects_count_der_aggregate_and_noncanonical_text() {
    let ip = IpAddr::V4(Ipv4Addr::new(203, 0, 113, 7));
    let fifth_root = pem_bundle(ROOT_CA_MAX_CERTIFICATES + 1, 32, "\n");
    let oversized_der = pem_block(ROOT_CA_MAX_DER_BYTES + 1, "\n");
    let malformed = b"unrelated\n-----BEGIN CERTIFICATE-----\nAA==\n-----END CERTIFICATE-----\n";
    let non_ascii = [0xff_u8];
    let mut mixed_newline = pem_block(32, "\r\n");
    mixed_newline
        .extend_from_slice(b"-----BEGIN CERTIFICATE-----\nAA==\n-----END CERTIFICATE-----\n");

    for ca in [
        fifth_root.as_slice(),
        oversized_der.as_slice(),
        malformed.as_slice(),
        non_ascii.as_slice(),
        mixed_newline.as_slice(),
    ] {
        assert!(matches!(
            DurabilityRootConfig::new(ip, 5432, "db.example", "d", "u", "p", ca),
            Err(DurabilityError::RootUnavailable)
        ));
    }
    assert!(matches!(
        checked_root_der_total(ROOT_CA_MAX_AGGREGATE_DER_BYTES, 1),
        Err(DurabilityError::RootUnavailable)
    ));
}

#[test]
fn selected_root_profile_is_strict_lazy_and_max_one() -> Result<(), Box<dyn std::error::Error>> {
    let config = config()?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    runtime.block_on(async {
        let (pool, _root_i_reservation) = build_root_pool(config);
        let options = pool.connect_options();

        assert_eq!(DB_PASS_DEADLINE, Duration::from_secs(2));
        assert_eq!(ROOT_RECOVERY_WINDOW, Duration::from_secs(5));
        assert_eq!(pool.options().get_max_connections(), 1);
        assert_eq!(pool.options().get_min_connections(), 0);
        assert_eq!(pool.options().get_acquire_timeout(), ROOT_RECOVERY_WINDOW);
        assert_eq!(pool.options().get_idle_timeout(), Some(HOLDER_IDLE_TIMEOUT));
        assert_eq!(pool.options().get_max_lifetime(), Some(HOLDER_MAX_LIFETIME));
        assert_eq!(options.get_host(), "db.example");
        assert_eq!(options.get_host_addr(), Some("203.0.113.7"));
        assert_eq!(options.get_port(), 5432);
        assert_eq!(options.get_username(), "explicit");
        assert_eq!(options.get_database(), Some("oteryn"));
        assert!(matches!(options.get_ssl_mode(), PgSslMode::VerifyFull));
        assert!(matches!(
            options.get_authentication_policy(),
            PgAuthenticationPolicy::ScramSha256
        ));
        assert!(options.get_ssl_tls13_only());
        assert!(!options.get_ssl_session_resumption());
        assert!(!options.get_ssl_use_default_roots());
        assert!(!options.has_ssl_client_auth());
        assert_eq!(pool.size(), 0);
        assert_eq!(pool.num_idle(), 0);
    });
    Ok(())
}

#[test]
fn dedicated_runtime_representation_is_complete_for_frozen_topology()
-> Result<(), Box<dyn std::error::Error>> {
    let profile = tokio::runtime::oteryn_wp3_dedicated_runtime_allocation_profile(
        WP3_RUNTIME_WORKER_THREADS,
        WP3_RUNTIME_MAX_BLOCKING_THREADS,
        WP3_RUNTIME_WORKER_STACK_BYTES,
    )
    .ok_or("missing WP3 runtime profile")?;

    assert_eq!(profile.scheduler_worker_count, 1);
    assert_eq!(profile.blocking_pool_thread_cap, 2);
    assert_eq!(profile.additional_blocking_worker_limit, 1);
    assert_eq!(profile.worker_stack_request, 2 * 1024 * 1024);
    assert_eq!(
        profile.blocking_pool_thread_cap,
        profile.scheduler_worker_count + profile.additional_blocking_worker_limit
    );
    assert_eq!(
        checked_runtime_stack_reservation(
            profile.worker_stack_request,
            profile.blocking_pool_thread_cap,
        )?,
        4 * 1024 * 1024
    );
    assert!(matches!(
        checked_runtime_stack_reservation(usize::MAX, 2),
        Err(DurabilityError::RootUnavailable)
    ));
    #[cfg(windows)]
    assert_eq!(
        profile
            .heap_requests
            .into_iter()
            .filter(|request| *request != 0)
            .count(),
        21
    );
    #[cfg(not(windows))]
    {
        assert_eq!(profile.heap_requests[17], 0);
        assert_eq!(
            profile
                .heap_requests
                .into_iter()
                .filter(|request| *request != 0)
                .count(),
            20
        );
    }
    Ok(())
}

#[test]
fn retained_config_is_reserved_against_same_root_i_for_its_full_lifetime()
-> Result<(), Box<dyn std::error::Error>> {
    let ip = IpAddr::V4(Ipv4Addr::new(203, 0, 113, 7));
    let ca = pem_block(32, "\n");
    let lengths = [
        "db.example".len(),
        "oteryn".len(),
        "explicit".len(),
        "test-secret".len(),
        ca.len(),
    ];
    let required = root_i_reservation_bytes(lengths)?;
    assert!(required < ROOT_TOTAL_RESIDENT_BYTES);

    let denied = Arc::new(RootResidentLedger::new(required - 1));
    assert!(matches!(
        DurabilityRootConfig::new_with_ledger(
            ip,
            5432,
            "db.example",
            "oteryn",
            "explicit",
            "test-secret",
            &ca,
            RootLedgerHandle::Test(denied.clone()),
        ),
        Err(DurabilityError::RootUnavailable)
    ));
    assert_eq!(denied.used(), 0);

    let ledger = Arc::new(RootResidentLedger::new(required));
    let config = DurabilityRootConfig::new_with_ledger(
        ip,
        5432,
        "db.example",
        "oteryn",
        "explicit",
        "test-secret",
        &ca,
        RootLedgerHandle::Test(ledger.clone()),
    )?;
    assert_eq!(config.root_i_reservation.bytes(), required);
    assert_eq!(ledger.used(), required);

    let root = DurabilityRoot::new(config)?;
    let other_owner = root.clone();
    assert_eq!(ledger.used(), required);
    drop(root);
    assert_eq!(ledger.used(), required);
    drop(other_owner);
    assert_eq!(ledger.used(), 0);
    Ok(())
}

#[test]
fn no_demand_maintenance_is_a_noop_and_never_connects() -> Result<(), Box<dyn std::error::Error>> {
    let root = DurabilityRoot::new(config()?)?;
    root.ready_demand.store(false, Ordering::Release);
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    runtime.block_on(async {
        let maintained = root.maintain_ready_once().await?;
        assert!(!maintained);
        assert!(!root.is_ready());
        assert!(!root.has_ready_demand());
        assert_eq!(root.pool.size(), 0);
        Ok::<(), DurabilityError>(())
    })?;
    drop(root);
    Ok(())
}

#[test]
fn budgeted_pass_never_outlives_its_outer_budget() -> Result<(), Box<dyn std::error::Error>> {
    let now = Instant::now();
    let pass = DB_PASS_DEADLINE;
    assert_eq!(
        bounded_pass_deadline(now, pass, now + Duration::from_millis(700)),
        now + Duration::from_millis(700)
    );
    assert_eq!(
        bounded_pass_deadline(now, pass, now + Duration::from_secs(9)),
        now + pass
    );
    // An expired budget issues no pass and records no ready demand.
    let root = DurabilityRoot::new(config()?)?;
    root.ready_demand.store(false, Ordering::Release);
    assert!(matches!(
        root.try_issue_semantic_pass_by(Instant::now()),
        Err(DurabilityError::RootPassDeadlineExceeded)
    ));
    assert!(!root.ready_demand.load(Ordering::Acquire));
    Ok(())
}

#[test]
fn boot_pass_deadline_is_bounded_shared_by_clones_and_ends_with_its_guard()
-> Result<(), Box<dyn std::error::Error>> {
    let root = DurabilityRoot::new(config()?)?;
    let clone = root.clone();
    assert_eq!(root.pass_deadline(), DB_PASS_DEADLINE);
    {
        let _boot = root.boot_pass_deadline(Duration::from_secs(15));
        assert_eq!(root.pass_deadline(), Duration::from_secs(15));
        assert_eq!(clone.pass_deadline(), Duration::from_secs(15));
    }
    assert_eq!(root.pass_deadline(), DB_PASS_DEADLINE);
    assert_eq!(clone.pass_deadline(), DB_PASS_DEADLINE);
    for (asked, bounded) in [
        (
            Duration::from_secs(3_600),
            DurabilityRoot::MAX_BOOT_PASS_DEADLINE,
        ),
        (Duration::MAX, DurabilityRoot::MAX_BOOT_PASS_DEADLINE),
        (Duration::from_millis(1), DB_PASS_DEADLINE),
        (Duration::ZERO, DB_PASS_DEADLINE),
    ] {
        let boot = clone.boot_pass_deadline(asked);
        assert_eq!(root.pass_deadline(), bounded);
        drop(boot);
        assert_eq!(root.pass_deadline(), DB_PASS_DEADLINE);
    }
    Ok(())
}
