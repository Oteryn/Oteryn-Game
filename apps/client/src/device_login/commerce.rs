use super::DeviceLogin;
use crate::account_services::Session;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::Mutex as AsyncMutex;

#[derive(Eq, Hash, PartialEq)]
struct SignIn {
    platform: String,
    client: String,
}

#[derive(Default)]
pub(super) struct Commerce(Mutex<HashMap<SignIn, Arc<AsyncMutex<Session>>>>);

impl DeviceLogin {
    pub(crate) fn commerce_session(
        &self,
        platform: &str,
        client: &str,
    ) -> Arc<AsyncMutex<Session>> {
        let key = SignIn {
            platform: platform.into(),
            client: client.into(),
        };
        self.commerce
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .entry(key)
            .or_default()
            .clone()
    }
}
