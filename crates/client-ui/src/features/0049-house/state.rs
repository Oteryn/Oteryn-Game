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
    pub pending_index: usize,
    pub world: [u8; 16],
    pub town: String,
    pub page: Page,
    pub index: usize,
    pub feedback: String,
}
impl Panel {
    pub fn apply(&mut self, command: &FeatureCommand, reply: &GatedCommandResult) {
        if self.pending.as_ref() != Some(command) {
            return;
        }
        self.pending = None;
        if !matches!(
            reply.status,
            CommandStatus::Accepted | CommandStatus::DuplicateReplay
        ) {
            self.feedback = tr!("The server refused the request").into();
            return;
        }
        let result = if command.kind == 57 {
            house::page(&reply.payload).map(|page| {
                self.page = page;
                self.index = 0;
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
        self.feedback = result.map_or_else(str::to_owned, |()| tr!("House updated").into());
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
