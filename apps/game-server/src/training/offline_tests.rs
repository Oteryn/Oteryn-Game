//! Plain proposed-data transitions and native formula previews, not authority/DB receipt tests.
#![allow(clippy::expect_used, clippy::panic)]
use super::*;
use crate::domain::{CharacterId, CharacterRevision};
use crate::foundation::{ChannelId, ConnectionGeneration, ScopeOwnershipGeneration, WorldId};
use crate::spell::mana_training::CompiledTrainingFormula;

const ACQUIRED: i64 = 3_000_000_000_000;
fn id(seed: u8) -> [u8; 16] {
    [
        seed, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, seed,
    ]
}
fn fence() -> CurrentCharacterGameplayFence {
    CurrentCharacterGameplayFence {
        character_id: CharacterId::from_bytes(id(41)).expect("character"),
        game_session_id: GameSessionId::decode(&id(50)).expect("session"),
        connection_generation: ConnectionGeneration::new(1).expect("generation"),
        character_lease_generation: 9,
        runtime_scope: RuntimeScopeRefV1::channel(
            WorldId::decode(&id(42)).expect("world"),
            ChannelId::decode(&id(43)).expect("channel"),
        ),
        scope_ownership_generation: ScopeOwnershipGeneration::new(1).expect("scope generation"),
        expected_character_revision: CharacterRevision::new(1).expect("revision"),
    }
}
fn lease() -> ProposedLeaseTimes {
    ProposedLeaseTimes {
        acquired_at_us: ACQUIRED,
        session_created_at_us: ACQUIRED,
        predecessor: Some(ProposedPredecessor {
            generation: 7,
            session: GameSessionId::decode(&id(51)).expect("old session"),
        }),
    }
}
fn build() -> DurableBuildState {
    DurableBuildState::new("druid", (0, 0), [(10, 0); 7]).expect("native build")
}
fn formula() -> CompiledTrainingFormula {
    CompiledTrainingFormula::from_profile(
        include_bytes!("../../../../tools/content-schema/native-gameplay/build-training.json"),
        "actual-content-r1",
    )
    .expect("native pinned formula")
}
fn logout(d: u64) -> LatestBuildReceipt {
    LatestBuildReceipt::Logout(ProposedLogoutMarker {
        key: MarkerKey {
            character: fence().character_id,
            lease: 7,
            kind: MarkerKind::Logout,
        },
        session: lease().predecessor.expect("predecessor").session,
        scope: fence().runtime_scope,
        as_of_us: ACQUIRED - i64::try_from(d).expect("seconds") * SECOND_US,
        committed_at_us: ACQUIRED - i64::try_from(d).expect("seconds") * SECOND_US,
    })
}
fn settle(latest: LatestBuildReceipt, pool: u32, skill: Option<OfflineSkill>) -> SettlementPlan {
    let result = plan_settlement(
        &fence(),
        lease(),
        ACQUIRED + SECOND_US,
        ProposedLookup::Missing,
        AdmissionKind::NewLease,
        Respawn::Resolved,
        ProgressionRow::Present,
        latest,
        ProposedOfflineColumns {
            stamina_minutes: 2340,
            pool_s: pool,
            skill,
        },
        &build(),
    )
    .expect("plan");
    let SettlementDisposition::Plan(plan) = result else {
        panic!("expected a concrete pending plan");
    };
    *plan
}

#[test]
fn offline_immediate_predecessor_latest_receipt_and_crash_d0() {
    let p = settle(logout(1200), 1000, Some(OfflineSkill::Sword));
    assert_eq!(p.offline_s, 1200);
    assert_eq!(p.skill_after, None);
    // 7 is the independently supplied predecessor of9; do not infer predecessor=8.
    for latest in [
        LatestBuildReceipt::Absent,
        LatestBuildReceipt::OtherBuildCarryingReceipt,
    ] {
        let p = settle(latest, 1000, Some(OfflineSkill::Sword));
        assert_eq!(
            (p.offline_s, p.pool_after_s, p.training),
            (0, 1000, TrainingWork::None)
        );
        assert_eq!(p.skill_after, None);
    }
    let LatestBuildReceipt::Logout(mut old) = logout(1200) else {
        panic!("expected proposed logout fixture");
    };
    old.key.lease = 6;
    let p = settle(
        LatestBuildReceipt::Logout(old),
        1000,
        Some(OfflineSkill::Sword),
    );
    assert_eq!((p.offline_s, p.pool_after_s), (0, 1000));
}
#[test]
fn offline_training_600_and_60_second_pool_boundaries() {
    let p = settle(logout(599), 1000, Some(OfflineSkill::Fist));
    assert_eq!(p.pool_after_s, 1000);
    assert_eq!(p.training, TrainingWork::None);
    for (pool, after, trained) in [
        (0, 600, None),
        (59, 541, None),
        (60, 540, Some(60)),
        (600, 0, Some(600)),
        (1000, 400, Some(600)),
    ] {
        let p = settle(logout(600), pool, Some(OfflineSkill::Fist));
        assert_eq!(p.pool_after_s, after);
        assert_eq!(p.skill_after, None);
        match (p.training, trained) {
            (TrainingWork::None, None) => {}
            (TrainingWork::NeedsRates { seconds, .. }, Some(expected)) => {
                assert_eq!(seconds, expected)
            }
            other => panic!("unexpected duration {other:?}"),
        }
    }
    assert!(matches!(
        settle(logout(600), 60, Some(OfflineSkill::Distance)).training,
        TrainingWork::NeedsRates {
            owner: TrainingRateOwner::TopVocationBaseAttackSpeedTimesFour,
            ..
        }
    ));
    assert!(matches!(
        settle(logout(600), 60, Some(OfflineSkill::MagicLevel)).training,
        TrainingWork::NeedsRates {
            owner: TrainingRateOwner::CurrentManaGainAndTopManaTicksWithOneSecondZeroFallback,
            ..
        }
    ));
}
#[test]
fn offline_21day_cap_pool_restore_and_stamina_owner_remain_explicit() {
    let p = settle(
        logout(OFFLINE_CAP_S + 1000),
        POOL_CAP_S,
        Some(OfflineSkill::MagicLevel),
    );
    assert_eq!(p.offline_s, OFFLINE_CAP_S);
    assert_eq!(p.pool_after_s, POOL_CAP_S);
    assert!(matches!(
        p.training,
        TrainingWork::NeedsRates {
            seconds: POOL_CAP_S,
            ..
        }
    ));
    let none = settle(logout(600), 100, None);
    assert_eq!(none.pool_after_s, 700);
    assert_eq!(none.training, TrainingWork::None);
    for stamina in [0, 2339, 2340, 2341, 2519, 2520] {
        let result = plan_settlement(
            &fence(),
            lease(),
            ACQUIRED + SECOND_US,
            ProposedLookup::Missing,
            AdmissionKind::NewLease,
            Respawn::Resolved,
            ProgressionRow::Present,
            logout(960),
            ProposedOfflineColumns {
                stamina_minutes: stamina,
                pool_s: 0,
                skill: None,
            },
            &build(),
        )
        .expect("plan");
        assert!(
            matches!(result,SettlementDisposition::Plan(p) if p.stamina==StaminaOwnerWork {before_minutes:stamina,offline_s:960} && p.commit==MarkerCommitRequired::NativeRecoveryCurrentFencesClockCauseAndChain)
        );
    }
}
#[test]
fn offline_uses_acquisition_not_later_asof_and_clamps_future_logout_time() {
    let first = settle(logout(600), 1000, Some(OfflineSkill::Axe));
    let result = plan_settlement(
        &fence(),
        lease(),
        ACQUIRED + 10000 * SECOND_US,
        ProposedLookup::Missing,
        AdmissionKind::NewLease,
        Respawn::Resolved,
        ProgressionRow::Present,
        logout(600),
        ProposedOfflineColumns {
            stamina_minutes: 2340,
            pool_s: 1000,
            skill: Some(OfflineSkill::Axe),
        },
        &build(),
    )
    .expect("later clock");
    assert!(
        matches!(result,SettlementDisposition::Plan(p) if p.offline_s==first.offline_s && p.pool_after_s==first.pool_after_s)
    );
    let LatestBuildReceipt::Logout(mut marker) = logout(0) else {
        panic!("expected proposed logout fixture");
    };
    marker.as_of_us += 1;
    marker.committed_at_us += 1;
    let p = settle(
        LatestBuildReceipt::Logout(marker),
        1000,
        Some(OfflineSkill::Axe),
    );
    assert_eq!(p.offline_s, 0);
}
#[test]
fn offline_replay_precedes_raw_binding_time_columns_and_activation() {
    let f = fence();
    let settlement_key = key(&f, MarkerKind::Settlement).expect("key");
    let logout_key = key(&f, MarkerKind::Logout).expect("key");
    let mut bad = lease();
    bad.acquired_at_us = -1;
    let invalid = ProposedOfflineColumns {
        stamina_minutes: 65535,
        pool_s: u32::MAX,
        skill: Some(OfflineSkill::MagicLevel),
    };
    assert_eq!(
        plan_settlement(
            &f,
            bad,
            -5,
            ProposedLookup::Existing(settlement_key),
            AdmissionKind::NewLease,
            Respawn::Resolved,
            ProgressionRow::Absent,
            LatestBuildReceipt::Absent,
            invalid,
            &build()
        ),
        Ok(SettlementDisposition::ReadExistingNativeMarker(
            settlement_key
        ))
    );
    assert_eq!(
        plan_logout(
            &f,
            bad,
            -5,
            ProposedLookup::Existing(logout_key),
            ProposedLookup::Missing,
            ActorEnd::StillInWorld,
            Respawn::Resolved,
            ProgressionRow::Present,
            invalid,
            None
        ),
        Ok(LogoutDisposition::ReadExistingNativeMarker(logout_key))
    );
    assert_eq!(
        plan_settlement(
            &f,
            lease(),
            ACQUIRED,
            ProposedLookup::Existing(logout_key),
            AdmissionKind::NewLease,
            Respawn::Resolved,
            ProgressionRow::Present,
            LatestBuildReceipt::Absent,
            ProposedOfflineColumns::default(),
            &build()
        ),
        Err(PlanError::MismatchedMarker)
    );
}
#[test]
fn offline_waits_respawn_reconnect_house_and_requires_initializer() {
    let f = fence();
    let call = |admission, respawn, progression| {
        plan_settlement(
            &f,
            lease(),
            ACQUIRED,
            ProposedLookup::Missing,
            admission,
            respawn,
            progression,
            LatestBuildReceipt::Absent,
            ProposedOfflineColumns::default(),
            &build(),
        )
        .expect("disposition")
    };
    assert_eq!(
        call(
            AdmissionKind::SameLeaseReconnect,
            Respawn::Resolved,
            ProgressionRow::Present
        ),
        SettlementDisposition::SameLeaseReconnect
    );
    assert_eq!(
        call(
            AdmissionKind::NewLease,
            Respawn::Pending,
            ProgressionRow::Present
        ),
        SettlementDisposition::WaitForRespawn
    );
    assert!(matches!(
        call(
            AdmissionKind::NewLease,
            Respawn::Resolved,
            ProgressionRow::Absent
        ),
        SettlementDisposition::Plan(p) if p.initialize_native_progression_in_same_transaction==ProgressionRow::Absent
    ));
    let mut instance = f;
    instance.runtime_scope =
        RuntimeScopeRefV1::instance(f.runtime_scope.world_id(), id(80)).expect("instance");
    assert_eq!(
        plan_settlement(
            &instance,
            lease(),
            ACQUIRED,
            ProposedLookup::Missing,
            AdmissionKind::NewLease,
            Respawn::Resolved,
            ProgressionRow::Present,
            LatestBuildReceipt::Absent,
            ProposedOfflineColumns::default(),
            &build()
        ),
        Ok(SettlementDisposition::NeedsHouseScopeOwner)
    );
}
#[test]
fn offline_marker_and_lease_identity_time_negatives_are_independent() {
    let LatestBuildReceipt::Logout(original) = logout(600) else {
        panic!("expected proposed logout fixture");
    };
    for mutation in 0..5 {
        let mut marker = original;
        match mutation {
            0 => marker.key.character = CharacterId::from_bytes(id(99)).expect("other"),
            1 => marker.session = GameSessionId::decode(&id(99)).expect("other"),
            2 => {
                marker.scope = RuntimeScopeRefV1::channel(
                    WorldId::decode(&id(99)).expect("other world"),
                    ChannelId::decode(&id(43)).expect("channel"),
                )
            }
            3 => marker.as_of_us -= 1,
            _ => marker.key.lease = 0,
        }
        assert!(
            plan_settlement(
                &fence(),
                lease(),
                ACQUIRED,
                ProposedLookup::Missing,
                AdmissionKind::NewLease,
                Respawn::Resolved,
                ProgressionRow::Present,
                LatestBuildReceipt::Logout(marker),
                ProposedOfflineColumns::default(),
                &build()
            )
            .is_err()
        );
    }
    let mut wrong = lease();
    wrong.predecessor.as_mut().expect("pred").generation = 9;
    assert_eq!(
        plan_settlement(
            &fence(),
            wrong,
            ACQUIRED,
            ProposedLookup::Missing,
            AdmissionKind::NewLease,
            Respawn::Resolved,
            ProgressionRow::Present,
            logout(600),
            ProposedOfflineColumns::default(),
            &build()
        ),
        Err(PlanError::InvalidLease)
    );
    for columns in [
        ProposedOfflineColumns {
            stamina_minutes: 2521,
            ..ProposedOfflineColumns::default()
        },
        ProposedOfflineColumns {
            pool_s: 43201,
            ..ProposedOfflineColumns::default()
        },
    ] {
        assert_eq!(
            plan_settlement(
                &fence(),
                lease(),
                ACQUIRED,
                ProposedLookup::Missing,
                AdmissionKind::NewLease,
                Respawn::Resolved,
                ProgressionRow::Present,
                logout(600),
                columns,
                &build()
            ),
            Err(PlanError::InvalidColumns)
        );
    }
    assert_eq!(
        plan_settlement(
            &fence(),
            lease(),
            ACQUIRED - 1,
            ProposedLookup::Missing,
            AdmissionKind::NewLease,
            Respawn::Resolved,
            ProgressionRow::Present,
            logout(600),
            ProposedOfflineColumns::default(),
            &build()
        ),
        Err(PlanError::InvalidTime)
    );
}
#[test]
fn offline_logout_restore_uses_current_session_only_and_requires_its_settlement() {
    let f = fence();
    let columns = ProposedOfflineColumns {
        stamina_minutes: 1000,
        pool_s: 100,
        skill: None,
    };
    let settled = ProposedLookup::Existing(key(&f, MarkerKind::Settlement).expect("key"));
    for (lookup, expected) in [(ProposedLookup::Missing, 100), (settled, 700)] {
        let result = plan_logout(
            &f,
            lease(),
            ACQUIRED + 600 * SECOND_US + 999999,
            ProposedLookup::Missing,
            lookup,
            ActorEnd::RemovedBeforeTerminalRelease,
            Respawn::Resolved,
            ProgressionRow::Present,
            columns,
            None,
        )
        .expect("logout");
        assert!(
            matches!(result,LogoutDisposition::Plan(LogoutPlan {pool_after_s,flush:FinalFlushOwnerStage::NativePendingTrainingAndStaminaAfterActorRemoval,..}) if pool_after_s==expected)
        );
    }
    let mut replacement = lease();
    replacement.session_created_at_us = ACQUIRED + 599 * SECOND_US;
    let result = plan_logout(
        &f,
        replacement,
        ACQUIRED + 600 * SECOND_US,
        ProposedLookup::Missing,
        settled,
        ActorEnd::RemovedBeforeTerminalRelease,
        Respawn::Resolved,
        ProgressionRow::Present,
        columns,
        None,
    )
    .expect("replacement session");
    assert!(matches!(
        result,
        LogoutDisposition::Plan(LogoutPlan {
            online_restore_s: 1,
            pool_after_s: 101,
            ..
        })
    ));
    assert!(matches!(
        plan_logout(
            &f,
            lease(),
            ACQUIRED + 90000 * SECOND_US,
            ProposedLookup::Missing,
            settled,
            ActorEnd::RemovedBeforeTerminalRelease,
            Respawn::Resolved,
            ProgressionRow::Present,
            columns,
            None
        )
        .expect("cap"),
        LogoutDisposition::Plan(LogoutPlan {
            pool_after_s: POOL_CAP_S,
            ..
        })
    ));
}
#[test]
fn offline_activation_uses_native_premium_gate_and_only_a_logout_proposal() {
    let f = fence();
    assert_eq!(
        plan_statue_activation(
            &f,
            &PremiumStatus::NotCurrent,
            OfflineSkill::Sword,
            ActivationBlock::Clear
        ),
        Err(ActivationRefusal::Sealed)
    );
    for block in [ActivationBlock::LogoutBlocked, ActivationBlock::PzLocked] {
        assert_eq!(
            plan_statue_activation(&f, &PremiumStatus::Current, OfflineSkill::Sword, block),
            Err(ActivationRefusal::PzBlocked)
        );
    }
    let activation = plan_statue_activation(
        &f,
        &PremiumStatus::Current,
        OfflineSkill::Sword,
        ActivationBlock::Clear,
    )
    .expect("proposal");
    let result = plan_logout(
        &f,
        lease(),
        ACQUIRED,
        ProposedLookup::Missing,
        ProposedLookup::Missing,
        ActorEnd::RemovedBeforeTerminalRelease,
        Respawn::Resolved,
        ProgressionRow::Present,
        ProposedOfflineColumns::default(),
        Some(activation),
    )
    .expect("logout proposal");
    assert!(matches!(
        result,
        LogoutDisposition::Plan(LogoutPlan {
            skill_after: Some(OfflineSkill::Sword),
            activation: Some(ProposedActivation {
                still_required:
                    ActivationOwnerStage::CurrentPremiumAndAdmittedStatueWorldUseReachPzCooldown,
                ..
            }),
            ..
        })
    ));
    for (end, respawn, progression) in [
        (
            ActorEnd::AlreadyReleased,
            Respawn::Resolved,
            ProgressionRow::Present,
        ),
        (
            ActorEnd::RemovedBeforeTerminalRelease,
            Respawn::Pending,
            ProgressionRow::Present,
        ),
        (
            ActorEnd::RemovedBeforeTerminalRelease,
            Respawn::Resolved,
            ProgressionRow::Absent,
        ),
    ] {
        assert_eq!(
            plan_logout(
                &f,
                lease(),
                ACQUIRED,
                ProposedLookup::Missing,
                ProposedLookup::Missing,
                end,
                respawn,
                progression,
                ProposedOfflineColumns::default(),
                Some(activation)
            ),
            Ok(LogoutDisposition::SkippedNoActivation)
        );
    }
    let mut wrong = activation;
    wrong.expected.game_session_id = GameSessionId::decode(&id(99)).expect("other session");
    assert_eq!(
        plan_logout(
            &f,
            lease(),
            ACQUIRED,
            ProposedLookup::Missing,
            ProposedLookup::Missing,
            ActorEnd::RemovedBeforeTerminalRelease,
            Respawn::Resolved,
            ProgressionRow::Present,
            ProposedOfflineColumns::default(),
            Some(wrong)
        ),
        Err(PlanError::MismatchedActivation)
    );
    for mutation in 0..3 {
        let mut current = f;
        match mutation {
            0 => {
                current.connection_generation =
                    ConnectionGeneration::new(2).expect("new connection")
            }
            1 => {
                current.scope_ownership_generation =
                    ScopeOwnershipGeneration::new(2).expect("new scope owner")
            }
            _ => {
                current.runtime_scope = RuntimeScopeRefV1::channel(
                    f.runtime_scope.world_id(),
                    ChannelId::decode(&id(44)).expect("other channel"),
                )
            }
        }
        assert_eq!(
            plan_logout(
                &current,
                lease(),
                ACQUIRED,
                ProposedLookup::Missing,
                ProposedLookup::Missing,
                ActorEnd::RemovedBeforeTerminalRelease,
                Respawn::Resolved,
                ProgressionRow::Present,
                ProposedOfflineColumns::default(),
                Some(activation)
            ),
            Err(PlanError::MismatchedActivation)
        );
    }
    let mut sequenced = f;
    sequenced.expected_character_revision = CharacterRevision::new(2).expect("sequenced revision");
    assert!(matches!(
        plan_logout(
            &sequenced,
            lease(),
            ACQUIRED,
            ProposedLookup::Missing,
            ProposedLookup::Missing,
            ActorEnd::RemovedBeforeTerminalRelease,
            Respawn::Resolved,
            ProgressionRow::Present,
            ProposedOfflineColumns::default(),
            Some(activation)
        )
        .expect("native sequencer may advance revision"),
        LogoutDisposition::Plan(_)
    ));
}
#[test]
fn offline_native_relevel_preview_advances_fist_and_magic_without_second_engine() {
    let f = formula();
    let work = settle(logout(600), 60, Some(OfflineSkill::Fist)).training;
    let preview =
        preview_raw_training(&work, RawTrainingDelta::Tries(50), &f).expect("native preview");
    assert_eq!(
        preview.rates_still_required,
        TrainingRateOwner::TopVocationBaseAttackSpeedTimesTwo
    );
    let PercentOwnerStage::LevelChangedWithShielding(after) = preview.result else {
        panic!("native advance");
    };
    assert_eq!(after.skills()[0], (11, 0));
    assert_eq!(after.skills()[5], (10, 15));
    assert_eq!(after.magic(), (0, 0));
    let magic = settle(logout(600), 60, Some(OfflineSkill::MagicLevel)).training;
    let preview =
        preview_raw_training(&magic, RawTrainingDelta::ManaSpent(1600), &f).expect("native magic");
    assert!(
        matches!(preview.result,PercentOwnerStage::LevelChangedWithShielding(after) if after.magic()==(1,0) && after.skills()[5]==(10,15))
    );
}
#[test]
fn offline_percent_owner_is_not_invented_from_changed_raw_progress() {
    let work = settle(logout(600), 60, Some(OfflineSkill::Fist)).training;
    let preview =
        preview_raw_training(&work, RawTrainingDelta::Tries(1), &formula()).expect("raw preview");
    let PercentOwnerStage::NeedsNativePercentProjection {
        without_shielding,
        if_percent_changed,
    } = preview.result
    else {
        panic!("requires percent owner");
    };
    assert_eq!(without_shielding.skills()[0], (10, 1));
    assert_eq!(without_shielding.skills()[5], (10, 0));
    assert_eq!(if_percent_changed.skills()[5], (10, 15));
    assert!(
        matches!(preview_raw_training(&work,RawTrainingDelta::Tries(0),&formula()).expect("zero").result,PercentOwnerStage::NoChange(after) if after==build())
    );
    assert_eq!(
        preview_raw_training(&work, RawTrainingDelta::ManaSpent(1), &formula()),
        Err(PlanError::InvalidGrant)
    );
    assert_eq!(
        preview_raw_training(&TrainingWork::None, RawTrainingDelta::Tries(1), &formula()),
        Err(PlanError::InvalidGrant)
    );
}
