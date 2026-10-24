#[cfg(test)]
mod account_achievement_linkage {
    use super::DurabilityRoot;
    use super::account_achievement::{
        AccountAchievement, AchievementCatalogueLookup, AchievementGrantError,
        AchievementGrantOutcome, AchievementGrantRequest, AchievementSourceEvent,
        FencedGrantingCharacter, record_achievement_grant,
    };

    #[test]
    fn account_achievement_api_is_linked() {
        let _ = std::mem::size_of::<AccountAchievement>();
        let _ = std::mem::size_of::<AchievementCatalogueLookup>();
        let _ = std::mem::size_of::<AchievementGrantError>();
        let _ = std::mem::size_of::<AchievementGrantOutcome>();
        let _ = std::mem::size_of::<AchievementGrantRequest>();
        let _ = std::mem::size_of::<AchievementSourceEvent>();
        let _ = FencedGrantingCharacter::after_fence;
        let _ = record_achievement_grant;
        let _ = DurabilityRoot::commit_test_achievement_grants;
    }
}

#[cfg(test)]
mod character_last_position_linkage {
    use super::DurabilityRoot;
    use super::character_last_position::{
        FinalLastPosition, LAST_POSITION_WRITE_BATCH, LastPositionCell, LastPositionKey,
        LastPositionRecord, LastPositionWrite, PeriodicLastPosition, StoredLastPosition,
    };
    use super::fresh_admission::FreshAdmissionStore;

    #[test]
    fn character_last_position_api_is_linked() {
        let _ = std::mem::size_of::<FinalLastPosition>();
        let _ = std::mem::size_of::<LastPositionCell>();
        let _ = std::mem::size_of::<LastPositionKey>();
        let _ = std::mem::size_of::<LastPositionRecord>();
        let _ = std::mem::size_of::<LastPositionWrite>();
        let _ = std::mem::size_of::<StoredLastPosition>();
        let _ = std::mem::size_of::<PeriodicLastPosition>();
        let _ = LAST_POSITION_WRITE_BATCH;
        let _ = DurabilityRoot::write_character_last_position;
        let _ = DurabilityRoot::write_character_last_positions;
        let _ = FreshAdmissionStore::current_sessions_at;
        let _ = DurabilityRoot::read_character_last_position;
        let _ = DurabilityRoot::consume_pending_respawn_placed;
        let _ = FreshAdmissionStore::with_final_position;
    }
}

#[cfg(test)]
mod bestiary_progress_linkage {
    use super::DurabilityRoot;
    use super::bestiary_progress::{
        BestiaryKillOccurrence, BestiaryKillOutcome, BestiaryKillRequest, BestiaryProgressError,
        BestiaryRaceProgress, CommittedBestiaryKill,
    };

    #[test]
    fn bestiary_progress_api_is_linked() {
        let _ = std::mem::size_of::<BestiaryKillOutcome>();
        let _ = std::mem::size_of::<BestiaryKillRequest>();
        let _ = std::mem::size_of::<BestiaryProgressError>();
        let _ = std::mem::size_of::<BestiaryRaceProgress>();
        let _ = std::mem::size_of::<CommittedBestiaryKill>();
        let _ = BestiaryKillOccurrence::from_bytes;
        let _ = BestiaryKillOccurrence::as_bytes;
        let _ = DurabilityRoot::commit_bestiary_kill;
        let _ = DurabilityRoot::reconcile_bestiary_kill;
        let _ = DurabilityRoot::read_bestiary_progress;
    }
}

#[cfg(test)]
mod character_progression_linkage {
    use super::DurabilityRoot;
    use super::character_progression::{
        CharacterProgressionError, CharacterProgressionState, CommittedExperienceAward,
        CurrentCharacterGameplayFence, ExperienceAwardRequest, ExperienceCommitOutcome,
        ExperienceRewardOccurrence, ProgressionInitializationOutcome,
        ProgressionInitializationRequest,
    };

    #[test]
    fn character_progression_api_is_linked() {
        let _ = std::mem::size_of::<CharacterProgressionError>();
        let _ = std::mem::size_of::<CharacterProgressionState>();
        let _ = std::mem::size_of::<CommittedExperienceAward>();
        let _ = std::mem::size_of::<CurrentCharacterGameplayFence>();
        let _ = std::mem::size_of::<ExperienceAwardRequest<2>>();
        let _ = std::mem::size_of::<ExperienceCommitOutcome>();
        let _ = ExperienceRewardOccurrence::from_bytes;
        let _ = ExperienceRewardOccurrence::as_bytes;
        let _ = DurabilityRoot::commit_character_experience::<2>;
        let _ = DurabilityRoot::reconcile_character_experience;
        let _ = DurabilityRoot::read_character_progression;
        let _ = std::mem::size_of::<ProgressionInitializationRequest<2>>();
        let _ = std::mem::size_of::<ProgressionInitializationOutcome>();
        let _ = DurabilityRoot::initialize_character_progression::<2>;
    }

    #[test]
    fn character_death_api_is_linked() {
        use super::character_death::{
            CharacterDeathOutcome, CharacterDeathRequest, CommittedCharacterDeath, DeathCell,
            PlayerDeathOccurrence,
        };
        let _ = std::mem::size_of::<CharacterDeathOutcome>();
        let _ = std::mem::size_of::<CharacterDeathRequest<2>>();
        let _ = std::mem::size_of::<CommittedCharacterDeath>();
        let _ = std::mem::size_of::<DeathCell>();
        let _ = PlayerDeathOccurrence::from_bytes;
        let _ = PlayerDeathOccurrence::as_bytes;
        let _ = DurabilityRoot::commit_character_death::<2>;
        let _ = DurabilityRoot::reconcile_character_death;
    }
}

#[cfg(test)]
mod charm_state_linkage {
    use super::DurabilityRoot;
    use super::charm_state::{
        BestiaryCharmEntry, BestiaryCharmFacts, CharacterCharmState, CharmCommand,
        CharmCommandEffect, CharmCommandOccurrence, CharmCommandOutcome, CharmCommandRequest,
        CharmFacts, CharmStateError, CommittedCharmCommand,
    };

    struct NoFacts;

    impl CharmFacts for NoFacts {
        async fn completed_stage(
            &self,
            _connection: &mut sqlx::postgres::PgConnection,
            _character: crate::domain::CharacterId,
            _race: &crate::domain::charm::BestiaryRaceKey,
        ) -> Result<crate::domain::charm::BestiaryStage, super::DurabilityError> {
            Ok(crate::domain::charm::BestiaryStage::NONE)
        }

        async fn completed_entry_charm_points(
            &self,
            _connection: &mut sqlx::postgres::PgConnection,
            _character: crate::domain::CharacterId,
        ) -> Result<Vec<u32>, super::DurabilityError> {
            Ok(Vec::new())
        }

        async fn promoted(
            &self,
            _connection: &mut sqlx::postgres::PgConnection,
            _character: crate::domain::CharacterId,
        ) -> Result<bool, super::DurabilityError> {
            Ok(false)
        }

        async fn slot_entitlement(
            &self,
            _connection: &mut sqlx::postgres::PgConnection,
            _character: crate::domain::CharacterId,
        ) -> Result<crate::domain::charm::CharmSlotEntitlement, super::DurabilityError> {
            Ok(crate::domain::charm::CharmSlotEntitlement::Free)
        }
    }

    #[test]
    fn charm_state_api_is_linked() {
        let _ = std::mem::size_of::<CharacterCharmState>();
        let _ = std::mem::size_of::<BestiaryCharmEntry>();
        let _ = BestiaryCharmFacts::new(Vec::<BestiaryCharmEntry>::new());
        let _ = DurabilityRoot::commit_charm_command::<BestiaryCharmFacts>;
        let _ = std::mem::size_of::<CharmCommand>();
        let _ = std::mem::size_of::<CharmCommandEffect>();
        let _ = std::mem::size_of::<CharmCommandOutcome>();
        let _ = std::mem::size_of::<CharmCommandRequest>();
        let _ = std::mem::size_of::<CharmStateError>();
        let _ = std::mem::size_of::<CommittedCharmCommand>();
        let _ = CharmCommandOccurrence::from_bytes;
        let _ = CharmCommandOccurrence::as_bytes;
        let _ = CharmCommand::charm;
        let _ = DurabilityRoot::commit_charm_command::<NoFacts>;
        let _ = DurabilityRoot::reconcile_charm_command;
        let _ = DurabilityRoot::read_character_charm_state;
    }
}

#[cfg(test)]
mod character_build_linkage {
    use super::DurabilityRoot;
    use super::character_build::{
        BuildChangeRequest, BuildCommitOutcome, BuildOccurrence, CommittedBuildChange,
        DurableBuildState, skill_tries_required,
    };

    #[test]
    fn character_build_api_is_linked() {
        let _ = std::mem::size_of::<BuildChangeRequest>();
        let _ = std::mem::size_of::<BuildCommitOutcome>();
        let _ = std::mem::size_of::<CommittedBuildChange>();
        let _ = DurableBuildState::vocation;
        let _ = DurableBuildState::magic;
        let _ = DurableBuildState::skills;
        let _ = BuildOccurrence::from_bytes;
        let _ = BuildOccurrence::as_bytes;
        let _ = skill_tries_required;
        let _ = DurabilityRoot::commit_character_build;
        let _ = DurabilityRoot::reconcile_character_build;
        let _ = DurabilityRoot::read_character_build_state;
    }
}

#[cfg(test)]
mod monk_state_linkage {
    use super::DurabilityRoot;
    use super::monk_state::{
        CommittedMonkStateSave, DurableMonkState, MonkStateSaveOccurrence, MonkStateSaveOutcome,
        MonkStateSaveRequest,
    };

    #[test]
    fn monk_state_api_is_linked() {
        let _ = std::mem::size_of::<CommittedMonkStateSave>();
        let _ = std::mem::size_of::<MonkStateSaveOutcome>();
        let _ = std::mem::size_of::<MonkStateSaveRequest>();
        let _ = DurableMonkState::new;
        let _ = DurableMonkState::harmony;
        let _ = DurableMonkState::serene_forced_remaining_micros;
        let _ = MonkStateSaveOccurrence::from_bytes;
        let _ = MonkStateSaveOccurrence::as_bytes;
        let _ = DurabilityRoot::commit_character_monk_state_save;
        let _ = DurabilityRoot::reconcile_character_monk_state_save;
        let _ = DurabilityRoot::read_character_monk_state;
    }
}

#[cfg(test)]
mod character_wheel_linkage {
    use super::DurabilityRoot;
    use super::character_wheel::{
        CommittedWheelChange, TEMPLE_REMOVAL_RADIUS, WheelAllocation, WheelChangeFacts,
        WheelChangeRequest, WheelCommitOutcome, WheelIneligibility, WheelOccurrence,
        WheelResetOutcome, WheelRuleset, WheelSlots, WheelStages, admit_character_wheel,
    };

    #[test]
    fn character_wheel_api_is_linked() {
        let _ = std::mem::size_of::<CommittedWheelChange>();
        let _ = std::mem::size_of::<WheelAllocation>();
        let _ = std::mem::size_of::<WheelChangeFacts>();
        let _ = std::mem::size_of::<WheelChangeRequest>();
        let _ = std::mem::size_of::<WheelCommitOutcome>();
        let _ = std::mem::size_of::<WheelResetOutcome>();
        let _ = WheelIneligibility::NotPremium;
        let _ = TEMPLE_REMOVAL_RADIUS;
        let _ = WheelSlots::points;
        let _ = WheelOccurrence::as_bytes;
        let _ = WheelRuleset::from_catalogue;
        let _ = WheelStages::derive;
        let _ = DurabilityRoot::commit_character_wheel;
        let _ = DurabilityRoot::reset_character_wheel;
        let _ = DurabilityRoot::reconcile_character_wheel;
        let _ = DurabilityRoot::read_character_wheel;
        let _ = admit_character_wheel;
    }
}

#[cfg(test)]
mod premium_fence_linkage {
    use super::DurabilityRoot;
    use super::premium_fence::{
        EntitlementState, PremiumEvidence, PremiumFenceOutcome, PremiumFenceView,
    };

    #[test]
    fn premium_fence_api_is_linked() {
        let _ = std::mem::size_of::<EntitlementState>();
        let _ = std::mem::size_of::<PremiumEvidence>();
        let _ = std::mem::size_of::<PremiumFenceOutcome>();
        let _ = std::mem::size_of::<PremiumFenceView>();
        let _ = DurabilityRoot::accept_premium_evidence;
        let _ = DurabilityRoot::load_premium_fence;
    }
}

#[cfg(test)]
mod item_mint_linkage {
    use super::DurabilityRoot;
    use super::item_mint::ItemMintCandidate;

    #[test]
    fn item_mint_api_is_linked() {
        let _ = ItemMintCandidate::transaction_id;
        let _ = ItemMintCandidate::event_id;
        let _ = ItemMintCandidate::item_instance_id;
        let _ = ItemMintCandidate::occurred_at_unix_ms;
        let _ = ItemMintCandidate::envelope;
        let _ = ItemMintCandidate::work_units_used;
        let _ = DurabilityRoot::freeze_item_mint;
        let _ = DurabilityRoot::commit_item_mint;
        let _ = DurabilityRoot::reconcile_item_mint;
        let _ = DurabilityRoot::read_item_instance;
    }
}

#[cfg(test)]
mod item_transfer_linkage {
    use super::DurabilityRoot;
    use super::item_transfer::ItemTransferCandidate;

    #[test]
    fn item_transfer_api_is_linked() {
        let _ = ItemTransferCandidate::transaction_id;
        let _ = ItemTransferCandidate::event_id;
        let _ = ItemTransferCandidate::occurred_at_unix_ms;
        let _ = ItemTransferCandidate::work_units_used;
        let _ = DurabilityRoot::freeze_item_transfer;
        let _ = DurabilityRoot::commit_item_transfer;
        let _ = DurabilityRoot::reconcile_item_transfer;
        let _ = DurabilityRoot::read_character_backpack;
        let _ = DurabilityRoot::read_item_source_location;
    }
}

#[cfg(test)]
mod item_decay_retire_linkage {
    use super::DurabilityRoot;
    use super::item_decay_retire::{
        CorpseDecayReport, CorpseDecaySchedule, DecayRetireCandidate, DecayRetireOutcome,
    };

    #[test]
    fn item_decay_retire_api_is_linked() {
        let _ = DecayRetireCandidate::step;
        let _ = DecayRetireCandidate::transaction_id;
        let _ = DecayRetireCandidate::event_id;
        let _ = DecayRetireCandidate::occurred_at_unix_ms;
        let _ = DecayRetireCandidate::deadline_unix_ms;
        let _ = DecayRetireCandidate::envelope;
        let _ = DecayRetireCandidate::work_units_used;
        let _ = DecayRetireOutcome::into_committed;
        let _ = std::mem::size_of::<CorpseDecayReport>();
        let _ = std::mem::size_of::<CorpseDecaySchedule>();
        let _ = DurabilityRoot::freeze_decay_retire;
        let _ = DurabilityRoot::commit_decay_retire;
        let _ = DurabilityRoot::reconcile_decay_retire;
        let _ = DurabilityRoot::read_corpse_decay_schedule;
        let _ = DurabilityRoot::retire_decayed_corpse;
    }
}

#[cfg(test)]
mod item_fee_burn_linkage {
    use super::item_fee_burn::{
        BurnedCoinStack, CommittedFeeBurn, FeeBurnCause, FeeBurnError, FeeBurnOutcome,
        FeeBurnRequest, burn_fee_in_transaction,
    };

    #[test]
    fn item_fee_burn_api_is_linked() {
        let _ = std::mem::size_of::<BurnedCoinStack>();
        let _ = std::mem::size_of::<CommittedFeeBurn>();
        let _ = std::mem::size_of::<FeeBurnCause>();
        let _ = std::mem::size_of::<FeeBurnError>();
        let _ = std::mem::size_of::<FeeBurnOutcome>();
        let _ = std::mem::size_of::<FeeBurnRequest>();
        let _ = burn_fee_in_transaction;
    }
}

#[cfg(test)]
mod reward_claim_mint_linkage {
    use super::DurabilityRoot;
    use super::reward_claim_mint::RewardClaimMintCandidate;

    #[test]
    fn reward_claim_mint_api_is_linked() {
        let _ = RewardClaimMintCandidate::transaction_id;
        let _ = RewardClaimMintCandidate::event_id;
        let _ = RewardClaimMintCandidate::item_instance_id;
        let _ = RewardClaimMintCandidate::occurred_at_unix_ms;
        let _ = RewardClaimMintCandidate::work_units_used;
        let _ = DurabilityRoot::freeze_reward_claim_mint;
        let _ = DurabilityRoot::commit_reward_claim_mint;
        let _ = DurabilityRoot::reconcile_reward_claim_mint;
    }
}

#[cfg(test)]
mod fresh_admission_composition_linkage {
    use super::DurabilityRoot;
    use super::fresh_admission_composition::{
        COMPOSITION_SOURCE_AUTHORITY, FreshAdmissionComposition, FreshAdmissionSubject,
    };

    #[test]
    fn fresh_admission_composition_api_is_linked() {
        assert!(!COMPOSITION_SOURCE_AUTHORITY.is_empty());
        let _ = std::mem::size_of::<FreshAdmissionComposition>();
        let _ = std::mem::size_of::<FreshAdmissionSubject>();
        let _ = DurabilityRoot::publish_fresh_admission_sources;
        let _ = DurabilityRoot::compose_fresh_admission;
        let _ = DurabilityRoot::commit_composed_fresh_admission;
        let _ = super::fresh_admission::FreshAdmissionStore::apply_registered_complete_reconnect;
    }
}

#[cfg(test)]
mod character_forge_dust_linkage {
    use super::character_forge_dust::{
        ForgeDustEntry, ForgeDustGainCause, ForgeDustOutcome, ForgeDustSpendCause,
        ForgeDustWriteError, ForgeDustWriteIds, gain_forge_dust_in_transaction, read_forge_dust,
        spend_forge_dust_in_transaction,
    };
    use super::character_forge_dust_audit::{
        ForgeDustAuditError, ForgeDustLedgerAudit, audit_forge_dust_ledger,
    };

    #[test]
    fn character_forge_dust_api_is_linked() {
        let _ = std::mem::size_of::<ForgeDustEntry>();
        let _ = std::mem::size_of::<ForgeDustGainCause>();
        let _ = std::mem::size_of::<ForgeDustOutcome>();
        let _ = std::mem::size_of::<ForgeDustSpendCause>();
        let _ = std::mem::size_of::<ForgeDustWriteError>();
        let _ = std::mem::size_of::<ForgeDustWriteIds>();
        let _ = std::mem::size_of::<ForgeDustAuditError>();
        let _ = std::mem::size_of::<ForgeDustLedgerAudit>();
        let _ = spend_forge_dust_in_transaction;
        let _ = gain_forge_dust_in_transaction;
        let _ = read_forge_dust;
        let _ = audit_forge_dust_ledger;
    }
}

#[cfg(test)]
mod recovery_evidence_composition_linkage {
    use super::DurabilityRoot;
    use super::recovery_evidence_composition::RecoveryEvidenceSubject;

    #[test]
    fn recovery_evidence_composition_api_is_linked() {
        let _ = RecoveryEvidenceSubject::new("01890f4c-3b2a-7cc2-8d11-9a321b7c0001", "key");
        let _ = DurabilityRoot::verify_registered_recovery;
        let _ = DurabilityRoot::revalidate_registered_recovery;
    }
}

#[cfg(test)]
mod native_admission_source_linkage {
    use super::DurabilityRoot;
    use super::native_admission_source::{
        DescriptorRegistration, FreshStoreProvenance, NativeSourceOperation, NativeSourceSubject,
        PendingPublication, SourceObservation,
    };

    #[test]
    fn native_admission_source_api_is_linked() {
        let _ = std::mem::size_of::<FreshStoreProvenance>();
        let _ = std::mem::size_of::<DescriptorRegistration>();
        let _ = std::mem::size_of::<SourceObservation>();
        let _ = std::mem::size_of::<PendingPublication>();
        let _ = [
            NativeSourceOperation::ReadAccountSecurityV1,
            NativeSourceOperation::ReadFreshSigningTrustV1,
            NativeSourceOperation::ReadRecoveryAccountSecurityV2,
            NativeSourceOperation::ReadRecoverySigningTrustV2,
        ];
        let _ = NativeSourceOperation::parse("ReadAccountSecurityV1");
        let _ = NativeSourceSubject::account_security("01890f4c-3b2a-7cc2-8d11-9a321b7c0001");
        let _ = NativeSourceSubject::signing_trust(
            "urn:oteryn:platform:game-admission",
            "oteryn-pre-admission-v1",
            "fresh_admission",
            "key",
        );
        let _ = DurabilityRoot::initialize_native_admission_source;
        let _ = DurabilityRoot::claim_native_admission_source_custody;
        let _ = DurabilityRoot::register_native_admission_descriptor;
        let _ = DurabilityRoot::accept_native_source_observation;
        let _ = DurabilityRoot::checkpoint_native_source_publication;
        let _ = DurabilityRoot::clear_native_source_publication;
        let _ = DurabilityRoot::pending_native_source_publications;
        let _ = DurabilityRoot::record_native_source_descriptor_issuance;
        let _ = DurabilityRoot::read_native_admission_source_registration;
    }
}

#[cfg(test)]
mod account_characters_projection_linkage {
    use super::DurabilityRoot;
    use super::account_characters_projection::{AccountCharactersStore, MAX_CHARACTER_TRANSACTION};

    #[test]
    fn account_characters_projection_api_is_linked() {
        assert_eq!(
            MAX_CHARACTER_TRANSACTION,
            super::DB_PASS_DEADLINE + std::time::Duration::from_secs(1)
        );
        let _ = std::mem::size_of::<AccountCharactersStore<'_, '_, '_, '_>>();
        let _ = DurabilityRoot::next_account_characters_snapshot;
        let _ = DurabilityRoot::clear_account_characters;
        let _ = DurabilityRoot::account_characters_watermark_facts;
    }
}

#[cfg(test)]
mod public_projections_linkage {
    use super::DurabilityRoot;
    use super::public_projections::PublicProjectionsStore;

    #[test]
    fn public_projections_api_is_linked() {
        let _ = std::mem::size_of::<PublicProjectionsStore<'_, '_, '_, '_>>();
        let _ = DurabilityRoot::next_public_profiles;
        let _ = DurabilityRoot::clear_public_profiles;
        let _ = DurabilityRoot::public_outbox_facts;
        let _ = DurabilityRoot::next_public_highscores;
        let _ = DurabilityRoot::clear_public_highscores;
        let _ = DurabilityRoot::public_online_cut;
        let _ = DurabilityRoot::persist_public_online;
        let _ = DurabilityRoot::ack_public_online;
    }
}

#[cfg(test)]
mod character_authority_linkage {
    use super::DurabilityRoot;
    use super::character_authority::{
        CharacterAuditDelivery, CharacterAuthorityError, CharacterAuthorityRecord,
        ReconciledCharacterAuthority,
    };

    #[test]
    fn character_authority_api_is_linked() {
        let _ = DurabilityRoot::configure_character_interpretation;
        let _ =
            std::mem::size_of::<crate::character_bootstrap_intent::CharacterBootstrapIntentV1>();
        let _ = std::mem::size_of::<CharacterAuditDelivery>();
        let _ = std::mem::size_of::<CharacterAuthorityError>();
        let _ = std::mem::size_of::<CharacterAuthorityRecord>();
        let _ = std::mem::size_of::<ReconciledCharacterAuthority<'_, '_>>();
        let _ = CharacterAuthorityError::Rejected;
        let _ = CharacterAuthorityError::Conflict;
        let _ = CharacterAuthorityError::NameUnavailable;
        let _ = |error: CharacterAuthorityError| match error {
            CharacterAuthorityError::Unavailable(inner) => Some(inner),
            _ => None,
        };
        let _ = DurabilityRoot::bootstrap_character;
        let _ = DurabilityRoot::reconcile_character_bootstrap;
        let _ = crate::character_bootstrap_intent::read_authenticated_intent;
        let _ = crate::character_bootstrap_intent::CharacterInterpretationV1::new;
        let _ = DurabilityRoot::open_character_authority;
        let _ = DurabilityRoot::read_current_character;
        let _ = DurabilityRoot::admit_fresh_character_recovery;
        let _ = DurabilityRoot::reconcile_character_recovery;
        let _ = DurabilityRoot::pending_character_audit;
        let _ = DurabilityRoot::acknowledge_character_audit;
        let _ = DurabilityRoot::expire_character_audit;
        let _ = oteryn_gameplay_characters::character_authority_audit::encode_bootstrap;
    }
}

#[cfg(test)]
mod runtime_scope_assignment_linkage {
    use super::DurabilityRoot;
    use super::runtime_scope_assignment::{
        AssignmentCommand, AssignmentError, AssignmentOutcome, AssignmentPredecessor,
        AssignmentReceipt, AssignmentRejection, AssignmentRequest, AssignmentState,
        BootstrapSecret, ControlActor, LaunchBinding, NodeRegistrationFact, OperationKey,
        ReconcileOutcome, RegistrationError, RuntimeScopeAssignment, RuntimeScopeAssignmentWriter,
    };

    #[test]
    fn runtime_scope_assignment_api_is_linked() {
        let _ = std::mem::size_of::<AssignmentCommand>();
        let _ = std::mem::size_of::<AssignmentError>();
        let _ = std::mem::size_of::<AssignmentOutcome>();
        let _ = std::mem::size_of::<AssignmentPredecessor>();
        let _ = std::mem::size_of::<AssignmentReceipt>();
        let _ = std::mem::size_of::<AssignmentRejection>();
        let _ = std::mem::size_of::<AssignmentRequest>();
        let _ = std::mem::size_of::<AssignmentState>();
        let _ = std::mem::size_of::<ReconcileOutcome>();
        let _ = std::mem::size_of::<RegistrationError>();
        let _ = std::mem::size_of::<RuntimeScopeAssignment>();
        let _ = BootstrapSecret::from_bytes([7; 32]);
        let _ = LaunchBinding::new("launch-1").map(|launch| launch.as_str().len());
        let _ = ControlActor::new("operator");
        let key = OperationKey::from_bytes([1; 32]);
        assert_eq!(OperationKey::from_text(&key.to_text()).ok(), Some(key));
        let _ = key.as_bytes();
        let _ = NodeRegistrationFact::new;
        let _ = NodeRegistrationFact::node_id;
        let _ = NodeRegistrationFact::registration_revision;
        let _ = super::runtime_scope_assignment::AssignmentReceipt::predecessor;
        let _ = DurabilityRoot::read_runtime_scope_predecessor;
        let _ = AssignmentRequest::encode;
        let _ = DurabilityRoot::issue_node_bootstrap_authorization;
        let _ = DurabilityRoot::register_node_incarnation;
        let _ = DurabilityRoot::require_current_node_registration;
        let _ = DurabilityRoot::revoke_node_registration;
        let _ = DurabilityRoot::revoke_node_bootstrap_authorization;
        let _ = DurabilityRoot::read_runtime_scope_assignment;
        let _ = DurabilityRoot::publish_runtime_readiness;
        let _ = super::runtime_scope_assignment::NodeIncarnationProof::new;
        let _ = super::runtime_scope_assignment::NodeIncarnationProof::fact;
        let _ = AssignmentError::NotCurrentHolder;
        let _ = RuntimeScopeAssignmentWriter::open;
        let _ = RuntimeScopeAssignmentWriter::unreconciled;
        let _ = RuntimeScopeAssignmentWriter::submit;
        let _ = RuntimeScopeAssignmentWriter::reconcile;
        if let RegistrationError::Unavailable(inner) =
            RegistrationError::from(super::DurabilityError::Unavailable)
        {
            assert!(matches!(inner, super::DurabilityError::Unavailable));
        }
        if let AssignmentError::Unavailable(inner) =
            AssignmentError::from(super::DurabilityError::Unavailable)
        {
            assert!(matches!(inner, super::DurabilityError::Unavailable));
        }
        let uuid = [1, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, 16];
        let (Ok(world), Ok(channel), Ok(node)) = (
            crate::foundation::WorldId::decode(&uuid),
            crate::foundation::ChannelId::decode(&uuid),
            crate::foundation::NodeId::decode(&uuid),
        ) else {
            return;
        };
        let scope = crate::foundation::RuntimeScopeRefV1::channel(world, channel);
        let target = NodeRegistrationFact::new(node, 1);
        let predecessor = AssignmentPredecessor {
            ownership_generation: 1,
            source_revision: 1,
            runtime_guard_publication_revision: Some(1),
        };
        let Ok(actor) = ControlActor::new("operator") else {
            return;
        };
        for command in [
            AssignmentCommand::Assign { scope, target },
            AssignmentCommand::Replace {
                scope,
                predecessor,
                target,
            },
            AssignmentCommand::Revoke { scope, predecessor },
        ] {
            let request = AssignmentRequest {
                operation_key: key,
                actor: actor.clone(),
                command,
            };
            assert!(request.encode().is_ok_and(|encoded| encoded.len() <= 1024));
        }
    }
}
