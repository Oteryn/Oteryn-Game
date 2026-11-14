//! Consent to replay the existing owner's frozen receipt, never a new purchase.
use super::*;
use std::sync::atomic::{AtomicBool, Ordering};
#[derive(Clone, Debug, PartialEq, Eq)]
struct Receipt {
    payload: Vec<u8>,
    command_type: u32,
    text: String,
}
#[derive(Clone, Debug)]
pub(crate) struct HirelingRetry {
    receipt: Receipt,
    submitted: Arc<AtomicBool>,
}
impl PartialEq for HirelingRetry {
    fn eq(&self, other: &Self) -> bool {
        self.receipt == other.receipt && Arc::ptr_eq(&self.submitted, &other.submitted)
    }
}
impl Eq for HirelingRetry {}
impl HirelingRetry {
    pub(crate) fn text(&self) -> &str {
        &self.receipt.text
    }
}
fn receipt(m: &Mailbox) -> Option<Receipt> {
    if !m.hireling_management.selected
        || !m.store.selected
        || m.hireling_management.pending.is_some()
        || m.store.purchase.is_some()
        || !matches!(m.store.pending_action, 185 | 6188)
    {
        return None;
    }
    let payload = m.store.confirmed_purchase.as_ref()?;
    let (quote, customization) =
        oteryn_client_runtime::cui3::store::decode_purchase_request_with_hireling(payload).ok()?;
    if !matches!(
        quote.offer_key.as_str(),
        "store:hireling:25438"
            | "store:hireling:25437"
            | "store:hireling:1001"
            | "store:hireling:1002"
            | "store:hireling:1003"
            | "store:hireling:1004"
    ) || quote.purchase_request_id != m.store.purchase_request_id
    {
        return None;
    }
    let entry = m
        .hireling_management
        .entries
        .iter()
        .find(|e| Some(e.hireling) == m.hireling_management.identity)?;
    if m.store.hireling_id != Some(entry.hireling)
        || customization
            .as_ref()
            .and_then(|c| c.hireling_id)
            .is_some_and(|id| id != entry.hireling)
    {
        return None;
    }
    let mut text = format!(
        "Retry original configuration for {}: {} ({} {})?\nThe original request and receipt are reused; its outcome is still unresolved.",
        entry.name, quote.offer_key, quote.price, quote.currency
    );
    if let Some(name) = customization.and_then(|c| c.name) {
        text.push_str(&format!("\nSubmitted name: {name}"));
    }
    Some(Receipt {
        payload: payload.clone(),
        command_type: m.store.pending_action,
        text,
    })
}
impl PlayLink {
    pub(crate) fn hireling_management_retry_review_bound(
        &self,
        expected: &HirelingSnapshot,
    ) -> Result<HirelingRetry, &'static str> {
        let m = lock(&self.mailbox);
        if !m.hireling_management.open || !self.hireling_management_matches(&m, expected) {
            return Err("Hireling configuration unavailable in this session");
        }
        let receipt = receipt(&m).ok_or("No unresolved hireling configuration for this target")?;
        Ok(HirelingRetry {
            receipt,
            submitted: Arc::new(AtomicBool::new(false)),
        })
    }
}
pub(super) fn submit(
    m: &mut Mailbox,
    commands: &tokio::sync::mpsc::UnboundedSender<PlayCommand>,
    review: &HirelingRetry,
) -> bool {
    if receipt(m).as_ref() != Some(&review.receipt) {
        return false;
    }
    let Some(command) = oteryn_client_ui::store::command(
        review.receipt.command_type,
        review.receipt.payload.clone(),
    ) else {
        return false;
    };
    // The shared consent token is consumed under the owner lock. A lost reply leaves
    // the panel pending, so snapshot equality alone cannot reject a duplicate click.
    if review
        .submitted
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return false;
    }
    if commands.send(PlayCommand::ServerMechanic(command)).is_err() {
        review.submitted.store(false, Ordering::Release);
        return false;
    }
    m.store.pending = true;
    m.store.feedback = None;
    true
}
