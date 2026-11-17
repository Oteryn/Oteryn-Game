use crate::play::mailbox::Mailbox;
use oteryn_client_runtime::raw_feature::{CommandStatus, FeatureCommand, GatedCommandResult};
use oteryn_client_ui::house::{format_ownership_date, sale::OwnershipPanel};
use std::sync::{Mutex, Weak};
#[derive(Clone, Debug, Default)]
pub struct Ownership {
    pub model: OwnershipPanel,
    pub open: bool,
    pub fields: [String; 3],
    pub(in crate::play) context: Weak<Mutex<Mailbox>>,
    pub(in crate::play) cursor: String,
    pub(in crate::play) history: Vec<String>,
    pub(in crate::play) navigation: Option<(String, Vec<String>)>,
}
impl Ownership {
    pub(in crate::play) fn apply(&mut self, command: &FeatureCommand, result: &GatedCommandResult) {
        if !self.model.accepts(command) {
            return;
        }
        self.model.apply(command, result);
        if matches!(
            result.status,
            CommandStatus::Accepted | CommandStatus::DuplicateReplay
        ) && self.model.data().is_some()
        {
            if let Some((cursor, history)) = self.navigation.take() {
                self.cursor = cursor;
                self.history = history;
            } else {
                self.cursor.clear();
                self.history.clear();
            }
            self.sync_fields();
        } else {
            self.navigation = None;
            self.fields = Default::default();
        }
    }
    pub(in crate::play) fn sync_fields(&mut self) {
        self.fields = Default::default();
        if let Some(row) = self.model.row() {
            let due = if row.due_at_unix_ms > 0 {
                row.due_at_unix_ms
            } else {
                self.model.data().map_or(0, |d| d.now_unix_ms + 172_800_000)
            };
            self.fields[1] = format_ownership_date(due);
            if row.transfer_state != 0 {
                self.fields[0] = row.recipient.clone();
                self.fields[2] = row.price.to_string();
            }
        }
    }
    pub(in crate::play) fn select(&mut self, index: usize) -> Result<(), &'static str> {
        self.model.select(index)?;
        self.sync_fields();
        Ok(())
    }
    pub(in crate::play) fn page(&mut self, direction: i8) -> Result<FeatureCommand, &'static str> {
        let (cursor, history) = if direction > 0 {
            let data = self.model.data().ok_or("Refresh the house list")?;
            if !data.has_more {
                return Err("No next page");
            }
            let cursor = data.rows.last().ok_or("No next page")?.house.clone();
            let mut history = self.history.clone();
            if history.len() == 4096 {
                history.remove(0);
            }
            history.push(self.cursor.clone());
            (cursor, history)
        } else if direction < 0 {
            let mut history = self.history.clone();
            let cursor = history.pop().ok_or("No previous page")?;
            (cursor, history)
        } else {
            (String::new(), Vec::new())
        };
        let command = self.model.query(cursor.clone())?;
        self.navigation = Some((cursor, history));
        Ok(command)
    }
}
