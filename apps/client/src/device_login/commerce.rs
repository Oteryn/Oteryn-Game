use super::DeviceLogin;
use crate::account_services::Session;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::Mutex as AsyncMutex;

#[derive(Eq, Hash, PartialEq)]
struct SignIn {
    platform: String,
    client: String,
    account: String,
}

#[derive(Default)]
pub(super) struct Commerce(Mutex<HashMap<SignIn, Arc<AsyncMutex<Session>>>>);

impl DeviceLogin {
    pub(crate) fn commerce_session(
        &self,
        platform: &str,
        client: &str,
        token: &oteryn_platform_client::native_login::AccessToken,
    ) -> Arc<AsyncMutex<Session>> {
        let Some(account) = token.account_id() else {
            return Arc::default();
        };
        let key = SignIn {
            platform: platform.into(),
            client: client.into(),
            account: account.into(),
        };
        self.commerce
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .entry(key)
            .or_default()
            .clone()
    }

    pub(crate) fn clear_commerce(&self) {
        self.commerce
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clear();
    }
}
