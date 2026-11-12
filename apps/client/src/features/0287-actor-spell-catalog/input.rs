use crate::play::PlayLink;
use oteryn_input_actions::NormalizedInputEvent;
impl PlayLink {
    pub fn spell_catalog_input(&self, events: &[NormalizedInputEvent]) -> bool {
        if oteryn_client_ui::spell_catalog::toggle_key(events) {
            self.spell_list_toggle();
            return true;
        }
        if !oteryn_client_ui::spell_catalog::open_key(events) {
            return false;
        }
        self.spell_catalog_open();
        true
    }
    pub fn spell_catalog_assign(&self, slot: usize) -> bool {
        assign(&mut crate::play::mailbox::lock(&self.mailbox), slot)
    }
    pub fn magical_archive_open(&self) {
        #[cfg(windows)]
        super::view::open_archive(self);
        #[cfg(not(windows))]
        {
            crate::play::mailbox::lock(&self.mailbox)
                .spell_catalog
                .archive_open = true;
        }
    }
    /// ToggleSpellListWidget: closes the spell list (catalogue or spellbook) when shown, else opens it.
    pub fn spell_list_toggle(&self) {
        {
            let mut mailbox = crate::play::mailbox::lock(&self.mailbox);
            if mailbox.spell_catalog.open || mailbox.spellbook_details.open {
                mailbox.spell_catalog.open = false;
                mailbox.spellbook_details.open = false;
                return;
            }
        }
        self.spell_catalog_open();
    }
    pub fn spell_catalog_open(&self) {
        if self.spellbook_open() {
            return;
        }
        #[cfg(windows)]
        super::view::open_window(self);
        #[cfg(not(windows))]
        {
            crate::play::mailbox::lock(&self.mailbox).spell_catalog.open = true;
        }
    }
}

pub(super) fn assign(mailbox: &mut crate::play::mailbox::Mailbox, slot: usize) -> bool {
    let Some(spell) = mailbox
        .spell_catalog
        .selected_spell()
        .and_then(|entry| std::num::NonZeroU32::new(entry.cast_index))
    else {
        return false;
    };
    if !mailbox.spell_bar.bar.bind(slot, Some(spell)) {
        return false;
    }
    mailbox.spell_bar.feedback = format!("Selected spell assigned to slot {}", slot + 1);
    true
}

impl PlayLink {
    pub fn spell_catalog_search(
        &self,
        search: &str,
        level: &str,
        group: &str,
        learned: bool,
        vocation: &str,
    ) -> bool {
        let mut mailbox = crate::play::mailbox::lock(&self.mailbox);
        filter(
            &mut mailbox,
            &[
                search.into(),
                level.into(),
                group.into(),
                u8::from(learned).to_string(),
                vocation.into(),
            ],
        )
    }
}
pub(super) fn filter(mailbox: &mut crate::play::mailbox::Mailbox, values: &[String]) -> bool {
    use oteryn_client_ui::spell_catalog::protocol;
    if values.len() != 5 || !matches!(values[3].as_str(), "0" | "1") {
        mailbox.spell_catalog.filter_error = true;
        return false;
    }
    if !protocol::set_filters(
        &mut mailbox.spell_catalog,
        &values[1],
        &values[2],
        values[3] == "1",
    ) || !protocol::set_vocation(&mut mailbox.spell_catalog, &values[4])
    {
        return false;
    }
    if mailbox.spell_catalog.filter != values[0] {
        protocol::navigate(&mut mailbox.spell_catalog, &values[0], 0);
    }
    true
}
