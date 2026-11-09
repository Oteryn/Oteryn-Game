//! One reviewed resource intent and decoded server result; no paid request is retried here.
use super::{ItemForgePanel, reply};
use oteryn_client_runtime::{
    cui3::item_forge::{Operation, Request},
    forge_preview as wire,
};
use std::time::{Duration, Instant};
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Content {
    Effects(wire::Effects),
    Exaltation {
        receipt: reply::Receipt,
        target: Option<wire::Effects>,
        donor: Option<wire::Effects>,
    },
    Confirm {
        operation: Operation,
        quote: Option<wire::ResourceQuote>,
    },
    Result(reply::Reply),
    Message(String),
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ResourceDialog {
    pub serial: u64,
    pub content: Option<Content>,
    pub native_open: bool,
    pub expires: Option<Instant>,
}
impl ResourceDialog {
    pub fn confirming(&self) -> bool {
        matches!(self.content, Some(Content::Confirm { .. }))
    }
    pub fn reset(&mut self) {
        self.serial = self.serial.saturating_add(1);
        self.content = None;
        self.native_open = false;
        self.expires = None;
    }
    pub fn offer(&mut self, operation: Operation, quote: Option<wire::ResourceQuote>) {
        self.reset();
        self.expires = Some(Instant::now() + Duration::from_secs(15));
        self.content = Some(Content::Confirm { operation, quote });
    }
    pub fn result(&mut self, reply: reply::Reply) {
        self.reset();
        self.content = Some(Content::Result(reply));
    }
    pub fn message(&mut self, message: &str) {
        self.reset();
        self.content = Some(Content::Message(message.into()));
    }
    pub fn dismiss(&mut self, serial: u64) {
        if self.serial == serial {
            self.reset();
        }
    }
}
pub fn operation_name(operation: Operation) -> &'static str {
    match operation {
        Operation::DustToSlivers => "Convert dust to slivers",
        Operation::SliversToCores => "Convert slivers to a core",
        Operation::DustLimit => "Raise the dust limit",
        Operation::QueryHistory => "Forge history",
        Operation::Fusion => "Fuse item pair",
        Operation::ConvergenceFusion => "Convergence fusion",
        Operation::Transfer => "Transfer item tier",
        Operation::ConvergenceTransfer => "Convergence transfer",
    }
}
pub fn confirmation_text(operation: Operation, quote: Option<wire::ResourceQuote>) -> String {
    let Some(q) = quote else {
        return format!(
            "{}?\nThis server does not provide a cost preview. Confirm to request this operation with its current rules.",
            operation_name(operation)
        );
    };
    let b = q.before;
    let mut text = format!(
        "{}?\n\nDust: {} → {}\nDust limit: {} → {}",
        operation_name(operation),
        b.dust,
        q.after_dust,
        b.dust_limit,
        q.after_limit
    );
    if q.full_resources {
        text.push_str(&format!(
            "\nSlivers: {} → {}\nCores: {} → {}",
            b.slivers, q.after_slivers, b.cores, q.after_cores
        ));
    }
    text.push_str("\n\nThe server checks your current resources when applying the operation.");
    text
}
fn receipt_text(receipt: &reply::Receipt) -> String {
    if !wire::resource_operation(receipt.operation) {
        return reply::render(receipt);
    }
    let mut text = format!(
        "{}: {}",
        operation_name(receipt.operation),
        if receipt.applied && receipt.succeeded {
            "Completed"
        } else if receipt.applied {
            "Attempt applied"
        } else {
            "Not applied"
        }
    );
    if let Some(reason) = &receipt.reason {
        text.push_str(&format!(
            "\n{}",
            match reason.as_str() {
                "INSUFFICIENT_DUST" => "Not enough dust",
                "INSUFFICIENT_SLIVERS" => "Not enough slivers",
                "MAXIMUM_LIMIT" => "The dust limit is already at its maximum",
                "STAGE_CHANGED" => "The dust limit changed; refresh before another operation",
                _ => reason,
            }
        ));
    }
    if let Some(r) = &receipt.resources {
        text.push_str(&format!(
            "\nDust after operation: {} / {}",
            r.dust, r.dust_limit
        ));
        if r.slivers_created > 0 {
            text.push_str(&format!("\nCreated: {} slivers", r.slivers_created));
        }
        if r.slivers_spent > 0 {
            text.push_str(&format!("\nSpent: {} slivers", r.slivers_spent));
        }
        if r.cores_created > 0 {
            text.push_str(&format!("\nCreated: {} cores", r.cores_created));
        }
        if r.gold_spent > 0 {
            text.push_str(&format!("\nSpent: {} gold", r.gold_spent));
        }
    }
    text
}
pub fn dialog_text(content: &Content, row: usize) -> String {
    match content {
        Content::Effects(e) => super::exaltation::effects_text(e),
        Content::Exaltation {
            receipt,
            target,
            donor,
        } => super::exaltation::result_text(receipt, target.as_ref(), donor.as_ref()),
        Content::Confirm { operation, quote } => confirmation_text(*operation, *quote),
        Content::Message(message) => message.clone(),
        Content::Result(reply::Reply::CurrentDust { dust, limit }) => {
            format!("Current dust: {dust} / {limit}")
        }
        Content::Result(reply::Reply::Result(receipt)) => receipt_text(receipt),
        Content::Result(reply::Reply::History(rows)) => rows.get(row).map_or_else(
            || "No Forge operations recorded".into(),
            |(ordinal, receipt)| format!("History #{ordinal}\n{}", receipt_text(receipt)),
        ),
    }
}
impl ItemForgePanel {
    pub fn confirm_resource(&mut self, serial: u64, occurrence: [u8; 16]) -> Option<Request> {
        if !self.selected
            || self.pending
            || self.retry.is_some()
            || self.preview.pending.is_some()
            || self.resource_dialog.serial != serial
        {
            return None;
        }
        if self
            .resource_dialog
            .expires
            .is_none_or(|at| at <= Instant::now())
        {
            self.resource_dialog
                .message("This resource review expired. Choose the operation again to refresh it.");
            return None;
        }
        let Some(Content::Confirm { operation, quote }) = self.resource_dialog.content.clone()
        else {
            return None;
        };
        if !wire::resource_operation(operation)
            || quote.is_some_and(|q| q.operation != operation)
            || occurrence[6] >> 4 != 7
            || occurrence[8] & 192 != 128
        {
            return None;
        }
        let request = Request {
            operation,
            occurrence,
            first_item: None,
            second_item: None,
            boost: false,
            protect: false,
            history_limit: 0,
        };
        request.encode()?;
        self.resource_dialog.reset();
        Some(request)
    }
}
#[cfg(test)]
#[path = "resource_dialog_tests.rs"]
mod tests;
