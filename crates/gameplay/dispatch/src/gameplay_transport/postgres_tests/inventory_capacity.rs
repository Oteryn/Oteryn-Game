//! INVCAP-1: the real connection sends the own character's capacity, read through the
//! session's item fence with the move capacity rules, and the client panel shows Tibia's Cap.
use super::fixture::{Harness, debug};
use super::kill_reward_e2e::scenario_thread::run_async;
use super::{Dependencies, TestResult, id, prepare_current_session};
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use crate::gameplay_transport::{
    capabilities::{OfferedCapability, SelectedCapabilities},
    connection::{ConnectionEnd, FirstEntryOutcome, IDLE_LIVENESS, serve_admitted},
    tcp_tls::{read_frame, write_frame},
};
use oteryn_dev_client::native_session as client;
use oteryn_protocol_oteryn::inventory_capacity::InventoryCapacity;
use oteryn_protocol_oteryn::{
    MessageType, ServerAcceptedValue, decode_wire_envelope, encode_server_accepted,
};
use std::time::Duration;

fn select(capabilities: &[u32]) -> TestResult<SelectedCapabilities> {
    Ok(SelectedCapabilities::select(
        &[
            OfferedCapability {
                id: 6,
                requires: &[],
            },
            OfferedCapability {
                id: 508,
                requires: &[6],
            },
        ],
        capabilities,
    )
    .ok_or("capacity capability selection")?)
}

#[test]
fn inventory_capacity_real_connection_postgres() -> TestResult {
    run_async(async |admin| {
        let h = Harness::create_with_admission_seed(admin, "inventory_capacity", true, false).await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let deps = Dependencies::new()?;
        deps.runtime
            .lock()
            .await
            .initialize_first_entry_position(deps.actor)
            .map_err(debug)?;
        let (controller, _) = super::death_ml_loss_composed::production_generation_with_pin(
            deps.admitted.world_id,
            "content-1",
        )?;
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = controller.active();
        deps.initialize(&owner).await;
        let current =
            crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                .current_session_at(deps.admitted.game_session_id)
                .await?
                .0;
        deps.runtime
            .lock()
            .await
            .bind_attacker_lease(
                deps.actor,
                deps.admitted.game_session_id,
                current.current_character_lease(),
            )
            .map_err(debug)?;
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.item_fence = crate::gameplay_transport::item_fence_of(current);
        admitted.controller.as_mut().ok_or("controller")?.transport =
            current.current_transport().ok_or("transport")?;
        // Without capability 508 the owner reads nothing.
        admitted.continuity.selected_capabilities = select(&[6])?;
        assert_eq!(owner.observe_inventory_capacity(admitted).await, None);
        admitted.continuity.selected_capabilities = select(&[6, 508])?;
        let expected = owner
            .observe_inventory_capacity(admitted)
            .await
            .ok_or("fenced capacity read")?;
        // The same derivation the item-move owner checks: capacity and the carried tree.
        let fence = admitted.item_fence.ok_or("item fence")?;
        let native = owner
            .active_generation
            .and_then(|generation| generation.native_gameplay())
            .ok_or("native gameplay")?;
        let (capacity, carried) = h
            .root
            .read_inventory_capacity(
                &authority,
                &h.node,
                fence,
                &crate::gameplay_transport::item_ref_admission::NativeItemWeights(native)
                    .with_wheel(owner.premium_coordinator),
            )
            .await
            .map_err(debug)?
            .ok_or("durable capacity")?;
        assert!(capacity > 0);
        assert_eq!(
            Some(expected),
            InventoryCapacity::from_carried(capacity, carried)
        );
        // A stale item fence (another connection generation) reads nothing.
        let mut stale = admitted;
        stale.continuity.connection_generation += 1;
        assert_eq!(owner.observe_inventory_capacity(stale).await, None);
        real_connection(&owner, admitted, expected).await?;
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

// The admission and lease are the fixture's committed facts; the serve loop, the sidecar
// delta, the client session decoder and the client panel reducer are production code.
async fn real_connection(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: crate::gameplay_transport::connection::AdmittedSession,
    expected: InventoryCapacity,
) -> TestResult {
    let (mut server, stream) = tokio::io::duplex(256 * 1024);
    let serving = async {
        let bytes = read_frame(&mut server).await?;
        assert_eq!(
            decode_wire_envelope(&bytes).map_err(debug)?.message_type(),
            MessageType::ClientBootstrap
        );
        let accepted = encode_server_accepted(&ServerAcceptedValue {
            game_session_id: admitted.game_session_id,
            world_id: admitted.world_id,
            channel_id: admitted.channel_id,
            connection_generation: admitted.continuity.connection_generation,
            current_server_sequence: admitted.continuity.server_sequence,
            next_command_id: admitted.continuity.next_command_id,
            schema_revision: 1,
            selected_capabilities: admitted.continuity.selected_capabilities.as_slice(),
        })
        .map_err(debug)?;
        write_frame(&mut server, &accepted).await?;
        Ok::<_, Box<dyn std::error::Error>>(
            serve_admitted(&mut server, admitted, owner, IDLE_LIVENESS).await,
        )
    };
    let driving = async {
        let mut session = client::Session::admit(
            stream,
            client::Admission {
                appearance_items: None,
                schema_revision: 1,
                character_id: oteryn_protocol_oteryn::CharacterId::decode(&id(41))
                    .map_err(debug)?,
                admission_material: b"committed fixture admission",
                client_build_id: "inventory-capacity-e2e",
                supported_capabilities: client::CLIENT_SUPPORTED_CAPABILITIES,
                deadline: Duration::from_secs(10),
            },
        )
        .await?;
        let received = loop {
            let pushed = session.take_events().into_iter().find_map(|event| match event {
                client::SessionEvent::InventoryCapacity(delta) => Some(delta.value),
                _ => None,
            });
            if let Some(capacity) = pushed {
                break capacity;
            }
            session.service_liveness(Duration::from_millis(20)).await?;
        };
        assert_eq!(received, expected);
        let mut panel = oteryn_client_ui::inventory::InventoryPanel::default();
        oteryn_client_ui::inventory_capacity::apply(&mut panel, received);
        assert_eq!(
            oteryn_client_ui::inventory_capacity::cap_text(&panel),
            Some(format!("Cap: {}", expected.free / 100))
        );
        // An unchanged capacity is not sent again.
        session.service_liveness(Duration::from_millis(500)).await?;
        assert!(!session.take_events().iter().any(|event| matches!(
            event,
            client::SessionEvent::InventoryCapacity(_)
        )));
        drop(session);
        Ok::<_, Box<dyn std::error::Error>>(())
    };
    let (end, ()) = tokio::time::timeout(Duration::from_secs(30), async {
        tokio::try_join!(serving, driving)
    })
    .await
    .map_err(|_| "inventory capacity real connection timed out")??;
    assert!(
        matches!(
            end,
            ConnectionEnd::AdmittedThenDisconnected(_) | ConnectionEnd::AdmittedThenControlLost(_)
        ),
        "{end:?}"
    );
    Ok(())
}
