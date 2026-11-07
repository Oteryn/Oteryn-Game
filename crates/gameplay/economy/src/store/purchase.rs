use oteryn_gameplay_domain::domain::{AccountId, CharacterId};
use sha2::{Digest, Sha256};

use super::{CurrencyKind, PurchaseQuote};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PurchaseRequestId([u8; 16]);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PurchaseRequestIdError {
    Nil,
    WrongVersion,
    WrongVariant,
}

impl PurchaseRequestId {
    pub fn from_bytes(bytes: [u8; 16]) -> Result<Self, PurchaseRequestIdError> {
        if bytes.iter().all(|byte| *byte == 0) {
            return Err(PurchaseRequestIdError::Nil);
        }
        if bytes[6] >> 4 != 7 {
            return Err(PurchaseRequestIdError::WrongVersion);
        }
        if bytes[8] >> 6 != 0b10 {
            return Err(PurchaseRequestIdError::WrongVariant);
        }
        Ok(Self(bytes))
    }
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PurchaseStoreOffer {
    pub purchase_request_id: PurchaseRequestId,
    pub account_id: AccountId,
    pub character_id: CharacterId,
    pub offer_key: String,
    pub catalog_revision: String,
    pub price: u64,
    pub currency: CurrencyKind,
}

impl PurchaseStoreOffer {
    #[must_use]
    pub fn from_quote(
        id: PurchaseRequestId,
        account_id: AccountId,
        character_id: CharacterId,
        quote: PurchaseQuote,
    ) -> Self {
        Self {
            purchase_request_id: id,
            account_id,
            character_id,
            offer_key: quote.offer_key,
            catalog_revision: quote.catalog_revision,
            price: quote.price,
            currency: quote.currency,
        }
    }

    /// SHA-256 of a length-delimited, versioned encoding of every immutable binding member.
    #[must_use]
    pub fn binding_digest(&self) -> [u8; 32] {
        let mut hash = Sha256::new();
        hash.update(b"oteryn.store.purchase-binding.v1\0");
        hash.update(self.account_id.as_bytes());
        hash.update(self.character_id.as_bytes());
        put_string(&mut hash, &self.offer_key);
        put_string(&mut hash, &self.catalog_revision);
        hash.update(self.price.to_be_bytes());
        hash.update([match self.currency {
            CurrencyKind::TibiaCoin => 0,
            CurrencyKind::TransferableTibiaCoin => 1,
        }]);
        hash.finalize().into()
    }
}

fn put_string(hash: &mut Sha256, value: &str) {
    hash.update((value.len() as u64).to_be_bytes());
    hash.update(value.as_bytes());
}

#[cfg(test)]
#[path = "tests/purchase_tests.rs"]
mod tests;
