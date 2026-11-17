//! Read-only Premium presentation; gameplay entitlement authority remains Platform/Game.
use super::account_services::{WireReply, required_option, unique_ids, valid_text};
use super::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PremiumState {
    None,
    Active,
    Expired,
    Revoked,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PremiumFeature {
    pub id: String,
    pub title: String,
    pub description: String,
    pub enabled: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PremiumStatus {
    pub version: u32,
    pub authority_revision: u64,
    pub generated_at_unix: u64,
    pub valid_for_seconds: u32,
    pub state: PremiumState,
    #[serde(deserialize_with = "required_option")]
    pub expires_at_unix: Option<u64>,
    pub features: Vec<PremiumFeature>,
}
impl WireReply for PremiumStatus {
    fn valid(&self) -> bool {
        self.version == 1
            && self.authority_revision > 0
            && (1..=253_402_300_799).contains(&self.generated_at_unix)
            && (1..=300).contains(&self.valid_for_seconds)
            && self
                .expires_at_unix
                .is_none_or(|t| (1..=253_402_300_799).contains(&t))
            && match self.state {
                PremiumState::None => self.expires_at_unix.is_none(),
                PremiumState::Active => self
                    .expires_at_unix
                    .is_some_and(|t| t > self.generated_at_unix),
                PremiumState::Expired => self
                    .expires_at_unix
                    .is_some_and(|t| t <= self.generated_at_unix),
                PremiumState::Revoked => true,
            }
            && self.features.len() <= 32
            && unique_ids(self.features.iter().map(|f| f.id.as_str()))
            && self.features.iter().all(|f| {
                valid_text(&f.id, 64)
                    && valid_text(&f.title, 80)
                    && valid_text(&f.description, 512)
                    && (self.state == PremiumState::Active || !f.enabled)
            })
    }
}
impl NativeLoginClient {
    /// Payer is resolved from the scoped token; there is no client-chosen account ID.
    pub async fn premium_status(
        &self,
        token: &AccessToken,
        cancel: CancellationToken,
    ) -> Result<PremiumStatus, NativeLoginError> {
        let url = self
            .platform
            .join("/api/v1/native-account/premium")
            .map_err(|_| PlatformClientError::InvalidBaseUrl)?;
        self.service_request(token, reqwest::Method::GET, url, None, cancel)
            .await
    }
}
