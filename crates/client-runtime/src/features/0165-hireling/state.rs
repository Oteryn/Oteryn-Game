use super::{CommandStatus, HirelingResponse, hireling};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HirelingRefusal {
    CapabilityNotSelected,
    Malformed,
    Rejected,
    ReconciliationRequired,
}

/// Command 220 has no state domain or admission snapshot. A retained canonical
/// reply can seed replay; `None` means the active state has not been queried.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HirelingState {
    pub selected: bool,
    pub active: Option<bool>,
}

impl HirelingState {
    pub fn from_snapshot(
        selected: &[u32],
        payload: Option<&[u8]>,
    ) -> Result<Self, HirelingRefusal> {
        let mut state = Self {
            selected: selected.contains(&hireling::CAPABILITY_HIRELING_V1),
            active: None,
        };
        if let Some(payload) = payload {
            state.apply(CommandStatus::Accepted, payload)?;
        }
        Ok(state)
    }

    pub fn apply(&mut self, status: CommandStatus, payload: &[u8]) -> Result<(), HirelingRefusal> {
        self.apply_response(&HirelingResponse {
            status,
            active: hireling::decode_reply(payload),
        })
    }

    pub fn apply_response(&mut self, response: &HirelingResponse) -> Result<(), HirelingRefusal> {
        if !self.selected {
            return Err(HirelingRefusal::CapabilityNotSelected);
        }
        match response.status {
            CommandStatus::Accepted | CommandStatus::DuplicateReplay => {
                self.active = Some(response.active.ok_or(HirelingRefusal::Malformed)?);
                Ok(())
            }
            CommandStatus::Rejected => Err(HirelingRefusal::Rejected),
            CommandStatus::DuplicateOutcomeExpired => Err(HirelingRefusal::ReconciliationRequired),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hireling_reply_apply_snapshot_and_refusals() -> Result<(), HirelingRefusal> {
        let selected = [hireling::CAPABILITY_HIRELING_V1];
        let payload = hireling::encode_reply(true);
        let mut state = HirelingState::from_snapshot(&selected, None)?;
        state.apply(CommandStatus::Accepted, &payload)?;
        assert_eq!(state.active, Some(true));
        assert_eq!(
            HirelingState::from_snapshot(&selected, Some(&payload))?,
            state
        );
        for (status, bytes, error) in [
            (
                CommandStatus::Accepted,
                &[2][..],
                HirelingRefusal::Malformed,
            ),
            (
                CommandStatus::Accepted,
                &[1, 0][..],
                HirelingRefusal::Malformed,
            ),
            (CommandStatus::Rejected, &[][..], HirelingRefusal::Rejected),
            (
                CommandStatus::DuplicateOutcomeExpired,
                &[][..],
                HirelingRefusal::ReconciliationRequired,
            ),
        ] {
            let before = state.clone();
            assert_eq!(state.apply(status, bytes), Err(error));
            assert_eq!(state, before);
        }
        assert_eq!(
            HirelingState::from_snapshot(&[], Some(&payload)),
            Err(HirelingRefusal::CapabilityNotSelected)
        );
        state.apply(
            CommandStatus::DuplicateReplay,
            &hireling::encode_reply(false),
        )?;
        assert_eq!(state.active, Some(false));
        Ok(())
    }
}
