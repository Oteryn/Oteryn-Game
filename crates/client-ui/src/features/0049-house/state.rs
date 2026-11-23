//! Catalogue/controller state; W17 owns final mailbox and native-window mounting.
use oteryn_client_runtime::{
    house::{self, Page},
    raw_feature::{CommandStatus, FeatureCommand, GatedCommandResult},
};
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Panel {
    pub selected: bool,
    pub open: bool,
    pub pending: Option<FeatureCommand>,
    pub pending_acl: bool,
    pub pending_acl_read: bool,
    pub acl_snapshot: Option<house::HouseAclSnapshot>,
    pub pending_index: usize,
    pub world: [u8; 16],
    pub town: String,
    pub page: Page,
    /// First keyset cursor of the displayed rows, or None for the first page.
    pub page_after: Option<String>,
    pub pending_after: Option<String>,
    pub collecting: bool,
    pub index: usize,
    pub feedback: String,
}
impl Panel {
    pub fn apply(&mut self, command: &FeatureCommand, reply: &GatedCommandResult) {
        if self.pending.as_ref() != Some(command) {
            return;
        }
        self.pending = None;
        let after = self.pending_after.take();
        let acl_read = std::mem::take(&mut self.pending_acl_read);
        if !matches!(
            reply.status,
            CommandStatus::Accepted | CommandStatus::DuplicateReplay
        ) {
            self.collecting = false;
            self.feedback = tr!("The server refused the request").into();
            return;
        }
        let result = if acl_read {
            house::decode_house_acl_snapshot(&reply.payload)
                .map_err(|_| "Invalid house ACL reply")
                .map(|snapshot| {
                    if let Some(row) = self.page.rows.get_mut(self.pending_index) {
                        row.acl_revision = snapshot.revision;
                    }
                    self.acl_snapshot = Some(snapshot);
                })
        } else if command.kind == 57 {
            house::page(&reply.payload).and_then(|mut page| {
                if self.collecting
                    && let Some(cursor) = after.as_ref()
                {
                    if self.page.cursor.as_ref() != Some(cursor)
                        || page
                            .rows
                            .first()
                            .is_some_and(|first| first.house <= *cursor)
                    {
                        return Err("Invalid house catalogue continuation");
                    }
                    if self.page.rows.len().saturating_add(page.rows.len()) > 16_384 {
                        return Err("House catalogue exceeds the browse limit");
                    }
                    self.page.rows.append(&mut page.rows);
                    self.page.cursor = page.cursor;
                } else {
                    self.page_after = after;
                    self.page = page;
                    self.index = 0;
                }
                self.collecting &= self.page.cursor.is_some();
                Ok(())
            })
        } else {
            house::result(&reply.payload).and_then(|(d, revision)| {
                if d != 1 && d != 11 {
                    return Err(match d {
                        3 => tr!("The server refused the action: catalogue changed; refresh before retrying"),
                        8 => tr!("The server refused the action: insufficient funds"),
                        9 => tr!("The server refused the action: balance limit reached"),
                        _ => tr!("The server refused the action"),
                    });
                }
                if let Some(row) = self.page.rows.get_mut(self.pending_index)
                    && self.pending_acl
                    && revision != 0
                {
                    row.acl_revision = revision;
                }
                Ok(())
            })
        };
        if result.is_err() {
            self.collecting = false;
        }
        self.feedback = result.map_or_else(str::to_owned, |()| {
            if acl_read {
                "Access list loaded; edit members and confirm the matching action".into()
            } else {
                tr!("House updated").into()
            }
        });
    }
    pub fn admitted(&mut self, world: [u8; 16], capabilities: &[u32]) {
        self.ended();
        self.world = world;
        self.selected = capabilities.contains(&49);
    }
    pub fn ended(&mut self) {
        *self = Self::default();
    }
}
