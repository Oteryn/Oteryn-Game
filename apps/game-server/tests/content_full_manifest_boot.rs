//! Measure the exact operator issuance and node activation paths on committed inputs.
#![cfg(target_os = "linux")]
use oteryn_game_server::content::*;
use oteryn_game_server::foundation::WorldId;
use std::path::PathBuf;
use std::time::{Duration, Instant};

#[test]
fn full_manifest_operator_identity_activates_on_node() -> Result<(), Box<dyn std::error::Error>> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| -> Result<(), String> {
            let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../content/spells.manifest.json");
            let world = WorldId::decode(&[
                1, 147, 79, 16, 124, 2, 112, 1, 128, 91, 59, 17, 34, 51, 68, 1,
            ])
            .map_err(|e| format!("{e:?}"))?;
            let started = Instant::now();
            let room = qualify_native_entry_room_from_gameplay_manifest(world, &manifest)
                .map_err(|e| format!("operator: {e:?}"))?;
            eprintln!(
                "FULL_MANIFEST_BOOT operator_seconds={:.3}",
                started.elapsed().as_secs_f64()
            );
            let issuance = NativeEntryActivationIssuance {
                world_id: world,
                activation_sequence: 1,
                server_artifact_digest: room.compiled().server_digest(),
                client_artifact_digest: room.compiled().client_digest(),
                frame_binding_digest: room.frame_binding().digest(),
            };
            drop(room);
            let started = Instant::now();
            let input = native_gameplay::NativeGameplayInput::from_manifest(&manifest)
                .map_err(|e| format!("manifest: {e:?}"))?;
            let mut controller = ContentActivationController::new();
            let pin = activate_native_entry_room_with_gameplay(
                &mut controller,
                &NodeBootQuiescence::before_channel_runtime(),
                world,
                &issuance,
                &input,
            )
            .map_err(|e| format!("node: {e:?}"))?;
            let elapsed = started.elapsed();
            let helper = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../tools/qualification/node_boot/activation_wait.sh");
            let result = std::process::Command::new("bash")
                .args([
                    "-c", "source \"$1\"; node_activation_wait_seconds 1 0", "activation-wait",
                ])
                .arg(helper)
                .output()
                .map_err(|error| error.to_string())?;
            if !result.status.success() {
                return Err("activation wait profile refused".into());
            }
            let budget = String::from_utf8(result.stdout)
                .map_err(|error| error.to_string())?
                .trim()
                .parse::<u64>()
                .map_err(|error| error.to_string())?;
            assert!(elapsed < Duration::from_secs(budget),
                "node activation completed in {elapsed:?}, beyond the qualification wait of {budget}s");
            assert_eq!(pin.activation_sequence(), 1);
            assert_eq!(
                pin.identity().server_artifact_digest(),
                issuance.server_artifact_digest
            );
            eprintln!(
                "FULL_MANIFEST_BOOT node_activation_seconds={:.3}",
                started.elapsed().as_secs_f64()
            );
            Ok(())
        })?
        .join()
        .map_err(|_| "full manifest boot thread panicked")?
        .map_err(Into::into)
}
