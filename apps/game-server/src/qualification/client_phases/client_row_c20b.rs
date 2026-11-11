//! C20b client replay capture.
//!
//! The real C20b journey runs immediately before this packet and buys/equips/consumes an Amulet
//! of Loss before losing the replacement backpack without it. A fresh admission after those
//! server-observed
//! actions supplies installed inventory state, re-encoded with the production codec for replay.
//! `crates/session/src/snapshot_state.rs::SnapshotState::install_domains` consumes it; inventory
//! deltas are consumed by `crates/session/src/domain_dispatch.rs::Session::apply_state_delta`,
//! and `crates/session/src/session_access.rs::Session::inventory` exposes the resulting state.

use super::*;

mod journey {
    use super::*;

    pub(super) async fn run_and_capture(clients: &SeamClients<'_>) -> TestResult {
        let mut client = super::super::row_connect_character(clients, 0xcb, 0).await?;
        let row_result: TestResult<_> = async {
            // Re-encode the installed core store. Revision 1 below is the replay peer's
            // local baseline, not a revision invented for the live server.
            let snapshot =
                oteryn_protocol_oteryn::item_view::encode_character_inventory_with_equip_drop(
                    client.inventory().ok_or("inventory state absent")?,
                    true,
                )
                .map_err(|error| format!("inventory snapshot encode: {error:?}"))?;
            let inventory = client
                .inventory()
                .ok_or("C20b fresh admission did not install inventory")?;
            if inventory.main_backpack.is_some() || !inventory.entries.is_empty() {
                return Err(
                    "C20b final server-observed inventory retained backpack custody".into(),
                );
            }
            capture(1, &snapshot, inventory)?;
            Ok(())
        }
        .await;
        let logout_result = row_finish(&mut client).await;
        logout_result?;
        row_result
    }

    fn capture(
        revision: u64,
        payload: &[u8],
        inventory: &oteryn_dev_client::CharacterInventory,
    ) -> TestResult {
        let Some(directory) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
            return Ok(());
        };
        let directory = std::path::Path::new(&directory);
        let expected = format!(
            "revision={revision}\nbackpack={}\nentries={}\nequipment={}\n",
            usize::from(inventory.main_backpack.is_some()),
            inventory.entries.len(),
            inventory.equipment.len()
        );
        for (name, bytes) in [
            ("c20b-inventory.bin", payload),
            ("c20b-expected.txt", expected.as_bytes()),
        ] {
            std::fs::write(directory.join(name), bytes)?;
        }

        Ok(())
    }
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    if std::env::var_os("E2E_ALL_CAPTURE_DIR").is_none() {
        return;
    }
    let started = std::time::Instant::now();
    let result = row_complete_with_timeout(Duration::from_secs(480), journey::run_and_capture(clients))
        .await
        .and_then(|result| result);
    evidence(&format!(
        "SEAM_EVIDENCE e2e_all scenario=C20b.client status={} seconds={:.3} evidence=real-C20b-actions-and-fresh-inventory-snapshot",
        if result.is_ok() { "PASS" } else { "FAIL" },
        started.elapsed().as_secs_f64()
    ));
    if let Err(error) = result {
        evidence(&format!("C20b client capture error={error}"));
    }
}
