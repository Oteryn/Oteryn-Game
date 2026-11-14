//! Session-bound reads and atomic local edits over the retained action-bar owner.
use super::*;
use crate::play::feature_local_spell_action_bar::view::{BarLayout, ItemUseMode, SpellAim};
use std::sync::{Arc, Mutex, Weak};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActionBarEditorChoice {
    pub label: String,
    pub action: BarAction,
    pub known: bool,
}
#[derive(Clone, Debug)]
pub struct ActionBarEditorSnapshot {
    session: Weak<Mutex<Mailbox>>,
    revision: u64,
    window_revision: u64,
    pub open: bool,
    pub layout: BarLayout,
    pub actions: BarActions,
    pub bindings: Vec<String>,
    pub choices: Vec<ActionBarEditorChoice>,
}
impl ActionBarEditorSnapshot {
    pub fn same_session(&self, other: &Self) -> bool {
        self.session.ptr_eq(&other.session)
    }
}
impl PartialEq for ActionBarEditorSnapshot {
    fn eq(&self, other: &Self) -> bool {
        self.session.ptr_eq(&other.session)
            && self.revision == other.revision
            && self.window_revision == other.window_revision
            && self.open == other.open
            && self.layout == other.layout
            && self.actions == other.actions
            && self.bindings == other.bindings
            && self.choices == other.choices
    }
}
impl Eq for ActionBarEditorSnapshot {}
fn snapshot(link: &PlayLink, mailbox: &Mailbox) -> ActionBarEditorSnapshot {
    let mut choices = oteryn_client_ui::spell_action_bar::dialog_spells(
        mailbox.spell_catalog.catalog.entries(),
        false,
    )
    .into_iter()
    .map(|entry| ActionBarEditorChoice {
        label: entry.name.clone(),
        known: entry.learned,
        action: BarAction::Spell {
            name: entry.name.clone(),
            words: entry.words.clone(),
            aim: SpellAim::Target,
        },
    })
    .collect::<Vec<_>>();
    let mut definitions = std::collections::BTreeSet::new();
    for item in super::super::item_actions::visible_items(mailbox) {
        if definitions.insert(item.item_definition_ref) {
            choices.push(ActionBarEditorChoice {
                label: format!("Object {}", item.item_definition_ref),
                known: true,
                action: BarAction::Use {
                    definition: item.item_definition_ref,
                    mode: ItemUseMode::CursorPosition,
                },
            });
        }
    }
    ActionBarEditorSnapshot {
        session: Arc::downgrade(&link.mailbox),
        revision: mailbox.bar_actions.edit,
        window_revision: mailbox.bar_actions.editor_revision,
        open: mailbox.spell_bar.open && !link.commands.is_closed(),
        layout: mailbox.bar_actions.layout.clone(),
        actions: mailbox.bar_actions.saved.clone(),
        bindings: mailbox.spell_bar.bar.fields(),
        choices,
    }
}
impl PlayLink {
    pub fn action_bar_editor_snapshot(&self) -> ActionBarEditorSnapshot {
        snapshot(self, &lock(&self.mailbox))
    }
    pub fn action_bar_editor_input_bound(
        &self,
        events: &[oteryn_input_actions::NormalizedInputEvent],
    ) -> bool {
        oteryn_client_ui::spell_action_bar::open_key(events) && self.action_bar_editor_open()
    }
    pub fn action_bar_editor_open(&self) -> bool {
        let mut mailbox = lock(&self.mailbox);
        if self.commands.is_closed()
            || mailbox.server_modal.is_active()
            || mailbox.world_input_blocked()
        {
            return false;
        }
        if !mailbox.spell_bar.open {
            mailbox.spell_bar.open = true;
            mailbox.bar_actions.editor_revision =
                crate::play::feature_0017_attack::view::next_edit();
        }
        true
    }
    /// Close checks only session identity. Draft/catalog/modal changes cannot trap the window.
    pub fn action_bar_editor_close_bound(&self, expected: &ActionBarEditorSnapshot) -> bool {
        let mut mailbox = lock(&self.mailbox);
        if !expected.session.ptr_eq(&Arc::downgrade(&self.mailbox)) || self.commands.is_closed() {
            return false;
        }
        mailbox.spell_bar.open = false;
        mailbox.bar_actions.editor_revision = crate::play::feature_0017_attack::view::next_edit();
        true
    }
    pub fn action_bar_editor_layout_bound(
        &self,
        expected: &ActionBarEditorSnapshot,
        layout: BarLayout,
    ) -> bool {
        let mut mailbox = lock(&self.mailbox);
        if !admit(self, &mailbox, expected)
            || !layout.valid()
            || layout == mailbox.bar_actions.layout
        {
            return false;
        }
        mailbox.bar_actions.layout = layout;
        mailbox.bar_actions.edited();
        true
    }
    pub fn action_bar_editor_multi_bound(
        &self,
        expected: &ActionBarEditorSnapshot,
        slot: usize,
        action: BarAction,
    ) -> bool {
        let mut mailbox = lock(&self.mailbox);
        if !admit(self, &mailbox, expected) {
            return false;
        }
        let BarAction::Multi { actions } = &action else {
            return false;
        };
        // Only identities from the same live native picker may enter a multiaction.
        if actions.iter().any(|action| {
            !expected
                .choices
                .iter()
                .any(|choice| match (&choice.action, action) {
                    (
                        BarAction::Spell { name, words, .. },
                        BarAction::Spell {
                            name: n, words: w, ..
                        },
                    ) => name == n && words == w,
                    (BarAction::Use { definition, .. }, BarAction::Use { definition: d, .. }) => {
                        definition == d
                    }
                    _ => false,
                })
        }) {
            return false;
        }
        set_action(&mut mailbox, slot, action)
    }
    pub fn action_bar_editor_labels(&self, actions: &[Option<BarAction>; 3]) -> [String; 3] {
        let mailbox = lock(&self.mailbox);
        std::array::from_fn(|i| {
            actions[i]
                .as_ref()
                .map_or_else(|| "Empty".into(), |action| label(&mailbox, action))
        })
    }
}
fn admit(link: &PlayLink, mailbox: &Mailbox, expected: &ActionBarEditorSnapshot) -> bool {
    !link.commands.is_closed()
        && !mailbox.world_input_blocked()
        && !mailbox.server_modal.is_active()
        && expected.open
        && snapshot(link, mailbox) == *expected
}
