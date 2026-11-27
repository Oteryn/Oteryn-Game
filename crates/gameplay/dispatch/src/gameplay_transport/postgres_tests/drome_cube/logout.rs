//! Logout immediately after a committed Cube reply on the same connection.
use super::*;
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::foundation::{GameSessionState, MessageType, decode_wire_envelope};
use crate::gameplay_transport::COMMAND_HOLDER_WINDOW;
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::composed_admission::{CONDITION_RELEASE_PROBE, HolderCheckoutProbe};
use crate::gameplay_transport::connection::{
    ConnectionEnd, ControllerBinding, IDLE_LIVENESS, LivenessPolicy, serve_admitted,
};
use oteryn_protocol_oteryn::logout::{
    CAPABILITY_LOGOUT_V1, COMMAND_TYPE_LOGOUT_INTENT, LogoutResult, decode_logout_result,
};
use oteryn_protocol_oteryn::world_object::{COMMAND_TYPE_USE_INTENT, decode_use_result};
use std::{cell::Cell, sync::Arc};
use tokio::io::AsyncWriteExt;

pub(super) async fn run(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    harness: &Harness,
    actor: ExactActorRef,
    session: GameSessionId,
    index: &ItemDefinitionIndex,
    case: Case,
) -> TestResult {
    use super::super::autowalk_fixtures::{command_reply, send};

    let store = FreshAdmissionStore::from_root(harness.root.clone());
    let current = store.current_session(session).await?;
    owner
        .runtime
        .lock()
        .await
        .bind_attacker_lease(actor, session, current.current_character_lease())
        .map_err(debug)?;
    let fixture = cases::admitted(owner, actor, session)?;
    let admitted = AdmittedSession {
        continuity: SessionContinuity {
            selected_capabilities: SelectedCapabilities::select(
                owner.offered_capabilities(),
                &[4, 14, 15, CAPABILITY_LOGOUT_V1, 31],
            )
            .ok_or("Cube logout capabilities")?,
            ..fixture.continuity
        },
        controller: Some(ControllerBinding {
            account_id: id(40),
            transport: current.current_transport().ok_or("active transport")?,
        }),
        ..fixture
    };
    let payload = encode_item_use_intent(&cases::intent(index)?).map_err(debug)?;
    assert!(
        admitted
            .continuity
            .selected_capabilities
            .contains(CAPABILITY_LOGOUT_V1)
    );
    let prove_liveness = matches!(case, Case::LogoutLiveness);
    let policy = if prove_liveness {
        LivenessPolicy {
            interval: COMMAND_HOLDER_WINDOW / (IDLE_LIVENESS.missed_limit + 1),
            ..IDLE_LIVENESS
        }
    } else {
        IDLE_LIVENESS
    };
    let probe = Arc::new(HolderCheckoutProbe::default());
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let wire_owner = WireOwner(owner);
    let serve = CONDITION_RELEASE_PROBE.scope(
        Cell::new(Some(Arc::clone(&probe))),
        serve_admitted(&mut server, admitted, &wire_owner, policy),
    );
    let drive = async {
        let draw = admitted.continuity.next_command_id;
        send(&mut client, draw, COMMAND_TYPE_USE_INTENT, &payload).await?;
        let (_, payload) = command_reply(&mut client, draw).await?;
        assert_eq!(
            decode_use_result(&payload).map_err(debug)?,
            UseDisposition::Committed
        );

        let logout = draw + 1;
        send(&mut client, logout, COMMAND_TYPE_LOGOUT_INTENT, &[]).await?;
        // Model a concurrent durability pass that finishes after the ordinary command
        // window but inside its own pass deadline. Advance time only after a real refusal.
        probe
            .contend_with(&harness.root, async {
                tokio::time::pause();
                let result = if prove_liveness {
                    acknowledge_while_held(&mut client, policy).await
                } else {
                    tokio::time::advance(COMMAND_HOLDER_WINDOW).await;
                    tokio::task::yield_now().await;
                    Ok(())
                };
                tokio::time::resume();
                result
            })
            .await??;
        let (_, payload) = command_reply(&mut client, logout).await?;
        client.shutdown().await?;
        assert_eq!(
            decode_logout_result(&payload).map_err(debug)?,
            LogoutResult::Accepted
        );
        Ok::<_, Box<dyn std::error::Error>>(())
    };
    let (end, result) = tokio::time::timeout(Duration::from_secs(30), async {
        tokio::join!(biased; serve, drive)
    })
    .await?;
    result?;
    assert!(
        matches!(end, ConnectionEnd::AdmittedThenLoggedOut(_)),
        "{end:?}"
    );
    assert_eq!(
        store.current_session(session).await?.session_state(),
        GameSessionState::Terminal
    );
    Ok(())
}

async fn acknowledge_while_held(
    client: &mut tokio::io::DuplexStream,
    policy: LivenessPolicy,
) -> TestResult {
    use super::super::autowalk_fixtures::{ack_probe, read};

    // Each ACK is sent before the next cadence and before we return the holder.
    for _ in 0..=policy.missed_limit {
        tokio::time::advance(policy.interval).await;
        loop {
            let frame = read(client).await?;
            match decode_wire_envelope(&frame)?.message_type() {
                MessageType::LivenessProbe => {
                    ack_probe(client, &frame).await?;
                    break;
                }
                MessageType::CommandResult => {
                    return Err("logout replied while the pass was held".into());
                }
                _ => {}
            }
        }
    }
    Ok(())
}
