use super::View;

/// IDs from earlier grants never enter the presentation or retry paths.
#[derive(Clone, Default)]
pub(crate) struct PriorRequests(std::collections::BTreeSet<String>);

impl std::fmt::Debug for PriorRequests {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PriorRequests")
    }
}

impl PriorRequests {
    pub(crate) fn blocked(&self) -> bool {
        !self.0.is_empty()
    }
    pub(crate) fn retain(&mut self, id: String) {
        self.0.insert(id);
    }
    pub(crate) fn contains(&self, id: &str) -> bool {
        self.0.contains(id)
    }
    pub(crate) fn reconcile(&mut self, id: &str) {
        self.0.remove(id);
    }
}

#[derive(Default)]
pub(crate) struct Session {
    pub(super) account: View,
    pub(super) coin: crate::coin_shop::View,
}

impl Session {
    pub(crate) fn reopen_coins(
        &mut self,
        character_id: Option<String>,
    ) -> &mut crate::coin_shop::View {
        self.coin.reopen(character_id);
        &mut self.coin
    }

    pub(crate) fn reopen(&mut self) {
        let mut prior = std::mem::take(&mut self.account.prior);
        if let Some(request) = self.account.pending.take() {
            prior.retain(request.request_id);
        }
        prior.0.append(&mut self.account.unresolved);
        self.account = View {
            prior,
            ..Default::default()
        };
        self.reopen_coins(None);
    }
}
