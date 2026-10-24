//! GUILD-BANK-1: bounded commands 68/69 under capability 55. UUIDs select data only.
pub use crate::charm_wire::CyclopediaWireError as GuildBankWireError;
use crate::charm_wire::{
    WireResult, push_message_field, push_varint_field, read_bytes, read_varint, set_once,
};

pub const CAPABILITY_GUILD_BANK_V1: u32 = 55;
pub const COMMAND_TYPE_GUILD_BANK_INTENT: u32 = 68;
pub const COMMAND_TYPE_GUILD_BANK_QUERY: u32 = 69;
pub const MAX_REQUEST_BYTES: usize = 160;
pub const MAX_REPLY_BYTES: usize = 16_384;
pub const MAX_LEDGER_ENTRIES: usize = 100;
pub const BALANCE_MAX: u64 = 999_999_999_999;
pub const HARD_MAX: u64 = 9_000_000_000_000_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GuildBankAction {
    Deposit(u64),
    Withdraw(u64),
    Claim,
    Disband(String),
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuildBankIntent {
    pub guild: [u8; 16],
    pub action: GuildBankAction,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuildBankQuery {
    /// None lists the current Account/World open claims.
    pub guild: Option<[u8; 16]>,
    /// Exclusive descending ledger cursor. Absent selects the newest page.
    pub before: Option<u64>,
    pub after_claim: Option<[u8; 16]>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuildBankEntry {
    pub ordinal: u64,
    /// 1 deposit, 2 withdraw, 3 disband payout. Symbolic storage kinds have no event IDs.
    pub kind: u8,
    pub amount: u64,
    pub before: u64,
    pub after: u64,
    pub actor: [u8; 16],
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuildBankClaim {
    pub guild: [u8; 16],
    pub amount: u64,
}
/// 1 OK, 2 NOT_ALLOWED, 3 GUILD_DISBANDING, 4 INSUFFICIENT_BALANCE,
/// 5 BALANCE_LIMIT, 6 JUNIOR, 7 NO_CLAIM. Refusals disclose no balances or ledger.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuildBankReply {
    pub code: u8,
    pub bank_balance: u64,
    pub guild_balance: u64,
    pub claim: u64,
    pub entries: Vec<GuildBankEntry>,
    pub next: Option<u64>,
    pub claims: Vec<GuildBankClaim>,
    pub next_claim: Option<[u8; 16]>,
}
fn malformed<T>() -> WireResult<T> {
    Err(GuildBankWireError::Malformed)
}
fn uuid(bytes: &[u8]) -> WireResult<[u8; 16]> {
    let id: [u8; 16] = bytes
        .try_into()
        .map_err(|_| GuildBankWireError::Malformed)?;
    if id[6] >> 4 != 7 || id[8] & 0xc0 != 0x80 {
        return malformed();
    }
    Ok(id)
}
fn bounded(bytes: &[u8], maximum: usize) -> WireResult<()> {
    if bytes.len() > maximum {
        Err(GuildBankWireError::LimitExceeded)
    } else {
        Ok(())
    }
}
pub fn encode_guild_bank_intent(value: &GuildBankIntent) -> WireResult<Vec<u8>> {
    uuid(&value.guild)?;
    let mut out = Vec::new();
    push_message_field(&mut out, 1, &value.guild);
    let (kind, amount, confirmation) = match &value.action {
        GuildBankAction::Deposit(n) => (1, Some(*n), None),
        GuildBankAction::Withdraw(n) => (2, Some(*n), None),
        GuildBankAction::Claim => (3, None, None),
        GuildBankAction::Disband(s) => (4, None, Some(s.as_str())),
    };
    push_varint_field(&mut out, 2, kind);
    if let Some(amount) = amount {
        if !(1..=BALANCE_MAX).contains(&amount) {
            return malformed();
        }
        push_varint_field(&mut out, 3, amount);
    }
    if let Some(s) = confirmation {
        if s.is_empty() || s.chars().count() > 29 || s.contains('\0') {
            return malformed();
        }
        push_message_field(&mut out, 4, s.as_bytes());
    }
    bounded(&out, MAX_REQUEST_BYTES)?;
    Ok(out)
}
pub fn decode_guild_bank_intent(bytes: &[u8]) -> WireResult<GuildBankIntent> {
    bounded(bytes, MAX_REQUEST_BYTES)?;
    let (mut guild, mut kind, mut amount, mut confirmation) = (None, None, None, None);
    let mut c = 0;
    while c < bytes.len() {
        match read_varint(bytes, &mut c)? {
            10 => set_once(&mut guild, uuid(read_bytes(bytes, &mut c)?)?)?,
            16 => set_once(&mut kind, read_varint(bytes, &mut c)?)?,
            24 => set_once(&mut amount, read_varint(bytes, &mut c)?)?,
            34 => set_once(
                &mut confirmation,
                std::str::from_utf8(read_bytes(bytes, &mut c)?)
                    .map_err(|_| GuildBankWireError::Malformed)?
                    .to_owned(),
            )?,
            _ => return malformed(),
        }
    }
    let action = match (kind, amount, confirmation) {
        (Some(1), Some(n), None) if (1..=BALANCE_MAX).contains(&n) => GuildBankAction::Deposit(n),
        (Some(2), Some(n), None) if (1..=BALANCE_MAX).contains(&n) => GuildBankAction::Withdraw(n),
        (Some(3), None, None) => GuildBankAction::Claim,
        (Some(4), None, Some(s))
            if !s.is_empty() && s.chars().count() <= 29 && !s.contains('\0') =>
        {
            GuildBankAction::Disband(s)
        }
        _ => return malformed(),
    };
    Ok(GuildBankIntent {
        guild: guild.ok_or(GuildBankWireError::Malformed)?,
        action,
    })
}
pub fn encode_guild_bank_query(value: &GuildBankQuery) -> WireResult<Vec<u8>> {
    if value.guild.is_none() && value.before.is_some()
        || value.guild.is_some() && value.after_claim.is_some()
    {
        return malformed();
    }
    let mut out = Vec::new();
    if let Some(guild) = value.guild {
        uuid(&guild)?;
        push_message_field(&mut out, 1, &guild);
    }
    if let Some(n) = value.before {
        if n == 0 || n > i64::MAX as u64 {
            return malformed();
        }
        push_varint_field(&mut out, 2, n);
    }
    if let Some(guild) = value.after_claim {
        uuid(&guild)?;
        push_message_field(&mut out, 3, &guild);
    }
    Ok(out)
}
pub fn decode_guild_bank_query(bytes: &[u8]) -> WireResult<GuildBankQuery> {
    bounded(bytes, MAX_REQUEST_BYTES)?;
    let (mut guild, mut before, mut after_claim) = (None, None, None);
    let mut c = 0;
    while c < bytes.len() {
        match read_varint(bytes, &mut c)? {
            10 => set_once(&mut guild, uuid(read_bytes(bytes, &mut c)?)?)?,
            16 => set_once(&mut before, read_varint(bytes, &mut c)?)?,
            26 => set_once(&mut after_claim, uuid(read_bytes(bytes, &mut c)?)?)?,
            _ => return malformed(),
        }
    }
    let query = GuildBankQuery {
        guild,
        before,
        after_claim,
    };
    encode_guild_bank_query(&query)?;
    Ok(query)
}
fn entry_bytes(e: &GuildBankEntry) -> WireResult<Vec<u8>> {
    uuid(&e.actor)?;
    if e.ordinal == 0
        || e.ordinal > i64::MAX as u64
        || e.kind == 0
        || e.kind > 3
        || e.amount == 0
        || e.amount > BALANCE_MAX
        || e.before > BALANCE_MAX
        || e.after > BALANCE_MAX
        || (if e.kind == 1 {
            e.before.checked_add(e.amount)
        } else {
            e.before.checked_sub(e.amount)
        }) != Some(e.after)
    {
        return malformed();
    }
    let mut out = Vec::new();
    for (f, v) in [
        (1, e.ordinal),
        (2, u64::from(e.kind)),
        (3, e.amount),
        (4, e.before),
        (5, e.after),
    ] {
        push_varint_field(&mut out, f, v);
    }
    push_message_field(&mut out, 6, &e.actor);
    Ok(out)
}
fn decode_entry(bytes: &[u8]) -> WireResult<GuildBankEntry> {
    bounded(bytes, 128)?;
    let mut fields = [None; 5];
    let mut actor = None;
    let mut c = 0;
    while c < bytes.len() {
        let key = read_varint(bytes, &mut c)?;
        match key {
            8 | 16 | 24 | 32 | 40 => {
                let i = usize::try_from(key / 8 - 1).map_err(|_| GuildBankWireError::Malformed)?;
                set_once(&mut fields[i], read_varint(bytes, &mut c)?)?;
            }
            50 => set_once(&mut actor, uuid(read_bytes(bytes, &mut c)?)?)?,
            _ => return malformed(),
        }
    }
    let mut f = fields
        .into_iter()
        .map(|v| v.ok_or(GuildBankWireError::Malformed));
    let e = GuildBankEntry {
        ordinal: f.next().ok_or(GuildBankWireError::Malformed)??,
        kind: u8::try_from(f.next().ok_or(GuildBankWireError::Malformed)??)
            .map_err(|_| GuildBankWireError::Malformed)?,
        amount: f.next().ok_or(GuildBankWireError::Malformed)??,
        before: f.next().ok_or(GuildBankWireError::Malformed)??,
        after: f.next().ok_or(GuildBankWireError::Malformed)??,
        actor: actor.ok_or(GuildBankWireError::Malformed)?,
    };
    entry_bytes(&e)?;
    Ok(e)
}
pub fn encode_guild_bank_reply(r: &GuildBankReply) -> WireResult<Vec<u8>> {
    check_claims(r)?;
    if !(1..=7).contains(&r.code)
        || r.bank_balance > HARD_MAX
        || r.guild_balance > BALANCE_MAX
        || r.claim > BALANCE_MAX
        || r.entries.len() > MAX_LEDGER_ENTRIES
        || (r.code != 1
            && (r.bank_balance != 0
                || r.guild_balance != 0
                || r.claim != 0
                || !r.entries.is_empty()
                || r.next.is_some()))
        || r.entries.windows(2).any(|w| w[0].ordinal <= w[1].ordinal)
        || r.next.is_some_and(|n| {
            r.entries.len() != MAX_LEDGER_ENTRIES || r.entries.last().is_none_or(|e| e.ordinal != n)
        })
    {
        return malformed();
    }
    let mut out = Vec::new();
    for (f, v) in [
        (1, u64::from(r.code)),
        (2, r.bank_balance),
        (3, r.guild_balance),
        (4, r.claim),
    ] {
        push_varint_field(&mut out, f, v);
    }
    for e in &r.entries {
        push_message_field(&mut out, 5, &entry_bytes(e)?);
    }
    if let Some(n) = r.next {
        push_varint_field(&mut out, 6, n);
    }
    for claim in &r.claims {
        let mut bytes = Vec::new();
        push_message_field(&mut bytes, 1, &claim.guild);
        push_varint_field(&mut bytes, 2, claim.amount);
        push_message_field(&mut out, 7, &bytes);
    }
    if let Some(guild) = r.next_claim {
        push_message_field(&mut out, 8, &guild);
    }
    bounded(&out, MAX_REPLY_BYTES)?;
    Ok(out)
}
pub fn decode_guild_bank_reply(bytes: &[u8]) -> WireResult<GuildBankReply> {
    bounded(bytes, MAX_REPLY_BYTES)?;
    let mut fields = [None; 4];
    let mut entries = Vec::new();
    let mut next = None;
    let mut claims = Vec::new();
    let mut next_claim = None;
    let mut c = 0;
    while c < bytes.len() {
        let key = read_varint(bytes, &mut c)?;
        match key {
            8 | 16 | 24 | 32 => {
                let i = usize::try_from(key / 8 - 1).map_err(|_| GuildBankWireError::Malformed)?;
                set_once(&mut fields[i], read_varint(bytes, &mut c)?)?;
            }
            42 => {
                if entries.len() >= MAX_LEDGER_ENTRIES {
                    return Err(GuildBankWireError::LimitExceeded);
                }
                entries.push(decode_entry(read_bytes(bytes, &mut c)?)?);
            }
            48 => set_once(&mut next, read_varint(bytes, &mut c)?)?,
            58 => {
                if claims.len() >= MAX_LEDGER_ENTRIES {
                    return Err(GuildBankWireError::LimitExceeded);
                }
                claims.push(decode_claim(read_bytes(bytes, &mut c)?)?);
            }
            66 => set_once(&mut next_claim, uuid(read_bytes(bytes, &mut c)?)?)?,
            _ => return malformed(),
        }
    }
    let [
        Some(code),
        Some(bank_balance),
        Some(guild_balance),
        Some(claim),
    ] = fields
    else {
        return malformed();
    };
    let r = GuildBankReply {
        code: u8::try_from(code).map_err(|_| GuildBankWireError::Malformed)?,
        bank_balance,
        guild_balance,
        claim,
        entries,
        next,
        claims,
        next_claim,
    };
    encode_guild_bank_reply(&r)?;
    Ok(r)
}
fn check_claims(r: &GuildBankReply) -> WireResult<()> {
    if r.claims.len() > MAX_LEDGER_ENTRIES
        || r.claims
            .iter()
            .any(|c| uuid(&c.guild).is_err() || c.amount == 0 || c.amount > BALANCE_MAX)
        || r.claims.windows(2).any(|w| w[0].guild >= w[1].guild)
        || (!r.claims.is_empty() && (r.code != 1 || !r.entries.is_empty() || r.next.is_some()))
        || r.next_claim.is_some_and(|id| {
            r.claims.len() != MAX_LEDGER_ENTRIES || r.claims.last().is_none_or(|c| c.guild != id)
        })
    {
        return malformed();
    }
    Ok(())
}
fn decode_claim(bytes: &[u8]) -> WireResult<GuildBankClaim> {
    bounded(bytes, 32)?;
    let (mut guild, mut amount) = (None, None);
    let mut c = 0;
    while c < bytes.len() {
        match read_varint(bytes, &mut c)? {
            10 => set_once(&mut guild, uuid(read_bytes(bytes, &mut c)?)?)?,
            16 => set_once(&mut amount, read_varint(bytes, &mut c)?)?,
            _ => return malformed(),
        }
    }
    Ok(GuildBankClaim {
        guild: guild.ok_or(GuildBankWireError::Malformed)?,
        amount: amount.ok_or(GuildBankWireError::Malformed)?,
    })
}
#[cfg(test)]
#[path = "guild_bank_tests.rs"]
mod tests;
