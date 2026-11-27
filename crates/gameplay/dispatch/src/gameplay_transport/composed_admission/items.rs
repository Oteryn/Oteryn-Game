use super::*;
use crate::durability::item_transfer::{CommittedItemTransfer, ItemTransferError};

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn read_committed_item_move(
        &self,
        _actor: ExactActorRef,
        command: connection::UseCommand,
    ) -> Result<Option<CommittedItemTransfer>, ItemTransferError> {
        let Some(fence) = command.item_fence else {
            return Ok(None);
        };
        let command_id = crate::foundation::CommandId::new(command.command_id)
            .map_err(|_| ItemTransferError::InvalidInput)?;
        let command = crate::foundation::CommandRef::new(command.game_session_id, command_id);
        super::AdmissionRetryBudget::for_command(self.root)
            .retry_command_guard(async || {
                match self
                    .root
                    .read_item_transfer_receipt(self.character, fence.character_id, command)
                    .await
                {
                    Err(ItemTransferError::Unavailable(error)) => Err(error),
                    result => Ok(result),
                }
            })
            .await?
    }
}
