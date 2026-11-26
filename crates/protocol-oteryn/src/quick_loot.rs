//! QUICKLOOT-1 bounded wire messages for manual loot and durable recipient assignment.
use crate::charm_wire::{
    CyclopediaWireError, push_message_field, push_nonzero_varint_field, push_varint_field,
    read_bytes, read_uint32, read_varint, set_once,
};
use crate::item_view::ItemHandle;

pub const CAPABILITY_QUICK_LOOT_V1: u32 = 98;
pub const COMMAND_TYPE_QUICK_LOOT: u32 = 114;
pub const CAPABILITY_QUICK_LOOT_DESTINATIONS_V1: u32 = 607;
pub const COMMAND_TYPE_QUICK_LOOT_DESTINATIONS: u32 = 6376;
pub const COMMAND_TYPE_QUICK_LOOT_DISTRIBUTE: u32 = 6377;
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
    UnassignLoot = 8,
    ReviewDestinations = 9,
    AssignObtain = 10,
    UnassignObtain = 11,
    AssignAllLoot = 12,
    AssignAllObtain = 13,
    Distribute = 14,
    AssignLoot = 15,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuickLootCategory {
    Gold = 1,
    CreatureProduct = 2,
    Equipment = 3,
    Consumable = 4,
    Other = 5,
    Armor = 6,
    Necklace = 7,
    Boots = 8,
    Containers = 9,
    Decoration = 10,
    Food = 11,
    Helmet = 12,
    Legs = 13,
    Potion = 14,
    Ring = 15,
    Rune = 16,
    Shield = 17,
    Tools = 18,
    Valuables = 19,
    Ammunition = 20,
    Axe = 21,
    Club = 22,
    DistanceWeapon = 23,
    Sword = 24,
    Wand = 25,
    Quiver = 26,
    FistWeapon = 27,
}
impl QuickLootCategory {
    pub const ALL: [Self; 27] = [
        Self::Gold,
        Self::CreatureProduct,
        Self::Equipment,
        Self::Consumable,
        Self::Other,
        Self::Armor,
        Self::Necklace,
        Self::Boots,
        Self::Containers,
        Self::Decoration,
        Self::Food,
        Self::Helmet,
        Self::Legs,
        Self::Potion,
        Self::Ring,
        Self::Rune,
        Self::Shield,
        Self::Tools,
        Self::Valuables,
        Self::Ammunition,
        Self::Axe,
        Self::Club,
        Self::DistanceWeapon,
        Self::Sword,
        Self::Wand,
        Self::Quiver,
        Self::FistWeapon,
    ];

    pub const fn from_wire(value: u32) -> Option<Self> {
        match value.checked_sub(1) {
            Some(index) if index < Self::ALL.len() as u32 => Some(Self::ALL[index as usize]),
            _ => None,
        }
    }
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
    pub selected_item: Option<ItemHandle>,
    pub review_filters: bool,
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

/// Command 114's result when the request selects a single item by handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuickLootClickOutcome {
    Moved = 1,
    Stale = 2,
    Rejected = 3,
}

impl QuickLootClickOutcome {
    pub const fn encode(self) -> [u8; 2] {
        [0x10, self as u8]
    }

    pub fn decode(payload: &[u8]) -> Result<Self, QuickLootWireError> {
        match payload {
            [0x10, 1] => Ok(Self::Moved),
            [0x10, 2] => Ok(Self::Stale),
            [0x10, 3] => Ok(Self::Rejected),
            _ => malformed(),
        }
    }
}

impl QuickLootAction {
    pub const fn command_type(self) -> u32 {
        match self {
            Self::Click
            | Self::LootAll
            | Self::Assign
            | Self::FilterAdd
            | Self::FilterRemove
            | Self::FilterClear
            | Self::FilterSelect => COMMAND_TYPE_QUICK_LOOT,
            Self::UnassignLoot
            | Self::ReviewDestinations
            | Self::AssignObtain
            | Self::UnassignObtain
            | Self::AssignAllLoot
            | Self::AssignAllObtain
            | Self::AssignLoot => COMMAND_TYPE_QUICK_LOOT_DESTINATIONS,
            Self::Distribute => COMMAND_TYPE_QUICK_LOOT_DISTRIBUTE,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuickLootDestination {
    pub category: Option<QuickLootCategory>,
    pub recipient: QuickLootRecipient,
    pub recipient_id: Option<[u8; 16]>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuickLootDestinations {
    pub loot: Vec<QuickLootDestination>,
    pub obtain: Vec<QuickLootDestination>,
    pub fallback_to_main: bool,
    pub filters: Option<QuickLootFilters>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuickLootFilters {
    pub mode: QuickLootFilterMode,
    pub skipped: Vec<u32>,
    pub accepted: Vec<u32>,
}

pub const MAX_QUICK_LOOT_FILTER_ITEMS: usize = 2048;
const MAX_DESTINATIONS_PER_KIND: usize = 1 + QuickLootCategory::ALL.len();
pub const MAX_QUICK_LOOT_DESTINATIONS_BYTES: usize =
    3 + 2 * MAX_DESTINATIONS_PER_KIND * 18 + 2 + 2 * (3 + 4 * MAX_QUICK_LOOT_FILTER_ITEMS);
const _: () = assert!(MAX_QUICK_LOOT_DESTINATIONS_BYTES <= crate::MAX_COMMAND_RESULT_PAYLOAD_BYTES);

// Each section is sorted by category, with the default row first when present.
pub fn encode_quick_loot_destinations(value: &QuickLootDestinations) -> WireResult<Vec<u8>> {
    let mut bytes = vec![u8::from(value.fallback_to_main)];
    for rows in [&value.loot, &value.obtain] {
        if rows.len() > MAX_DESTINATIONS_PER_KIND {
            return malformed();
        }
        bytes.push(rows.len() as u8);
        let mut previous = None;
        for row in rows {
            let category = row.category.map_or(0, |c| c as u8);
            if previous.is_some_and(|p| p >= category)
                || row.recipient_id.is_some()
                    == (row.recipient == QuickLootRecipient::MainContainer)
            {
                return malformed();
            }
            previous = Some(category);
            bytes.extend([category, row.recipient as u8]);
            if let Some(id) = row.recipient_id {
                bytes.extend(id);
            }
        }
    }
    if let Some(filters) = &value.filters {
        push_varint_field(&mut bytes, 1, filters.mode as u64);
        for (tag, refs) in [(2, &filters.skipped), (3, &filters.accepted)] {
            if refs.len() > MAX_QUICK_LOOT_FILTER_ITEMS
                || refs.first() == Some(&0)
                || refs.windows(2).any(|pair| pair[0] >= pair[1])
            {
                return malformed();
            }
            if !refs.is_empty() {
                let packed: Vec<_> = refs
                    .iter()
                    .flat_map(|reference| reference.to_le_bytes())
                    .collect();
                push_message_field(&mut bytes, tag, &packed);
            }
        }
    }
    Ok(bytes)
}

fn read_filter_references(bytes: &[u8]) -> WireResult<Vec<u32>> {
    let (refs, tail) = bytes.as_chunks::<4>();
    if !tail.is_empty() || refs.len() > MAX_QUICK_LOOT_FILTER_ITEMS {
        return malformed();
    }
    Ok(refs.iter().copied().map(u32::from_le_bytes).collect())
}

pub fn decode_quick_loot_destinations(mut bytes: &[u8]) -> WireResult<QuickLootDestinations> {
    if bytes.len() > MAX_QUICK_LOOT_DESTINATIONS_BYTES {
        return Err(QuickLootWireError::LimitExceeded);
    }
    let original = bytes;
    let (fallback, rest) = bytes.split_first().ok_or(QuickLootWireError::Malformed)?;
    bytes = rest;
    let mut by_kind = [Vec::new(), Vec::new()];
    for rows in &mut by_kind {
        let (count, rest) = bytes.split_first().ok_or(QuickLootWireError::Malformed)?;
        if usize::from(*count) > MAX_DESTINATIONS_PER_KIND {
            return malformed();
        }
        bytes = rest;
        for _ in 0..*count {
            let ([category, kind], rest) = bytes
                .split_first_chunk::<2>()
                .ok_or(QuickLootWireError::Malformed)?;
            let category = if *category == 0 {
                None
            } else {
                Some(
                    QuickLootCategory::from_wire(u32::from(*category))
                        .ok_or(QuickLootWireError::Malformed)?,
                )
            };
            let (recipient, recipient_id) = match kind {
                3 => {
                    bytes = rest;
                    (QuickLootRecipient::MainContainer, None)
                }
                1 | 2 => {
                    let (id, rest) = rest
                        .split_first_chunk::<16>()
                        .ok_or(QuickLootWireError::Malformed)?;
                    bytes = rest;
                    (
                        if *kind == 1 {
                            QuickLootRecipient::Container
                        } else {
                            QuickLootRecipient::GoldPouch
                        },
                        Some(*id),
                    )
                }
                _ => return malformed(),
            };
            rows.push(QuickLootDestination {
                category,
                recipient,
                recipient_id,
            });
        }
    }
    let mut mode = None;
    let mut skipped = None;
    let mut accepted = None;
    read_fields(bytes, |number, wire, input, cursor| match (number, wire) {
        (1, 0) => set_once(&mut mode, decode_filter_mode(read_uint32(input, cursor)?)?),
        (2, 2) => set_once(
            &mut skipped,
            read_filter_references(read_bytes(input, cursor)?)?,
        ),
        (3, 2) => set_once(
            &mut accepted,
            read_filter_references(read_bytes(input, cursor)?)?,
        ),
        _ => malformed(),
    })?;
    let filters = mode.map(|mode| QuickLootFilters {
        mode,
        skipped: skipped.unwrap_or_default(),
        accepted: accepted.unwrap_or_default(),
    });
    let [loot, obtain] = by_kind;
    let result = QuickLootDestinations {
        loot,
        obtain,
        fallback_to_main: *fallback == 1,
        filters,
    };
    if encode_quick_loot_destinations(&result)? != original {
        return malformed();
    }
    Ok(result)
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

pub fn quick_loot_filter_review_request() -> QuickLootRequest {
    QuickLootRequest {
        action: QuickLootAction::ReviewDestinations,
        corpse: None,
        selected_item: None,
        review_filters: true,
        category: None,
        recipient: None,
        recipient_id: None,
        fallback_to_main: false,
        filter_mode: None,
        filter_definition_ref: None,
    }
}

pub fn encode_quick_loot_request(request: &QuickLootRequest) -> WireResult<Vec<u8>> {
    validate_request(request, false)?;
    Ok(encode_request(request, None))
}

/// Encode a native item handle for an assignment or distribution. Leave the
/// corresponding UUID field absent. The server resolves the handle on entry.
pub fn encode_quick_loot_view_request(
    request: &QuickLootRequest,
    handle: ItemHandle,
) -> WireResult<Vec<u8>> {
    if request.action.command_type() == COMMAND_TYPE_QUICK_LOOT
        || request.corpse.is_some()
        || request.recipient_id.is_some()
    {
        return malformed();
    }
    validate_request(request, true)?;
    Ok(encode_request(request, Some(handle.get().to_le_bytes())))
}

fn encode_request(r: &QuickLootRequest, handle: Option<[u8; 8]>) -> Vec<u8> {
    let request = r;
    let mut out = Vec::new();
    push_varint_field(&mut out, 1, request.action as u64);
    if let Some(value) = handle.filter(|_| request.action == QuickLootAction::Distribute) {
        push_message_field(&mut out, 2, &value);
    } else if let Some(value) = request.corpse {
        push_message_field(&mut out, 2, &value);
    }
    if let Some(value) = request.category {
        push_varint_field(&mut out, 3, value as u64);
    }
    if let Some(value) = request.recipient {
        push_varint_field(&mut out, 4, value as u64);
    }
    if let Some(value) = handle.filter(|_| request.action != QuickLootAction::Distribute) {
        push_message_field(&mut out, 5, &value);
    } else if let Some(value) = request.recipient_id {
        push_message_field(&mut out, 5, &value);
    }
    push_nonzero_varint_field(&mut out, 6, u64::from(request.fallback_to_main));
    if let Some(mode) = request.filter_mode {
        push_varint_field(&mut out, 7, mode as u64);
    }
    if let Some(reference) = request.filter_definition_ref {
        push_varint_field(&mut out, 8, u64::from(reference));
    }
    if let Some(item) = request.selected_item {
        push_message_field(&mut out, 9, &item.get().to_le_bytes());
    }
    push_nonzero_varint_field(&mut out, 10, u64::from(request.review_filters));
    out
}

fn validate_request(r: &QuickLootRequest, has_handle: bool) -> WireResult<()> {
    let has_source = r.corpse.is_some() || (has_handle && r.action == QuickLootAction::Distribute);
    let has_recipient =
        r.recipient_id.is_some() || (has_handle && r.action != QuickLootAction::Distribute);
    let filter_action = matches!(
        r.action,
        QuickLootAction::FilterAdd
            | QuickLootAction::FilterRemove
            | QuickLootAction::FilterClear
            | QuickLootAction::FilterSelect
    );
    if r.action == QuickLootAction::Assign && r.category.is_some_and(|c| c as u8 > 5) {
        return malformed();
    }
    if !filter_action && (r.filter_mode.is_some() || r.filter_definition_ref.is_some()) {
        return malformed();
    }
    if r.action != QuickLootAction::Click && r.selected_item.is_some() {
        return malformed();
    }
    if r.review_filters && r.action != QuickLootAction::ReviewDestinations {
        return malformed();
    }
    let valid = match r.action {
        QuickLootAction::Click => {
            (has_source || r.selected_item.is_some())
                && r.category.is_none()
                && r.recipient.is_none()
                && !has_recipient
                && !r.fallback_to_main
        }
        QuickLootAction::LootAll => {
            !has_source
                && r.category.is_none()
                && r.recipient.is_none()
                && !has_recipient
                && !r.fallback_to_main
        }
        QuickLootAction::Assign
        | QuickLootAction::AssignLoot
        | QuickLootAction::AssignObtain
        | QuickLootAction::AssignAllLoot
        | QuickLootAction::AssignAllObtain => {
            !has_source
                && (!matches!(
                    r.action,
                    QuickLootAction::AssignAllLoot | QuickLootAction::AssignAllObtain
                ) || r.category.is_none())
                && (!matches!(
                    r.action,
                    QuickLootAction::AssignObtain | QuickLootAction::AssignAllObtain
                ) || !r.fallback_to_main)
                && r.recipient.is_some()
                && has_recipient == !matches!(r.recipient, Some(QuickLootRecipient::MainContainer))
        }
        QuickLootAction::UnassignLoot
        | QuickLootAction::UnassignObtain
        | QuickLootAction::ReviewDestinations
        | QuickLootAction::Distribute => {
            has_source == (r.action == QuickLootAction::Distribute)
                && (!matches!(
                    r.action,
                    QuickLootAction::ReviewDestinations | QuickLootAction::Distribute
                ) || r.category.is_none())
                && r.recipient.is_none()
                && !has_recipient
                && !r.fallback_to_main
        }
        QuickLootAction::FilterAdd
        | QuickLootAction::FilterRemove
        | QuickLootAction::FilterClear
        | QuickLootAction::FilterSelect => {
            !has_source
                && r.category.is_none()
                && r.recipient.is_none()
                && !has_recipient
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
fn decode_filter_mode(value: u32) -> WireResult<QuickLootFilterMode> {
    enum_value(
        value,
        &[
            (1, QuickLootFilterMode::Skip),
            (2, QuickLootFilterMode::Accept),
        ],
    )
}

fn decode_item_handle(bytes: &[u8]) -> WireResult<ItemHandle> {
    let bytes = bytes
        .try_into()
        .map_err(|_| QuickLootWireError::Malformed)?;
    ItemHandle::new(u64::from_le_bytes(bytes)).ok_or(QuickLootWireError::Malformed)
}

pub fn decode_quick_loot_request(payload: &[u8]) -> WireResult<QuickLootRequest> {
    decode_quick_loot_view_request(payload, |_| None)
}

/// Resolve a reference only through the admitted session's current handle table.
pub fn decode_quick_loot_view_request(
    payload: &[u8],
    resolve: impl Fn(ItemHandle) -> Option<[u8; 16]>,
) -> WireResult<QuickLootRequest> {
    if payload.len() > MAX_QUICK_LOOT_REQUEST_BYTES {
        return Err(QuickLootWireError::LimitExceeded);
    }
    let (mut action, mut corpse, mut category, mut recipient, mut recipient_id, mut fallback) =
        (None, None, None, None, None, None);
    let (mut filter_mode, mut filter_definition_ref) = (None, None);
    let mut selected_item = None;
    let mut review_filters = None;
    let mut raw_handle = None;
    let mut reference = |bytes: &[u8]| -> WireResult<[u8; 16]> {
        if let Ok(handle_bytes) = <[u8; 8]>::try_from(bytes) {
            let handle = decode_item_handle(&handle_bytes)?;
            set_once(&mut raw_handle, handle_bytes)?;
            resolve(handle).ok_or(QuickLootWireError::Malformed)
        } else {
            uuid(bytes)
        }
    };
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
                        (8, QuickLootAction::UnassignLoot),
                        (9, QuickLootAction::ReviewDestinations),
                        (10, QuickLootAction::AssignObtain),
                        (11, QuickLootAction::UnassignObtain),
                        (12, QuickLootAction::AssignAllLoot),
                        (13, QuickLootAction::AssignAllObtain),
                        (14, QuickLootAction::Distribute),
                        (15, QuickLootAction::AssignLoot),
                    ],
                )?,
            ),
            (2, 2) => set_once(&mut corpse, reference(read_bytes(input, cursor)?)?),
            (3, 0) => set_once(
                &mut category,
                QuickLootCategory::from_wire(read_uint32(input, cursor)?)
                    .ok_or(QuickLootWireError::Malformed)?,
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
            (5, 2) => set_once(&mut recipient_id, reference(read_bytes(input, cursor)?)?),
            (6, 0) => {
                let v = read_uint32(input, cursor)?;
                if v > 1 {
                    return malformed();
                }
                set_once(&mut fallback, v == 1)
            }
            (7, 0) => set_once(
                &mut filter_mode,
                decode_filter_mode(read_uint32(input, cursor)?)?,
            ),
            (8, 0) => set_once(&mut filter_definition_ref, read_uint32(input, cursor)?),
            (9, 2) => set_once(
                &mut selected_item,
                decode_item_handle(read_bytes(input, cursor)?)?,
            ),
            (10, 0) if read_uint32(input, cursor)? == 1 => set_once(&mut review_filters, true),
            _ => malformed(),
        }
    })?;
    let result = QuickLootRequest {
        action: action.ok_or(QuickLootWireError::Malformed)?,
        corpse,
        selected_item,
        review_filters: review_filters.unwrap_or_default(),
        category,
        recipient,
        recipient_id,
        fallback_to_main: fallback.unwrap_or(false),
        filter_mode,
        filter_definition_ref,
    };
    validate_request(&result, false)?;
    if raw_handle.is_some() && result.action.command_type() == COMMAND_TYPE_QUICK_LOOT {
        return malformed();
    }
    if encode_request(&result, raw_handle) != payload {
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
    fn empty_destinations() -> QuickLootDestinations {
        QuickLootDestinations {
            fallback_to_main: true,
            loot: vec![QuickLootDestination {
                category: None,
                recipient: QuickLootRecipient::MainContainer,
                recipient_id: None,
            }],
            obtain: Vec::new(),
            filters: None,
        }
    }

    #[test]
    fn legacy_quick_loot_destinations_round_trip() -> WireResult<()> {
        let short = [1, 1, 0, 3, 0];
        let value = empty_destinations();
        assert_eq!(encode_quick_loot_destinations(&value)?, short);
        assert_eq!(decode_quick_loot_destinations(&short)?, value);
        let request = QuickLootRequest {
            review_filters: false,
            ..quick_loot_filter_review_request()
        };
        assert_eq!(encode_quick_loot_request(&request)?, [8, 9]);
        assert_eq!(decode_quick_loot_request(&[8, 9])?, request);
        Ok(())
    }

    #[test]
    fn quick_loot_destinations_with_filters_round_trip() -> WireResult<()> {
        let value = QuickLootDestinations {
            filters: Some(QuickLootFilters {
                mode: QuickLootFilterMode::Accept,
                skipped: vec![256],
                accepted: vec![3, 5],
            }),
            ..empty_destinations()
        };
        let encoded = [
            1, 1, 0, 3, 0, 8, 2, 18, 4, 0, 1, 0, 0, 26, 8, 3, 0, 0, 0, 5, 0, 0, 0,
        ];
        assert_eq!(encode_quick_loot_destinations(&value)?, encoded);
        assert_eq!(decode_quick_loot_destinations(&encoded)?, value);
        assert!(decode_quick_loot_destinations(&encoded[..encoded.len() - 1]).is_err());
        let missing_mode = [&encoded[..5], &encoded[7..]].concat();
        assert!(decode_quick_loot_destinations(&missing_mode).is_err());
        let request = quick_loot_filter_review_request();
        assert_eq!(encode_quick_loot_request(&request)?, [8, 9, 80, 1]);
        assert_eq!(decode_quick_loot_request(&[8, 9, 80, 1])?, request);
        Ok(())
    }

    #[test]
    fn maximum_quick_loot_filter_review_fits_registered_result_limit() -> WireResult<()> {
        let rows: Vec<_> = std::iter::once(None)
            .chain(QuickLootCategory::ALL.map(Some))
            .map(|category| QuickLootDestination {
                category,
                recipient: QuickLootRecipient::GoldPouch,
                recipient_id: Some([1; 16]),
            })
            .collect();
        let refs: Vec<_> = (1..=MAX_QUICK_LOOT_FILTER_ITEMS as u32).collect();
        let value = QuickLootDestinations {
            fallback_to_main: true,
            loot: rows.clone(),
            obtain: rows,
            filters: Some(QuickLootFilters {
                mode: QuickLootFilterMode::Skip,
                skipped: refs.clone(),
                accepted: refs,
            }),
        };
        let bytes = encode_quick_loot_destinations(&value)?;
        let registry: serde_json::Value =
            serde_json::from_str(crate::PROTOCOL_OTERYN_V1_REGISTRY_JSON)
                .map_err(|_| QuickLootWireError::Malformed)?;
        let command = registry["command_types"]
            .as_array()
            .and_then(|rows| {
                rows.iter()
                    .find(|r| r["id"] == COMMAND_TYPE_QUICK_LOOT_DESTINATIONS)
            })
            .ok_or(QuickLootWireError::Malformed)?;
        assert_eq!(command["max_result_payload_bytes"], bytes.len());
        assert_eq!(bytes.len(), MAX_QUICK_LOOT_DESTINATIONS_BYTES);
        assert_eq!(decode_quick_loot_destinations(&bytes)?, value);
        Ok(())
    }

    #[test]
    fn corpse_only_click_round_trips_with_legacy_bytes() -> WireResult<()> {
        let r = QuickLootRequest {
            action: QuickLootAction::Click,
            corpse: Some([7; 16]),
            selected_item: None,
            review_filters: false,
            category: None,
            recipient: None,
            recipient_id: None,
            fallback_to_main: false,
            filter_mode: None,
            filter_definition_ref: None,
        };
        let legacy = [&[0x08, 1, 0x12, 16][..], &[7; 16]].concat();
        assert_eq!(encode_quick_loot_request(&r)?, legacy);
        assert_eq!(decode_quick_loot_request(&legacy), Ok(r));
        let mut bad = r;
        bad.fallback_to_main = true;
        assert!(encode_quick_loot_request(&bad).is_err());
        Ok(())
    }
    #[test]
    fn selected_item_click_round_trips_and_rejects_invalid_shapes() -> WireResult<()> {
        let fields = [
            &[0x08, 1, 0x12, 16][..],
            &[7; 16],
            &[0x4a, 8],
            &8_u64.to_le_bytes(),
        ];
        let bytes = fields.concat();
        let request = decode_quick_loot_request(&bytes)?;
        assert_eq!(request.action, QuickLootAction::Click);
        assert_eq!(request.corpse, Some([7; 16]));
        assert_eq!(request.selected_item, ItemHandle::new(8));
        assert_eq!(encode_quick_loot_request(&request)?, bytes);

        let mut duplicated = bytes.clone();
        duplicated.extend_from_slice(&fields[2..].concat());
        let no_corpse = [&[0x08, 1][..], fields[2], fields[3]].concat();
        let handle_only = QuickLootRequest {
            corpse: None,
            ..request
        };
        assert_eq!(decode_quick_loot_request(&no_corpse), Ok(handle_only));
        assert_eq!(encode_quick_loot_request(&handle_only)?, no_corpse);
        let loot_all = [&[0x08, 2][..], fields[2], fields[3]].concat();
        let legacy_uuid = [fields[0], fields[1], &[0x4a, 16], &[8; 16]].concat();
        let zero_handle = [fields[0], fields[1], fields[2], &[0; 8]].concat();
        for malformed in [
            duplicated.as_slice(),
            &loot_all,
            &legacy_uuid,
            &zero_handle,
            &bytes[..bytes.len() - 1],
        ] {
            assert_eq!(
                decode_quick_loot_request(malformed),
                Err(QuickLootWireError::Malformed)
            );
        }
        assert_eq!(
            encode_quick_loot_request(&QuickLootRequest {
                action: QuickLootAction::LootAll,
                corpse: None,
                ..request
            }),
            Err(QuickLootWireError::Malformed)
        );
        Ok(())
    }
    #[test]
    fn selected_item_outcomes_round_trip_without_item_ids() -> WireResult<()> {
        for outcome in [
            QuickLootClickOutcome::Moved,
            QuickLootClickOutcome::Stale,
            QuickLootClickOutcome::Rejected,
        ] {
            assert_eq!(QuickLootClickOutcome::decode(&outcome.encode())?, outcome);
        }
        assert_eq!(QuickLootClickOutcome::decode(&[0x10, 0]), malformed());
        assert_eq!(
            QuickLootClickOutcome::decode(&[0x10, 1, 0x10, 2]),
            malformed()
        );
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
                selected_item: None,
                review_filters: false,
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
