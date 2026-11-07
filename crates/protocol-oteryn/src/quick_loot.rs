//! QUICKLOOT-1 bounded wire messages for manual loot and durable recipient assignment.
use crate::charm_wire::{
    CyclopediaWireError, push_message_field, push_nonzero_varint_field, push_varint_field,
    read_bytes, read_uint32, read_varint, set_once,
};

pub const CAPABILITY_QUICK_LOOT_V1: u32 = 98;
pub const COMMAND_TYPE_QUICK_LOOT: u32 = 114;
pub const MAX_QUICK_LOOT_REQUEST_BYTES: usize = 64;
pub const MAX_QUICK_LOOT_RESULT_BYTES: usize = 32_768;
pub const MAX_QUICK_LOOT_RESULTS: usize = 1_280;
pub type QuickLootWireError = CyclopediaWireError;

type WireResult<T> = Result<T, QuickLootWireError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuickLootAction {
    Click = 1,
    LootAll = 2,
    Assign = 3,
    FilterAdd = 4,
    FilterRemove = 5,
    FilterClear = 6,
    FilterSelect = 7,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuickLootCategory {
    Gold = 1,
    CreatureProduct = 2,
    Equipment = 3,
    Consumable = 4,
    Other = 5,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuickLootRecipient {
    Container = 1,
    GoldPouch = 2,
    MainContainer = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuickLootFilterMode {
    Skip = 1,
    Accept = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuickLootRequest {
    pub action: QuickLootAction,
    pub corpse: Option<[u8; 16]>,
    pub category: Option<QuickLootCategory>,
    pub recipient: Option<QuickLootRecipient>,
    pub recipient_id: Option<[u8; 16]>,
    pub fallback_to_main: bool,
    pub filter_mode: Option<QuickLootFilterMode>,
    pub filter_definition_ref: Option<u32>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuickLootItemResult {
    pub item: [u8; 16],
    pub accepted: bool,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuickLootResult {
    pub items: Vec<QuickLootItemResult>,
}

fn malformed<T>() -> WireResult<T> {
    Err(QuickLootWireError::Malformed)
}
fn enum_value<T>(value: u32, values: &[(u32, T)]) -> WireResult<T>
where
    T: Copy,
{
    values
        .iter()
        .find(|(wire, _)| *wire == value)
        .map(|(_, value)| *value)
        .ok_or(QuickLootWireError::Malformed)
}
fn read_fields(
    mut input: &[u8],
    mut field: impl FnMut(u64, u64, &[u8], &mut usize) -> WireResult<()>,
) -> WireResult<()> {
    let mut cursor = 0;
    while cursor < input.len() {
        let key = read_varint(input, &mut cursor)?;
        field(key >> 3, key & 7, input, &mut cursor)?;
    }
    input = &input[cursor..];
    debug_assert!(input.is_empty());
    Ok(())
}
fn uuid(bytes: &[u8]) -> WireResult<[u8; 16]> {
    bytes.try_into().map_err(|_| QuickLootWireError::Malformed)
}

pub fn encode_quick_loot_request(request: &QuickLootRequest) -> WireResult<Vec<u8>> {
    validate_request(request)?;
    let mut out = Vec::new();
    push_varint_field(&mut out, 1, request.action as u64);
    if let Some(value) = request.corpse {
        push_message_field(&mut out, 2, &value);
    }
    if let Some(value) = request.category {
        push_varint_field(&mut out, 3, value as u64);
    }
    if let Some(value) = request.recipient {
        push_varint_field(&mut out, 4, value as u64);
    }
    if let Some(value) = request.recipient_id {
        push_message_field(&mut out, 5, &value);
    }
    push_nonzero_varint_field(&mut out, 6, u64::from(request.fallback_to_main));
    if let Some(mode) = request.filter_mode {
        push_varint_field(&mut out, 7, mode as u64);
    }
    if let Some(reference) = request.filter_definition_ref {
        push_varint_field(&mut out, 8, u64::from(reference));
    }
    Ok(out)
}
fn validate_request(r: &QuickLootRequest) -> WireResult<()> {
    let filter_action = matches!(
        r.action,
        QuickLootAction::FilterAdd
            | QuickLootAction::FilterRemove
            | QuickLootAction::FilterClear
            | QuickLootAction::FilterSelect
    );
    if !filter_action && (r.filter_mode.is_some() || r.filter_definition_ref.is_some()) {
        return malformed();
    }
    let valid = match r.action {
        QuickLootAction::Click => {
            r.corpse.is_some()
                && r.category.is_none()
                && r.recipient.is_none()
                && r.recipient_id.is_none()
                && !r.fallback_to_main
        }
        QuickLootAction::LootAll => {
            r.corpse.is_none()
                && r.category.is_none()
                && r.recipient.is_none()
                && r.recipient_id.is_none()
                && !r.fallback_to_main
        }
        QuickLootAction::Assign => {
            r.corpse.is_none()
                && r.recipient.is_some()
                && r.recipient_id.is_some()
                    == !matches!(r.recipient, Some(QuickLootRecipient::MainContainer))
        }
        QuickLootAction::FilterAdd
        | QuickLootAction::FilterRemove
        | QuickLootAction::FilterClear
        | QuickLootAction::FilterSelect => {
            r.corpse.is_none()
                && r.category.is_none()
                && r.recipient.is_none()
                && r.recipient_id.is_none()
                && !r.fallback_to_main
                && r.filter_mode.is_some()
                && if matches!(
                    r.action,
                    QuickLootAction::FilterAdd | QuickLootAction::FilterRemove
                ) {
                    r.filter_definition_ref.is_some_and(|r| r > 0)
                } else {
                    r.filter_definition_ref.is_none()
                }
        }
    };
    if valid { Ok(()) } else { malformed() }
}
pub fn decode_quick_loot_request(payload: &[u8]) -> WireResult<QuickLootRequest> {
    if payload.len() > MAX_QUICK_LOOT_REQUEST_BYTES {
        return Err(QuickLootWireError::LimitExceeded);
    }
    let (mut action, mut corpse, mut category, mut recipient, mut recipient_id, mut fallback) =
        (None, None, None, None, None, None);
    let (mut filter_mode, mut filter_definition_ref) = (None, None);
    read_fields(payload, |number, wire, input, cursor| {
        match (number, wire) {
            (1, 0) => set_once(
                &mut action,
                enum_value(
                    read_uint32(input, cursor)?,
                    &[
                        (1, QuickLootAction::Click),
                        (2, QuickLootAction::LootAll),
                        (3, QuickLootAction::Assign),
                        (4, QuickLootAction::FilterAdd),
                        (5, QuickLootAction::FilterRemove),
                        (6, QuickLootAction::FilterClear),
                        (7, QuickLootAction::FilterSelect),
                    ],
                )?,
            ),
            (2, 2) => set_once(&mut corpse, uuid(read_bytes(input, cursor)?)?),
            (3, 0) => set_once(
                &mut category,
                enum_value(
                    read_uint32(input, cursor)?,
                    &[
                        (1, QuickLootCategory::Gold),
                        (2, QuickLootCategory::CreatureProduct),
                        (3, QuickLootCategory::Equipment),
                        (4, QuickLootCategory::Consumable),
                        (5, QuickLootCategory::Other),
                    ],
                )?,
            ),
            (4, 0) => set_once(
                &mut recipient,
                enum_value(
                    read_uint32(input, cursor)?,
                    &[
                        (1, QuickLootRecipient::Container),
                        (2, QuickLootRecipient::GoldPouch),
                        (3, QuickLootRecipient::MainContainer),
                    ],
                )?,
            ),
            (5, 2) => set_once(&mut recipient_id, uuid(read_bytes(input, cursor)?)?),
            (6, 0) => {
                let v = read_uint32(input, cursor)?;
                if v > 1 {
                    return malformed();
                }
                set_once(&mut fallback, v == 1)
            }
            (7, 0) => set_once(
                &mut filter_mode,
                enum_value(
                    read_uint32(input, cursor)?,
                    &[
                        (1, QuickLootFilterMode::Skip),
                        (2, QuickLootFilterMode::Accept),
                    ],
                )?,
            ),
            (8, 0) => set_once(&mut filter_definition_ref, read_uint32(input, cursor)?),
            _ => malformed(),
        }
    })?;
    let result = QuickLootRequest {
        action: action.ok_or(QuickLootWireError::Malformed)?,
        corpse,
        category,
        recipient,
        recipient_id,
        fallback_to_main: fallback.unwrap_or(false),
        filter_mode,
        filter_definition_ref,
    };
    validate_request(&result)?;
    if encode_quick_loot_request(&result)? != payload {
        return malformed();
    }
    Ok(result)
}

pub fn encode_quick_loot_result(result: &QuickLootResult) -> WireResult<Vec<u8>> {
    if result.items.len() > MAX_QUICK_LOOT_RESULTS {
        return Err(QuickLootWireError::LimitExceeded);
    }
    let mut out = Vec::new();
    for item in &result.items {
        let mut row = Vec::new();
        push_message_field(&mut row, 1, &item.item);
        push_nonzero_varint_field(&mut row, 2, u64::from(item.accepted));
        push_message_field(&mut out, 1, &row);
    }
    if out.len() > MAX_QUICK_LOOT_RESULT_BYTES {
        return Err(QuickLootWireError::LimitExceeded);
    }
    Ok(out)
}
pub fn decode_quick_loot_result(payload: &[u8]) -> WireResult<QuickLootResult> {
    if payload.len() > MAX_QUICK_LOOT_RESULT_BYTES {
        return Err(QuickLootWireError::LimitExceeded);
    }
    let mut items = Vec::new();
    read_fields(payload, |number, wire, input, cursor| {
        if (number, wire) != (1, 2) {
            return malformed();
        }
        if items.len() == MAX_QUICK_LOOT_RESULTS {
            return Err(QuickLootWireError::LimitExceeded);
        }
        let row = read_bytes(input, cursor)?;
        let (mut id, mut accepted) = (None, None);
        read_fields(row, |n, w, i, c| match (n, w) {
            (1, 2) => set_once(&mut id, uuid(read_bytes(i, c)?)?),
            (2, 0) => {
                let v = read_uint32(i, c)?;
                if v > 1 {
                    return malformed();
                }
                set_once(&mut accepted, v == 1)
            }
            _ => malformed(),
        })?;
        items.push(QuickLootItemResult {
            item: id.ok_or(QuickLootWireError::Malformed)?,
            accepted: accepted.unwrap_or(false),
        });
        Ok(())
    })?;
    let result = QuickLootResult { items };
    if encode_quick_loot_result(&result)? != payload {
        return malformed();
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn requests_round_trip_and_reject_ambiguous_shapes() -> WireResult<()> {
        let r = QuickLootRequest {
            action: QuickLootAction::Click,
            corpse: Some([7; 16]),
            category: None,
            recipient: None,
            recipient_id: None,
            fallback_to_main: false,
            filter_mode: None,
            filter_definition_ref: None,
        };
        let b = encode_quick_loot_request(&r)?;
        assert_eq!(decode_quick_loot_request(&b), Ok(r));
        let mut bad = r;
        bad.fallback_to_main = true;
        assert!(encode_quick_loot_request(&bad).is_err());
        Ok(())
    }
    #[test]
    fn accepted_and_skipped_round_trip() -> WireResult<()> {
        let r = QuickLootResult {
            items: vec![
                QuickLootItemResult {
                    item: [1; 16],
                    accepted: true,
                },
                QuickLootItemResult {
                    item: [2; 16],
                    accepted: false,
                },
            ],
        };
        let b = encode_quick_loot_result(&r)?;
        assert_eq!(decode_quick_loot_result(&b), Ok(r));
        Ok(())
    }
    #[test]
    fn maximum_bounded_quick_loot_result_fits_the_wire() -> WireResult<()> {
        let result = QuickLootResult {
            items: vec![
                QuickLootItemResult {
                    item: [1; 16],
                    accepted: true
                };
                MAX_QUICK_LOOT_RESULTS
            ],
        };
        let encoded = encode_quick_loot_result(&result)?;
        assert!(
            crate::encode_command_result(1, 1, 1, crate::CommandStatus::Accepted, &encoded).is_ok()
        );
        let registry: serde_json::Value =
            serde_json::from_str(crate::PROTOCOL_OTERYN_V1_REGISTRY_JSON)
                .map_err(|_| QuickLootWireError::Malformed)?;
        let command = registry["command_types"]
            .as_array()
            .and_then(|rows| rows.iter().find(|r| r["id"] == COMMAND_TYPE_QUICK_LOOT))
            .ok_or(QuickLootWireError::Malformed)?;
        assert_eq!(
            command["max_result_payload_bytes"],
            MAX_QUICK_LOOT_RESULT_BYTES
        );
        assert_eq!(decode_quick_loot_result(&encoded), Ok(result));
        Ok(())
    }
    #[test]
    fn wire_limits_reject_before_allocating() {
        assert!(decode_quick_loot_request(&[0; MAX_QUICK_LOOT_REQUEST_BYTES + 1]).is_err());
        assert!(decode_quick_loot_result(&vec![0; MAX_QUICK_LOOT_RESULT_BYTES + 1]).is_err());
        let oversized = QuickLootResult {
            items: vec![
                QuickLootItemResult {
                    item: [1; 16],
                    accepted: true
                };
                MAX_QUICK_LOOT_RESULTS + 1
            ],
        };
        assert!(encode_quick_loot_result(&oversized).is_err());
    }
    #[test]
    fn filter_actions_round_trip_and_reject_missing_definition() -> WireResult<()> {
        for action in [
            QuickLootAction::FilterAdd,
            QuickLootAction::FilterRemove,
            QuickLootAction::FilterClear,
            QuickLootAction::FilterSelect,
        ] {
            let request = QuickLootRequest {
                action,
                corpse: None,
                category: None,
                recipient: None,
                recipient_id: None,
                fallback_to_main: false,
                filter_mode: Some(QuickLootFilterMode::Skip),
                filter_definition_ref: if matches!(
                    action,
                    QuickLootAction::FilterAdd | QuickLootAction::FilterRemove
                ) {
                    Some(7)
                } else {
                    None
                },
            };
            assert_eq!(
                decode_quick_loot_request(&encode_quick_loot_request(&request)?),
                Ok(request)
            );
            assert!(
                encode_quick_loot_request(&QuickLootRequest {
                    filter_mode: None,
                    ..request
                })
                .is_err()
            );
        }
        Ok(())
    }
}
