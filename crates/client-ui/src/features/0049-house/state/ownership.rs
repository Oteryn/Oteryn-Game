//! Exact server selections and explicit confirmations for every disposition action.
use oteryn_client_runtime::{
    house::house_sale as wire,
    raw_feature::{CommandStatus, FeatureCommand, GatedCommandResult},
};
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Confirmation {
    intent: wire::HouseSaleRequest,
    identity: std::sync::Arc<()>,
    text: String,
}
impl Confirmation {
    pub fn text(&self) -> &str {
        &self.text
    }
}
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OwnershipPanel {
    selected: bool,
    data: Option<wire::HouseSaleReply>,
    index: usize,
    recipient: String,
    due: i64,
    price: u64,
    confirmation: Option<Confirmation>,
    authorized: Option<wire::HouseSaleRequest>,
    pending: Option<FeatureCommand>,
    pub feedback: String,
}
impl OwnershipPanel {
    pub fn admitted(&mut self, capabilities: &[u32]) {
        *self = Self {
            selected: capabilities.contains(&wire::CAPABILITY),
            ..Self::default()
        };
    }
    pub fn ended(&mut self) {
        *self = Self::default();
    }
    pub fn data(&self) -> Option<&wire::HouseSaleReply> {
        self.data.as_ref()
    }
    pub fn row(&self) -> Option<&wire::HouseSaleRow> {
        self.data.as_ref()?.rows.get(self.index)
    }
    pub fn cancel_review(&mut self) {
        self.confirmation = None;
        self.authorized = None;
    }
    pub fn available(&self) -> bool {
        self.selected
    }
    pub fn index(&self) -> usize {
        self.index
    }
    pub fn accepts(&self, command: &FeatureCommand) -> bool {
        self.pending.as_ref() == Some(command)
    }
    pub fn pending(&self) -> bool {
        self.pending.is_some()
    }
    pub fn select(&mut self, index: usize) -> Result<(), &'static str> {
        if self.pending() || self.data.as_ref().is_none_or(|d| index >= d.rows.len()) {
            return Err("Select a house");
        }
        self.index = index;
        self.confirmation = None;
        self.authorized = None;
        Ok(())
    }
    pub fn terms(&mut self, recipient: String, due: i64, price: u64) {
        self.recipient = recipient;
        self.due = due;
        self.price = price;
        self.confirmation = None;
        self.authorized = None;
    }
    pub fn query(&mut self, after_house: String) -> Result<FeatureCommand, &'static str> {
        super::protocol::ownership_request(self, &wire::HouseSaleRequest::catalogue(after_house))
    }
    fn intent(&self, action: u8) -> Result<wire::HouseSaleRequest, &'static str> {
        if self.pending() {
            return Err("Waiting for the server");
        }
        let row = self.row().ok_or("Select a house")?;
        if !(1..=6).contains(&action) || row.allowed & (1 << action) == 0 {
            return Err("This action is unavailable");
        }
        let request = wire::HouseSaleRequest {
            action,
            house: row.house.clone(),
            expected_revision: row.revision,
            due_at_unix_ms: if matches!(action, 1 | 3) { self.due } else { 0 },
            recipient: if action == 3 {
                self.recipient.clone()
            } else {
                String::new()
            },
            price: if action == 3 { self.price } else { 0 },
            after_house: String::new(),
        };
        if !request.valid()
            || (action == 3
                && self
                    .price
                    .checked_add(row.rent)
                    .is_none_or(|v| v > wire::MAX_GOLD))
        {
            return Err("Check the recipient, date and gold price");
        }
        Ok(request)
    }
    pub fn prepare(&mut self, action: u8) -> Result<Confirmation, &'static str> {
        let request = self.intent(action)?;
        let row = self.row().ok_or("Select a house")?;
        let body = match action {
            1 => format!(
                "Move out of {} at {}? Movable contents go to depot mail inboxes; bound items go to their owners.",
                row.name,
                super::ownership_date::format(request.due_at_unix_ms)
            ),
            2 => format!(
                "Cancel the scheduled move-out from {} on {}?",
                row.name,
                super::ownership_date::format(row.due_at_unix_ms)
            ),
            3 => format!(
                "Offer {} to {} at {} for {} gold? The recipient must also reserve {} gold rent. The house is emptied before handover.",
                row.name,
                request.recipient,
                super::ownership_date::format(request.due_at_unix_ms),
                request.price,
                row.rent
            ),
            4 => format!(
                "Cancel the transfer of {} to {} on {}? Any reserved {} gold will be refunded.",
                row.name,
                row.recipient,
                super::ownership_date::format(row.due_at_unix_ms),
                row.price + row.rent
            ),
            5 => format!(
                "Accept {} from {} on {} for {} gold plus {} gold rent ({} gold total)? The server reserves the total until the empty house is handed over.",
                row.name,
                row.owner,
                super::ownership_date::format(row.due_at_unix_ms),
                row.price,
                row.rent,
                row.price + row.rent
            ),
            6 => format!(
                "Reject the transfer of {} on {}? Any reserved {} gold will be refunded.",
                row.name,
                super::ownership_date::format(row.due_at_unix_ms),
                row.price + row.rent
            ),
            _ => return Err("This action is unavailable"),
        };
        let review = Confirmation {
            intent: request,
            identity: std::sync::Arc::new(()),
            text: body,
        };
        self.confirmation = Some(review.clone());
        Ok(review)
    }
    pub fn confirm(
        &mut self,
        review: Confirmation,
        accepted: bool,
    ) -> Result<Option<FeatureCommand>, &'static str> {
        let matches = self.confirmation.as_ref().is_some_and(|current| {
            std::sync::Arc::ptr_eq(&current.identity, &review.identity)
                && current.intent == review.intent
        });
        if !matches {
            return if accepted {
                Err("The review changed; review the action again")
            } else {
                Ok(None)
            };
        }
        self.confirmation = None;
        if !accepted {
            return Ok(None);
        }
        if self.intent(review.intent.action)? != review.intent {
            return Err("The selection changed; review the action again");
        }
        self.authorized = Some(review.intent.clone());
        super::protocol::ownership_request(self, &review.intent).map(Some)
    }
    pub fn apply(&mut self, command: &FeatureCommand, result: &GatedCommandResult) {
        if self.pending.as_ref() != Some(command) {
            return;
        }
        self.pending = None;
        self.confirmation = None;
        self.authorized = None;
        if !matches!(
            result.status,
            CommandStatus::Accepted | CommandStatus::DuplicateReplay
        ) {
            self.data = None;
            self.feedback = "The server refused the action; refresh the house list".into();
            return;
        }
        match wire::decode_reply(&result.payload) {
            Ok(data) => {
                self.data = Some(data);
                self.index = 0;
                self.feedback = "House information updated".into();
            }
            Err(_) => {
                self.data = None;
                self.feedback = "Invalid house reply".into();
            }
        }
    }
}
pub fn request(
    panel: &mut OwnershipPanel,
    intent: &wire::HouseSaleRequest,
) -> Result<FeatureCommand, &'static str> {
    if !panel.selected {
        return Err("Unavailable in this session");
    }
    if panel.pending() {
        return Err("Waiting for the server");
    }
    if intent.action != 0
        && (panel.authorized.as_ref() != Some(intent) || panel.intent(intent.action)? != *intent)
    {
        return Err("Review the action before confirming");
    }
    let command = FeatureCommand {
        capability: wire::CAPABILITY,
        kind: wire::COMMAND,
        payload: wire::encode_request(intent).map_err(|_| "Invalid house request")?,
    };
    panel.confirmation = None;
    panel.authorized = None;
    panel.pending = Some(command.clone());
    Ok(command)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn loaded() -> Result<OwnershipPanel, &'static str> {
        let mut panel = OwnershipPanel::default();
        panel.admitted(&[wire::CAPABILITY]);
        let query = panel.query(String::new())?;
        let data = wire::HouseSaleReply {
            now_unix_ms: 1_800_000_000_000,
            has_more: false,
            rows: vec![wire::HouseSaleRow {
                house: "oteryn:content.house.admiral_s_avenue_1".into(),
                name: "Harbour House".into(),
                revision: 7,
                owner: "Elara Vale".into(),
                recipient: String::new(),
                transfer_state: 0,
                due_at_unix_ms: 0,
                price: 0,
                rent: 50,
                allowed: (1 << 1) | (1 << 3),
                move_out_state: 0,
            }],
        };
        panel.apply(
            &query,
            &GatedCommandResult {
                command_id: 1,
                server_sequence: 1,
                status: CommandStatus::Accepted,
                payload: wire::encode_reply(&data).map_err(|_| "fixture encoding")?,
            },
        );
        Ok(panel)
    }
    #[test]
    fn house_ownership_exact_selection_confirmation_is_required_and_edits_invalidate_it()
    -> Result<(), &'static str> {
        let mut panel = loaded()?;
        panel.terms("Mira Vale".into(), 1_800_172_800_000, 200);
        let intent = panel.intent(3)?;
        assert!(super::super::protocol::ownership_request(&mut panel, &intent).is_err());
        let review = panel.prepare(3)?;
        for expected in ["Harbour House", "Mira Vale", "200 gold", "50 gold rent"] {
            assert!(review.text().contains(expected));
        }
        assert!(
            super::super::protocol::ownership_request(&mut panel, &intent).is_err(),
            "preparing is not confirmation"
        );
        assert_eq!(panel.confirm(review, false)?, None);
        let stale = panel.prepare(3)?;
        panel.terms("Mira Vale".into(), 1_800_172_800_000, 201);
        assert!(panel.confirm(stale, true).is_err());
        let stale = panel.prepare(3)?;
        panel.select(0)?;
        assert!(panel.confirm(stale, true).is_err());
        let old_review = panel.prepare(3)?;
        let current_review = panel.prepare(3)?;
        assert!(
            panel.confirm(old_review, true).is_err(),
            "another dialog cannot replace what was reviewed"
        );
        let duplicate = current_review.clone();
        let command = panel
            .confirm(current_review, true)?
            .ok_or("confirmed action must emit")?;
        let decoded = wire::decode_request(&command.payload).map_err(|_| "request decoding")?;
        assert_eq!(
            (
                decoded.house.as_str(),
                decoded.expected_revision,
                decoded.price
            ),
            ("oteryn:content.house.admiral_s_avenue_1", 7, 201)
        );
        assert!(panel.confirm(duplicate, true).is_err());
        panel.ended();
        assert!(panel.query(String::new()).is_err());
        Ok(())
    }
}
