//! Crystal 201189f protocolgame.cpp:7060-7138: outcome, tiers, bonus and newest-first
//! history. Oteryn's existing receipt JSON replaces the donor's binary packets.
use oteryn_client_runtime::cui3::item_forge::{MAX_HISTORY, Operation};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Receipt {
    pub operation: Operation,
    pub succeeded: bool,
    pub applied: bool,
    pub reason: Option<String>,
    pub bonus: u8,
    pub resources: Option<Resources>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resources {
    pub dust: u64,
    pub dust_limit: u64,
    pub first_tier: Option<u8>,
    pub second_tier: Option<u8>,
    pub gold_spent: u64,
    pub cores_spent: u64,
    pub slivers_spent: u64,
    pub slivers_created: u64,
    pub cores_created: u64,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reply {
    CurrentDust { dust: u64, limit: u64 },
    Result(Receipt),
    History(Vec<(u64, Receipt)>),
}
fn operation(s: &str) -> Option<Operation> {
    Some(match s {
        "history" => Operation::QueryHistory,
        "fusion" => Operation::Fusion,
        "convergence_fusion" => Operation::ConvergenceFusion,
        "transfer" => Operation::Transfer,
        "convergence_transfer" => Operation::ConvergenceTransfer,
        "dust_to_slivers" => Operation::DustToSlivers,
        "slivers_to_cores" => Operation::SliversToCores,
        "dust_limit" => Operation::DustLimit,
        _ => return None,
    })
}
fn tier(v: &Value) -> Option<Option<u8>> {
    if v.is_null() {
        Some(None)
    } else {
        Some(Some(u8::try_from(v.as_u64()?).ok().filter(|v| *v <= 10)?))
    }
}
fn receipt(v: &Value) -> Option<Receipt> {
    let op = operation(v.get("operation")?.as_str()?)?;
    let succeeded = v.get("succeeded")?.as_bool()?;
    let applied = if op == Operation::DustLimit {
        succeeded
    } else {
        v.get("applied")?.as_bool()?
    };
    let reason = match v.get("reason") {
        None | Some(Value::Null) => None,
        Some(v) => Some(v.as_str().filter(|s| s.len() <= 128)?.to_owned()),
    };
    let bonus = match v.get("bonus") {
        None if op == Operation::DustLimit => 0,
        Some(v) => u8::try_from(v.as_u64()?).ok().filter(|v| *v <= 8)?,
        _ => return None,
    };
    let r = v.get("result")?;
    let resources = if r.is_null() {
        None
    } else {
        let dust = r.get("dust")?.as_u64()?;
        let dust_limit = r.get("dust_limit")?.as_u64()?;
        if dust > dust_limit {
            return None;
        }
        let (
            first_tier,
            second_tier,
            gold_spent,
            cores_spent,
            slivers_spent,
            slivers_created,
            cores_created,
        ) = if op == Operation::DustLimit {
            (None, None, 0, 0, 0, 0, 0)
        } else {
            (
                tier(r.get("first_tier")?)?,
                tier(r.get("second_tier")?)?,
                r.get("gold_spent")?.as_u64()?,
                r.get("cores_spent")?.as_u64()?,
                r.get("slivers_spent")?.as_u64()?,
                r.get("slivers_created")?.as_u64()?,
                r.get("cores_created")?.as_u64()?,
            )
        };
        Some(Resources {
            dust,
            dust_limit,
            first_tier,
            second_tier,
            gold_spent,
            cores_spent,
            slivers_spent,
            slivers_created,
            cores_created,
        })
    };
    Some(Receipt {
        operation: op,
        succeeded,
        applied,
        reason,
        bonus,
        resources,
    })
}
pub fn decode(bytes: &[u8]) -> Option<Reply> {
    // Existing capability replies are bounded; history_limit on command 181 is at most 50.
    if bytes.len() > 32768 {
        return None;
    }
    let v: Value = serde_json::from_slice(bytes).ok()?;
    if let (Some(dust), Some(limit)) = (
        v.get("current_dust").and_then(Value::as_u64),
        v.get("dust_limit").and_then(Value::as_u64),
    ) {
        return (dust <= limit).then_some(Reply::CurrentDust { dust, limit });
    }
    if let Some(rows) = v.as_array() {
        if rows.len() > usize::from(MAX_HISTORY) {
            return None;
        }
        let mut history = Vec::with_capacity(rows.len());
        let mut previous = u64::MAX;
        for row in rows {
            let ordinal = row.get("ordinal")?.as_u64()?;
            if ordinal == 0 || ordinal >= previous {
                return None;
            }
            previous = ordinal;
            history.push((ordinal, receipt(row.get("receipt")?)?));
        }
        Some(Reply::History(history))
    } else {
        Some(Reply::Result(receipt(&v)?))
    }
}
pub fn render(receipt: &Receipt) -> String {
    let mut s = format!(
        "{:?}: {}",
        receipt.operation,
        if receipt.succeeded {
            "succeeded"
        } else {
            "failed"
        }
    );
    s.push_str(if receipt.applied {
        tr!("; attempt applied")
    } else {
        tr!("; no changes applied")
    });
    if let Some(reason) = &receipt.reason {
        s.push_str(&format!(" ({reason})"));
    }
    s.push_str(&lfmt!("\nBonus: {}", receipt.bonus));
    if let Some(r) = &receipt.resources {
        s.push_str(&lfmt!("\nTiers: {:?} / {:?}\nDust: {} / {}\nSpent: {} gold, {} cores, {} slivers\nCreated: {} slivers, {} cores",r.first_tier,r.second_tier,r.dust,r.dust_limit,r.gold_spent,r.cores_spent,r.slivers_spent,r.slivers_created,r.cores_created));
    }
    s
}
