//! Oteryn Character Bazaar v1. Platform orchestrates; Game receipts prove custody effects.
use super::account_services::{WireReply, required_option, unique_ids, valid_id, valid_text};
use super::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuctionView {
    All,
    Own,
    History,
}
impl AuctionView {
    fn query(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Own => "own",
            Self::History => "history",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuctionState {
    Open,
    Settling,
    Sold,
    Expired,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Auction {
    pub id: String,
    pub character_id: String,
    pub character_name: String,
    pub world_id: String,
    pub world_name: String,
    pub level: u32,
    pub revision: u64,
    pub minimum_next_bid: u64,
    pub highest_bid: u64,
    pub ends_at: u64,
    pub state: AuctionState,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OwnedCharacter {
    pub id: String,
    pub name: String,
    pub world_id: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TransferWorld {
    pub id: String,
    pub name: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BazaarPage {
    pub version: u32,
    pub view: AuctionView,
    pub auctions: Vec<Auction>,
    pub owned_characters: Vec<OwnedCharacter>,
    pub worlds: Vec<TransferWorld>,
    pub balance: u64,
    #[serde(deserialize_with = "required_option")]
    pub next_cursor: Option<String>,
    pub recent_operations: Vec<OperationReceipt>,
}
impl WireReply for BazaarPage {
    fn valid(&self) -> bool {
        self.version == 1
            && self.auctions.len() <= 50
            && self.owned_characters.len() <= 100
            && self.worlds.len() <= 256
            && self.balance <= MAX_COINS
            && self.recent_operations.len() <= 50
            && self.recent_operations.iter().all(WireReply::valid)
            && unique_ids(self.auctions.iter().map(|a| a.id.as_str()))
            && unique_ids(self.owned_characters.iter().map(|c| c.id.as_str()))
            && unique_ids(self.worlds.iter().map(|w| w.id.as_str()))
            && unique_ids(self.recent_operations.iter().map(|r| r.request_id.as_str()))
            && self.next_cursor.as_ref().is_none_or(|s| valid_text(s, 256))
            && self.auctions.iter().all(|a| {
                valid_id(&a.id)
                    && valid_id(&a.character_id)
                    && valid_id(&a.world_id)
                    && valid_text(&a.character_name, 64)
                    && valid_text(&a.world_name, 64)
                    && a.revision > 0
                    && a.minimum_next_bid > 0
                    && a.minimum_next_bid <= MAX_COINS
                    && a.highest_bid <= MAX_COINS
                    && a.ends_at > 0
            })
            && self
                .owned_characters
                .iter()
                .all(|c| valid_id(&c.id) && valid_id(&c.world_id) && valid_text(&c.name, 64))
            && self
                .worlds
                .iter()
                .all(|w| valid_id(&w.id) && valid_text(&w.name, 64))
    }
}
pub const MAX_COINS: u64 = 999_999_999_999;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub enum BazaarCommand {
    Bid {
        auction_id: String,
        expected_revision: u64,
        coins: u64,
    },
    Sell {
        character_id: String,
        minimum_bid: u64,
        duration_hours: u32,
    },
    Transfer {
        quote_id: String,
    },
}
impl BazaarCommand {
    pub fn valid(&self) -> bool {
        match self {
            Self::Bid {
                auction_id,
                expected_revision,
                coins,
            } => valid_id(auction_id) && *expected_revision > 0 && (1..=MAX_COINS).contains(coins),
            Self::Sell {
                character_id,
                minimum_bid,
                duration_hours,
            } => {
                valid_id(character_id)
                    && (1..=MAX_COINS).contains(minimum_bid)
                    && (1..=168).contains(duration_hours)
            }
            Self::Transfer { quote_id } => valid_id(quote_id),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BazaarRequest {
    pub version: u32,
    pub request_id: String,
    pub command: BazaarCommand,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationState {
    Pending,
    Committed,
    Rejected,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperationReceipt {
    pub version: u32,
    pub request_id: String,
    pub state: OperationState,
    #[serde(deserialize_with = "required_option")]
    pub receipt_id: Option<String>,
    pub code: String,
    pub charged_coins: u64,
    pub balance: u64,
}
impl WireReply for OperationReceipt {
    fn valid(&self) -> bool {
        self.version == 1
            && valid_id(&self.request_id)
            && valid_text(&self.code, 96)
            && self.charged_coins <= MAX_COINS
            && self.balance <= MAX_COINS
            && match self.state {
                OperationState::Committed => self.receipt_id.as_ref().is_some_and(|s| valid_id(s)),
                OperationState::Pending | OperationState::Rejected => {
                    self.receipt_id.is_none() && self.charged_coins == 0
                }
            }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TransferQuote {
    pub version: u32,
    pub quote_id: String,
    pub character_id: String,
    pub destination_world_id: String,
    pub price_coins: u64,
    pub cooldown_seconds: u64,
    pub valid_for_seconds: u32,
    pub blockers: Vec<String>,
}
impl WireReply for TransferQuote {
    fn valid(&self) -> bool {
        self.version == 1
            && valid_id(&self.quote_id)
            && valid_id(&self.character_id)
            && valid_id(&self.destination_world_id)
            && self.price_coins <= MAX_COINS
            && (1..=300).contains(&self.valid_for_seconds)
            && self.blockers.len() <= 16
            && self.blockers.iter().all(|b| valid_text(b, 160))
    }
}

impl NativeLoginClient {
    pub async fn bazaar_page(
        &self,
        token: &AccessToken,
        view: AuctionView,
        cursor: Option<&str>,
        cancel: CancellationToken,
    ) -> Result<BazaarPage, NativeLoginError> {
        if cursor.is_some_and(|s| !valid_text(s, 256)) {
            return Err(PlatformClientError::InvalidPayload.into());
        }
        let mut url = self
            .platform
            .join("/api/v1/native-account/bazaar/auctions")
            .map_err(|_| PlatformClientError::InvalidBaseUrl)?;
        url.query_pairs_mut()
            .append_pair("view", view.query())
            .append_pair("limit", "50");
        if let Some(cursor) = cursor {
            url.query_pairs_mut().append_pair("cursor", cursor);
        }
        let page: BazaarPage = self
            .service_request(token, reqwest::Method::GET, url, None, cancel)
            .await?;
        if page.view != view {
            return Err(PlatformClientError::InvalidPayload.into());
        }
        Ok(page)
    }
    pub async fn bazaar_command(
        &self,
        token: &AccessToken,
        request: &BazaarRequest,
        cancel: CancellationToken,
    ) -> Result<OperationReceipt, NativeLoginError> {
        if request.version != 1 || !valid_id(&request.request_id) || !request.command.valid() {
            return Err(PlatformClientError::InvalidPayload.into());
        }
        let url = self
            .platform
            .join("/api/v1/native-account/bazaar/commands")
            .map_err(|_| PlatformClientError::InvalidBaseUrl)?;
        let receipt: OperationReceipt = self
            .service_request(
                token,
                reqwest::Method::POST,
                url,
                Some(
                    serde_json::to_value(request)
                        .map_err(|_| PlatformClientError::InvalidPayload)?,
                ),
                cancel,
            )
            .await?;
        if receipt.request_id != request.request_id {
            return Err(PlatformClientError::InvalidPayload.into());
        }
        Ok(receipt)
    }
    pub async fn bazaar_status(
        &self,
        token: &AccessToken,
        request_id: &str,
        cancel: CancellationToken,
    ) -> Result<OperationReceipt, NativeLoginError> {
        if !valid_id(request_id) {
            return Err(PlatformClientError::InvalidPayload.into());
        }
        let url = self
            .platform
            .join(&format!(
                "/api/v1/native-account/bazaar/commands/{request_id}"
            ))
            .map_err(|_| PlatformClientError::InvalidBaseUrl)?;
        let receipt: OperationReceipt = self
            .service_request(token, reqwest::Method::GET, url, None, cancel)
            .await?;
        if receipt.request_id != request_id {
            return Err(PlatformClientError::InvalidPayload.into());
        }
        Ok(receipt)
    }
    pub async fn transfer_quote(
        &self,
        token: &AccessToken,
        character_id: &str,
        world_id: &str,
        cancel: CancellationToken,
    ) -> Result<TransferQuote, NativeLoginError> {
        if !valid_id(character_id) || !valid_id(world_id) {
            return Err(PlatformClientError::InvalidPayload.into());
        }
        let url = self
            .platform
            .join("/api/v1/native-account/bazaar/transfer-quotes")
            .map_err(|_| PlatformClientError::InvalidBaseUrl)?;
        let quote:TransferQuote=self.service_request(token,reqwest::Method::POST,url,Some(serde_json::json!({"version":1,"character_id":character_id,"destination_world_id":world_id})),cancel).await?;
        if quote.character_id != character_id || quote.destination_world_id != world_id {
            return Err(PlatformClientError::InvalidPayload.into());
        }
        Ok(quote)
    }
}
