use super::DeviceLogin;
use crate::account_services::{PriorRequests, Session};
use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex};
use tokio::sync::Semaphore;

#[derive(Eq, Hash, PartialEq)]
struct SignIn {
    platform: String,
    client: String,
}

#[derive(Default)]
pub(super) struct Commerce(Mutex<HashMap<SignIn, (PriorRequests, PriorRequests)>>);

static BAZAAR_POST: LazyLock<Arc<Semaphore>> = LazyLock::new(|| Arc::new(Semaphore::new(1)));
static COIN_POST: LazyLock<Arc<Semaphore>> = LazyLock::new(|| Arc::new(Semaphore::new(1)));

impl Commerce {
    pub(super) fn clear(&self) {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clear();
    }
}

impl DeviceLogin {
    pub(crate) fn commerce_session(
        &self,
        platform: &str,
        client: &str,
        character: Option<String>,
    ) -> Session {
        let key = SignIn {
            platform: platform.into(),
            client: client.into(),
        };
        let (account, coin) = self
            .commerce
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .entry(key)
            .or_insert_with(|| {
                (
                    PriorRequests::new(BAZAAR_POST.clone()),
                    PriorRequests::new(COIN_POST.clone()),
                )
            })
            .clone();
        Session::new(account, coin, character)
    }
}
