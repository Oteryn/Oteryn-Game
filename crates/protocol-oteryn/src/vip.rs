//! VIP list intent, result and state domain typed payloads (VIP-2, CP D1444).
//!
//! Schema: `docs/contracts/protocol-oteryn/v1/vip_v1.proto`, from VIP-0 §7 and §8. Capability 31
//! `VIP_V1` (no requirement), command type 31 `VIP_INTENT` and state domain 21 `VIP` are
//! registered in `PROTOCOL_OTERYN_V1_REGISTRY.json`. Capabilities 29 and 30 are claimed by
//! ADMIT-CAP-1 (CP D1442), so the capability takes the next free number.
//!
//! Decoding is strict, and encoding refuses the same values as a fault before any byte is
//! emitted: zero or unknown enums, the reserved `TRAINING` status, an intent with no or several
//! variants, a zero entry number or handle, a fixed group where a custom one is named, names and
//! texts that are empty (where required), oversized or invalid UTF-8, an icon above
//! [`MAX_VIP_ICON`], a wait outside `1..=`[`MAX_VIP_WAIT_SECONDS`] with `EXHAUSTED` or any wait
//! with another code, list items out of order, repeated or over their resource row, payloads over
//! their byte bound, and unknown or repeated fields all fail closed. Trimming, control characters
//! and the name grammar are the server's rules (VIP-0 §5).

pub use crate::charm_wire::CyclopediaWireError as VipWireError;
use crate::charm_wire::{
    WireResult, push_message_field, push_nonzero_varint_field, push_varint_field, read_bytes,
    read_uint32, read_varint, set_once,
};

/// Registered capability `VIP_V1`: command type 31, state domain 21.
pub const CAPABILITY_VIP_V1: u32 = 31;
/// Registered command type `VIP_INTENT` (capability 31).
pub const COMMAND_TYPE_VIP_INTENT: u32 = 31;
/// Registered state domain `VIP` (capability 31).
pub const STATE_DOMAIN_VIP: u32 = 21;
/// Snapshot type 1: the whole list and consent view ([`VipSnapshot`]).
pub const SNAPSHOT_TYPE_VIP_V1: u32 = 1;
/// Delta type 1: one entry, inserted or replaced.
pub const DELTA_TYPE_VIP_ENTRY_V1: u32 = 1;
/// Delta type 2: one entry removed.
pub const DELTA_TYPE_VIP_ENTRY_REMOVED_V1: u32 = 2;
/// Delta type 3: the custom groups, replaced whole.
pub const DELTA_TYPE_VIP_GROUPS_V1: u32 = 3;
/// Delta type 4: one entry's status.
pub const DELTA_TYPE_VIP_STATUS_V1: u32 = 4;
/// Delta type 5: the consent view, replaced whole.
pub const DELTA_TYPE_VIP_CONSENT_V1: u32 = 5;

/// `VIP0-RL-02`: entries per list (the Premium cap; the free cap 20 is below it).
pub const MAX_VIP_ENTRIES: usize = 100;
/// `VIP0-RL-03`: custom groups per list (with Premium; 0 without).
pub const MAX_VIP_CUSTOM_GROUPS: usize = 5;
/// The first custom group number; 1 to 3 are the fixed groups.
pub const FIRST_VIP_CUSTOM_GROUP: u8 = 4;
/// The last group number.
pub const LAST_VIP_GROUP: u8 = 8;
/// `VIP0-RL-04`: description characters.
pub const MAX_VIP_DESCRIPTION_CHARS: usize = 128;
/// `VIP0-RL-04`: description bytes.
pub const MAX_VIP_DESCRIPTION_BYTES: usize = 512;
/// `VIP0-RL-05`: the largest icon.
pub const MAX_VIP_ICON: u8 = 10;
/// `VIP0-RL-06`: custom group name characters.
pub const MAX_VIP_GROUP_NAME_CHARS: usize = 25;
/// `VIP0-RL-06`: 25 characters of at most 4 bytes.
pub const MAX_VIP_GROUP_NAME_BYTES: usize = 100;
/// The 0022 name grammar: at most 29 ASCII letters and spaces.
pub const MAX_VIP_NAME_BYTES: usize = 29;
/// `VIP0-RL-07`: one token every 3 s, so a wait is at most 3 s.
pub const MAX_VIP_WAIT_SECONDS: u32 = 3;
/// `VIP0-RL-13`: pending invitations per target.
pub const MAX_VIP_INVITATIONS: usize = 20;
/// `VIP0-RL-15`: accepted watchers per target.
pub const MAX_VIP_WATCHERS: usize = 100;

/// An edit: oneof tag 1 + length 2, entry 11, description 515, icon 2, notify 2, groups 3.
pub const MAX_VIP_INTENT_BYTES: usize = 536;
/// Result 1 + 1 and wait 1 + 1.
pub const MAX_VIP_RESULT_BYTES: usize = 4;
/// Entry 11, name 31, description 515, icon 2, notify 2, groups 3, status 2.
pub const MAX_VIP_ENTRY_BYTES: usize = 566;
/// Entry 11.
pub const MAX_VIP_ENTRY_REMOVED_BYTES: usize = 11;
/// Five groups of 1 + 1 + (number 2, name 102).
pub const MAX_VIP_GROUPS_BYTES: usize = 530;
/// Entry 11, status 2, announce 2.
pub const MAX_VIP_STATUS_BYTES: usize = 15;
/// 20 invitations of 1 + 1 + 53, 100 watchers of 1 + 1 + 42, visibility 2.
pub const MAX_VIP_CONSENT_BYTES: usize = 5_502;
/// `VIP0-RL-08`: groups 530, 100 entries of 1 + 2 + 566 and the consent view 1 + 2 + 5,502.
pub const MAX_VIP_SNAPSHOT_BYTES: usize = 62_935;

/// `VipVisibility`: the character's own "hide my status" setting (VIP-0 §5.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VipVisibility {
    Shown = 1,
    Hidden = 2,
}

impl VipVisibility {
    fn from_wire(value: u32) -> WireResult<Self> {
        match value {
            1 => Ok(Self::Shown),
            2 => Ok(Self::Hidden),
            _ => Err(VipWireError::Malformed),
        }
    }
}

/// `VipStatus`. `TRAINING` (4) is reserved and never built.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VipStatus {
    Offline = 1,
    Online = 2,
    Unconfirmed = 3,
}

impl VipStatus {
    fn from_wire(value: u32) -> WireResult<Self> {
        match value {
            1 => Ok(Self::Offline),
            2 => Ok(Self::Online),
            3 => Ok(Self::Unconfirmed),
            _ => Err(VipWireError::Malformed),
        }
    }
}

/// The groups of an entry: bit n-1 for group n (1..=8).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct VipGroupSet(u8);

impl VipGroupSet {
    /// The set of these group numbers; `None` for a number outside 1..=8.
    pub fn of(groups: &[u8]) -> Option<Self> {
        let mut set = Self::default();
        for &group in groups {
            if !(1..=LAST_VIP_GROUP).contains(&group) {
                return None;
            }
            set.0 |= 1 << (group - 1);
        }
        Some(set)
    }

    pub fn contains(self, group: u8) -> bool {
        (1..=LAST_VIP_GROUP).contains(&group) && self.0 & (1 << (group - 1)) != 0
    }

    /// The group numbers in ascending order.
    pub fn groups(self) -> Vec<u8> {
        (1..=LAST_VIP_GROUP)
            .filter(|group| self.contains(*group))
            .collect()
    }

    pub const fn bits(self) -> u8 {
        self.0
    }

    fn from_wire(value: u32) -> WireResult<Self> {
        u8::try_from(value)
            .map(Self)
            .map_err(|_| VipWireError::Malformed)
    }
}

/// `VipIntentV1`: exactly one intent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VipIntent {
    Add {
        name: String,
    },
    Remove {
        entry_no: u64,
    },
    Edit {
        entry_no: u64,
        description: String,
        icon: u8,
        notify: bool,
        groups: VipGroupSet,
    },
    GroupAdd {
        name: String,
    },
    GroupRename {
        group_no: u8,
        name: String,
    },
    GroupRemove {
        group_no: u8,
    },
    Accept {
        invitation: u64,
    },
    Reject {
        invitation: u64,
    },
    Revoke {
        watcher: u64,
    },
    Withdraw {
        entry_no: u64,
    },
    SetStatus(VipVisibility),
}

/// `VipResultCode`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VipResultCode {
    Ok = 1,
    UnknownName = 2,
    AlreadyListed = 3,
    ListFull = 4,
    GroupLimit = 5,
    NameTaken = 6,
    NotFound = 7,
    Exhausted = 8,
    VipUnavailable = 9,
    Rejected = 10,
}

impl VipResultCode {
    const ALL: [Self; 10] = [
        Self::Ok,
        Self::UnknownName,
        Self::AlreadyListed,
        Self::ListFull,
        Self::GroupLimit,
        Self::NameTaken,
        Self::NotFound,
        Self::Exhausted,
        Self::VipUnavailable,
        Self::Rejected,
    ];
}

/// `VipIntentResultV1`: `wait_seconds` is `1..=`[`MAX_VIP_WAIT_SECONDS`] with `Exhausted` and 0
/// otherwise.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VipIntentResult {
    pub code: VipResultCode,
    pub wait_seconds: u32,
}

impl VipIntentResult {
    /// A result without a wait.
    pub const fn of(code: VipResultCode) -> Self {
        Self {
            code,
            wait_seconds: 0,
        }
    }

    /// Whether the command status is `REJECTED`: every code but `OK`.
    pub const fn is_rejected(self) -> bool {
        !matches!(self.code, VipResultCode::Ok)
    }
}

/// `VipGroupV1`: a custom group (4..=8).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VipGroup {
    pub group_no: u8,
    pub name: String,
}

/// `VipEntryV1`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VipEntry {
    pub entry_no: u64,
    pub name: String,
    pub description: String,
    pub icon: u8,
    pub notify: bool,
    pub groups: VipGroupSet,
    pub status: VipStatus,
}

/// `VipStatusV1`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VipStatusUpdate {
    pub entry_no: u64,
    pub status: VipStatus,
    pub announce: bool,
}

/// `VipInvitationV1`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VipInvitation {
    pub handle: u64,
    pub inviter_name: String,
    pub expires_at_ms: u64,
}

/// `VipWatcherV1`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VipWatcher {
    pub handle: u64,
    pub name: String,
}

/// `VipConsentViewV1`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VipConsentView {
    pub invitations: Vec<VipInvitation>,
    pub watchers: Vec<VipWatcher>,
    pub visibility: VipVisibility,
}

impl Default for VipConsentView {
    fn default() -> Self {
        Self {
            invitations: Vec::new(),
            watchers: Vec::new(),
            visibility: VipVisibility::Shown,
        }
    }
}

/// `VipSnapshotV1`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VipSnapshot {
    pub groups: Vec<VipGroup>,
    pub entries: Vec<VipEntry>,
    pub consent: VipConsentView,
}

/// One domain 21 delta.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VipDelta {
    Entry(VipEntry),
    EntryRemoved { entry_no: u64 },
    Groups(Vec<VipGroup>),
    Status(VipStatusUpdate),
    Consent(VipConsentView),
}

impl VipDelta {
    /// The registered delta type of this delta.
    pub const fn delta_type(&self) -> u32 {
        match self {
            Self::Entry(_) => DELTA_TYPE_VIP_ENTRY_V1,
            Self::EntryRemoved { .. } => DELTA_TYPE_VIP_ENTRY_REMOVED_V1,
            Self::Groups(_) => DELTA_TYPE_VIP_GROUPS_V1,
            Self::Status(_) => DELTA_TYPE_VIP_STATUS_V1,
            Self::Consent(_) => DELTA_TYPE_VIP_CONSENT_V1,
        }
    }
}

// ---- text and number rules ----

fn check_text(text: &str, required: bool, max_bytes: usize, max_chars: usize) -> WireResult<()> {
    if required && text.is_empty() {
        return Err(VipWireError::Malformed);
    }
    if text.len() > max_bytes || text.chars().count() > max_chars {
        return Err(VipWireError::LimitExceeded);
    }
    Ok(())
}

fn check_name(name: &str) -> WireResult<()> {
    check_text(name, true, MAX_VIP_NAME_BYTES, MAX_VIP_NAME_BYTES)
}

fn check_description(text: &str) -> WireResult<()> {
    check_text(
        text,
        false,
        MAX_VIP_DESCRIPTION_BYTES,
        MAX_VIP_DESCRIPTION_CHARS,
    )
}

fn check_group_name(name: &str) -> WireResult<()> {
    check_text(
        name,
        true,
        MAX_VIP_GROUP_NAME_BYTES,
        MAX_VIP_GROUP_NAME_CHARS,
    )
}

fn check_nonzero(value: u64) -> WireResult<u64> {
    if value == 0 {
        Err(VipWireError::Malformed)
    } else {
        Ok(value)
    }
}

fn check_custom_group(group_no: u8) -> WireResult<u8> {
    if (FIRST_VIP_CUSTOM_GROUP..=LAST_VIP_GROUP).contains(&group_no) {
        Ok(group_no)
    } else {
        Err(VipWireError::Malformed)
    }
}

fn check_icon(icon: u8) -> WireResult<u8> {
    if icon > MAX_VIP_ICON {
        Err(VipWireError::LimitExceeded)
    } else {
        Ok(icon)
    }
}

fn read_text(
    input: &[u8],
    cursor: &mut usize,
    check: fn(&str) -> WireResult<()>,
) -> WireResult<String> {
    let bytes = read_bytes(input, cursor)?;
    let text = std::str::from_utf8(bytes).map_err(|_| VipWireError::Malformed)?;
    check(text)?;
    Ok(text.to_owned())
}

fn read_group_no(input: &[u8], cursor: &mut usize) -> WireResult<u8> {
    let value = u8::try_from(read_uint32(input, cursor)?).map_err(|_| VipWireError::Malformed)?;
    check_custom_group(value)
}

fn read_bool(input: &[u8], cursor: &mut usize) -> WireResult<bool> {
    match read_varint(input, cursor)? {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(VipWireError::Malformed),
    }
}

/// Reads the fields of a message: `field(number, wire_type, input, cursor)` consumes one value or
/// refuses the key.
fn read_fields(
    input: &[u8],
    mut field: impl FnMut(u64, u64, &[u8], &mut usize) -> WireResult<()>,
) -> WireResult<()> {
    let mut cursor = 0;
    while cursor < input.len() {
        let key = read_varint(input, &mut cursor)?;
        field(key >> 3, key & 0x07, input, &mut cursor)?;
    }
    Ok(())
}

fn push_text(output: &mut Vec<u8>, field: u64, text: &str) {
    if !text.is_empty() {
        push_message_field(output, field, text.as_bytes());
    }
}

fn check_bound(payload: &[u8], maximum: usize) -> WireResult<()> {
    if payload.len() > maximum {
        Err(VipWireError::LimitExceeded)
    } else {
        Ok(())
    }
}

// ---- intent ----

/// Encodes a `VipIntentV1` (the client side).
pub fn encode_vip_intent(intent: &VipIntent) -> WireResult<Vec<u8>> {
    let mut inner = Vec::new();
    let field = match intent {
        VipIntent::Add { name } => {
            check_name(name)?;
            push_text(&mut inner, 1, name);
            1
        }
        VipIntent::GroupAdd { name } => {
            check_group_name(name)?;
            push_text(&mut inner, 1, name);
            4
        }
        VipIntent::Remove { entry_no } | VipIntent::Withdraw { entry_no } => {
            push_varint_field(&mut inner, 1, check_nonzero(*entry_no)?);
            if matches!(intent, VipIntent::Remove { .. }) {
                2
            } else {
                10
            }
        }
        VipIntent::Edit {
            entry_no,
            description,
            icon,
            notify,
            groups,
        } => {
            check_description(description)?;
            push_varint_field(&mut inner, 1, check_nonzero(*entry_no)?);
            push_text(&mut inner, 2, description);
            push_nonzero_varint_field(&mut inner, 3, u64::from(check_icon(*icon)?));
            push_nonzero_varint_field(&mut inner, 4, u64::from(*notify));
            push_nonzero_varint_field(&mut inner, 5, u64::from(groups.bits()));
            3
        }
        VipIntent::GroupRename { group_no, name } => {
            check_group_name(name)?;
            push_varint_field(&mut inner, 1, u64::from(check_custom_group(*group_no)?));
            push_text(&mut inner, 2, name);
            5
        }
        VipIntent::GroupRemove { group_no } => {
            push_varint_field(&mut inner, 1, u64::from(check_custom_group(*group_no)?));
            6
        }
        VipIntent::Accept { invitation: handle }
        | VipIntent::Reject { invitation: handle }
        | VipIntent::Revoke { watcher: handle } => {
            push_varint_field(&mut inner, 1, check_nonzero(*handle)?);
            match intent {
                VipIntent::Accept { .. } => 7,
                VipIntent::Reject { .. } => 8,
                _ => 9,
            }
        }
        VipIntent::SetStatus(visibility) => {
            push_varint_field(&mut inner, 1, *visibility as u64);
            11
        }
    };
    let mut output = Vec::with_capacity(inner.len() + 3);
    push_message_field(&mut output, field, &inner);
    check_bound(&output, MAX_VIP_INTENT_BYTES)?;
    Ok(output)
}

/// Decodes a `VipIntentV1`; the server answers `REJECTED` to any error.
pub fn decode_vip_intent(payload: &[u8]) -> WireResult<VipIntent> {
    check_bound(payload, MAX_VIP_INTENT_BYTES)?;
    let mut intent = None;
    read_fields(payload, |field, wire, input, cursor| {
        if wire != 2 || !(1..=11).contains(&field) {
            return Err(VipWireError::Malformed);
        }
        let inner = read_bytes(input, cursor)?;
        set_once(&mut intent, decode_intent_variant(field, inner)?)
    })?;
    intent.ok_or(VipWireError::Malformed)
}

fn decode_intent_variant(field: u64, inner: &[u8]) -> WireResult<VipIntent> {
    match field {
        1 | 4 => {
            let mut name = None;
            let check = if field == 1 {
                check_name
            } else {
                check_group_name
            };
            read_fields(inner, |number, wire, input, cursor| match (number, wire) {
                (1, 2) => set_once(&mut name, read_text(input, cursor, check)?),
                _ => Err(VipWireError::Malformed),
            })?;
            let name = name.ok_or(VipWireError::Malformed)?;
            Ok(if field == 1 {
                VipIntent::Add { name }
            } else {
                VipIntent::GroupAdd { name }
            })
        }
        2 | 7 | 8 | 9 | 10 => {
            let mut value = None;
            read_fields(inner, |number, wire, input, cursor| match (number, wire) {
                (1, 0) => set_once(&mut value, read_varint(input, cursor)?),
                _ => Err(VipWireError::Malformed),
            })?;
            let value = check_nonzero(value.unwrap_or(0))?;
            Ok(match field {
                2 => VipIntent::Remove { entry_no: value },
                7 => VipIntent::Accept { invitation: value },
                8 => VipIntent::Reject { invitation: value },
                9 => VipIntent::Revoke { watcher: value },
                _ => VipIntent::Withdraw { entry_no: value },
            })
        }
        3 => {
            let (mut entry, mut description, mut icon, mut notify, mut groups) =
                (None, None, None, None, None);
            read_fields(inner, |number, wire, input, cursor| match (number, wire) {
                (1, 0) => set_once(&mut entry, read_varint(input, cursor)?),
                (2, 2) => set_once(
                    &mut description,
                    read_text(input, cursor, check_description)?,
                ),
                (3, 0) => set_once(&mut icon, read_uint32(input, cursor)?),
                (4, 0) => set_once(&mut notify, read_bool(input, cursor)?),
                (5, 0) => set_once(
                    &mut groups,
                    VipGroupSet::from_wire(read_uint32(input, cursor)?)?,
                ),
                _ => Err(VipWireError::Malformed),
            })?;
            let icon = u8::try_from(icon.unwrap_or(0)).map_err(|_| VipWireError::LimitExceeded)?;
            Ok(VipIntent::Edit {
                entry_no: check_nonzero(entry.unwrap_or(0))?,
                description: description.unwrap_or_default(),
                icon: check_icon(icon)?,
                notify: notify.unwrap_or(false),
                groups: groups.unwrap_or_default(),
            })
        }
        5 => {
            let (mut group, mut name) = (None, None);
            read_fields(inner, |number, wire, input, cursor| match (number, wire) {
                (1, 0) => set_once(&mut group, read_group_no(input, cursor)?),
                (2, 2) => set_once(&mut name, read_text(input, cursor, check_group_name)?),
                _ => Err(VipWireError::Malformed),
            })?;
            Ok(VipIntent::GroupRename {
                group_no: group.ok_or(VipWireError::Malformed)?,
                name: name.ok_or(VipWireError::Malformed)?,
            })
        }
        6 => {
            let mut group = None;
            read_fields(inner, |number, wire, input, cursor| match (number, wire) {
                (1, 0) => set_once(&mut group, read_group_no(input, cursor)?),
                _ => Err(VipWireError::Malformed),
            })?;
            Ok(VipIntent::GroupRemove {
                group_no: group.ok_or(VipWireError::Malformed)?,
            })
        }
        _ => {
            let mut visibility = None;
            read_fields(inner, |number, wire, input, cursor| match (number, wire) {
                (1, 0) => set_once(
                    &mut visibility,
                    VipVisibility::from_wire(read_uint32(input, cursor)?)?,
                ),
                _ => Err(VipWireError::Malformed),
            })?;
            Ok(VipIntent::SetStatus(
                visibility.ok_or(VipWireError::Malformed)?,
            ))
        }
    }
}

// ---- result ----

fn check_result(result: VipIntentResult) -> WireResult<()> {
    let waits = result.code == VipResultCode::Exhausted;
    if waits != (result.wait_seconds != 0) || result.wait_seconds > MAX_VIP_WAIT_SECONDS {
        return Err(VipWireError::Malformed);
    }
    Ok(())
}

/// Encodes a `VipIntentResultV1` (the server side).
pub fn encode_vip_intent_result(result: VipIntentResult) -> WireResult<Vec<u8>> {
    check_result(result)?;
    let mut output = Vec::with_capacity(MAX_VIP_RESULT_BYTES);
    push_varint_field(&mut output, 1, result.code as u64);
    push_nonzero_varint_field(&mut output, 2, u64::from(result.wait_seconds));
    Ok(output)
}

/// Decodes a `VipIntentResultV1` (the client side).
pub fn decode_vip_intent_result(payload: &[u8]) -> WireResult<VipIntentResult> {
    check_bound(payload, MAX_VIP_RESULT_BYTES)?;
    let (mut code, mut wait) = (None, None);
    read_fields(payload, |number, wire, input, cursor| {
        match (number, wire) {
            (1, 0) => set_once(&mut code, read_uint32(input, cursor)?),
            (2, 0) => set_once(&mut wait, read_uint32(input, cursor)?),
            _ => Err(VipWireError::Malformed),
        }
    })?;
    let code = code.ok_or(VipWireError::Malformed)?;
    let code = VipResultCode::ALL
        .into_iter()
        .find(|known| *known as u32 == code)
        .ok_or(VipWireError::Malformed)?;
    let result = VipIntentResult {
        code,
        wait_seconds: wait.unwrap_or(0),
    };
    check_result(result)?;
    Ok(result)
}

// ---- domain 21 ----

fn encode_group(group: &VipGroup) -> WireResult<Vec<u8>> {
    check_group_name(&group.name)?;
    let mut output = Vec::new();
    push_varint_field(
        &mut output,
        1,
        u64::from(check_custom_group(group.group_no)?),
    );
    push_text(&mut output, 2, &group.name);
    Ok(output)
}

fn decode_group(input: &[u8]) -> WireResult<VipGroup> {
    let (mut group, mut name) = (None, None);
    read_fields(input, |number, wire, input, cursor| match (number, wire) {
        (1, 0) => set_once(&mut group, read_group_no(input, cursor)?),
        (2, 2) => set_once(&mut name, read_text(input, cursor, check_group_name)?),
        _ => Err(VipWireError::Malformed),
    })?;
    Ok(VipGroup {
        group_no: group.ok_or(VipWireError::Malformed)?,
        name: name.ok_or(VipWireError::Malformed)?,
    })
}

fn check_groups(groups: &[VipGroup]) -> WireResult<()> {
    if groups.len() > MAX_VIP_CUSTOM_GROUPS {
        return Err(VipWireError::LimitExceeded);
    }
    if groups
        .windows(2)
        .any(|pair| pair[0].group_no >= pair[1].group_no)
    {
        return Err(VipWireError::Malformed);
    }
    Ok(())
}

fn encode_groups_into(output: &mut Vec<u8>, groups: &[VipGroup]) -> WireResult<()> {
    check_groups(groups)?;
    for group in groups {
        push_message_field(output, 1, &encode_group(group)?);
    }
    Ok(())
}

fn encode_entry(entry: &VipEntry) -> WireResult<Vec<u8>> {
    check_name(&entry.name)?;
    check_description(&entry.description)?;
    let mut output = Vec::new();
    push_varint_field(&mut output, 1, check_nonzero(entry.entry_no)?);
    push_text(&mut output, 2, &entry.name);
    push_text(&mut output, 3, &entry.description);
    push_nonzero_varint_field(&mut output, 4, u64::from(check_icon(entry.icon)?));
    push_nonzero_varint_field(&mut output, 5, u64::from(entry.notify));
    push_nonzero_varint_field(&mut output, 6, u64::from(entry.groups.bits()));
    push_varint_field(&mut output, 7, entry.status as u64);
    Ok(output)
}

fn decode_entry(input: &[u8]) -> WireResult<VipEntry> {
    let (mut entry, mut name, mut description, mut icon, mut notify, mut groups, mut status) =
        (None, None, None, None, None, None, None);
    read_fields(input, |number, wire, input, cursor| match (number, wire) {
        (1, 0) => set_once(&mut entry, read_varint(input, cursor)?),
        (2, 2) => set_once(&mut name, read_text(input, cursor, check_name)?),
        (3, 2) => set_once(
            &mut description,
            read_text(input, cursor, check_description)?,
        ),
        (4, 0) => set_once(&mut icon, read_uint32(input, cursor)?),
        (5, 0) => set_once(&mut notify, read_bool(input, cursor)?),
        (6, 0) => set_once(
            &mut groups,
            VipGroupSet::from_wire(read_uint32(input, cursor)?)?,
        ),
        (7, 0) => set_once(
            &mut status,
            VipStatus::from_wire(read_uint32(input, cursor)?)?,
        ),
        _ => Err(VipWireError::Malformed),
    })?;
    let icon = u8::try_from(icon.unwrap_or(0)).map_err(|_| VipWireError::LimitExceeded)?;
    Ok(VipEntry {
        entry_no: check_nonzero(entry.unwrap_or(0))?,
        name: name.ok_or(VipWireError::Malformed)?,
        description: description.unwrap_or_default(),
        icon: check_icon(icon)?,
        notify: notify.unwrap_or(false),
        groups: groups.unwrap_or_default(),
        status: status.ok_or(VipWireError::Malformed)?,
    })
}

fn encode_consent(view: &VipConsentView) -> WireResult<Vec<u8>> {
    if view.invitations.len() > MAX_VIP_INVITATIONS || view.watchers.len() > MAX_VIP_WATCHERS {
        return Err(VipWireError::LimitExceeded);
    }
    if view
        .invitations
        .windows(2)
        .any(|pair| pair[0].handle >= pair[1].handle)
        || view
            .watchers
            .windows(2)
            .any(|pair| pair[0].handle >= pair[1].handle)
    {
        return Err(VipWireError::Malformed);
    }
    let mut output = Vec::new();
    for invitation in &view.invitations {
        check_name(&invitation.inviter_name)?;
        let mut inner = Vec::new();
        push_varint_field(&mut inner, 1, check_nonzero(invitation.handle)?);
        push_text(&mut inner, 2, &invitation.inviter_name);
        push_varint_field(&mut inner, 3, check_nonzero(invitation.expires_at_ms)?);
        push_message_field(&mut output, 1, &inner);
    }
    for watcher in &view.watchers {
        check_name(&watcher.name)?;
        let mut inner = Vec::new();
        push_varint_field(&mut inner, 1, check_nonzero(watcher.handle)?);
        push_text(&mut inner, 2, &watcher.name);
        push_message_field(&mut output, 2, &inner);
    }
    push_varint_field(&mut output, 3, view.visibility as u64);
    Ok(output)
}

fn decode_consent(input: &[u8]) -> WireResult<VipConsentView> {
    let (mut invitations, mut watchers, mut visibility) = (Vec::new(), Vec::new(), None);
    read_fields(input, |number, wire, input, cursor| match (number, wire) {
        (1, 2) => {
            if invitations.len() == MAX_VIP_INVITATIONS {
                return Err(VipWireError::LimitExceeded);
            }
            let (mut handle, mut name, mut expires) = (None, None, None);
            read_fields(
                read_bytes(input, cursor)?,
                |number, wire, input, cursor| match (number, wire) {
                    (1, 0) => set_once(&mut handle, read_varint(input, cursor)?),
                    (2, 2) => set_once(&mut name, read_text(input, cursor, check_name)?),
                    (3, 0) => set_once(&mut expires, read_varint(input, cursor)?),
                    _ => Err(VipWireError::Malformed),
                },
            )?;
            invitations.push(VipInvitation {
                handle: check_nonzero(handle.unwrap_or(0))?,
                inviter_name: name.ok_or(VipWireError::Malformed)?,
                expires_at_ms: check_nonzero(expires.unwrap_or(0))?,
            });
            Ok(())
        }
        (2, 2) => {
            if watchers.len() == MAX_VIP_WATCHERS {
                return Err(VipWireError::LimitExceeded);
            }
            let (mut handle, mut name) = (None, None);
            read_fields(
                read_bytes(input, cursor)?,
                |number, wire, input, cursor| match (number, wire) {
                    (1, 0) => set_once(&mut handle, read_varint(input, cursor)?),
                    (2, 2) => set_once(&mut name, read_text(input, cursor, check_name)?),
                    _ => Err(VipWireError::Malformed),
                },
            )?;
            watchers.push(VipWatcher {
                handle: check_nonzero(handle.unwrap_or(0))?,
                name: name.ok_or(VipWireError::Malformed)?,
            });
            Ok(())
        }
        (3, 0) => set_once(
            &mut visibility,
            VipVisibility::from_wire(read_uint32(input, cursor)?)?,
        ),
        _ => Err(VipWireError::Malformed),
    })?;
    let view = VipConsentView {
        invitations,
        watchers,
        visibility: visibility.ok_or(VipWireError::Malformed)?,
    };
    // The same ordering rule as the encoder.
    encode_consent(&view)?;
    Ok(view)
}

/// Encodes a `VipSnapshotV1` (snapshot type 1 of domain 21).
pub fn encode_vip_snapshot(snapshot: &VipSnapshot) -> WireResult<Vec<u8>> {
    if snapshot.entries.len() > MAX_VIP_ENTRIES {
        return Err(VipWireError::LimitExceeded);
    }
    if snapshot
        .entries
        .windows(2)
        .any(|pair| pair[0].entry_no >= pair[1].entry_no)
    {
        return Err(VipWireError::Malformed);
    }
    let mut output = Vec::new();
    encode_groups_into(&mut output, &snapshot.groups)?;
    for entry in &snapshot.entries {
        push_message_field(&mut output, 2, &encode_entry(entry)?);
    }
    push_message_field(&mut output, 3, &encode_consent(&snapshot.consent)?);
    check_bound(&output, MAX_VIP_SNAPSHOT_BYTES)?;
    Ok(output)
}

/// Decodes a `VipSnapshotV1` (the client side).
pub fn decode_vip_snapshot(payload: &[u8]) -> WireResult<VipSnapshot> {
    check_bound(payload, MAX_VIP_SNAPSHOT_BYTES)?;
    let (mut groups, mut entries, mut consent) = (Vec::new(), Vec::new(), None);
    read_fields(payload, |number, wire, input, cursor| {
        match (number, wire) {
            (1, 2) => {
                if groups.len() == MAX_VIP_CUSTOM_GROUPS {
                    return Err(VipWireError::LimitExceeded);
                }
                groups.push(decode_group(read_bytes(input, cursor)?)?);
                Ok(())
            }
            (2, 2) => {
                if entries.len() == MAX_VIP_ENTRIES {
                    return Err(VipWireError::LimitExceeded);
                }
                entries.push(decode_entry(read_bytes(input, cursor)?)?);
                Ok(())
            }
            (3, 2) => set_once(&mut consent, decode_consent(read_bytes(input, cursor)?)?),
            _ => Err(VipWireError::Malformed),
        }
    })?;
    let snapshot = VipSnapshot {
        groups,
        entries,
        consent: consent.ok_or(VipWireError::Malformed)?,
    };
    // The same ordering rules as the encoder.
    encode_vip_snapshot(&snapshot)?;
    Ok(snapshot)
}

/// Encodes one domain 21 delta payload; its type is [`VipDelta::delta_type`].
pub fn encode_vip_delta(delta: &VipDelta) -> WireResult<Vec<u8>> {
    let (output, maximum) = match delta {
        VipDelta::Entry(entry) => (encode_entry(entry)?, MAX_VIP_ENTRY_BYTES),
        VipDelta::EntryRemoved { entry_no } => {
            let mut output = Vec::new();
            push_varint_field(&mut output, 1, check_nonzero(*entry_no)?);
            (output, MAX_VIP_ENTRY_REMOVED_BYTES)
        }
        VipDelta::Groups(groups) => {
            let mut output = Vec::new();
            encode_groups_into(&mut output, groups)?;
            (output, MAX_VIP_GROUPS_BYTES)
        }
        VipDelta::Status(update) => {
            let mut output = Vec::new();
            push_varint_field(&mut output, 1, check_nonzero(update.entry_no)?);
            push_varint_field(&mut output, 2, update.status as u64);
            push_nonzero_varint_field(&mut output, 3, u64::from(update.announce));
            (output, MAX_VIP_STATUS_BYTES)
        }
        VipDelta::Consent(view) => (encode_consent(view)?, MAX_VIP_CONSENT_BYTES),
    };
    check_bound(&output, maximum)?;
    Ok(output)
}

/// Decodes one domain 21 delta payload of `delta_type` (the client side).
pub fn decode_vip_delta(delta_type: u32, payload: &[u8]) -> WireResult<VipDelta> {
    match delta_type {
        DELTA_TYPE_VIP_ENTRY_V1 => {
            check_bound(payload, MAX_VIP_ENTRY_BYTES)?;
            decode_entry(payload).map(VipDelta::Entry)
        }
        DELTA_TYPE_VIP_ENTRY_REMOVED_V1 => {
            check_bound(payload, MAX_VIP_ENTRY_REMOVED_BYTES)?;
            let mut entry = None;
            read_fields(payload, |number, wire, input, cursor| {
                match (number, wire) {
                    (1, 0) => set_once(&mut entry, read_varint(input, cursor)?),
                    _ => Err(VipWireError::Malformed),
                }
            })?;
            Ok(VipDelta::EntryRemoved {
                entry_no: check_nonzero(entry.unwrap_or(0))?,
            })
        }
        DELTA_TYPE_VIP_GROUPS_V1 => {
            check_bound(payload, MAX_VIP_GROUPS_BYTES)?;
            let mut groups = Vec::new();
            read_fields(payload, |number, wire, input, cursor| {
                match (number, wire) {
                    (1, 2) => {
                        if groups.len() == MAX_VIP_CUSTOM_GROUPS {
                            return Err(VipWireError::LimitExceeded);
                        }
                        groups.push(decode_group(read_bytes(input, cursor)?)?);
                        Ok(())
                    }
                    _ => Err(VipWireError::Malformed),
                }
            })?;
            check_groups(&groups)?;
            Ok(VipDelta::Groups(groups))
        }
        DELTA_TYPE_VIP_STATUS_V1 => {
            check_bound(payload, MAX_VIP_STATUS_BYTES)?;
            let (mut entry, mut status, mut announce) = (None, None, None);
            read_fields(payload, |number, wire, input, cursor| {
                match (number, wire) {
                    (1, 0) => set_once(&mut entry, read_varint(input, cursor)?),
                    (2, 0) => set_once(
                        &mut status,
                        VipStatus::from_wire(read_uint32(input, cursor)?)?,
                    ),
                    (3, 0) => set_once(&mut announce, read_bool(input, cursor)?),
                    _ => Err(VipWireError::Malformed),
                }
            })?;
            Ok(VipDelta::Status(VipStatusUpdate {
                entry_no: check_nonzero(entry.unwrap_or(0))?,
                status: status.ok_or(VipWireError::Malformed)?,
                announce: announce.unwrap_or(false),
            }))
        }
        DELTA_TYPE_VIP_CONSENT_V1 => {
            check_bound(payload, MAX_VIP_CONSENT_BYTES)?;
            decode_consent(payload).map(VipDelta::Consent)
        }
        _ => Err(VipWireError::Malformed),
    }
}

#[cfg(test)]
#[path = "vip_tests.rs"]
mod tests;
