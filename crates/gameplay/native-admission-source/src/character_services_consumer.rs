//! The existing one-exchange consumer alternates v1/v2 with independent pagination.
use super::*;
use crate::character_services_v2::{
    ListPendingCharacterCommandsV2, PendingCharacterCommandsV2, ReadCharacterCommandIntentV2,
};
impl<C: ReportClock, S: CommandSource, D: Decider> Consumer<C, S, D> {
    /// No new loop or writer when disabled; v1 construction remains unchanged.
    pub fn with_services(mut self, enabled: bool) -> Self {
        self.services_enabled = enabled;
        self.services_wire = enabled;
        self
    }
    pub(super) fn next_wire(&mut self) {
        if self.services_enabled {
            self.services_wire = !self.services_wire;
            std::mem::swap(&mut self.cursor, &mut self.other_cursor);
        }
    }
    pub(super) fn list_operation(&self) -> Operation {
        if self.services_wire {
            Operation::ListPendingCharacterCommandsV2
        } else {
            Operation::ListPendingCharacterCommandsV1
        }
    }
    pub(super) fn read_operation(&self) -> Operation {
        if self.services_wire {
            Operation::ReadCharacterCommandIntentV2
        } else {
            Operation::ReadCharacterCommandIntentV1
        }
    }
    pub(super) fn list_body(&self, after: Option<&Key>) -> Result<String, SourceError> {
        let body = encode_list(&self.source_authority, after)?;
        if !self.services_wire {
            return Ok(body);
        }
        let body = body
            .replacen("\"contract_version\":1", "\"contract_version\":2", 1)
            .replacen(
                "ListPendingCharacterCommandsV1",
                "ListPendingCharacterCommandsV2",
                1,
            );
        ListPendingCharacterCommandsV2::decode(body.as_bytes())?;
        Ok(body)
    }
    pub(super) fn read_body(&self, command: &str, operation: &str) -> Result<String, SourceError> {
        let body = encode_read(&self.source_authority, command, operation)?;
        if !self.services_wire {
            return Ok(body);
        }
        let body = body
            .replacen("\"contract_version\":1", "\"contract_version\":2", 1)
            .replacen(
                "ReadCharacterCommandIntentV1",
                "ReadCharacterCommandIntentV2",
                1,
            );
        ReadCharacterCommandIntentV2::decode(body.as_bytes())?;
        Ok(body)
    }
    pub(super) fn pending_page(
        &self,
        raw: &[u8],
        after: Option<&Key>,
    ) -> Result<Vec<PendingEntry>, SourceError> {
        if !self.services_wire {
            return decode_pending(raw, after);
        }
        PendingCharacterCommandsV2::decode(raw)?;
        let body = std::str::from_utf8(raw)
            .map_err(|_| SourceError::InvalidInput)?
            .replacen("\"contract_version\":2", "\"contract_version\":1", 1);
        decode_pending(body.as_bytes(), after)
    }
}

#[cfg(test)]
#[path = "character_services_consumer_tests.rs"]
mod tests;
