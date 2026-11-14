//! Atomic local preference edits over the retained battle-list owner.
use super::*;
use std::sync::{
    Arc, Mutex, Weak,
    atomic::{AtomicBool, Ordering},
};

#[derive(Clone)]
pub(crate) struct BattleConfigBinding {
    mailbox: Weak<Mutex<crate::play::mailbox::Mailbox>>,
    used: Arc<AtomicBool>,
    pub(crate) lists: BattleListsState,
    pub(crate) open: bool,
}
impl BattleConfigBinding {
    pub(crate) fn same_projection(&self, other: &Self) -> bool {
        !self.used.load(Ordering::Relaxed)
            && self.mailbox.ptr_eq(&other.mailbox)
            && self.open == other.open
            && self.lists == other.lists
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum BattleConfigAction {
    Open,
    Close,
    Select(usize),
    Save(BattleListPreferences),
    Add,
    Remove(bool),
}
impl PlayLink {
    pub(crate) fn battle_config_binding(&self) -> BattleConfigBinding {
        let mailbox = lock(&self.mailbox);
        BattleConfigBinding {
            mailbox: Arc::downgrade(&self.mailbox),
            used: Arc::default(),
            lists: mailbox.battle_lists.clone(),
            open: mailbox.battle.open && !mailbox.battle.shutdown,
        }
    }
    pub(crate) fn battle_config_apply(
        &self,
        expected: &BattleConfigBinding,
        action: BattleConfigAction,
    ) -> bool {
        if !expected.mailbox.ptr_eq(&Arc::downgrade(&self.mailbox)) {
            return false;
        }
        let mut mailbox = lock(&self.mailbox);
        if !mailbox.shared_frontend
            || mailbox.custom_hotkeys_ended
            || mailbox.battle.shutdown
            || self.commands.is_closed()
        {
            return false;
        }
        // Close depends only on the session. Combat settlement belongs to the owner.
        if action == BattleConfigAction::Close {
            mailbox.battle.open = false;
            mailbox.battle_lists.edit = next_edit();
            return true;
        }
        if expected.used.load(Ordering::Relaxed)
            || mailbox.battle.open != expected.open
            || mailbox.battle_lists != expected.lists
            || mailbox.server_modal.is_active()
            || mailbox.world_input_blocked()
            || mailbox.lens.active
        {
            return false;
        }
        if action == BattleConfigAction::Open {
            if mailbox.battle.open {
                return false;
            }
            expected.used.store(true, Ordering::Relaxed);
            mailbox.battle.open = true;
            mailbox.battle_lists.edit = next_edit();
            return true;
        }
        if !mailbox.battle.open {
            return false;
        }
        let mut candidate = mailbox.battle_lists.preferences.clone();
        let mut active = mailbox.battle_lists.active;
        match action {
            BattleConfigAction::Select(index) => {
                if index >= candidate.len() {
                    return false;
                }
                active = index;
            }
            BattleConfigAction::Save(mut draft) => {
                let Some(target) = candidate.get_mut(active) else {
                    return false;
                };
                draft.name = draft.name.trim().into();
                draft.search = draft.search.trim().into();
                *target = draft;
            }
            BattleConfigAction::Add => {
                if candidate.len() >= 4 {
                    return false;
                }
                let Some(name) = (1..=8).map(|i| format!("List {i}")).find(|name| {
                    !candidate
                        .iter()
                        .any(|list| list.name.eq_ignore_ascii_case(name))
                }) else {
                    return false;
                };
                candidate.push(BattleListPreferences {
                    name,
                    ..Default::default()
                });
                active = candidate.len() - 1;
            }
            BattleConfigAction::Remove(confirmed) => {
                if !confirmed || candidate.len() <= 1 || active >= candidate.len() {
                    return false;
                }
                candidate.remove(active);
                active = active.min(candidate.len() - 1);
            }
            BattleConfigAction::Open | BattleConfigAction::Close => return false,
        }
        if !validate_lists(&candidate) {
            return false;
        }
        expected.used.store(true, Ordering::Relaxed);
        mailbox.battle_lists.preferences = candidate;
        mailbox.battle_lists.active = active;
        mailbox.battle_lists.dirty = true;
        mailbox.battle_lists.edit = next_edit();
        true
    }
}
