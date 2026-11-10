use super::{Error, Request, request::uuid};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Receipt {
    pub operation_id: String,
    pub kind: String,
    pub account_id: String,
    pub counterparty_account_id: Option<String>,
    pub amount: String,
    pub transferable_after: String,
    pub non_transferable_after: String,
    pub reserved_after: String,
    pub occurred_at: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Mutation {
    pub result: String,
    pub receipt: Receipt,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Balance {
    pub result: String,
    pub account_id: String,
    pub transferable: String,
    pub non_transferable: String,
    pub reserved: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    pub entry_id: String,
    pub operation_id: String,
    pub kind: String,
    pub coin_class: String,
    pub counterparty_account_id: Option<String>,
    pub available_delta: String,
    pub reserved_delta: String,
    pub occurred_at: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct History {
    pub result: String,
    pub entries: Vec<Entry>,
    pub next_cursor: Option<String>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Refusal {
    result: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reply {
    Mutation(Mutation),
    Balance(Balance),
    History(History),
    Refused(String),
}
fn canonical<T: DeserializeOwned + Serialize>(bytes: &[u8]) -> Result<T, Error> {
    let value: T = serde_json::from_slice(bytes)?;
    if serde_json::to_vec(&value)? != bytes {
        return Err(Error::Invalid);
    }
    Ok(value)
}
fn number(v: &str, signed: bool) -> bool {
    v.parse::<i64>()
        .is_ok_and(|n| (signed || n >= 0) && n.to_string() == v)
}
fn time(v: &str) -> bool {
    v.len() >= 20
        && v.len() <= 30
        && v.as_bytes().get(10) == Some(&b'T')
        && v.ends_with('Z')
        && v.bytes()
            .all(|c| c.is_ascii_digit() || b"-:TZ.".contains(&c))
}
fn counterparty(v: &Option<String>) -> bool {
    v.as_ref().is_none_or(|v| uuid(v, None))
}
pub fn decode(request: &Request, status: u16, bytes: &[u8]) -> Result<Reply, Error> {
    let max = if matches!(request, Request::History { .. }) {
        65536
    } else {
        2048
    };
    if bytes.is_empty() || bytes.len() > max {
        return Err(Error::Unavailable);
    }
    if status != 200 {
        let r: Refusal = canonical(bytes)?;
        let expected = match r.result.as_str() {
            "insufficient_transferable"
            | "account_inactive"
            | "transfer_blocked"
            | "idempotency_conflict" => 409,
            "unknown_account" | "invalid" => 422,
            "unavailable" => 503,
            _ => return Err(Error::Invalid),
        };
        if status != expected {
            return Err(Error::Invalid);
        }
        return Ok(Reply::Refused(r.result));
    }
    match request {
        Request::Balance(account) => {
            let r: Balance = canonical(bytes)?;
            if r.result != "ok"
                || r.account_id != *account
                || ![&r.transferable, &r.non_transferable, &r.reserved]
                    .iter()
                    .all(|n| number(n, false))
            {
                return Err(Error::Invalid);
            }
            Ok(Reply::Balance(r))
        }
        Request::History { cursor, .. } => {
            let r: History = canonical(bytes)?;
            if r.result != "ok"
                || r.entries.len() > 25
                || r.next_cursor != r.entries.last().map(|e| e.entry_id.clone())
            {
                return Err(Error::Invalid);
            }
            let mut previous = (!cursor.is_empty()).then_some(cursor.as_str());
            for e in &r.entries {
                if !uuid(&e.entry_id, Some(b'7'))
                    || !uuid(&e.operation_id, Some(b'7'))
                    || !counterparty(&e.counterparty_account_id)
                    || !matches!(e.coin_class.as_str(), "transferable" | "non_transferable")
                    || !number(&e.available_delta, true)
                    || !number(&e.reserved_delta, true)
                    || !time(&e.occurred_at)
                    || previous.is_some_and(|p| e.entry_id.as_str() >= p)
                {
                    return Err(Error::Invalid);
                }
                previous = Some(&e.entry_id);
            }
            Ok(Reply::History(r))
        }
        _ => {
            let r: Mutation = canonical(bytes)?;
            let p = &r.receipt;
            if !matches!(r.result.as_str(), "ok" | "replay")
                || !uuid(&p.operation_id, Some(b'7'))
                || !uuid(&p.account_id, None)
                || !counterparty(&p.counterparty_account_id)
                || !matches!(
                    p.kind.as_str(),
                    "transfer" | "reserve" | "settlement" | "release"
                )
                || ![
                    &p.amount,
                    &p.transferable_after,
                    &p.non_transferable_after,
                    &p.reserved_after,
                ]
                .iter()
                .all(|n| number(n, false))
                || !time(&p.occurred_at)
                || !request.matches(p)
            {
                return Err(Error::Invalid);
            }
            Ok(Reply::Mutation(r))
        }
    }
}
