//! Stable client intent bindings. World position and current lamp state are not replay inputs.
use super::receipts;
use super::super::DurabilityError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HirelingLampInput {
    ItemInstance([u8; 16]),
    ItemDefinition(u32),
    LegacySummon { x: u16, y: u16, floor: i8 },
}

impl HirelingLampInput {
    pub fn binding(self) -> [u8; 32] {
        let mut payload = b"lamp-client-input-v1\0".to_vec();
        match self {
            Self::ItemInstance(id) => { payload.push(0); payload.extend(id); }
            Self::ItemDefinition(index) => { payload.push(1); payload.extend(index.to_be_bytes()); }
            Self::LegacySummon { x, y, floor } => {
                payload.push(2); payload.extend(x.to_be_bytes()); payload.extend(y.to_be_bytes()); payload.extend(floor.to_be_bytes());
            }
        }
        receipts::binding(&payload)
    }

    pub(super) fn validate(self, item: [u8; 16], position: (u16, u16, i8)) -> Result<(), DurabilityError> {
        match self {
            Self::ItemInstance(id) if id == item => Ok(()),
            Self::ItemDefinition(index) if index != 0 => Ok(()),
            Self::LegacySummon { x, y, floor } if (x, y, floor) == position => Ok(()),
            _ => Err(DurabilityError::InvalidStoredState),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HirelingReturnInput { Management, LegacyDismiss }

impl HirelingReturnInput {
    pub fn binding(self, hireling: [u8; 16]) -> [u8; 32] {
        match self {
            Self::Management => receipts::return_binding(hireling),
            Self::LegacyDismiss => receipts::binding(b"legacy-dismiss-client-input-v1\0"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn replay_binds_source_kind_and_exact_client_input() {
        let item = HirelingLampInput::ItemInstance([1; 16]);
        assert_ne!(item.binding(), HirelingLampInput::ItemInstance([2; 16]).binding());
        assert_ne!(item.binding(), HirelingLampInput::ItemDefinition(1).binding());
        assert_ne!(HirelingLampInput::ItemDefinition(1).binding(), HirelingLampInput::ItemDefinition(2).binding());
        assert!(item.validate([1; 16], (1, 2, -7)).is_ok());
        assert!(item.validate([1; 16], (4, 5, -7)).is_ok());
        assert!(item.validate([2; 16], (1, 2, -7)).is_err());
        assert!(HirelingLampInput::ItemDefinition(0).validate([1; 16], (1, 2, -7)).is_err());
        assert_ne!(HirelingReturnInput::Management.binding([1; 16]), HirelingReturnInput::LegacyDismiss.binding([1; 16]));
        assert_eq!(HirelingReturnInput::LegacyDismiss.binding([1; 16]), HirelingReturnInput::LegacyDismiss.binding([2; 16]));
    }
}
