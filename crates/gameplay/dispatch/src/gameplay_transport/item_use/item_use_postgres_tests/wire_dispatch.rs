//! The existing BAGS-USE wire exchange, shared by its PostgreSQL regression cases.

use super::*;
use oteryn_protocol_oteryn::world_object::COMMAND_TYPE_USE_INTENT;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub(super) async fn exchange(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    payloads: &[Vec<u8>],
) -> TestResult<Vec<u8>> {
    exchange_with_probe(owner, admitted, payloads, None).await
}

pub(super) struct PotionBagReadProbe {
    pub(super) partly_spent: bool,
    pub(super) observed: std::cell::Cell<bool>,
}

impl PotionBagReadProbe {
    pub(super) async fn observe(
        &self,
        owner: &ComposedFreshAdmission<'_, '_, '_>,
        actor: ExactActorRef,
        fence: Option<crate::durability::item_transfer::CurrentCharacterItemFence>,
        target: ItemKey,
    ) -> Option<super::super::super::container_view::ContainerObservation> {
        use crate::durability::DurabilityError;
        use crate::gameplay_transport::{
            AdmissionRetryBudget, COMMAND_HOLDER_WINDOW, RECONCILE_BACKOFF,
        };

        self.observed.set(true);
        let mut budget = AdmissionRetryBudget::for_command(owner.root);
        let _holder = if self.partly_spent {
            budget.deadline -= COMMAND_HOLDER_WINDOW - RECONCILE_BACKOFF;
            owner.root.await_ready().await.expect("ready root");
            Some(owner.root.try_issue_semantic_pass().expect("held root"))
        } else {
            None
        };
        budget
            .retry_command_guard(async || {
                owner
                    .observe_container(actor, fence, target)
                    .await
                    .ok_or(DurabilityError::RootUnavailable)
            })
            .await
            .ok()
    }
}

pub(super) async fn exchange_with_probe(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    payloads: &[Vec<u8>],
    potion_bag_read_probe: Option<&PotionBagReadProbe>,
) -> TestResult<Vec<u8>> {
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    for (command_id, payload) in (1..).zip(payloads) {
        let frame = oteryn_protocol_oteryn::encode_client_command(
            1,
            &oteryn_protocol_oteryn::ClientCommandValue {
                command_id,
                command_type: COMMAND_TYPE_USE_INTENT,
                payload,
            },
        )?;
        client
            .write_all(&u32::try_from(frame.len())?.to_be_bytes())
            .await?;
        client.write_all(&frame).await?;
    }
    client.shutdown().await?;
    let end = crate::gameplay_transport::connection::serve_admitted(
        &mut server,
        admitted,
        &WireOwner(owner, potion_bag_read_probe),
        crate::gameplay_transport::connection::IDLE_LIVENESS,
    )
    .await;
    assert!(
        matches!(
            end,
            crate::gameplay_transport::connection::ConnectionEnd::AdmittedThenDisconnected(_)
        ),
        "{end:?}"
    );
    drop(server);
    let mut output = Vec::new();
    client.read_to_end(&mut output).await?;
    Ok(output)
}
