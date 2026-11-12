//! Three independently docked rows; layout edits are local and never predict a cast.
use crate::play::{PlayLink, mailbox::lock};
use serde::{Deserialize, Serialize};

pub const ROW_SLOTS: usize = 50;
fn default_count() -> u8 {
    50
}
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
    #[serde(default = "default_count")]
    pub count: u8,
}
impl RowLayout {
    pub fn visible_slots(&self, capacity: usize) -> std::ops::Range<usize> {
        let count = capacity.min(usize::from(self.count));
        let offset = usize::from(self.offset).min(usize::from(self.count) - count);
        offset..offset + count
    }
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
                count: default_count(),
            }),
        }
    }
}
impl BarLayout {
    pub fn valid(&self) -> bool {
        self.rows
            .iter()
            .all(|row| (1..=50).contains(&row.count) && row.offset < row.count)
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
    Aim {
        slot: usize,
        aim: super::SpellAim,
    },
    AssignMultiaction {
        slot: usize,
    },
    Count {
        bar: usize,
        count: u8,
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
                    .is_some_and(|row| {
                        row.enabled && row.locked && slot % ROW_SLOTS < usize::from(row.count)
                    })
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
        if let BarGesture::AssignMultiaction { slot } = gesture {
            if slot >= oteryn_client_runtime::spell::ACTION_BAR_SLOTS {
                return false;
            }
            #[cfg(windows)]
            super::open_multi(self, slot);
            return true;
        }
        let mut mailbox = lock(&self.mailbox);
        if let BarGesture::Aim { slot, aim } = gesture {
            if slot >= oteryn_client_runtime::spell::ACTION_BAR_SLOTS {
                return false;
            }
            let action = match mailbox.bar_actions.saved.0.get(&slot) {
                Some(super::BarAction::Spell { name, words, .. }) => super::BarAction::Spell {
                    name: name.clone(),
                    words: words.clone(),
                    aim,
                },
                Some(_) => return false,
                None => {
                    let Some(index) = u32::try_from(slot + 1)
                        .ok()
                        .and_then(std::num::NonZeroU32::new)
                        .and_then(|slot| mailbox.spell_bar.bar.spell(slot))
                    else {
                        return false;
                    };
                    let Some(entry) = mailbox
                        .spell_catalog
                        .catalog
                        .entries()
                        .iter()
                        .find(|entry| entry.cast_index == index.get())
                    else {
                        return false;
                    };
                    super::BarAction::Spell {
                        name: entry.name.clone(),
                        words: entry.words.clone(),
                        aim,
                    }
                }
            };
            return super::assignments::set_action(&mut mailbox, slot, action);
        }
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
                    count: row.count,
                };
            }
            BarGesture::Count { bar, count } => {
                if !(1..=50).contains(&count) {
                    return false;
                }
                let Some(row) = mailbox.bar_actions.layout.rows.get_mut(bar) else {
                    return false;
                };
                row.count = count;
                row.offset = row.offset.min(count - 1);
            }
            BarGesture::Scroll { bar, by } => {
                let Some(row) = mailbox.bar_actions.layout.rows.get_mut(bar) else {
                    return false;
                };
                row.offset = row.offset.saturating_add_signed(by).min(row.count - 1);
            }
            BarGesture::Swap { from, to } => {
                if [from, to].iter().any(|slot| {
                    mailbox
                        .bar_actions
                        .layout
                        .rows
                        .get(slot / ROW_SLOTS)
                        .is_none_or(|row| {
                            !row.enabled || row.locked || slot % ROW_SLOTS >= usize::from(row.count)
                        })
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
            BarGesture::Cast { .. }
            | BarGesture::AssignMultiaction { .. }
            | BarGesture::Aim { .. } => return false,
        }
        mailbox.bar_actions.edited();
        true
    }
}
