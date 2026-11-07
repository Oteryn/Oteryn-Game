use crate::play::{mailbox::lock, Outbox, Stepper};
use oteryn_client_runtime::world_interaction::Command;
impl Outbox {
    pub(in crate::play) fn targeting_initialize(&self, enabled: bool) {
        lock(&self.0).world_context.use_with = enabled;
    }

    pub(in crate::play) fn rotation_initialize(&self, enabled: bool) {
        let mut mailbox = lock(&self.0);
        mailbox.world_context.rotate = enabled;
        mailbox.rotation_probe = oteryn_client_ui::world_rotate::InventoryProbe::new(enabled);
    }

    pub(in crate::play) fn world_context_initialize(&self, look: bool, bed: bool) {
        let mut mailbox = lock(&self.0);
        mailbox.world_context.look = look;
        mailbox.world_context.bed = bed;
    }
    pub(in crate::play) async fn world_context_run(
        &self,
        session: &mut impl Stepper,
        command: &Command,
    ) -> Result<(), oteryn_session::SessionError> {
        let result = session.world_context(command).await;
        let mut mailbox = lock(&self.0);
        mailbox.world_context.pending = false;
        match result {
            Ok(text) => {
                mailbox
                    .world_context
                    .set_feedback(text, std::time::Instant::now());
                Ok(())
            }
            Err(error) => {
                mailbox
                    .world_context
                    .set_feedback("Session unavailable".into(), std::time::Instant::now());
                Err(error)
            }
        }
    }
}

macro_rules! session_trait {
    ($($methods:tt)*) => {
        super::feature_0037_depot::protocol::session_trait! {
            $($methods)*
            fn targeting_source_current(&self, source: oteryn_session::ItemHandle) -> bool {
                self.inventory().is_some_and(|inventory| {
                    inventory.main_backpack.iter().chain(&inventory.entries)
                        .any(|item| item.handle == source)
                })
            }
            fn world_context(
                &mut self,
                _command: &oteryn_client_runtime::world_interaction::Command,
            ) -> impl std::future::Future<
                Output = Result<String, oteryn_session::SessionError>,
            > + Send {
                async {
                    Err(oteryn_session::SessionError::CapabilityNotSelected { capability: 28 })
                }
            }
        }
    };
}
pub(in crate::play) use session_trait;

macro_rules! dispatch {
    ($command:expr, $session:ident, $outbox:ident, $held:ident, $publish:ident; $($arms:tt)*) => {
        super::feature_0051_staff::protocol::dispatch! {
            $command, $session, $outbox, $held, $publish;
            Ok(crate::combat_input::PlayCommand::WorldContext(command)) => {
                match $outbox.world_context_run(&mut $session, &command).await {
                    Ok(()) => {
                        $publish(&mut $session, &$outbox, &mut $held);
                        continue;
                    }
                    Err(error) => error,
                }
            }
            $($arms)*
        }
    };
}
pub(in crate::play) use dispatch;

#[cfg(any(windows, test))]
pub(in crate::play) fn inventory_rotation_available(
    mailbox: &crate::play::mailbox::Mailbox,
) -> bool {
    if mailbox.inventory_shutdown
        || !mailbox.world_context.rotate
        || mailbox.world_context.pending
        || mailbox.world_input_blocked()
    {
        return false;
    }
    let (Some(item), Some(map)) = (mailbox.inventory.selected_item(), mailbox.map.as_ref()) else {
        return false;
    };
    mailbox.rotation_probe.available(
        item,
        (
            map.header.content_generation,
            map.header.bundle_digest,
            map.header.reset_epoch,
        ),
    )
}

impl Outbox {
    pub(in crate::play) fn rotation_probe_result(
        &self,
        result: &oteryn_client_runtime::raw_feature::GatedCommandResult,
    ) {
        let mut mailbox = lock(&self.0);
        let item = mailbox.inventory.selected_item();
        let map = mailbox.map.as_ref().map(|map| {
            (
                map.header.content_generation,
                map.header.bundle_digest,
                map.header.reset_epoch,
            )
        });
        oteryn_client_ui::world_rotate::apply_probe_result(
            &mut mailbox.rotation_probe,
            result,
            item,
            map,
        );
    }
}

impl crate::play::PlayLink {
    #[cfg(any(windows, test))]
    pub(in crate::play) fn inventory_rotate_probe_shared(
        shared: &crate::play::mailbox::SharedMailbox,
        commands: &tokio::sync::mpsc::UnboundedSender<crate::combat_input::PlayCommand>,
    ) -> bool {
        let mut mailbox = lock(shared);
        if mailbox.inventory_shutdown || mailbox.world_input_blocked() {
            return false;
        }
        let (Some(item), Some(map)) = (mailbox.inventory.selected_item(), mailbox.map.as_ref())
        else {
            return false;
        };
        let binding = (
            map.header.content_generation,
            map.header.bundle_digest,
            map.header.reset_epoch,
        );
        let Some(command) = oteryn_client_ui::world_rotate::probe_request(
            &mut mailbox.rotation_probe,
            item,
            binding,
        ) else {
            return false;
        };
        if commands
            .send(crate::combat_input::PlayCommand::Feature(command))
            .is_err()
        {
            mailbox.rotation_probe.disable();
            return false;
        }
        true
    }

    #[cfg(any(windows, test))]
    pub(in crate::play) fn inventory_rotate_shared(
        shared: &crate::play::mailbox::SharedMailbox,
        commands: &tokio::sync::mpsc::UnboundedSender<crate::combat_input::PlayCommand>,
    ) -> bool {
        let mut mailbox = lock(shared);
        if !inventory_rotation_available(&mailbox) {
            return false;
        }
        let Some(item) = mailbox.inventory.selected_item() else {
            return false;
        };
        let Some(command) = oteryn_client_ui::world_rotate::request(
            oteryn_client_runtime::world_interaction::Target::Item(item.handle),
            Some(item.item_definition_ref),
            None,
        ) else {
            return false;
        };
        if commands
            .send(crate::combat_input::PlayCommand::WorldContext(command))
            .is_err()
        {
            return false;
        }
        mailbox.world_context.pending = true;
        mailbox.world_context.set_feedback(
            "Rotate: waiting for server".into(),
            std::time::Instant::now(),
        );
        true
    }
}
