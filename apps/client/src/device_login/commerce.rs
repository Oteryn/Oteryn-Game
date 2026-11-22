use super::DeviceLogin;
use crate::account_services::Session;
use std::sync::{Arc, Mutex};
use tokio::sync::Mutex as AsyncMutex;

struct SignIn {
    platform: String,
    client: String,
    session: Arc<AsyncMutex<Session>>,
}

#[derive(Default)]
pub(super) struct Commerce(Mutex<Option<SignIn>>);

impl DeviceLogin {
    pub(crate) fn commerce_session(
        &self,
        platform: &str,
        client: &str,
    ) -> Arc<AsyncMutex<Session>> {
        let mut signin = self
            .commerce
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(current) = signin.as_ref()
            && current.platform == platform
            && current.client == client
        {
            return current.session.clone();
        }
        let session = Arc::default();
        *signin = Some(SignIn {
            platform: platform.into(),
            client: client.into(),
            session: Arc::clone(&session),
        });
        session
    }

    pub(crate) fn clear_commerce(&self) {
        self.commerce
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take();
    }
}
