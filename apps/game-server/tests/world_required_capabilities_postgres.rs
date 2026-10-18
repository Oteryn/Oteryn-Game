// Store-boundary qualification using the real atomic owner and existing independent
// signed fresh-admission fixture. This does not stand in for Platform/TLS or transport E2E.
extern crate oteryn_game_server as production_server;
extern crate self as oteryn_game_server;
pub use production_server::admission_evidence;
pub use production_server::domain;
#[path = "support/authority_matrix.rs"]
mod authority_matrix;
#[allow(dead_code, unused_imports)]
#[path = "../src/character_bootstrap_intent.rs"]
pub mod character_bootstrap_intent;
#[allow(dead_code, unused_imports)]
#[path = "../src/character_recovery_fence.rs"]
pub mod character_recovery_fence;
#[allow(dead_code, unused_imports)]
#[path = "../src/durability/mod.rs"]
mod durability;
#[allow(dead_code, unused_imports)]
#[path = "../src/foundation/mod.rs"]
pub mod foundation;
#[allow(dead_code, unused_imports)]
#[path = "../src/native_admission_source/mod.rs"]
pub mod native_admission_source;
#[expect(
    dead_code,
    reason = "shared corruption helpers are unused in this target"
)]
#[path = "support/postgres.rs"]
mod postgres;
// The durability mount's NPC travel fixture needs the shared harness; it uses a subset.
#[allow(dead_code)]
#[path = "support/bestiary_postgres_harness.rs"]
mod bestiary_postgres_harness;

// The shared PostgreSQL support module includes its existing reconnect contract tests.
// Their root data builders are reused verbatim from durability_postgres.rs
// (foundation_error through record_for_actor_epoch_with_protection), excluding
// the unused postgres_clock helper. Only visibility changes for root reexports.
mod reconnect_contract_fixture {
    use super::{foundation::*, postgres};
    use std::time::{SystemTime, UNIX_EPOCH};

    pub(super) fn foundation_error(error: ReconnectDurabilityErrorV1) -> std::io::Error {
        std::io::Error::other(format!(
            "Foundation V1 record construction failed: {error:?}"
        ))
    }

    pub(super) fn unix_now() -> Result<i64, ReconnectDurabilityErrorV1> {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_error| ReconnectDurabilityErrorV1::InvalidRecord)?
            .as_secs()
            .try_into()
            .map_err(|_error| ReconnectDurabilityErrorV1::InvalidRecord)
    }

    pub(super) fn uuid_v7(raw: u64) -> [u8; 16] {
        let mut value = [0u8; 16];
        value[8..].copy_from_slice(&raw.to_be_bytes());
        value[6] = 0x70;
        value[8] = (value[8] & 0x3f) | 0x80;
        value
    }

    pub(super) fn record(
        game_session_raw: u64,
        attempt_raw: u64,
        transport_byte: u8,
        now: i64,
    ) -> Result<ReconnectDurabilityRecordV1, ReconnectDurabilityErrorV1> {
        record_with_prepared_deadline(
            game_session_raw,
            attempt_raw,
            transport_byte,
            now,
            now + 115,
        )
    }

    fn record_with_prepared_deadline(
        game_session_raw: u64,
        attempt_raw: u64,
        transport_byte: u8,
        now: i64,
        prepared_deadline: i64,
    ) -> Result<ReconnectDurabilityRecordV1, ReconnectDurabilityErrorV1> {
        record_for_epoch(
            game_session_raw,
            attempt_raw,
            transport_byte,
            now,
            prepared_deadline,
            3,
            7,
            8,
            0x55,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn record_for_epoch(
        game_session_raw: u64,
        attempt_raw: u64,
        transport_byte: u8,
        now: i64,
        prepared_deadline: i64,
        control_loss_epoch: u64,
        predecessor_generation: u64,
        candidate_generation: u64,
        recovery_nonce_byte: u8,
    ) -> Result<ReconnectDurabilityRecordV1, ReconnectDurabilityErrorV1> {
        record_for_epoch_with_protection(
            game_session_raw,
            attempt_raw,
            transport_byte,
            now,
            prepared_deadline,
            control_loss_epoch,
            predecessor_generation,
            candidate_generation,
            recovery_nonce_byte,
            ProtectionEntitlementV1::unused(),
        )
    }

    pub(super) fn record_for_actor(
        game_session_raw: u64,
        character_raw: u64,
        attempt_raw: u64,
        transport_byte: u8,
        now: i64,
    ) -> Result<ReconnectDurabilityRecordV1, ReconnectDurabilityErrorV1> {
        record_for_actor_epoch_with_protection(
            game_session_raw,
            character_raw,
            attempt_raw,
            transport_byte,
            now,
            now + 115,
            3,
            7,
            8,
            0x55,
            ProtectionEntitlementV1::unused(),
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn record_for_epoch_with_protection(
        game_session_raw: u64,
        attempt_raw: u64,
        transport_byte: u8,
        now: i64,
        prepared_deadline: i64,
        control_loss_epoch: u64,
        predecessor_generation: u64,
        candidate_generation: u64,
        recovery_nonce_byte: u8,
        protection_entitlement: ProtectionEntitlementV1,
    ) -> Result<ReconnectDurabilityRecordV1, ReconnectDurabilityErrorV1> {
        record_for_actor_epoch_with_protection(
            game_session_raw,
            11,
            attempt_raw,
            transport_byte,
            now,
            prepared_deadline,
            control_loss_epoch,
            predecessor_generation,
            candidate_generation,
            recovery_nonce_byte,
            protection_entitlement,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn record_for_actor_epoch_with_protection(
        game_session_raw: u64,
        character_raw: u64,
        attempt_raw: u64,
        transport_byte: u8,
        now: i64,
        prepared_deadline: i64,
        control_loss_epoch: u64,
        predecessor_generation: u64,
        candidate_generation: u64,
        recovery_nonce_byte: u8,
        protection_entitlement: ProtectionEntitlementV1,
    ) -> Result<ReconnectDurabilityRecordV1, ReconnectDurabilityErrorV1> {
        let game_session_id = GameSessionId::decode(&uuid_v7(game_session_raw))
            .map_err(|_error| ReconnectDurabilityErrorV1::InvalidRecord)?;
        let character_id = CharacterId::decode(&uuid_v7(character_raw))
            .map_err(|_error| ReconnectDurabilityErrorV1::InvalidRecord)?;
        let world_id = WorldId::decode(&uuid_v7(12))
            .map_err(|_error| ReconnectDurabilityErrorV1::InvalidRecord)?;
        let channel_id = ChannelId::decode(&uuid_v7(13))
            .map_err(|_error| ReconnectDurabilityErrorV1::InvalidRecord)?;
        let identity = ReconnectIdentityV1::new(
            game_session_id,
            ReconnectAttemptRef::new(attempt_raw)
                .map_err(|_error| ReconnectDurabilityErrorV1::InvalidRecord)?,
            &postgres::fixture_account_for_character(character_raw),
            character_id,
            world_id,
            RuntimeScopeRefV1::channel(world_id, channel_id),
        )?;
        let connection = ReconnectConnectionFenceV1::new(
            ConnectionGeneration::new(predecessor_generation)
                .map_err(|_error| ReconnectDurabilityErrorV1::InvalidRecord)?,
            ConnectionGeneration::new(candidate_generation)
                .map_err(|_error| ReconnectDurabilityErrorV1::InvalidRecord)?,
            AuthenticatedTransportRefV1::decode(&[transport_byte; 16])
                .map_err(|_error| ReconnectDurabilityErrorV1::InvalidRecord)?,
        )?;
        let authority = ReconnectAuthorityFenceV1::new(
            9,
            ScopeOwnershipGeneration::new(10)
                .map_err(|_error| ReconnectDurabilityErrorV1::InvalidRecord)?,
        )?;
        let continuity = ReconnectContinuityV1::new(
            ControlLossEpochRefV1::new(control_loss_epoch)?,
            now + 120,
            prepared_deadline,
            protection_entitlement,
        )?;
        let fnd02 = Fnd02ReconciliationFenceV1::new(
            CommandId::new(3).map_err(|_error| ReconnectDurabilityErrorV1::InvalidRecord)?,
            vec![
                PendingCommandReconciliationV1::new(
                    CommandId::new(1)
                        .map_err(|_error| ReconnectDurabilityErrorV1::InvalidRecord)?,
                    PendingCommandDispositionV1::PendingOriginal,
                ),
                PendingCommandReconciliationV1::new(
                    CommandId::new(2)
                        .map_err(|_error| ReconnectDurabilityErrorV1::InvalidRecord)?,
                    PendingCommandDispositionV1::TerminalOutcomeRetained,
                ),
            ],
            41,
            vec![
                StateDomainRevisionV1::new(1, 4)?,
                StateDomainRevisionV1::new(2, 7)?,
            ],
        )?;
        let platform = AuthorityEvidenceFenceV1::new(
            "platform-security",
            "reconnect",
            "account",
            "sec:17",
            "decision:sec:17",
            now,
        )?;
        let trust = AuthorityEvidenceFenceV1::new(
            "proof-trust",
            "reconnect",
            "recovery-key",
            "trust:21",
            "decision:trust:21",
            now,
        )?;
        let compatibility = ReconnectCompatibilityEvidenceV1::new(
            1,
            1,
            "rules:1",
            "content:2",
            "map:3",
            "world:4",
            12,
            platform,
            trust,
            Some(now + 110),
        )?;
        ReconnectDurabilityRecordV1::new(
            identity,
            connection,
            authority,
            continuity,
            ReconnectProofV1::ReauthenticatedRecovery {
                recovery_grant_nonce: [recovery_nonce_byte; 32],
            },
            fnd02,
            compatibility,
        )
    }
}

use reconnect_contract_fixture::{
    foundation_error, record, record_for_actor, record_for_epoch, unix_now, uuid_v7,
};

// Keep the ADMIT0 cases separate from the focused harness support. The canonical
// durability_postgres mount waits for its allocated shared-file slot.
#[path = "support/world_required_capabilities_postgres_cases.rs"]
mod world_required_capabilities_postgres_cases;
