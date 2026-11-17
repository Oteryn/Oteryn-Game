use super::{ComposedFreshAdmission, actor_is_current};
use crate::durability::item_mint::TypedDefinitionRef;
use crate::gameplay_transport::connection::{AdmittedSession, UseOutcome};
use crate::gameplay_transport::item_ref_admission::pinned_index;
use oteryn_protocol_oteryn::{server_modal::Dialog, world_object::UseDisposition};
use std::num::NonZeroU32;

fn reading_dialog(text: Option<String>) -> Dialog {
    Dialog {
        id: 1,
        title: "Text".into(),
        message: text.unwrap_or_else(|| "It is empty.".into()),
        choices: Vec::new(),
        buttons: vec![(1, "Ok".into())],
        cancel_button: Some(1),
        image: None,
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn offer_carried_book(
        &self,
        admitted: AdmittedSession,
        instance: [u8; 16],
        definition: NonZeroU32,
    ) -> Option<UseOutcome> {
        let root = self.states.server_messages.connection_root(
            admitted.game_session_id,
            admitted.continuity.connection_generation,
        )?;
        let definition = {
            let runtime = self.runtime.lock().await;
            let index = pinned_index(&runtime, self.active_generation)?;
            let (key, revision) = index.definition_at(definition)?;
            // The generated Botany chest grant stores this book's body on the instance.
            if key != "oteryn:item.tibia.i11699" {
                return None;
            }
            if !actor_is_current(&runtime, admitted) {
                return Some(UseOutcome::rejected());
            }
            TypedDefinitionRef {
                family: "Item".into(),
                production_key: key.into(),
                revision_ref: revision.into(),
            }
        };
        let text = match self
            .root
            .read_carried_book(
                self.character,
                self.holder,
                admitted.item_fence?,
                instance,
                definition,
            )
            .await
        {
            Ok(Some(text)) => text,
            Ok(None) => {
                return Some(UseOutcome {
                    disposition: UseDisposition::StaleState,
                    ..UseOutcome::rejected()
                });
            }
            Err(_) => return Some(UseOutcome::rejected()),
        };
        let runtime = self.runtime.lock().await;
        if !actor_is_current(&runtime, admitted)
            || pinned_index(&runtime, self.active_generation).is_none()
            || !self
                .states
                .player_trades
                .physical(admitted.game_session_id, Some(instance))
            || self.states.server_messages.connection_root(root.0, root.1) != Some(root)
        {
            return Some(UseOutcome::rejected());
        }
        let queued = self
            .states
            .server_modals
            .queue_reading(root, reading_dialog(text));
        Some(UseOutcome {
            disposition: if queued.is_some() {
                UseDisposition::Committed
            } else {
                UseDisposition::Rejected
            },
            ..UseOutcome::rejected()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oteryn_protocol_oteryn::server_modal::{Answer, Reply};

    #[test]
    fn botany_book_text_window_preserves_body_and_has_only_a_close_button()
    -> Result<(), &'static str> {
        let body = "First page\nSecond page";
        let dialog = reading_dialog(Some(body.into()));
        assert_eq!(dialog.message, body);
        assert_eq!(dialog.buttons, vec![(1, "Ok".into())]);
        assert!(dialog.accepts(Answer {
            dialog_id: dialog.id,
            button_id: 1,
            choice_id: None
        }));
        assert_eq!(
            Reply::decode(&Reply::Dialog(dialog.clone()).encode()?)?,
            Reply::Dialog(dialog)
        );
        assert_eq!(reading_dialog(None).message, "It is empty.");
        Ok(())
    }
}
