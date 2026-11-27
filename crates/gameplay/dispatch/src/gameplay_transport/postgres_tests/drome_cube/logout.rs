//! Logout immediately after a committed Cube reply on the same connection.
use super::*;
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::foundation::{CommandStatus, GameSessionState};
use crate::gameplay_transport::COMMAND_HOLDER_WINDOW;
use crate::gameplay_transport::composed_admission::{CONDITION_RELEASE_PROBE, HolderCheckoutProbe};
use crate::gameplay_transport::connection::{
    ConnectionEnd, ControllerBinding, IDLE_LIVENESS, serve_admitted,
};
use oteryn_protocol_oteryn::logout::{COMMAND_TYPE_LOGOUT_INTENT, LogoutResult, decode_logout_result};
use oteryn_protocol_oteryn::world_object::{COMMAND_TYPE_USE_INTENT, decode_use_result};
use std::{cell::Cell, sync::Arc};
use tokio::io::AsyncWriteExt;

pub(super) async fn run(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    harness: &Harness,
    actor: ExactActorRef,
    session: GameSessionId,
    index: &ItemDefinitionIndex,
) -> TestResult {
    use super::super::autowalk_fixtures::send;
    use super::super::quick_loot::command_reply;

    let store = FreshAdmissionStore::from_root(harness.root.clone());
    let current = store.current_session(session).await?;
    owner.runtime.lock().await.bind_attacker_lease(
        actor,
        session,
        current.current_character_lease(),
    ).map_err(debug)?;
    let admitted = AdmittedSession {
        controller: Some(ControllerBinding {
            account_id: id(40),
            transport: current.current_transport().ok_or("active transport")?,
        }),
        ..cases::admitted(owner, actor, session)?
    };
    let payload = encode_item_use_intent(&cases::intent(index)?).map_err(debug)?;
    let probe = Arc::new(HolderCheckoutProbe::default());
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let wire_owner = WireOwner(owner);
    let serve = CONDITION_RELEASE_PROBE.scope(
        Cell::new(Some(Arc::clone(&probe))),
        serve_admitted(&mut server, admitted, &wire_owner, IDLE_LIVENESS),
    );
    let drive = async {
        let draw = admitted.continuity.next_command_id;
        send(&mut client, draw, COMMAND_TYPE_USE_INTENT, &payload).await?;
        let (status, result) = command_reply(&mut client, draw).await?;
        assert_eq!(status, CommandStatus::Accepted);
        assert_eq!(decode_use_result(&result).map_err(debug)?, UseDisposition::Committed);

        let logout = draw + 1;
        send(&mut client, logout, COMMAND_TYPE_LOGOUT_INTENT, &[]).await?;
        // Model a concurrent durability pass that finishes after the ordinary command
        // window but inside its own pass deadline. Advance time only after a real refusal.
        probe.contend_with(&harness.root, async {
            tokio::time::pause();
            tokio::time::advance(COMMAND_HOLDER_WINDOW).await;
            tokio::task::yield_now().await;
            tokio::time::resume();
        }).await?;
        let (status, result) = command_reply(&mut client, logout).await?;
        client.shutdown().await?;
        assert_eq!(decode_logout_result(&result).map_err(debug)?, LogoutResult::Accepted);
        assert_eq!(status, CommandStatus::Accepted);
        Ok::<_, Box<dyn std::error::Error>>(())
    };
    let (end, result) = tokio::time::timeout(Duration::from_secs(30), async {
        tokio::join!(biased; serve, drive)
    }).await?;
    result?;
    assert!(matches!(end, ConnectionEnd::AdmittedThenLoggedOut(_)), "{end:?}");
    assert_eq!(store.current_session(session).await?.session_state(), GameSessionState::Terminal);
    Ok(())
}
