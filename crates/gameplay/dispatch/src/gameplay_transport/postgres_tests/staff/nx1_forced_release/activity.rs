// NX-1 focused staff release case; included in the original test module.
#[test]
fn nx1_staff_activity_return_requires_current_transport_and_signed_decision_postgres() -> TestResult
{
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let began = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| activity_scenario(false).map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "staff activity release panicked")??;
    eprintln!(
        "NX-1 staff Activity return PostgreSQL duration: {:?}",
        began.elapsed()
    );
    Ok(())
}
fn activity_scenario(lost: bool) -> TestResult {
    use crate::durability::activity_handoff::{
        ActivityHandoffOwner, ActivityRefusal, ActivityReturnOutcome, ActivityReturnReason,
        ActivityReturnRequest,
    };
    use crate::foundation::{AuthenticatedTransportRefV1, GameSessionId};
    run(async |admin| {
        let signing = Signing::new()?;
        let h =
            Harness::create_with_admission_seed(admin, "nx1_staff_activity", true, false).await?;
        let entered = super::super::activity_runtime::enter_fixture(&h).await?;
        h.seed_pvp_target().await?;
        crate::gameplay_transport::monk_save::postgres_tests::appearance_change_postgres::observer_visibility_tests::admit(&h).await?;
        signing.bootstrap(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let owner = ActivityHandoffOwner::new(h.root.clone());
        let placement = owner
            .read_return_placement(&authority, &h.node, entered.fence)
            .await?
            .ok_or("placement")?;
        if lost {
            record_activity_loss(&h, entered.fence.game_session_id).await?;
        }
        let current = FreshAdmissionStore::from_root(h.root.clone())
            .current_session(entered.fence.game_session_id)
            .await?;
        let transport = current
            .current_transport()
            .unwrap_or(AuthenticatedTransportRefV1::decode(&[251; 16]).map_err(debug)?);
        let request = ActivityReturnRequest {
            return_id: id(100),
            handoff_id: entered.receipt.handoff_id,
            command_id: None,
            source: entered.fence,
            destination_game_session_id: GameSessionId::decode(&id(101)).map_err(debug)?,
            destination_transport: if lost {
                None
            } else {
                Some(AuthenticatedTransportRefV1::decode(&[101; 16]).map_err(debug)?)
            },
            destination_generation: placement.destination_generation,
            reason: if lost {
                ActivityReturnReason::StaffKickLost {
                    epoch: current.current_control_loss_epoch().ok_or("lost epoch")?,
                }
            } else {
                ActivityReturnReason::StaffKick { transport }
            },
            current_vitals: entered.request.participants[0].source_position.vitals,
            current_rest: entered.request.participants[0].source_position.rest,
        };
        assert_eq!(
            owner
                .return_participant(&authority, &h.node, request.clone())
                .await?,
            ActivityReturnOutcome::Refused(ActivityRefusal::NotDue)
        );
        let issuer = FreshAdmissionStore::from_root(h.root.clone())
            .current_session(GameSessionId::decode(&id(110)).map_err(debug)?)
            .await?;
        let issuer_fence =
            crate::gameplay_transport::item_fence_of(issuer).ok_or("Channel issuer fence")?;
        authorized_kick(&signing, &h, &authority, issuer_fence, 97).await?;
        let mut wrong = request.clone();
        wrong.reason = if lost {
            ActivityReturnReason::StaffKickLost {
                epoch: crate::foundation::ControlLossEpochRefV1::new(2).map_err(debug)?,
            }
        } else {
            ActivityReturnReason::StaffKick {
                transport: AuthenticatedTransportRefV1::decode(&[252; 16]).map_err(debug)?,
            }
        };
        assert_eq!(
            owner.return_participant(&authority, &h.node, wrong).await?,
            ActivityReturnOutcome::Refused(ActivityRefusal::NotDue)
        );
        let returned = match owner
            .return_participant(&authority, &h.node, request.clone())
            .await?
        {
            ActivityReturnOutcome::Returned(receipt) => receipt,
            other => return Err(format!("staff activity return refused: {other:?}").into()),
        };
        assert_eq!(
            owner
                .return_participant(&authority, &h.node, request)
                .await?,
            ActivityReturnOutcome::Replayed(returned.clone())
        );
        let destination = FreshAdmissionStore::from_root(h.root.clone())
            .current_session(returned.destination_game_session_id)
            .await?;
        let destination_fence = crate::gameplay_transport::item_fence_of(destination)
            .ok_or("returned destination fence")?;
        assert!(
            h.root
                .staff_live_status(&authority, &h.node, destination_fence)
                .await?
                .kicked,
            "immutable scope return inherits the captured disconnect across a crash before final release"
        );
        if lost {
            let row:(i16,Option<String>,String)=sqlx::query_as("SELECT reason,command_id::text,release_loss_epoch::text FROM game_activity_runtime_returns").fetch_one(&h.pool).await?;
            assert_eq!(row, (11, None, "1".into()));
            assert_eq!(
                destination.current_original_grace_deadline(),
                current.current_original_grace_deadline()
            );
            let epoch = destination
                .current_control_loss_epoch()
                .ok_or("returned loss epoch")?;
            assert!(
                h.root
                    .staff_lost_session_kicked(
                        &authority,
                        &h.node,
                        destination_fence,
                        crate::domain::AccountId::from_bytes(id(40)).map_err(debug)?,
                        epoch
                    )
                    .await?
            );
            let store = FreshAdmissionStore::from_root(h.root.clone());
            let outcome = store
                .release_staff_lost_session(
                    returned.destination_game_session_id,
                    &crate::gameplay_transport::canonical_uuid(&id(40)),
                    epoch,
                )
                .await?;
            assert!(matches!(
                outcome,
                crate::durability::fresh_admission::ExpiredLossReleaseV1::Released { .. }
            ));
            assert_eq!(
                store
                    .current_session(returned.destination_game_session_id)
                    .await?
                    .session_state(),
                GameSessionState::Terminal
            );
        } else {
            let row:(i16,Option<String>,Vec<u8>)=sqlx::query_as("SELECT reason,command_id::text,release_transport_ref FROM game_activity_runtime_returns").fetch_one(&h.pool).await?;
            assert_eq!(row, (10, None, transport.to_bytes().to_vec()));
        }
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}
