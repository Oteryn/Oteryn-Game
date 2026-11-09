//! Three independently docked rows; layout edits are local and never predict a cast.
use crate::play::{PlayLink, mailbox::lock};
use serde::{Deserialize, Serialize};

pub const ROW_SLOTS: usize = 12;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BarDock {
    Bottom,
    Left,
    Right,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RowLayout {
    pub dock: BarDock,
    pub enabled: bool,
    pub locked: bool,
    pub offset: u8,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BarLayout {
    pub rows: [RowLayout; 3],
}
impl Default for BarLayout {
    fn default() -> Self {
        Self {
            rows: [BarDock::Bottom, BarDock::Left, BarDock::Right].map(|dock| RowLayout {
                dock,
                enabled: dock == BarDock::Bottom,
                locked: true,
                offset: 0,
            }),
        }
    }
}
impl BarLayout {
    pub fn valid(&self) -> bool {
        self.rows
            .iter()
            .all(|row| usize::from(row.offset) < ROW_SLOTS)
    }
}
#[derive(Clone, Copy, Debug)]
pub enum BarGesture {
    Configure {
        bar: usize,
        dock: BarDock,
        enabled: bool,
        locked: bool,
    },
    Scroll {
        bar: usize,
        by: i8,
    },
    Swap {
        from: usize,
        to: usize,
    },
    Cast {
        slot: usize,
    },
}
impl PlayLink {
    pub fn bar_layout(&self) -> BarLayout {
        lock(&self.mailbox).bar_actions.layout.clone()
    }
    pub fn bar_gesture(&self, gesture: BarGesture) -> bool {
        if let BarGesture::Cast { slot } = gesture {
            let allowed = {
                let mailbox = lock(&self.mailbox);
                mailbox
                    .bar_actions
                    .layout
                    .rows
                    .get(slot / ROW_SLOTS)
                    .is_some_and(|row| row.enabled && row.locked)
            };
            if !allowed {
                return false;
            }
            if let Some(slot) = u32::try_from(slot + 1)
                .ok()
                .and_then(std::num::NonZeroU32::new)
            {
                self.cast_spell_slot(slot);
                return true;
            }
            return false;
        }
        let mut mailbox = lock(&self.mailbox);
        match gesture {
            BarGesture::Configure {
                bar,
                dock,
                enabled,
                locked,
            } => {
                let Some(row) = mailbox.bar_actions.layout.rows.get_mut(bar) else {
                    return false;
                };
                *row = RowLayout {
                    dock,
                    enabled,
                    locked,
                    offset: row.offset,
                };
            }
            BarGesture::Scroll { bar, by } => {
                let Some(row) = mailbox.bar_actions.layout.rows.get_mut(bar) else {
                    return false;
                };
                row.offset = row.offset.saturating_add_signed(by).min(11);
            }
            BarGesture::Swap { from, to } => {
                if [from, to].iter().any(|slot| {
                    mailbox
                        .bar_actions
                        .layout
                        .rows
                        .get(slot / ROW_SLOTS)
                        .is_none_or(|row| !row.enabled || row.locked)
                }) {
                    return false;
                }
                mailbox.spellbook_details.archive.slots.swap(from, to);
                mailbox.spellbook_details.archive_edit =
                    crate::play::feature_0017_attack::view::next_edit();
                mailbox.spell_bar.bar.swap(from, to);
                let first = mailbox.bar_actions.saved.0.remove(&from);
                let second = mailbox.bar_actions.saved.0.remove(&to);
                if let Some(action) = first {
                    mailbox.bar_actions.saved.0.insert(to, action);
                }
                if let Some(action) = second {
                    mailbox.bar_actions.saved.0.insert(from, action);
                }
            }
            BarGesture::Cast { .. } => return false,
        }
        mailbox.bar_actions.edited();
        true
    }
}
