use super::*;

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn read_committed_item_move(
        &self,
        _actor: ExactActorRef,
        command: connection::UseCommand,
    ) -> Result<
        Option<crate::durability::item_transfer::CommittedItemTransfer>,
        crate::durability::item_transfer::ItemTransferError,
    > {
        let Some(fence) = command.item_fence else {
            return Ok(None);
        };
        let command_id = crate::foundation::CommandId::new(command.command_id)
            .map_err(|_| crate::durability::item_transfer::ItemTransferError::InvalidInput)?;
        super::AdmissionRetryBudget::for_command(self.root)
            .retry_logged_holder(
                "item_move_transfer_receipt",
                async || {
                    self.root
                        .read_item_transfer_receipt(
                            self.character,
                            fence.character_id,
                            crate::foundation::CommandRef::new(command.game_session_id, command_id),
                        )
                        .await
                },
                super::item_move::transfer_holder_busy,
            )
            .await
    }
}
