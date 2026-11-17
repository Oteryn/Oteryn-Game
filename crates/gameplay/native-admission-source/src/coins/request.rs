use super::Error;
use serde::Serialize;
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Request {
    Transfer {
        operation: String,
        account: String,
        recipient: String,
        amount: u64,
    },
    Reserve {
        operation: String,
        account: String,
        amount: u64,
        price: u64,
    },
    Settle {
        operation: String,
        reservation: String,
        buyer: String,
        amount: u64,
    },
    Release {
        operation: String,
        account: String,
        reservation: String,
    },
    Operation(String),
    Balance(String),
    History {
        account: String,
        cursor: String,
    },
}
pub(super) fn uuid(value: &str, version: Option<u8>) -> bool {
    value != "00000000-0000-0000-0000-000000000000"
        && crate::runtime_status::canonical_uuid(value, version)
}
fn positive(value: u64) -> bool {
    value > 0 && value <= i64::MAX as u64
}
#[derive(Serialize)]
struct Transfer<'a> {
    operation_id: &'a str,
    sender_account_id: &'a str,
    recipient_account_id: &'a str,
    amount: String,
}
#[derive(Serialize)]
struct Reserve<'a> {
    operation_id: &'a str,
    seller_account_id: &'a str,
    amount: String,
    gold_price: String,
}
#[derive(Serialize)]
struct Settle<'a> {
    operation_id: &'a str,
    reservation_id: &'a str,
    buyer_account_id: &'a str,
    amount: String,
}
#[derive(Serialize)]
struct Release<'a> {
    operation_id: &'a str,
    seller_account_id: &'a str,
    reservation_id: &'a str,
}
impl Request {
    pub fn wire(&self) -> Result<(&'static str, String, String), Error> {
        let (method, path, body) = match self {
            Self::Transfer {
                operation,
                account,
                recipient,
                amount,
            } if uuid(operation, Some(b'7'))
                && uuid(account, None)
                && uuid(recipient, None)
                && account != recipient
                && positive(*amount) =>
            {
                (
                    "POST",
                    "transfers".into(),
                    serde_json::to_string(&Transfer {
                        operation_id: operation,
                        sender_account_id: account,
                        recipient_account_id: recipient,
                        amount: amount.to_string(),
                    })?,
                )
            }
            Self::Reserve {
                operation,
                account,
                amount,
                price,
            } if uuid(operation, Some(b'7'))
                && uuid(account, None)
                && positive(*amount)
                && positive(*price) =>
            {
                (
                    "POST",
                    "market/offers".into(),
                    serde_json::to_string(&Reserve {
                        operation_id: operation,
                        seller_account_id: account,
                        amount: amount.to_string(),
                        gold_price: price.to_string(),
                    })?,
                )
            }
            Self::Settle {
                operation,
                reservation,
                buyer,
                amount,
            } if uuid(operation, Some(b'7'))
                && uuid(reservation, Some(b'7'))
                && uuid(buyer, None)
                && positive(*amount) =>
            {
                (
                    "POST",
                    "market/settlements".into(),
                    serde_json::to_string(&Settle {
                        operation_id: operation,
                        reservation_id: reservation,
                        buyer_account_id: buyer,
                        amount: amount.to_string(),
                    })?,
                )
            }
            Self::Release {
                operation,
                account,
                reservation,
            } if uuid(operation, Some(b'7'))
                && uuid(account, None)
                && uuid(reservation, Some(b'7')) =>
            {
                (
                    "POST",
                    "market/cancellations".into(),
                    serde_json::to_string(&Release {
                        operation_id: operation,
                        seller_account_id: account,
                        reservation_id: reservation,
                    })?,
                )
            }
            Self::Operation(id) if uuid(id, Some(b'7')) => {
                ("GET", format!("operations/{id}"), String::new())
            }
            Self::Balance(id) if uuid(id, None) => {
                ("GET", format!("accounts/{id}/balance"), String::new())
            }
            Self::History { account, cursor }
                if uuid(account, None) && (cursor.is_empty() || uuid(cursor, Some(b'7'))) =>
            {
                (
                    "GET",
                    format!("accounts/{account}/history?cursor={cursor}&limit=25"),
                    String::new(),
                )
            }
            _ => return Err(Error::Invalid),
        };
        if body.len() > 1024 {
            return Err(Error::Invalid);
        }
        Ok((method, format!("/internal/v1/coins/{path}"), body))
    }
    pub(super) fn matches(&self, receipt: &super::Receipt) -> bool {
        match self {
            Self::Transfer {
                operation,
                account,
                recipient,
                amount,
            } => {
                receipt.operation_id == *operation
                    && receipt.kind == "transfer"
                    && receipt.account_id == *account
                    && receipt.counterparty_account_id.as_ref() == Some(recipient)
                    && receipt.amount == amount.to_string()
            }
            Self::Reserve {
                operation,
                account,
                amount,
                ..
            } => {
                receipt.operation_id == *operation
                    && receipt.kind == "reserve"
                    && receipt.account_id == *account
                    && receipt.counterparty_account_id.is_none()
                    && receipt.amount == amount.to_string()
            }
            Self::Settle {
                operation,
                buyer,
                amount,
                ..
            } => {
                receipt.operation_id == *operation
                    && receipt.kind == "settlement"
                    && receipt.counterparty_account_id.as_ref() == Some(buyer)
                    && receipt.amount == amount.to_string()
            }
            Self::Release {
                operation, account, ..
            } => {
                receipt.operation_id == *operation
                    && receipt.kind == "release"
                    && receipt.account_id == *account
                    && receipt.counterparty_account_id.is_none()
            }
            Self::Operation(id) => receipt.operation_id == *id,
            _ => false,
        }
    }
}
