const MIGRATION: &str =
    include_str!("../../../../../apps/game-server/migrations/0001_admission_reconnect_journal.sql");
// Read at run time rather than `include_str!`: the file belongs to oteryn-gameplay-foundation,
// and compiling it into this crate's dep-info would count its unit tests as duplicated here.
#[allow(clippy::unwrap_used)] // test-only fixture read; a missing file must fail the test
static ADMISSION_RECOVERY: std::sync::LazyLock<String> = std::sync::LazyLock::new(|| {
    // Resolve from the workspace root: game-server test targets path-include this module,
    // so the manifest directory is not always this crate's.
    const SOURCE: &str = "crates/gameplay/foundation/src/foundation/admission_recovery_inner.rs";
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .find(|dir| dir.join(SOURCE).is_file())
        .unwrap();
    std::fs::read_to_string(root.join(SOURCE)).unwrap()
});

fn session_schema() -> Option<&'static str> {
    MIGRATION
        .split_once("CREATE TABLE game_durability_transport_ref_reservations")
        .map(|(session, _rest)| session)
}

#[test]
fn migration_ledger_inspection_is_bounded_before_checksum_materialization() {
    let sql = super::MIGRATION_LEDGER_INSPECTION_SQL;
    assert!(sql.contains("unnest($1::BIGINT[], $2::BIGINT[])"));
    assert!(sql.contains("octet_length(m.checksum)::BIGINT = e.checksum_len"));
    assert!(sql.contains("THEN m.checksum ELSE NULL END AS bounded_checksum"));
    assert!(sql.contains("LIMIT $3"));
}

#[test]
fn reconnect_attempts_index_supports_actor_epoch_budget_count() {
    assert!(
        MIGRATION
            .contains("ON game_durability_reconnect_attempts (character_id, control_loss_epoch)")
    );
}

#[test]
fn record_derived_current_authority_is_test_only_but_current_facts_remain_public()
-> Result<(), Box<dyn std::error::Error>> {
    let manifest = include_str!("../../../../../apps/game-server/Cargo.toml");
    let (production, development) = manifest
        .split_once("[dev-dependencies]")
        .ok_or("server dev dependencies")?;
    let production = production
        .split_once("[dependencies]")
        .ok_or("server production dependencies")?
        .1;
    assert!(!production.contains("test-support"));
    for package in ["foundation", "content", "durability"] {
        let dependency = format!("oteryn-gameplay-{package} = ");
        let fixture = development
            .lines()
            .find(|line| line.starts_with(&dependency))
            .ok_or("gameplay test dependency")?;
        assert!(fixture.contains(r#"features = ["test-support"]"#));
    }
    assert!(
        ADMISSION_RECOVERY.contains("pub fn from_current_facts("),
        "production callers must be able to supply independently observed current authority"
    );
    assert!(
        ADMISSION_RECOVERY
            .contains("#[cfg(any(test, feature = \"test-support\"))]\n    pub fn from_record("),
        "record-derived current authority must not be production-accessible"
    );
    Ok(())
}

#[test]
fn record_derived_candidate_binding_is_internal_and_its_convenience_is_test_only() {
    let candidate_impl = ADMISSION_RECOVERY
        .split_once("impl ReconnectCandidateBindingV1 {")
        .and_then(|(_before, rest)| rest.split_once("\n}\n"))
        .map(|(implementation, _after)| implementation);
    assert!(
        candidate_impl.is_some(),
        "candidate binding implementation must remain present"
    );
    let Some(candidate_impl) = candidate_impl else {
        return;
    };
    let production_candidate_impl = candidate_impl.replace(
        "#[cfg(any(test, feature = \"test-support\"))]\n    pub fn from_record(",
        "",
    );

    assert!(
        !production_candidate_impl.contains("\n    pub fn from_record("),
        "record-derived candidate binding must not be production-public"
    );
    assert!(
        candidate_impl.contains("fn expected_binding_from_record("),
        "immutable record derivation must remain an internal expected-binding helper"
    );
    assert!(
        candidate_impl
            .contains("#[cfg(any(test, feature = \"test-support\"))]\n    pub fn from_record("),
        "the record-derived candidate convenience must remain available only to tests"
    );
}

#[test]
fn identity_derived_authority_claim_convenience_is_test_only_across_sibling_family() {
    for type_name in ["CharacterWorldEligibilityClaimV1", "AccountPresenceClaimV1"] {
        let marker = format!("impl {type_name} {{");
        let implementation = ADMISSION_RECOVERY
            .split_once(&marker)
            .and_then(|(_before, rest)| rest.split_once("\n}\n"))
            .map(|(implementation, _after)| implementation);
        assert!(
            implementation.is_some(),
            "{type_name}: implementation missing"
        );
        let Some(implementation) = implementation else {
            continue;
        };
        let production_impl = implementation
                .replace(
                    "#[cfg(any(test, feature = \"test-support\"))]\n    #[must_use]\n    pub fn from_identity(",
                    "",
                )
                .replace("#[cfg(any(test, feature = \"test-support\"))]\n    pub fn from_identity(", "");

        assert!(
            !production_impl.contains("\n    pub fn from_identity("),
            "{type_name}: identity-derived authority convenience must not be production-public"
        );
        assert!(
            implementation.contains("fn expected_from_identity("),
            "{type_name}: immutable identity derivation must remain an internal expected-value helper"
        );
        assert!(
                implementation.contains("#[cfg(any(test, feature = \"test-support\"))]\n    pub fn from_identity(")
                    || implementation
                        .contains("#[cfg(any(test, feature = \"test-support\"))]\n    #[must_use]\n    pub fn from_identity("),
                "{type_name}: identity-derived convenience must remain available only to tests"
            );
    }
}

#[test]
fn generic_v1_terminal_reconciliation_is_not_a_production_recovery_api() {
    const ADMISSION_JOURNAL: &str = include_str!("admission_journal.rs");
    const DURABILITY: &str = include_str!("core_modules.rs");

    assert!(
        ADMISSION_JOURNAL.contains("pub async fn reconcile("),
        "generic V1 reconciliation collapses typed terminal reasons and must remain crate-internal"
    );
    assert!(
        DURABILITY.contains("pub const fn legacy("),
        "the V2 legacy adapter must not expose generic V1 terminal reconciliation to production callers"
    );
    assert!(
        DURABILITY.contains("pub async fn reconcile("),
        "typed V2 reconciliation must remain the public production recovery boundary"
    );
}

#[test]
fn reconnect_session_schema_binds_actor_and_runtime_scope_identity() {
    let session = session_schema();
    assert!(
        session.is_some(),
        "session table must precede transport reservations"
    );
    let Some(session) = session else {
        return;
    };
    for required in [
        "account_id UUID NOT NULL",
        "character_id UUID NOT NULL",
        "world_id UUID NOT NULL",
        "runtime_scope_kind SMALLINT NOT NULL",
        "runtime_scope_world_id UUID NOT NULL",
        "runtime_scope_channel_id UUID NULL",
        "runtime_scope_instance_id UUID NULL",
    ] {
        assert!(
            session.contains(required),
            "reconnect session schema must retain {required}"
        );
    }
}

#[test]
fn reconnect_authority_fences_preserve_the_full_unsigned_range() {
    for required in [
        "predecessor_generation NUMERIC(20, 0) NOT NULL",
        "character_lease_generation NUMERIC(20, 0) NOT NULL",
        "scope_ownership_generation NUMERIC(20, 0) NOT NULL",
        "current_generation NUMERIC(20, 0) NOT NULL",
    ] {
        assert!(
            MIGRATION.contains(required),
            "reconnect session schema must retain full u64 range for {required}"
        );
    }
    assert_eq!(
        MIGRATION
            .matches("control_loss_epoch NUMERIC(20, 0) NOT NULL")
            .count(),
        3,
        "session, actor-wide continuity and attempt ControlLossEpoch mirrors must retain the full u64 range"
    );
}
