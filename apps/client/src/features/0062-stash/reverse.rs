//! Reverse traversal uses the server's sorted pages and their original row handles.
use super::*;
use oteryn_client_runtime::stash::{
    StashCommand,
    stash::{self, StashAction, StashIntent},
};
use oteryn_session::SessionError;

impl Outbox {
    pub(in crate::play) async fn stash_execute(
        &self,
        session: &mut impl Stepper,
        command: &StashCommand,
    ) -> Result<StashResponse, SessionError> {
        let (reverse, last) = {
            let mailbox = lock(&self.0);
            (mailbox.stash.reverse, mailbox.stash.last_server_page)
        };
        if !reverse {
            return session.stash(command).await;
        }
        let (_, filter) = stash::decode_stash_query(&command.payload)
            .map_err(|_| SessionError::SessionUnusable)?;
        let page_command = |page, action| {
            StashCommand::filtered(StashIntent { page, action }, filter.clone())
                .ok_or(SessionError::SessionUnusable)
        };
        let logical = command.intent.page;
        let mut last = last.unwrap_or(0);
        let mut physical = last.saturating_sub(logical);
        let mut response = session
            .stash(&page_command(physical, command.intent.action)?)
            .await?;
        if logical == 0 && command.intent.action == StashAction::Browse {
            // Find the final page in logarithmic reads. Never repeat a stow or withdrawal.
            let mut low = 0;
            let mut high = None;
            for _ in 0..32 {
                if response.reply.outcome != stash::StashOutcome::Opened {
                    break;
                }
                if response.reply.has_more {
                    low = physical.saturating_add(1).min(stash::MAX_STASH_PAGE);
                } else if !response.reply.rows.is_empty() || physical == 0 {
                    last = physical;
                    break;
                } else {
                    high = Some(physical - 1);
                }
                let next = high.map_or_else(
                    || {
                        physical
                            .saturating_mul(2)
                            .max(low)
                            .min(stash::MAX_STASH_PAGE)
                    },
                    |end| low + end.saturating_sub(low) / 2,
                );
                if next == physical {
                    last = physical;
                    break;
                }
                physical = next;
                response = session
                    .stash(&page_command(physical, StashAction::Browse)?)
                    .await?;
            }
            lock(&self.0).stash.last_server_page = Some(last);
        }
        response.reply.rows.reverse();
        response.reply.has_more = physical > 0
            && matches!(
                response.reply.outcome,
                stash::StashOutcome::Opened | stash::StashOutcome::Committed
            );
        response.reply.page = last.saturating_sub(physical);
        Ok(response)
    }
}
