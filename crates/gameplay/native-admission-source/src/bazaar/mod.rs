//! Exact Bazaar authority v2 intent boundary (contract §§2–3).
//!
//! Parsing does not grant authority, reserve funds, or move custody. Callers must
//! compare the whole-byte digest with the durable operation before checking TTL.

pub mod transport;

use super::SourceError;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const ISSUER: &str = "OTERYN_PLATFORM_BAZAAR_AUTHORITY";
pub const AUDIENCE: &str = "OTERYN_GAME_BAZAAR_AUTHORITY";
pub const INTENT_BYTES: usize = 4096;
pub const MAX_TTL_SECONDS: u64 = 300;
pub const MAX_FUTURE_SKEW_SECONDS: u64 = 5;
pub const BANK_CEILING: u64 = 999999999999;

/// Wire scalar whose canonical representation has been checked at decode time.
macro_rules! scalar {
    ($name:ident, $valid:expr) => {
        #[derive(Debug, Clone, PartialEq, Eq, Serialize)]
        #[serde(transparent)]
        pub struct $name(String);
        impl $name {
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }
        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                let value = String::deserialize(deserializer)?;
                if ($valid)(&value) {
                    Ok(Self(value))
                } else {
                    Err(serde::de::Error::custom("invalid Bazaar scalar"))
                }
            }
        }
    };
}

fn decimal(value: &str) -> Option<u64> {
    if value == "0" {
        Some(0)
    } else {
        super::character_commands::counter(value)
    }
}
scalar!(Id, |s: &str| super::character_commands::uuid_v7(s));
scalar!(Hash, |s: &str| s.len() == 64
    && s.bytes()
        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)));

macro_rules! number {
    ($name:ident, $min:expr, $max:expr) => {
        #[derive(Debug, Clone, PartialEq, Eq)]
        pub struct $name(u64);
        impl $name {
            pub fn get(&self) -> u64 {
                self.0
            }
        }
        impl Serialize for $name {
            fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                serializer.serialize_str(&self.0.to_string())
            }
        }
        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                let text = String::deserialize(deserializer)?;
                let value = decimal(&text)
                    .filter(|n| ($min..=$max).contains(n))
                    .ok_or_else(|| serde::de::Error::custom("invalid Bazaar counter"))?;
                Ok(Self(value))
            }
        }
    };
}
number!(Counter, 1, u64::MAX);
number!(ZeroCounter, 0, u64::MAX);
number!(Gold, 0, BANK_CEILING);
number!(PositiveGold, 1, BANK_CEILING);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Currency {
    Gold,
    Coins,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Funding {
    pub currency: Currency,
    pub reservation_id: Id,
    // A required nullable member: canonical re-encoding rejects missing fields.
    pub hold_revision: Option<Counter>,
    pub gross_amount: Counter,
}
impl Funding {
    fn valid(&self) -> bool {
        match self.currency {
            Currency::Gold => {
                self.hold_revision.is_some() && self.gross_amount.get() <= BANK_CEILING
            }
            Currency::Coins => {
                self.hold_revision.is_none() && self.gross_amount.get() <= i64::MAX as u64
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TransferPurpose {
    BazaarListingEscrow,
    BazaarSettlement,
    BazaarEscrowReturn,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum HoldPurpose {
    Offer,
    Reserve,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ReleaseReason {
    Outbid,
    Declined,
    Cancelled,
    Unsold,
    Excess,
    TerminalRefusal,
}

macro_rules! intent {
    ($name:ident { $($field:ident: $ty:ty),* $(,)? }) => {
        #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
        #[serde(deny_unknown_fields)]
        pub struct $name {
            pub contract_version: u8,
            pub command: String,
            pub variant: String,
            pub issuer_authority: String,
            pub issuer_decision_id: Id,
            pub source_revision: Counter,
            pub operation_id: Id,
            pub listing_id: Id,
            pub seller_account_id: Id,
            pub world_id: Id,
            $(pub $field: $ty,)*
            pub issued_at_source: Counter,
            pub expires_at_source: Counter,
            pub audience: String,
        }
    };
}

intent!(LockBazaarCharacter {
    character_id: Id,
    expected_character_revision: Counter,
    eligibility_revision: Counter,
    snapshot_schema_version: u8
});

intent!(BindBazaarCharacterTransfer {
    character_id: Id,
    lock_id: Id,
    expected_lock_revision: Counter,
    transfer_operation_id: Id,
    transfer_binding_digest: Hash,
    purpose: TransferPurpose,
    sale_id: Option<Id>,
    funding: Option<Funding>
});

intent!(UnlockBazaarCharacter {
    character_id: Id,
    lock_id: Id,
    expected_lock_revision: Counter
});

intent!(EscrowBazaarItem {
    seller_character_id: Id,
    item_instance_id: Id,
    expected_state_revision: Counter,
    quantity: u32,
    transferability_revision: Counter
});

intent!(DeliverBazaarItem {
    escrow_id: Id,
    expected_escrow_revision: Counter,
    buyer_account_id: Id,
    buyer_character_id: Id,
    escrow_receipt_digest: Hash,
    sale_id: Id,
    funding: Funding
});

intent!(ReturnBazaarItem {
    escrow_id: Id,
    expected_escrow_revision: Counter,
    seller_character_id: Id,
    escrow_receipt_digest: Hash
});

intent!(HoldBazaarGold {
    buyer_account_id: Id,
    buyer_character_id: Id,
    offer_id: Id,
    hold_purpose: HoldPurpose,
    amount: PositiveGold,
    expected_hold_revision: ZeroCounter
});

intent!(ReleaseBazaarGold {
    buyer_account_id: Id,
    hold_id: Id,
    expected_hold_revision: Counter,
    release_reason: ReleaseReason,
    amount: PositiveGold
});

intent!(PayoutBazaarGold {
    buyer_account_id: Id,
    hold_id: Id,
    expected_hold_revision: Counter,
    sale_id: Id,
    gross_amount: PositiveGold,
    fee_bps: u16,
    fee_amount: Gold,
    seller_amount: PositiveGold,
    delivery_operation_id: Id,
    delivery_receipt_digest: Hash
});

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Intent {
    LockBazaarCharacter(LockBazaarCharacter),
    BindBazaarCharacterTransfer(BindBazaarCharacterTransfer),
    UnlockBazaarCharacter(UnlockBazaarCharacter),
    EscrowBazaarItem(EscrowBazaarItem),
    DeliverBazaarItem(DeliverBazaarItem),
    ReturnBazaarItem(ReturnBazaarItem),
    HoldBazaarGold(HoldBazaarGold),
    ReleaseBazaarGold(ReleaseBazaarGold),
    PayoutBazaarGold(PayoutBazaarGold),
}

impl Intent {
    pub fn encode(&self) -> Result<Vec<u8>, SourceError> {
        let raw = match self {
            Self::LockBazaarCharacter(wire) => serde_json::to_vec(wire),
            Self::BindBazaarCharacterTransfer(wire) => serde_json::to_vec(wire),
            Self::UnlockBazaarCharacter(wire) => serde_json::to_vec(wire),
            Self::EscrowBazaarItem(wire) => serde_json::to_vec(wire),
            Self::DeliverBazaarItem(wire) => serde_json::to_vec(wire),
            Self::ReturnBazaarItem(wire) => serde_json::to_vec(wire),
            Self::HoldBazaarGold(wire) => serde_json::to_vec(wire),
            Self::ReleaseBazaarGold(wire) => serde_json::to_vec(wire),
            Self::PayoutBazaarGold(wire) => serde_json::to_vec(wire),
        }
        .map_err(|_| SourceError::InvalidInput)?;
        if raw.len() > INTENT_BYTES {
            return Err(SourceError::CapacityExceeded);
        }
        Ok(raw)
    }

    /// Digest of the canonical bytes. A durable caller must hash the received
    /// bytes and compare operation identity before parsing or checking time.
    pub fn binding_digest(&self) -> Result<[u8; 32], SourceError> {
        Ok(Sha256::digest(self.encode()?).into())
    }

    /// Only for unseen attempts, after the durable operation-identity comparison.
    pub fn time_valid(&self, trusted_now: u64) -> bool {
        let (issued, expires) = match self {
            Self::LockBazaarCharacter(wire) => {
                (wire.issued_at_source.get(), wire.expires_at_source.get())
            }
            Self::BindBazaarCharacterTransfer(wire) => {
                (wire.issued_at_source.get(), wire.expires_at_source.get())
            }
            Self::UnlockBazaarCharacter(wire) => {
                (wire.issued_at_source.get(), wire.expires_at_source.get())
            }
            Self::EscrowBazaarItem(wire) => {
                (wire.issued_at_source.get(), wire.expires_at_source.get())
            }
            Self::DeliverBazaarItem(wire) => {
                (wire.issued_at_source.get(), wire.expires_at_source.get())
            }
            Self::ReturnBazaarItem(wire) => {
                (wire.issued_at_source.get(), wire.expires_at_source.get())
            }
            Self::HoldBazaarGold(wire) => {
                (wire.issued_at_source.get(), wire.expires_at_source.get())
            }
            Self::ReleaseBazaarGold(wire) => {
                (wire.issued_at_source.get(), wire.expires_at_source.get())
            }
            Self::PayoutBazaarGold(wire) => {
                (wire.issued_at_source.get(), wire.expires_at_source.get())
            }
        };
        issued <= trusted_now.saturating_add(MAX_FUTURE_SKEW_SECONDS) && trusted_now < expires
    }
}

/// Closed, byte-exact decoder. A valid intent is still untrusted authorization
/// until the owning durable transaction has checked current domain fences.
pub fn decode_intent(raw: &[u8], command: &str, operation_id: &str) -> Result<Intent, SourceError> {
    if raw.is_empty() || raw.len() > INTENT_BYTES || raw.contains(&b'\\') {
        return Err(SourceError::InvalidInput);
    }
    macro_rules! decode {
        ($ty:ident, $valid:expr) => {{
            let wire: $ty = serde_json::from_slice(raw).map_err(|_| SourceError::InvalidInput)?;
            let issued = wire.issued_at_source.get();
            let expires = wire.expires_at_source.get();
            if wire.contract_version != 2
                || wire.command != command
                || wire.variant != "PLATFORM_BAZAAR"
                || wire.issuer_authority != ISSUER
                || wire.audience != AUDIENCE
                || wire.operation_id.as_str() != operation_id
                || expires <= issued
                || expires - issued > MAX_TTL_SECONDS
                || !($valid)(&wire)
                || serde_json::to_vec(&wire).map_err(|_| SourceError::InvalidInput)? != raw
            {
                return Err(SourceError::InvalidInput);
            }
            Intent::$ty(wire)
        }};
    }
    Ok(match command {
        "LockBazaarCharacter" => decode!(LockBazaarCharacter, |w: &LockBazaarCharacter| w
            .snapshot_schema_version
            == 1),
        "BindBazaarCharacterTransfer" => decode!(
            BindBazaarCharacterTransfer,
            |w: &BindBazaarCharacterTransfer| {
                let settlement = w.purpose == TransferPurpose::BazaarSettlement;
                w.sale_id.is_some() == settlement
                    && w.funding.is_some() == settlement
                    && w.funding.as_ref().is_none_or(Funding::valid)
            }
        ),
        "UnlockBazaarCharacter" => decode!(UnlockBazaarCharacter, |_: &UnlockBazaarCharacter| true),
        "EscrowBazaarItem" => decode!(EscrowBazaarItem, |w: &EscrowBazaarItem| w.quantity > 0),
        "DeliverBazaarItem" => {
            decode!(DeliverBazaarItem, |w: &DeliverBazaarItem| w.funding.valid()
                && w.buyer_account_id != w.seller_account_id)
        }
        "ReturnBazaarItem" => decode!(ReturnBazaarItem, |_: &ReturnBazaarItem| true),
        "HoldBazaarGold" => decode!(HoldBazaarGold, |w: &HoldBazaarGold| w.buyer_account_id
            != w.seller_account_id),
        "ReleaseBazaarGold" => decode!(ReleaseBazaarGold, |w: &ReleaseBazaarGold| w
            .buyer_account_id
            != w.seller_account_id),
        "PayoutBazaarGold" => decode!(PayoutBazaarGold, |w: &PayoutBazaarGold| {
            w.fee_bps == 250 && w.buyer_account_id != w.seller_account_id
        }),
        _ => return Err(SourceError::InvalidInput),
    })
}

#[cfg(test)]
mod tests;
