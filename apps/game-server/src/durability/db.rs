use crate::durability::DurabilityError;
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use std::time::Duration;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);

pub async fn connect(database_url: &str, max_connections: u32) -> Result<PgPool, DurabilityError> {
    PgPoolOptions::new()
        .max_connections(max_connections)
        .acquire_timeout(CONNECT_TIMEOUT)
        .connect(database_url)
        .await
        .map_err(DurabilityError::from)
}

#[cfg(test)]
mod runtime_scope_identity_red_tests {
    use oteryn_game_server::foundation::{
        AuthenticatedTransportRefV1, AuthorityEvidenceFenceV1, ChannelId, CharacterId,
        CharacterLease, CommandId, ConnectionGeneration, ControlLossEpochRefV1,
        Fnd02ReconciliationFenceV1, FreshAdmissionCommit, FreshAdmissionFacts,
        GameSessionAuthoritySnapshot, GameSessionId, GameSessionState, ProtectionEntitlementV1,
        ReconnectAttemptBudgetV1, ReconnectAttemptRef, ReconnectAuthorityFenceV1,
        ReconnectCompatibilityEvidenceV1, ReconnectConnectionFenceV1, ReconnectContinuityV1,
        ReconnectCurrentAuthorityV1, ReconnectDurabilityErrorV1, ReconnectDurabilityFlowV2,
        ReconnectDurabilityRecordV1, ReconnectIdentityV1, ReconnectPrepareCompletionV2,
        ReconnectPrepareDispositionV2, ReconnectProofV1, RuntimeScopeRefV1,
        ScopeOwnershipGeneration, TerminalGameSessionReplacementAuthorizationV1, WorldId,
    };

    const ACCOUNT: &str = "123e4567-e89b-12d3-a456-426614174000";

    fn uuid_v7(raw: u64) -> [u8; 16] {
        let mut value = [0_u8; 16];
        value[8..].copy_from_slice(&raw.to_be_bytes());
        value[6] = 0x70;
        value[8] = (value[8] & 0x3f) | 0x80;
        value
    }

    fn game_session(raw: u64) -> Result<GameSessionId, ReconnectDurabilityErrorV1> {
        GameSessionId::decode(&uuid_v7(raw)).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)
    }

    fn character(raw: u64) -> Result<CharacterId, ReconnectDurabilityErrorV1> {
        CharacterId::decode(&uuid_v7(raw)).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)
    }

    fn world(raw: u64) -> Result<WorldId, ReconnectDurabilityErrorV1> {
        WorldId::decode(&uuid_v7(raw)).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)
    }

    fn channel(raw: u64) -> Result<ChannelId, ReconnectDurabilityErrorV1> {
        ChannelId::decode(&uuid_v7(raw)).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)
    }

    fn candidate_record() -> Result<ReconnectDurabilityRecordV1, ReconnectDurabilityErrorV1> {
        candidate_record_for_channel(13)
    }

    fn candidate_record_for_channel(
        channel_raw: u64,
    ) -> Result<ReconnectDurabilityRecordV1, ReconnectDurabilityErrorV1> {
        let world_id = world(12)?;
        let identity = ReconnectIdentityV1::new(
            game_session(20)?,
            ReconnectAttemptRef::new(1).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
            ACCOUNT,
            character(11)?,
            world_id,
            RuntimeScopeRefV1::channel(world_id, channel(channel_raw)?),
        )?;
        let connection = ReconnectConnectionFenceV1::new(
            ConnectionGeneration::new(7).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
            ConnectionGeneration::new(8).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
            AuthenticatedTransportRefV1::decode(&[0x71; 16])
                .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
        )?;
        let authority = ReconnectAuthorityFenceV1::new(
            9,
            ScopeOwnershipGeneration::new(10)
                .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
        )?;
        let continuity = ReconnectContinuityV1::new(
            ControlLossEpochRefV1::new(3)?,
            120,
            115,
            ProtectionEntitlementV1::unused(),
        )?;
        let fnd02 = Fnd02ReconciliationFenceV1::new(
            CommandId::new(3).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
            vec![],
            41,
            vec![],
        )?;
        let platform = AuthorityEvidenceFenceV1::new(
            "platform-security",
            "reconnect",
            "account",
            "sec:17",
            "decision:sec:17",
            100,
        )?;
        let trust = AuthorityEvidenceFenceV1::new(
            "proof-trust",
            "reconnect",
            "recovery-key",
            "trust:21",
            "decision:trust:21",
            101,
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
            Some(110),
        )?;
        ReconnectDurabilityRecordV1::new(
            identity,
            connection,
            authority,
            continuity,
            ReconnectProofV1::ReauthenticatedRecovery {
                recovery_grant_nonce: [0x55; 32],
            },
            fnd02,
            compatibility,
        )
    }

    fn predecessor_snapshot(
        current_channel_raw: u64,
    ) -> Result<GameSessionAuthoritySnapshot<AuthenticatedTransportRefV1>, ReconnectDurabilityErrorV1>
    {
        let facts = FreshAdmissionFacts::new(
            [0x44; 32],
            character(11)?,
            world(12)?,
            channel(current_channel_raw)?,
            9,
            10,
        )
        .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?;
        let initial_transport = AuthenticatedTransportRefV1::decode(&[0x70; 16])
            .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?;
        let commit = FreshAdmissionCommit::from_facts(game_session(10)?, facts, initial_transport)
            .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?;
        GameSessionAuthoritySnapshot::new(
            commit,
            GameSessionState::Terminal,
            ConnectionGeneration::new(7).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
            None,
            CharacterLease::new(character(11)?, 9)
                .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
            ScopeOwnershipGeneration::new(10)
                .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
        )
        .with_control_loss_continuity(ControlLossEpochRefV1::new(3)?, 120)
    }

    #[test]
    fn terminal_replacement_rejects_same_world_generation_different_current_runtime_scope()
    -> Result<(), ReconnectDurabilityErrorV1> {
        let candidate = candidate_record()?;
        let result = TerminalGameSessionReplacementAuthorizationV1::from_current_authority(
            ACCOUNT,
            game_session(10)?,
            game_session(20)?,
            predecessor_snapshot(14)?,
            &candidate,
        );
        assert_eq!(result, Err(ReconnectDurabilityErrorV1::StaleAuthority));
        Ok(())
    }

    #[test]
    fn final_revalidation_can_supply_actual_current_runtime_scope_and_reject_drift()
    -> Result<(), ReconnectDurabilityErrorV1> {
        let record = candidate_record()?;
        let mut budget = ReconnectAttemptBudgetV1::new(record.continuity().control_loss_epoch());
        budget.reserve(
            record.identity().reconnect_attempt_ref(),
            record.connection().transport_ref(),
        )?;
        let (mut flow, request) = ReconnectDurabilityFlowV2::begin(record.clone(), None);
        flow.accept_prepare_completion(
            ReconnectPrepareCompletionV2::for_request(
                &request,
                ReconnectPrepareDispositionV2::Prepared,
            ),
            &mut budget,
        )?;

        let current = ReconnectCurrentAuthorityV1::from_current_facts(
            &record,
            record.authority(),
            record.fnd02().clone(),
            record.compatibility().clone(),
            GameSessionState::Reconnectable,
            false,
            105,
        )?
        .with_current_runtime_scope(RuntimeScopeRefV1::channel(
            record.identity().world_id(),
            channel(14)?,
        ))?;

        assert_eq!(
            flow.authorize_commit(current, 104),
            Err(ReconnectDurabilityErrorV1::StaleAuthority)
        );
        Ok(())
    }

    #[test]
    fn replacement_authorization_rejects_runtime_scope_record_substitution()
    -> Result<(), ReconnectDurabilityErrorV1> {
        let candidate = candidate_record()?;
        let authorization = TerminalGameSessionReplacementAuthorizationV1::from_current_authority(
            ACCOUNT,
            game_session(10)?,
            game_session(20)?,
            predecessor_snapshot(13)?,
            &candidate,
        )?;
        let substituted = candidate_record_for_channel(14)?;

        assert!(!crate::durability::replacement_authorization_matches_record(
            &authorization,
            &substituted,
        ));
        Ok(())
    }
}