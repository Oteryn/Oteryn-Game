//! Native catalogue and confirmed purchase presentation.
use oteryn_client_runtime::server_mechanic::ServerMechanicCommand;
use oteryn_client_ui::store::{self, StorePanel};

pub(in crate::play) fn purchase(
    panel: &mut StorePanel,
    selection: usize,
    confirmed: bool,
) -> Option<ServerMechanicCommand> {
    if panel.pending || !panel.selected {
        return None;
    }
    panel.selected_offer = selection;
    panel.purchase_confirmed = confirmed;
    if confirmed && panel.confirmed_purchase.is_none() {
        // CLIGAP-W09 accepted transport difference: retries reuse the confirmed
        // Game transaction and quote until a decoded terminal result.
        use sha2::{Digest, Sha256};
        use std::sync::atomic::{AtomicU64, Ordering};
        static SEQUENCE: AtomicU64 = AtomicU64::new(0);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()?;
        let mut hash = Sha256::new();
        hash.update(now.as_nanos().to_be_bytes());
        hash.update(std::process::id().to_be_bytes());
        hash.update(SEQUENCE.fetch_add(1, Ordering::Relaxed).to_be_bytes());
        panel
            .purchase_request_id
            .copy_from_slice(&hash.finalize()[..16]);
        panel.purchase_request_id[..6].copy_from_slice(&now.as_millis().to_be_bytes()[10..]);
        panel.purchase_request_id[6] = (panel.purchase_request_id[6] & 15) | 0x70;
        panel.purchase_request_id[8] = (panel.purchase_request_id[8] & 63) | 0x80;
    }
    store::purchase(panel)
}

// An optional Game identity bypasses the server's owned-Hireling selection modal.
pub(in crate::play) fn customization(
    panel: &mut StorePanel,
    name: &str,
    sex: &str,
    identity: &str,
) {
    panel.hireling_name = name.into();
    panel.hireling_sex = sex.parse().unwrap_or(0);
    let hex: String = identity.chars().filter(|c| *c != '-').collect();
    panel.hireling_id = (hex.len() == 32 && hex.is_ascii())
        .then(|| {
            let mut id = [0; 16];
            for (i, byte) in id.iter_mut().enumerate() {
                *byte = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).ok()?;
            }
            Some(id)
        })
        .flatten();
}

pub(in crate::play) fn panel_text(panel: &StorePanel) -> String {
    if !panel.selected {
        return tr!("Store unavailable in this session").into();
    }
    if panel.pending {
        return tr!("Pending store action…").into();
    }
    let mut status = if let Some(feedback) = &panel.feedback {
        feedback.clone()
    } else if let Some(result) = &panel.purchase {
        lfmt!("Purchase: {:?}", result.code)
    } else {
        panel.catalogue.as_ref().map_or_else(
            || tr!("Load the server catalogue").into(),
            |page| lfmt!("Balance: {} coins", page.total_coins),
        )
    };
    if let Some(preview) = store::offer_preview(panel) {
        status.push_str("\r\n");
        status.push_str(&preview);
    }
    status
}

#[cfg(windows)]
pub(in crate::play) use windows::open_window;

#[cfg(any(windows, test))]
#[path = "native/assets.rs"]
mod assets;
#[cfg(windows)]
#[path = "native/browser.rs"]
mod windows;

#[cfg(windows)]
#[path = "native/classic.rs"]
mod classic;
#[cfg(windows)]
pub(in crate::play) use classic::open_window as open_classic_window;
