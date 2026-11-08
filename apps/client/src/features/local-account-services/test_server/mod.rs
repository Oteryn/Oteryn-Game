//! Test-only native Platform authority. No listener or verifier is compiled into the product.
use oteryn_foundation::CancellationToken;
use oteryn_platform_client::native_login::bazaar::*;
use oteryn_platform_client::{
    PlatformClientConfig,
    native_login::{AccessToken, NativeLoginClient},
};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
pub(crate) mod coins;
pub(crate) mod premium;

pub(crate) type TestResult = Result<(), Box<dyn std::error::Error>>;
pub(crate) const CHARACTER: &str = "01890a5d-ac96-774b-bcce-b302099a8058";
pub(crate) const AUCTION: &str = "01890a5d-ac96-774b-bcce-b302099a8057";
pub(crate) const WORLD: &str = "01890a5d-ac96-774b-bcce-b302099a8059";
pub(crate) const DESTINATION: &str = "01890a5d-ac96-774b-bcce-b302099a805a";
pub(crate) const QUOTE: &str = "01890a5d-ac96-774b-bcce-b302099a805b";

pub(crate) struct MockPlatform {
    pub runtime: tokio::runtime::Runtime,
    pub base: String,
    pub authority: Arc<Mutex<Authority>>,
}
#[derive(Default)]
pub(crate) struct Authority {
    pub coins: Option<coins::CoinAuthority>,
    pub premium_reply: Option<serde_json::Value>,
    pub premium_http_status: Option<u16>,
    pub commands: BTreeMap<String, (BazaarRequest, OperationReceipt)>,
    pub requests: Vec<String>,
    pub effects: u32,
    pub fail_after_commit: bool,
    pub pending_first: bool,
    pub paginate: bool,
    pub fail_pages: bool,
    pub offline: bool,
    pub lease_active: bool,
    pub in_guild: bool,
    pub has_house: bool,
    pub market: bool,
    pub cooldown: bool,
}
impl Authority {
    fn blockers(&self) -> Vec<String> {
        [
            (!self.offline, "Character must be offline"),
            (self.lease_active, "Character lease is active"),
            (self.in_guild, "Leave the guild"),
            (self.has_house, "Release the house"),
            (self.market, "Close active market offers"),
            (self.cooldown, "World-transfer cooldown is active"),
        ]
        .into_iter()
        .filter(|(blocked, _)| *blocked)
        .map(|(_, reason)| reason.to_owned())
        .collect()
    }
    fn page(&self, view: AuctionView) -> BazaarPage {
        BazaarPage {
            version: 1,
            view,
            balance: self
                .coins
                .as_ref()
                .map_or(1000, |c| c.balance.available_coins)
                .saturating_sub(u64::from(self.effects) * 50),
            auctions: vec![Auction {
                id: AUCTION.into(),
                character_id: CHARACTER.into(),
                character_name: "Ari".into(),
                world_id: WORLD.into(),
                world_name: "North".into(),
                level: 100,
                revision: 1,
                minimum_next_bid: 10,
                highest_bid: 0,
                ends_at: 2_000_000_000,
                state: if view == AuctionView::History {
                    AuctionState::Sold
                } else {
                    AuctionState::Open
                },
            }],
            owned_characters: vec![OwnedCharacter {
                id: CHARACTER.into(),
                name: "Ari".into(),
                world_id: WORLD.into(),
            }],
            worlds: vec![TransferWorld {
                id: DESTINATION.into(),
                name: "South".into(),
            }],
            next_cursor: None,
            recent_operations: self.commands.values().map(|(_, r)| r.clone()).collect(),
        }
    }
    fn route(
        &mut self,
        method: &str,
        target: &str,
        body: &str,
        authorized: bool,
        signature: Option<&str>,
    ) -> (u16, String) {
        if method == "POST" && target == "/mock-provider/webhook" {
            return match self
                .coins
                .as_mut()
                .map(|c| c.webhook(body.as_bytes(), signature.unwrap_or_default()))
            {
                Some(Ok(())) => (200, "{}".into()),
                _ => (400, "{}".into()),
            };
        }
        if method == "POST" && target == "/oauth/token" {
            return (
                200,
                r#"{"token_type":"Bearer","access_token":"services-test-token"}"#.into(),
            );
        }
        if method == "GET" && target == "/v1/client/directory" {
            return (
                200,
                format!(
                    r#"{{"epoch":1,"worlds":[{{"id":"{WORLD}","name":"North","channels":[{{"id":"{DESTINATION}","name":"First"}}],"characters":[]}}]}}"#
                ),
            );
        }
        if !authorized {
            return (401, String::new());
        }
        if method == "GET" && target == "/api/v1/native-account/premium" {
            let reply = self.premium_reply.clone().unwrap_or_else(|| {
                serde_json::json!(premium::snapshot(
                    oteryn_platform_client::native_login::premium::PremiumState::Active
                ))
            });
            return (self.premium_http_status.unwrap_or(200), reply.to_string());
        }
        if let Some(reply) = self
            .coins
            .as_mut()
            .and_then(|c| c.route(method, target, body))
        {
            return reply;
        }
        if method == "GET" && target == "/api/v1/game-auth/native-characters" {
            return (
                200,
                format!(
                    r#"{{"protocol_version":2,"characters":[{{"character_id":"{CHARACTER}","world_id":"{WORLD}","name":"Ari","availability":"AVAILABLE"}}]}}"#
                ),
            );
        }
        if method == "POST" && target == "/api/v1/game-auth/tickets" {
            return (
                200,
                r#"{"protocol_version":1,"ticket":"test-ticket","expires_in":60}"#.into(),
            );
        }
        if method == "GET" && target.starts_with("/api/v1/native-account/bazaar/auctions?") {
            if self.fail_pages {
                return (503, String::new());
            }
            let view = if target.contains("view=own") {
                AuctionView::Own
            } else if target.contains("view=history") {
                AuctionView::History
            } else {
                AuctionView::All
            };
            let mut page = self.page(view);
            if self.paginate {
                if target.contains("cursor=") {
                    if let Some(auction) = page.auctions.first_mut() {
                        auction.character_name = "Next character".into();
                    }
                } else {
                    page.next_cursor = Some("next/page?view=history&limit=1".into());
                }
            }
            return (200, serde_json::to_string(&page).unwrap_or_default());
        }
        if method == "POST" && target == "/api/v1/native-account/bazaar/transfer-quotes" {
            let quote = TransferQuote {
                version: 1,
                quote_id: QUOTE.into(),
                character_id: CHARACTER.into(),
                destination_world_id: DESTINATION.into(),
                price_coins: 50,
                cooldown_seconds: 86_400,
                valid_for_seconds: 120,
                blockers: self.blockers(),
            };
            return (200, serde_json::to_string(&quote).unwrap_or_default());
        }
        if method == "POST" && target == "/api/v1/native-account/bazaar/commands" {
            let Ok(request) = serde_json::from_str::<BazaarRequest>(body) else {
                return (400, String::new());
            };
            if let Some((original, receipt)) = self.commands.get(&request.request_id) {
                return if original == &request {
                    (200, serde_json::to_string(receipt).unwrap_or_default())
                } else {
                    (409, String::new())
                };
            }
            if request.version != 1 || !request.command.valid() {
                return (400, String::new());
            }
            let eligible = match &request.command {
                BazaarCommand::Bid {
                    auction_id,
                    expected_revision,
                    coins,
                } => {
                    auction_id == AUCTION && *expected_revision == 1 && (10..=1000).contains(coins)
                }
                BazaarCommand::Sell { character_id, .. } => {
                    character_id == CHARACTER && self.blockers().is_empty()
                }
                BazaarCommand::Transfer { quote_id } => {
                    quote_id == QUOTE && self.blockers().is_empty()
                }
            };
            let receipt = OperationReceipt {
                version: 1,
                request_id: request.request_id.clone(),
                state: if eligible && self.pending_first {
                    OperationState::Pending
                } else if eligible {
                    OperationState::Committed
                } else {
                    OperationState::Rejected
                },
                receipt_id: (eligible && !self.pending_first).then(|| request.request_id.clone()),
                code: if eligible {
                    "MOCK_COMMITTED"
                } else {
                    "NOT_ELIGIBLE"
                }
                .into(),
                charged_coins: if eligible && !self.pending_first {
                    50
                } else {
                    0
                },
                balance: if eligible && !self.pending_first {
                    950
                } else {
                    1000
                },
            };
            self.commands
                .insert(request.request_id.clone(), (request, receipt.clone()));
            if eligible && !self.pending_first {
                self.effects += 1;
            }
            if self.fail_after_commit {
                self.fail_after_commit = false;
                return (503, String::new());
            }
            return (200, serde_json::to_string(&receipt).unwrap_or_default());
        }
        if method == "GET"
            && let Some(id) = target.strip_prefix("/api/v1/native-account/bazaar/commands/")
        {
            if let Some((_, receipt)) = self.commands.get_mut(id) {
                if receipt.state == OperationState::Pending {
                    receipt.state = OperationState::Committed;
                    receipt.receipt_id = Some(id.into());
                    receipt.charged_coins = 50;
                    receipt.balance = 950;
                    self.effects += 1;
                }
                return (200, serde_json::to_string(receipt).unwrap_or_default());
            }
            return (404, String::new());
        }
        (404, String::new())
    }
}
mod http;

type TestResultWith<T> = Result<T, Box<dyn std::error::Error>>;
pub(crate) fn test_thread(test: impl FnOnce() -> TestResult + Send + 'static) -> TestResult {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || test().map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "test thread failed")?
        .map_err(Into::into)
}
