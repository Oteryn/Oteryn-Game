//! The real login and modal router supply the configured notice and keyboard descriptor.
use super::fixture::{Harness, debug};
use super::kill_reward_e2e_fixtures::scenario_thread::run_async;
use super::{Dependencies, TestResult, prepare_current_session};
use crate::gameplay_transport::{
    capabilities::{OfferedCapability, SelectedCapabilities},
    connection::FirstEntryOutcome,
};
use oteryn_client_ui::server_modal::{self as ui, Panel};
use oteryn_protocol_oteryn::server_modal::{self as wire, input::decode_reply_with_input};
use oteryn_session::{CommandStatus, GatedCommandResult};
#[test]
fn modal_defaults_real_dispatch_to_client_keyboard_model() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else { return Ok(()) };
    run_async(async move |_| {
        let h = Harness::create_with_admission_seed(admin, "modal_defaults", true, false).await?;
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
        let configured_motd = "The server owns this notice and its confirmation button.";
        owner.server_motd = configured_motd;
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
        let offers = [
            OfferedCapability {
                id: 92,
                requires: &[],
            },
            OfferedCapability {
                id: 450,
                requires: &[92],
            },
            OfferedCapability {
                id: 490,
                requires: &[450],
            },
            OfferedCapability {
                id: 505,
                requires: &[450],
            },
        ];
        admitted.continuity.selected_capabilities =
            SelectedCapabilities::select(&offers, &[92, 450, 490, 505]).ok_or("select defaults")?;
        let mut panel = Panel {
            selected: true,
            defaults: true,
            input: true,
            ..Default::default()
        };
        let query = ui::query(&mut panel).ok_or("modal query")?;
        let (status, payload) = super::cyclopedia_fixtures::support::exchange(
            &owner,
            &mut admitted,
            1,
            query.kind,
            &query.payload,
        )
        .await?;
        assert_eq!(status, CommandStatus::Accepted);
        assert!(ui::apply_result(
            &mut panel,
            &GatedCommandResult {
                command_id: 1,
                status,
                server_sequence: 1,
                payload
            }
        ));
        assert_eq!(ui::keyboard_button(&panel, false), Some(1));
        assert_eq!(ui::keyboard_button(&panel, true), Some(2));
        assert_eq!(
            panel.active.as_ref().ok_or("notice")?.title,
            "Server notice"
        );
        assert_eq!(
            panel.active.as_ref().ok_or("notice")?.message,
            configured_motd
        );
        assert!(admitted.continuity.server_motd_sent);

        // The exchange closes the physical connection, then establishes another one
        // through the snapshot path with this same GameSession's delivery receipt.
        let next_id = admitted.continuity.next_command_id;
        let (status, payload) = super::cyclopedia_fixtures::support::exchange(
            &owner,
            &mut admitted,
            next_id,
            wire::COMMAND_TYPE_SERVER_MODAL_QUERY,
            &[],
        )
        .await?;
        assert_eq!(status, CommandStatus::Accepted);
        assert_eq!(
            decode_reply_with_input(&payload).map_err(debug)?,
            (wire::Reply::None, None)
        );
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}
