use crate::play::{Outbox, mailbox::lock};
use oteryn_session::{GatedSnapshot, SessionError, SessionEvent};
impl Outbox {
    pub(in crate::play) fn spell_catalog_initialize(
        &self,
        selected: bool,
        payload: Option<Vec<u8>>,
    ) -> Result<(), SessionError> {
        let mut panel = oteryn_client_ui::spell_catalog::Panel {
            selected,
            ..Default::default()
        };
        if selected {
            let row = GatedSnapshot {
                domain_id: 100,
                snapshot_type: 1,
                revision: 1,
                payload: payload.ok_or(SessionError::MissingDomain(100))?,
            };
            oteryn_client_ui::spell_catalog::protocol::apply(
                &mut panel,
                oteryn_session::spell_catalog::decode_snapshot(&row)?,
            );
        }
        lock(&self.0).spell_catalog = panel;
        Ok(())
    }
}
pub(in crate::play) fn reduce(
    mailbox: &mut crate::play::mailbox::Mailbox,
    events: &[SessionEvent],
) {
    for event in events {
        if let Some(Ok(catalog)) = oteryn_client_runtime::spell_catalog::replacement(event) {
            oteryn_client_ui::spell_catalog::protocol::apply(&mut mailbox.spell_catalog, catalog);
        }
    }
}

/// Uses the production decoder/initializer for shared frontend caller tests only.
#[cfg(all(test, feature = "bevy-shell"))]
pub(crate) fn bevy_catalog_fixture(outbox: &Outbox, payload: Vec<u8>) -> Result<(), SessionError> {
    outbox.spell_catalog_initialize(true, Some(payload))
}
