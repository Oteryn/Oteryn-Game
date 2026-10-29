use crate::content::native_gameplay::NativeGameplayInput;
use crate::content::{
    ContentActivationController, NativeEntryActivationIssuance, NodeBootQuiescence,
    activate_native_entry_room_with_gameplay, qualify_selected_native_gameplay_room,
};
use crate::gameplay_transport::world_required_capabilities::{
    RequiredCapabilities, offered_for_scope,
};

#[test]
fn shipped_world_offers_field_runes_and_coin_conversion() -> Result<(), Box<dyn std::error::Error>> {
    // Native content compilation uses the same large stack as node/dispatch scenarios.
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            let topology: toml::Value = toml::from_str(include_str!(
                "../../../../../deploy/synology-game/preprod-topology.toml"
            ))?;
            assert_eq!(topology["world_bundle"]["enabled"].as_bool(), Some(true));
            let input = NativeGameplayInput::from_manifest(
                &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("../../content/spells.manifest.json"),
            )?;
            let world = crate::content::native_gameplay_test_world_id()?;
            let room = qualify_selected_native_gameplay_room(world, &input)?;
            // Test issuance for these exact bytes; production reads its independent issuance
            // from durability before calling this same activation entry point.
            let issuance = NativeEntryActivationIssuance {
                world_id: world,
                activation_sequence: 1,
                server_artifact_digest: room.compiled().server_digest(),
                client_artifact_digest: room.compiled().client_digest(),
                frame_binding_digest: room.frame_binding().digest(),
            };
            let mut controller = ContentActivationController::new();
            activate_native_entry_room_with_gameplay(
                &mut controller,
                &NodeBootQuiescence::before_channel_runtime(),
                world,
                &issuance,
                &input,
            )?;
            let active = controller.active().ok_or("no active generation")?;
            let index = active
                .native_gameplay()
                .and_then(|native| native.item_index())
                .ok_or("shipped native content has no Item index")?;
            assert!(!index.is_empty());
            // This is the scope offer used by prepare_channel before readiness and by fresh
            // admission. Selecting the full offer also proves dependency closure.
            let offered = offered_for_scope(
                topology["world_bundle"]["enabled"]
                    .as_bool()
                    .ok_or("missing shipped World bundle switch")?,
                Some(active),
            );
            let supported: Vec<_> = offered.iter().map(|capability| capability.id).collect();
            let required = RequiredCapabilities::for_boot(
                &[91, 171],
                &supported,
                offered,
                &[4, 11, 15, 22, 30],
            )
            .map_err(|error| format!("shipped boot closure: {error:?}"))?;
            let selected = required
                .select_for_session(&supported, &[])
                .map_err(|error| format!("shipped session selection: {error:?}"))?;
            for id in [4, 11, 15, 22, 30, 91, 171] {
                assert!(offered.iter().any(|capability| capability.id == id), "{id}");
                assert!(selected.contains(id), "{id} lost through dependency closure");
            }
            Ok(())
        })?
        .join()
        .map_err(|_| "shipped capability test thread panicked")?
        .map_err(|error| error.to_string())?;
    Ok(())
}
