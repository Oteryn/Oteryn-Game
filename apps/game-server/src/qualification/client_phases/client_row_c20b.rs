//! C20b client replay capture.
//!
//! The real C20b journey runs immediately before this packet and buys/equips/consumes an Amulet
//! of Loss before losing the replacement backpack without it. A fresh admission after those
//! server-observed
//! actions supplies the exact inventory snapshot bytes consumed by
//! `crates/session/src/snapshot_state.rs::SnapshotState::install_domains`; subsequent inventory
//! deltas are consumed by `crates/session/src/domain_dispatch.rs::Session::apply_state_delta`,
//! and `crates/session/src/session_access.rs::Session::inventory` exposes the resulting state.

use super::*;

pub(super) fn capture_inventory(client: &mut oteryn_dev_client::DevClientSession) -> TestResult {
        let snapshot = client
            .session_mut()
            .gated_snapshot(oteryn_protocol_oteryn::item_view::STATE_DOMAIN_CHARACTER_INVENTORY)
            .ok_or("C20b fresh admission omitted the inventory snapshot")?
            .clone();
        let inventory = client
            .inventory()
            .ok_or("C20b fresh admission did not install inventory")?;
        if inventory.main_backpack.is_some() || !inventory.entries.is_empty() {
            return Err("C20b final server-observed inventory retained backpack custody".into());
        }
        capture(snapshot.revision, &snapshot.payload, inventory)
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

pub(super) async fn run_packet(prerequisite: impl Future<Output = TestResult>) {
    let started = std::time::Instant::now();
    let result = verdict::indexed_timeout(
        Duration::from_secs(1050),
        verdict::observe(prerequisite),
    )
    .await
    .map_err(|_| "C20b client capture timed out".into())
    .and_then(|result| result);
    let (status, detail) = match result {
        Ok(()) => ("PASS", "current-run-C20b-actions-and-fresh-inventory-snapshot".to_owned()),
        Err(error) => ("FAIL", error.to_string()),
    };
    evidence(&format!(
        "e2e_all scenario=C20b.client status={status} seconds={:.3} evidence={detail}",
        started.elapsed().as_secs_f64()
    ));
}
