use super::View;

use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

/// Only a current-bearer receipt can identify a retained request. Only a live POST holds the lock.
#[derive(Clone)]
pub(crate) struct PriorRequests {
    ids: Arc<Mutex<BTreeSet<String>>>,
    post: Arc<Semaphore>,
}

impl std::fmt::Debug for PriorRequests {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PriorRequests")
    }
}

impl Default for PriorRequests {
    fn default() -> Self {
        Self::new(Arc::new(Semaphore::new(1)))
    }
}

impl PriorRequests {
    pub(crate) fn new(post: Arc<Semaphore>) -> Self {
        Self {
            ids: Default::default(),
            post,
        }
    }

    pub(crate) fn blocked(&self) -> bool {
        self.post.available_permits() == 0
    }

    pub(crate) fn start(&self) -> Option<OwnedSemaphorePermit> {
        self.post.clone().try_acquire_owned().ok()
    }

    fn ids(&self) -> std::sync::MutexGuard<'_, BTreeSet<String>> {
        self.ids
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    pub(crate) fn retain(&self, id: String) {
        self.ids().insert(id);
    }
    pub(crate) fn contains(&self, id: &str) -> bool {
        self.ids().contains(id)
    }
    pub(crate) fn reconcile(&self, id: &str) {
        self.ids().remove(id);
    }
}

#[derive(Default)]
pub(crate) struct Session {
    pub(super) account: View,
    pub(crate) coin: crate::coin_shop::View,
}

impl Session {
    pub(crate) fn new(
        account: PriorRequests,
        coin: PriorRequests,
        character: Option<String>,
    ) -> Self {
        Self {
            account: View {
                prior: account,
                ..View::default()
            },
            coin: crate::coin_shop::View::with_prior(character, coin),
        }
    }
}
