//! GUILD0 §9 body-codec preparation. No capability, command or state-domain ID is allocated
//! here. The candidate .proto and this module are deliberately unregistered until the CP slot.
//! These values are untrusted requests/display data, never membership, access or money proof.
//! Candidate GUILDS_V1 Q1 and its `decline` extension are not adopted by this component.

pub use crate::charm_wire::CyclopediaWireError as GuildWireError;
use crate::charm_wire::{
    WireResult, push_message_field, push_varint_field, read_bytes, read_varint, set_once,
};
use crate::world_spatial_entities::{EntityRef, MAX_SNAPSHOT_ENTITIES};

pub const MAX_NAME_BYTES: usize = 29;
pub const MIN_RANKS: usize = 3;
pub const MAX_RANKS: usize = 20;
pub const MAX_QUERY_ENTRIES: usize = 100;
pub const MAX_QUERY_ENTRY_BYTES: usize = 512;
/// A rank entry's level costs2 bytes, its name tag/length at most3. This is a transport
/// projection budget, not a new rank naming grammar or a registered resource-limit row.
pub const MAX_RANK_NAME_BYTES: usize = MAX_QUERY_ENTRY_BYTES - 5;
pub const MAX_TITLE_BYTES: usize = 29 * 4;
pub const MAX_MESSAGE_BYTES: usize = 255 * 4;
pub const MAX_AMOUNT: u64 = 9_000_000_000_000_000;
///20 rank fields (tag+two-byte length+name), revision(tag+uint64), outer oneof framing.
pub const MAX_INTENT_BYTES: usize = MAX_RANKS * (3 + MAX_RANK_NAME_BYTES) + 11 + 3;
pub const MAX_CURSOR_BYTES: usize = 51;
pub const MAX_QUERY_BYTES: usize = 2 + 2 + MAX_NAME_BYTES + 2 + MAX_CURSOR_BYTES;
/// Entity UUID field18 + generation11 + emblem2; repeated message framing2.
pub const MAX_EMBLEM_BYTES: usize = 31;
pub const MAX_EMBLEMS_BYTES: usize = MAX_SNAPSHOT_ENTITIES * (2 + MAX_EMBLEM_BYTES);

/// Existing Game-issued UUIDv7 shape. Neither construction nor decoding proves existence,
/// World binding, current membership, or ownership. Reuses the foundation UUID decoder.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct GuildIdentity([u8; 16]);
impl GuildIdentity {
    pub fn decode(bytes: &[u8]) -> WireResult<Self> {
        crate::decode_uuid_v7(bytes)
            .map(Self)
            .map_err(|_| GuildWireError::Malformed)
    }
    pub const fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }
}

/// FND-02 owns CommandRef. The current session, Account and World never come from this body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GuildIntent {
    Found {
        name: String,
    },
    Invite {
        character: String,
    },
    RevokeInvite {
        character: String,
    },
    Accept {
        guild: GuildIdentity,
    },
    Leave,
    Exclude {
        character: String,
    },
    SetRank {
        character: String,
        level: u8,
        expected_revision: u64,
    },
    EditRanks {
        names: Vec<String>,
        expected_revision: u64,
    },
    SetTitle {
        character: String,
        title: String,
        expected_revision: u64,
    },
    SetMessage {
        text: String,
        expected_revision: u64,
    },
    Resign {
        successor: String,
    },
    Disband {
        confirm: String,
    },
    Deposit {
        amount: u64,
    },
    Withdraw {
        amount: u64,
    },
    ClaimDisbandPayout {
        guild: GuildIdentity,
    },
}

fn size(input: &[u8], maximum: usize) -> WireResult<()> {
    if input.len() > maximum {
        Err(GuildWireError::LimitExceeded)
    } else {
        Ok(())
    }
}
fn text(value: &str, maximum: usize, empty: bool) -> WireResult<()> {
    size(value.as_bytes(), maximum)?;
    if !empty && value.is_empty() {
        return Err(GuildWireError::Malformed);
    }
    Ok(())
}
fn read_text(input: &[u8], cursor: &mut usize, maximum: usize, empty: bool) -> WireResult<String> {
    let bytes = read_bytes(input, cursor)?;
    let value = std::str::from_utf8(bytes).map_err(|_| GuildWireError::Malformed)?;
    text(value, maximum, empty)?;
    Ok(value.to_owned())
}
fn fields<'a>(
    input: &'a [u8],
    mut field: impl FnMut(u64, u64, &'a [u8], &mut usize) -> WireResult<()>,
) -> WireResult<()> {
    let mut cursor = 0;
    while cursor < input.len() {
        let key = read_varint(input, &mut cursor)?;
        field(key >> 3, key & 7, input, &mut cursor)?;
    }
    Ok(())
}
fn required<T>(value: Option<T>) -> WireResult<T> {
    value.ok_or(GuildWireError::Malformed)
}
fn revision(value: u64) -> WireResult<()> {
    if value == 0 {
        Err(GuildWireError::Malformed)
    } else {
        Ok(())
    }
}
fn rank(level: u8) -> WireResult<()> {
    if (1..=MAX_RANKS as u8).contains(&level) {
        Ok(())
    } else {
        Err(GuildWireError::Malformed)
    }
}

fn check_intent(intent: &GuildIntent) -> WireResult<()> {
    match intent {
        GuildIntent::Found { name } => text(name, MAX_NAME_BYTES, false),
        GuildIntent::Invite { character }
        | GuildIntent::RevokeInvite { character }
        | GuildIntent::Exclude { character } => text(character, MAX_NAME_BYTES, false),
        GuildIntent::Accept { .. }
        | GuildIntent::Leave
        | GuildIntent::ClaimDisbandPayout { .. } => Ok(()),
        GuildIntent::SetRank {
            character,
            level,
            expected_revision,
        } => {
            text(character, MAX_NAME_BYTES, false)?;
            rank(*level)?;
            revision(*expected_revision)
        }
        GuildIntent::EditRanks {
            names,
            expected_revision,
        } => {
            if !(MIN_RANKS..=MAX_RANKS).contains(&names.len()) {
                return Err(GuildWireError::LimitExceeded);
            }
            for name in names {
                text(name, MAX_RANK_NAME_BYTES, false)?;
            }
            revision(*expected_revision)
        }
        GuildIntent::SetTitle {
            character,
            title,
            expected_revision,
        } => {
            text(character, MAX_NAME_BYTES, false)?;
            text(title, MAX_TITLE_BYTES, true)?;
            revision(*expected_revision)
        }
        GuildIntent::SetMessage {
            text: value,
            expected_revision,
        } => {
            text(value, MAX_MESSAGE_BYTES, true)?;
            revision(*expected_revision)
        }
        GuildIntent::Resign { successor } => text(successor, MAX_NAME_BYTES, false),
        GuildIntent::Disband { confirm } => text(confirm, MAX_NAME_BYTES, false),
        GuildIntent::Deposit { amount } | GuildIntent::Withdraw { amount } => {
            if *amount == 0 || *amount > MAX_AMOUNT {
                Err(GuildWireError::Malformed)
            } else {
                Ok(())
            }
        }
    }
}

pub fn encode_guild_intent(intent: &GuildIntent) -> WireResult<Vec<u8>> {
    check_intent(intent)?;
    let mut inner = Vec::new();
    let tag = match intent {
        GuildIntent::Found { name } => {
            push_message_field(&mut inner, 1, name.as_bytes());
            1
        }
        GuildIntent::Invite { character } => {
            push_message_field(&mut inner, 1, character.as_bytes());
            2
        }
        GuildIntent::RevokeInvite { character } => {
            push_message_field(&mut inner, 1, character.as_bytes());
            3
        }
        GuildIntent::Accept { guild } => {
            push_message_field(&mut inner, 1, guild.as_bytes());
            4
        }
        GuildIntent::Leave => 5,
        GuildIntent::Exclude { character } => {
            push_message_field(&mut inner, 1, character.as_bytes());
            6
        }
        GuildIntent::SetRank {
            character,
            level,
            expected_revision,
        } => {
            push_message_field(&mut inner, 1, character.as_bytes());
            push_varint_field(&mut inner, 2, u64::from(*level));
            push_varint_field(&mut inner, 3, *expected_revision);
            7
        }
        GuildIntent::EditRanks {
            names,
            expected_revision,
        } => {
            for name in names {
                push_message_field(&mut inner, 1, name.as_bytes());
            }
            push_varint_field(&mut inner, 2, *expected_revision);
            8
        }
        GuildIntent::SetTitle {
            character,
            title,
            expected_revision,
        } => {
            push_message_field(&mut inner, 1, character.as_bytes());
            push_message_field(&mut inner, 2, title.as_bytes());
            push_varint_field(&mut inner, 3, *expected_revision);
            9
        }
        GuildIntent::SetMessage {
            text,
            expected_revision,
        } => {
            push_message_field(&mut inner, 1, text.as_bytes());
            push_varint_field(&mut inner, 2, *expected_revision);
            10
        }
        GuildIntent::Resign { successor } => {
            push_message_field(&mut inner, 1, successor.as_bytes());
            11
        }
        GuildIntent::Disband { confirm } => {
            push_message_field(&mut inner, 1, confirm.as_bytes());
            12
        }
        GuildIntent::Deposit { amount } => {
            push_varint_field(&mut inner, 1, *amount);
            13
        }
        GuildIntent::Withdraw { amount } => {
            push_varint_field(&mut inner, 1, *amount);
            14
        }
        GuildIntent::ClaimDisbandPayout { guild } => {
            push_message_field(&mut inner, 1, guild.as_bytes());
            15
        }
    };
    let mut output = Vec::new();
    push_message_field(&mut output, tag, &inner);
    size(&output, MAX_INTENT_BYTES)?;
    Ok(output)
}

pub fn decode_guild_intent(input: &[u8]) -> WireResult<GuildIntent> {
    size(input, MAX_INTENT_BYTES)?;
    let mut variant = None;
    fields(input, |tag, wire, input, cursor| {
        if wire != 2 || !(1..=15).contains(&tag) {
            return Err(GuildWireError::Malformed);
        }
        set_once(&mut variant, (tag, read_bytes(input, cursor)?))
    })?;
    let (tag, body) = required(variant)?;
    let (mut first, mut second, mut number, mut expected, mut guild) =
        (None, None, None, None, None);
    let mut names = Vec::new();
    fields(body, |field, wire, input, cursor| {
        match (tag, field, wire) {
            (1..=3 | 6 | 7 | 9 | 11 | 12, 1, 2) => {
                set_once(&mut first, read_text(input, cursor, MAX_NAME_BYTES, false)?)
            }
            (4 | 15, 1, 2) => set_once(
                &mut guild,
                GuildIdentity::decode(read_bytes(input, cursor)?)?,
            ),
            (7, 2, 0) | (13 | 14, 1, 0) => set_once(&mut number, read_varint(input, cursor)?),
            (7 | 9, 3, 0) | (8 | 10, 2, 0) => set_once(&mut expected, read_varint(input, cursor)?),
            (8, 1, 2) => {
                if names.len() == MAX_RANKS {
                    return Err(GuildWireError::LimitExceeded);
                }
                names.push(read_text(input, cursor, MAX_RANK_NAME_BYTES, false)?);
                Ok(())
            }
            (9, 2, 2) => set_once(
                &mut second,
                read_text(input, cursor, MAX_TITLE_BYTES, true)?,
            ),
            (10, 1, 2) => set_once(
                &mut second,
                read_text(input, cursor, MAX_MESSAGE_BYTES, true)?,
            ),
            _ => Err(GuildWireError::Malformed),
        }
    })?;
    let result = match tag {
        1 => GuildIntent::Found {
            name: required(first)?,
        },
        2 => GuildIntent::Invite {
            character: required(first)?,
        },
        3 => GuildIntent::RevokeInvite {
            character: required(first)?,
        },
        4 => GuildIntent::Accept {
            guild: required(guild)?,
        },
        5 => GuildIntent::Leave,
        6 => GuildIntent::Exclude {
            character: required(first)?,
        },
        7 => GuildIntent::SetRank {
            character: required(first)?,
            level: u8::try_from(required(number)?).map_err(|_| GuildWireError::Malformed)?,
            expected_revision: required(expected)?,
        },
        8 => GuildIntent::EditRanks {
            names,
            expected_revision: required(expected)?,
        },
        9 => GuildIntent::SetTitle {
            character: required(first)?,
            title: required(second)?,
            expected_revision: required(expected)?,
        },
        10 => GuildIntent::SetMessage {
            text: required(second)?,
            expected_revision: required(expected)?,
        },
        11 => GuildIntent::Resign {
            successor: required(first)?,
        },
        12 => GuildIntent::Disband {
            confirm: required(first)?,
        },
        13 => GuildIntent::Deposit {
            amount: required(number)?,
        },
        14 => GuildIntent::Withdraw {
            amount: required(number)?,
        },
        15 => GuildIntent::ClaimDisbandPayout {
            guild: required(guild)?,
        },
        _ => return Err(GuildWireError::Malformed),
    };
    check_intent(&result)?;
    Ok(result)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuildSection {
    Summary = 1,
    Roster = 2,
    Invitations = 3,
    Activity = 4,
    DisbandClaims = 5,
    Public = 6,
}
impl GuildSection {
    fn decode(value: u64) -> WireResult<Self> {
        match value {
            1 => Ok(Self::Summary),
            2 => Ok(Self::Roster),
            3 => Ok(Self::Invitations),
            4 => Ok(Self::Activity),
            5 => Ok(Self::DisbandClaims),
            6 => Ok(Self::Public),
            _ => Err(GuildWireError::Malformed),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GuildCursorKey {
    Roster {
        level: u8,
        character: [u8; 16],
    },
    /// Accepted invitation key is (GuildId, CharacterId), not an invented row number.
    Invitation {
        character: [u8; 16],
    },
    /// Accepted claim key is GuildId; claims span the current Account and World.
    DisbandClaim {
        guild: GuildIdentity,
    },
    /// Existing event foundation time unit, not database-clock/current-authority evidence.
    Activity {
        occurred_at_unix_ms: u64,
        event: [u8; 16],
    },
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuildCursor {
    /// Present for guild-scoped lists; absent for Account+World disband claims.
    pub guild: Option<GuildIdentity>,
    pub key: GuildCursorKey,
}
impl GuildCursor {
    pub const fn section(&self) -> GuildSection {
        match self.key {
            GuildCursorKey::Roster { .. } => GuildSection::Roster,
            GuildCursorKey::Invitation { .. } => GuildSection::Invitations,
            GuildCursorKey::DisbandClaim { .. } => GuildSection::DisbandClaims,
            GuildCursorKey::Activity { .. } => GuildSection::Activity,
        }
    }
    /// Structural keyset binding only. The query owner must independently resolve and authorize
    /// the selected guild before comparing this untrusted cursor with that resolved identity.
    pub fn check_binding(&self, guild: GuildIdentity, section: GuildSection) -> WireResult<()> {
        if self.guild == Some(guild) && self.section() == section {
            Ok(())
        } else {
            Err(GuildWireError::Malformed)
        }
    }
}
pub fn encode_guild_cursor(cursor: &GuildCursor) -> WireResult<Vec<u8>> {
    let claims = matches!(cursor.key, GuildCursorKey::DisbandClaim { .. });
    if claims == cursor.guild.is_some() {
        return Err(GuildWireError::Malformed);
    }
    let mut output = Vec::new();
    if let Some(guild) = cursor.guild {
        push_message_field(&mut output, 1, guild.as_bytes());
    }
    push_varint_field(&mut output, 2, cursor.section() as u64);
    match &cursor.key {
        GuildCursorKey::Roster { level, character } => {
            rank(*level)?;
            crate::decode_uuid_v7(character).map_err(|_| GuildWireError::Malformed)?;
            let mut key = Vec::new();
            push_varint_field(&mut key, 1, u64::from(*level));
            push_message_field(&mut key, 2, character);
            push_message_field(&mut output, 3, &key);
        }
        GuildCursorKey::Invitation { character } => {
            crate::decode_uuid_v7(character).map_err(|_| GuildWireError::Malformed)?;
            push_message_field(&mut output, 4, character);
        }
        GuildCursorKey::DisbandClaim { guild } => {
            push_message_field(&mut output, 6, guild.as_bytes())
        }
        GuildCursorKey::Activity {
            occurred_at_unix_ms,
            event,
        } => {
            if *occurred_at_unix_ms > i64::MAX as u64 {
                return Err(GuildWireError::Malformed);
            }
            crate::decode_uuid_v7(event).map_err(|_| GuildWireError::Malformed)?;
            let mut key = Vec::new();
            push_varint_field(&mut key, 1, *occurred_at_unix_ms);
            push_message_field(&mut key, 2, event);
            push_message_field(&mut output, 5, &key);
        }
    };
    size(&output, MAX_CURSOR_BYTES)?;
    Ok(output)
}
pub fn decode_guild_cursor(input: &[u8]) -> WireResult<GuildCursor> {
    size(input, MAX_CURSOR_BYTES)?;
    let (mut guild, mut section, mut key) = (None, None, None);
    fields(input, |tag, wire, input, cursor| match (tag, wire) {
        (1, 2) => set_once(
            &mut guild,
            GuildIdentity::decode(read_bytes(input, cursor)?)?,
        ),
        (2, 0) => set_once(
            &mut section,
            GuildSection::decode(read_varint(input, cursor)?)?,
        ),
        (3..=6, 2) => set_once(&mut key, (tag, read_bytes(input, cursor)?)),
        _ => Err(GuildWireError::Malformed),
    })?;
    let section = required(section)?;
    let (tag, body) = required(key)?;
    let key = match (tag, section) {
        (4, GuildSection::Invitations) => GuildCursorKey::Invitation {
            character: crate::decode_uuid_v7(body).map_err(|_| GuildWireError::Malformed)?,
        },
        (6, GuildSection::DisbandClaims) => GuildCursorKey::DisbandClaim {
            guild: GuildIdentity::decode(body)?,
        },
        (3, GuildSection::Roster) | (5, GuildSection::Activity) => {
            let (mut number, mut identity) = (None, None);
            fields(body, |tag, wire, input, cursor| match (tag, wire) {
                (1, 0) => set_once(&mut number, read_varint(input, cursor)?),
                (2, 2) => set_once(
                    &mut identity,
                    crate::decode_uuid_v7(read_bytes(input, cursor)?)
                        .map_err(|_| GuildWireError::Malformed)?,
                ),
                _ => Err(GuildWireError::Malformed),
            })?;
            if section == GuildSection::Roster {
                GuildCursorKey::Roster {
                    level: u8::try_from(required(number)?)
                        .map_err(|_| GuildWireError::Malformed)?,
                    character: required(identity)?,
                }
            } else {
                GuildCursorKey::Activity {
                    occurred_at_unix_ms: required(number)?,
                    event: required(identity)?,
                }
            }
        }
        _ => return Err(GuildWireError::Malformed),
    };
    let result = GuildCursor { guild, key };
    encode_guild_cursor(&result)?;
    Ok(result)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuildQuery {
    pub section: GuildSection,
    pub public_guild: Option<String>,
    pub cursor: Option<GuildCursor>,
}
fn check_query(query: &GuildQuery) -> WireResult<()> {
    if let Some(name) = &query.public_guild {
        text(name, MAX_NAME_BYTES, false)?;
    }
    if (query.section == GuildSection::Public && query.public_guild.is_none())
        || (query.public_guild.is_some()
            && !matches!(query.section, GuildSection::Roster | GuildSection::Public))
    {
        return Err(GuildWireError::Malformed);
    }
    if let Some(cursor) = &query.cursor {
        if cursor.section() != query.section {
            return Err(GuildWireError::Malformed);
        }
        encode_guild_cursor(cursor)?;
    }
    Ok(())
}
pub fn encode_guild_query(query: &GuildQuery) -> WireResult<Vec<u8>> {
    check_query(query)?;
    let mut output = Vec::new();
    push_varint_field(&mut output, 1, query.section as u64);
    if let Some(name) = &query.public_guild {
        push_message_field(&mut output, 2, name.as_bytes());
    }
    if let Some(cursor) = &query.cursor {
        push_message_field(&mut output, 3, &encode_guild_cursor(cursor)?);
    }
    size(&output, MAX_QUERY_BYTES)?;
    Ok(output)
}
pub fn decode_guild_query(input: &[u8]) -> WireResult<GuildQuery> {
    size(input, MAX_QUERY_BYTES)?;
    let (mut section, mut name, mut page) = (None, None, None);
    fields(input, |tag, wire, input, cursor| match (tag, wire) {
        (1, 0) => set_once(
            &mut section,
            GuildSection::decode(read_varint(input, cursor)?)?,
        ),
        (2, 2) => set_once(&mut name, read_text(input, cursor, MAX_NAME_BYTES, false)?),
        (3, 2) => set_once(&mut page, decode_guild_cursor(read_bytes(input, cursor)?)?),
        _ => Err(GuildWireError::Malformed),
    })?;
    let result = GuildQuery {
        section: required(section)?,
        public_guild: name,
        cursor: page,
    };
    check_query(&result)?;
    Ok(result)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuildEmblem {
    None = 1,
    OwnGuild = 2,
    OtherGuild = 3,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuildEmblemEntry {
    pub entity: EntityRef,
    pub emblem: GuildEmblem,
}
/// Only emblem data. The accepted own-member name/rank domain composition remains CP work;
/// candidate Q1 must not disclose another guild's names/ranks through this body.
pub fn encode_guild_emblems(entries: &[GuildEmblemEntry]) -> WireResult<Vec<u8>> {
    if entries.len() > MAX_SNAPSHOT_ENTITIES {
        return Err(GuildWireError::LimitExceeded);
    }
    let mut output = Vec::new();
    let mut previous = None;
    for entry in entries {
        revision(entry.entity.generation)?;
        if previous.is_some_and(|p| p >= entry.entity.identity) {
            return Err(GuildWireError::Malformed);
        }
        previous = Some(entry.entity.identity);
        let mut inner = Vec::new();
        push_message_field(&mut inner, 1, &entry.entity.identity);
        push_varint_field(&mut inner, 2, entry.entity.generation);
        push_varint_field(&mut inner, 3, entry.emblem as u64);
        size(&inner, MAX_EMBLEM_BYTES)?;
        push_message_field(&mut output, 1, &inner);
    }
    size(&output, MAX_EMBLEMS_BYTES)?;
    Ok(output)
}
pub fn decode_guild_emblems(input: &[u8]) -> WireResult<Vec<GuildEmblemEntry>> {
    size(input, MAX_EMBLEMS_BYTES)?;
    let mut entries = Vec::new();
    fields(input, |tag, wire, input, cursor| {
        if (tag, wire) != (1, 2) {
            return Err(GuildWireError::Malformed);
        }
        if entries.len() == MAX_SNAPSHOT_ENTITIES {
            return Err(GuildWireError::LimitExceeded);
        }
        let body = read_bytes(input, cursor)?;
        size(body, MAX_EMBLEM_BYTES)?;
        let (mut identity, mut generation, mut emblem) = (None, None, None);
        fields(body, |tag, wire, input, cursor| match (tag, wire) {
            (1, 2) => set_once(
                &mut identity,
                <[u8; 16]>::try_from(read_bytes(input, cursor)?)
                    .map_err(|_| GuildWireError::Malformed)?,
            ),
            (2, 0) => set_once(&mut generation, read_varint(input, cursor)?),
            (3, 0) => set_once(
                &mut emblem,
                match read_varint(input, cursor)? {
                    1 => GuildEmblem::None,
                    2 => GuildEmblem::OwnGuild,
                    3 => GuildEmblem::OtherGuild,
                    _ => return Err(GuildWireError::Malformed),
                },
            ),
            _ => Err(GuildWireError::Malformed),
        })?;
        entries.push(GuildEmblemEntry {
            entity: EntityRef {
                identity: required(identity)?,
                generation: required(generation)?,
            },
            emblem: required(emblem)?,
        });
        Ok(())
    })?;
    encode_guild_emblems(&entries)?;
    Ok(entries)
}

/// Closed guild-specific results from accepted GUILD0§9. Common foundation status/error
/// results stay in the outer envelope; this does not allocate global error numbers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuildResult {
    Ok = 1,
    NotAllowed = 2,
    NameTaken = 3,
    NameInvalid = 4,
    AlreadyInGuild = 5,
    AccountHasPosition = 6,
    NotPremium = 7,
    GuildFull = 8,
    NotInvited = 9,
    Junior = 10,
    InsufficientFunds = 11,
    BalanceLimit = 12,
    StaleRevision = 13,
    GuildDisbanding = 14,
    GuildDeadlinePassed = 15,
    InvitationLimit = 16,
}
impl GuildResult {
    pub const ALL: [Self; 16] = [
        Self::Ok,
        Self::NotAllowed,
        Self::NameTaken,
        Self::NameInvalid,
        Self::AlreadyInGuild,
        Self::AccountHasPosition,
        Self::NotPremium,
        Self::GuildFull,
        Self::NotInvited,
        Self::Junior,
        Self::InsufficientFunds,
        Self::BalanceLimit,
        Self::StaleRevision,
        Self::GuildDisbanding,
        Self::GuildDeadlinePassed,
        Self::InvitationLimit,
    ];
}
pub fn encode_guild_result(result: GuildResult) -> Vec<u8> {
    let mut output = Vec::new();
    push_varint_field(&mut output, 1, result as u64);
    output
}
pub fn decode_guild_result(input: &[u8]) -> WireResult<GuildResult> {
    size(input, 2)?;
    let mut result = None;
    fields(input, |tag, wire, input, cursor| {
        if (tag, wire) != (1, 0) {
            return Err(GuildWireError::Malformed);
        }
        let value = read_varint(input, cursor)?;
        let value = GuildResult::ALL
            .iter()
            .find(|item| **item as u64 == value)
            .copied()
            .ok_or(GuildWireError::Malformed)?;
        set_once(&mut result, value)
    })?;
    required(result)
}

#[cfg(test)]
#[path = "guild_tests.rs"]
mod tests;
